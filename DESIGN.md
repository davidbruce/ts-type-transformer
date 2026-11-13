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
| `overloads` | `rename` | same-named members | `f`, `f_with_x`, ... with `jsName` | no |

Four passes need the type checker. That is why lowering lives here and not in
each generator: a generator written in F# or C has no checker.

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

`cargo test` runs `tests/e2e.rs`. For each tested target (F# via Fable, C and
Zig via wasm32) it:

1. lowers `tests/fixtures/<name>/api.d.ts` with `examples/targets/<target>.toml`;
2. generates bindings with a reference generator in `tests/generators/`;
3. compiles them with the real toolchain;
4. runs `test.fs` / `test.c` / `test.zig` against the fixture's `impl.mjs` in Node.

The generators are thin on purpose: they print what the config produced and
panic on anything else. If a generator needs to be clever, the config or the
XML is wrong. `*_rejects_*` tests pin that strictness, and
`lowered_constructs_are_absent` checks every config on larger inputs.

A missing toolchain skips its target with a message.

## Known gaps

- Synthetic names come from the first place a type appears (`Shape_new_kind`). Not stable or pretty.
- `M[K]` with `K extends keyof M` widens to the union of all values. The per-key expansion (`on_abort(...)`) is not implemented.
- A union with no common supertype (`string | number`) becomes `any`.
- Tuples, and object types that are a `var`'s own type, are never hoisted.
- Output is not closed: references to other files (`Promise`) are not pulled in.
- Only the `fsharp`, `c` and `zig` example targets are tested, and only on one small fixture.
- Each construct has one lowering strategy so far. Values are strings so more can be added
  (`unions = "tagged"`, `generics = "monomorphize"`).
