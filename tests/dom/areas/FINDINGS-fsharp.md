# Findings: F# DOM area programs

**Status:** Fixed: 6 (`Function.Create` and `Function.Invoke`), 7 (`NumberOrNumberArray`).

Numbers match `// FINDING n` in tests/dom/areas/*/test.fs.

1. Awkward: the value `Option` (the `<option>` constructor) hides F#'s `Option` module when `Bindings` is open.
2. Awkward: `AddEventListenerOptions.Create (capture = true)` picks the inherited `EventListenerOptions.Create`.
3. Awkward: a typed listener is an F# function; each use passes a fresh wrapper, so `removeEventListener` finds nothing unless one closure is bound and reused.
4, 5. Awkward: `RequestInit` and `BlobPropertyBag` have no `Create`.
6. Can't express: `setTimeout` takes `TimerHandler = String | Function`; `Function` is empty: an F# function needs a cast.
7. Awkward: `IntersectionObserverInit.threshold` is typed `VibratePattern`.
8. Awkward: `createElementNS (ns, name)` is ambiguous between two overloads of one shape.
9. Can't express: `SVGElement.className` is `obj`, not `SVGAnimatedString`.
