// The DOM tree from C++: nodes made, attached, moved, cloned, searched and
// walked. Every CHECK must hold.
#include <string>

#include "areas/core/Document.hpp"
#include "areas/core/functions.hpp"

extern "C" TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

extern "C" __attribute__((export_name("run"))) void run() {
    Document doc = get_document();
    HTMLElement body = doc.body();

    // A list built from elements and text.
    HTMLUListElement list = doc.createElement(HTMLElementTagNameMapKeys::ul);
    list.id("list");
    for (const char* word : {"one", "two", "three"}) {
        HTMLLIElement item = doc.createElement(HTMLElementTagNameMapKeys::li);
        item.appendChild(doc.createTextNode(word));
        item.setAttribute("data-word", word);
        list.appendChild(item);
    }
    body.appendChild(list);
    CHECK(list.childElementCount() == 3);
    CHECK(list.textContent() == "onetwothree");
    CHECK(list.firstElementChild()->textContent() == "one");
    CHECK(list.lastElementChild()->getAttribute("data-word") == "three");

    // Parents, siblings and containment.
    Element two = *list.children().item(1);
    CHECK(two.parentElement() && *two.parentElement() == list);
    CHECK(two.previousElementSibling()->textContent() == "one");
    CHECK(two.nextElementSibling()->textContent() == "three");
    CHECK(body.contains(two));
    CHECK(!list.contains(body));
    CHECK(two.matches("li[data-word=two]"));
    CHECK(!two.matches("p"));
    CHECK(two.closest("ul")->id() == "list");
    CHECK(two.tagName() == "LI" && two.nodeName() == "LI" && two.nodeType() == Node::ELEMENT_NODE);
    CHECK(two.isConnected());

    // Moved: appending a node that is in the tree moves it.
    list.appendChild(two);
    CHECK(list.lastElementChild()->textContent() == "two");
    CHECK(list.textContent() == "onethreetwo");

    // insertBefore, removed and replaced.
    HTMLLIElement zero = doc.createElement(HTMLElementTagNameMapKeys::li);
    zero.textContent("zero");
    Element first = *list.firstElementChild();
    list.insertBefore(zero, first);
    CHECK(list.firstElementChild()->textContent() == "zero");
    CHECK(list.childElementCount() == 4);
    list.removeChild(first);
    CHECK(!first.parentNode());
    CHECK(!first.isConnected());
    HTMLLIElement nil = doc.createElement(HTMLElementTagNameMapKeys::li);
    nil.textContent("nil");
    list.replaceChild(nil, zero);
    CHECK(list.firstElementChild()->textContent() == "nil");
    two.remove();
    CHECK(list.childElementCount() == 2);
    CHECK(list.textContent() == "nilthree");

    // A fragment's children move in at once; it is left empty.
    DocumentFragment fragment = doc.createDocumentFragment();
    for (const char* word : {"a", "b"}) {
        HTMLLIElement item = doc.createElement(HTMLElementTagNameMapKeys::li);
        item.textContent(word);
        fragment.appendChild(item);
    }
    CHECK(fragment.childElementCount() == 2);
    list.appendChild(fragment);
    CHECK(list.childElementCount() == 4);
    CHECK(fragment.childNodes().length() == 0);
    CHECK(!fragment.hasChildNodes());

    // A deep clone copies the subtree; a shallow one does not.
    Node deep = list.cloneNode(true);
    Node shallow = list.cloneNode();
    CHECK(deep.textContent() == list.textContent());
    CHECK(shallow.childNodes().length() == 0);
    CHECK(deep.isEqualNode(list) && !deep.isSameNode(list) && deep.isSameNode(deep));
    CHECK(!deep.parentNode());

    // Markup parsed and written.
    HTMLDivElement host = doc.createElement(HTMLElementTagNameMapKeys::div);
    host.innerHTML("<p class=\"x\">hi <b>there</b></p>");
    CHECK(host.querySelector("b")->textContent() == "there");
    CHECK(host.innerHTML() == "<p class=\"x\">hi <b>there</b></p>");
    Element p = *host.firstElementChild();
    CHECK(p.outerHTML().starts_with("<p class=\"x\">"));
    CHECK(p.classList().contains("x"));
    p.classList().toggle("y");
    CHECK(p.getAttribute("class") == "x y");
    p.classList().replace("x", "z");
    CHECK(p.className() == "z y" && p.classList().length() == 2);
    p.classList().remove({"y"});
    CHECK(p.className() == "z");
    CHECK(!p.hasAttribute("id"));
    p.toggleAttribute("hidden");
    CHECK(p.hasAttribute("hidden"));
    p.removeAttribute("hidden");
    CHECK(!p.hasAttribute("hidden"));

    // Text, comment and the other node types.
    Text text = doc.createTextNode("plain");
    Comment comment = doc.createComment("note");
    CHECK(text.nodeType() == Node::TEXT_NODE && comment.nodeType() == Node::COMMENT_NODE);
    CHECK(text.nodeValue() == "plain" && comment.data() == "note");
    CHECK(doc.nodeType() == Node::DOCUMENT_NODE);

    // Positions compared: `nil` precedes `a`, which is contained by the list.
    Element a = *list.children().item(2);
    double position = nil.compareDocumentPosition(a);
    CHECK((uint32_t(position) & uint32_t(Node::DOCUMENT_POSITION_FOLLOWING)) != 0);
    double inside = a.compareDocumentPosition(list);
    CHECK((uint32_t(inside) & uint32_t(Node::DOCUMENT_POSITION_CONTAINS)) != 0);
    CHECK(a.getRootNode() == doc);

    // A walk over the list's elements.
    TreeWalker walker = doc.createTreeWalker(list, 1);
    std::string seen;
    while (std::optional<Node> n = walker.nextNode()) seen += *n->textContent() + ",";
    CHECK(seen == "nil,three,a,b,");
    CHECK(walker.root() == list);
    walker.currentNode(list);
    CHECK(walker.firstChild()->textContent() == "nil");
    CHECK(walker.nextSibling()->textContent() == "three");

    // Queries over the tree.
    CHECK(doc.querySelectorAll("#list li").length() == 4);
    CHECK(list.getElementsByTagName("li").length() == 4);
    CHECK(!doc.querySelector("#nothing-here"));
}
