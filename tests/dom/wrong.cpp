// Must not compile: a keydown listener takes a KeyboardEvent, not a PointerEvent.
#include "areas/events/PointerEvent.hpp"
#include "areas/html/HTMLElement.hpp"

void wrong(HTMLElement el, ts::Listener<PointerEvent> onClick) {
    el.addEventListener(HTMLElementEventMapKeys::keydown, onClick);
}
