// The DOM tree from C: nodes made, attached, moved, cloned, searched and
// walked. Every CHECK must hold.
#include "bindings.h"

TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)
#define STR(s, text) JsString_equals((s), (text))

static HTMLLIElement item(Document doc, const char* word) {
    HTMLLIElement li = Document_createElement_li(doc);
    Node_appendChild(li, Document_createTextNode(doc, word));
    Element_setAttribute(li, "data-word", word);
    return li;
}

__attribute__((export_name("run"))) void run(void) {
    Document doc = get_document();
    HTMLElement body = Document_get_body(doc);

    // A list built from elements and text.
    HTMLUListElement list = Document_createElement_ul(doc);
    Element_set_id(list, "list");
    const char* words[] = { "one", "two", "three" };
    for (int i = 0; i < 3; i++) Node_appendChild(list, item(doc, words[i]));
    CHECK(!Node_get_isConnected(list));
    Node_appendChild(body, list);
    CHECK(Node_get_isConnected(list));
    CHECK(ParentNode_get_childElementCount(list) == 3);
    CHECK(STR(Node_get_textContent(list), "onetwothree"));
    CHECK(STR(Node_get_textContent(ParentNode_get_firstElementChild(list)), "one"));
    CHECK(STR(Element_getAttribute(ParentNode_get_lastElementChild(list), "data-word"), "three"));
    CHECK(Node_hasChildNodes(list));
    CHECK(Node_get_nodeType(list) == Node_ELEMENT_NODE);
    CHECK(STR(Node_get_nodeName(list), "UL"));
    CHECK(STR(Element_get_tagName(list), "UL"));
    CHECK(js_same(Node_get_ownerDocument(list), doc));

    // Parents, siblings and containment.
    HTMLCollection kids = ParentNode_get_children(list);
    Element two = HTMLCollectionBase_item(kids, 1);
    CHECK(js_same(Node_get_parentElement(two), list));
    CHECK(js_same(Node_get_parentNode(two), list));
    CHECK(STR(Node_get_textContent(NonDocumentTypeChildNode_get_previousElementSibling(two)), "one"));
    CHECK(STR(Node_get_textContent(NonDocumentTypeChildNode_get_nextElementSibling(two)), "three"));
    CHECK(js_same(Node_get_previousSibling(two), NonDocumentTypeChildNode_get_previousElementSibling(two)));
    CHECK(Node_contains(body, two));
    CHECK(!Node_contains(two, body));
    CHECK(Element_matches(two, "li[data-word=two]"));
    CHECK(!Element_matches(two, "li.nope"));
    CHECK(STR(Element_get_id(Element_closest(two, "ul")), "list"));
    CHECK(Element_closest(two, "table").h == 0);

    // Moved: appending a node that is in the tree moves it.
    Node_appendChild(list, two);
    CHECK(STR(Node_get_textContent(ParentNode_get_lastElementChild(list)), "two"));
    CHECK(STR(Node_get_textContent(list), "onethreetwo"));

    // Removed and replaced.
    Element first = ParentNode_get_firstElementChild(list);
    HTMLLIElement zero = Document_createElement_li(doc);
    Node_set_textContent(zero, "zero");
    Node_replaceChild(list, zero, first);
    CHECK(Node_get_parentNode(first).h == 0);
    CHECK(STR(Node_get_textContent(ParentNode_get_firstElementChild(list)), "zero"));
    ChildNode_remove(two);
    CHECK(ParentNode_get_childElementCount(list) == 2);
    CHECK(Node_get_parentNode(two).h == 0);
    Node_removeChild(list, zero);
    CHECK(ParentNode_get_childElementCount(list) == 1);
    // insertBefore puts zero back in front of "three".
    Node_insertBefore(list, zero, ParentNode_get_firstElementChild(list));
    CHECK(STR(Node_get_textContent(list), "zerothree"));
    CHECK(js_same(Node_get_nextSibling(zero), ParentNode_get_lastElementChild(list)));

    // A fragment's children move in at once; it is left empty.
    DocumentFragment fragment = Document_createDocumentFragment(doc);
    Node_appendChild(fragment, item(doc, "a"));
    Node_appendChild(fragment, item(doc, "b"));
    CHECK(Node_get_nodeType(fragment) == Node_DOCUMENT_FRAGMENT_NODE);
    CHECK(STR(Node_get_textContent(fragment), "ab"));
    Node_appendChild(list, fragment);
    CHECK(ParentNode_get_childElementCount(list) == 4);
    CHECK(NodeListOf_item(Node_get_childNodes(fragment), 0).h == 0);
    CHECK(!Node_hasChildNodes(fragment));

    // append and prepend take nodes or strings.
    ParentNode_append(list, "tail", item(doc, "z"));
    CHECK(ParentNode_get_childElementCount(list) == 5);
    CHECK(STR(Node_get_textContent(list), "zerothreeabtailz"));
    CHECK(Node_get_nodeType(Node_get_childNodes(list).h ? NodeListOf_item(Node_get_childNodes(list), 4) : (Node){0}) == Node_TEXT_NODE);

    // A deep clone copies the subtree; a shallow one does not.
    Node deep = Node_cloneNode(list, js_some_bool(true));
    Node shallow = Node_cloneNode(list);
    CHECK(STR(Node_get_textContent(deep), "zerothreeabtailz"));
    CHECK(!Node_hasChildNodes(shallow));
    CHECK(Node_isEqualNode(deep, list) && !Node_isSameNode(deep, list));
    CHECK(Node_isSameNode(list, list));
    Element deep_el = Element_cast(deep);
    CHECK(deep_el.h != 0 && Text_cast(deep).h == 0);
    CHECK(STR(Element_get_id(deep_el), "list"));
    CHECK(Node_get_parentNode(deep).h == 0);

    // Markup parsed and written.
    HTMLDivElement host = Document_createElement_div(doc);
    Element_set_innerHTML(host, "<p class=\"x\">hi <b>there</b></p>");
    CHECK(STR(Node_get_textContent(ParentNode_querySelector(host, "b")), "there"));
    CHECK(STR(Element_get_innerHTML(host), "<p class=\"x\">hi <b>there</b></p>"));
    Element p = ParentNode_get_firstElementChild(host);
    char markup[64];
    JsString outer = Element_get_outerHTML(p);
    JsString_copy(outer, markup, sizeof markup);
    CHECK(markup[0] == '<' && markup[1] == 'p' && JsString_len(outer) == 32);
    CHECK(DOMTokenList_contains(Element_get_classList(p), "x"));
    DOMTokenList classes = Element_get_classList(p);
    CHECK(DOMTokenList_toggle(classes, "y", js_none_bool()));
    CHECK(STR(Element_getAttribute(p, "class"), "x y"));
    CHECK(DOMTokenList_get_length(classes) == 2);
    CHECK(DOMTokenList_replace(classes, "x", "w"));
    CHECK(STR(Element_get_className(p), "w y"));
    DOMTokenList_remove(classes, "w");
    CHECK(STR(DOMTokenList_toString(classes), "y"));
    NodeListOf bs = ParentNode_querySelectorAll(host, "p, b");
    int found = 0;
    JsIterator it = NodeListOf_iterate(bs);
    Node n;
    while (NodeListOf_iterate_next(it, &n)) found++;
    CHECK(found == 2);
    Element_insertAdjacentHTML(host, InsertPosition_beforeend, "<i>end</i>");
    CHECK(ParentNode_get_childElementCount(host) == 2);
    CHECK(STR(Node_get_textContent(ParentNode_get_lastElementChild(host)), "end"));

    // Positions compared: `zero` precedes `a`.
    Element a = HTMLCollectionBase_item(ParentNode_get_children(list), 2);
    CHECK(((int)Node_compareDocumentPosition(zero, a) & Node_DOCUMENT_POSITION_FOLLOWING) != 0);
    CHECK(((int)Node_compareDocumentPosition(a, zero) & Node_DOCUMENT_POSITION_PRECEDING) != 0);
    CHECK(((int)Node_compareDocumentPosition(body, a) & Node_DOCUMENT_POSITION_CONTAINED_BY) != 0);
    CHECK(((int)Node_compareDocumentPosition(a, host) & Node_DOCUMENT_POSITION_DISCONNECTED) != 0);

    // A walk over the list's elements (SHOW_ELEMENT is 1).
    TreeWalker walker = Document_createTreeWalker(doc, list, js_some_double(1), (NodeFilter){ .tag = -1 });
    CHECK(js_same(TreeWalker_get_root(walker), list));
    const char* expect[] = { "zero", "three", "a", "b", "z" };
    int seen = 0;
    Node cur;
    while ((cur = TreeWalker_nextNode(walker)).h) {
        CHECK(STR(Node_get_textContent(cur), expect[seen]));
        seen++;
    }
    CHECK(seen == 5);
}
