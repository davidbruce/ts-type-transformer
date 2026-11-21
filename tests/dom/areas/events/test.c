// Events from C: listeners and their options, dispatch, bubbling, cancelling,
// custom events and events built from init dictionaries. Every CHECK must hold.
#include "bindings.h"

TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)
#define STR(s, text) JsString_equals((s), (text))

// One listener's state: its tag goes into the shared order log when it runs.
typedef struct {
    char tag;
    int count;
    int phase;
    bool prevent;
    bool stop;
    bool stop_immediate;
    bool current_is_target;
    double detail;
    double x;
    bool ctrl;
    bool shift;
    char key[16];
} Rec;

static char order[32];
static int order_len;
static void order_reset(void) { order_len = 0; order[0] = 0; }
static bool order_is(const char* text) {
    int i = 0;
    while (text[i] && i < order_len && order[i] == text[i]) i++;
    return text[i] == 0 && i == order_len;
}

static void on_event(void* ctx, Event ev) {
    Rec* r = ctx;
    r->count++;
    if (order_len < 31) { order[order_len++] = r->tag; order[order_len] = 0; }
    r->phase = (int)Event_get_eventPhase(ev);
    r->current_is_target = js_same(Event_get_currentTarget(ev), Event_get_target(ev));
    if (r->prevent) Event_preventDefault(ev);
    if (r->stop) Event_stopPropagation(ev);
    if (r->stop_immediate) Event_stopImmediatePropagation(ev);
}

static void on_custom(void* ctx, Event ev) {
    Rec* r = ctx;
    r->count++;
    CustomEvent ce = CustomEvent_cast(ev);
    CHECK(ce.h != 0 && MouseEvent_cast(ev).h == 0);
    r->detail = js_to_number(CustomEvent_get_detail(ce));
}

static void on_mouse(void* ctx, Event ev) {
    Rec* r = ctx;
    MouseEvent me = MouseEvent_cast(ev);
    CHECK(me.h != 0 && KeyboardEvent_cast(ev).h == 0);
    r->count++;
    r->x = MouseEvent_get_clientX(me) * 1000 + MouseEvent_get_clientY(me) + MouseEvent_get_button(me) * 1000000;
    r->ctrl = MouseEvent_get_ctrlKey(me);
    r->shift = MouseEvent_get_shiftKey(me);
}

static void on_key(void* ctx, Event ev) {
    Rec* r = ctx;
    KeyboardEvent ke = KeyboardEvent_cast(ev);
    CHECK(ke.h != 0 && MouseEvent_cast(ev).h == 0);
    r->count++;
    JsString_copy(KeyboardEvent_get_key(ke), r->key, sizeof r->key);
    r->ctrl = KeyboardEvent_get_ctrlKey(ke);
    r->shift = KeyboardEvent_get_repeat(ke);
    r->x = KeyboardEvent_get_keyCode(ke);
}

static EventListenerOrEventListenerObject listener(EventListener_fn fn, Rec* r) {
    return EventListenerOrEventListenerObject_of_EventListener(EventListener_from(fn, r));
}

static Event make(const char* type, bool bubbles, bool cancelable) {
    EventInit init = EventInit_create();
    EventInit_set_bubbles(init, js_some_bool(bubbles));
    EventInit_set_cancelable(init, js_some_bool(cancelable));
    return Event_new(type, init);
}

static Rec outer_bubble = { .tag = 'B' }, outer_capture = { .tag = 'C' }, inner = { .tag = 'I' }, inner2 = { .tag = 'J' };
static Rec once_rec = { .tag = 'O' }, signal_rec = { .tag = 'S' }, passive_rec = { .tag = 'P' }, removed = { .tag = 'R' };
static Rec custom_rec, mouse_rec, key_rec, doc_rec = { .tag = 'D' };

__attribute__((export_name("run"))) void run(void) {
    Document doc = get_document();
    HTMLElement body = Document_get_body(doc);
    HTMLDivElement outer = Document_createElement_div(doc);
    HTMLButtonElement button = Document_createElement_button(doc);
    Node_appendChild(outer, button);
    Node_appendChild(body, outer);

    // Capture runs first, then the target, then bubbling.
    EventListenerOrEventListenerObject bubble_l = listener(on_event, &outer_bubble);
    EventTarget_addEventListener(outer, "ping", bubble_l);
    AddEventListenerOptions capture = AddEventListenerOptions_create();
    EventListenerOptions_set_capture(capture, js_some_bool(true));
    EventTarget_addEventListener(outer, "ping", listener(on_event, &outer_capture), capture);
    EventTarget_addEventListener(button, "ping", listener(on_event, &inner));
    order_reset();
    CHECK(EventTarget_dispatchEvent(button, make("ping", true, false)));
    CHECK(order_is("CIB"));
    CHECK(outer_capture.phase == Event_CAPTURING_PHASE);
    CHECK(inner.phase == Event_AT_TARGET);
    CHECK(outer_bubble.phase == Event_BUBBLING_PHASE);
    CHECK(!outer_capture.current_is_target && inner.current_is_target && !outer_bubble.current_is_target);

    // An event that does not bubble still has a capture phase.
    order_reset();
    EventTarget_dispatchEvent(button, make("ping", false, false));
    CHECK(order_is("CI"));

    // stopPropagation at the target keeps bubbling away.
    inner.stop = true;
    order_reset();
    EventTarget_dispatchEvent(button, make("ping", true, false));
    CHECK(order_is("CI"));
    inner.stop = false;

    // stopImmediatePropagation also silences the target's later listeners.
    inner.stop_immediate = true;
    EventTarget_addEventListener(button, "ping", listener(on_event, &inner2));
    order_reset();
    EventTarget_dispatchEvent(button, make("ping", true, false));
    CHECK(order_is("CI") && inner2.count == 0);
    inner.stop_immediate = false;
    order_reset();
    EventTarget_dispatchEvent(button, make("ping", true, false));
    CHECK(order_is("CIJB") && inner2.count == 1);

    // preventDefault: only a cancelable event is affected.
    inner.prevent = true;
    Event cancelable = make("ping", true, true);
    CHECK(Event_get_cancelable(cancelable) && !Event_get_defaultPrevented(cancelable));
    CHECK(!EventTarget_dispatchEvent(button, cancelable));
    CHECK(Event_get_defaultPrevented(cancelable));
    Event fixed = make("ping", true, false);
    CHECK(EventTarget_dispatchEvent(button, fixed));
    CHECK(!Event_get_defaultPrevented(fixed));
    inner.prevent = false;

    // Event fields.
    Event probe = make("probe", true, true);
    CHECK(STR(Event_get_type(probe), "probe"));
    CHECK(Event_get_bubbles(probe) && Event_get_cancelable(probe) && !Event_get_composed(probe));
    CHECK(!Event_get_isTrusted(probe));
    CHECK(Event_get_target(probe).h == 0 && Event_get_eventPhase(probe) == Event_NONE);
    CHECK(Event_get_timeStamp(probe) >= 0);
    Event plain = Event_new("plain");
    CHECK(!Event_get_bubbles(plain) && !Event_get_cancelable(plain));

    // once: the listener runs one time.
    AddEventListenerOptions once = AddEventListenerOptions_create();
    AddEventListenerOptions_set_once(once, js_some_bool(true));
    EventTarget_addEventListener(button, "once", listener(on_event, &once_rec), once);
    EventTarget_dispatchEvent(button, Event_new("once"));
    EventTarget_dispatchEvent(button, Event_new("once"));
    CHECK(once_rec.count == 1);

    // passive: preventDefault is ignored.
    AddEventListenerOptions passive = AddEventListenerOptions_create();
    AddEventListenerOptions_set_passive(passive, js_some_bool(true));
    passive_rec.prevent = true;
    EventTarget_addEventListener(button, "soft", listener(on_event, &passive_rec), passive);
    Event soft = make("soft", false, true);
    CHECK(EventTarget_dispatchEvent(button, soft));
    CHECK(passive_rec.count == 1 && !Event_get_defaultPrevented(soft));

    // removeEventListener needs the very listener that was added.
    EventListenerOrEventListenerObject removable = listener(on_event, &removed);
    EventTarget_addEventListener(button, "gone", removable);
    EventTarget_dispatchEvent(button, Event_new("gone"));
    CHECK(removed.count == 1);
    EventTarget_removeEventListener(button, "gone", removable);
    EventTarget_dispatchEvent(button, Event_new("gone"));
    CHECK(removed.count == 1);
    // A capture listener is removed only with capture set.
    EventListenerOrEventListenerObject cap_l = listener(on_event, &removed);
    EventTarget_addEventListener(button, "cap", cap_l, capture);
    EventListenerOptions plain_opts = EventListenerOptions_create();
    EventTarget_removeEventListener(button, "cap", cap_l, plain_opts);
    EventTarget_dispatchEvent(button, Event_new("cap"));
    CHECK(removed.count == 2);
    EventListenerOptions cap_opts = EventListenerOptions_create();
    EventListenerOptions_set_capture(cap_opts, js_some_bool(true));
    EventTarget_removeEventListener(button, "cap", cap_l, cap_opts);
    EventTarget_dispatchEvent(button, Event_new("cap"));
    CHECK(removed.count == 2);

    // An AbortSignal removes the listener when it aborts.
    AbortController controller = AbortController_new();
    AbortSignal signal = AbortController_get_signal(controller);
    CHECK(!AbortSignal_get_aborted(signal));
    AddEventListenerOptions with_signal = AddEventListenerOptions_create();
    AddEventListenerOptions_set_signal(with_signal, signal);
    EventTarget_addEventListener(button, "sig", listener(on_event, &signal_rec), with_signal);
    EventTarget_dispatchEvent(button, Event_new("sig"));
    CHECK(signal_rec.count == 1);
    AbortController_abort(controller);
    CHECK(AbortSignal_get_aborted(signal));
    EventTarget_dispatchEvent(button, Event_new("sig"));
    CHECK(signal_rec.count == 1);
    CHECK(AbortSignal_get_reason(signal).h != 0);
    // throwIfAborted throws the reason: an AbortError.
    AbortSignal_throwIfAborted(signal);
    JsValue thrown = js_take_error();
    CHECK(thrown.h != 0 && STR(js_error_name(thrown), "AbortError"));

    // CustomEvent carries a detail.
    CustomEventInit detail_init = CustomEventInit_create();
    CustomEventInit_set_detail(detail_init, js_number(42));
    EventInit_set_bubbles(detail_init, js_some_bool(true));
    EventTarget_addEventListener(outer, "custom", EventListenerOrEventListenerObject_of_EventListener(EventListener_from(on_custom, &custom_rec)));
    CustomEvent custom = CustomEvent_new("custom", detail_init);
    CHECK(js_to_number(CustomEvent_get_detail(custom)) == 42);
    EventTarget_dispatchEvent(button, custom);
    CHECK(custom_rec.count == 1 && custom_rec.detail == 42);
    // A custom event without a detail has a null one.
    CHECK(CustomEvent_get_detail(CustomEvent_new("custom")).h == 0);

    // MouseEvent from an init dictionary.
    MouseEventInit mouse_init = MouseEventInit_create();
    MouseEventInit_set_clientX(mouse_init, js_some_double(12));
    MouseEventInit_set_clientY(mouse_init, js_some_double(34));
    MouseEventInit_set_button(mouse_init, js_some_double(2));
    EventModifierInit_set_ctrlKey(mouse_init, js_some_bool(true));
    EventInit_set_bubbles(mouse_init, js_some_bool(true));
    EventTarget_addEventListener(button, "mousedown", listener(on_mouse, &mouse_rec));
    MouseEvent mouse = MouseEvent_new("mousedown", mouse_init);
    CHECK(MouseEvent_get_clientX(mouse) == 12 && MouseEvent_get_clientY(mouse) == 34);
    CHECK(MouseEvent_get_button(mouse) == 2 && MouseEvent_get_ctrlKey(mouse) && !MouseEvent_get_shiftKey(mouse));
    EventTarget_dispatchEvent(button, mouse);
    CHECK(mouse_rec.count == 1 && mouse_rec.x == 2000000 + 12 * 1000 + 34 && mouse_rec.ctrl && !mouse_rec.shift);

    // KeyboardEvent from an init dictionary, heard at the document.
    KeyboardEventInit key_init = KeyboardEventInit_create();
    KeyboardEventInit_set_key(key_init, "Enter");
    KeyboardEventInit_set_code(key_init, "NumpadEnter");
    KeyboardEventInit_set_keyCode(key_init, js_some_double(13));
    KeyboardEventInit_set_repeat(key_init, js_some_bool(true));
    EventModifierInit_set_shiftKey(key_init, js_some_bool(true));
    EventInit_set_bubbles(key_init, js_some_bool(true));
    EventTarget_addEventListener(doc, "keydown", listener(on_key, &key_rec));
    KeyboardEvent key = KeyboardEvent_new("keydown", key_init);
    CHECK(STR(KeyboardEvent_get_code(key), "NumpadEnter") && KeyboardEvent_get_shiftKey(key));
    EventTarget_dispatchEvent(button, key);
    CHECK(key_rec.count == 1 && key_rec.key[0] == 'E' && key_rec.key[4] == 'r' && key_rec.key[5] == 0);
    CHECK(key_rec.x == 13 && key_rec.shift /* repeat */ && !key_rec.ctrl);

    // A real click through click() reaches the listeners, bubbling to the document.
    EventTarget_addEventListener(doc, "click", listener(on_event, &doc_rec));
    order_reset();
    HTMLElement_click(button);
    CHECK(doc_rec.count == 1 && doc_rec.phase == Event_BUBBLING_PHASE);

    // Listeners released: forget them and the handle count does not grow.
    uint32_t live = js_live_handles();
    for (int i = 0; i < 100; i++) {
        js_scope_open();
        EventListener l = EventListener_from(on_event, &removed);
        EventTarget_addEventListener(button, "loop", EventListenerOrEventListenerObject_of_EventListener(l));
        EventTarget_removeEventListener(button, "loop", EventListenerOrEventListenerObject_of_EventListener(l));
        js_forget_callback(l);
        js_scope_close();
    }
    CHECK(js_live_handles() == live);
}
