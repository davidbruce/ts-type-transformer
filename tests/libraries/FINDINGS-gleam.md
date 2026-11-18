# Findings: Gleam library programs

**Status:** Fixed: 1, 3, 4, 5, 6, 8, 13, 14, 15, 16, 17, 18, 21; partly 9 (a table is built; atoms are opaque).

Numbers match `// FINDING n` in tests/libraries/*/test.gleam. Paths are
under `target/e2e/libraries/<lib>-gleam/src/bindings/`.

## Blocking

13. **pixi: `runtime_ffi.mjs` imports `"*.frag"`** (an ambient `declare module '*.frag'`): `bun build` fails, no pixi program builds. Passes (51 checks) with it removed.
15. **konva: the default export isn't unwrapped.** `PK["konva"].Stage` is not a constructor. Passes (36) with `"konva": P0.default`.
17. **echarts: the bindings don't compile.** `areas/echarts.gleam:718` defines `pub type List(...) = SharedList(...)`, shadowing Gleam's `List`: 20 arity errors. Passes (21) with it renamed.

## Runtime bugs

1. marked: `get_marked_instance()` reads `markedInstance`, which isn't exported.
3. marked: extension renderer callbacks get `undefined`: `types_ffi.mjs:409` does `((v) => [v[0]])(a0)`, a rest parameter read as a tuple.
5. node: every `path` function throws: `PK["path"].path.X` (`areas/path_ffi.mjs`).
6. node: a `(...args)` listener gets only its first argument, through `Array.from` (`AnyArrayFn_from`).
16. konva: `GetSet` properties are called without `this` (`node_ffi.mjs:1549` returns `self.x`, `get_set_ffi.mjs:6` calls `self()`).

## Can't express

2. marked: the module's `parse` is `Dynamic`; `parseInline` isn't bound.
7. node: `GlobalBufferEncoding` is opaque with no values: no encoding can be given.
8. three: a `Vector3` can't be given where a `Vector3Like` is wanted; `vector3_like.create()` has no setters, there's no `as_vector3_like`.
9. tanstack_table: no table can be constructed: `constructTable` needs `coreReactivityFeature` from the unbound `@tanstack/table-core/store-reactivity-bindings`; `Atom`s are opaque.
11. pixi: `getBounds`, `toGlobal`, `setSize`... are on mixins with no `as_` upcast from `Container`.
12. pixi: `Texture.WHITE` (`Texture(BufferImageSource(Nil), Nil)`) is rejected by `SpriteOptionsOrTexture`.
14. konva: a Rect/Circle/Text can't be `add`ed: `GroupOrShapeShape` wants `Shape(ShapeConfig(Nil), Nil)`.
18. echarts: a series can't carry its `type`: `bar_series_option.gleam` has only `pub const type_ = "bar"`, no setter.
20. pixi: `ApplicationInitOptions.canvas` takes `PixiMixinsICanvas`; an `HTMLCanvasElement` doesn't fit.

## Awkward

4. marked: a string argument to a callback becomes a list of one-character `Dynamic`s (`f(toList(Array.from(a0)))`).
10. pixi: `add_child` takes the whole rest array (a list plus `runtime.codec_list(...)`); returns `Container(Dynamic, Nil)`.
19. prosemirror: `set_key`/`set_plugins` take `PluginKey(Dynamic, _)`; a `Plugin(Float, _)` can't be widened.

## Not bugs

- pixi `Point.add`, `Rectangle.intersection` are from `pixi.js/math-extras`, never loaded.
- rapier's `Vector` is an interface, built with `vector.create()` and setters.
