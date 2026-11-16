# Findings: C library programs

Numbers match `// FINDING n` in tests/libraries/*/test.c (there is no 17).
Generated paths are under `target/e2e/libraries/<lib>-c-run/bindings/`
unless they start with `glue.mjs`.

## Runtime bugs

8. **Rest-parameter callbacks get only their first argument.** marked's renderer `heading`, node's `EventEmitter.on` listener and echarts' `chart.on` handler are wrapped `(a0) => f(ctx, ref(a0))` (`marked-c-run/glue.mjs:323`, `node-c-run/glue.mjs:343`), but typed `JsTuple_Tokens_Heading` (`ts_types.h:702`) or `JsArray args`. marked: `Cannot read properties of null (reading 'depth')`; node and echarts: `JsArray_length(args)` is 0.
11. **node: every `path_*` function throws.** The glue runs `PK["path"].path.join(...)` (`node-c-run/glue.mjs:385`), reading `export = path` as a property of the module.
16. **konva: no class can be constructed.** `new PK["konva"].Stage(...)` (`konva-c-run/glue.mjs:325`); the module exports only `default`. Settings work (`PK["konva"].default.version`). `Konva_get_Stage()` is a bare `JsValue`.
7, 15. **Literal-typed properties are only `#define`s, so objects made in C lack them.** marked's `marked_options` (`MarkedOptions & {async: true}`) has only `#define marked_options_async true`; echarts' `BarSeriesOption_type` is only `"bar"`, no setter: a series made with `BarSeriesOption_create()` has no `type`.

## Can't express

5. **marked: a function-typed variable can't be called.** `get_lexer()`, `get_parser()`, `get_marked_use()` return handles with only `_from`. No top-level `parseInline` getter.
13. **tanstack_table: function-typed properties can't be called** (`getRowModel`, `setSorting`, `ColumnHelper.accessor`). Worked around with `functionalUpdate(f, x)`, which loses `this` (`row.getValue` throws).
14. **tanstack_table: options aren't constructible.** `TableOptions_Table` members are readonly (getters only); `constructTable` needs `features.coreReactivityFeature` from the unbound subpath `@tanstack/table-core/store-reactivity-bindings`; `Atom`/`ReadonlyAtom` are opaque.
18. **prosemirror: `view.dispatch` can't be called** (`EditorView_get_dispatch` returns a `Transaction_fn` with no call).

## Awkward

1. date_fns: `isValid(any)` takes `JsValue`: `(JsValue){ d.h }`.
2. date_fns: `differenceInDays(Date, Date)` doesn't compile; the plain overload is `(string, string)`, others are `differenceInDays_with_Date_and_Date`.
3. Short function names become global macros (`max`, `min`, `add`, `map`, `use`, `point`, `init`); test-page's `canvas()` is `test_page_canvas` beside leaflet, plain `canvas()` beside pixi.
4. katex: a thrown `ParseError` arrives as `JsValue`: `(ParseError){ error.h }`.
6. marked: `marked(src)` needs `(marked_options_no_async){0}`; the optional-options overload is `marked_with_opt_MarkedOptions`.
9. node: `BufferEncoding` (enum) and `global_BufferEncoding` (opaque handle) are unrelated; `readFileSync` wants the handle. `PathLike_of` takes only `JsString`.
10. Zero-argument calls of default-argument macros (`EventEmitter_new()`, `Matrix_new()`) fail under `-std=c11 -pedantic -Werror` (C23 variadic extension); use `Matrix_new__0()`.
12. three: `Vector3` isn't a `Vector3Like`: `(Vector3Like){ v.h }`; optional numbers need `js_some_double(3)`.
19. prosemirror: `EditorView_new` wants `dist_DOMNode`; `view.dom` is opaque without DOM bindings.
20. pixi: `Rectangle` has 16 constructors from `string | number` unions; `contains` is `ShapePrimitive_contains(rect, ...)`.

## Not bugs

- leaflet `Circle_getBounds` threw because the circle wasn't on a map.
- echarts `resize` with explicit sizes sticks until `"auto"`: library behaviour.
