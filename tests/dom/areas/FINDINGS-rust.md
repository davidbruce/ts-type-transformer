# Findings: Rust DOM area programs

**Status:** Fixed: 1, 3 (checked downcasts `T::cast(&v)` for classes).

Numbers match `// FINDING n` in tests/dom/areas/*/test.rs. No runtime bugs.

1. Can't express: no downcast from `Event` to a subtype (`CustomEvent`, `MouseEvent`) or `CSSRule` to `CSSStyleRule`; a listener added by a string name gets a plain `Event`. Rewrapped by hand.
2. Awkward: the DOM's `Option` constructor (`handle!(Option)`) hides `core::option::Option` under `use bindings::*`.
3. See 1 (`CSSRule` to `CSSStyleRule`).
4. Awkward: unions have no `PartialEq`, and a subtype doesn't convert into one (`ElementOrProcessingInstruction` from `HTMLStyleElement`).
5. Can't express: a `JsValue` can't become a `JsArray<T>` (private phantom), nor a `JsArray` a `JsValue`; no `JSON` global.
6. Can't express: `setTimeout` takes `TimerHandler = String | Function`; `Function` has no constructor from a closure. Worked around with `VoidFunction::new`.
7. Awkward: `IntersectionObserverInit::set_threshold` takes `VibratePattern` (`number | number[]` under another alias's name).
