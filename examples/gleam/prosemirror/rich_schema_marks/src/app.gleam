import dom/areas/core
import dom/areas/core/document
import dom/areas/core/element
import dom/areas/core/node as dom_node
import dom/areas/html/html_element
import dom/areas/html/html_element_event_map as ev
import gleam/dynamic
import gleam/dynamic/decode
import gleam/float
import gleam/int
import gleam/list
import gleam/option.{None, Some}
import gleam/string
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
import prosemirror/mark_spec_attrs
import prosemirror/runtime as pm_rt
import prosemirror/types as t

fn hole_spec(tag: String) -> t.DOMOutputSpec {
  t.DOMOutputSpecArray([dynamic.string(tag), dynamic.int(0)])
}

fn build_schema() -> t.Schema(Nil) {
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
    |> node_spec.with_defining(True)
    |> node_spec.with_to_dom(fn(n) {
      let lvl = case
        decode.run(attrs.get_at(node.get_attrs(n), "level"), decode.int)
      {
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

  let quote_spec =
    node_spec.create()
    |> node_spec.with_content("block+")
    |> node_spec.with_group("block")
    |> node_spec.with_defining(True)
    |> node_spec.with_to_dom(fn(_) { hole_spec("blockquote") })

  let cb_spec =
    node_spec.create()
    |> node_spec.with_content("text*")
    |> node_spec.with_marks("")
    |> node_spec.with_group("block")
    |> node_spec.with_code(True)
    |> node_spec.with_defining(True)
    |> node_spec.with_to_dom(fn(_) {
      t.DOMOutputSpecArray([
        dynamic.string("pre"),
        dynamic.array([dynamic.string("code"), dynamic.int(0)]),
      ])
    })

  let hr_spec =
    node_spec.create()
    |> node_spec.with_group("block")
    |> node_spec.with_atom(True)
    |> node_spec.with_to_dom(fn(_) {
      t.DOMOutputSpecArray([dynamic.string("hr")])
    })

  let text_spec =
    node_spec.create()
    |> node_spec.with_group("inline")

  let nodes =
    schema_spec.nodes_create()
    |> schema_spec.nodes_with_at("doc", doc_spec)
    |> schema_spec.nodes_with_at("heading", heading_spec)
    |> schema_spec.nodes_with_at("paragraph", para_spec)
    |> schema_spec.nodes_with_at("blockquote", quote_spec)
    |> schema_spec.nodes_with_at("code_block", cb_spec)
    |> schema_spec.nodes_with_at("horizontal_rule", hr_spec)
    |> schema_spec.nodes_with_at("text", text_spec)

  let make_mark = fn(tag: String) {
    mark_spec.create()
    |> mark_spec.with_to_dom(fn(_, _) { hole_spec(tag) })
  }
  let marks =
    schema_spec.marks_create()
    |> schema_spec.marks_with_at("strong", make_mark("strong"))
    |> schema_spec.marks_with_at("em", make_mark("em"))
    |> schema_spec.marks_with_at("code", make_mark("code"))

  let spec =
    schema_spec.create()
    |> schema_spec.with_nodes(nodes)
    |> schema_spec.with_marks(marks)
  schema.new(spec)
}

fn make_initial_doc(sch: t.Schema(Nil)) -> t.Node(Nil) {
  let ha =
    attrs.create()
    |> attrs.with_at("level", dynamic.int(2))
  let h =
    schema.node_with(
      sch,
      "heading",
      Some(ha),
      schema.text(sch, "ProseMirror Document Schema"),
      [],
    )
  let ms = schema.mark(sch, "strong")
  let me = schema.mark(sch, "em")
  let mc = schema.mark(sch, "code")
  let p1 =
    schema.node_list_with(
      sch,
      "paragraph",
      None,
      [
        schema.text(sch, "Inline marks compose cleanly: "),
        schema.text_with(sch, "bold", Some([ms])),
        schema.text(sch, ", "),
        schema.text_with(sch, "italic", Some([me])),
        schema.text(sch, ", and "),
        schema.text_with(sch, "inline_code", Some([mc])),
        schema.text(sch, " spans."),
      ],
      [],
    )
  let q_inner =
    schema.node_with(
      sch,
      "paragraph",
      None,
      schema.text(
        sch,
        "Every document conforms to a strict schema enforced on every transaction.",
      ),
      [],
    )
  let quote = schema.node_with(sch, "blockquote", None, q_inner, [])
  let cb =
    schema.node_with(
      sch,
      "code_block",
      None,
      schema.text(
        sch,
        "schema.node(\"paragraph\", null, [schema.text(\"typed\")])",
      ),
      [],
    )
  let outro =
    schema.node_with(
      sch,
      "paragraph",
      None,
      schema.text(
        sch,
        "Select any text in this editor and click a toolbar action to toggle marks or switch block types.",
      ),
      [],
    )
  schema.node_list_with(
    sch,
    "doc",
    None,
    [h, p1, quote, cb, outro],
    [],
  )
}

fn make_state(sch: t.Schema(Nil)) -> t.EditorState(Nil) {
  editor_state_config.create()
  |> editor_state_config.with_schema(sch)
  |> editor_state_config.with_doc(make_initial_doc(sch))
  |> editor_state.create
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
            False -> #(
              1.0,
              float.min(fragment.get_size(node.get_content(d)), 2.0),
            )
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

fn update_meta(
  sch,
  doc,
  st: t.EditorState(Nil),
  dispatches_ref: pm_rt.Ref(Int),
  action_ref: pm_rt.Ref(String),
) {
  let #(f, t) = target_range(st)
  let d = editor_state.get_doc(st)
  let marks = schema.get_marks(sch)
  let assert Some(ms) = schema.marks_get_at(marks, "strong")
  let assert Some(me) = schema.marks_get_at(marks, "em")
  let assert Some(mc) = schema.marks_get_at(marks, "code")
  let hb = node.range_has_mark_type(d, f, t, ms)
  let he = node.range_has_mark_type(d, f, t, me)
  let hc = node.range_has_mark_type(d, f, t, mc)

  set_btn_active(doc, "btn-0", hb)
  set_btn_active(doc, "btn-1", he)
  set_btn_active(doc, "btn-2", hc)

  let bname =
    node_type.get_name(
      node.get_type(
        resolved_pos.get_parent(
          selection.get_dollar_from(editor_state.get_selection(st)),
        ),
      ),
    )
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
  let action = pm_rt.ref_get(action_ref)
  let dispatches = pm_rt.ref_get(dispatches_ref)
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
    Some(btn) ->
      html_element.add_event_listener(btn, ev.click, fn(_) { action() })
    None -> Nil
  }
}

pub fn main() {
  let doc = core.get_document()
  let sch = build_schema()
  let view_ref = pm_rt.ref_new(None)
  let dispatches_ref = pm_rt.ref_new(0)
  let action_ref =
    pm_rt.ref_new("Initialized schema with 7 node types & 3 marks")

  let props =
    direct_editor_props.create()
    |> direct_editor_props.with_state(make_state(sch))
    |> direct_editor_props.with_dispatch_transaction(fn(tr) {
      case pm_rt.ref_get(view_ref) {
        Some(v) -> {
          pm_rt.ref_update(dispatches_ref, fn(n) { n + 1 })
          let next = editor_state.apply(editor_view.get_state(v), tr)
          editor_view.update_state(v, next)
          update_meta(sch, doc, next, dispatches_ref, action_ref)
        }
        None -> Nil
      }
    })

  let assert Some(mount) = document.get_element_by_id(doc, "editor")
  let view = editor_view.new_node2(mount, props)
  pm_rt.ref_set(view_ref, Some(view))
  update_meta(sch, doc, editor_view.get_state(view), dispatches_ref, action_ref)

  let toggle_mark = fn(mkey: String, label: String) {
    let st = editor_view.get_state(view)
    let #(f, t) = target_range(st)
    case t >. f {
      True -> {
        let assert Some(mt) = schema.marks_get_at(schema.get_marks(sch), mkey)
        let tr = editor_state.get_tr(st)
        case node.range_has_mark_type(editor_state.get_doc(st), f, t, mt) {
          True -> {
            transform.remove_mark_type_with(tr, f, t, Some(mt))
            pm_rt.ref_set(action_ref, "Removed " <> label)
          }
          False -> {
            let m = schema.mark_mark_type(sch, mt)
            transform.add_mark(tr, f, t, m)
            pm_rt.ref_set(action_ref, "Added " <> label)
          }
        }
        editor_view.dispatch(view, tr)
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
    let pname =
      node_type.get_name(
        node.get_type(resolved_pos.get_parent(selection.get_dollar_from(sel))),
      )
    let tr = editor_state.get_tr(st)
    let nodes = schema.get_nodes(sch)
    case pname == "heading" {
      True -> {
        let assert Some(pt) = schema.nodes_get_at(nodes, "paragraph")
        transform.set_block_type(
          tr,
          selection.get_from(sel),
          Some(selection.get_to(sel)),
          pt,
        )
        pm_rt.ref_set(action_ref, "Converted heading -> paragraph")
      }
      False -> {
        let assert Some(ht) = schema.nodes_get_at(nodes, "heading")
        let ha =
          attrs.create()
          |> attrs.with_at("level", dynamic.int(2))
        transform.set_block_type_with(
          tr,
          selection.get_from(sel),
          Some(selection.get_to(sel)),
          ht,
          Some(ha),
        )
        pm_rt.ref_set(action_ref, "Converted block -> heading[level=2]")
      }
    }
    editor_view.dispatch(view, tr)
  })
  wire_btn(doc, "btn-4", fn() {
    pm_rt.ref_set(dispatches_ref, 0)
    pm_rt.ref_set(action_ref, "Reset initial schema document")
    let fresh = make_state(sch)
    editor_view.update_state(view, fresh)
    update_meta(sch, doc, fresh, dispatches_ref, action_ref)
  })
}
