import dom/areas/core
import dom/areas/core/document
import dom/areas/core/element
import dom/areas/core/node as dom_node
import dom/areas/html/html_element
import gleam/dynamic.{type Dynamic}
import gleam/dynamic/decode
import gleam/float
import gleam/int
import gleam/list
import gleam/option.{None, Some}
import prosemirror/areas/main/editor_state_apply_transaction_result as apply_res
import prosemirror/areas/main/mark_spec_attrs
import prosemirror/areas/prosemirror_model/attribute_spec
import prosemirror/areas/prosemirror_model/attrs
import prosemirror/areas/prosemirror_model/fragment
import prosemirror/areas/prosemirror_model/mark_spec
import prosemirror/areas/prosemirror_model/node
import prosemirror/areas/prosemirror_model/node_spec
import prosemirror/areas/prosemirror_model/schema
import prosemirror/areas/prosemirror_model/schema_spec
import prosemirror/areas/prosemirror_state/editor_state
import prosemirror/areas/prosemirror_state/editor_state_config
import prosemirror/areas/prosemirror_state/plugin
import prosemirror/areas/prosemirror_state/plugin_key
import prosemirror/areas/prosemirror_state/plugin_spec
import prosemirror/areas/prosemirror_state/state_field
import prosemirror/areas/prosemirror_state/transaction
import prosemirror/areas/prosemirror_transform/step
import prosemirror/areas/prosemirror_transform/transform
import prosemirror/areas/prosemirror_view/direct_editor_props
import prosemirror/areas/prosemirror_view/editor_view
import prosemirror/runtime as pm_rt
import prosemirror/types as t

@external(javascript, "./app_ffi.mjs", "newCell")
fn new_cell(v: a) -> Dynamic

@external(javascript, "./app_ffi.mjs", "getCell")
fn get_cell(c: Dynamic) -> a

@external(javascript, "./app_ffi.mjs", "setCell")
fn set_cell(c: Dynamic, v: a) -> Nil

fn hole_spec(tag: String) -> t.DOMOutputSpec {
  t.DOMOutputSpecArray([dynamic.string(tag), dynamic.int(0)])
}

fn build_schema() -> t.Schema(Dynamic, Dynamic, Nil) {
  let doc_spec = node_spec.create()
  node_spec.set_content(doc_spec, Some("block+"))

  let lvl_spec = attribute_spec.create()
  attribute_spec.set_default(lvl_spec, dynamic.int(2))
  let h_attrs = mark_spec_attrs.create()
  mark_spec_attrs.set_at(h_attrs, "level", lvl_spec)

  let heading_spec = node_spec.create()
  node_spec.set_attrs(heading_spec, Some(h_attrs))
  node_spec.set_content(heading_spec, Some("inline*"))
  node_spec.set_group(heading_spec, Some("block"))
  node_spec.set_to_dom(
    heading_spec,
    Some(fn(n) {
      let lvl = case decode.run(attrs.get_at(node.get_attrs(n), "level"), decode.int) {
        Ok(1) -> "h1"
        _ -> "h2"
      }
      hole_spec(lvl)
    }),
  )

  let para_spec = node_spec.create()
  node_spec.set_content(para_spec, Some("inline*"))
  node_spec.set_group(para_spec, Some("block"))
  node_spec.set_to_dom(para_spec, Some(fn(_) { hole_spec("p") }))

  let text_spec = node_spec.create()
  node_spec.set_group(text_spec, Some("inline"))

  let nodes = node_spec.create()
  node_spec.set_at(nodes, "doc", pm_rt.dynamic(doc_spec))
  node_spec.set_at(nodes, "heading", pm_rt.dynamic(heading_spec))
  node_spec.set_at(nodes, "paragraph", pm_rt.dynamic(para_spec))
  node_spec.set_at(nodes, "text", pm_rt.dynamic(text_spec))

  let make_mark = fn(tag: String) {
    let m = mark_spec.create()
    mark_spec.set_to_dom(m, Some(fn(_, _) { hole_spec(tag) }))
    m
  }
  let marks = mark_spec.create()
  mark_spec.set_at(marks, "strong", pm_rt.dynamic(make_mark("strong")))
  mark_spec.set_at(marks, "code", pm_rt.dynamic(make_mark("code")))

  let spec = schema_spec.create()
  schema_spec.set_nodes(spec, pm_rt.dynamic(nodes))
  schema_spec.set_marks(spec, pm_rt.dynamic(marks))
  schema.new(spec)
}

fn p_node(sch, txt: String) -> t.Node(Nil) {
  schema.node(sch, t.StringOrNodeTypeString("paragraph"), None, Some(t.FragmentOrNodeOrNodeArrayNode(schema.text(sch, txt, None))), None)
}

fn make_initial_doc(sch: t.Schema(Dynamic, Dynamic, Nil)) -> t.Node(Nil) {
  let ha = attrs.create()
  attrs.set_at(ha, "level", dynamic.int(2))
  let h =
    schema.node(
      sch,
      t.StringOrNodeTypeString("heading"),
      Some(ha),
      Some(t.FragmentOrNodeOrNodeArrayNode(schema.text(sch, "Step-Inversion Undo & Transaction Filter", None))),
      None,
    )
  schema.node(
    sch,
    t.StringOrNodeTypeString("doc"),
    None,
    Some(
      t.FragmentOrNodeOrNodeArrayArray([
        h,
        p_node(sch, "Every document change produces atomic Transform steps that can be inverted against prior snapshots."),
        p_node(sch, "A ProseMirror Plugin inspects transactions in filterTransaction and can enforce a read-only lock."),
        p_node(sch, "Click Append Revision or Bold First Line, then step backward with Undo and forward with Redo."),
      ]),
    ),
    None,
  )
}

fn update_meta(doc, audit_key, st: t.EditorState(Nil), undo_c, redo_c, locked_c, blocked_c, action_c) {
  let edits = case plugin_key.get_state(audit_key, st, pm_rt.codec_dynamic()) {
    Some(v) ->
      case decode.run(v, decode.int) {
        Ok(n) -> n
        Error(_) -> 0
      }
    None -> 0
  }
  let undo_stack: List(List(t.Step(Nil))) = get_cell(undo_c)
  let redo_stack: List(List(t.Step(Nil))) = get_cell(redo_c)
  let locked: Bool = get_cell(locked_c)
  let blocked: Int = get_cell(blocked_c)
  let action: String = get_cell(action_c)
  let msg =
    action
    <> " · edits="
    <> int.to_string(edits)
    <> " · undoDepth="
    <> int.to_string(list.length(undo_stack))
    <> " · redoDepth="
    <> int.to_string(list.length(redo_stack))
    <> " · lock="
    <> case locked {
      True -> "LOCKED"
      False -> "OPEN"
    }
    <> " · blocked="
    <> int.to_string(blocked)
    <> " · blocks="
    <> int.to_string(float.round(node.get_child_count(editor_state.get_doc(st))))
  case document.get_element_by_id(doc, "meta") {
    Some(meta) -> dom_node.set_text_content(meta, Some(msg))
    None -> Nil
  }
  case document.get_element_by_id(doc, "btn-4") {
    Some(btn) ->
      element.set_class_name(btn, case locked {
        True -> "active"
        False -> ""
      })
    None -> Nil
  }
}

fn wire_btn(doc, id: String, action: fn() -> Nil) {
  case document.get_element_by_id(doc, id) {
    Some(btn) -> html_element.add_event_listener_click(btn, fn(_) { action() }, None)
    None -> Nil
  }
}

pub fn main() {
  let doc = core.get_document()
  let sch = build_schema()
  let audit_key = plugin_key.new(Some("audit"))

  let view_c = new_cell(None)
  let locked_c = new_cell(False)
  let blocked_c = new_cell(0)
  let rev_c = new_cell(0)
  let action_c = new_cell("Ready — open write access")
  let undo_c = new_cell([])
  let redo_c = new_cell([])

  let field = state_field.create()
  state_field.set_init(field, fn(_, _) { dynamic.int(0) }, pm_rt.codec_dynamic())
  state_field.set_apply(
    field,
    fn(tr, prev, _, _) {
      let n = case decode.run(prev, decode.int) {
        Ok(k) -> k
        Error(_) -> 0
      }
      case transform.get_doc_changed(tr) {
        True -> dynamic.int(n + 1)
        False -> dynamic.int(n)
      }
    },
    pm_rt.codec_dynamic(),
  )

  let pspec = plugin_spec.create()
  plugin_spec.set_key(pspec, Some(audit_key))
  plugin_spec.set_state(pspec, Some(field))
  plugin_spec.set_filter_transaction(
    pspec,
    Some(fn(tr, _) {
      case get_cell(locked_c) && transform.get_doc_changed(tr) {
        True -> {
          set_cell(blocked_c, get_cell(blocked_c) + 1)
          set_cell(action_c, "Blocked document mutation (filterTransaction=false)")
          False
        }
        False -> True
      }
    }),
  )

  let scfg = editor_state_config.create()
  editor_state_config.set_schema(scfg, Some(sch))
  editor_state_config.set_doc(scfg, Some(make_initial_doc(sch)))
  editor_state_config.set_plugins(scfg, Some([plugin.new(pspec)]))

  let props = direct_editor_props.create()
  direct_editor_props.set_state(props, editor_state.create(scfg))
  direct_editor_props.set_dispatch_transaction(
    props,
    Some(fn(tr) {
      case get_cell(view_c) {
        Some(v) -> {
          let res = editor_state.apply_transaction(editor_view.get_state(v), tr)
          case apply_res.get_transactions(res) {
            [] -> Nil
            _ -> {
              case transform.get_doc_changed(tr) {
                True -> {
                  let steps = transform.get_steps(tr)
                  let docs = transform.get_docs(tr)
                  let inv =
                    list.map2(steps, docs, fn(s, d) { step.invert(s, d) })
                    |> list.reverse
                  case decode.run(transaction.get_meta(tr, t.StringOrPluginOrPluginKeyString("history")), decode.string) {
                    Ok("undo") -> set_cell(redo_c, [inv, ..get_cell(redo_c)])
                    Ok(_) -> set_cell(undo_c, [inv, ..get_cell(undo_c)])
                    Error(_) -> {
                      set_cell(undo_c, [inv, ..get_cell(undo_c)])
                      set_cell(redo_c, [])
                    }
                  }
                }
                False -> Nil
              }
              editor_view.update_state(v, apply_res.get_state(res))
            }
          }
          update_meta(doc, audit_key, editor_view.get_state(v), undo_c, redo_c, locked_c, blocked_c, action_c)
        }
        None -> Nil
      }
    }),
  )

  let assert Some(mount) = document.get_element_by_id(doc, "editor")
  let view = editor_view.new(Some(pm_rt.dynamic(mount)), props)
  set_cell(view_c, Some(view))
  update_meta(doc, audit_key, editor_view.get_state(view), undo_c, redo_c, locked_c, blocked_c, action_c)

  wire_btn(doc, "btn-0", fn() {
    let rev = get_cell(rev_c) + 1
    set_cell(rev_c, rev)
    let st = editor_view.get_state(view)
    let msg = "Revision #" <> int.to_string(rev) <> " recorded via Transform step."
    let tr = editor_state.get_tr(st)
    let _ = transform.insert(tr, fragment.get_size(node.get_content(editor_state.get_doc(st))), t.FragmentOrNodeOrNodeArrayNode(p_node(sch, msg)))
    case get_cell(locked_c) {
      False -> set_cell(action_c, "Appended Revision #" <> int.to_string(rev))
      True -> Nil
    }
    editor_view.get_dispatch(view)(tr)
  })

  wire_btn(doc, "btn-1", fn() {
    let st = editor_view.get_state(view)
    let d = editor_state.get_doc(st)
    case node.get_child_count(d) >=. 2.0 {
      True -> {
        let fp = node.get_node_size(node.child(d, 0.0)) +. 1.0
        let tp = fp +. fragment.get_size(node.get_content(node.child(d, 1.0)))
        case tp >. fp {
          True -> {
            let tr = editor_state.get_tr(st)
            let _ = transform.add_mark(tr, fp, tp, schema.mark(sch, t.StringOrMarkTypeString("strong"), None))
            case get_cell(locked_c) {
              False -> set_cell(action_c, "Bolded first body paragraph")
              True -> Nil
            }
            editor_view.get_dispatch(view)(tr)
          }
          False -> Nil
        }
      }
      False -> Nil
    }
  })

  wire_btn(doc, "btn-2", fn() {
    let st = editor_view.get_state(view)
    case get_cell(undo_c) {
      [] -> {
        set_cell(action_c, "Undo stack empty")
        update_meta(doc, audit_key, st, undo_c, redo_c, locked_c, blocked_c, action_c)
      }
      [batch, ..rest] -> {
        case get_cell(locked_c) {
          False -> {
            set_cell(undo_c, rest)
            set_cell(action_c, "Undid " <> int.to_string(list.length(batch)) <> " step(s) via Step.invert")
          }
          True -> Nil
        }
        let tr = editor_state.get_tr(st)
        let _ = transaction.set_meta(tr, t.StringOrPluginOrPluginKeyString("history"), dynamic.string("undo"))
        list.each(batch, fn(s) {
          let _ = transform.step(tr, s)
          Nil
        })
        editor_view.get_dispatch(view)(tr)
      }
    }
  })

  wire_btn(doc, "btn-3", fn() {
    let st = editor_view.get_state(view)
    case get_cell(redo_c) {
      [] -> {
        set_cell(action_c, "Redo stack empty")
        update_meta(doc, audit_key, st, undo_c, redo_c, locked_c, blocked_c, action_c)
      }
      [batch, ..rest] -> {
        case get_cell(locked_c) {
          False -> {
            set_cell(redo_c, rest)
            set_cell(action_c, "Redid " <> int.to_string(list.length(batch)) <> " inverted step(s)")
          }
          True -> Nil
        }
        let tr = editor_state.get_tr(st)
        let _ = transaction.set_meta(tr, t.StringOrPluginOrPluginKeyString("history"), dynamic.string("redo"))
        list.each(batch, fn(s) {
          let _ = transform.step(tr, s)
          Nil
        })
        editor_view.get_dispatch(view)(tr)
      }
    }
  })

  wire_btn(doc, "btn-4", fn() {
    let next_lock = !get_cell(locked_c)
    set_cell(locked_c, next_lock)
    set_cell(action_c, case next_lock {
      True -> "Enabled read-only lock"
      False -> "Disabled read-only lock"
    })
    update_meta(doc, audit_key, editor_view.get_state(view), undo_c, redo_c, locked_c, blocked_c, action_c)
  })
}
