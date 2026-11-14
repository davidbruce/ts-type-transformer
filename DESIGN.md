# Design: capability-driven lowering

Experiment on the `tsrs-prototype` branch (`src/bin/tsrs_proto/`).

## Idea

Targets differ in *which* TypeScript constructs they lack, not in *how much*
flattening they need. OCaml has inheritance and generics but no overloading;
Rust has generics but no unions or optional arguments; C has none of them.
So there is no single flattening dial. There is one strategy per construct,
and a target is a config file that picks them. The tool knows no languages.

```
.d.ts -> front end -> IR -> lowering passes -> XML
            (tsrs)    as written,   one per construct,
                      refs resolved chosen by config
```

The XML schema is the same at every level. Lowering only removes element
kinds, so a generator handles exactly the kinds its config leaves in. The
output starts with a `<lowering>` element naming the strategies in effect.

## Config

```toml
[constructs]            # each: "keep" (default) or the strategy below
inheritance = "flatten"
generics = "erase"

[limits]
union_param_overloads = 16
intersection_members = 64

[naming]
separator = "_"
overloads = "with_params"   # or "numbered"
```

`--config target.toml`, `--set generics=erase` to override one setting,
`--default-config` to print a documented template. Unknown keys and values
are errors. Example targets are in `examples/targets/`.

| Construct | Strategy | Removes | Becomes | Needs checker |
|---|---|---|---|---|
| `classes` | `pair` | `var X: { prototype: X; new() }` + `interface X` | `<class>` with constructors and statics | no |
| `type_ops` | `evaluate` | `keyof`, `T[K]`, conditional, mapped, template, `typeof`, predicates, `Partial<T>`-style aliases | their evaluated type | yes |
| `intersections` | `merge` | `A & B` | one merged object type | yes |
| `inheritance` | `flatten` | `<extends>` | inherited members copied in, base type arguments applied; `<ancestor>` list kept | yes |
| `generics` | `erase` | type parameters | their constraint, or `any` | no |
| `nullable` | `wrap` | `T \| null \| undefined` | `<nullable absent="null\|undefined\|either">` | no |
| `literals` | `enum` | literal types | string unions -> `<enum>`, constants -> `const=`, others widened | no |
| `anonymous` | `hoist` | inline object and function types | named synthetic declarations | no |
| `optional_params` | `overloads` | optional parameters | one overload per arity | no |
| `union_params` | `overloads` | union-typed parameters | one overload per member | no |
| `unions` | `erase` | remaining unions | common supertype, else `any` | yes |
| `unions` | `name` | inline unions | a named declaration: `boolean_or_Options`, or named by where it appears | no |
| `keyed_params` | `expand` | `f<K extends keyof M>(key: K, value: M[K])` | one overload per key: `addEventListener_click`, `createElement_div` | yes |
| `keyed_params` | `map` | the same | kept generic, `keyof M` and `M[K]` as written, for targets with type-level keys | no |
| `overloads` | `rename` | same-named members | `f`, `f_with_x`, ... with `jsName` | no |

Four passes need the type checker. That is why lowering lives here and not in
each generator: a generator written in F# or C has no checker.

## Keyed signatures

`keyed_params` picks how `addEventListener<K extends keyof M>(type: K, listener: (ev: M[K]) => any)`
reaches a target. `expand` suits languages without generics (C:
`HTMLElement_addEventListener_click`). `map` keeps one generic method and the
map; the example targets use it as:

- Zig, comptime: `el.addEventListener(.click, JsListener(PointerEvent).from(f, &ctx))`,
  where `HTMLElementEventMap.Type(.click)` is `PointerEvent`.
- F#, typed keys: `el.addEventListener (HTMLElementEventMapKeys.click, fun (e: PointerEvent) -> ...)`,
  where `click: TypedKey<HTMLElementEventMap, PointerEvent>`.

Both reject a listener of the wrong event type at compile time.

## Modules

`[modules]` assigns each top-level declaration a module, written as
`module="..."`: the first `[[modules.area]]` whose `match` pattern fits its
name, else `default`; a namespace is a module of its own. Areas belong to a
library, not a target, so they live in their own file and are passed as a
second `--config` (`examples/modules/dom.toml`).

Module names become file and folder names, so the tool rejects area names
that are equal ignoring letter case, or reserved on Windows (`CON`, `NUL`,
`COM1`, ...). Namespace modules are marked `moduleKind="namespace"`: they come
from the library and cannot be renamed, so layouts keep them in a folder of
their own, apart from areas (`areas/css/`, `namespaces/CSS/`).

## Generators

`--emit c|cpp|zig|rust|fsharp --out <dir>` writes bindings for the lowered
XML, and for wasm targets the JavaScript glue; `--glue-for <module.wasm>`
trims the glue to what a compiled module imports. The generators
(`src/generators/`) are thin: what a config left in that a generator cannot
express is an error naming the setting to change.

Every wasm target shares one model (`generators::wasm`) and one glue. A
handle is a counted reference: what a function returns is the caller's,
arguments are lent, what a callback returns is given to JavaScript. A call
that throws returns a zero value and keeps the exception to take. Promises
settle into a function; iterators and async iterators are walked; unions are
told apart by their tests, and a bare `T` by what `T` is.

- C (C11): `ts_types.h` with every handle, enum, union and callback; one
  header per declaration in `areas/<module>/` or `namespaces/<module>/`.
  Inheritance is kept: `_Generic` upcasts (`AS_Element(x)`) let any subtype
  be passed. Optional values are `JsOptionalDouble`, `JsOptionalBool`, null
  handles and `E_absent`; unions tagged structs (`Value v = { .tag =
  Value_number, .as.number = 42 }`); arrays and tuples typed
  (`JsArray_double`, `JsTuple_double_JsString`). Handles are released one by
  one or by scope (`js_scope_open()`/`js_scope_close()`). The DOM: 8.5 MB.
- C++ (C++20, WASI SDK): a value type per JavaScript type, inheriting
  virtually from `ts::Handle`; copies retain, destructors release. Overloads,
  `std::optional`, `std::variant`, `ts::Array<T>`, `ts::Tuple<...>`, lambdas
  (`release()`, `ts::Scoped`), class templates with defaults and constraints,
  keyed methods over typed keys, `co_await` on promises (`ts::Task`), range-for
  over iterables. `X.hpp` includes what its members name; non-template
  definitions are one `.cpp` per module, compiled once. 6.9 MB.
- Zig: a file per module and a root `bindings.zig`; Zig reads only what is
  used. `?T`, tagged unions, `JsArrayOf(T)`, `JsTuple(&.{...})`; callbacks
  are comptime functions with Zig types, behind a C-convention trampoline;
  `defer jsRelease(x)` or `JsScope`.
- Rust (wasm32-unknown-unknown): a handle type per JavaScript type, `Clone`
  retaining and `Drop` releasing it. Each type's members are a trait
  (`ElementMethods`) implemented for it and its subtypes; where two traits
  share a name, an inherent method says which. Parameters of object types
  take `&impl AsRef<T>`, so any subtype goes in. `Option`, enums for unions,
  `JsArray<T>`, `Tuple2<A, B>`, closures, `Promise::then`, `Iterator`.
- F#: files must not refer to each other in cycles: they are ordered by what
  F# cannot refer to forward, and a member that mentions a later file's type
  becomes an extension there. Iterables are `seq<T>`; constraints of methods,
  functions, and interfaces every reference satisfies, are F# constraints.

## Unions

Every union member says how to recognise it at runtime, whatever the config:

```xml
<union>
  <primitive test="typeof:string" name="string"/>
  <ref test="instanceof:Widget" name="Widget" kind="class"/>
  <ref test="has:verbose" name="Options" kind="interface"/>
</union>
<union discriminant="kind">
  <ref test="tag:circle" name="Circle" kind="interface"/>
  <ref test="tag:square" name="Square" kind="interface"/>
</union>
```

`instanceof` is given for classes with a global constructor, `has:` for the
first required property no other member has, `tag:` when one required
property has a distinct literal value in every member. F# turns these into
`[<Erase>]` unions with `CaseTest`s or `[<TypeScriptTaggedUnion>]`; the wasm
glue into a tag function, so Zig can `switch` on a union a call returns.

## Overload names

Overloads with fixed values (from `keyed_params` or literal parameters) are a
family named after them: `addEventListener_click`,
`addEventListener_click_with_options`. Within a family the first keeps the
name, the others are named by the parameters they add (only the last, when
they extend it), else by the types that differ among same-named overloads, else
the one type that differs, else a number. An overload whose parameters repeat
an earlier one's is dropped: TypeScript would never choose it. A callback type
used in several places is named by what it takes: `HTMLElement_MouseEvent_fn`.

## Provenance

Lowering never silently loses the link to the source:

- `jsName` on renamed members: the real JavaScript name.
- `from` on inherited members: the declaring interface.
- `const` on widened literal properties: the value.
- `synthetic="true"` on declarations a pass created.
- `lossy="reason"` on every `any` a pass introduced.

## Measured (lib.dom.d.ts)

Constructs left in the output, from `--census`, with no config (raw) and
with each example target:

| | raw | fsharp | ocaml | rust | c | cpp | zig |
|---|---|---|---|---|---|---|---|
| `extends` | 843 | 843 | 843 | 843 | 843 | 843 | 0 |
| type parameters | 274 | 262 | 262 | 267 | 0 | 446 | 3172 |
| overloads | 493 | 493 | 0 | 0 | 0 | 1469 | 0 |
| optional parameters | 1198 | 1215 | 1215 | 1215 | 7932 | 0 | 4971 |
| unions | 2156 | 156 | 739 | 156 | 110 | 156 | 176 |
| literal types | 3349 | 0 | 0 | 0 | 0 | 0 | 0 |
| type-level expressions | 402 | 393 | 198 | 393 | 0 | 741 | 5906 |
| `any` introduced (`lossy`) | 0 | 2 | 2 | 2 | 2 | 2 | 2 |
| output lines | 86k | 75k | 78k | 73k | 181k | 89k | 457k |

Type parameters and type-level expressions that remain are keyed signatures
(`K extends keyof M`, `M[K]`) kept for typed keys.

## Tests

`cargo test` runs `tests/e2e.rs`: about 40 seconds, or 12 with
`-- --skip dom::`. Targets: F# (Fable), C and Zig (wasm32), C++ (WASI SDK),
Rust (`rustup target add wasm32-unknown-unknown`). A missing toolchain or
Chrome skips its tests with a message.

**Fixtures in Node.** For each of `shapes`, `callbacks`, `data`, `globals` and
`unions` in `tests/fixtures/`, and each target: lower `api.d.ts` with
`examples/targets/<target>.toml`, generate bindings, compile them with the
real toolchain, and run `test.*` against `impl.mjs` in Node. A program may
export `finish`, called once asynchronous work has settled.

**The whole DOM in Chrome.** `dom::{fsharp,c,zig,cpp,rust}` generate bindings
for all of `lib.dom.d.ts`, compile them, and run `tests/dom/test.*` in
headless Chrome: build part of the page, read existing content, add and
remove listeners driven by real clicks, dispatch events, set styles, iterate,
catch an exception, fetch, and check that handles are freed. Then
`tests/dom/verify.mjs` inspects the page with plain DOM calls.

Strictness is pinned: `*_rejects_*` tests, `lowered_constructs_are_absent`
over every config, `generated_names_do_not_depend_on_order`, `wrong.c` and
`wrong.cpp` (which must not compile), and `fsharp_checks_constraints`.

## Known gaps

- OCaml has a config but no generator: it needs `js_of_ocaml` or `wasm_of_ocaml`.
- 8 DOM unions have members no runtime test can tell apart (two array types,
  `ArrayBufferLike | ArrayBufferView`): bound as untyped values.
- C++ class templates check constraints in constructors, not with `requires`:
  a template is named before its arguments are complete. Rust states none.
- F# has no async iteration. Rust and Zig callbacks with a union argument
  receive it decoded; C's receive a handle.
- Hoisted names are stable but come from paths: `SVGSVGElement_getEnclosureList_result`.
- Zig flattens inheritance: its DOM bindings are 26 MB on disk, read lazily.
- Each construct has one lowering strategy so far. Values are strings so more
  can be added (`generics = "monomorphize"`).
