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
        .with_to_dom(&NodeToDOMOutputSpecFn::new(|n: Node| {
            let l = n.attrs().at("level").as_f64() as i32;
            hole_spec(if l == 1 { "h1" } else { "h2" })
        }));

    let para = NodeSpec::default()
        .with_content("inline*")
        .with_group("block")
        .with_to_dom(&NodeToDOMOutputSpecFn::new(|_: Node| hole_spec("p")));

    let quote = NodeSpec::default()
        .with_content("block+")
        .with_group("block")
        .with_defining(true)
        .with_to_dom(&NodeToDOMOutputSpecFn::new(|_: Node| {
            hole_spec("blockquote")
        }));

    let cblock = NodeSpec::default()
        .with_content("text*")
        .with_marks("")
        .with_group("block")
        .with_code(true)
        .with_defining(true)
        .with_to_dom(&NodeToDOMOutputSpecFn::new(|_: Node| {
            let inner = JsArray::<JsValue>::from([JsValue::from("code"), JsValue::from(0.0)]);
            DOMOutputSpec::Array(JsArray::from([JsValue::from("pre"), JsValue::from(inner)]))
        }));

    let hr = NodeSpec::default()
        .with_group("block")
        .with_atom(true)
        .with_to_dom(&NodeToDOMOutputSpecFn::new(|_: Node| {
            DOMOutputSpec::Array(JsArray::from(["hr"]))
        }));

    let text = NodeSpec::default().with_group("inline");

    let nodes = JsValue::object()
        .with("doc", &doc)
        .with("heading", &heading)
        .with("paragraph", &para)
        .with("blockquote", &quote)
        .with("code_block", &cblock)
        .with("horizontal_rule", &hr)
        .with("text", &text);

    let make_mark = |tag: &'static str| {
        MarkSpec::default().with_to_dom(&MarkBooleanToDOMOutputSpecFn::new(
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

fn make_initial_doc(schema: &Schema<JsValue, JsValue>) -> Node {
    let ha = Attrs::default();
    ha.set_at("level", 2.0);
    let h = schema.node_with(
        "heading",
        &ha,
        schema.text("ProseMirror Document Schema"),
        None,
    );

    let m_strong = JsArray::<Mark>::from([schema.mark("strong")]);
    let m_em = JsArray::<Mark>::from([schema.mark("em")]);
    let m_code = JsArray::<Mark>::from([schema.mark("code")]);

    let p1 = schema.node_with(
        "paragraph",
        None,
        JsArray::<Node>::from([
            schema.text("Inline marks compose cleanly: "),
            schema.text_with("bold", &m_strong),
            schema.text(", "),
            schema.text_with("italic", &m_em),
            schema.text(", and "),
            schema.text_with("inline_code", &m_code),
            schema.text(" spans."),
        ]),
        None,
    );
    let q_inner = schema.node_with(
        "paragraph",
        None,
        schema.text("Every document conforms to a strict schema enforced on every transaction."),
        None,
    );
    let quote = schema.node_with("blockquote", None, q_inner, None);
    let cb = schema.node_with(
        "code_block",
        None,
        schema.text("schema.node(\"paragraph\", null, [schema.text(\"typed\")])"),
        None,
    );
    let outro = schema.node_with(
        "paragraph",
        None,
        schema.text("Select any text in this editor and click a toolbar action to toggle marks or switch block types."),
        None,
    );
    schema.node_with(
        "doc",
        None,
        JsArray::<Node>::from([h, p1, quote, cb, outro]),
        None,
    )
}

fn make_state(schema: &Schema<JsValue, JsValue>) -> EditorState {
    let cfg = EditorStateConfig::default()
        .with_schema(schema)
        .with_doc(&make_initial_doc(schema));
    EditorState::create(&cfg)
}

fn target_range(st: &EditorState) -> (f64, f64) {
    let sel = st.selection();
    if !sel.empty() {
        return (sel.from(), sel.to());
    }
    let doc = st.doc();
    if doc.child_count() >= 2.0 {
        let c0 = doc.child(0.0);
        let c1 = doc.child(1.0);
        let start_pos = c0.node_size() + 1.0;
        let len = c1.content().size();
        if len > 0.0 {
            return (start_pos, start_pos + len.min(12.0));
        }
    }
    let sz = doc.content().size();
    (1.0, sz.min(2.0))
}

struct AppState {
    schema: Schema<JsValue, JsValue>,
    view: Option<EditorView>,
    doc: dom::Document,
    meta: dom::HTMLElement,
    dispatches: usize,
    last_action: String,
}

fn update_meta(app: &AppState, st: &EditorState) {
    let (f, t) = target_range(st);
    let doc = st.doc();
    let marks = app.schema.marks();
    let hb = doc.range_has_mark(f, t, marks.at("strong").expect("missing strong"));
    let he = doc.range_has_mark(f, t, marks.at("em").expect("missing em"));
    let hc = doc.range_has_mark(f, t, marks.at("code").expect("missing code"));

    app.doc
        .get_element_by_id("btn-0")
        .expect("missing #btn-0")
        .set_class_name(if hb { "active" } else { "" });
    app.doc
        .get_element_by_id("btn-1")
        .expect("missing #btn-1")
        .set_class_name(if he { "active" } else { "" });
    app.doc
        .get_element_by_id("btn-2")
        .expect("missing #btn-2")
        .set_class_name(if hc { "active" } else { "" });

    let bname = st.selection().dollar_from().parent().type_().name();
    let msg = format!(
        "{} \u{00B7} blocks={} \u{00B7} nodeSize={} \u{00B7} range=[{}..{}] \u{00B7} block={} \u{00B7} dispatches={}",
        app.last_action,
        doc.child_count() as usize,
        doc.node_size() as usize,
        f as usize,
        t as usize,
        bname,
        app.dispatches
    );
    app.meta.set_text_content(&msg);
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let dom_doc = dom::document();
    let app = Rc::new(RefCell::new(AppState {
        schema: build_schema(),
        view: None,
        doc: dom_doc.clone(),
        meta: dom_doc.get_element_by_id("meta").expect("missing #meta"),
        dispatches: 0,
        last_action: "Initialized schema with 7 node types & 3 marks".to_string(),
    }));

    let props = DirectEditorProps::default()
        .with_state(&make_state(&app.borrow().schema))
        .with_dispatch_transaction(&TransactionFn::new({
            let app = app.clone();
            move |tr: Transaction| {
                let view = app.borrow().view.clone();
                if let Some(view) = view {
                    let next = view.state().apply(&tr);
                    view.update_state(&next);
                    let mut st = app.borrow_mut();
                    st.dispatches += 1;
                    update_meta(&st, &next);
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
        update_meta(&st, &view.state());
    }

    for (idx, bid) in ["btn-0", "btn-1", "btn-2", "btn-3", "btn-4"]
        .iter()
        .enumerate()
    {
        let btn = dom_doc.get_element_by_id(bid).expect("missing button");
        let app_c = app.clone();
        let cb = dom::JsListener::new(move |_: dom::PointerEvent| {
            if idx < 3 {
                let (mark_key, label) = match idx {
                    0 => ("strong", "bold"),
                    1 => ("em", "italic"),
                    _ => ("code", "code"),
                };
                let (view, mt, (f, t)) = {
                    let st = app_c.borrow();
                    let v = st.view.as_ref().expect("missing view").clone();
                    let es = v.state();
                    (
                        v,
                        st.schema.marks().at(mark_key).expect("missing mark"),
                        target_range(&es),
                    )
                };
                if t > f {
                    let es = view.state();
                    let tr = es.tr();
                    let added = if es.doc().range_has_mark(f, t, mt.clone()) {
                        tr.remove_mark_with(f, t, mt);
                        false
                    } else {
                        let m = app_c.borrow().schema.mark(mt);
                        tr.add_mark(f, t, &m);
                        true
                    };
                    app_c.borrow_mut().last_action =
                        format!("{} {}", if added { "Added" } else { "Removed" }, label);
                    view.dispatch(&tr);
                }
            } else if idx == 3 {
                let view = app_c.borrow().view.as_ref().expect("missing view").clone();
                let es = view.state();
                let sel = es.selection();
                let pname = sel.dollar_from().parent().type_().name();
                let tr = es.tr();
                if pname == "heading" {
                    let pt = app_c
                        .borrow()
                        .schema
                        .nodes()
                        .at("paragraph")
                        .expect("missing paragraph");
                    tr.set_block_type(sel.from(), sel.to(), &pt);
                    app_c.borrow_mut().last_action = "Converted heading -> paragraph".to_string();
                } else {
                    let ht = app_c
                        .borrow()
                        .schema
                        .nodes()
                        .at("heading")
                        .expect("missing heading");
                    let ha = Attrs::default();
                    ha.set_at("level", 2.0);
                    tr.set_block_type_with(sel.from(), sel.to(), &ht, &ha);
                    app_c.borrow_mut().last_action =
                        "Converted block -> heading[level=2]".to_string();
                }
                view.dispatch(&tr);
            } else {
                let (view, fresh) = {
                    let mut st = app_c.borrow_mut();
                    st.dispatches = 0;
                    st.last_action = "Reset initial schema document".to_string();
                    let fresh = make_state(&st.schema);
                    let v = st.view.as_ref().expect("missing view").clone();
                    v.update_state(&fresh);
                    update_meta(&st, &fresh);
                    (v, fresh)
                };
                let _ = (view, fresh);
            }
        });
        btn.add_event_listener(dom::HTMLElementEventMap::CLICK, &cb, None);
    }
}
