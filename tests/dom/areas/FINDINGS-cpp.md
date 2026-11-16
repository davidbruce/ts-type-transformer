# Findings: C++ DOM area programs

**Status:** Fixed: 9.

Numbers match `// FINDING n` in tests/dom/areas/*/test.cpp. Lines refer to
`target/e2e/dom-cpp/bindings/`.

1. Awkward: `Document::addEventListener("deep", listener)` is ambiguous (`const EventListenerOrEventListenerObject&` vs `std::optional<...>`).
2. Awkward: a string-typed listener gets an `Event`; no cast to `CustomEvent<T>`.
3. Awkward: `FormData::append("k", "v")` is ambiguous between `string_view` and `string_or_Blob`.
4. Can't express: no downcast (`CSSRule` to `CSSStyleRule`, `Node` to an element, `Event` to `CustomEvent`).
5. Can't express: `Response.json()` gives a `ts::Value` with no object access; no object literal.
6. Awkward: `Response`'s static `json(data)` hides the inherited `Body::json()`.
7. Same as 5 for stored JSON; no `JSON` object.
8. Can't express: `setTimeout` takes `TimerHandler = variant<string, Function>`; `Function` has no constructor from a lambda; `...arguments` is required.
9. **Runtime bug:** assigning a temporary to a handle type with several virtual-base paths (`Document`, `Element`, `HTMLDivElement`) leaves it null: the implicit move assignment runs `Handle& operator=(Handle o)` (`ts_runtime.hpp:81`, `src/generators/cpp.rs:458`) once per base path, each after the first taking from an emptied source.
10. Awkward: `ts::Array<double>{}` is a null handle, not an empty array.
