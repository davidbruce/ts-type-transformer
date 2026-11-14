// Drives a real page through the generated DOM bindings; every CHECK must hold.
#include <string>

// A header brings in every type its members name.
#include "areas/core/Document.hpp"
#include "areas/core/functions.hpp"

extern "C" TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

// A real request, awaited: the page's own verify.mjs.
static int fetched_status = 0;
static bool fetched_text = false;
static ts::Task fetch_page() {
    ts::Result<Response> r = co_await fetch("./verify.mjs");
    if (!r) co_return;
    fetched_status = int(r->status());
    ts::Result<std::string> text = co_await r->text();
    fetched_text = text && text->find("verify") != std::string::npos;
}

extern "C" __attribute__((export_name("finish"))) void finish() {
    CHECK(fetched_status == 200 && fetched_text);
}

extern "C" __attribute__((export_name("run"))) void run() {
    Document doc = get_document();
    HTMLElement body = doc.body();

    // A typed key selects the element type: createElement("div") is an HTMLDivElement,
    // which is an HTMLElement, an Element, a Node...
    HTMLDivElement div = doc.createElement(HTMLElementTagNameMapKeys::div);
    HTMLElement box = div;
    box.id("made-by-cpp");
    box.textContent("hello from cpp");
    box.classList().add({"box", "made"});
    body.appendChild(box);
    CHECK(doc.getElementById("made-by-cpp") == box);
    CHECK(box.classList().contains("box") && box.classList().contains("made"));

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
    // Frees the lambda.
    onClick.release();
    CHECK(onClick.h == 0);

    // An options object, { once: true }, and events made and dispatched here.
    int pings = 0;
    AddEventListenerOptions once = AddEventListenerOptions::create();
    once.once(true);
    {
        // Released at the end of the block.
        ts::Scoped<EventListener> onPing = [&](Event) {
            pings++;
            box.setAttribute("data-pings", std::to_string(pings));
        };
        box.addEventListener("ping", onPing, once);
        box.dispatchEvent(Event::create("ping"));
        box.dispatchEvent(Event::create("ping"));
    }
    CHECK(pings == 1);

    // Style, layout and the window.
    box.style().setProperty("color", "red");
    CHECK(box.style().color() == "red");
    CHECK(box.getBoundingClientRect().width() > 0);
    CHECK(box.children().length() == 2);
    CHECK(get_innerWidth() > 0);

    // Collections iterate.
    int children = 0;
    std::string tags;
    for (Element child : box.children()) {
        children++;
        tags += child.tagName() + " ";
    }
    CHECK(children == 2 && tags == "P BUTTON ");
    doc.title("driven by cpp");

    // An invalid selector throws: the call returns nothing, and the exception is kept.
    CHECK(!box.querySelector("<bad"));
    std::optional<ts::Error> bad = ts::take_error();
    CHECK(bad && bad->message().find("not a valid selector") != std::string::npos);
    fetch_page();

    // Handles are counted: elements and strings fetched in a loop are freed.
    uint32_t live = ts::live_handles();
    for (int i = 0; i < 500; i++) {
        HTMLElement found = *doc.getElementById("made-by-cpp");
        CHECK(found.classList().contains("box") && found.id() == "made-by-cpp");
    }
    CHECK(ts::live_handles() == live);
}
