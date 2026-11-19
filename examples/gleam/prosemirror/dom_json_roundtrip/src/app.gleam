import dom/areas/core
import dom/areas/core/document
import dom/areas/core/element
import dom/areas/core/node as dom_node
import dom/areas/html/html_element
import gleam/bool
import gleam/dynamic.{type Dynamic}
import gleam/dynamic/decode
import gleam/float
import gleam/int
import gleam/list
import gleam/option.{None, Some}
import gleam/string
import prosemirror/areas/main/mark_spec_attrs
import prosemirror/areas/prosemirror_model/attribute_spec
import prosemirror/areas/prosemirror_model/attrs
import prosemirror/areas/prosemirror_model/dom_parser
import prosemirror/areas/prosemirror_model/dom_serializer
import prosemirror/areas/prosemirror_model/generic_parse_rule
import prosemirror/areas/prosemirror_model/mark_spec
import prosemirror/areas/prosemirror_model/node
import prosemirror/areas/prosemirror_model/node_spec
import prosemirror/areas/prosemirror_model/node_type
import prosemirror/areas/prosemirror_model/schema
import prosemirror/areas/prosemirror_model/schema_spec
import prosemirror/areas/prosemirror_model/tag_parse_rule
import prosemirror/areas/prosemirror_state/editor_state
import prosemirror/areas/prosemirror_state/editor_state_config
import prosemirror/areas/prosemirror_transform/transform
import prosemirror/areas/prosemirror_view/direct_editor_props
import prosemirror/areas/prosemirror_view/editor_view
import prosemirror/runtime as pm_rt
import prosemirror/types as t

@external(javascript, "./app_ffi.mjs", "coerce")
fn coerce(a: a) -> b

@external(javascript, "./app_ffi.mjs", "newCell")
fn new_cell(v: a) -> Dynamic

@external(javascript, "./app_ffi.mjs", "getCell")
fn get_cell(c: Dynamic) -> a

@external(javascript, "./app_ffi.mjs", "setCell")
fn set_cell(c: Dynamic, v: a) -> Nil

fn hole_spec(tag: String) -> t.DOMOutputSpec {
  t.DOMOutputSpecArray([dynamic.string(tag), dynamic.int(0)])
}

fn tag_rule(tag: String, level: Int) -> t.TagParseRule(Nil) {
  let r = tag_parse_rule.create()
  tag_parse_rule.set_tag(r, tag)
  case level > 0 {
    True -> {
      let a = attrs.create()
      attrs.set_at(a, "level", dynamic.int(level))
      generic_parse_rule.set_attrs(r, Some(a))
    }
    False -> Nil
  }
  r
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
  node_spec.set_parse_dom(heading_spec, Some([tag_rule("h1", 1), tag_rule("h2", 2)]))
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
  node_spec.set_parse_dom(para_spec, Some([tag_rule("p", 0)]))
  node_spec.set_to_dom(para_spec, Some(fn(_) { hole_spec("p") }))

  let quote_spec = node_spec.create()
  node_spec.set_content(quote_spec, Some("block+"))
  node_spec.set_group(quote_spec, Some("block"))
  node_spec.set_defining(quote_spec, Some(True))
  node_spec.set_parse_dom(quote_spec, Some([tag_rule("blockquote", 0)]))
  node_spec.set_to_dom(quote_spec, Some(fn(_) { hole_spec("blockquote") }))

  let text_spec = node_spec.create()
  node_spec.set_group(text_spec, Some("inline"))

  let nodes = node_spec.create()
  node_spec.set_at(nodes, "doc", pm_rt.dynamic(doc_spec))
  node_spec.set_at(nodes, "heading", pm_rt.dynamic(heading_spec))
  node_spec.set_at(nodes, "paragraph", pm_rt.dynamic(para_spec))
  node_spec.set_at(nodes, "blockquote", pm_rt.dynamic(quote_spec))
  node_spec.set_at(nodes, "text", pm_rt.dynamic(text_spec))

  let make_mark = fn(tag: String) {
    let m = mark_spec.create()
    mark_spec.set_parse_dom(m, Some([t.ParseRuleTagParseRule(tag_rule(tag, 0))]))
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

const spec_html = "<h2>DOMParser &amp; JSON Serialization</h2><p>ProseMirror converts <strong>HTML</strong> via <code>DOMParser</code> and renders back through <em>DOMSerializer</em>.</p><blockquote><p>Schema parseDOM rules keep DOM and JSON representations aligned.</p></blockquote><p>Trigger any action above to inspect live DOMSerializer HTML and Node.eq() round-trips.</p>"

const changelog_html = "<h2>Release 1.4 Notes</h2><p>Added <strong>typed bindings</strong> for <code>DOMParser</code> and <em>DOMSerializer</em>.</p><blockquote><p>All round-trips verified with Node.eq().</p></blockquote><p>Edit directly above to watch HTML output update live.</p>"

fn parse_html(doc, parser, html: String) -> t.Node(Nil) {
  let div = document.create_element_div(doc, None)
  element.set_inner_html(div, html)
  dom_parser.parse(parser, coerce(div), None)
}

fn describe_doc(d: t.Node(Nil)) -> String {
  let nb = float.round(node.get_child_count(d))
  let nsz = float.round(node.get_node_size(d))
  let header = "doc(blocks=" <> int.to_string(nb) <> ", nodeSize=" <> int.to_string(nsz) <> ")"
  let rows =
    list.index_map(node.get_children(d), fn(b, i) {
      let tname = node_type.get_name(node.get_type(b))
      let kind = case tname {
        "heading" -> {
          let lvl = case decode.run(attrs.get_at(node.get_attrs(b), "level"), decode.int) {
            Ok(k) -> k
            Error(_) -> 2
          }
          "heading[level=" <> int.to_string(lvl) <> "]"
        }
        _ -> tname
      }
      "  [" <> int.to_string(i) <> "] " <> kind <> " (" <> int.to_string(float.round(node.get_node_size(b))) <> " bytes): " <> node.get_text_content(b)
    })
  string.join([header, ..rows], "\n")
}

fn refresh_output(doc, serializer, d: t.Node(Nil), status_c) {
  let div = document.create_element_div(doc, None)
  let _ = dom_serializer.serialize_fragment(serializer, node.get_content(d), None, Some(t.HTMLElementOrDocumentFragmentHTMLElement(coerce(div))))
  let html_out = element.get_inner_html(div)
  let status: String = get_cell(status_c)
  let full =
    "SERIALIZED HTML (DOMSerializer):\n"
    <> html_out
    <> "\n\nDOCUMENT AST & JSON ROUND-TRIP:\n"
    <> describe_doc(d)
    <> "\n\nROUND-TRIP STATUS: "
    <> status
  case document.get_element_by_id(doc, "output") {
    Some(out) -> dom_node.set_text_content(out, Some(full))
    None -> Nil
  }
  let msg =
    status
    <> " · blocks="
    <> int.to_string(float.round(node.get_child_count(d)))
    <> " · nodeSize="
    <> int.to_string(float.round(node.get_node_size(d)))
    <> " · htmlBytes="
    <> int.to_string(string.length(html_out))
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
  let parser = dom_parser.from_schema(sch)
  let serializer = dom_serializer.from_schema(sch)
  let view_c = new_cell(None)
  let status_c = new_cell("Parsed Spec HTML via DOMParser")

  let cfg = editor_state_config.create()
  editor_state_config.set_schema(cfg, Some(sch))
  editor_state_config.set_doc(cfg, Some(parse_html(doc, parser, spec_html)))

  let props = direct_editor_props.create()
  direct_editor_props.set_state(props, editor_state.create(cfg))
  direct_editor_props.set_dispatch_transaction(
    props,
    Some(fn(tr) {
      case get_cell(view_c) {
        Some(v) -> {
          let next = editor_state.apply(editor_view.get_state(v), tr)
          editor_view.update_state(v, next)
          refresh_output(doc, serializer, editor_state.get_doc(next), status_c)
        }
        None -> Nil
      }
    }),
  )

  let assert Some(mount) = document.get_element_by_id(doc, "editor")
  let view = editor_view.new(Some(pm_rt.dynamic(mount)), props)
  set_cell(view_c, Some(view))
  refresh_output(doc, serializer, editor_state.get_doc(editor_view.get_state(view)), status_c)

  wire_btn(doc, "btn-0", fn() {
    set_cell(status_c, "Parsed Spec HTML via DOMParser")
    let c = editor_state_config.create()
    editor_state_config.set_schema(c, Some(sch))
    editor_state_config.set_doc(c, Some(parse_html(doc, parser, spec_html)))
    let next = editor_state.create(c)
    editor_view.update_state(view, next)
    refresh_output(doc, serializer, editor_state.get_doc(next), status_c)
  })

  wire_btn(doc, "btn-1", fn() {
    set_cell(status_c, "Parsed Changelog HTML via DOMParser")
    let c = editor_state_config.create()
    editor_state_config.set_schema(c, Some(sch))
    editor_state_config.set_doc(c, Some(parse_html(doc, parser, changelog_html)))
    let next = editor_state.create(c)
    editor_view.update_state(view, next)
    refresh_output(doc, serializer, editor_state.get_doc(next), status_c)
  })

  wire_btn(doc, "btn-2", fn() {
    let d = editor_state.get_doc(editor_view.get_state(view))
    let rebuilt = node.from_json(sch, node.to_json(d))
    set_cell(status_c, "JSON toJSON->fromJSON eq() = " <> string.lowercase(bool.to_string(node.eq(d, rebuilt))))
    refresh_output(doc, serializer, d, status_c)
  })

  wire_btn(doc, "btn-3", fn() {
    let st = editor_view.get_state(view)
    let d = editor_state.get_doc(st)
    case node.get_child_count(d) >=. 2.0 {
      True -> {
        let insert_pos = node.get_node_size(node.child(d, 0.0)) +. node.get_node_size(node.child(d, 1.0)) -. 1.0
        let mc = schema.mark(sch, t.StringOrMarkTypeString("code"), None)
        let span = schema.text(sch, " roundtrip()", Some([mc]))
        let tr = editor_state.get_tr(st)
        let _ = transform.insert(tr, insert_pos, t.FragmentOrNodeOrNodeArrayNode(span))
        set_cell(status_c, "Inserted code span at pos " <> int.to_string(float.round(insert_pos)))
        editor_view.get_dispatch(view)(tr)
      }
      False -> Nil
    }
  })

  wire_btn(doc, "btn-4", fn() {
    let d = editor_state.get_doc(editor_view.get_state(view))
    let div = document.create_element_div(doc, None)
    let _ = dom_serializer.serialize_fragment(serializer, node.get_content(d), None, Some(t.HTMLElementOrDocumentFragmentHTMLElement(coerce(div))))
    let reparsed = dom_parser.parse(parser, coerce(div), None)
    set_cell(status_c, "DOM serializeFragment->parse eq() = " <> string.lowercase(bool.to_string(node.eq(d, reparsed))))
    let c = editor_state_config.create()
    editor_state_config.set_schema(c, Some(sch))
    editor_state_config.set_doc(c, Some(reparsed))
    let next = editor_state.create(c)
    editor_view.update_state(view, next)
    refresh_output(doc, serializer, editor_state.get_doc(next), status_c)
  })
}
