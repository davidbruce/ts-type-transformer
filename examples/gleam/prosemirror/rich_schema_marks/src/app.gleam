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
import gleam/string
import prosemirror/areas/main/mark_spec_attrs
import prosemirror/areas/main/schema_marks
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

  let para_spec = node_spec.create()
  node_spec.set_content(para_spec, Some("inline*"))
  node_spec.set_group(para_spec, Some("block"))
  node_spec.set_to_dom(para_spec, Some(fn(_) { hole_spec("p") }))

  let quote_spec = node_spec.create()
  node_spec.set_content(quote_spec, Some("block+"))
  node_spec.set_group(quote_spec, Some("block"))
  node_spec.set_defining(quote_spec, Some(True))
  node_spec.set_to_dom(quote_spec, Some(fn(_) { hole_spec("blockquote") }))

  let cb_spec = node_spec.create()
  node_spec.set_content(cb_spec, Some("text*"))
  node_spec.set_marks(cb_spec, Some(""))
  node_spec.set_group(cb_spec, Some("block"))
  node_spec.set_code(cb_spec, Some(True))
  node_spec.set_defining(cb_spec, Some(True))
  node_spec.set_to_dom(
    cb_spec,
    Some(fn(_) {
      t.DOMOutputSpecArray([
        dynamic.string("pre"),
        dynamic.array([dynamic.string("code"), dynamic.int(0)]),
      ])
    }),
  )

  let hr_spec = node_spec.create()
  node_spec.set_group(hr_spec, Some("block"))
  node_spec.set_atom(hr_spec, Some(True))
  node_spec.set_to_dom(hr_spec, Some(fn(_) { t.DOMOutputSpecArray([dynamic.string("hr")]) }))

  let text_spec = node_spec.create()
  node_spec.set_group(text_spec, Some("inline"))

  let nodes = node_spec.create()
  node_spec.set_at(nodes, "doc", pm_rt.dynamic(doc_spec))
  node_spec.set_at(nodes, "heading", pm_rt.dynamic(heading_spec))
  node_spec.set_at(nodes, "paragraph", pm_rt.dynamic(para_spec))
  node_spec.set_at(nodes, "blockquote", pm_rt.dynamic(quote_spec))
  node_spec.set_at(nodes, "code_block", pm_rt.dynamic(cb_spec))
  node_spec.set_at(nodes, "horizontal_rule", pm_rt.dynamic(hr_spec))
  node_spec.set_at(nodes, "text", pm_rt.dynamic(text_spec))

  let make_mark = fn(tag: String) {
    let m = mark_spec.create()
    mark_spec.set_to_dom(m, Some(fn(_, _) { hole_spec(tag) }))
    m
  }
  let marks = mark_spec.create()
  mark_spec.set_at(marks, "strong", pm_rt.dynamic(make_mark("strong")))
  mark_spec.set_at(marks, "em", pm_rt.dynamic(make_mark("em")))
  mark_spec.set_at(marks, "code", pm_rt.dynamic(make_mark("code")))

  let spec = schema_spec.create()
  schema_spec.set_nodes(spec, pm_rt.dynamic(nodes))
  schema_spec.set_marks(spec, pm_rt.dynamic(marks))
  schema.new(spec)
}

fn make_initial_doc(sch: t.Schema(Dynamic, Dynamic, Nil)) -> t.Node(Nil) {
  let ha = attrs.create()
  attrs.set_at(ha, "level", dynamic.int(2))
  let h =
    schema.node(
      sch,
      t.StringOrNodeTypeString("heading"),
      Some(ha),
      Some(t.FragmentOrNodeOrNodeArrayNode(schema.text(sch, "ProseMirror Document Schema", None))),
      None,
    )
  let ms = schema.mark(sch, t.StringOrMarkTypeString("strong"), None)
  let me = schema.mark(sch, t.StringOrMarkTypeString("em"), None)
  let mc = schema.mark(sch, t.StringOrMarkTypeString("code"), None)
  let p1 =
    schema.node(
      sch,
      t.StringOrNodeTypeString("paragraph"),
      None,
      Some(
        t.FragmentOrNodeOrNodeArrayArray([
          schema.text(sch, "Inline marks compose cleanly: ", None),
          schema.text(sch, "bold", Some([ms])),
          schema.text(sch, ", ", None),
          schema.text(sch, "italic", Some([me])),
          schema.text(sch, ", and ", None),
          schema.text(sch, "inline_code", Some([mc])),
          schema.text(sch, " spans.", None),
        ]),
      ),
      None,
    )
  let q_inner =
    schema.node(
      sch,
      t.StringOrNodeTypeString("paragraph"),
      None,
      Some(t.FragmentOrNodeOrNodeArrayNode(schema.text(sch, "Every document conforms to a strict schema enforced on every transaction.", None))),
      None,
    )
  let quote = schema.node(sch, t.StringOrNodeTypeString("blockquote"), None, Some(t.FragmentOrNodeOrNodeArrayNode(q_inner)), None)
  let cb =
    schema.node(
      sch,
      t.StringOrNodeTypeString("code_block"),
      None,
      Some(t.FragmentOrNodeOrNodeArrayNode(schema.text(sch, "schema.node(\"paragraph\", null, [schema.text(\"typed\")])", None))),
      None,
    )
  let outro =
    schema.node(
      sch,
      t.StringOrNodeTypeString("paragraph"),
      None,
      Some(
        t.FragmentOrNodeOrNodeArrayNode(
          schema.text(sch, "Select any text in this editor and click a toolbar action to toggle marks or switch block types.", None),
        ),
      ),
      None,
    )
  schema.node(
    sch,
    t.StringOrNodeTypeString("doc"),
    None,
    Some(t.FragmentOrNodeOrNodeArrayArray([h, p1, quote, cb, outro])),
    None,
  )
}

fn make_state(sch: t.Schema(Dynamic, Dynamic, Nil)) -> t.EditorState(Nil) {
  let cfg = editor_state_config.create()
  editor_state_config.set_schema(cfg, Some(sch))
  editor_state_config.set_doc(cfg, Some(make_initial_doc(sch)))
  editor_state.create(cfg)
}

fn target_range(st: t.EditorState(Nil)) -> #(Float, Float) {
  let sel = editor_state.get_selection(st)
  case selection.get_empty(sel) {
    False -> #(selection.get_from(sel), selection.get_to(sel))
    True -> {
      let d = editor_state.get_doc(st)
      case node.get_child_count(d) >=. 2.0 {
        True -> {
          let c0 = node.child(d, 0.0)
          let c1 = node.child(d, 1.0)
          let start_pos = node.get_node_size(c0) +. 1.0
          let len = fragment.get_size(node.get_content(c1))
          case len >. 0.0 {
            True -> #(start_pos, start_pos +. float.min(len, 12.0))
            False -> #(1.0, float.min(fragment.get_size(node.get_content(d)), 2.0))
          }
        }
        False -> #(1.0, float.min(fragment.get_size(node.get_content(d)), 2.0))
      }
    }
  }
}

fn set_btn_active(doc, id: String, on: Bool) {
  case document.get_element_by_id(doc, id) {
    Some(el) ->
      element.set_class_name(el, case on {
        True -> "active"
        False -> ""
      })
    None -> Nil
  }
}

fn update_meta(sch, doc, st: t.EditorState(Nil), dispatches_c, action_c) {
  let #(f, t) = target_range(st)
  let d = editor_state.get_doc(st)
  let marks = schema.get_marks(sch)
  let assert Some(ms) = schema_marks.get_at(marks, "strong")
  let assert Some(me) = schema_marks.get_at(marks, "em")
  let assert Some(mc) = schema_marks.get_at(marks, "code")
  let hb = node.range_has_mark(d, f, t, t.MarkOrMarkTypeMarkType(ms))
  let he = node.range_has_mark(d, f, t, t.MarkOrMarkTypeMarkType(me))
  let hc = node.range_has_mark(d, f, t, t.MarkOrMarkTypeMarkType(mc))

  set_btn_active(doc, "btn-0", hb)
  set_btn_active(doc, "btn-1", he)
  set_btn_active(doc, "btn-2", hc)

  let bname = node_type.get_name(node.get_type(resolved_pos.get_parent(selection.get_from_new(editor_state.get_selection(st)))))
  let active =
    [
      case hb {
        True -> "strong"
        False -> ""
      },
      case he {
        True -> "em"
        False -> ""
      },
      case hc {
        True -> "code"
        False -> ""
      },
    ]
    |> list.filter(fn(s) { s != "" })
  let mtxt = case active {
    [] -> "none"
    _ -> string.join(active, ", ")
  }
  let action: String = get_cell(action_c)
  let dispatches: Int = get_cell(dispatches_c)
  let msg =
    action
    <> " · blocks="
    <> int.to_string(float.round(node.get_child_count(d)))
    <> " · nodeSize="
    <> int.to_string(float.round(node.get_node_size(d)))
    <> " · range=["
    <> int.to_string(float.round(f))
    <> ".."
    <> int.to_string(float.round(t))
    <> "] · block="
    <> bname
    <> " · marks=["
    <> mtxt
    <> "] · dispatches="
    <> int.to_string(dispatches)
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

pub fn main() {
  let doc = core.get_document()
  let sch = build_schema()
  let view_c = new_cell(None)
  let dispatches_c = new_cell(0)
  let action_c = new_cell("Initialized schema with 7 node types & 3 marks")

  let props = direct_editor_props.create()
  direct_editor_props.set_state(props, make_state(sch))
  direct_editor_props.set_dispatch_transaction(
    props,
    Some(fn(tr) {
      case get_cell(view_c) {
        Some(v) -> {
          set_cell(dispatches_c, get_cell(dispatches_c) + 1)
          let next = editor_state.apply(editor_view.get_state(v), tr)
          editor_view.update_state(v, next)
          update_meta(sch, doc, next, dispatches_c, action_c)
        }
        None -> Nil
      }
    }),
  )

  let assert Some(mount) = document.get_element_by_id(doc, "editor")
  let view = editor_view.new(Some(pm_rt.dynamic(mount)), props)
  set_cell(view_c, Some(view))
  update_meta(sch, doc, editor_view.get_state(view), dispatches_c, action_c)

  let toggle_mark = fn(mkey: String, label: String) {
    let st = editor_view.get_state(view)
    let #(f, t) = target_range(st)
    case t >. f {
      True -> {
        let assert Some(mt) = schema_marks.get_at(schema.get_marks(sch), mkey)
        let tr = editor_state.get_tr(st)
        case node.range_has_mark(editor_state.get_doc(st), f, t, t.MarkOrMarkTypeMarkType(mt)) {
          True -> {
            let _ = transform.remove_mark(tr, f, t, Some(t.MarkOrMarkTypeMarkType(mt)))
            set_cell(action_c, "Removed " <> label)
          }
          False -> {
            let m = schema.mark(sch, t.StringOrMarkTypeMarkType(mt), None)
            let _ = transform.add_mark(tr, f, t, m)
            set_cell(action_c, "Added " <> label)
          }
        }
        editor_view.get_dispatch(view)(tr)
      }
      False -> Nil
    }
  }

  wire_btn(doc, "btn-0", fn() { toggle_mark("strong", "bold") })
  wire_btn(doc, "btn-1", fn() { toggle_mark("em", "italic") })
  wire_btn(doc, "btn-2", fn() { toggle_mark("code", "code") })
  wire_btn(doc, "btn-3", fn() {
    let st = editor_view.get_state(view)
    let sel = editor_state.get_selection(st)
    let pname = node_type.get_name(node.get_type(resolved_pos.get_parent(selection.get_from_new(sel))))
    let tr = editor_state.get_tr(st)
    let nodes = schema.get_nodes(sch)
    case pname == "heading" {
      True -> {
        let assert Some(pt) = schema_nodes.get_at(nodes, "paragraph")
        let _ = transform.set_block_type(tr, selection.get_from(sel), Some(selection.get_to(sel)), pt, None)
        set_cell(action_c, "Converted heading -> paragraph")
      }
      False -> {
        let assert Some(ht) = schema_nodes.get_at(nodes, "heading")
        let ha = attrs.create()
        attrs.set_at(ha, "level", dynamic.int(2))
        let _ = transform.set_block_type(tr, selection.get_from(sel), Some(selection.get_to(sel)), ht, Some(pm_rt.dynamic(ha)))
        set_cell(action_c, "Converted block -> heading[level=2]")
      }
    }
    editor_view.get_dispatch(view)(tr)
  })
  wire_btn(doc, "btn-4", fn() {
    set_cell(dispatches_c, 0)
    set_cell(action_c, "Reset initial schema document")
    let fresh = make_state(sch)
    editor_view.update_state(view, fresh)
    update_meta(sch, doc, fresh, dispatches_c, action_c)
  })
}
