import dom/areas/core
import dom/areas/core/document
import dom/areas/core/element
import dom/areas/core/node as dom_node
import dom/areas/html/html_element
import dom/areas/html/html_element_event_map as ev
import gleam/dynamic.{type Dynamic}
import gleam/dynamic/decode
import gleam/float
import gleam/int
import gleam/list
import gleam/option.{None, Some}
import prosemirror/mark_spec_attrs
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

fn hole_spec(tag: String) -> t.DOMOutputSpec {
  t.DOMOutputSpecArray([dynamic.string(tag), dynamic.int(0)])
}

fn build_schema() -> t.Schema(Dynamic, Dynamic, Nil) {
  let doc_spec =
    node_spec.create()
    |> node_spec.with_content("block+")

  let lvl_spec =
    attribute_spec.create()
    |> attribute_spec.with_default(dynamic.int(2))
  let h_attrs =
    mark_spec_attrs.create()
    |> mark_spec_attrs.with_at("level", lvl_spec)

  let heading_spec =
    node_spec.create()
    |> node_spec.with_attrs(h_attrs)
    |> node_spec.with_content("inline*")
    |> node_spec.with_group("block")
    |> node_spec.with_to_dom(fn(n) {
      let lvl = case decode.run(attrs.get_at(node.get_attrs(n), "level"), decode.int) {
        Ok(1) -> "h1"
        _ -> "h2"
      }
      hole_spec(lvl)
    })

  let para_spec =
    node_spec.create()
    |> node_spec.with_content("inline*")
    |> node_spec.with_group("block")
    |> node_spec.with_to_dom(fn(_) { hole_spec("p") })

  let text_spec =
    node_spec.create()
    |> node_spec.with_group("inline")

  let nodes =
    node_spec.create()
    |> node_spec.with_at("doc", pm_rt.dynamic(doc_spec))
    |> node_spec.with_at("heading", pm_rt.dynamic(heading_spec))
    |> node_spec.with_at("paragraph", pm_rt.dynamic(para_spec))
    |> node_spec.with_at("text", pm_rt.dynamic(text_spec))

  let make_mark = fn(tag: String) {
    mark_spec.create()
    |> mark_spec.with_to_dom(fn(_, _) { hole_spec(tag) })
  }
  let marks =
    mark_spec.create()
    |> mark_spec.with_at("strong", pm_rt.dynamic(make_mark("strong")))
    |> mark_spec.with_at("code", pm_rt.dynamic(make_mark("code")))

  let spec =
    schema_spec.create()
    |> schema_spec.with_nodes(pm_rt.dynamic(nodes))
    |> schema_spec.with_marks(pm_rt.dynamic(marks))
  schema.new(spec)
}

fn p_node(sch, txt: String) -> t.Node(Nil) {
  schema.node(sch, t.StringOrNodeTypeString("paragraph"), None, Some(t.FragmentOrNodeOrNodeArrayNode(schema.text(sch, txt, None))), None)
}

fn make_initial_doc(sch: t.Schema(Dynamic, Dynamic, Nil)) -> t.Node(Nil) {
  let ha =
    attrs.create()
    |> attrs.with_at("level", dynamic.int(2))
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

fn update_meta(
  doc,
  audit_key,
  st: t.EditorState(Nil),
  undo_ref: pm_rt.Ref(List(List(t.Step(Nil)))),
  redo_ref: pm_rt.Ref(List(List(t.Step(Nil)))),
  locked_ref: pm_rt.Ref(Bool),
  blocked_ref: pm_rt.Ref(Int),
  action_ref: pm_rt.Ref(String),
) {
  let edits = case plugin_key.get_state(audit_key, st, pm_rt.codec_dynamic()) {
    Some(v) ->
      case decode.run(v, decode.int) {
        Ok(n) -> n
        Error(_) -> 0
      }
    None -> 0
  }
  let undo_stack = pm_rt.ref_get(undo_ref)
  let redo_stack = pm_rt.ref_get(redo_ref)
  let locked = pm_rt.ref_get(locked_ref)
  let blocked = pm_rt.ref_get(blocked_ref)
  let action = pm_rt.ref_get(action_ref)
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
    Some(btn) -> html_element.add_event_listener(btn, ev.click, fn(_) { action() }, None)
    None -> Nil
  }
}

pub fn main() {
  let doc = core.get_document()
  let sch = build_schema()
  let audit_key = plugin_key.new(Some("audit"))

  let view_ref = pm_rt.ref_new(None)
  let locked_ref = pm_rt.ref_new(False)
  let blocked_ref = pm_rt.ref_new(0)
  let rev_ref = pm_rt.ref_new(0)
  let action_ref = pm_rt.ref_new("Ready — open write access")
  let undo_ref: pm_rt.Ref(List(List(t.Step(Nil)))) = pm_rt.ref_new([])
  let redo_ref: pm_rt.Ref(List(List(t.Step(Nil)))) = pm_rt.ref_new([])

  let field =
    state_field.create()
    |> state_field.with_init(fn(_, _) { dynamic.int(0) }, pm_rt.codec_dynamic())
    |> state_field.with_apply(
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

  let pspec =
    plugin_spec.create()
    |> plugin_spec.with_key(audit_key)
    |> plugin_spec.with_state(field)
    |> plugin_spec.with_filter_transaction(fn(tr, _) {
      case pm_rt.ref_get(locked_ref) && transform.get_doc_changed(tr) {
        True -> {
          pm_rt.ref_update(blocked_ref, fn(b) { b + 1 })
          pm_rt.ref_set(action_ref, "Blocked document mutation (filterTransaction=false)")
          False
        }
        False -> True
      }
    })

  let scfg =
    editor_state_config.create()
    |> editor_state_config.with_schema(sch)
    |> editor_state_config.with_doc(make_initial_doc(sch))
    |> editor_state_config.with_plugins([plugin.new(pspec)])

  let props =
    direct_editor_props.create()
    |> direct_editor_props.with_state(editor_state.create(scfg))
    |> direct_editor_props.with_dispatch_transaction(fn(tr) {
      case pm_rt.ref_get(view_ref) {
        Some(v) -> {
          let res = editor_state.apply_transaction(editor_view.get_state(v), tr)
          case editor_state.apply_transaction_result_get_transactions(res) {
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
                    Ok("undo") -> {
                      pm_rt.ref_update(redo_ref, fn(stack) { [inv, ..stack] })
                      Nil
                    }
                    Ok(_) -> {
                      pm_rt.ref_update(undo_ref, fn(stack) { [inv, ..stack] })
                      Nil
                    }
                    Error(_) -> {
                      pm_rt.ref_update(undo_ref, fn(stack) { [inv, ..stack] })
                      pm_rt.ref_set(redo_ref, [])
                    }
                  }
                }
                False -> Nil
              }
              editor_view.update_state(v, editor_state.apply_transaction_result_get_state(res))
            }
          }
          update_meta(doc, audit_key, editor_view.get_state(v), undo_ref, redo_ref, locked_ref, blocked_ref, action_ref)
        }
        None -> Nil
      }
    })

  let assert Some(mount) = document.get_element_by_id(doc, "editor")
  let view = editor_view.new(Some(t.EditorViewNewPlaceOrDistDOMNodeNode2(dom_node.upcast(mount))), props)
  pm_rt.ref_set(view_ref, Some(view))
  update_meta(doc, audit_key, editor_view.get_state(view), undo_ref, redo_ref, locked_ref, blocked_ref, action_ref)

  wire_btn(doc, "btn-0", fn() {
    let rev = pm_rt.ref_update(rev_ref, fn(r) { r + 1 })
    let st = editor_view.get_state(view)
    let msg = "Revision #" <> int.to_string(rev) <> " recorded via Transform step."
    let tr =
      editor_state.get_tr(st)
      |> transform.insert(fragment.get_size(node.get_content(editor_state.get_doc(st))), t.FragmentOrNodeOrNodeArrayNode(p_node(sch, msg)))
    case pm_rt.ref_get(locked_ref) {
      False -> pm_rt.ref_set(action_ref, "Appended Revision #" <> int.to_string(rev))
      True -> Nil
    }
    editor_view.dispatch(view, tr)
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
            let tr =
              editor_state.get_tr(st)
              |> transform.add_mark(fp, tp, schema.mark(sch, t.StringOrMarkTypeString("strong"), None))
            case pm_rt.ref_get(locked_ref) {
              False -> pm_rt.ref_set(action_ref, "Bolded first body paragraph")
              True -> Nil
            }
            editor_view.dispatch(view, tr)
          }
          False -> Nil
        }
      }
      False -> Nil
    }
  })

  wire_btn(doc, "btn-2", fn() {
    let st = editor_view.get_state(view)
    case pm_rt.ref_get(undo_ref) {
      [] -> {
        pm_rt.ref_set(action_ref, "Undo stack empty")
        update_meta(doc, audit_key, st, undo_ref, redo_ref, locked_ref, blocked_ref, action_ref)
      }
      [batch, ..rest] -> {
        case pm_rt.ref_get(locked_ref) {
          False -> {
            pm_rt.ref_set(undo_ref, rest)
            pm_rt.ref_set(action_ref, "Undid " <> int.to_string(list.length(batch)) <> " step(s) via Step.invert")
          }
          True -> Nil
        }
        let tr =
          editor_state.get_tr(st)
          |> transaction.set_meta(t.StringOrPluginOrPluginKeyString("history"), dynamic.string("undo"))
        list.each(batch, fn(s) {
          transform.step(tr, s)
          Nil
        })
        editor_view.dispatch(view, tr)
      }
    }
  })

  wire_btn(doc, "btn-3", fn() {
    let st = editor_view.get_state(view)
    case pm_rt.ref_get(redo_ref) {
      [] -> {
        pm_rt.ref_set(action_ref, "Redo stack empty")
        update_meta(doc, audit_key, st, undo_ref, redo_ref, locked_ref, blocked_ref, action_ref)
      }
      [batch, ..rest] -> {
        case pm_rt.ref_get(locked_ref) {
          False -> {
            pm_rt.ref_set(redo_ref, rest)
            pm_rt.ref_set(action_ref, "Redid " <> int.to_string(list.length(batch)) <> " inverted step(s)")
          }
          True -> Nil
        }
        let tr =
          editor_state.get_tr(st)
          |> transaction.set_meta(t.StringOrPluginOrPluginKeyString("history"), dynamic.string("redo"))
        list.each(batch, fn(s) {
          transform.step(tr, s)
          Nil
        })
        editor_view.dispatch(view, tr)
      }
    }
  })

  wire_btn(doc, "btn-4", fn() {
    let next_lock = pm_rt.ref_update(locked_ref, fn(cur) { !cur })
    pm_rt.ref_set(action_ref, case next_lock {
      True -> "Enabled read-only lock"
      False -> "Disabled read-only lock"
    })
    update_meta(doc, audit_key, editor_view.get_state(view), undo_ref, redo_ref, locked_ref, blocked_ref, action_ref)
  })
}
