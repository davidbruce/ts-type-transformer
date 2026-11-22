// Observers from C: MutationObserver on children, attributes and text,
// ResizeObserver and IntersectionObserver reporting a layout, and a timeout
// signal. Callbacks run later: busy() waits and finish() checks what they left.
// Every CHECK must hold.
#include "bindings.h"

TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)
#define STR(s, text) JsString_equals((s), (text))

static int pending;
__attribute__((export_name("busy"))) int busy(void) { return pending; }

static MutationObserverInit init(bool child, bool attrs, bool text, bool subtree) {
    MutationObserverInit o = MutationObserverInit_create();
    if (child) MutationObserverInit_set_childList(o, js_some_bool(true));
    if (attrs) MutationObserverInit_set_attributes(o, js_some_bool(true));
    if (text) MutationObserverInit_set_characterData(o, js_some_bool(true));
    if (subtree) MutationObserverInit_set_subtree(o, js_some_bool(true));
    return o;
}

// An observer whose callback is never wanted: records are taken by hand.
static void never(void* ctx, JsArray_MutationRecord records, MutationObserver observer) { (void)ctx; (void)records; (void)observer; }

// A callback that counts what it is given, and sees its own observer.
static int cb_calls, cb_records, cb_added;
static MutationObserver cb_observer;
static bool cb_saw_observer;
static void on_mutations(void* ctx, JsArray_MutationRecord records, MutationObserver observer) {
    (void)ctx;
    cb_calls++;
    cb_records += (int)JsArray_MutationRecord_length(records);
    cb_saw_observer = js_same(observer, cb_observer);
    for (uint32_t i = 0; i < JsArray_MutationRecord_length(records); i++) {
        MutationRecord r = JsArray_MutationRecord_get(records, i);
        cb_added += (int)NodeList_get_length(MutationRecord_get_addedNodes(r));
    }
    pending--;
}

// ResizeObserver.
static int resizes;
static double first_width, first_height, first_inline, second_width;
static HTMLElement resize_target;
static void on_resize(void* ctx, JsArray_ResizeObserverEntry entries, ResizeObserver observer) {
    (void)ctx;
    ResizeObserverEntry entry = JsArray_ResizeObserverEntry_get(entries, JsArray_ResizeObserverEntry_length(entries) - 1);
    resizes++;
    if (resizes == 1) {
        first_width = DOMRectReadOnly_get_width(ResizeObserverEntry_get_contentRect(entry));
        first_height = DOMRectReadOnly_get_height(ResizeObserverEntry_get_contentRect(entry));
        first_inline = ResizeObserverSize_get_inlineSize(JsArray_ResizeObserverSize_get(ResizeObserverEntry_get_contentBoxSize(entry), 0));
        CHECK(js_same(ResizeObserverEntry_get_target(entry), resize_target));
        CSSStyleProperties_set_width(ElementCSSInlineStyle_get_style(resize_target), "150px");
    } else if (resizes == 2) {
        second_width = DOMRectReadOnly_get_width(ResizeObserverEntry_get_contentRect(entry));
        ResizeObserver_disconnect(observer);
        pending--;
    }
}

// IntersectionObserver: one target in view, one far below it.
static HTMLElement in_view, below;
static int io_calls, in_view_seen, below_seen;
static bool in_view_hit, below_hit_first, below_hit_later;
static double in_view_ratio, below_ratio_later;
static void on_intersect(void* ctx, JsArray_IntersectionObserverEntry entries, IntersectionObserver observer) {
    (void)ctx;
    (void)observer;
    io_calls++;
    for (uint32_t i = 0; i < JsArray_IntersectionObserverEntry_length(entries); i++) {
        IntersectionObserverEntry e = JsArray_IntersectionObserverEntry_get(entries, i);
        Element target = IntersectionObserverEntry_get_target(e);
        bool hit = IntersectionObserverEntry_get_isIntersecting(e);
        if (js_same(target, in_view)) {
            in_view_seen++;
            // The first report; scrolling away later reports it leaving.
            if (in_view_seen == 1) {
                in_view_hit = hit;
                in_view_ratio = IntersectionObserverEntry_get_intersectionRatio(e);
            }
        } else if (js_same(target, below)) {
            below_seen++;
            if (below_seen == 1) {
                below_hit_first = hit;
                Element_scrollIntoView(below);
            } else if (hit) {
                below_hit_later = true;
                below_ratio_later = IntersectionObserverEntry_get_intersectionRatio(e);
            }
        }
    }
    if (in_view_seen >= 1 && below_hit_later) {
        IntersectionObserver_disconnect(observer);
        pending--;
    }
}

// AbortSignal.timeout.
static int abort_events;
static void on_abort(void* ctx, Event ev) { (void)ctx; (void)ev; abort_events++; pending--; }
static AbortSignal timed;

__attribute__((export_name("run"))) void run(void) {
    Document doc = get_document();
    HTMLElement body = Document_get_body(doc);
    HTMLDivElement box = Document_createElement_div(doc);
    Node_appendChild(body, box);

    // childList: appended and removed children are recorded, in order.
    MutationObserver kids = MutationObserver_new(MutationCallback_from(never, 0));
    MutationObserver_observe(kids, box, init(true, false, false, false));
    HTMLElement a = HTMLParagraphElement_as_HTMLElement(Document_createElement_p(doc));
    Node_appendChild(box, a);
    Node_removeChild(box, a);
    JsArray_MutationRecord taken = MutationObserver_takeRecords(kids);
    CHECK(JsArray_MutationRecord_length(taken) == 2);
    MutationRecord added = JsArray_MutationRecord_get(taken, 0), removed = JsArray_MutationRecord_get(taken, 1);
    CHECK(MutationRecord_get_type(added) == MutationRecordType_childList && MutationRecord_get_type(removed) == MutationRecordType_childList);
    CHECK(js_same(MutationRecord_get_target(added), box));
    CHECK(NodeList_get_length(MutationRecord_get_addedNodes(added)) == 1 && NodeList_get_length(MutationRecord_get_removedNodes(added)) == 0);
    CHECK(js_same(NodeList_item(MutationRecord_get_addedNodes(added), 0), a));
    CHECK(NodeList_get_length(MutationRecord_get_removedNodes(removed)) == 1 && js_same(NodeList_item(MutationRecord_get_removedNodes(removed), 0), a));
    // The queue is emptied by takeRecords.
    CHECK(JsArray_MutationRecord_length(MutationObserver_takeRecords(kids)) == 0);
    // Attribute changes are not childList records; grandchildren need subtree.
    Element_setAttribute(box, "data-x", "1");
    HTMLElement inner = HTMLParagraphElement_as_HTMLElement(Document_createElement_p(doc));
    Node_appendChild(box, inner);
    Node_appendChild(inner, Document_createElement_span(doc));
    CHECK(JsArray_MutationRecord_length(MutationObserver_takeRecords(kids)) == 1);
    MutationObserver_disconnect(kids);
    Node_removeChild(box, inner);
    CHECK(JsArray_MutationRecord_length(MutationObserver_takeRecords(kids)) == 0);

    // subtree: changes anywhere below are seen.
    MutationObserver deep = MutationObserver_new(MutationCallback_from(never, 0));
    MutationObserver_observe(deep, box, init(true, false, false, true));
    HTMLElement branch = HTMLDivElement_as_HTMLElement(Document_createElement_div(doc));
    Node_appendChild(box, branch);
    Node_appendChild(branch, Document_createElement_span(doc));
    JsArray_MutationRecord deep_records = MutationObserver_takeRecords(deep);
    CHECK(JsArray_MutationRecord_length(deep_records) == 2);
    CHECK(js_same(MutationRecord_get_target(JsArray_MutationRecord_get(deep_records, 1)), branch));
    MutationObserver_disconnect(deep);

    // attributes: with the old value, and an attributeFilter.
    MutationObserver attrs = MutationObserver_new(MutationCallback_from(never, 0));
    MutationObserverInit watch = init(false, true, false, false);
    MutationObserverInit_set_attributeOldValue(watch, js_some_bool(true));
    JsArray_JsString only = JsArray_JsString_new();
    JsArray_JsString_push(only, "data-x");
    MutationObserverInit_set_attributeFilter(watch, only);
    MutationObserver_observe(attrs, box, watch);
    Element_setAttribute(box, "data-x", "2");
    Element_setAttribute(box, "data-y", "ignored");
    Element_removeAttribute(box, "data-x");
    JsArray_MutationRecord attr_records = MutationObserver_takeRecords(attrs);
    CHECK(JsArray_MutationRecord_length(attr_records) == 2);
    MutationRecord changed = JsArray_MutationRecord_get(attr_records, 0);
    CHECK(MutationRecord_get_type(changed) == MutationRecordType_attributes);
    CHECK(STR(MutationRecord_get_attributeName(changed), "data-x") && STR(MutationRecord_get_oldValue(changed), "1"));
    CHECK(STR(MutationRecord_get_oldValue(JsArray_MutationRecord_get(attr_records, 1)), "2"));
    CHECK(MutationRecord_get_attributeNamespace(changed).h == 0);
    MutationObserver_disconnect(attrs);

    // characterData: a text node edited, with the old text.
    Text text = Document_createTextNode(doc, "before");
    Node_appendChild(box, text);
    MutationObserver chars = MutationObserver_new(MutationCallback_from(never, 0));
    MutationObserverInit watch_text = init(false, false, true, false);
    MutationObserverInit_set_characterDataOldValue(watch_text, js_some_bool(true));
    MutationObserver_observe(chars, text, watch_text);
    CharacterData_set_data(text, "after");
    CharacterData_appendData(text, "!");
    JsArray_MutationRecord text_records = MutationObserver_takeRecords(chars);
    CHECK(JsArray_MutationRecord_length(text_records) == 2);
    CHECK(MutationRecord_get_type(JsArray_MutationRecord_get(text_records, 0)) == MutationRecordType_characterData);
    CHECK(STR(MutationRecord_get_oldValue(JsArray_MutationRecord_get(text_records, 0)), "before"));
    CHECK(STR(MutationRecord_get_oldValue(JsArray_MutationRecord_get(text_records, 1)), "after"));
    MutationObserver_disconnect(chars);

    // A callback is called once with the batch, after the code that changed things.
    HTMLDivElement watched = Document_createElement_div(doc);
    Node_appendChild(body, watched);
    cb_observer = MutationObserver_new(MutationCallback_from(on_mutations, 0));
    MutationObserver_observe(cb_observer, watched, init(true, true, false, false));
    pending++;
    for (int i = 0; i < 3; i++) Node_appendChild(watched, Document_createElement_span(doc));
    Element_setAttribute(watched, "title", "changed");
    CHECK(cb_calls == 0);

    // ResizeObserver: reports the size on observing, and again when it changes.
    HTMLDivElement sized = Document_createElement_div(doc);
    resize_target = HTMLDivElement_as_HTMLElement(sized);
    CSSStyleDeclaration style = ElementCSSInlineStyle_get_style(sized);
    CSSStyleProperties_set_width(style, "100px");
    CSSStyleProperties_set_height(style, "50px");
    Node_appendChild(body, sized);
    ResizeObserver resize = ResizeObserver_new(ResizeObserverCallback_from(on_resize, 0));
    ResizeObserver_observe(resize, sized);
    pending++;

    // IntersectionObserver: one target in view, one far below; scrolling to it reports it too.
    HTMLDivElement visible = Document_createElement_div(doc);
    in_view = HTMLDivElement_as_HTMLElement(visible);
    Node_set_textContent(visible, "in view");
    CSSStyleProperties_set_height(ElementCSSInlineStyle_get_style(visible), "40px");
    Node_appendChild(body, visible);
    HTMLDivElement far = Document_createElement_div(doc);
    below = HTMLDivElement_as_HTMLElement(far);
    Node_set_textContent(far, "far below");
    CSSStyleDeclaration far_style = ElementCSSInlineStyle_get_style(far);
    CSSStyleProperties_set_marginTop(far_style, "4000px");
    CSSStyleProperties_set_height(far_style, "40px");
    Node_appendChild(body, far);
    IntersectionObserver intersect = IntersectionObserver_new(IntersectionObserverCallback_from(on_intersect, 0));
    CHECK(IntersectionObserver_get_root(intersect).tag == -1);
    CHECK(STR(IntersectionObserver_get_rootMargin(intersect), "0px 0px 0px 0px"));
    CHECK(JsArray_double_length(IntersectionObserver_get_thresholds(intersect)) == 1);
    // The threshold option is a number or an array of them, given as either.
    IntersectionObserverInit steps = IntersectionObserverInit_create();
    JsArray_double marks = JsArray_double_new();
    JsArray_double_push(marks, 0);
    JsArray_double_push(marks, 0.5);
    JsArray_double_push(marks, 1);
    IntersectionObserverInit_set_threshold(steps, marks);
    CHECK(JsArray_double_length(IntersectionObserver_get_thresholds(IntersectionObserver_new(IntersectionObserverCallback_from(on_intersect, 0), steps))) == 3);
    IntersectionObserverInit half = IntersectionObserverInit_create();
    IntersectionObserverInit_set_threshold(half, 0.5);
    CHECK(JsArray_double_get(IntersectionObserver_get_thresholds(IntersectionObserver_new(IntersectionObserverCallback_from(on_intersect, 0), half)), 0) == 0.5);
    IntersectionObserver_observe(intersect, visible);
    IntersectionObserver_observe(intersect, far);
    pending++;

    // AbortSignal.timeout aborts by itself, with a TimeoutError.
    timed = AbortSignal_timeout(40);
    CHECK(!AbortSignal_get_aborted(timed));
    EventTarget_addEventListener(timed, "abort", EventListenerOrEventListenerObject_of_EventListener(EventListener_from(on_abort, 0)));
    pending++;
    // any() follows whichever signal aborts first.
    AbortController manual = AbortController_new();
    JsArray_AbortSignal signals = JsArray_AbortSignal_new();
    JsArray_AbortSignal_push(signals, AbortController_get_signal(manual));
    JsArray_AbortSignal_push(signals, timed);
    AbortSignal either = AbortSignal_any(signals);
    CHECK(!AbortSignal_get_aborted(either));
    AbortController_abort(manual);
    CHECK(AbortSignal_get_aborted(either) && !AbortSignal_get_aborted(timed));
}

__attribute__((export_name("finish"))) void finish(void) {
    CHECK(pending == 0);
    CHECK(cb_calls == 1 && cb_records == 4 && cb_added == 3 && cb_saw_observer);
    CHECK(resizes == 2);
    CHECK(first_width == 100 && first_height == 50 && first_inline == 100);
    CHECK(second_width == 150);
    CHECK(io_calls >= 2 && in_view_seen >= 1 && in_view_hit && in_view_ratio > 0);
    CHECK(!below_hit_first && below_hit_later && below_ratio_later > 0);
    CHECK(AbortSignal_get_aborted(timed) && abort_events == 1);
    JsValue reason = AbortSignal_get_reason(timed);
    CHECK(reason.h != 0 && STR(js_error_name(reason), "TimeoutError"));
}
