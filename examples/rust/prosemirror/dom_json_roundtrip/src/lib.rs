#![no_main]

use dom::{DocumentMethods, ElementMethods};
use prosemirror::prosemirror_model::*;
use prosemirror::prosemirror_model::{SchemaSpec, SchemaSpecMethods};
use prosemirror::prosemirror_state::*;
use prosemirror::prosemirror_transform::*;
use prosemirror::prosemirror_view::*;
use prosemirror::*;
use std::cell::RefCell;
use std::rc::Rc;

fn hole_spec(tag: &str) -> DOMOutputSpec {
    DOMOutputSpec::Array(JsArray::from(vec![JsValue::from(tag), JsValue::from(0.0)]))
}

fn tag_rule(tag: &str, level: i32) -> TagParseRule {
    let r = TagParseRule::create();
    r.set_tag(tag);
    if level > 0 {
        let a = Attrs::create();
        a.set_at("level", JsValue::from(level as f64));
        r.set_attrs(Some(&a));
    }
    r
}

fn build_schema() -> Schema<JsValue, JsValue> {
    let doc = NodeSpec::create();
    doc.set_content(Some("block+"));

    let lvl = AttributeSpec::create();
    lvl.set_default(JsValue::from(2.0));
    let hattrs = MarkSpecAttrs::create();
    hattrs.set_at("level", &lvl);

    let heading = NodeSpec::create();
    heading.set_attrs(Some(&hattrs));
    heading.set_content(Some("inline*"));
    heading.set_group(Some("block"));
    heading.set_defining(Some(true));
    heading.set_parse_dom(Some(&JsArray::<TagParseRule>::from(vec![tag_rule("h1", 1), tag_rule("h2", 2)])));
    heading.set_to_dom(Some(&NodeToDOMOutputSpecFn::new(|n: Node| {
        let l = n.attrs().at("level").as_f64() as i32;
        hole_spec(if l == 1 { "h1" } else { "h2" })
    })));

    let para = NodeSpec::create();
    para.set_content(Some("inline*"));
    para.set_group(Some("block"));
    para.set_parse_dom(Some(&JsArray::<TagParseRule>::from(vec![tag_rule("p", 0)])));
    para.set_to_dom(Some(&NodeToDOMOutputSpecFn::new(|_: Node| hole_spec("p"))));

    let quote = NodeSpec::create();
    quote.set_content(Some("block+"));
    quote.set_group(Some("block"));
    quote.set_defining(Some(true));
    quote.set_parse_dom(Some(&JsArray::<TagParseRule>::from(vec![tag_rule("blockquote", 0)])));
    quote.set_to_dom(Some(&NodeToDOMOutputSpecFn::new(|_: Node| hole_spec("blockquote"))));

    let text = NodeSpec::create();
    text.set_group(Some("inline"));

    let nodes = NodeSpec::create();
    nodes.set_at("doc", JsValue::from(&doc));
    nodes.set_at("heading", JsValue::from(&heading));
    nodes.set_at("paragraph", JsValue::from(&para));
    nodes.set_at("blockquote", JsValue::from(&quote));
    nodes.set_at("text", JsValue::from(&text));

    let make_mark = |tag: &'static str| {
        let m = MarkSpec::create();
        m.set_parse_dom(Some(&JsArray::<ParseRule>::from(vec![ParseRule::TagParseRule(tag_rule(tag, 0))])));
        m.set_to_dom(Some(&MarkBooleanToDOMOutputSpecFn::new(move |_: Mark, _: bool| hole_spec(tag))));
        m
    };
    let marks = MarkSpec::create();
    marks.set_at("strong", JsValue::from(&make_mark("strong")));
    marks.set_at("em", JsValue::from(&make_mark("em")));
    marks.set_at("code", JsValue::from(&make_mark("code")));

    let spec = SchemaSpec::<JsValue, JsValue>::create();
    spec.set_nodes(JsValue::from(&nodes));
    spec.set_marks(JsValue::from(&marks));
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
    parser.parse(&div, None)
}

fn refresh_output(app: &AppState, doc: &Node) {
    let div = app.doc.create_element("div", None);
    app.serializer
        .serialize_fragment(&doc.content(), None, HTMLElementOrDocumentFragment::HTMLElement(div.clone()));
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
        ast.push_str(&format!("  [{i}] {kind} ({} bytes): {}\n", b.node_size() as usize, b.text_content()));
    }
    let full = format!(
        "SERIALIZED HTML (DOMSerializer):\n{html_out}\n\nDOCUMENT AST & JSON ROUND-TRIP:\n{ast}\nROUND-TRIP STATUS: {}",
        app.status
    );
    app.output.set_text_content(Some(&full));

    let msg = format!("{} \u{00B7} blocks={nb} \u{00B7} nodeSize={nsz} \u{00B7} htmlBytes={}", app.status, html_out.len());
    app.meta.set_text_content(Some(&msg));
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
        output: dom_doc.get_element_by_id("output").unwrap(),
        meta: dom_doc.get_element_by_id("meta").unwrap(),
        status: "Parsed Spec HTML via DOMParser".to_string(),
    }));

    let scfg = EditorStateConfig::create();
    scfg.set_schema(Some(&schema));
    scfg.set_doc(Some(&initial_doc));

    let props = DirectEditorProps::create();
    props.set_state(&EditorState::create(&scfg));
    props.set_dispatch_transaction(Some(&TransactionFn::new({
        let app = app.clone();
        move |tr: Transaction| {
            let view = app.borrow().view.clone();
            if let Some(view) = view {
                let next = view.state().apply(&tr);
                view.update_state(&next);
                refresh_output(&app.borrow(), &next.doc());
            }
        }
    })));

    let mount = dom_doc.get_element_by_id("editor").unwrap();
    let place = JsValue::from(&mount);
    let view = EditorView::new(Some(&place), &props);
    {
        let mut st = app.borrow_mut();
        st.view = Some(view.clone());
        refresh_output(&st, &view.state().doc());
    }

    for (idx, bid) in ["btn-0", "btn-1", "btn-2", "btn-3", "btn-4"].iter().enumerate() {
        let btn = dom_doc.get_element_by_id(bid).unwrap();
        let app_c = app.clone();
        let cb = dom::JsListener::new(move |_: dom::PointerEvent| {
            let view = app_c.borrow().view.as_ref().unwrap().clone();
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
                    let next_doc = parse_html(&st.doc, &st.parser, if idx == 0 { SPEC_HTML } else { CHANGELOG_HTML });
                    let cfg = EditorStateConfig::create();
                    cfg.set_schema(Some(&st.schema));
                    cfg.set_doc(Some(&next_doc));
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
                        let insert_pos = doc.child(0.0).node_size() + doc.child(1.0).node_size() - 1.0;
                        let span = {
                            let mut st = app_c.borrow_mut();
                            st.status = format!("Inserted code span at pos {}", insert_pos as usize);
                            let marks = JsArray::<Mark>::from(vec![st.schema.mark("code", None)]);
                            st.schema.text(" roundtrip()", Some(&marks))
                        };
                        let tr = es.tr();
                        tr.insert(insert_pos, span);
                        call_transaction_fn(&view.dispatch(), &tr);
                    }
                }
                _ => {
                    let mut st = app_c.borrow_mut();
                    let div = st.doc.create_element("div", None);
                    st.serializer
                        .serialize_fragment(&doc.content(), None, HTMLElementOrDocumentFragment::HTMLElement(div.clone()));
                    let reparsed = st.parser.parse(&div, None);
                    st.status = format!("DOM serializeFragment->parse eq() = {}", doc.js_eq(&reparsed));
                    let cfg = EditorStateConfig::create();
                    cfg.set_schema(Some(&st.schema));
                    cfg.set_doc(Some(&reparsed));
                    let next = EditorState::create(&cfg);
                    view.update_state(&next);
                    refresh_output(&st, &next.doc());
                }
            }
        });
        btn.add_event_listener(dom::HTMLElementEventMap::CLICK, &cb, None);
        std::mem::forget(cb);
    }
}
