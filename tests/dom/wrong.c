// Must not compile: a CSSStyleDeclaration is not an Element.
#include "areas/core/Element.h"
#include "areas/css/CSSStyleDeclaration.h"

void wrong(CSSStyleDeclaration style) {
    Element_setAttribute(style, "data-x", "1");
}
