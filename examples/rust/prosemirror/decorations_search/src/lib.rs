#![no_main]

use dom::ElementMethods;
use prosemirror::prosemirror_model::*;
use prosemirror::prosemirror_model::{SchemaSpec, SchemaSpecMethods};
use prosemirror::prosemirror_state::*;
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

    let cblock = NodeSpec::create();
    cblock.set_content(Some("text*"));
    cblock.set_marks(Some(""));
    cblock.set_group(Some("block"));
    cblock.set_code(Some(true));
    cblock.set_to_dom(Some(&NodeToDOMOutputSpecFn::new(|_: Node| {
        let inner = JsArray::<JsValue>::from(vec![JsValue::from("code"), JsValue::from(0.0)]);
        DOMOutputSpec::Array(JsArray::from(vec![JsValue::from("pre"), JsValue(inner.0)]))
    })));

    let text = NodeSpec::create();
    text.set_group(Some("inline"));

    let nodes = NodeSpec::create();
    nodes.set_at("doc", JsValue::from(&doc));
    nodes.set_at("heading", JsValue::from(&heading));
    nodes.set_at("paragraph", JsValue::from(&para));
    nodes.set_at("code_block", JsValue::from(&cblock));
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
    let h = schema.node("heading", Some(&ha), schema.text("Immutable State & View Decorations", None), None);
    let cb = schema.node(
        "code_block",
        None,
        schema.text("const nextState = state.apply(transaction);\nview.updateState(nextState); // sync view", None),
        None,
    );
    schema.node(
        "doc",
        None,
        JsArray::<Node>::from(vec![
            h,
            p_node(schema, "A ProseMirror schema validates every document tree before state updates commit to the view."),
            p_node(schema, "Each transaction maps positions forward so state plugins can recompute decorations without mutating the schema."),
            cb,
            p_node(schema, "Inline decorations highlight matching tokens while node decorations outline matching blocks in the view."),
        ]),
        None,
    )
}

fn compute_decorations(doc: &Node, raw_query: &str) -> (DecorationSet, usize, usize) {
    let q = raw_query.trim().to_ascii_lowercase();
    if q.is_empty() {
        return (DecorationSet::empty(), 0, 0);
    }
    let decos = JsArray::<Decoration>::new();
    let inl_attr = DecorationAttrs::create();
    inl_attr.set_class(Some("pm-match"));
    let blk_attr = DecorationAttrs::create();
    blk_attr.set_class(Some("pm-hit-block"));

    let mut inl_hits = 0usize;
    let mut blk_hits = 0usize;
    let mut block_pos = 0.0;
    for i in 0..(doc.child_count() as usize) {
        let blk = doc.child(i as f64);
        let mut inline_pos = block_pos + 1.0;
        let mut matched = false;
        for j in 0..(blk.child_count() as usize) {
            let ch = blk.child(j as f64);
            if ch.is_text() {
                let txt = ch.text_content().to_ascii_lowercase();
                let mut at = 0usize;
                while let Some(rel) = txt[at..].find(&q) {
                    let idx = at + rel;
                    let fp = inline_pos + idx as f64;
                    let tp = fp + q.len() as f64;
                    decos.push(Decoration::inline(fp, tp, &inl_attr, None));
                    inl_hits += 1;
                    matched = true;
                    at = idx + q.len();
                }
            }
            inline_pos += ch.node_size();
        }
        if matched {
            decos.push(Decoration::node(block_pos, block_pos + blk.node_size(), &blk_attr, None));
            blk_hits += 1;
        }
        block_pos += blk.node_size();
    }
    (DecorationSet::create(doc, decos), inl_hits, blk_hits)
}

struct AppState {
    key: PluginKey<JsValue>,
    view: Option<EditorView>,
    doc: dom::Document,
    meta: dom::HTMLElement,
    active_btn: usize,
}

fn sync_meta(app: &AppState, st: &EditorState) {
    let q = app
        .key
        .get_state(st)
        .filter(|v| !v.is_null())
        .map_or_else(String::new, |v| v.as_string());
    let (dset, inl, blk) = compute_decorations(&st.doc(), &q);
    let total = dset.find(None, None, None).len();
    let qdisp = if q.is_empty() { "(none)".to_string() } else { format!("\"{q}\"") };
    let msg = format!(
        "Query: {} \u{00B7} inline matches={} \u{00B7} highlighted blocks={} \u{00B7} decorations={}",
        qdisp, inl, blk, total
    );
    app.meta.set_text_content(Some(&msg));
    for (i, id) in ["btn-0", "btn-1", "btn-2", "btn-3", "btn-4"].iter().enumerate() {
        app.doc
            .get_element_by_id(id)
            .unwrap()
            .set_class_name(if i == app.active_btn { "active" } else { "" });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let dom_doc = dom::document();
    let schema = build_schema();
    let key = PluginKey::<JsValue>::new(Some("search"));

    let app = Rc::new(RefCell::new(AppState {
        key: key.clone(),
        view: None,
        doc: dom_doc.clone(),
        meta: dom_doc.get_element_by_id("meta").unwrap(),
        active_btn: 0,
    }));

    let field = StateField::<JsValue>::create();
    field.set_init(&StateFieldInit::new(|_, _| JsValue::from("state")));
    field.set_apply(&StateFieldApply::new(|tr, prev: JsValue, _, _| {
        let mv = tr.get_meta("search");
        if mv.is_null() { prev } else { mv }
    }));

    let eprops = EditorProps::<Plugin<JsValue>>::create();
    eprops.set_decorations(Some(&EditorPropsDecorations::<Plugin<JsValue>>::new({
        let key = key.clone();
        move |st: EditorState| {
            let q = key
                .get_state(&st)
                .filter(|v| !v.is_null())
                .map_or_else(String::new, |v| v.as_string());
            let (dset, _, _) = compute_decorations(&st.doc(), &q);
            Some(dset.into())
        }
    })));

    let pspec = PluginSpec::<JsValue>::create();
    pspec.set_key(Some(&key));
    pspec.set_state(Some(&field));
    pspec.set_props(Some(&eprops));

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
                let next = view.state().apply(&tr);
                view.update_state(&next);
                sync_meta(&app.borrow(), &next);
            }
        }
    })));

    let mount = dom_doc.get_element_by_id("editor").unwrap();
    let place = JsValue::from(&mount);
    let view = EditorView::new(Some(&place), &props);
    {
        let mut st = app.borrow_mut();
        st.view = Some(view.clone());
        sync_meta(&st, &view.state());
    }

    for (idx, (bid, query)) in [
        ("btn-0", "state"),
        ("btn-1", "schema"),
        ("btn-2", "transaction"),
        ("btn-3", "view"),
        ("btn-4", ""),
    ]
    .iter()
    .enumerate()
    {
        let btn = dom_doc.get_element_by_id(bid).unwrap();
        let app_c = app.clone();
        let q = *query;
        let cb = dom::JsListener::new(move |_: dom::PointerEvent| {
            let view = {
                let mut st = app_c.borrow_mut();
                st.active_btn = idx;
                st.view.as_ref().unwrap().clone()
            };
            let tr = view.state().tr();
            tr.set_meta("search", q);
            call_transaction_fn(&view.dispatch(), &tr);
        });
        btn.add_event_listener(dom::HTMLElementEventMap::CLICK, &cb, None);
        std::mem::forget(cb);
    }
}
