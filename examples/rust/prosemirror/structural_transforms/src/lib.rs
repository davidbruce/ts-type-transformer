#![no_main]

use prosemirror::prosemirror_model::SchemaSpec;
use prosemirror::prosemirror_model::*;
use prosemirror::prosemirror_state::*;
use prosemirror::prosemirror_transform::*;
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

    let tag_node = |content: &str, group: Option<&str>, tag: &'static str, defining: bool| {
        let mut s = NodeSpec::default().with_content(content);
        if let Some(g) = group {
            s = s.with_group(g);
        }
        if defining {
            s = s.with_defining(true);
        }
        s.with_to_dom(&NodeToDOMOutputSpecFn::new(move |_: Node| hole_spec(tag)))
    };

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
        .with("paragraph", &tag_node("inline*", Some("block"), "p", false))
        .with("blockquote", &tag_node("block+", Some("block"), "blockquote", true))
        .with("bullet_list", &tag_node("list_item+", Some("block"), "ul", false))
        .with("list_item", &tag_node("paragraph block*", None, "li", true))
        .with("horizontal_rule", &hr)
        .with("text", &text);

    let make_mark = |tag: &'static str| {
        MarkSpec::default().with_to_dom(&MarkBooleanToDOMOutputSpecFn::new(move |_: Mark, _: bool| {
            hole_spec(tag)
        }))
    };
    let marks = JsValue::object()
        .with("strong", &make_mark("strong"))
        .with("code", &make_mark("code"));

    let spec = SchemaSpec::<JsValue, JsValue>::default()
        .with_nodes(nodes)
        .with_marks(marks);
    Schema::new(&spec)
}

fn p_node(schema: &Schema<JsValue, JsValue>, t: &str) -> Node {
    schema.node("paragraph", None, schema.text(t, None), None)
}
fn li_node(schema: &Schema<JsValue, JsValue>, t: &str) -> Node {
    schema.node("list_item", None, p_node(schema, t), None)
}

fn make_initial_doc(schema: &Schema<JsValue, JsValue>) -> Node {
    let ha = Attrs::default();
    ha.set_at("level", JsValue::from(2.0));
    let h = schema.node("heading", Some(&ha), schema.text("Structural Transform Pipeline", None), None);
    let quote = schema.node("blockquote", None, p_node(schema, "Blockquote content can be lifted back to top-level document depth."), None);
    let ulist = schema.node(
        "bullet_list",
        None,
        JsArray::<Node>::from([
            li_node(schema, "Split at cursor position"),
            li_node(schema, "Wrap range with blockquote"),
            li_node(schema, "Lift nested block to root"),
        ]),
        None,
    );
    schema.node(
        "doc",
        None,
        JsArray::<Node>::from([
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
    let cfg = EditorStateConfig::default()
        .with_schema(schema)
        .with_doc(&make_initial_doc(schema));
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
        meta: dom_doc.get_element_by_id("meta").expect("missing #meta"),
        total_steps: 0,
        note_counter: 0,
        last_op: "Initialized document structure".to_string(),
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
                    st.total_steps += tr.steps().len();
                    update_meta(&st, &next.doc());
                }
            }
        }));

    let mount = dom_doc.get_element_by_id("editor").expect("missing #editor");
    let place = JsValue::from(&mount);
    let view = EditorView::new(Some(&place), &props);
    {
        let mut st = app.borrow_mut();
        st.view = Some(view.clone());
        update_meta(&st, &view.state().doc());
    }

    for (idx, bid) in ["btn-0", "btn-1", "btn-2", "btn-3", "btn-4"].iter().enumerate() {
        let btn = dom_doc.get_element_by_id(bid).expect("missing button");
        let app_c = app.clone();
        let cb = dom::JsListener::new(move |_: dom::PointerEvent| {
            let view = app_c.borrow().view.as_ref().expect("missing view").clone();
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
                        view.dispatch().call(&tr);
                    } else {
                        let mut st = app_c.borrow_mut();
                        st.last_op = "No splittable paragraph found".to_string();
                        update_meta(&st, &doc);
                    }
                }
                1 => {
                    let qtype = app_c.borrow().schema.nodes().at("blockquote").expect("missing blockquote");
                    let sel = es.selection();
                    let mut found: Option<NodeRange> = None;
                    if sel.dollar_from().depth() == 1.0 && sel.dollar_from().parent().type_().name() == "paragraph" {
                        if let Some(r) = sel.dollar_from().block_range(Some(&sel.dollar_to()), None) {
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
                            view.dispatch().call(&tr);
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
                                    view.dispatch().call(&tr);
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
                    tr.insert(doc.content().size(), JsArray::<Node>::from([hr, note]));
                    view.dispatch().call(&tr);
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
    }
}
