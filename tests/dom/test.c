// Drives a real page through the generated DOM bindings; every CHECK must hold.
#include "bindings.h"

TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

static bool str_eq(JsString s, const char* expected) {
    char buf[128];
    uint32_t n = JsString_len(s);
    if (n != ts_strlen(expected) || n > sizeof buf) return false;
    JsString_read(s, buf);
    for (uint32_t i = 0; i < n; i++)
        if (buf[i] != expected[i]) return false;
    return true;
}

// What a listener needs: where to count, and where to show the count.
typedef struct {
    HTMLElement box;
    const char* attribute;
    int count;
} Counter;

static void bump(Counter* counter) {
    counter->count++;
    char digits[2] = { '0' + counter->count, 0 };
    HTMLElement_setAttribute(counter->box, counter->attribute, digits);
}

static JsValue on_click(void* ctx, JsValue ev) {
    bump(ctx);
    return (JsValue){ 0 };
}

static void on_ping(void* ctx, Event ev) { bump(ctx); }

static Counter clicks, pings;

__attribute__((export_name("run"))) void run(void) {
    Document doc = get_document();
    HTMLElement body = Document_get_body(doc);

    // Create an element and put it in the page.
    HTMLElement box = Document_createElement_with_string(doc, "div");
    HTMLElement_set_id(box, "made-by-c");
    HTMLElement_set_textContent(box, "hello from c");
    JsArray classes = JsArray_new();
    JsArray_push(classes, js_string("box"));
    DOMTokenList_add_with_tokens(HTMLElement_get_classList(box), classes);
    HTMLElement_appendChild(body, HTMLElement_as_Node(box));
    CHECK(Document_getElementById(doc, "made-by-c").h == box.h);
    CHECK(DOMTokenList_contains(HTMLElement_get_classList(box), "box"));

    // Read an element that was already in the page, and copy its text.
    HTMLElement seed = Document_getElementById(doc, "seed");
    CHECK(str_eq(HTMLElement_get_textContent(seed), "seed text"));
    CHECK(str_eq(js_to_string(HTMLElement_getAttribute(seed, "data-x")), "42"));
    CHECK(HTMLElement_getAttribute(seed, "data-missing").h == 0);
    char text[64];
    JsString seed_text = HTMLElement_get_textContent(seed);
    JsString_read(seed_text, text);
    text[JsString_len(seed_text)] = 0;
    HTMLElement copy = Document_createElement_with_string(doc, "p");
    HTMLElement_set_textContent(copy, text);
    HTMLElement_appendChild(box, HTMLElement_as_Node(copy));

    // A listener called by real clicks, then removed.
    HTMLElement button = Document_createElement_with_string(doc, "button");
    HTMLElement_set_textContent(button, "press");
    HTMLElement_appendChild(box, HTMLElement_as_Node(button));
    clicks = (Counter){ box, "data-count", 0 };
    HTMLElement_addEventListener_listener listener = HTMLElement_addEventListener_listener_from(on_click, &clicks);
    HTMLElement_addEventListener(button, HTMLAnchorElement_addEventListener_type_click, listener);
    HTMLElement_click(button);
    HTMLElement_click(button);
    CHECK(clicks.count == 2);
    HTMLElement_removeEventListener(button, HTMLAnchorElement_addEventListener_type_click, listener);
    HTMLElement_click(button);
    CHECK(clicks.count == 2);

    // An options object, { once: true }, and events made and dispatched here.
    pings = (Counter){ box, "data-pings", 0 };
    AddEventListenerOptions once = AddEventListenerOptions_create();
    AddEventListenerOptions_set_once(once, js_bool(true));
    JsValue ping_listener = { EventListener_from(on_ping, &pings).h };
    HTMLElement_addEventListener_with_string_and_EventListenerOrEventListenerObject_and_options_string_EventListenerOrEventListenerObject_AddEventListenerOptions(box, "ping", ping_listener, once);
    HTMLElement_dispatchEvent(box, Event_new("ping"));
    HTMLElement_dispatchEvent(box, Event_new("ping"));
    CHECK(pings.count == 1);

    // Style, layout and the window.
    CSSStyleDeclaration_setProperty(HTMLElement_get_style(box), "color", js_string("red"));
    CHECK(str_eq(CSSStyleDeclaration_get_color(HTMLElement_get_style(box)), "red"));
    CHECK(DOMRect_get_width(HTMLElement_getBoundingClientRect(box)) > 0);
    CHECK(HTMLCollection_get_length(HTMLElement_get_children(box)) == 2);
    CHECK(get_innerWidth() > 0);
    Document_set_title(doc, "driven by c");
}
