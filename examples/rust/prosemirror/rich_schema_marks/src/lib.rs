#![no_main]

use dom::ElementMethods;
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
    heading.set_to_dom(Some(&NodeToDOMOutputSpecFn::new(|n: Node| {
        let l = n.attrs().at("level").as_f64() as i32;
        hole_spec(if l == 1 { "h1" } else { "h2" })
    })));

    let para = NodeSpec::create();
    para.set_content(Some("inline*"));
    para.set_group(Some("block"));
    para.set_to_dom(Some(&NodeToDOMOutputSpecFn::new(|_: Node| hole_spec("p"))));

    let quote = NodeSpec::create();
    quote.set_content(Some("block+"));
    quote.set_group(Some("block"));
    quote.set_defining(Some(true));
    quote.set_to_dom(Some(&NodeToDOMOutputSpecFn::new(|_: Node| hole_spec("blockquote"))));

    let cblock = NodeSpec::create();
    cblock.set_content(Some("text*"));
    cblock.set_marks(Some(""));
    cblock.set_group(Some("block"));
    cblock.set_code(Some(true));
    cblock.set_defining(Some(true));
    cblock.set_to_dom(Some(&NodeToDOMOutputSpecFn::new(|_: Node| {
        let inner = JsArray::<JsValue>::from(vec![JsValue::from("code"), JsValue::from(0.0)]);
        DOMOutputSpec::Array(JsArray::from(vec![JsValue::from("pre"), JsValue(inner.0)]))
    })));

    let hr = NodeSpec::create();
    hr.set_group(Some("block"));
    hr.set_atom(Some(true));
    hr.set_to_dom(Some(&NodeToDOMOutputSpecFn::new(|_: Node| {
        DOMOutputSpec::Array(JsArray::from(vec![JsValue::from("hr")]))
    })));

    let text = NodeSpec::create();
    text.set_group(Some("inline"));

    let nodes = NodeSpec::create();
    nodes.set_at("doc", JsValue::from(&doc));
    nodes.set_at("heading", JsValue::from(&heading));
    nodes.set_at("paragraph", JsValue::from(&para));
    nodes.set_at("blockquote", JsValue::from(&quote));
    nodes.set_at("code_block", JsValue::from(&cblock));
    nodes.set_at("horizontal_rule", JsValue::from(&hr));
    nodes.set_at("text", JsValue::from(&text));

    let make_mark = |tag: &'static str| {
        let m = MarkSpec::create();
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

fn make_initial_doc(schema: &Schema<JsValue, JsValue>) -> Node {
    let ha = Attrs::create();
    ha.set_at("level", JsValue::from(2.0));
    let h = schema.node("heading", Some(&ha), schema.text("ProseMirror Document Schema", None), None);

    let m_strong = JsArray::<Mark>::from(vec![schema.mark("strong", None)]);
    let m_em = JsArray::<Mark>::from(vec![schema.mark("em", None)]);
    let m_code = JsArray::<Mark>::from(vec![schema.mark("code", None)]);

    let p1 = schema.node(
        "paragraph",
        None,
        JsArray::<Node>::from(vec![
            schema.text("Inline marks compose cleanly: ", None),
            schema.text("bold", Some(&m_strong)),
            schema.text(", ", None),
            schema.text("italic", Some(&m_em)),
            schema.text(", and ", None),
            schema.text("inline_code", Some(&m_code)),
            schema.text(" spans.", None),
        ]),
        None,
    );
    let q_inner = schema.node("paragraph", None, schema.text("Every document conforms to a strict schema enforced on every transaction.", None), None);
    let quote = schema.node("blockquote", None, q_inner, None);
    let cb = schema.node("code_block", None, schema.text("schema.node(\"paragraph\", null, [schema.text(\"typed\")])", None), None);
    let outro = schema.node(
        "paragraph",
        None,
        schema.text("Select any text in this editor and click a toolbar action to toggle marks or switch block types.", None),
        None,
    );
    schema.node("doc", None, JsArray::<Node>::from(vec![h, p1, quote, cb, outro]), None)
}

fn make_state(schema: &Schema<JsValue, JsValue>) -> EditorState {
    let cfg = EditorStateConfig::create();
    cfg.set_schema(Some(schema));
    cfg.set_doc(Some(&make_initial_doc(schema)));
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
    let hb = doc.range_has_mark(f, t, MarkOrMarkType::MarkType(marks.at("strong").unwrap()));
    let he = doc.range_has_mark(f, t, MarkOrMarkType::MarkType(marks.at("em").unwrap()));
    let hc = doc.range_has_mark(f, t, MarkOrMarkType::MarkType(marks.at("code").unwrap()));

    app.doc.get_element_by_id("btn-0").unwrap().set_class_name(if hb { "active" } else { "" });
    app.doc.get_element_by_id("btn-1").unwrap().set_class_name(if he { "active" } else { "" });
    app.doc.get_element_by_id("btn-2").unwrap().set_class_name(if hc { "active" } else { "" });

    let bname = st.selection().from_new().parent().type_().name();
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
    app.meta.set_text_content(Some(&msg));
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let dom_doc = dom::document();
    let app = Rc::new(RefCell::new(AppState {
        schema: build_schema(),
        view: None,
        doc: dom_doc.clone(),
        meta: dom_doc.get_element_by_id("meta").unwrap(),
        dispatches: 0,
        last_action: "Initialized schema with 7 node types & 3 marks".to_string(),
    }));

    let props = DirectEditorProps::create();
    props.set_state(&make_state(&app.borrow().schema));
    props.set_dispatch_transaction(Some(&TransactionFn::new({
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
    })));

    let mount = dom_doc.get_element_by_id("editor").unwrap();
    let place = JsValue::from(&mount);
    let view = EditorView::new(Some(&place), &props);
    {
        let mut st = app.borrow_mut();
        st.view = Some(view.clone());
        update_meta(&st, &view.state());
    }

    for (idx, bid) in ["btn-0", "btn-1", "btn-2", "btn-3", "btn-4"].iter().enumerate() {
        let btn = dom_doc.get_element_by_id(bid).unwrap();
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
                    let v = st.view.as_ref().unwrap().clone();
                    let es = v.state();
                    (v, st.schema.marks().at(mark_key).unwrap(), target_range(&es))
                };
                if t > f {
                    let es = view.state();
                    let tr = es.tr();
                    let added = if es.doc().range_has_mark(f, t, MarkOrMarkType::MarkType(mt.clone())) {
                        tr.remove_mark(f, t, MarkOrMarkType::MarkType(mt));
                        false
                    } else {
                        let m = app_c.borrow().schema.mark(mt, None);
                        tr.add_mark(f, t, &m);
                        true
                    };
                    app_c.borrow_mut().last_action = format!("{} {}", if added { "Added" } else { "Removed" }, label);
                    call_transaction_fn(&view.dispatch(), &tr);
                }
            } else if idx == 3 {
                let view = app_c.borrow().view.as_ref().unwrap().clone();
                let es = view.state();
                let sel = es.selection();
                let pname = sel.from_new().parent().type_().name();
                let tr = es.tr();
                if pname == "heading" {
                    let pt = app_c.borrow().schema.nodes().at("paragraph").unwrap();
                    tr.set_block_type(sel.from(), Some(sel.to()), &pt, None);
                    app_c.borrow_mut().last_action = "Converted heading -> paragraph".to_string();
                } else {
                    let ht = app_c.borrow().schema.nodes().at("heading").unwrap();
                    let ha = Attrs::create();
                    ha.set_at("level", JsValue::from(2.0));
                    tr.set_block_type(sel.from(), Some(sel.to()), &ht, Some(&JsValue::from(&ha)));
                    app_c.borrow_mut().last_action = "Converted block -> heading[level=2]".to_string();
                }
                call_transaction_fn(&view.dispatch(), &tr);
            } else {
                let (view, fresh) = {
                    let mut st = app_c.borrow_mut();
                    st.dispatches = 0;
                    st.last_action = "Reset initial schema document".to_string();
                    let fresh = make_state(&st.schema);
                    let v = st.view.as_ref().unwrap().clone();
                    v.update_state(&fresh);
                    update_meta(&st, &fresh);
                    (v, fresh)
                };
                let _ = (view, fresh);
            }
        });
        btn.add_event_listener(dom::HTMLElementEventMap::CLICK, &cb, None);
        std::mem::forget(cb);
    }
}
