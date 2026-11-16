# Findings: C DOM area programs

Numbers match `// FINDING n` in tests/dom/areas/*/test.c. No runtime bugs.

1. Awkward: `Node_or_string_of(x)` selects on the exact type: an `HTMLLIElement` is rejected where `Node_or_string` is wanted.
2. Awkward: `getComputedStyle(a1, a2)` has no default for the optional `pseudoElt`.
3. Awkward: `FormData_new(form, submitter)` has no default macros.
4. Awkward: zero-argument calls of optional-argument macros (`Path2D_new()`, `Headers_new()`) fail under `-std=c11 -pedantic -Werror`.
5. Can't express: `HeadersInit_map` (a record) has only `_create`, no way to set entries; no arbitrary object for `Response_json`.
6. Can't express: `setTimeout` takes `TimerHandler`; `Function` has no `_from` (worked around with `VoidFunction_from`); no default for the extra arguments.
7. Awkward: `IntersectionObserverInit_set_threshold` takes `VibratePattern`.
8. Awkward: no downcasts: listeners get `Event`; `cloneNode` returns `Node`; promise results `JsValue`.
9. Awkward: `Promise_then` loses the result type.

Not in comments: `get_name()` returns void (lib.dom declares `name: void`); the harness fails a page on any 404.
