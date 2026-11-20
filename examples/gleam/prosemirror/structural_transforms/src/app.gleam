import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node as dom_node
import dom/areas/html/html_element
import gleam/dynamic.{type Dynamic}
import gleam/dynamic/decode
import gleam/float
import gleam/int
import gleam/list
import gleam/option.{None, Some}
import gleam/string
import prosemirror/areas/main/mark_spec_attrs
import prosemirror/areas/prosemirror_model/node_range
import prosemirror/areas/main/schema_nodes
import prosemirror/areas/prosemirror_model/attribute_spec
import prosemirror/areas/prosemirror_model/attrs
import prosemirror/areas/prosemirror_model/fragment
import prosemirror/areas/prosemirror_model/mark_spec
import prosemirror/areas/prosemirror_model/node
import prosemirror/areas/prosemirror_model/node_spec
import prosemirror/areas/prosemirror_model/node_type
import prosemirror/areas/prosemirror_model/resolved_pos
import prosemirror/areas/prosemirror_model/schema
import prosemirror/areas/prosemirror_model/schema_spec
import prosemirror/areas/prosemirror_state/editor_state
import prosemirror/areas/prosemirror_state/editor_state_config
import prosemirror/areas/prosemirror_state/selection
import prosemirror/areas/prosemirror_transform
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
  node_spec.set_defining(heading_spec, Some(True))
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

  let tag_node = fn(content: String, grp: option.Option(String), tag: String, def: Bool) {
    let s = node_spec.create()
    node_spec.set_content(s, Some(content))
    case grp {
      Some(g) -> node_spec.set_group(s, Some(g))
      None -> Nil
    }
    case def {
      True -> node_spec.set_defining(s, Some(True))
      False -> Nil
    }
    node_spec.set_to_dom(s, Some(fn(_) { hole_spec(tag) }))
    s
  }

  let hr_spec = node_spec.create()
  node_spec.set_group(hr_spec, Some("block"))
  node_spec.set_atom(hr_spec, Some(True))
  node_spec.set_to_dom(hr_spec, Some(fn(_) { t.DOMOutputSpecArray([dynamic.string("hr")]) }))

  let text_spec = node_spec.create()
  node_spec.set_group(text_spec, Some("inline"))

  let nodes = node_spec.create()
  node_spec.set_at(nodes, "doc", pm_rt.dynamic(doc_spec))
  node_spec.set_at(nodes, "heading", pm_rt.dynamic(heading_spec))
  node_spec.set_at(nodes, "paragraph", pm_rt.dynamic(tag_node("inline*", Some("block"), "p", False)))
  node_spec.set_at(nodes, "blockquote", pm_rt.dynamic(tag_node("block+", Some("block"), "blockquote", True)))
  node_spec.set_at(nodes, "bullet_list", pm_rt.dynamic(tag_node("list_item+", Some("block"), "ul", False)))
  node_spec.set_at(nodes, "list_item", pm_rt.dynamic(tag_node("paragraph block*", None, "li", True)))
  node_spec.set_at(nodes, "horizontal_rule", pm_rt.dynamic(hr_spec))
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

fn li_node(sch, txt: String) -> t.Node(Nil) {
  schema.node(sch, t.StringOrNodeTypeString("list_item"), None, Some(t.FragmentOrNodeOrNodeArrayNode(p_node(sch, txt))), None)
}

fn make_initial_doc(sch: t.Schema(Dynamic, Dynamic, Nil)) -> t.Node(Nil) {
  let ha = attrs.create()
  attrs.set_at(ha, "level", dynamic.int(2))
  let h =
    schema.node(
      sch,
      t.StringOrNodeTypeString("heading"),
      Some(ha),
      Some(t.FragmentOrNodeOrNodeArrayNode(schema.text(sch, "Structural Transform Pipeline", None))),
      None,
    )
  let quote = schema.node(
    sch,
    t.StringOrNodeTypeString("blockquote"),
    None,
    Some(t.FragmentOrNodeOrNodeArrayNode(p_node(sch, "Blockquote content can be lifted back to top-level document depth."))),
    None,
  )
  let ulist =
    schema.node(
      sch,
      t.StringOrNodeTypeString("bullet_list"),
      None,
      Some(
        t.FragmentOrNodeOrNodeArrayArray([
          li_node(sch, "Split at cursor position"),
          li_node(sch, "Wrap range with blockquote"),
          li_node(sch, "Lift nested block to root"),
        ]),
      ),
      None,
    )
  schema.node(
    sch,
    t.StringOrNodeTypeString("doc"),
    None,
    Some(
      t.FragmentOrNodeOrNodeArrayArray([
        h,
        p_node(sch, "ProseMirror transforms express structural edits as invertible steps."),
        p_node(sch, "Wrap blocks in quotes, lift nested content, or split paragraphs cleanly."),
        quote,
        ulist,
      ]),
    ),
    None,
  )
}

fn make_state(sch: t.Schema(Dynamic, Dynamic, Nil)) -> t.EditorState(Nil) {
  let cfg = editor_state_config.create()
  editor_state_config.set_schema(cfg, Some(sch))
  editor_state_config.set_doc(cfg, Some(make_initial_doc(sch)))
  editor_state.create(cfg)
}

fn update_meta(doc, d: t.Node(Nil), steps_c, op_c) {
  let names = list.map(node.get_children(d), fn(b) { node_type.get_name(node.get_type(b)) })
  let op: String = get_cell(op_c)
  let steps: Int = get_cell(steps_c)
  let msg =
    op
    <> " · blocks="
    <> int.to_string(list.length(names))
    <> " · outline=["
    <> string.join(names, " · ")
    <> "] · totalSteps="
    <> int.to_string(steps)
    <> " · nodeSize="
    <> int.to_string(float.round(node.get_node_size(d)))
  case document.get_element_by_id(doc, "meta") {
    Some(meta) -> dom_node.set_text_content(meta, Some(msg))
    None -> Nil
  }
}

fn wire_btn(doc, id: String, action: fn() -> Nil) {
  case document.get_element_by_id(doc, id) {
    Some(btn) -> html_element.add_event_listener_click(btn, fn(_) { action() }, None)
    None -> Nil
  }
}

fn find_para_split(children: List(t.Node(Nil)), d: t.Node(Nil), pos: Float) -> option.Option(Float) {
  case children {
    [] -> None
    [ch, ..rest] ->
      case node_type.get_name(node.get_type(ch)) == "paragraph" {
        True -> {
          let len = fragment.get_size(node.get_content(ch))
          case len >=. 8.0 {
            True -> {
              let mid = pos +. 1.0 +. int.to_float(float.round(len) / 2)
              case prosemirror_transform.can_split(d, mid, None, None) {
                True -> Some(mid)
                False -> find_para_split(rest, d, pos +. node.get_node_size(ch))
              }
            }
            False -> find_para_split(rest, d, pos +. node.get_node_size(ch))
          }
        }
        False -> find_para_split(rest, d, pos +. node.get_node_size(ch))
      }
  }
}

fn find_para_wrap(children: List(t.Node(Nil)), d: t.Node(Nil), qtype, pos: Float) -> option.Option(t.NodeRange(Nil)) {
  case children {
    [] -> None
    [ch, ..rest] ->
      case node_type.get_name(node.get_type(ch)) == "paragraph" {
        True -> {
          let rp = node.resolve(d, pos +. 1.0)
          case resolved_pos.block_range(rp, None, None) {
            Some(r) ->
              case prosemirror_transform.find_wrapping(r, qtype, None, None) {
                Some(_) -> Some(r)
                None -> find_para_wrap(rest, d, qtype, pos +. node.get_node_size(ch))
              }
            None -> find_para_wrap(rest, d, qtype, pos +. node.get_node_size(ch))
          }
        }
        False -> find_para_wrap(rest, d, qtype, pos +. node.get_node_size(ch))
      }
  }
}

fn find_quote_lift(children: List(t.Node(Nil)), d: t.Node(Nil), pos: Float) -> option.Option(#(t.NodeRange(Nil), Float)) {
  case children {
    [] -> None
    [ch, ..rest] ->
      case node_type.get_name(node.get_type(ch)) == "blockquote" && node.get_child_count(ch) >. 0.0 {
        True -> {
          let rp = node.resolve(d, pos +. 2.0)
          case resolved_pos.block_range(rp, None, None) {
            Some(r) ->
              case prosemirror_transform.lift_target(r) {
                Some(tgt) -> Some(#(r, tgt))
                None -> find_quote_lift(rest, d, pos +. node.get_node_size(ch))
              }
            None -> find_quote_lift(rest, d, pos +. node.get_node_size(ch))
          }
        }
        False -> find_quote_lift(rest, d, pos +. node.get_node_size(ch))
      }
  }
}

pub fn main() {
  let doc = core.get_document()
  let sch = build_schema()
  let view_c = new_cell(None)
  let steps_c = new_cell(0)
  let notes_c = new_cell(0)
  let op_c = new_cell("Initialized document structure")

  let props = direct_editor_props.create()
  direct_editor_props.set_state(props, make_state(sch))
  direct_editor_props.set_dispatch_transaction(
    props,
    Some(fn(tr) {
      case get_cell(view_c) {
        Some(v) -> {
          set_cell(steps_c, get_cell(steps_c) + list.length(transform.get_steps(tr)))
          let next = editor_state.apply(editor_view.get_state(v), tr)
          editor_view.update_state(v, next)
          update_meta(doc, editor_state.get_doc(next), steps_c, op_c)
        }
        None -> Nil
      }
    }),
  )

  let assert Some(mount) = document.get_element_by_id(doc, "editor")
  let view = editor_view.new(Some(pm_rt.dynamic(mount)), props)
  set_cell(view_c, Some(view))
  update_meta(doc, editor_state.get_doc(editor_view.get_state(view)), steps_c, op_c)

  wire_btn(doc, "btn-0", fn() {
    let st = editor_view.get_state(view)
    let d = editor_state.get_doc(st)
    let sel_from = selection.get_from(editor_state.get_selection(st))
    let sp = case prosemirror_transform.can_split(d, sel_from, None, None) {
      True -> Some(sel_from)
      False -> find_para_split(node.get_children(d), d, 0.0)
    }
    case sp {
      Some(p) -> {
        let tr = editor_state.get_tr(st)
        let _ = transform.split(tr, p, None, None)
        set_cell(op_c, "Split paragraph at pos " <> int.to_string(float.round(p)))
        editor_view.get_dispatch(view)(tr)
      }
      None -> {
        set_cell(op_c, "No splittable paragraph found")
        update_meta(doc, d, steps_c, op_c)
      }
    }
  })

  wire_btn(doc, "btn-1", fn() {
    let st = editor_view.get_state(view)
    let d = editor_state.get_doc(st)
    let assert Some(qtype) = schema_nodes.get_at(schema.get_nodes(sch), "blockquote")
    let sel = editor_state.get_selection(st)
    let sf = selection.get_from_new(sel)
    let from_sel = case resolved_pos.get_depth(sf) == 1.0 && node_type.get_name(node.get_type(resolved_pos.get_parent(sf))) == "paragraph" {
      True ->
        case resolved_pos.block_range(sf, Some(selection.get_to_new(sel)), None) {
          Some(r) ->
            case prosemirror_transform.find_wrapping(r, qtype, None, None) {
              Some(_) -> Some(r)
              None -> None
            }
          None -> None
        }
      False -> None
    }
    let found = case from_sel {
      Some(r) -> Some(r)
      None -> find_para_wrap(node.get_children(d), d, qtype, 0.0)
    }
    case found {
      Some(r) ->
        case prosemirror_transform.find_wrapping(r, qtype, None, None) {
          Some(wrapped) -> {
            let tr = editor_state.get_tr(st)
            let _ = transform.wrap(tr, r, wrapped)
            set_cell(op_c, "Wrapped block [" <> int.to_string(float.round(node_range.get_start(r))) <> ".." <> int.to_string(float.round(node_range.get_end(r))) <> "] in blockquote")
            editor_view.get_dispatch(view)(tr)
          }
          None -> Nil
        }
      None -> {
        set_cell(op_c, "No top-level paragraph left to wrap")
        update_meta(doc, d, steps_c, op_c)
      }
    }
  })

  wire_btn(doc, "btn-2", fn() {
    let st = editor_view.get_state(view)
    let d = editor_state.get_doc(st)
    case find_quote_lift(node.get_children(d), d, 0.0) {
      Some(#(r, target)) -> {
        let tr = editor_state.get_tr(st)
        let _ = transform.lift(tr, r, target)
        set_cell(op_c, "Lifted blockquote child to target depth " <> int.to_string(float.round(target)))
        editor_view.get_dispatch(view)(tr)
      }
      None -> {
        set_cell(op_c, "No blockquote left to lift")
        update_meta(doc, d, steps_c, op_c)
      }
    }
  })

  wire_btn(doc, "btn-3", fn() {
    let st = editor_view.get_state(view)
    let d = editor_state.get_doc(st)
    let n = get_cell(notes_c) + 1
    set_cell(notes_c, n)
    let sz = fragment.get_size(node.get_content(d))
    let hr = schema.node(sch, t.StringOrNodeTypeString("horizontal_rule"), None, None, None)
    let note = p_node(sch, "Appended structural section #" <> int.to_string(n) <> " at doc offset " <> int.to_string(float.round(sz)) <> ".")
    let tr = editor_state.get_tr(st)
    let _ = transform.insert(tr, sz, t.FragmentOrNodeOrNodeArrayArray([hr, note]))
    set_cell(op_c, "Inserted horizontal_rule + note #" <> int.to_string(n))
    editor_view.get_dispatch(view)(tr)
  })

  wire_btn(doc, "btn-4", fn() {
    set_cell(steps_c, 0)
    set_cell(notes_c, 0)
    set_cell(op_c, "Reset structural sample")
    let fresh = make_state(sch)
    editor_view.update_state(view, fresh)
    update_meta(doc, editor_state.get_doc(fresh), steps_c, op_c)
  })
}
