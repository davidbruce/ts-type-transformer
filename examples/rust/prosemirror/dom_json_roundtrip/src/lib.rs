#![no_main]

use prosemirror::prosemirror_model::SchemaSpec;
use prosemirror::prosemirror_model::*;
use prosemirror::prosemirror_state::*;
use prosemirror::prosemirror_view::*;
use prosemirror::*;
use std::cell::RefCell;
use std::rc::Rc;

fn hole_spec(tag: &str) -> DOMOutputSpec {
    DOMOutputSpec::Array(JsArray::from([JsValue::from(tag), JsValue::from(0.0)]))
}

fn tag_rule(tag: &str, level: i32) -> TagParseRule {
    let mut r = TagParseRule::default().with_tag(tag);
    if level > 0 {
        let a = Attrs::default();
        a.set_at("level", level as f64);
        r = r.with_attrs(&a);
    }
    r
}

fn build_schema() -> Schema<JsValue, JsValue> {
    let doc = NodeSpec::default().with_content("block+");

    let lvl = <AttributeSpec as Default>::default().with_default(2.0);
    let hattrs = MarkSpecAttrs::default();
    hattrs.set_at("level", &lvl);

    let heading = NodeSpec::default()
        .with_attrs(&hattrs)
        .with_content("inline*")
        .with_group("block")
        .with_defining(true)
        .with_parse_dom(&JsArray::<TagParseRule>::from([
            tag_rule("h1", 1),
            tag_rule("h2", 2),
        ]))
        .with_to_dom(&NodeToDOMOutputSpecFn::new(|n: Node| {
            let l = n.attrs().at("level").as_f64() as i32;
            hole_spec(if l == 1 { "h1" } else { "h2" })
        }));

    let para = NodeSpec::default()
        .with_content("inline*")
        .with_group("block")
        .with_parse_dom(&JsArray::<TagParseRule>::from([tag_rule("p", 0)]))
        .with_to_dom(&NodeToDOMOutputSpecFn::new(|_: Node| hole_spec("p")));

    let quote = NodeSpec::default()
        .with_content("block+")
        .with_group("block")
        .with_defining(true)
        .with_parse_dom(&JsArray::<TagParseRule>::from([tag_rule("blockquote", 0)]))
        .with_to_dom(&NodeToDOMOutputSpecFn::new(|_: Node| {
            hole_spec("blockquote")
        }));

    let text = NodeSpec::default().with_group("inline");

    let nodes = JsValue::object()
        .with("doc", &doc)
        .with("heading", &heading)
        .with("paragraph", &para)
        .with("blockquote", &quote)
        .with("text", &text);

    let make_mark = |tag: &'static str| {
        MarkSpec::default()
            .with_parse_dom(&JsArray::<ParseRule>::from([tag_rule(tag, 0)]))
            .with_to_dom(&MarkBooleanToDOMOutputSpecFn::new(
                move |_: Mark, _: bool| hole_spec(tag),
            ))
    };
    let marks = JsValue::object()
        .with("strong", &make_mark("strong"))
        .with("em", &make_mark("em"))
        .with("code", &make_mark("code"));

    let spec = SchemaSpec::<JsValue, JsValue>::default()
        .with_nodes(nodes)
        .with_marks(marks);
    Schema::new(&spec)
}

const SPEC_HTML: &str = "<h2>DOMParser &amp; JSON Serialization</h2><p>ProseMirror converts <strong>HTML</strong> via <code>DOMParser</code> and renders back through <em>DOMSerializer</em>.</p><blockquote><p>Schema parseDOM rules keep DOM and JSON representations aligned.</p></blockquote><p>Trigger any action above to inspect live DOMSerializer HTML and Node.eq() round-trips.</p>";
const CHANGELOG_HTML: &str = "<h2>Release 1.4 Notes</h2><p>Added <strong>typed bindings</strong> for <code>DOMParser</code> and <em>DOMSerializer</em>.</p><blockquote><p>All round-trips verified with Node.eq().</p></blockquote><p>Edit directly above to watch HTML output update live.</p>";

struct AppState {
    schema: Schema<JsValue, JsValue>,
    parser: DOMParser,
    serializer: DOMSerializer,
    view: Option<EditorView>,
    doc: dom::Document,
    output: dom::HTMLElement,
    meta: dom::HTMLElement,
    status: String,
}

fn parse_html(dom_doc: &dom::Document, parser: &DOMParser, html: &str) -> Node {
    let div = dom_doc.create_element("div", None);
    div.set_inner_html(html);
    parser.parse(&div)
}

fn refresh_output(app: &AppState, doc: &Node) {
    let div = app.doc.create_element("div", None);
    app.serializer
        .serialize_fragment_with(&doc.content(), None, div.clone());
    let html_out = div.inner_html();

    let nb = doc.child_count() as usize;
    let nsz = doc.node_size() as usize;
    let mut ast = format!("doc(blocks={nb}, nodeSize={nsz})\n");
    for i in 0..nb {
        let b = doc.child(i as f64);
        let tname = b.type_().name();
        let kind = if tname == "heading" {
            format!("heading[level={}]", b.attrs().at("level").as_f64() as i32)
        } else {
            tname
        };
        ast.push_str(&format!(
            "  [{i}] {kind} ({} bytes): {}\n",
            b.node_size() as usize,
            b.text_content()
        ));
    }
    let full = format!(
        "SERIALIZED HTML (DOMSerializer):\n{html_out}\n\nDOCUMENT AST & JSON ROUND-TRIP:\n{ast}\nROUND-TRIP STATUS: {}",
        app.status
    );
    app.output.set_text_content(&full);

    let msg = format!(
        "{} \u{00B7} blocks={nb} \u{00B7} nodeSize={nsz} \u{00B7} htmlBytes={}",
        app.status,
        html_out.len()
    );
    app.meta.set_text_content(&msg);
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let dom_doc = dom::document();
    let schema = build_schema();
    let parser = DOMParser::from_schema(&schema);
    let serializer = DOMSerializer::from_schema(&schema);

    let initial_doc = parse_html(&dom_doc, &parser, SPEC_HTML);
    let app = Rc::new(RefCell::new(AppState {
        schema: schema.clone(),
        parser,
        serializer,
        view: None,
        doc: dom_doc.clone(),
        output: dom_doc
            .get_element_by_id("output")
            .expect("missing #output"),
        meta: dom_doc.get_element_by_id("meta").expect("missing #meta"),
        status: "Parsed Spec HTML via DOMParser".to_string(),
    }));

    let scfg = EditorStateConfig::default()
        .with_schema(&schema)
        .with_doc(&initial_doc);

    let props = DirectEditorProps::default()
        .with_state(&EditorState::create(&scfg))
        .with_dispatch_transaction(&TransactionFn::new({
            let app = app.clone();
            move |tr: Transaction| {
                let view = app.borrow().view.clone();
                if let Some(view) = view {
                    let next = view.state().apply(&tr);
                    view.update_state(&next);
                    refresh_output(&app.borrow(), &next.doc());
                }
            }
        }));

    let mount = dom_doc
        .get_element_by_id("editor")
        .expect("missing #editor");
    let place = EditorViewNewPlaceOrDistDOMNode::from(AsRef::<dom::Node>::as_ref(&mount));
    let view = EditorView::new(place, &props);
    {
        let mut st = app.borrow_mut();
        st.view = Some(view.clone());
        refresh_output(&st, &view.state().doc());
    }

    for (idx, bid) in ["btn-0", "btn-1", "btn-2", "btn-3", "btn-4"]
        .iter()
        .enumerate()
    {
        let btn = dom_doc.get_element_by_id(bid).expect("missing button");
        let app_c = app.clone();
        let cb = dom::JsListener::new(move |_: dom::PointerEvent| {
            let view = app_c.borrow().view.as_ref().expect("missing view").clone();
            let es = view.state();
            let doc = es.doc();
            match idx {
                0 | 1 => {
                    let mut st = app_c.borrow_mut();
                    st.status = if idx == 0 {
                        "Parsed Spec HTML via DOMParser"
                    } else {
                        "Parsed Changelog HTML via DOMParser"
                    }
                    .to_string();
                    let next_doc = parse_html(
                        &st.doc,
                        &st.parser,
                        if idx == 0 { SPEC_HTML } else { CHANGELOG_HTML },
                    );
                    let cfg = EditorStateConfig::default()
                        .with_schema(&st.schema)
                        .with_doc(&next_doc);
                    let next = EditorState::create(&cfg);
                    view.update_state(&next);
                    refresh_output(&st, &next.doc());
                }
                2 => {
                    let mut st = app_c.borrow_mut();
                    let rebuilt = Node::from_json(&st.schema, doc.to_json());
                    st.status = format!("JSON toJSON->fromJSON eq() = {}", doc.js_eq(&rebuilt));
                    refresh_output(&st, &doc);
                }
                3 => {
                    if doc.child_count() >= 2.0 {
                        let insert_pos =
                            doc.child(0.0).node_size() + doc.child(1.0).node_size() - 1.0;
                        let span = {
                            let mut st = app_c.borrow_mut();
                            st.status =
                                format!("Inserted code span at pos {}", insert_pos as usize);
                            let marks = JsArray::<Mark>::from([st.schema.mark("code")]);
                            st.schema.text_with(" roundtrip()", &marks)
                        };
                        let tr = es.tr();
                        tr.insert(insert_pos, span);
                        view.dispatch(&tr);
                    }
                }
                _ => {
                    let mut st = app_c.borrow_mut();
                    let div = st.doc.create_element("div", None);
                    st.serializer
                        .serialize_fragment_with(&doc.content(), None, div.clone());
                    let reparsed = st.parser.parse(&div);
                    st.status = format!(
                        "DOM serializeFragment->parse eq() = {}",
                        doc.js_eq(&reparsed)
                    );
                    let cfg = EditorStateConfig::default()
                        .with_schema(&st.schema)
                        .with_doc(&reparsed);
                    let next = EditorState::create(&cfg);
                    view.update_state(&next);
                    refresh_output(&st, &next.doc());
                }
            }
        });
        btn.add_event_listener(dom::HTMLElementEventMap::CLICK, &cb, None);
    }
}
