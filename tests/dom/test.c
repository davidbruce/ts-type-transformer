// Drives a real page through the generated DOM bindings; every CHECK must hold.
// Only the declarations used: one header each. Inherited members are called
// on the type that declares them, after a cast: Element_setAttribute(HTMLElement_as_Element(el), ...).
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
    Element_setAttribute(HTMLElement_as_Element(counter->box), counter->attribute, digits);
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
    Element_set_id(HTMLElement_as_Element(box), "made-by-c");
    Node_set_textContent(HTMLElement_as_Node(box), "hello from c");
    JsArray classes = JsArray_new();
    JsArray_push(classes, js_string("box"));
    DOMTokenList_add_with_tokens(Element_get_classList(HTMLElement_as_Element(box)), classes);
    Node_appendChild(HTMLElement_as_Node(body), HTMLElement_as_Node(box));
    CHECK(Document_getElementById(doc, "made-by-c").h == box.h);
    CHECK(DOMTokenList_contains(Element_get_classList(HTMLElement_as_Element(box)), "box"));

    // Read an element that was already in the page, and copy its text.
    HTMLElement seed = Document_getElementById(doc, "seed");
    CHECK(str_eq(Node_get_textContent(HTMLElement_as_Node(seed)), "seed text"));
    CHECK(str_eq((Element_getAttribute(HTMLElement_as_Element(seed), "data-x")), "42"));
    CHECK(Element_getAttribute(HTMLElement_as_Element(seed), "data-missing").h == 0);
    char text[64];
    JsString seed_text = Node_get_textContent(HTMLElement_as_Node(seed));
    JsString_read(seed_text, text);
    text[JsString_len(seed_text)] = 0;
    HTMLElement copy = HTMLParagraphElement_as_HTMLElement(Document_createElement_p(doc));
    Node_set_textContent(HTMLElement_as_Node(copy), text);
    Node_appendChild(HTMLElement_as_Node(box), HTMLElement_as_Node(copy));

    // A listener called by real clicks, then removed.
    HTMLElement button = HTMLButtonElement_as_HTMLElement(Document_createElement_button(doc));
    Node_set_textContent(HTMLElement_as_Node(button), "press");
    Node_appendChild(HTMLElement_as_Node(box), HTMLElement_as_Node(button));
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
    AddEventListenerOptions_set_once(once, js_bool(true));
    JsValue ping_listener = { EventListener_from(on_ping, &pings).h };
    HTMLElement_addEventListener_with_AddEventListenerOptions(box, "ping", ping_listener, once);
    EventTarget_dispatchEvent(HTMLElement_as_EventTarget(box), Event_new("ping"));
    EventTarget_dispatchEvent(HTMLElement_as_EventTarget(box), Event_new("ping"));
    CHECK(pings.count == 1);

    // Style, layout and the window.
    CSSStyleDeclaration style = ElementCSSInlineStyle_get_style(HTMLElement_as_ElementCSSInlineStyle(box));
    CSSStyleDeclarationBase_setProperty(CSSStyleDeclaration_as_CSSStyleDeclarationBase(style), "color", "red");
    CHECK(str_eq(CSSStyleProperties_get_color(CSSStyleDeclaration_as_CSSStyleProperties(style)), "red"));
    CHECK(DOMRect_get_width(Element_getBoundingClientRect(HTMLElement_as_Element(box))) > 0);
    CHECK(HTMLCollectionBase_get_length(HTMLCollection_as_HTMLCollectionBase(ParentNode_get_children(HTMLElement_as_ParentNode(box)))) == 2);
    CHECK(get_innerWidth() > 0);
    Document_set_title(doc, "driven by c");
}
