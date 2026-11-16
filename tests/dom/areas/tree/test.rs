// The DOM tree from Rust: nodes made, attached, moved, cloned, searched and
// walked. Every check! must hold.
#![no_main]
use bindings::*;

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

#[unsafe(no_mangle)]
pub extern "C" fn run() {
    let doc = document();
    let body = doc.body();

    // A list built from elements and text.
    let list: HTMLUListElement = doc.create_element(HTMLElementTagNameMap::UL, None);
    list.set_id("list");
    for word in ["one", "two", "three"] {
        let item: HTMLLIElement = doc.create_element(HTMLElementTagNameMap::LI, None);
        item.append_child(doc.create_text_node(word));
        item.set_attribute("data-word", word);
        list.append_child(item);
    }
    body.append_child(list.clone());
    check!(list.child_element_count() == 3.0);
    check!(list.text_content() == "onetwothree");
    check!(list.first_element_child().unwrap().text_content() == "one");
    check!(list.last_element_child().unwrap().get_attribute("data-word").as_deref() == Some("three"));

    // Parents, siblings and containment.
    let two = list.children().item(1.0).unwrap();
    check!(two.parent_element() == Some(list.clone().into()));
    check!(two.previous_element_sibling().unwrap().text_content() == "one");
    check!(two.next_element_sibling().unwrap().text_content() == "three");
    check!(body.contains(Some(two.as_ref())));
    check!(two.matches("li[data-word=two]"));
    check!(two.closest("ul").map(|u| u.id()).as_deref() == Some("list"));

    // Moved: appending a node that is in the tree moves it.
    list.append_child(two.clone());
    check!(list.last_element_child().unwrap().text_content() == "two");
    check!(list.text_content() == "onethreetwo");

    // Removed and replaced.
    let first = list.first_element_child().unwrap();
    let zero: HTMLLIElement = doc.create_element(HTMLElementTagNameMap::LI, None);
    zero.set_text_content(Some("zero"));
    list.replace_child(&zero, first.clone());
    check!(first.parent_node().is_none());
    check!(list.first_element_child().unwrap().text_content() == "zero");
    two.remove();
    check!(list.child_element_count() == 2.0);

    // A fragment's children move in at once; it is left empty.
    let fragment = doc.create_document_fragment();
    for word in ["a", "b"] {
        let item: HTMLLIElement = doc.create_element(HTMLElementTagNameMap::LI, None);
        item.set_text_content(Some(word));
        fragment.append_child(item);
    }
    list.append_child(fragment.clone());
    check!(list.child_element_count() == 4.0);
    check!(fragment.child_nodes().length() == 0.0);

    // A deep clone copies the subtree; a shallow one does not.
    let deep = list.clone_node(Some(true));
    let shallow = list.clone_node(None);
    // A Node's text may be null; an element's is a string.
    check!(deep.text_content() == Some(list.text_content()));
    check!(shallow.child_nodes().length() == 0.0);
    check!(deep.is_equal_node(Some(list.as_ref())) && !deep.is_same_node(Some(list.as_ref())));

    // Markup parsed and written.
    let host: HTMLDivElement = doc.create_element(HTMLElementTagNameMap::DIV, None);
    host.set_inner_html("<p class=\"x\">hi <b>there</b></p>");
    check!(host.query_selector("b").unwrap().text_content() == "there");
    check!(host.inner_html() == "<p class=\"x\">hi <b>there</b></p>");
    let p = host.first_element_child().unwrap();
    check!(p.outer_html().starts_with("<p class=\"x\">"));
    check!(p.class_list().contains("x"));
    p.class_list().toggle("y", None);
    check!(p.get_attribute("class").as_deref() == Some("x y"));

    // Positions compared: `two` is gone; `zero` precedes `a`.
    let a = list.children().item(2.0).unwrap();
    let position = zero.compare_document_position(&a);
    check!(position as u32 & 4 != 0);

    // A walk over the list's elements.
    let walker = doc.create_tree_walker(&list, Some(1.0), None);
    let mut seen = Vec::new();
    while let Some(n) = walker.next_node() {
        seen.push(n.text_content().unwrap_or_default());
    }
    check!(seen == ["zero", "three", "a", "b"]);
}
