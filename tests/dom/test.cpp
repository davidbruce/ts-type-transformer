// Drives a real page through the generated DOM bindings; every CHECK must hold.
#include <string>

#include "areas/core/AddEventListenerOptions.hpp"
#include "areas/core/DOMRect.hpp"
#include "areas/core/DOMTokenList.hpp"
#include "areas/core/Document.hpp"
#include "areas/core/NodeListOf.hpp"
#include "areas/core/functions.hpp"
#include "areas/css/CSSStyleDeclaration.hpp"
#include "areas/events/Event.hpp"
#include "areas/events/PointerEvent.hpp"
#include "areas/html/HTMLButtonElement.hpp"
#include "areas/html/HTMLCollection.hpp"
#include "areas/html/HTMLDivElement.hpp"
#include "areas/html/HTMLElement.hpp"
#include "areas/html/HTMLParagraphElement.hpp"

extern "C" TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

extern "C" __attribute__((export_name("run"))) void run() {
    Document doc = get_document();
    HTMLElement body = doc.body();

    // A typed key selects the element type: createElement("div") is an HTMLDivElement,
    // which is an HTMLElement, an Element, a Node...
    HTMLDivElement div = doc.createElement(HTMLElementTagNameMapKeys::div);
    HTMLElement box = div;
    box.id("made-by-cpp");
    box.textContent("hello from cpp");
    ts::Array classes = ts::Array::create();
    classes.push(std::string("box"));
    box.classList().add(classes);
    body.appendChild(box);
    CHECK(doc.getElementById("made-by-cpp")->h == box.h);
    CHECK(box.classList().contains("box"));

    // Read an element that was already in the page, and copy its text.
    HTMLElement seed = *doc.getElementById("seed");
    CHECK(seed.textContent() == "seed text");
    CHECK(seed.getAttribute("data-x") == "42");
    CHECK(!seed.getAttribute("data-missing"));
    HTMLElement copy = doc.createElement(HTMLElementTagNameMapKeys::p);
    copy.textContent(seed.textContent());
    box.appendChild(copy);

    // An element where a std::optional<Node> is wanted: a derived-to-base conversion.
    HTMLParagraphElement extra = doc.createElement(HTMLElementTagNameMapKeys::p);
    box.insertBefore(extra, copy);
    CHECK(box.children().length() == 2);
    extra.remove();
    CHECK(box.children().length() == 1);

    // Generic types are class templates: querySelectorAll("p") is a NodeListOf<HTMLParagraphElement>.
    NodeListOf<HTMLParagraphElement> paragraphs = box.querySelectorAll(HTMLElementTagNameMapKeys::p);
    CHECK(paragraphs.length() == 1);
    CHECK(paragraphs.item(0).textContent() == "seed text");
    // A default type argument: querySelectorAll(selectors) is a NodeListOf<Element>.
    NodeListOf<Element> all = box.querySelectorAll("p, button");
    CHECK(all.length() == 1);

    // A typed key selects the event type: the listener of "click" takes a PointerEvent.
    HTMLElement button = doc.createElement(HTMLElementTagNameMapKeys::button);
    button.textContent("press");
    box.appendChild(button);
    int clicks = 0;
    ts::Listener<PointerEvent> onClick = [&](PointerEvent) {
        clicks++;
        box.setAttribute("data-count", std::to_string(clicks));
    };
    button.addEventListener(HTMLElementEventMapKeys::click, onClick);
    button.click();
    button.click();
    CHECK(clicks == 2);
    button.removeEventListener(HTMLElementEventMapKeys::click, onClick);
    button.click();
    CHECK(clicks == 2);

    // An options object, { once: true }, and events made and dispatched here.
    int pings = 0;
    AddEventListenerOptions once = AddEventListenerOptions::create();
    once.once(ts::Value::boolean(true));
    EventListener onPing = [&](Event) {
        pings++;
        box.setAttribute("data-pings", std::to_string(pings));
    };
    box.addEventListener("ping", onPing, once);
    box.dispatchEvent(Event::create("ping"));
    box.dispatchEvent(Event::create("ping"));
    CHECK(pings == 1);

    // Style, layout and the window.
    box.style().setProperty("color", "red");
    CHECK(box.style().color() == "red");
    CHECK(box.getBoundingClientRect().width() > 0);
    CHECK(box.children().length() == 2);
    CHECK(get_innerWidth() > 0);
    doc.title("driven by cpp");
}
