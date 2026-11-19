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
    heading.set_to_dom(Some(&NodeToDOMOutputSpecFn::new(|n: Node| {
        let l = n.attrs().at("level").as_f64() as i32;
        hole_spec(if l == 1 { "h1" } else { "h2" })
    })));

    let para = NodeSpec::create();
    para.set_content(Some("inline*"));
    para.set_group(Some("block"));
    para.set_to_dom(Some(&NodeToDOMOutputSpecFn::new(|_: Node| hole_spec("p"))));

    let text = NodeSpec::create();
    text.set_group(Some("inline"));

    let nodes = NodeSpec::create();
    nodes.set_at("doc", JsValue::from(&doc));
    nodes.set_at("heading", JsValue::from(&heading));
    nodes.set_at("paragraph", JsValue::from(&para));
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

fn make_initial_doc(schema: &Schema<JsValue, JsValue>) -> Node {
    let ha = Attrs::create();
    ha.set_at("level", JsValue::from(2.0));
    let h = schema.node("heading", Some(&ha), schema.text("Step-Inversion Undo & Transaction Filter", None), None);
    schema.node(
        "doc",
        None,
        JsArray::<Node>::from(vec![
            h,
            p_node(schema, "Every document change produces atomic Transform steps that can be inverted against prior snapshots."),
            p_node(schema, "A ProseMirror Plugin inspects transactions in filterTransaction and can enforce a read-only lock."),
            p_node(schema, "Click Append Revision or Bold First Line, then step backward with Undo and forward with Redo."),
        ]),
        None,
    )
}

struct AppState {
    schema: Schema<JsValue, JsValue>,
    audit_key: PluginKey<JsValue>,
    view: Option<EditorView>,
    doc: dom::Document,
    meta: dom::HTMLElement,
    locked: bool,
    blocked: usize,
    rev: usize,
    last_action: String,
    undo_stack: Vec<Vec<Step>>,
    redo_stack: Vec<Vec<Step>>,
}

fn update_meta(app: &AppState, st: &EditorState) {
    let edits = app
        .audit_key
        .get_state(st)
        .filter(|v| !v.is_null())
        .map_or(0usize, |v| v.as_f64() as usize);
    let msg = format!(
        "{} \u{00B7} edits={} \u{00B7} undoDepth={} \u{00B7} redoDepth={} \u{00B7} lock={} \u{00B7} blocked={} \u{00B7} blocks={}",
        app.last_action,
        edits,
        app.undo_stack.len(),
        app.redo_stack.len(),
        if app.locked { "LOCKED" } else { "OPEN" },
        app.blocked,
        st.doc().child_count() as usize
    );
    app.meta.set_text_content(Some(&msg));
    app.doc
        .get_element_by_id("btn-4")
        .unwrap()
        .set_class_name(if app.locked { "active" } else { "" });
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let dom_doc = dom::document();
    let schema = build_schema();
    let audit_key = PluginKey::<JsValue>::new(Some("audit"));

    let app = Rc::new(RefCell::new(AppState {
        schema: schema.clone(),
        audit_key: audit_key.clone(),
        view: None,
        doc: dom_doc.clone(),
        meta: dom_doc.get_element_by_id("meta").unwrap(),
        locked: false,
        blocked: 0,
        rev: 0,
        last_action: "Ready \u{2014} open write access".to_string(),
        undo_stack: Vec::new(),
        redo_stack: Vec::new(),
    }));

    let field = StateField::<JsValue>::create();
    field.set_init(&StateFieldInit::new(|_, _| JsValue::from(0.0)));
    field.set_apply(&StateFieldApply::new(|tr, prev: JsValue, _, _| {
        JsValue::from(prev.as_f64() + if tr.doc_changed() { 1.0 } else { 0.0 })
    }));

    let pspec = PluginSpec::<JsValue>::create();
    pspec.set_key(Some(&audit_key));
    pspec.set_state(Some(&field));
    pspec.set_filter_transaction(Some(&PluginSpecFilterTransaction::new({
        let app = app.clone();
        move |tr: Transaction, _| {
            let mut st = app.borrow_mut();
            if st.locked && tr.doc_changed() {
                st.blocked += 1;
                st.last_action = "Blocked document mutation (filterTransaction=false)".to_string();
                false
            } else {
                true
            }
        }
    })));

    let scfg = EditorStateConfig::create();
    scfg.set_schema(Some(&schema));
    scfg.set_doc(Some(&make_initial_doc(&schema)));
    scfg.set_plugins(Some(&JsArray::<Plugin<JsValue>>::from(vec![Plugin::new(&pspec)])));

    let props = DirectEditorProps::create();
    props.set_state(&EditorState::create(&scfg));
    props.set_dispatch_transaction(Some(&TransactionFn::new({
        let app = app.clone();
        move |tr: Transaction| {
            let view = app.borrow().view.clone();
            if let Some(view) = view {
                let res = view.state().apply_transaction(&tr);
                if !res.transactions().is_empty() {
                    if tr.doc_changed() {
                        let steps = tr.steps();
                        let docs = tr.docs();
                        let mut inv = Vec::new();
                        for k in (0..steps.len()).rev() {
                            inv.push(steps.get(k).invert(&docs.get(k)));
                        }
                        let hm = tr.get_meta("history");
                        let mut st = app.borrow_mut();
                        if hm.is_null() {
                            st.undo_stack.push(inv);
                            st.redo_stack.clear();
                        } else if hm.as_string() == "undo" {
                            st.redo_stack.push(inv);
                        } else {
                            st.undo_stack.push(inv);
                        }
                    }
                    view.update_state(&res.state());
                }
                update_meta(&app.borrow(), &view.state());
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
            let view = app_c.borrow().view.as_ref().unwrap().clone();
            let es = view.state();
            match idx {
                0 => {
                    let node = {
                        let mut st = app_c.borrow_mut();
                        st.rev += 1;
                        let msg = format!("Revision #{} recorded via Transform step.", st.rev);
                        if !st.locked {
                            st.last_action = format!("Appended Revision #{}", st.rev);
                        }
                        p_node(&st.schema, &msg)
                    };
                    let tr = es.tr();
                    tr.insert(es.doc().content().size(), node);
                    call_transaction_fn(&view.dispatch(), &tr);
                }
                1 => {
                    let doc = es.doc();
                    if doc.child_count() >= 2.0 {
                        let fp = doc.child(0.0).node_size() + 1.0;
                        let tp = fp + doc.child(1.0).content().size();
                        if tp > fp {
                            let mark = app_c.borrow().schema.mark("strong", None);
                            let tr = es.tr();
                            tr.add_mark(fp, tp, &mark);
                            {
                                let mut st = app_c.borrow_mut();
                                if !st.locked {
                                    st.last_action = format!("Bolded first paragraph [{}..{}]", fp as usize, tp as usize);
                                }
                            }
                            call_transaction_fn(&view.dispatch(), &tr);
                        }
                    }
                }
                2 => {
                    let batch = {
                        let mut st = app_c.borrow_mut();
                        if st.undo_stack.is_empty() {
                            st.last_action = "Undo stack empty".to_string();
                            update_meta(&st, &es);
                            None
                        } else if st.locked {
                            st.undo_stack.last().cloned()
                        } else {
                            let b = st.undo_stack.pop();
                            st.last_action = "Undid step(s) via Step.invert".to_string();
                            b
                        }
                    };
                    if let Some(batch) = batch {
                        let tr = es.tr();
                        tr.set_meta("history", "undo");
                        for s in &batch {
                            tr.step(s);
                        }
                        call_transaction_fn(&view.dispatch(), &tr);
                    }
                }
                3 => {
                    let batch = {
                        let mut st = app_c.borrow_mut();
                        if st.redo_stack.is_empty() {
                            st.last_action = "Redo stack empty".to_string();
                            update_meta(&st, &es);
                            None
                        } else if st.locked {
                            st.redo_stack.last().cloned()
                        } else {
                            let b = st.redo_stack.pop();
                            st.last_action = "Redid inverted step(s)".to_string();
                            b
                        }
                    };
                    if let Some(batch) = batch {
                        let tr = es.tr();
                        tr.set_meta("history", "redo");
                        for s in &batch {
                            tr.step(s);
                        }
                        call_transaction_fn(&view.dispatch(), &tr);
                    }
                }
                _ => {
                    let mut st = app_c.borrow_mut();
                    st.locked = !st.locked;
                    st.last_action = if st.locked { "Enabled read-only lock" } else { "Disabled read-only lock" }.to_string();
                    update_meta(&st, &es);
                }
            }
        });
        btn.add_event_listener(dom::HTMLElementEventMap::CLICK, &cb, None);
        std::mem::forget(cb);
    }
}
