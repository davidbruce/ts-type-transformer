// Must not compile.
#include "areas/html/HTMLElement.hpp"

// A keydown listener takes a KeyboardEvent, not a PointerEvent.
void wrong(HTMLElement el, ts::Listener<PointerEvent> onClick) {
    el.addEventListener(HTMLElementEventMapKeys::keydown, onClick);
}

// querySelectorAll<E extends Element>: an Event is not an Element.
void wrong_type(HTMLElement el) {
    el.querySelectorAll<Event>("p");
}

// NodeListOf<TNode extends Node>: the class template itself refuses an Event.
void wrong_class(NodeListOf<Event> list);
