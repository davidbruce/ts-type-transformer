# Design: capability-driven lowering

Experiment on the `tsrs-prototype` branch (`src/bin/tsrs_proto/`).

## Idea

Targets differ in *which* TypeScript constructs they lack, not in *how much*
flattening they need. OCaml has inheritance and generics but no overloading;
Rust has generics but no unions or optional arguments; C has none of them.
So there is no single flattening dial. There is one switch per construct, and
a profile is a named set of switches.

```
.d.ts -> front end -> IR -> lowering passes -> XML
            (tsrs)    as written,   one per construct,
                      refs resolved chosen by profile
```

The XML schema is the same at every level. Lowering only removes element
kinds, so a generator handles exactly the kinds its profile leaves in.

## Switches

`true` = lower the construct away. `--profile P`, then `--lower a,b` / `--keep a,b`.

| Switch | Removes | Becomes | Needs checker |
|---|---|---|---|
| `classes` | `var X: { prototype: X; new() }` + `interface X` | `<class>` with constructors and statics | no |
| `typeops` | `keyof`, `T[K]`, conditional, mapped, template, `typeof`, predicates, `Partial<T>`-style aliases | their evaluated type | yes |
| `intersections` | `A & B` | one merged object type | yes |
| `inheritance` | `<extends>` | inherited members copied in, base type arguments applied; `<ancestor>` list kept | yes |
| `generics` | type parameters | their constraint, or `any` | no |
| `nullable` | `T \| null \| undefined` | `<nullable>` | no |
| `literals` | literal types | string unions -> `<enum>`, constants -> `const=`, others widened | no |
| `anonymous` | inline object and function types | named synthetic declarations | no |
| `optionals` | optional parameters | one overload per arity | no |
| `param_unions` | union-typed parameters | one overload per member | no |
| `unions` | remaining unions | common supertype, else `any` | yes |
| `overloads` | same-named members | `f`, `f_with_x`, ... with `jsName` | no |

Profiles: `raw` (nothing), `fsharp`, `ocaml`, `rust`, `c` (everything). See `Options::profile`.

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

Constructs left in the output, from `--census`:

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

## Known gaps

- Synthetic names come from the first place a type appears (`Shape_new_kind`). Not stable or pretty.
- `M[K]` with `K extends keyof M` widens to the union of all values. The per-key expansion (`on_abort(...)`) is not implemented.
- A union with no common supertype (`string | number`) becomes `any`.
- Tuples, and object types that are a `var`'s own type, are never hoisted.
- Output is not closed: references to other files (`Promise`) are not pulled in.
- The `rust` and `ocaml` profiles are guesses. No generator has consumed any profile yet.
