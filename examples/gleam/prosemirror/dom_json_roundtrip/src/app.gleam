import dom/areas/core
import dom/areas/core/document
import dom/areas/core/element
import dom/areas/core/node as dom_node
import dom/areas/html/html_element
import dom/areas/html/html_element_event_map as ev
import dom/areas/html/html_element_tag_name_map as tag
import gleam/bool
import gleam/dynamic.{type Dynamic}
import gleam/dynamic/decode
import gleam/float
import gleam/int
import gleam/list
import gleam/option.{None, Some}
import gleam/string
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
import prosemirror/mark_spec_attrs
import prosemirror/runtime as pm_rt
import prosemirror/types as t

fn hole_spec(tag: String) -> t.DOMOutputSpec {
  t.DOMOutputSpecArray([dynamic.string(tag), dynamic.int(0)])
}

fn tag_rule(tag: String, level: Int) -> t.TagParseRule(Nil) {
  let rule =
    tag_parse_rule.create()
    |> tag_parse_rule.with_tag(tag)
  case level > 0 {
    True -> {
      let a =
        attrs.create()
        |> attrs.with_at("level", dynamic.int(level))
      generic_parse_rule.with_attrs(rule, a)
    }
    False -> rule
  }
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
    |> node_spec.with_defining(True)
    |> node_spec.with_parse_dom([tag_rule("h1", 1), tag_rule("h2", 2)])
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
    |> node_spec.with_parse_dom([tag_rule("p", 0)])
    |> node_spec.with_to_dom(fn(_) { hole_spec("p") })

  let quote_spec =
    node_spec.create()
    |> node_spec.with_content("block+")
    |> node_spec.with_group("block")
    |> node_spec.with_defining(True)
    |> node_spec.with_parse_dom([tag_rule("blockquote", 0)])
    |> node_spec.with_to_dom(fn(_) { hole_spec("blockquote") })

  let text_spec =
    node_spec.create()
    |> node_spec.with_group("inline")

  let nodes =
    node_spec.create()
    |> node_spec.with_at("doc", pm_rt.dynamic(doc_spec))
    |> node_spec.with_at("heading", pm_rt.dynamic(heading_spec))
    |> node_spec.with_at("paragraph", pm_rt.dynamic(para_spec))
    |> node_spec.with_at("blockquote", pm_rt.dynamic(quote_spec))
    |> node_spec.with_at("text", pm_rt.dynamic(text_spec))

  let make_mark = fn(tag: String) {
    mark_spec.create()
    |> mark_spec.with_parse_dom([t.ParseRuleTagParseRule(tag_rule(tag, 0))])
    |> mark_spec.with_to_dom(fn(_, _) { hole_spec(tag) })
  }
  let marks =
    mark_spec.create()
    |> mark_spec.with_at("strong", pm_rt.dynamic(make_mark("strong")))
    |> mark_spec.with_at("em", pm_rt.dynamic(make_mark("em")))
    |> mark_spec.with_at("code", pm_rt.dynamic(make_mark("code")))

  let spec =
    schema_spec.create()
    |> schema_spec.with_nodes(pm_rt.dynamic(nodes))
    |> schema_spec.with_marks(pm_rt.dynamic(marks))
  schema.new(spec)
}

const spec_html =
  "<h2>DOMParser &amp; JSON Serialization</h2><p>ProseMirror converts <strong>HTML</strong> via <code>DOMParser</code> and renders back through <em>DOMSerializer</em>.</p><blockquote><p>Schema parseDOM rules keep DOM and JSON representations aligned.</p></blockquote><p>Trigger any action above to inspect live DOMSerializer HTML and Node.eq() round-trips.</p>"

const changelog_html =
  "<h2>Release 1.4 Notes</h2><p>Added <strong>typed bindings</strong> for <code>DOMParser</code> and <em>DOMSerializer</em>.</p><blockquote><p>All round-trips verified with Node.eq().</p></blockquote><p>Edit directly above to watch HTML output update live.</p>"

fn parse_html(doc, parser, html: String) -> t.Node(Nil) {
  let div = document.create_element(doc, tag.div)
  element.set_inner_html(div, html)
  dom_parser.parse(parser, div)
}

fn describe_doc(d: t.Node(Nil)) -> String {
  let nb = float.round(node.get_child_count(d))
  let nsz = float.round(node.get_node_size(d))
  let header =
    "doc(blocks="
    <> int.to_string(nb)
    <> ", nodeSize="
    <> int.to_string(nsz)
    <> ")"
  let rows =
    list.index_map(node.get_children(d), fn(b, i) {
      let tname = node_type.get_name(node.get_type(b))
      let kind = case tname {
        "heading" -> {
          let lvl = case
            decode.run(attrs.get_at(node.get_attrs(b), "level"), decode.int)
          {
            Ok(k) -> k
            Error(_) -> 2
          }
          "heading[level=" <> int.to_string(lvl) <> "]"
        }
        _ -> tname
      }
      "  ["
      <> int.to_string(i)
      <> "] "
      <> kind
      <> " ("
      <> int.to_string(float.round(node.get_node_size(b)))
      <> " bytes): "
      <> node.get_text_content(b)
    })
  string.join([header, ..rows], "\n")
}

fn refresh_output(
  doc,
  serializer,
  d: t.Node(Nil),
  status_ref: pm_rt.Ref(String),
) {
  let div = document.create_element(doc, tag.div)
  dom_serializer.serialize_fragment_with(
    serializer,
    node.get_content(d),
    None,
    Some(t.HTMLElementOrDocumentFragmentHTMLElement(html_element.upcast(div))),
  )
  let html_out = element.get_inner_html(div)
  let status = pm_rt.ref_get(status_ref)
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
    Some(btn) ->
      html_element.add_event_listener(btn, ev.click, fn(_) { action() })
    None -> Nil
  }
}

pub fn main() {
  let doc = core.get_document()
  let sch = build_schema()
  let parser = dom_parser.from_schema(sch)
  let serializer = dom_serializer.from_schema(sch)
  let view_ref = pm_rt.ref_new(None)
  let status_ref = pm_rt.ref_new("Parsed Spec HTML via DOMParser")

  let cfg =
    editor_state_config.create()
    |> editor_state_config.with_schema(sch)
    |> editor_state_config.with_doc(parse_html(doc, parser, spec_html))

  let props =
    direct_editor_props.create()
    |> direct_editor_props.with_state(editor_state.create(cfg))
    |> direct_editor_props.with_dispatch_transaction(fn(tr) {
      case pm_rt.ref_get(view_ref) {
        Some(v) -> {
          let next = editor_state.apply(editor_view.get_state(v), tr)
          editor_view.update_state(v, next)
          refresh_output(
            doc,
            serializer,
            editor_state.get_doc(next),
            status_ref,
          )
        }
        None -> Nil
      }
    })

  let assert Some(mount) = document.get_element_by_id(doc, "editor")
  let view =
    editor_view.new(
      Some(t.EditorViewNewPlaceOrDistDOMNodeNode2(dom_node.upcast(mount))),
      props,
    )
  pm_rt.ref_set(view_ref, Some(view))
  refresh_output(
    doc,
    serializer,
    editor_state.get_doc(editor_view.get_state(view)),
    status_ref,
  )

  wire_btn(doc, "btn-0", fn() {
    pm_rt.ref_set(status_ref, "Parsed Spec HTML via DOMParser")
    let c =
      editor_state_config.create()
      |> editor_state_config.with_schema(sch)
      |> editor_state_config.with_doc(parse_html(doc, parser, spec_html))
    let next = editor_state.create(c)
    editor_view.update_state(view, next)
    refresh_output(doc, serializer, editor_state.get_doc(next), status_ref)
  })

  wire_btn(doc, "btn-1", fn() {
    pm_rt.ref_set(status_ref, "Parsed Changelog HTML via DOMParser")
    let c =
      editor_state_config.create()
      |> editor_state_config.with_schema(sch)
      |> editor_state_config.with_doc(parse_html(doc, parser, changelog_html))
    let next = editor_state.create(c)
    editor_view.update_state(view, next)
    refresh_output(doc, serializer, editor_state.get_doc(next), status_ref)
  })

  wire_btn(doc, "btn-2", fn() {
    let d = editor_state.get_doc(editor_view.get_state(view))
    let rebuilt = node.from_json(sch, node.to_json(d))
    pm_rt.ref_set(
      status_ref,
      "JSON toJSON->fromJSON eq() = "
        <> string.lowercase(bool.to_string(node.eq(d, rebuilt))),
    )
    refresh_output(doc, serializer, d, status_ref)
  })

  wire_btn(doc, "btn-3", fn() {
    let st = editor_view.get_state(view)
    let d = editor_state.get_doc(st)
    case node.get_child_count(d) >=. 2.0 {
      True -> {
        let insert_pos =
          node.get_node_size(node.child(d, 0.0))
          +. node.get_node_size(node.child(d, 1.0))
          -. 1.0
        let mc = schema.mark(sch, t.StringOrMarkTypeString("code"))
        let span = schema.text_with(sch, " roundtrip()", Some([mc]))
        let tr =
          editor_state.get_tr(st)
          |> transform.insert(insert_pos, t.FragmentOrNodeOrNodeArrayNode(span))
        pm_rt.ref_set(
          status_ref,
          "Inserted code span at pos " <> int.to_string(float.round(insert_pos)),
        )
        editor_view.dispatch(view, tr)
      }
      False -> Nil
    }
  })

  wire_btn(doc, "btn-4", fn() {
    let d = editor_state.get_doc(editor_view.get_state(view))
    let div = document.create_element(doc, tag.div)
    dom_serializer.serialize_fragment_with(
      serializer,
      node.get_content(d),
      None,
      Some(t.HTMLElementOrDocumentFragmentHTMLElement(html_element.upcast(div))),
    )
    let reparsed = dom_parser.parse(parser, div)
    pm_rt.ref_set(
      status_ref,
      "DOM serializeFragment->parse eq() = "
        <> string.lowercase(bool.to_string(node.eq(d, reparsed))),
    )
    let c =
      editor_state_config.create()
      |> editor_state_config.with_schema(sch)
      |> editor_state_config.with_doc(reparsed)
    let next = editor_state.create(c)
    editor_view.update_state(view, next)
    refresh_output(doc, serializer, editor_state.get_doc(next), status_ref)
  })
}
