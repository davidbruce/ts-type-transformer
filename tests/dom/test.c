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
    HTMLDivElement div = Document_createElement_div(doc, (ElementCreationOptions){ 0 });
    HTMLElement box = HTMLDivElement_as_HTMLElement(div);
    Element_set_id(box, "made-by-c");
    Node_set_textContent(box, "hello from c");
    JsArray_JsString classes = JsArray_JsString_new();
    JsArray_JsString_push(classes, "box");
    DOMTokenList_add(Element_get_classList(box), classes);
    js_release(classes);
    Node_appendChild(body, box);
    CHECK(Document_getElementById(doc, "made-by-c").h == box.h);
    CHECK(DOMTokenList_contains(Element_get_classList(box), "box"));

    // Read an element that was already in the page, and copy its text.
    HTMLElement seed = Document_getElementById(doc, "seed");
    CHECK(JsString_equals(Node_get_textContent(seed), "seed text"));
    CHECK(JsString_equals((Element_getAttribute(seed, "data-x")), "42"));
    CHECK(Element_getAttribute(seed, "data-missing").h == 0);
    char text[64];
    JsString seed_text = Node_get_textContent(seed);
    CHECK(JsString_copy(seed_text, text, sizeof text) == 9);
    // What does not fit is cut, and the copy still ends in NUL.
    char short_text[5];
    CHECK(JsString_copy(seed_text, short_text, sizeof short_text) == 9 && short_text[3] == 'd' && short_text[4] == 0);
    HTMLElement copy = HTMLParagraphElement_as_HTMLElement(Document_createElement_p(doc, (ElementCreationOptions){ 0 }));
    Node_set_textContent(copy, text);
    Node_appendChild(box, copy);

    // A listener called by real clicks, then removed.
    HTMLElement button = HTMLButtonElement_as_HTMLElement(Document_createElement_button(doc, (ElementCreationOptions){ 0 }));
    Node_set_textContent(button, "press");
    Node_appendChild(box, button);
    clicks = (Counter){ box, "data-count", 0 };
    // One method per event, with the event's own type.
    HTMLElement_PointerEvent_fn listener = HTMLElement_PointerEvent_fn_from(on_click, &clicks);
    HTMLElement_addEventListener_click(button, listener, js_none_bool());
    HTMLElement_click(button);
    HTMLElement_click(button);
    CHECK(clicks.count == 2);
    HTMLElement_removeEventListener_click(button, listener, js_none_bool());
    HTMLElement_click(button);
    CHECK(clicks.count == 2);

    // An options object, { once: true }, and events made and dispatched here.
    pings = (Counter){ box, "data-pings", 0 };
    AddEventListenerOptions once = AddEventListenerOptions_create();
    AddEventListenerOptions_set_once(once, js_some_bool(true));
    JsValue ping_listener = { EventListener_from(on_ping, &pings).h };
    Element_addEventListener_with_AddEventListenerOptions(box, "ping", ping_listener, once);
    EventTarget_dispatchEvent(box, Event_new("ping", (EventInit){ 0 }));
    EventTarget_dispatchEvent(box, Event_new("ping", (EventInit){ 0 }));
    CHECK(pings.count == 1);

    // Style, layout and the window.
    CSSStyleDeclaration style = ElementCSSInlineStyle_get_style(box);
    CSSStyleDeclarationBase_setProperty(style, "color", "red", NULL);
    CHECK(JsString_equals(CSSStyleProperties_get_color(style), "red"));
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
        CHECK(DOMTokenList_contains(Element_get_classList(found), "box") && JsString_equals(Element_get_id(found), "made-by-c"));
        js_scope_close();
    }
    CHECK(js_live_handles() == live);
}
