#![no_main]

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

    let tag_node = |content: &str, group: Option<&str>, tag: &'static str, defining: bool| {
        let s = NodeSpec::create();
        s.set_content(Some(content));
        if let Some(g) = group {
            s.set_group(Some(g));
        }
        if defining {
            s.set_defining(Some(true));
        }
        s.set_to_dom(Some(&NodeToDOMOutputSpecFn::new(move |_: Node| hole_spec(tag))));
        s
    };

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
    nodes.set_at("paragraph", JsValue::from(&tag_node("inline*", Some("block"), "p", false)));
    nodes.set_at("blockquote", JsValue::from(&tag_node("block+", Some("block"), "blockquote", true)));
    nodes.set_at("bullet_list", JsValue::from(&tag_node("list_item+", Some("block"), "ul", false)));
    nodes.set_at("list_item", JsValue::from(&tag_node("paragraph block*", None, "li", true)));
    nodes.set_at("horizontal_rule", JsValue::from(&hr));
    nodes.set_at("text", JsValue::from(&text));

    let make_mark = |tag: &'static str| {
        let m = MarkSpec::create();
        m.set_to_dom(Some(&MarkBooleanToDOMOutputSpecFn::new(move |_: Mark, _: bool| hole_spec(tag))));
        m
    };
    let marks = MarkSpec::create();
    marks.set_at("strong", JsValue::from(&make_mark("strong")));
    marks.set_at("code", JsValue::from(&make_mark("code")));

    let spec = SchemaSpec::<JsValue, JsValue>::create();
    spec.set_nodes(JsValue::from(&nodes));
    spec.set_marks(JsValue::from(&marks));
    Schema::new(&spec)
}

fn p_node(schema: &Schema<JsValue, JsValue>, t: &str) -> Node {
    schema.node("paragraph", None, schema.text(t, None), None)
}
fn li_node(schema: &Schema<JsValue, JsValue>, t: &str) -> Node {
    schema.node("list_item", None, p_node(schema, t), None)
}

fn make_initial_doc(schema: &Schema<JsValue, JsValue>) -> Node {
    let ha = Attrs::create();
    ha.set_at("level", JsValue::from(2.0));
    let h = schema.node("heading", Some(&ha), schema.text("Structural Transform Pipeline", None), None);
    let quote = schema.node("blockquote", None, p_node(schema, "Blockquote content can be lifted back to top-level document depth."), None);
    let ulist = schema.node(
        "bullet_list",
        None,
        JsArray::<Node>::from(vec![
            li_node(schema, "Split at cursor position"),
            li_node(schema, "Wrap range with blockquote"),
            li_node(schema, "Lift nested block to root"),
        ]),
        None,
    );
    schema.node(
        "doc",
        None,
        JsArray::<Node>::from(vec![
            h,
            p_node(schema, "ProseMirror transforms express structural edits as invertible steps."),
            p_node(schema, "Wrap blocks in quotes, lift nested content, or split paragraphs cleanly."),
            quote,
            ulist,
        ]),
        None,
    )
}

fn make_state(schema: &Schema<JsValue, JsValue>) -> EditorState {
    let cfg = EditorStateConfig::create();
    cfg.set_schema(Some(schema));
    cfg.set_doc(Some(&make_initial_doc(schema)));
    EditorState::create(&cfg)
}

struct AppState {
    schema: Schema<JsValue, JsValue>,
    view: Option<EditorView>,
    meta: dom::HTMLElement,
    total_steps: usize,
    note_counter: usize,
    last_op: String,
}

fn update_meta(app: &AppState, doc: &Node) {
    let n = doc.child_count() as usize;
    let mut outline = String::new();
    for i in 0..n {
        if i > 0 {
            outline.push_str(" \u{00B7} ");
        }
        outline.push_str(&doc.child(i as f64).type_().name());
    }
    let msg = format!(
        "{} \u{00B7} blocks={} \u{00B7} outline=[{}] \u{00B7} totalSteps={} \u{00B7} nodeSize={}",
        app.last_op,
        n,
        outline,
        app.total_steps,
        doc.node_size() as usize
    );
    app.meta.set_text_content(Some(&msg));
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let dom_doc = dom::document();
    let app = Rc::new(RefCell::new(AppState {
        schema: build_schema(),
        view: None,
        meta: dom_doc.get_element_by_id("meta").unwrap(),
        total_steps: 0,
        note_counter: 0,
        last_op: "Initialized document structure".to_string(),
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
                st.total_steps += tr.steps().len();
                update_meta(&st, &next.doc());
            }
        }
    })));

    let mount = dom_doc.get_element_by_id("editor").unwrap();
    let place = JsValue::from(&mount);
    let view = EditorView::new(Some(&place), &props);
    {
        let mut st = app.borrow_mut();
        st.view = Some(view.clone());
        update_meta(&st, &view.state().doc());
    }

    for (idx, bid) in ["btn-0", "btn-1", "btn-2", "btn-3", "btn-4"].iter().enumerate() {
        let btn = dom_doc.get_element_by_id(bid).unwrap();
        let app_c = app.clone();
        let cb = dom::JsListener::new(move |_: dom::PointerEvent| {
            let view = app_c.borrow().view.as_ref().unwrap().clone();
            let es = view.state();
            let doc = es.doc();
            match idx {
                0 => {
                    let sel_from = es.selection().from();
                    let mut split_pos = if can_split(&doc, sel_from, None, None) { Some(sel_from) } else { None };
                    if split_pos.is_none() {
                        let mut pos = 0.0;
                        for k in 0..(doc.child_count() as usize) {
                            let ch = doc.child(k as f64);
                            if split_pos.is_none() && ch.type_().name() == "paragraph" {
                                let len = ch.content().size() as usize;
                                if len >= 8 {
                                    let mid = pos + 1.0 + (len / 2) as f64;
                                    if can_split(&doc, mid, None, None) {
                                        split_pos = Some(mid);
                                    }
                                }
                            }
                            pos += ch.node_size();
                        }
                    }
                    if let Some(sp) = split_pos {
                        let tr = es.tr();
                        tr.split(sp, None, None);
                        app_c.borrow_mut().last_op = format!("Split paragraph at pos {}", sp as usize);
                        call_transaction_fn(&view.dispatch(), &tr);
                    } else {
                        let mut st = app_c.borrow_mut();
                        st.last_op = "No splittable paragraph found".to_string();
                        update_meta(&st, &doc);
                    }
                }
                1 => {
                    let qtype = app_c.borrow().schema.nodes().at("blockquote").unwrap();
                    let sel = es.selection();
                    let mut found: Option<NodeRange> = None;
                    if sel.from_new().depth() == 1.0 && sel.from_new().parent().type_().name() == "paragraph" {
                        if let Some(r) = sel.from_new().block_range(Some(&sel.to_new()), None) {
                            if find_wrapping(&r, &qtype, None, None).is_some() {
                                found = Some(r);
                            }
                        }
                    }
                    if found.is_none() {
                        let mut pos = 0.0;
                        for k in 0..(doc.child_count() as usize) {
                            let ch = doc.child(k as f64);
                            if found.is_none() && ch.type_().name() == "paragraph" {
                                let rp = doc.resolve(pos + 1.0);
                                if let Some(r) = rp.block_range(Some(&rp), None) {
                                    if find_wrapping(&r, &qtype, None, None).is_some() {
                                        found = Some(r);
                                    }
                                }
                            }
                            pos += ch.node_size();
                        }
                    }
                    if let Some(r) = found {
                        if let Some(wrapped) = find_wrapping(&r, &qtype, None, None) {
                            let tr = es.tr();
                            tr.wrap(&r, wrapped);
                            app_c.borrow_mut().last_op = format!("Wrapped block [{}..{}] in blockquote", r.start() as usize, r.end() as usize);
                            call_transaction_fn(&view.dispatch(), &tr);
                            return;
                        }
                    }
                    let mut st = app_c.borrow_mut();
                    st.last_op = "No top-level paragraph left to wrap".to_string();
                    update_meta(&st, &doc);
                }
                2 => {
                    let mut pos = 0.0;
                    for k in 0..(doc.child_count() as usize) {
                        let ch = doc.child(k as f64);
                        if ch.type_().name() == "blockquote" && ch.child_count() > 0.0 {
                            let rp = doc.resolve(pos + 2.0);
                            if let Some(r) = rp.block_range(Some(&rp), None) {
                                if let Some(target) = lift_target(&r) {
                                    let tr = es.tr();
                                    tr.lift(&r, target);
                                    app_c.borrow_mut().last_op = format!("Lifted blockquote child to depth {}", target as usize);
                                    call_transaction_fn(&view.dispatch(), &tr);
                                    return;
                                }
                            }
                        }
                        pos += ch.node_size();
                    }
                    let mut st = app_c.borrow_mut();
                    st.last_op = "No blockquote left to lift".to_string();
                    update_meta(&st, &doc);
                }
                3 => {
                    let (hr, note) = {
                        let mut st = app_c.borrow_mut();
                        st.note_counter += 1;
                        let msg = format!("Appended structural section #{} at doc offset {}.", st.note_counter, doc.content().size() as usize);
                        st.last_op = format!("Inserted horizontal_rule + note #{}", st.note_counter);
                        (st.schema.node("horizontal_rule", None, JsArray::<Node>::new(), None), p_node(&st.schema, &msg))
                    };
                    let tr = es.tr();
                    tr.insert(doc.content().size(), JsArray::<Node>::from(vec![hr, note]));
                    call_transaction_fn(&view.dispatch(), &tr);
                }
                _ => {
                    let mut st = app_c.borrow_mut();
                    st.total_steps = 0;
                    st.note_counter = 0;
                    st.last_op = "Reset structural sample".to_string();
                    let fresh = make_state(&st.schema);
                    view.update_state(&fresh);
                    update_meta(&st, &fresh.doc());
                }
            }
        });
        btn.add_event_listener(dom::HTMLElementEventMap::CLICK, &cb, None);
        std::mem::forget(cb);
    }
}
