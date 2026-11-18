# Findings: Rust library programs

**Status:** Fixed: 1, 2, 5, 6, 7, 10, 11, 12, 15, 16, 19, 20.

Numbers match `// FINDING n` in tests/libraries/*/test.rs. Paths are under
`target/e2e/libraries/<lib>-rust-run/bindings/src/` unless `glue.mjs`.

## Blocking

19. **konva: the default export isn't unwrapped.** The glue reads `PK["konva"].Stage`; `import("konva")` has only `default`: `PK.konva.Stage is not a constructor`. With `impl.mjs` giving `.default`, all 39 checks pass.
20. **echarts: the crate doesn't compile with test-page bound.** `pub use echarts::SeriesOption$1Methods as _;` (`lib.rs:240`); `$1` names reach Rust identifiers (`types.rs`, `areas_main.rs`, `echarts.rs`). echarts alone compiles.
6. **node: every `path` function throws.** `PK["path"].path.join(...)` (`glue.mjs:384`): `export = path` read as a property of the module.

## Runtime bugs

1. marked: `marked_instance()` reads `markedInstance`, declared but not exported: a null handle.
2, 5. Rest-parameter callbacks (`...args`) get their first argument as the "array": marked renderer hooks, node `AnyArrayFn` (`glue.mjs:345`), echarts handlers. `(a0) => f(ctx, ref(a0))`.
16. konva: `GetSet` properties (`x`, `fill`, ...) throw: `Node_get_x` returns `self.x` and `GetSet_invoke` calls it bare, losing `this` (`reading 'getX'`).

## Can't express

3. node: `Buffer` isn't a `Uint8Array`; bytes from `read_file_sync` can't be read.
4. node: `BufferEncoding` is an opaque `GlobalBufferEncoding`: `"utf8"` can't be passed.
7. three: `Vector3` isn't `AsRef<Vector3Like>` (nor `Quaternion` a `QuaternionLike`): `add`, `sub`, `dot`, `lerp`, ... don't compile.
9. tanstack: `TableOptionsCore`/`TableOptionsTable` have getters only.
10. tanstack (major): a value of function type (`table.getRowModel`, `setSorting`, `columnHelper.accessor`, reactivity callbacks) can be made but not called; only types with call signatures have `invoke`. Worked around with `functionalUpdate(fn, arg)`. `Atom<T>` is opaque; `coreReactivityFeature` is in an unbound sub-entry.
12. pixi: `Texture::white()` is `Texture<BufferImageSource>`, not accepted as `Texture<TextureSource<JsValue>>`.
14. pixi: an `HTMLCanvasElement` isn't an `ICanvas`.
15. konva: a `Rect`/`Circle` (`Shape<RectConfig>`) can't be passed to `layer.add`: `GroupOrShape` takes only `Shape<ShapeConfig>`.
18. prosemirror: `this`-returning methods (`addMark`, `delete`, ...) return `Transform`, not `Transaction`.

## Awkward

8. three: `get_attribute<K: JsType>(name: K)` needs a `String`; returns a union enum.
11. `JsArray` and `JsValue` don't convert either way.
13. pixi: `add_child` spreads its argument: a single child needs `JsArray::from(vec![child])`.
17. prosemirror: the root and `prosemirror_model` both define `SchemaSpec`: glob imports clash (E0659).
- marked: top-level `parse` is a `JsValue`; no `parseInline`; options required (`marked_with_opt_marked_options`).
- node: `path::sep()` is `Sep { Value, Value1 }`.
- rapier: `World::new` takes 14 arguments, 12 internal `Raw*`.
- konva: `StageConfig::container` wants `HTMLDivElement`; DOM types are opaque; the `GetSet` setter takes `JsValue`.
- pixi: `Texture::<TextureSource<JsValue>>::white()` needs an unused type parameter; Rectangle arguments are `NumberOrString`.
- echarts: event names for `on`/`trigger` must be `String`.
