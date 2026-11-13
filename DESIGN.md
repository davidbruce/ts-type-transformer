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

How a generator lays out files is its own choice. The test generators:

- C: `ts_types.h` with every handle, enum and callback; one header per
  declaration in `areas/<module>/` or `namespaces/<module>/`; umbrella
  headers. A program includes what it uses. Inheritance is kept: a member is
  defined once, on its declaring type, and every type has a cast to each
  ancestor (`Element_setAttribute(HTMLElement_as_Element(el), ...)`). Per-key
  methods share one wasm import per call shape (`addEventListener_keyed`).
  File names that would still clash get a suffix (`point_2.h`, `Con_.h`).
- Zig: a file per module, in the same folders, importing by name what it uses
  from the others, and a root `bindings.zig`. Zig reads only what is used.
- F#: files must not refer to each other in cycles. Files are ordered by what
  F# cannot refer to forward: base types and namespaces. What other modules
  inherit from is split off as `<module>.base`; aliases go where their parts
  are. A member that mentions a type of a later file becomes an extension
  member there (`type Node with member this.ownerDocument ...`): 725 of
  11,500 for the DOM. One file per area, the largest 168 KB.

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

| | raw | fsharp | ocaml | rust | c |
|---|---|---|---|---|---|
| `extends` | 843 | 843 | 843 | 843 | 0 |
| type parameters | 574 | 562 | 562 | 1521 | 0 |
| overloads | 793 | 793 | 0 | 0 | 0 |
| optional parameters | 1798 | 1815 | 1815 | 0 | 0 |
| unions | 2756 | 1617 | 1617 | 0 | 0 |
| literal types | 3349 | 0 | 0 | 0 | 0 |
| type-level expressions | 1021 | 0 | 0 | 0 | 0 |
| `any` introduced (`lossy`) | 0 | 2 | 2 | 329 | 1433 |
| output lines | 102k | 100k | 100k | 126k | 508k |

## Tests

`cargo test` runs `tests/e2e.rs` (about 25 seconds). Missing toolchains or a
missing Chrome skip their tests with a message.

**Fixtures in Node.** For each of `shapes`, `callbacks`, `data` and `globals`
in `tests/fixtures/`, and each target (F# via Fable, C and Zig via wasm32):

1. lower `api.d.ts` with `examples/targets/<target>.toml`;
2. generate bindings with a reference generator in `tests/generators/`;
3. compile them with the real toolchain;
4. run `test.fs` / `test.c` / `test.zig` against `impl.mjs` in Node.

**The whole DOM in Chrome.** `dom::{fsharp,c,zig}` generate bindings for all of
`lib.dom.d.ts`, compile them (Zig references every declaration, so all of it
is type-checked), and run `tests/dom/test.*` in headless Chrome via
`tests/run_browser.mjs`. The program builds part of the page, reads existing
content, adds and removes listeners driven by real clicks, dispatches events,
and sets styles; then `tests/dom/verify.mjs` inspects the page with plain DOM
calls, independent of the bindings.

**The wasm boundary.** Objects, arrays, strings coming back and untyped values
are integer handles; a callback is a handle made from a wasm function pointer
and a context pointer. The JavaScript side is generated per API
(`generators::wasm::glue`): about 107,000 imports, 10 MB, for the DOM.

The generators are thin on purpose: they print what the config produced and
panic on anything else. If a generator needs to be clever, the config or the
XML is wrong. `*_rejects_*` tests pin that strictness, and
`lowered_constructs_are_absent` checks every config on larger inputs.

## Known gaps

- Synthetic names come from the first place a type appears (`Shape_new_kind`). Not stable or pretty.
- `keyed_params = "expand"` still makes the C bindings large (36 MB): every HTML and SVG element redeclares `addEventListener` with its own `this`. The wasm glue is trimmed to what a program imports.
- 18 DOM unions have members no test can tell apart (`ArrayBufferLike | ArrayBufferView`); F# binds them as `obj`.
- A union with no common supertype (`string | number`) becomes `any`.
- Tuples, and object types that are a `var`'s own type, are never hoisted.
- Output is not closed: references to other files (`Promise`) are not pulled in.
- Only the `fsharp`, `c` and `zig` example targets are tested; `rust` and `ocaml` are not.
- Each construct has one lowering strategy so far. Values are strings so more can be added
  (`unions = "tagged"`, `generics = "monomorphize"`).
