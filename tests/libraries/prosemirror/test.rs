// ProseMirror from Rust, in a page: a schema defined here, documents from
// JSON and built by hand, an EditorState with transactions, marks,
// selections and a plugin, and an EditorView on the page. Every check! must
// hold.
#![no_main]
use bindings::prosemirror_model::*;
// FINDING 17: the root and prosemirror_model both define `SchemaSpec` (and
// its Methods trait): a glob import of both is ambiguous.
use bindings::prosemirror_model::SchemaSpec;
use bindings::prosemirror_state::*;
use bindings::prosemirror_view::*;
use bindings::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[link(wasm_import_module = "env")]
unsafe extern "C" {
    #[link_name = "check"]
    fn check_at(ok: i32, line: u32);
}
macro_rules! check {
    ($c:expr) => {
        unsafe { check_at($c as i32, line!()) }
    };
}

// A plain JavaScript object of values, made with the one dictionary type the
// bindings have for it.
fn object(entries: &[(&str, JsValue)]) -> NodeSpec {
    let o = NodeSpec::create();
    for (k, v) in entries {
        o.set_at(k, v.clone());
    }
    o
}
fn value(o: &NodeSpec) -> JsValue {
    JsValue::from(o)
}
fn list(items: Vec<JsValue>) -> JsValue {
    JsValue(JsArray::<JsValue>::from(items).0.clone())
}

fn make_schema() -> Schema {
    // How a node and a mark are drawn in a view: [tag, 0] puts the content in the tag.
    let tag = |name: &'static str| {
        DOMOutputSpec::Array(JsArray::<JsValue>::from(vec![
            JsValue::from(name),
            JsValue::from(0.0),
        ]))
    };
    let doc = NodeSpec::create();
    doc.set_content(Some("paragraph+"));
    doc.set_to_dom(Some(&NodeSpecToDOMFn::new(move |_: Node| tag("div"))));
    let paragraph = NodeSpec::create();
    paragraph.set_content(Some("text*"));
    paragraph.set_group(Some("block"));
    paragraph.set_to_dom(Some(&NodeSpecToDOMFn::new(move |_: Node| tag("p"))));
    let text = NodeSpec::create();
    text.set_group(Some("inline"));
    let strong = MarkSpec::create();
    strong.set_to_dom(Some(&MarkSpecToDOMFn::new(
        move |_: Mark, _: bool| tag("strong"),
    )));
    let spec = SchemaSpec::create();
    spec.set_nodes([("doc", &doc), ("paragraph", &paragraph), ("text", &text)]);
    spec.set_marks([("strong", &strong)]);
    Schema::new(&spec)
}

fn json_doc(paragraphs: &[&str]) -> JsValue {
    let text = |t: &str| value(&object(&[("type", "text".into()), ("text", t.into())]));
    let para = |t: &str| {
        value(&object(&[
            ("type", "paragraph".into()),
            ("content", list(vec![text(t)])),
        ]))
    };
    value(&object(&[
        ("type", "doc".into()),
        (
            "content",
            list(paragraphs.iter().map(|t| para(t)).collect()),
        ),
    ]))
}

fn nodes(items: Vec<Node>) -> JsArray<Node> {
    JsArray::<Node>::from(items)
}

#[unsafe(no_mangle)]
pub extern "C" fn run() {
    let schema = make_schema();
    check!(schema.top_node_type().name() == "doc");

    // A document from JSON, and one built by hand: the same.
    let doc = Node::from_json(&schema, json_doc(&["hello", "world"]));
    check!(doc.child_count() == 2.0);
    check!(doc.text_content() == "helloworld");
    check!(doc.node_size() == 16.0);
    check!(doc.child(0.0).node_size() == 7.0 && doc.child(0.0).content().size() == 5.0);
    check!(doc.child(1.0).text_content() == "world");
    check!(doc.child(0.0).type_().name() == "paragraph" && doc.child(0.0).is_textblock());
    let paragraph =
        |t: &str| schema.node_with("paragraph", None, nodes(vec![schema.text(t)]), None);
    let built = schema.node_with(
        "doc",
        None,
        nodes(vec![paragraph("hello"), paragraph("world")]),
        None,
    );
    check!(built.js_eq(&doc));
    check!(!built.js_eq(&Node::from_json(&schema, json_doc(&["hello", "there"]))));

    // Positions: inside the first paragraph, after "he".
    let at = doc.resolve(3.0);
    check!(
        at.parent().type_().name() == "paragraph" && at.parent_offset() == 2.0 && at.depth() == 1.0
    );
    check!(at.start() == 1.0 && at.end() == 6.0);
    check!(
        doc.node_at(1.0)
            .is_some_and(|n| n.is_text() && n.text().as_deref() == Some("hello"))
    );
    check!(doc.text_between_with(0.0, 14.0, Some(" "), None) == "hello world");

    // Marks: a bold run in a paragraph.
    let strong = schema.mark("strong");
    let marks = JsArray::<Mark>::from(vec![strong.clone()]);
    let bold = schema.text_with("bold", Some(&marks));
    check!(bold.marks().len() == 1 && bold.marks().get(0).type_().name() == "strong");
    check!(strong.is_in_set(bold.marks()));
    check!(!strong.is_in_set(Mark::none()));
    let mixed = schema.node_with(
        "doc",
        None,
        nodes(vec![schema.node_with(
            "paragraph",
            None,
            nodes(vec![schema.text("plain "), bold.clone()]),
            None,
        )]),
        None,
    );
    check!(
        mixed.range_has_mark(7.0, 9.0, MarkMarkType::Mark(strong.clone()))
            && !mixed.range_has_mark(1.0, 5.0, MarkMarkType::Mark(strong.clone()))
    );
    check!(mixed.text_content() == "plain bold" && mixed.child(0.0).child_count() == 2.0);

    // The schema refuses what it does not allow: a call that throws gives
    // a zero value and the exception is kept.
    let unknown = js::attempt(|| schema.node_with("heading", None, nodes(vec![]), None));
    check!(unknown.is_err_and(|e| e.message().contains("Unknown node type")));
    let bad = js::attempt(|| {
        schema.nodes().at("doc").unwrap().create_checked_with(
            None,
            nodes(vec![schema.text("loose")]),
            None,
        )
    });
    check!(bad.is_err_and(|e| e.message().contains("Invalid content")));

    state_and_transactions(&schema, &doc);
    editor_view(&doc);
}

fn state_and_transactions(schema: &Schema, doc: &Node) {
    // A state on the document: the selection starts at the document's start.
    let config = EditorStateConfig::create();
    config.set_doc(Some(doc));
    let state = EditorState::create(&config);
    check!(state.doc().js_eq(doc));
    check!(state.selection().anchor() == 1.0 && state.selection().empty());
    check!(state.schema().top_node_type().name() == "doc");

    // A transaction inserts text at a position; applying it gives a new state
    // and leaves the old one as it was.
    let tr = state.tr();
    tr.insert_text_with("big ", Some(1.0), None);
    check!(tr.doc_changed() && tr.steps().len() == 1);
    let next = state.apply(&tr);
    check!(next.doc().text_content() == "big helloworld");
    check!(state.doc().text_content() == "helloworld");
    check!(next.doc().child(0.0).text_content() == "big hello");

    // Marks over a range, by a transform step: bold the word "hello".
    let strong = schema.mark("strong");
    let bolded = next.apply(&next.tr().add_mark(5.0, 10.0, &strong));
    let first = bolded.doc().child(0.0);
    check!(first.child_count() == 2.0 && first.child(1.0).text().as_deref() == Some("hello"));
    check!(first.child(1.0).marks().len() == 1 && first.child(0.0).marks().is_empty());
    let tr = bolded.tr();
    tr.remove_mark_with(5.0, 10.0, MarkMarkType::Mark(strong.clone()));
    let cleared = bolded.apply(&tr);
    check!(cleared.doc().child(0.0).child_count() == 1.0);

    // A selection set in a transaction, and replaced text.
    let selecting = state.tr();
    selecting.set_selection(&TextSelection::create_with(&state.doc(), 2.0, Some(5.0)));
    let selected = state.apply(&selecting);
    check!(
        selected.selection().from() == 2.0
            && selected.selection().to() == 5.0
            && !selected.selection().empty()
    );
    check!({
        let slice = selected.selection().content();
        slice.size() == 3.0 && slice.content().text_between(0.0, slice.content().size()) == "ell"
    });
    let tr = selected.tr();
    tr.replace_selection_with(&schema.text("ipp"));
    let replaced = selected.apply(&tr);
    check!(replaced.doc().child(0.0).text_content() == "hippo");
    check!(replaced.selection().empty());

    // Deleting a range; and a whole paragraph inserted.
    let tr = state.tr();
    tr.delete(1.0, 6.0);
    let deleted = state.apply(&tr);
    check!(
        deleted.doc().child(0.0).child_count() == 0.0 && deleted.doc().text_content() == "world"
    );
    let tr = state.tr();
    tr.insert(
        0.0,
        nodes(vec![schema.node_with(
            "paragraph",
            None,
            nodes(vec![schema.text("zero")]),
            None,
        )]),
    );
    let inserted = state.apply(&tr);
    check!(
        inserted.doc().child_count() == 3.0 && inserted.doc().child(0.0).text_content() == "zero"
    );

    // A plugin with state: counts the transactions applied.
    let key = PluginKey::<JsValue>::new_with(Some("counter"));
    let field = StateField::<JsValue>::create();
    field.set_init(&StateFieldInitFn::new(|_, _| JsValue::from(0.0)));
    field.set_apply(&StateFieldApplyFn::new(|_tr, count: JsValue, _old, _new| {
        JsValue::from(count.as_f64() + 1.0)
    }));
    let spec = PluginSpec::<JsValue>::create();
    spec.set_state(Some(&field));
    spec.set_key(Some(&key));
    let plugin = Plugin::new(&spec);
    let with_plugin = EditorStateConfig::create();
    with_plugin.set_doc(Some(doc));
    with_plugin.set_plugins(Some(&JsArray::<Plugin<JsValue>>::from(vec![
        plugin.clone(),
    ])));
    let counted = EditorState::create(&with_plugin);
    check!(key.get_state(&counted).is_some_and(|c| c.as_f64() == 0.0));
    let tr = counted.tr();
    tr.insert_text_with("a", Some(1.0), None);
    let counted = counted.apply(&tr);
    let tr = counted.tr();
    tr.insert_text_with("b", Some(1.0), None);
    let counted = counted.apply(&tr);
    check!(key.get_state(&counted).is_some_and(|c| c.as_f64() == 2.0));
    check!(
        plugin
            .get_state(&counted)
            .is_some_and(|c| c.as_f64() == 2.0)
    );
    check!(counted.plugins().len() == 1 && counted.doc().text_content() == "bahelloworld");
}

fn editor_view(doc: &Node) {
    // A view on the page's #stage. Its dispatchTransaction is ours: it counts
    // the transactions and applies each to the view's state.
    let config = EditorStateConfig::create();
    config.set_doc(Some(doc));
    let state = EditorState::create(&config);
    let seen = Rc::new(Cell::new(0));
    let view_cell: Rc<RefCell<Option<EditorView>>> = Rc::new(RefCell::new(None));
    let dispatch = DispatchTransactionFn::new({
        let (seen, view_cell) = (seen.clone(), view_cell.clone());
        move |tr: Transaction| {
            seen.set(seen.get() + 1);
            if let Some(view) = view_cell.borrow().as_ref() {
                let next = view.state().apply(&tr);
                view.update_state(&next);
            }
        }
    });
    let props = DirectEditorProps::create();
    props.set_state(&state);
    props.set_dispatch_transaction(Some(&dispatch));
    let place = EditorViewNewPlaceObject::create().with_mount(&test_page::element("stage"));
    let view = EditorView::new(Some(Place::from(place)), &props);
    *view_cell.borrow_mut() = Some(view.clone());
    check!(view.state().doc().js_eq(doc));
    check!(view.editable() && !view.is_destroyed());
    check!(view.dom().0.0 != 0);

    // Pasting text goes through the view's own handling to our dispatch.
    check!(view.paste_text("Z"));
    check!(seen.get() == 1);
    check!(view.state().doc().text_content() == "Zhelloworld");

    // The view's own `dispatch` reaches ours.
    view.dispatch(&view.state().tr());
    check!(seen.get() == 2);

    // A new state set directly does not dispatch.
    let replaced = EditorStateConfig::create();
    replaced.set_doc(Some(&doc.child(1.0).type_().schema().node_with(
        "doc",
        None,
        nodes(vec![doc.child(1.0)]),
        None,
    )));
    view.update_state(&EditorState::create(&replaced));
    check!(view.state().doc().text_content() == "world" && seen.get() == 2);

    view.destroy();
    check!(view.is_destroyed());
}
