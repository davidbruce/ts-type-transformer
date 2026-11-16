# Findings: Zig DOM area programs

Numbers match `// FINDING n` in tests/dom/areas/*/test.zig.

1. Runtime bug: `HTMLOptionElement.new()` runs `new HTMLOptionElement()`: "Illegal constructor". The real constructor is the global `Option` (`declare var Option: { new(text?, value?, ...): HTMLOptionElement }`), which has no binding; likewise `HTMLImageElement.new` (`Image`), `HTMLAudioElement.new` (`Audio`).
2. Awkward: no downcasts: `CSSRule` to `CSSStyleRule`, `Event` to `CustomEvent`/`MouseEvent`, `SVGElement` to `SVGLineElement`; handles rebuilt by hand.
3. Awkward: `Promise` has no type parameter: results cast by hand.
4. Awkward / can't express: Body's `json()` is `json2` (the static `Response.json` took the name); a parsed object's fields can't be read.
5. Awkward: `setTimeout` takes `TimerHandler = string | Function`; `Function` has no `from`. Worked around with `VoidFunction.from`.
