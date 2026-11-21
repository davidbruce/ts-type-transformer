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
        .with_to_dom(&NodeToDOMOutputSpecFn::new(|n: Node| {
            let l = n.attrs().at("level").as_f64() as i32;
            hole_spec(if l == 1 { "h1" } else { "h2" })
        }));

    let para = NodeSpec::default()
        .with_content("inline*")
        .with_group("block")
        .with_to_dom(&NodeToDOMOutputSpecFn::new(|_: Node| hole_spec("p")));

    let cblock = NodeSpec::default()
        .with_content("text*")
        .with_marks("")
        .with_group("block")
        .with_code(true)
        .with_to_dom(&NodeToDOMOutputSpecFn::new(|_: Node| {
            let inner = JsArray::<JsValue>::from([JsValue::from("code"), JsValue::from(0.0)]);
            DOMOutputSpec::Array(JsArray::from([JsValue::from("pre"), JsValue::from(inner)]))
        }));

    let text = NodeSpec::default().with_group("inline");

    let nodes = JsValue::object()
        .with("doc", &doc)
        .with("heading", &heading)
        .with("paragraph", &para)
        .with("code_block", &cblock)
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

fn make_initial_doc(schema: &Schema<JsValue, JsValue>) -> Node {
    let ha = Attrs::default();
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
        JsArray::<Node>::from([
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
    let inl_attr = DecorationAttrs::default().with_class("pm-match");
    let blk_attr = DecorationAttrs::default().with_class("pm-hit-block");

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
            .expect("missing button")
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
        meta: dom_doc.get_element_by_id("meta").expect("missing #meta"),
        active_btn: 0,
    }));

    let field = StateField::<JsValue>::default()
        .with_init(&StateFieldInit::new(|_, _| JsValue::from("state")))
        .with_apply(&StateFieldApply::new(|tr, prev: JsValue, _, _| {
            let mv = tr.get_meta("search");
            if mv.is_null() { prev } else { mv }
        }));

    let eprops = EditorProps::<Plugin<JsValue>>::default().with_decorations(
        &EditorPropsDecorations::<Plugin<JsValue>>::new({
            let key = key.clone();
            move |st: EditorState| {
                let q = key
                    .get_state(&st)
                    .filter(|v| !v.is_null())
                    .map_or_else(String::new, |v| v.as_string());
                let (dset, _, _) = compute_decorations(&st.doc(), &q);
                Some(dset.into())
            }
        }),
    );

    let pspec = PluginSpec::<JsValue>::default()
        .with_key(&key)
        .with_state(&field)
        .with_props(&eprops);

    let scfg = EditorStateConfig::default()
        .with_schema(&schema)
        .with_doc(&make_initial_doc(&schema))
        .with_plugins(&JsArray::<Plugin<JsValue>>::from([Plugin::new(&pspec)]));

    let props = DirectEditorProps::default()
        .with_state(&EditorState::create(&scfg))
        .with_dispatch_transaction(&TransactionFn::new({
            let app = app.clone();
            move |tr: Transaction| {
                let view = app.borrow().view.clone();
                if let Some(view) = view {
                    let next = view.state().apply(&tr);
                    view.update_state(&next);
                    sync_meta(&app.borrow(), &next);
                }
            }
        }));

    let mount = dom_doc.get_element_by_id("editor").expect("missing #editor");
    let place = EditorViewNewPlaceOrDistDOMNode::from(AsRef::<dom::Node>::as_ref(&mount));
    let view = EditorView::new(Some(place), &props);
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
        let btn = dom_doc.get_element_by_id(bid).expect("missing button");
        let app_c = app.clone();
        let q = *query;
        let cb = dom::JsListener::new(move |_: dom::PointerEvent| {
            let view = {
                let mut st = app_c.borrow_mut();
                st.active_btn = idx;
                st.view.as_ref().expect("missing view").clone()
            };
            let tr = view.state().tr();
            tr.set_meta("search", q);
            view.dispatch().call(&tr);
        });
        btn.add_event_listener(dom::HTMLElementEventMap::CLICK, &cb, None);
    }
}
