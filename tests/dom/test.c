// Drives a real page through the generated DOM bindings; every CHECK must hold.
// Only the declarations used: one header each. Inherited members are called
// by the type that declares them, with any subtype: Element_setAttribute(div, ...).
// C11 _Generic does the upcast, and rejects what is not a subtype.
#include "areas/core/AddEventListenerOptions.h"
#include "areas/core/DOMRect.h"
#include "areas/core/DOMTokenList.h"
#include "areas/core/Element.h"
#include "areas/core/ElementCSSInlineStyle.h"
#include "areas/core/EventTarget.h"
#include "areas/core/Node.h"
#include "areas/core/ParentNode.h"
#include "areas/core/Document.h"
#include "areas/core/functions.h"
#include "areas/css/CSSStyleDeclaration.h"
#include "areas/css/CSSStyleDeclarationBase.h"
#include "areas/css/CSSStyleProperties.h"
#include "areas/html/HTMLCollectionBase.h"
#include "areas/events/Event.h"
#include "areas/html/HTMLButtonElement.h"
#include "areas/html/HTMLCollection.h"
#include "areas/html/HTMLDivElement.h"
#include "areas/html/HTMLElement.h"
#include "areas/html/HTMLParagraphElement.h"

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
    Element_setAttribute(counter->box, counter->attribute, digits);
}

static JsValue on_click(void* ctx, PointerEvent ev) {
    bump(ctx);
    return (JsValue){ 0 };
}

static void on_ping(void* ctx, Event ev) { bump(ctx); }

static Counter clicks, pings;

__attribute__((export_name("run"))) void run(void) {
    Document doc = get_document();
    HTMLElement body = Document_get_body(doc);

    // Create an element and put it in the page.
    // One method per tag, returning the tag's own element type.
    HTMLDivElement div = Document_createElement_div(doc);
    HTMLElement box = HTMLDivElement_as_HTMLElement(div);
    Element_set_id(box, "made-by-c");
    Node_set_textContent(box, "hello from c");
    JsArray_JsString classes = JsArray_JsString_new();
    JsArray_JsString_push(classes, "box");
    DOMTokenList_add_with_tokens(Element_get_classList(box), classes);
    js_release(classes);
    Node_appendChild(body, box);
    CHECK(Document_getElementById(doc, "made-by-c").h == box.h);
    CHECK(DOMTokenList_contains(Element_get_classList(box), "box"));

    // Read an element that was already in the page, and copy its text.
    HTMLElement seed = Document_getElementById(doc, "seed");
    CHECK(str_eq(Node_get_textContent(seed), "seed text"));
    CHECK(str_eq((Element_getAttribute(seed, "data-x")), "42"));
    CHECK(Element_getAttribute(seed, "data-missing").h == 0);
    char text[64];
    JsString seed_text = Node_get_textContent(seed);
    JsString_read(seed_text, text);
    text[JsString_len(seed_text)] = 0;
    HTMLElement copy = HTMLParagraphElement_as_HTMLElement(Document_createElement_p(doc));
    Node_set_textContent(copy, text);
    Node_appendChild(box, copy);

    // A listener called by real clicks, then removed.
    HTMLElement button = HTMLButtonElement_as_HTMLElement(Document_createElement_button(doc));
    Node_set_textContent(button, "press");
    Node_appendChild(box, button);
    clicks = (Counter){ box, "data-count", 0 };
    // One method per event, with the event's own type.
    HTMLElement_PointerEvent_fn listener = HTMLElement_PointerEvent_fn_from(on_click, &clicks);
    HTMLElement_addEventListener_click(button, listener);
    HTMLElement_click(button);
    HTMLElement_click(button);
    CHECK(clicks.count == 2);
    HTMLElement_removeEventListener_click(button, listener);
    HTMLElement_click(button);
    CHECK(clicks.count == 2);

    // An options object, { once: true }, and events made and dispatched here.
    pings = (Counter){ box, "data-pings", 0 };
    AddEventListenerOptions once = AddEventListenerOptions_create();
    AddEventListenerOptions_set_once(once, js_some_bool(true));
    JsValue ping_listener = { EventListener_from(on_ping, &pings).h };
    HTMLElement_addEventListener_with_AddEventListenerOptions(box, "ping", ping_listener, once);
    EventTarget_dispatchEvent(box, Event_new("ping"));
    EventTarget_dispatchEvent(box, Event_new("ping"));
    CHECK(pings.count == 1);

    // Style, layout and the window.
    CSSStyleDeclaration style = ElementCSSInlineStyle_get_style(box);
    CSSStyleDeclarationBase_setProperty(style, "color", "red");
    CHECK(str_eq(CSSStyleProperties_get_color(style), "red"));
    CHECK(DOMRect_get_width(Element_getBoundingClientRect(box)) > 0);
    CHECK(HTMLCollectionBase_get_length(ParentNode_get_children(box)) == 2);

    // Collections iterate.
    HTMLCollection kids = ParentNode_get_children(box);
    JsIterator it = HTMLCollectionBase_iterate(kids);
    Element child;
    int children = 0;
    while (HTMLCollectionBase_iterate_next(it, &child)) {
        children++;
        js_release(child);
    }
    js_release(it);
    js_release(kids);
    CHECK(children == 2);
    CHECK(get_innerWidth() > 0);
    Document_set_title(doc, "driven by c");

    // An invalid selector throws: the call returns a null handle, and the exception is kept.
    CHECK(ParentNode_querySelector(box, "<bad").h == 0);
    JsValue bad = js_take_error();
    CHECK(bad.h != 0);
    js_release(bad);

    // A handle a function returns is the caller's to release: one by one, or
    // all that came back while a scope was open.
    uint32_t live = js_live_handles();
    for (int i = 0; i < 500; i++) {
        js_scope_open();
        HTMLElement found = Document_getElementById(doc, "made-by-c");
        CHECK(DOMTokenList_contains(Element_get_classList(found), "box") && str_eq(Element_get_id(found), "made-by-c"));
        js_scope_close();
    }
    CHECK(js_live_handles() == live);
}
