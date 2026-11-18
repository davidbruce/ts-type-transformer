// Events from C++: listeners added and removed, options, bubbling, custom
// events and constructed mouse and keyboard events. Every CHECK must hold.
#include <string>

#include "areas/core/AbortController.hpp"
#include "areas/core/Document.hpp"
#include "areas/core/functions.hpp"

extern "C" TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

extern "C" __attribute__((export_name("run"))) void run() {
    Document doc = get_document();
    HTMLElement body = doc.body();
    HTMLDivElement outer = doc.createElement(HTMLElementTagNameMapKeys::div);
    HTMLDivElement inner = doc.createElement(HTMLElementTagNameMapKeys::div);
    HTMLButtonElement button = doc.createElement(HTMLElementTagNameMapKeys::button);
    inner.appendChild(button);
    outer.appendChild(inner);
    body.appendChild(outer);

    // Add, fire and remove a listener.
    int clicks = 0;
    ts::Listener<PointerEvent> onClick = [&](PointerEvent e) {
        clicks++;
        CHECK(e.type() == "click");
    };
    button.addEventListener(HTMLElementEventMapKeys::click, onClick);
    button.click();
    button.click();
    CHECK(clicks == 2);
    button.removeEventListener(HTMLElementEventMapKeys::click, onClick);
    button.click();
    CHECK(clicks == 2);
    onClick.release();

    // Events bubble from the target up; the target and currentTarget differ.
    std::string order;
    ts::Scoped<EventListener> atButton = [&](Event e) {
        order += "button ";
        CHECK(e.target() == button && e.currentTarget() == button);
        CHECK(e.eventPhase() == Event::AT_TARGET);
    };
    ts::Scoped<EventListener> atInner = [&](Event e) {
        order += "inner ";
        CHECK(e.target() == button && e.currentTarget() == inner);
        CHECK(e.eventPhase() == Event::BUBBLING_PHASE);
    };
    ts::Scoped<EventListener> atOuter = [&](Event) { order += "outer "; };
    button.addEventListener("ping", atButton);
    inner.addEventListener("ping", atInner);
    outer.addEventListener("ping", atOuter);
    EventInit bubbling = EventInit::create();
    bubbling.bubbles(true);
    CHECK(button.dispatchEvent(Event::create("ping", bubbling)));
    CHECK(order == "button inner outer ");

    // A plain Event does not bubble.
    order.clear();
    button.dispatchEvent(Event::create("ping"));
    CHECK(order == "button ");

    // Capturing listeners run first, on the way down.
    order.clear();
    ts::Scoped<EventListener> capturing = [&](Event e) {
        order += "capture ";
        CHECK(e.eventPhase() == Event::CAPTURING_PHASE);
    };
    outer.addEventListener("ping", capturing, true);
    button.dispatchEvent(Event::create("ping", bubbling));
    CHECK(order == "capture button inner outer ");
    outer.removeEventListener("ping", capturing, true);

    // stopPropagation keeps the event from the ancestors.
    order.clear();
    ts::Scoped<EventListener> stopper = [&](Event e) {
        order += "stop ";
        e.stopPropagation();
    };
    inner.addEventListener("stop", stopper);
    ts::Scoped<EventListener> after = [&](Event) { order += "outer-stop "; };
    outer.addEventListener("stop", after);
    button.dispatchEvent(Event::create("stop", bubbling));
    CHECK(order == "stop ");

    // stopImmediatePropagation keeps the same target's other listeners too.
    int first = 0, second = 0;
    ts::Scoped<EventListener> one = [&](Event e) {
        first++;
        e.stopImmediatePropagation();
    };
    ts::Scoped<EventListener> two = [&](Event) { second++; };
    button.addEventListener("halt", one);
    button.addEventListener("halt", two);
    button.dispatchEvent(Event::create("halt"));
    CHECK(first == 1 && second == 0);

    // once: the listener removes itself after one call.
    int onces = 0;
    AddEventListenerOptions once = AddEventListenerOptions::create();
    once.once(true);
    ts::Scoped<EventListener> onOnce = [&](Event) { onces++; };
    button.addEventListener("once", onOnce, once);
    button.dispatchEvent(Event::create("once"));
    button.dispatchEvent(Event::create("once"));
    CHECK(onces == 1);

    // preventDefault: only on a cancelable event.
    EventInit cancelable = EventInit::create();
    cancelable.cancelable(true);
    ts::Scoped<EventListener> prevent = [&](Event e) { e.preventDefault(); };
    button.addEventListener("veto", prevent);
    Event plain = Event::create("veto");
    CHECK(button.dispatchEvent(plain) && !plain.defaultPrevented());
    Event vetoable = Event::create("veto", cancelable);
    CHECK(vetoable.cancelable());
    CHECK(!button.dispatchEvent(vetoable));
    CHECK(vetoable.defaultPrevented());

    // passive: preventDefault is ignored.
    AddEventListenerOptions passive = AddEventListenerOptions::create();
    passive.passive(true);
    ts::Scoped<EventListener> passivePrevent = [&](Event e) { e.preventDefault(); };
    inner.addEventListener("soft", passivePrevent, passive);
    Event soft = Event::create("soft", cancelable);
    inner.dispatchEvent(soft);
    CHECK(!soft.defaultPrevented());

    // An AbortController's signal removes a listener.
    AbortController controller = AbortController::create();
    AddEventListenerOptions withSignal = AddEventListenerOptions::create();
    withSignal.signal(controller.signal());
    int aborted = 0;
    ts::Scoped<EventListener> onSignal = [&](Event) { aborted++; };
    button.addEventListener("signalled", onSignal, withSignal);
    button.dispatchEvent(Event::create("signalled"));
    CHECK(aborted == 1 && !controller.signal().aborted());
    controller.abort();
    CHECK(controller.signal().aborted());
    button.dispatchEvent(Event::create("signalled"));
    CHECK(aborted == 1);

    // A CustomEvent carries a detail.
    CustomEventInit<ts::Value> init = CustomEventInit<ts::Value>::create();
    init.detail(ts::Value::number(42));
    init.bubbles(true);
    double got = 0;
    ts::Scoped<EventListener> onCustom = [&](Event e) {
        auto ce = CustomEvent<ts::Value>::cast(e);
        CHECK(ce.has_value() && !MouseEvent::cast(e).has_value());
        got = ce->detail().to_number();
    };
    CustomEvent<ts::Value> custom = CustomEvent<ts::Value>::create("custom", init);
    CHECK(custom.detail().to_number() == 42);
    CHECK(custom.bubbles());
    outer.addEventListener("custom", onCustom);
    button.dispatchEvent(custom);
    CHECK(got == 42);
    CHECK(CustomEvent<ts::Value>::create("bare").detail().is_null());

    // A MouseEvent built with an init dictionary reaches a typed listener.
    MouseEventInit mouse = MouseEventInit::create();
    mouse.clientX(12);
    mouse.clientY(34);
    mouse.button(1);
    mouse.shiftKey(true);
    mouse.bubbles(true);
    double x = -1, y = -1, which = -1;
    bool shift = false;
    ts::Listener<MouseEvent> onMouse = [&](MouseEvent e) {
        x = e.clientX();
        y = e.clientY();
        which = e.button();
        shift = e.shiftKey();
    };
    button.addEventListener(HTMLElementEventMapKeys::mousedown, onMouse);
    MouseEvent down = MouseEvent::create("mousedown", mouse);
    button.dispatchEvent(down);
    CHECK(x == 12 && y == 34 && which == 1 && shift);
    CHECK(down.clientX() == 12 && !down.ctrlKey() && down.isTrusted() == false);
    onMouse.release();

    // A KeyboardEvent, too, with its key and code.
    KeyboardEventInit key = KeyboardEventInit::create();
    key.key("Enter");
    key.code("Enter");
    key.ctrlKey(true);
    key.repeat(true);
    key.bubbles(true);
    std::string seen;
    ts::Listener<KeyboardEvent> onKey = [&](KeyboardEvent e) {
        seen = e.key() + "/" + e.code();
        if (e.ctrlKey() && e.repeat()) seen += "/ctrl+repeat";
    };
    inner.addEventListener(HTMLElementEventMapKeys::keydown, onKey);
    button.dispatchEvent(KeyboardEvent::create("keydown", key));
    CHECK(seen == "Enter/Enter/ctrl+repeat");
    KeyboardEvent up = KeyboardEvent::create("keyup");
    CHECK(up.key() == "" && !up.shiftKey() && up.type() == "keyup");
    onKey.release();

    // composedPath lists the ancestors a bubbling event passes.
    std::string path_ok;
    size_t path_len = 0;
    ts::Scoped<EventListener> onPath = [&](Event e) { path_len = e.composedPath().size(); };
    outer.addEventListener("path", onPath);
    button.dispatchEvent(Event::create("path", bubbling));
    CHECK(path_len >= 4);

    // Listeners on the document and window see bubbled events too.
    int docHits = 0;
    ts::Scoped<EventListener> onDoc = [&](Event) { docHits++; };
    // FINDING 1: Document::addEventListener(string, listener) is ambiguous between
    // its own `const EventListenerOrEventListenerObject&` and the inherited
    // `std::optional<...>` overload; a variant made by hand picks the first.
    EventListenerOrEventListenerObject asVariant = EventListener(onDoc);
    doc.addEventListener("deep", asVariant);
    button.dispatchEvent(Event::create("deep", bubbling));
    CHECK(docHits == 1);
}
