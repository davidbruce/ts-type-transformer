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

[rename]                # names for declarations a pass generated
SVGSVGElement_getEnclosureList_result = "SVGShapeElement"

[library]               # an ES module, not globals: written as <declarations import="pts">
module = "pts"
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
| `redeclarations` | `drop` | a method a type declares again as its ancestor does, but for `this` | the ancestor's alone | no |

Four passes need the type checker. That is why lowering lives here and not in
each generator: a generator written in F# or C has no checker.

`declare class` is a `<class>` as written: constructors, statics, `extends`
and `implements`; private and protected members are left out.

`[rename]` names generated declarations (hoisted types, enums, unions), and
every reference follows. A declared name is the library's and JavaScript's,
so renaming one is an error; a name a target did not generate is skipped,
since a library's table serves every target.

The XML has a schema, `schema/declarations.rng` (RELAX NG, notes in
`schema/README.md`); `xml_follows_the_schema` validates every fixture, the
DOM and Pts against it for each target.

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
(`src/generators/`) are thin and return errors: everything a config left in
that a target cannot express, each kind once, with how often, where first,
and the setting that lowers it.

```
C generator cannot express <literal>: set literals = "enum" (8 places, first at declarations/typeAlias[x_or_x]/union/literal)
```

Every wasm target shares one model (`generators::wasm`) and one glue. A
handle is a counted reference: what a function returns is the caller's,
arguments are lent, what a callback returns is given to JavaScript. Promises
settle into a function; iterators and async iterators are walked; unions are
told apart by their tests, and a bare `T` by what `T` is.

The glue is `makeEnv(getInstance, options)`:

- `root`: the object the API is found on, for a library that is a module
  rather than globals (`import * as pts from "pts"; makeEnv(.., { root: pts })`).
- `check`: handle mistakes are reported: `handle 7 used after its release`,
  `released twice`. Off by default: slots are reused.
- `unchecked(e)`: called with an exception nobody took (default `console.error`).

A call that throws returns a zero value and keeps the exception to take
(`js_take_error`, `ts::attempt`, `jsTry`, `js::attempt`); one not taken by
the time control returns to JavaScript is reported, not lost.

- C (C11): `ts_types.h` with every handle, enum, union and callback; one
  header per declaration in `areas/<module>/` or `namespaces/<module>/`.
  Inheritance is kept: `_Generic` upcasts (`AS_Element(x)`) let any subtype
  be passed. A function's name is also a macro, so trailing optional
  arguments may be left out (`Event_new("ping")`; all of them, `orUnit()`,
  in C23) and the first of a family of overloads picks one by its
  argument's type (`Shape_label(s, "a")`, `Shape_label(s, 7)`). Each shorter
  call is one line that makes the next longer one, and is written once
  where several overloads share it. Optional values are `JsOptionalDouble`,
  `JsOptionalBool`, null handles and `E_absent`; unions tagged structs made
  from a member (`Value_of(widget)`, `Value_of_number(42)`); arrays and
  tuples typed (`JsArray_double`, `JsTuple_double_JsString`). Handles are
  released one by one or by scope (`js_scope_open()`/`js_scope_close()`).
  The DOM: 10 MB.
- C++ (C++20, WASI SDK): a value type per JavaScript type, inheriting
  virtually from `ts::Handle`; copies retain, destructors release. Overloads,
  `std::optional`, `std::variant`, `ts::Array<T>`, `ts::Tuple<...>`, lambdas
  (`release()`, `ts::Scoped`), class templates with defaults and
  `requires`-clauses, keyed methods over typed keys, `co_await` on promises
  (`ts::Task`), range-for over iterables and over subclasses of Array.
  Constraints are checked of types that are only declared: each type's bases
  are declared beside it (`ts::bases<X>`) and `ts::extends` walks them.
  `X.hpp` includes what its members name; non-template definitions are one
  `.cpp` per module, compiled once. 7 MB.
- Zig: a file per module and a root `bindings.zig`; Zig reads only what is
  used. `?T`, tagged unions, `JsArrayOf(T)`, `JsTuple(&.{...})`; callbacks
  are comptime functions with Zig types, behind a C-convention trampoline;
  `defer jsRelease(x)` or `JsScope`. Trailing optional parameters are one
  struct whose fields default to null (`s.resize(2.6, .{})`,
  `s.resize(2.6, .{ .round = true })`); a lone one stays a `?T`. Inheritance is flattened, and an
  inherited member is written once: the furthest ancestor that declares it
  writes it in `X.Methods(Self)`, and each subtype's member is an alias. 12 MB.
- Rust (wasm32-unknown-unknown): a crate (`Cargo.toml`, `src/`), compiled
  once: a DOM program then builds in under a second. A handle type per
  JavaScript type, `Clone` retaining and `Drop` releasing it. Each type's
  members are a trait (`ElementMethods`) implemented for it and its subtypes;
  where two traits share a name, an inherent method says which. Names are
  Rust's: `get_element_by_id`, `text_content()` and `set_text_content(..)`,
  `ShapeKind::Square`, `HTMLElementEventMap::CLICK`. Arguments are borrowed:
  an object is `&impl AsRef<T>`, so any subtype goes in; a union or an array
  is `impl Into<..>`, so `describe(42.0)` and `join("-", ["a", "b"])` work; a
  value of a type parameter goes by value. Overloads that differ in a
  key-or-string parameter are one method (`query_selector(HTMLElementTagNameMap::P)`,
  `query_selector("p.intro")`), and where they differ in other parameters too,
  the key fixes their types (`add_event_listener(HTMLElementEventMap::CLICK,
  &listener, None)`, `add_event_listener("ping", on_ping, &options)`). An
  optional union argument is any member or `None`. A defaulted type argument is stated with
  `query_selector_as::<T>`. `T extends Node` is `T: Is<Node>`. Callbacks are
  closures, kept (`new`, `release`) or for the length of a call and free to
  borrow (`scoped`); promises are awaited under `js::spawn`. 6.6 MB.
- F#: files must not refer to each other in cycles: they are ordered by what
  F# cannot refer to forward, and a member that mentions a later file's type
  becomes an extension there. Iterables are `seq<T>`, async iterables
  `JsAsyncIterator<'T>`, a subclass of Array a `seq<T>` with an indexer;
  constraints of methods, functions and interfaces are F# constraints, kept
  through keyed values (`getElementsByTagName<'K when 'K :> Element>`). An
  index signature is `JsRecord<string, T>`; `f(a, b?, ...rest)` is two
  overloads. A library that is a module is imported (`[<Import("Pt",
  "pts")>]`): values, functions and classes, and a union case of a class is
  tested by `instanceof` on the import.

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

## Benchmarks

`cargo test --test e2e bench -- --ignored --nocapture` runs
`tests/bench/` in Node: the same loops in TypeScript, F# (Fable) and each
wasm target, warmed, interleaved, best of five, checksums compared.
Nanoseconds per iteration on an Apple M4 Max:

| | TypeScript | F# | C | C++ | Zig | Rust |
|---|---|---|---|---|---|---|
| call a function | 0.7 | 0.7 | 6.9 | 6.8 | 6.9 | 6.9 |
| make an object, call a method | 0.8 | 0.8 | 23 | 24 | 23 | 24 |
| set and get a property | 0.6 | 0.6 | 7.1 | 7.8 | 7.1 | 7.2 |
| pass a string (same bytes) | 0.4 | 0.3 | 22 | 19 | 23 | 24 |
| pass a string (new each time) | 0.5 | 0.5 | 59 | 60 | 61 | 61 |
| receive a string | 0.3 | 0.5 | 44 | 42 | 46 | 56 |
| be called back | 6.0 | 6.2 | 5.0 | 5.0 | 5.0 | 6.4 |
| compute without crossing | 1.1 | 1.1 | 0.6 | 1.0 | 0.6 | 0.6 |

F# compiles to JavaScript and costs what TypeScript does: Fable's loops are
the same JavaScript, and V8 inlines the call. (The workloads keep their
numbers off V8's small-integer path: a sum that outgrows it is boxed on every
iteration, and once made a call cost 5 ns in both.) A wasm target
pays 5 to 8 ns to cross the boundary, a handle-table slot per object, and a
decode or encode per string (a string at the same address with the same
bytes is decoded once). Code that stays inside wasm is as fast or faster.
So: bind coarse operations, not inner loops; keep strings out of hot paths.

## Tests

`cargo test` runs `tests/e2e.rs`: 49 tests in about 25 seconds, or 10 with
`-- --skip dom::`. Targets: F# (Fable), C and Zig (wasm32), C++ (WASI SDK),
Rust (`rustup target add wasm32-unknown-unknown`). A missing toolchain or
Chrome skips its tests with a message.

**Fixtures in Node.** For each of `shapes`, `callbacks`, `data`, `globals`,
`unions` and `pts` in `tests/fixtures/`, and each target: lower `api.d.ts`
with `examples/targets/<target>.toml`, generate bindings, compile them with
the real toolchain, and run `test.*` against `impl.mjs` in Node, with handle
checks on. A program may export `finish`, called once asynchronous work has
settled.

**A real library.** `pts` is Pts 1.0.3 (Apache-2.0), vendored unchanged: its
`index.d.ts` and its module. Every target compiles all of its bindings and
runs the same program: constructors with rest arguments, accessors, methods
that return `this`, statics, unions as arguments, a subclass of Array
indexed and iterated.

**The whole DOM in Chrome.** `dom::{fsharp,c,zig,cpp,rust}` generate bindings
for all of `lib.dom.d.ts`, compile them, and run `tests/dom/test.*` in
headless Chrome: build part of the page, read existing content, add and
remove listeners driven by real clicks, dispatch events, set styles, iterate,
catch an exception, fetch, and check that handles are freed. Then
`tests/dom/verify.mjs` inspects the page with plain DOM calls.

Strictness is pinned: `*_rejects_*` and `every_problem_is_reported`,
`lowered_constructs_are_absent` over every config,
`generated_names_do_not_depend_on_order`, `mistakes_are_reported`, `wrong.c`
and `wrong.cpp` (which must not compile), `fsharp_checks_constraints` and
`rust_checks_constraints`.

## Known gaps

- OCaml has a config but no generator: it needs `js_of_ocaml` or `wasm_of_ocaml`.
- 8 DOM unions have members no runtime test can tell apart (two array types,
  `ArrayBufferLike | ArrayBufferView`): bound as untyped values. A union with
  an `any` member is `any`.
- F# on Pts skips 7 unions whose members no test tells apart. Namespaced
  classes of a module library are still tested through `globalThis`.
- C: an empty call (`orUnit()`) is C23; under C11 `-pedantic` warns of it. A
  compound literal with a comma needs parentheses to pass through a macro:
  unions have `U_of` so it is seldom written. `_Generic` takes exact types,
  so a subtype is upcast first.
- Rust: a nullable union argument takes a member or `None`, not `Some(member)`.
- The wasm glue takes a module library as `options.root`; it does not read
  `import` yet.
- Rust and Zig callbacks with a union argument receive it decoded; C's
  receive a handle.
- Each construct has one lowering strategy so far. Values are strings so more
  can be added (`generics = "monomorphize"`).
