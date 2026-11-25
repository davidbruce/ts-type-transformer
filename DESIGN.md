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
| `aliases` | `expand` | generic aliases of other than an object, function or union: `type List<T> = T[]` | each use is the type with its arguments: `number[]` | no |
| `generics` | `erase` | type parameters | their constraint where it says what a value is, else what the library instantiates them with, else their default | no |
| `generics` | `checking` | the same, but those the user fills | as `erase`; `Array<T>`, `map<U>(f): U[]` stay | no |
| `constraints` | `nominal` | a declaration's type parameter constraints, which TypeScript checks by shape | kept only where every use in the output names a type declared to extend it, or a type parameter so constrained, as C++'s `ts::extends` checks | no |
| `constraints` | `exact` | the same | as `nominal`, but a type parameter meets only the constraint it has: Rust's `T: Is<Readable>` is not `Is<Stream>` | no |
| `nullable` | `wrap` | `T \| null \| undefined`, and `T \| false` | `<nullable absent="null\|undefined\|either\|false">` | no |
| `literals` | `enum` | literal types | string unions -> `<enum>`, constants -> `const=`, others widened | no |
| `anonymous` | `hoist` | inline object and function types | named synthetic declarations | no |
| `optional_params` | `overloads` | optional parameters | one overload per arity | no |
| `union_params` | `overloads` | union-typed parameters | one overload per member | no |
| `union_params` | `untyped` | the same, where a member has no type of its own (`string \| symbol`) | one overload per member; other unions stay | no |
| `unions` | `erase` | remaining unions | common supertype, else `any` | yes |
| `unions` | `name` | inline unions | a named declaration: `boolean_or_Options`, or named by where it appears | no |
| `keyed_params` | `expand` | `f<K extends keyof M>(key: K, value: M[K])` | one overload per key: `addEventListener_click`, `createElement_div` | yes |
| `keyed_params` | `map` | the same | kept generic, `keyof M` and `M[K]` as written, for targets with type-level keys | no |
| `overloads` | `rename` | same-named members | `f`, `f_with_x`, ... with `jsName` | no |
| `redeclarations` | `drop` | a method a type declares again as its ancestor does, but for `this` | the ancestor's alone | no |

`generics = "checking"` erases what only TypeScript's checking uses and
keeps what the user fills: a type parameter of a class or of an interface
with methods or an index, without a default (`Array<T>`, `NodeListOf<TNode
extends Node>`, TanStack's `Table<TData>`); one nothing constrains, without
a default or defaulting to `any` (prosemirror's `OrderedMap<T = any>`), or
defaulting to another (Konva's `GetSet<Type, This, SetType = Type>`); a
function's that its result depends on, unconstrained (`map<U>(f): U[]`) or
handing the caller's value back (`appendChild<T extends Node>(node: T): T`).
The rest goes. Where its constraint says what a value is, to its default if
that is a type (what a reference that does not say gets: pixi's
`toGlobal<P extends PointData = Point>` gives a `Point`), else to the
constraint (date-fns' `DateType extends Date`; prosemirror's `Nodes extends
string`, over which `{ [name in Nodes]: NodeSpec }` is a record). An event
map's (`T extends EventMap<T> = any`), to its default. Else to what the
library instantiates it with (ECharts' `LineStyleOption<Clr = ZRColor>` is
used with `ZRColor | ZRColor[]` too, so split lines take an array of
colours), else to its default. `erase` erases every one so; one the user
fills, to its constraint or default. A merged declaration's parameter has
the default any of its declarations gives.

`T | false`, of an object or a string with nothing else of `true`,
`boolean`, `null` or `undefined` beside it, is a nullable `T` whose absent
value is `false` (marked's renderer, three's octree `Intersection | false`):
an option in each target but F#, which keeps `T | bool`. A property whose
type is only call signatures is a method of each (`Marked#parse`). Overloads
an options object's boolean literal tells apart pass it themselves, the one
that leaves it open gone: `marked(src)` is a string, `marked_async(src)` a
promise, whatever the library is set to.

A rest parameter of a tuple of fixed length is its elements, as TypeScript
has it: pixi's `stroke(...args: Parameters<GraphicsContext['stroke']>)` is
`stroke(style?)`, and marked's renderer callbacks take their token.

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

Overloads a library writes by hand are keyed signatures too: where a parameter
of string literals tells same-named overloads apart (`on(type: 'click' |
'dblclick', fn)` beside `on(type: 'load', fn)` and `on(type: string, fn)`),
each is one overload per literal, with the literal fixed (`on_click`,
`on_dblclick`), under either strategy and the same `keyed_overloads` bound.
A union of literals that no other overload differs from stays an enum
parameter, as does a lone one.

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

## Packages

`--package date-fns` names a package as a user imports it, resolved from
`--from <dir>` as TypeScript resolves an import there: by `exports`,
`types`, `typesVersions` or `@types` (every installed `@types` package is
loaded, as TypeScript does). A package can be an ambient module that
another declares (`--package fs`, `declare module "fs"` in `@types/node`;
`node:fs` is the same module). Several `--package`s are bound together:
one checker and one front end, so what one package refers to of another is
the same declaration, and each top-level declaration says which package it
is imported from, `import="prosemirror-model"`, and is in that package's
module of the output. Names two packages share are apart in the output
(`pb_Node`) and keep their JavaScript name (`jsName="Node"`); a union's
`instanceof:pb_Node` names the declaration, which says where it is reached.
The `packages` fixture runs this in C, F# and Gleam.

Or the input is an entry file. The checker reads what it imports from the disk,
and the output is what the package declares and the entry file exports,
followed through `export *`, `export { a as b }`, `export * as ns` and
import types (`import("./x").Y`), and what those refer to in the same
package. Another package's types are referred to, not copied.

A file with top-level exports is an ES module, and bindings import it by its
package's name: `[library] module`, else the `name` of the nearest
package.json above the file, written as `<declarations import="...">`. With
neither, the tool warns and the bindings treat it as globals: a name made up
from the file's would be imported and not found.

What a module exports under another name is known by that name
(`export { Shape as Figure }` is `Figure`, in references and `instanceof`
tests too). A default export keeps its own name and says what it is
exported as, `jsName="default"`; an unnamed one is named for its file
(`anon_default`, or for `index.d.ts` its directory). A `const`, and every
variable a module exports, is `readonly="true"` and has no setter.

A bare `lib.*.d.ts` input is one of TypeScript's lib files, checked with
`--lib` (by default es2020 and the DOM's). A namespace it declares keeps its
declarations from the other lib files (`Intl` in lib.es2020.intl.d.ts has
lib.es5.d.ts's too), and `--include Intl` adds a lib namespace to any input.

## Generators

`--emit c|cpp|zig|rust|fsharp --out <dir>` writes bindings for the lowered
XML, and for wasm targets the JavaScript glue; `--glue-for <module.wasm>`
trims the glue to what a compiled module imports, and `--glue-out <file>`
writes only that glue (an echarts demo's is 0.1 MB, not 5.9). The generators
(`src/generators/`) are thin and return errors: everything a config left in
that a target cannot express, each kind once, with how often, where first,
and the setting that lowers it.

References: each type declaration has an `id` (its path, `Konva.Shape`),
and each reference to one a `to`, found by the checker's symbol, never by
name. A reference without `to` is a type declared elsewhere, even where
one declared here has its name: GeoJSON's `Point` beside leaflet's, the
DOM's `Node` beside prosemirror's. Generators look references up by `to`;
F# puts such a type in a module `External`, the model numbers it apart.

Names: lowering makes declarations' names apart in the XML; each target
then spells them its own way (`foo_bar` and `fooBar` are one `foo_bar` in
snake case; C's `Shape_area` is a method and could be a function too).
`names.rs` renames the model until every identifier a target makes is unique
in its scope, given the target's naming: how it spells types and functions,
what else a type's name makes (C's `AS_Shape`, Gleam's `shape_as_named`),
whether types and functions share a namespace, and whether functions
overload (C++). What the library declares claims names first, a top-level
declaration before a namespace's (`StreamPipeOptions` before a flattened
`Stream.PipeOptions`), then functions, then what lowering made up, which
takes the `_2`. Within a type its methods come first, then its statics, then
its properties' accessors: Rapier's `setTranslation(x, y, z)` keeps its name,
and the setter of `translation` is `setTranslation2`; pts' `form.line(pts)`
is `line`, and the static `CanvasForm.line(ctx, pts)` `line_2`.

Documentation: the model carries each function's, class's and enum's (and its
members') `<doc>`, and each generator writes it once, at the definition, in its
target's comment form: `//` in C, `///` in C++, Zig, Rust, Gleam, F#. A
member has its first paragraph and tags, a type all of it. What could break
the code is taken out: a backslash ending a C or C++ line, Rust fences made
`text` and indented blocks unindented (rustdoc would run them as tests),
F#'s markup escaped (`<summary>`, `<param>`). `@deprecated` is a line, not an
attribute: a use of it must not warn. Where lowering renamed a member or
fixed an argument and the name does not say so, the documentation ends with
`JavaScript: open(map, content)`. In Zig, a parameter of `anytype` that is a
union, a function or an option struct's fields also gets a line saying what
it takes: the one documentation the library did not write.

A namespace's types are its namespace's, as each target has namespaces:
F#'s nested modules (`Stream.PipeOptions`); Rust's modules
(`stream::PipeOptions`, defined in `src/stream.rs`, their member traits in
scope with the crate's); Zig's structs in `bindings.zig`
(`bindings.stream.PipeOptions`); C++'s aliases of the flat definitions in a
namespace (`NodeJS::ArrayBufferView`), or in the class the namespace shares
its name with (`Stream::Duplex`); Gleam's aliases in the namespace's module
(`stream.PipeOptions`), each class's functions in a module of its own name
(`bindings/namespaces/stream/readable`). C has none: `Stream_PipeOptions`.
Where a function or a generic class has the namespace's name, its
namespace is `<name>_ns`.

With several packages bound together, each is a module of its declarations
as a namespace is: `pb::Node` beside `pa::Node` (F#'s `Pb.Node`, a module of
each package; Gleam's `bindings/areas/pb`, its namespaces in it),
`prosemirror_model::Node`,
`fs::promises::FileHandle` for `fs/promises`, a scope's `@` part left out;
a namespace of its package's name (node's `stream` `Stream`, `path`) is that
module. A declaration's package is the ambient module it is declared in
(`declare module "stream"`), bound or not, as JavaScript imports it from
there; in `declare global`, or an ambient-module package's file outside any
module, none. The XML's `packages` says which were asked for: a program
must give their modules, and others only where it uses them; a type-only
module (`stream/iter`) is imported by nothing. A declared
type that must give way (to Zig's `Key` of a map's struct, Gleam's `Bool`)
is named by where it comes from, before a number: its ambient module or
file, `origin` in the XML where there are several: node's `readline` `Key`
is `ReadlineKey`, three's `OperatorNode` `Bool` `OperatorNodeBool`. Less
the words the name has already: `stream/web`'s `StreamPipeOptions` is
`WebStreamPipeOptions`; where the name has them all (`AddDaysOptions` of
addDays.d.ts), the origin adds nothing and a number is taken. The model
names each XML path apart first (`cluster.Worker` beside a class
`cluster_Worker`), a top-level declaration before a namespace's. The
`clashes` fixture runs each case in every language.

C, C++, Zig, Rust and Gleam write from one binding model (`model.rs`):
functions for properties, option bags and overloads, each saying how
JavaScript is called. `wasm.rs` is the wasm boundary and its glue; `js.rs`
the JavaScript that reaches the API, which Gleam's FFI uses too. F# reads
the XML itself: it has methods, overloads and subtyping.

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
- `modules`: with several packages bound together, each one's module by its
  name (`{ "prosemirror-model": model, "prosemirror-state": state }`).
  A package's names are looked up only in its module, never as globals (a
  missing `Node` is not the DOM's), and makeEnv throws if one is not given.
- `check`: handle mistakes are reported: `handle 7 used after its release`,
  `released twice`. Off by default: slots are reused.
- `unchecked(e)`: called with an exception nobody took (default `console.error`).

JavaScript's built-ins have types of their own in every wasm target:
`ArrayBuffer`, `SharedArrayBuffer`, `DataView`, the typed arrays and views
copy to and from linear memory (`ArrayBuffer_from(bytes, n)`,
`Uint8Array::to_vec()`); `Error` has its `name`, `message` and `stack`, and
can be made (`Error::new`); `Date` its time and ISO string. Element types
are the shared model's (`Elem::U8`), named by each generator.

A call that throws returns a zero value and keeps the exception to take
(`js_take_error`, `ts::attempt`, `jsTry`, `js::attempt`); one not taken by
the time control returns to JavaScript is reported, not lost.

- C (C11): `ts_types.h` with every handle, enum, union and callback; one
  header per declaration in `areas/<module>/` or `namespaces/<module>/`.
  Inheritance is kept: `_Generic` upcasts (`AS_Element(x)`) let any subtype
  be passed, and a type names its ancestors' functions too
  (`HTMLElement_appendChild`), but a family of more than 150, one per literal
  (`ParentNode_querySelector_div`), only where it takes it directly
  (`Document_querySelector_canvas`). A function's name is also a macro, so trailing optional
  arguments may be left out (`Event_new("ping")`; all of them, `orUnit()`,
  in C23), `x?: T | null` as `x?: T` (`node_writeFileSync(path, text)`), and the first of a family of overloads picks one by its
  argument's type (`Shape_label(s, "a")`, `Shape_label(s, 7)`). Each shorter
  call is one line that makes the next longer one, and is written once
  where several overloads share it. Optional values are `JsOptionalDouble`,
  `JsOptionalBool`, null handles and `E_absent`; unions tagged structs made
  from a member (`Value_of(widget)`, `Value_of_number(42)`); arrays and
  tuples typed (`JsArray_double`, `JsTuple_double_JsString`). A union
  parameter, a setter, an array's `push` and a family's overload take the
  union or any member whose C type tells it apart (`leaflet_marker(ll, opts)`,
  `Popup_setContent(note, "text")`): the macro applies `U_of`, and a value
  that is no member is `ts_no_member`. A subtype of a member is upcast first (where the member has at most 40).
  An object an interface only holds properties of is made in one expression,
  `X_create(.a = 1, .name = "x")`: a struct of the settable properties, own
  and inherited, and an object with those named set, and no others; `false`,
  0 and "" are values. What is absent is a signed byte of -1 for a flag, one
  NaN for a number, NULL, a null handle, `tag == -1`; the macro lists those
  first and silences the compiler's warning of the designators that override
  them. `X_create()` and `(X_create)()` are the empty object, made by the
  function. A union's property is a field for each member, the primary one
  by its name (as Gleam's: `.left = 16`, `.fill = "#111"`), the others with
  the member's (`.left_string = "10%"`, `.fill_canvas_gradient`,
  `.title_list`, `.xAxis_category`, `.series_bar`): no tagged union to make,
  and no `{ .number = 0 }` a `{ .boolean = false }` would equal once C11
  zeroes the rest. A bag nests: `.icon_div = divIcon(DivIconOptions_create(.className = "pin"))`; of objects only, the one named by what they all share is plain (`.icon` beside `.icon_div`).
  A property of a type with no C value is a `JsValue` field, a property
  set in more than one way is left out, and a property named as a C keyword
  is a field with an underscore (`.inline_`). A long list of properties none
  optional (`ElementTagNameMap`) is a map, not a bag. A union only a value goes into, because no test tells its
  members apart, is typed for parameters and setters; what returns it is a
  `JsValue`. Handles are released one by one or by scope
  (`js_scope_open()`/`js_scope_close()`), or by `js_scope();`, which
  closes where its block is left, on any path.
  The DOM: 21 MB of headers.
- C++ (C++20, WASI SDK): headers include each other by their path from the
  bindings' root, so they are compiled with `-I` on it. A value type per JavaScript type, inheriting
  virtually from `ts::Handle`; copies retain, destructors release. `new X(a, b)`
  is `X(a, b)` as well as `X::create(a, b)` (a class template's arguments its
  defaults: `Mesh(geometry, material)`). `X(nullptr)` is the null handle,
  and `X()` too, but for a module's class JavaScript makes of nothing
  (`new Scene()`): that is `Scene::create()`, and `Scene()`, a member left
  unset or `std::make_shared` of a struct holding one do not compile, where
  they would be null. lib.dom's globals keep `X()`: it declares `new()` on
  nearly all of them, most illegal. Overloads,
  `std::optional`, `std::variant`, `ts::Array<T>`, `ts::Tuple<...>`, lambdas
  (`release()`, `ts::Scoped`), class templates with defaults and
  `requires`-clauses (a default its constraint does not take by name drops
  the constraint: three's `Points<>`; a method's type parameter no argument
  deduces is its constraint, `intersectObjects(objects)`), keyed methods over typed keys, `co_await` on promises
  (`ts::Task`), range-for over iterables and over subclasses of Array.
  Constraints are checked of types that are only declared: each type's bases
  are declared beside it (`ts::bases<X>`) and `ts::extends` walks them.
  `X.hpp` includes what its members name; non-template definitions are one
  `.cpp` per module, compiled once. An object a caller fills in (no methods)
  is made in one expression from the properties it names:
  `MapOptions({.attributionControl = false, .zoomAnimation = false})`, or a
  braced list where a `MapOptions` goes (`leaflet::map("map", {{.zoomSnap = 1}})`).
  `X::Fields` has a `ts::Field<T>` for each property that can be set, its
  ancestors' first, each in the order declared: designators follow that order
  (`.top` before `.left`, which the library declares so). What is not named is
  not set at all; `false`, `0` and `""` are set. A property declared again is
  where it is declared last, one set in more than one way is left out, and a
  long list of properties none optional (`ElementTagNameMap`) is a map, not a
  bag. A nested bag is made as `Inner({...})`; a tuple or an array field takes a braced list (`.at = {1, 2, 3}`). A union no test tells apart is
  typed where a value goes in, in an array too (`ts::Array<Loose>`), and
  what comes back is a `ts::Value`; a `ts::Array<>` takes any array. A
  variable initialized from a template's value needs no argument list
  (`Polyline line = leaflet::polyline(..)`). The DOM: 6 MB of headers and 5 MB of `.cpp`.
- Zig: a file per module and a root `bindings.zig`; Zig reads only what is
  used. `?T`, tagged unions, `JsArrayOf(T)`, `JsTuple(&.{...})`; callbacks
  are comptime functions with Zig types, behind a C-convention trampoline;
  `defer jsRelease(x)` or `JsScope`. Trailing optional parameters are one
  struct whose fields default to null (`s.resize(2.6, .{})`,
  `s.resize(2.6, .{ .round = true })`); a lone one stays a `?T`. A number
  parameter takes any integer or float (`shape.setX(i * 40)`, `s.resize(n, .{})`),
  and nothing else; a number a call returns is an `f64`. A method that returns
  its receiver (`addTo(map): this`) returns it, so it chains
  (`marker.addTo(m).bindPopup(p)`) or is discarded (`_ = marker.addTo(m);`). A union
  whose members no test tells apart is still a tagged union where a value goes in
  (`loose(.{ .A = a })`, or a `JsValue`) and a `JsValue` where one comes out. Names are
  Zig's, camelCase: `getTextContent()`, `setTextContent(..)`,
  `addEventListenerWithString`. Inheritance is flattened, and an
  inherited member is written once: the furthest ancestor that declares it
  writes it in `X.Methods(Self)`, and each subtype's member is an alias of it, named once per type (`M_Element.x`). The DOM: 13 MB.
- Rust (wasm32-unknown-unknown): a crate (`Cargo.toml`, `src/`), compiled
  once: a DOM program then builds in under a second. Every crate shares the
  runtime's, `ts_runtime`, written beside it: one `JsValue` for the DOM and a
  library, and one `ts_drop_closure` in a program. A handle type per
  JavaScript type, `Clone` retaining and `Drop` releasing it. Each type's
  members are a trait (`ElementMethods`) implemented for it and its subtypes,
  and its own inherent methods; a type nothing extends has only those;
  where two traits share a name, an inherent method says which. Names are
  Rust's: `get_element_by_id`, `text_content()` and `set_text_content(..)`,
  `ShapeKind::Square`, `HTMLElementEventMap::CLICK`. Arguments are borrowed:
  an object is `&impl AsRef<T>`, so any subtype goes in; a union or an array
  is `impl Into<..>`, so `describe(42.0)` and `join("-", ["a", "b"])` work, and
  so does a union whose members no test tells apart (`loose(&a)`), which only
  a value goes into and a getter gives as a `JsValue`; a parameter of `any`,
  or of a union nothing types (Pixi's `fill`, of colors another package
  declares), is `impl Into<JsValue>`: `&handle`, `4.0`, `"text"`, and `None`
  where it is optional; a value of a type parameter
  goes by value. A function with trailing optional arguments is two: one
  without them (`lat_lng(46.2, 6.1)`, `World::new(&gravity)`) that calls the
  one with them, `_with` (`map_with("map", &opts)`; `_full` where a member is
  named so). There an optional or nullable argument is its value or `None`: a
  string is `"x"`, `&text` or `None`, an object `&opts`, `Some(&opts)` or
  `None`, a number or flag `500.0`, `Some(500.0)` or `None`. There is no
  shorter form before a rest, or where what it leaves out names the
  function's type parameter.
  Overloads that differ in a
  key-or-string parameter are one method (`query_selector(HTMLElementTagNameMap::P)`,
  `query_selector("p.intro")`), and where they differ in other parameters too,
  the key fixes their types (`add_event_listener(HTMLElementEventMap::CLICK,
  &listener, None)`, `add_event_listener("ping", on_ping, &options)`). A
  nullable union argument is any member, the union or `None`. A kept defaulted type argument is stated with
  `name_as::<T>`; a constructor of a generic class takes the
  class's parameters it does not name at their defaults, as `HashMap::new`
  (`Mesh::new_with(&geometry, &material)`, `Application::new()`), and a
  shorter form those its arguments left out alone named (date-fns'
  `add_days(date, 3.0)`, of `ResultDate = DateType`). An option bag's
  `create()` and `default()` stay generic: where it goes says what it is. `T extends Node` is `T: Is<Node>`. Callbacks are
  closures, kept (`new`, `release`, or freed once JavaScript collects the
  function) or for the length of a call and free to borrow (`scoped`); promises are awaited under `js::spawn`. The DOM: 12 MB.
- Gleam (JavaScript target): no wasm and no glue, as F#; from the shared
  model, as the wasm targets, since Gleam has no methods, subtypes, overloads
  or optional arguments. A function with trailing optional arguments is two, one
  export: `create_element(doc, tag)` without them, `create_element_with(doc,
  tag, options)` with them, bare (`_full` where a member is named `_with`); a
  callback or an object of methods that ends a tail of two stays an `Option`
  (`world.step_with(w, q, None)`), and so does a type nothing in the bindings
  makes or a call gives back (pts' `CanvasLineJoin` without the DOM's bindings).
  More than four are an option bag of them,
  folded into the class's module: `sphere_geometry.new_with(
  sphere_geometry.args_create() |> sphere_geometry.args_with_radius(0.36))`.
  Overloads that differ only in their optional tail are one plain function,
  `name_with` the one ending in an options object, and `name_with_<first>` each
  other (`set_option(chart, opt)`, `set_option_with(chart, opt, opts)`,
  `set_option_with_not_merge(chart, opt, False, False)`).
  A union going in is one function per member. The plain name takes its
  primary member: of `T | T[]` the one, a string beside objects, a record
  beside other objects, of objects only the one named by what they all share
  (Leaflet's `LatLng`), else a lone array of other things (Konva's `points`), or
  none, else the first as declared; an inline union's
  number before its string, the checker having put string first
  (`with_left(16.0)`, `with_font_size(13.0)`, `add_days(date, 3.0)`,
  `schema.node("heading")`). Each other member is
  `name_<member>`: `with_left_string("10%")`, `with_title_list([..])`,
  `with_color_fn(f)`; an option object by the `type` its `create` fixes
  (`with_x_axis_category`), else by its name less what the members share
  (`with_series_bar`); a nullable one's null `name_null`. One in the optional
  tail is in `name_<member>_with` alone. `name_of` takes the unions as they
  are where a function has several, two of one, or a union of objects comes
  back (`difference_in_days(later, earlier)` takes Dates,
  `difference_in_days_of` `DateArg`s). Bindings grow 1-20%, pts' 43%: its
  point union is in nearly every function, often twice. Subtypes are phantom types along each class's chain
  of first bases: `Node(a)` is opaque, `Element(a) = Node(ElementTag(a))`,
  so an `Element(Nil)` is a `Node` to any function that takes a `Node(a)`,
  and only an Element. A function's parameters are open (`Node(a)`, any
  subtype); what comes back, what a callback is given, what a list or
  variant holds, are closed (`Node(Nil)`). Members are their declaring
  type's, in its module: `named.get_name(shape)`; a subtype's module names
  them too, as constants of the ancestor's (`pub const add_to = m_leaflet_layer.add_to`),
  but for the DOM, whose would be too many. A type parameter bound to a class is that
  class, open: `append_child(self: Node(a), node: Node(t)) -> Node(t)`.
  Members of bases off the chain (mixins) are the class's own, with an
  upcast to the base; `node.upcast(x)` closes a type at Node, for a list of
  several kinds; a class at runtime has checked downcasts,
  `html_input_element.from(target)` and `.cast(dynamic)`. An interface has
  none: nothing at runtime says what it is. Where a base's arguments name the
  class itself (`BitmapFont extends AbstractBitmapFont<BitmapFont>`), they are
  `Dynamic`: an alias may not be defined in terms of itself. A subclass of
  Array has `to_list`, `get_at` and `length`. The DOM is 61,823
  functions; flattened, it was 300,398.
  Gleam modules may not import each other in a cycle, as the API's areas
  would (`Element.ownerDocument`, `Document.body`). Every type is in
  `bindings/types`, which imports only `bindings/runtime` (`Promise`,
  `JsError`, `Codec`); an area's functions are in `bindings/<area>`, a
  class's in `bindings/<area>/<class>` (`shape.area(s)`), which import
  types only. A name JavaScript reserves is `new_` in the FFI. Each has an FFI module that calls the API and
  converts what Gleam holds otherwise:
  an `Option(a)` for a nullable value or optional argument, a `List(a)` for an
  array, custom types for unions and string enums (a union no test tells
  apart is one where a value goes in, a parameter or a setter's, and
  `Dynamic` where one comes out; a variant holding a class is made of any
  subtype, `scene_entity_camera(cam)`), a Gleam function for a
  callback (the same JavaScript function each time), `Promise(a)` to
  `promise_then`, `attempt` for an exception. Generics are Gleam's, without
  constraints. Gleam's values of a type parameter are not JavaScript's when
  they convert (a `Box(Mode)` holds a string, Gleam a variant), and Gleam
  erases types: so a function whose type parameter crosses takes a codec for
  it, `box_get_value(b, mode_codec())`, which the type checker ties to the
  `t` of `Box(t)`. Each type has one (`shape_codec()`), with `codec_float()`,
  `codec_option(c)`, `codec_list(c)`; a handle crosses as it is (`Box(t)` as
  an argument needs none). `to_fixed(x, 2)` in the runtime is JavaScript's `toFixed`. A `$` method is `add_new`. The DOM runs in
  Chrome, as the other targets' does.
- F#: files must not refer to each other in cycles: they are ordered by what
  F# cannot refer to forward, and a member that mentions a later file's type
  becomes an extension there. Iterables are `seq<T>`, async iterables
  `JsAsyncIterator<'T>`, a subclass of Array a `seq<T>` with an indexer;
  constraints of methods, functions and interfaces are F# constraints, kept
  through keyed values (`getElementsByTagName<'K when 'K :> Element>`). An
  index signature is `JsRecord<string, T>`. In an erased union a function of
  two or more arguments is a `System.Func`: Fable keeps a function value
  curried (TanStack's `AccessorFn<'TData> | string`). `f(a, b?, ...rest)` is two
  overloads. A union argument is an overload of each member too, a generic
  alias's with its arguments put in (date-fns' `DateArg`: `addDays (date,
  3.0)`, no `!^`), and a function's kept type parameter no required argument
  names, defaulted to one that does, is that one: the result follows the
  argument. A library that is a module is imported (`[<Import("Pt",
  "pts")>]`): values, functions and classes, and a union case of a class is
  tested by `instanceof` on the import. The built-ins Fable has are its
  types (`JS.Uint8Array`, `JS.DataView`, `JS.Date`); an `Error` is an `exn`,
  as Fable catches it, and `jsError "..."` makes one JavaScript recognises:
  an F# exception is not an `Error` there. Overloads F# calls alike are
  named by their marks (see Overload names); else `name_2`. An abbreviation shows the type
  parameters its F# type does, the others dropped where it is named. A
  generic base reached twice at other arguments is inherited once, and its
  members declared again at the arguments given (F# allows one
  instantiation). Unions past `U9` nest: `U9<A, ..., H, U3<I, J, K>>`.

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

`instanceof` is given for classes with a global constructor, `array` for
arrays and tuples (an alias of one too), `has:` for the first required
property no other member has, never one every object has (`toString`), `tag:`
when one required property has a distinct literal value in every member. F#
turns these into `[<Erase>]` unions with `CaseTest`s or
`[<TypeScriptTaggedUnion>]`, named however many members no test tells apart:
any member converts to it, and a case without a test of its own matches by
Fable's guess. The wasm glue makes a tag function, so Zig can `switch` on a
union a call returns.
A library's alias of a named union (`type Wrapped = Content`) is declared as
the target's own alias (`typedef`, `using`, `pub const`, `pub type`, an F#
abbreviation). `T | T[]` of a union `T` is flat in the model, its array still
of `T` (ECharts' `BarSeriesOption | .. | SeriesOption[]`). A `symbol` is a
handle of its own, `JsSymbol` in every target, not `any`: `string | symbol`
stays a union (node's event names), told by `typeof`.

## Overload names

Overloads with fixed values (from `keyed_params` or literal parameters) are a
family named after them: `addEventListener_click`,
`addEventListener_click_with_options`. The lowering marks each overload with
what a target may lose of it (`marks` in the XML): its type parameters'
constraints (`Dictionary`), the literals of its options (`async` for
`{async: true}`, a string its value; `false` is the default). Within a
family the first without marks of its own keeps the name; the others are
named by their marks (`marked_async`, `merge_Dictionary`, a call signature's
`invoke_async`), else by the parameters they add (only the last, when they
extend it), else by the types that differ among same-named overloads, else
the one type that differs, else a number. F# and C++ keep overloads, so
name them only where they collide: F# by the same marks (`markedAsync`), C++
not at all, a `requires` clause choosing. An overload whose parameters and
constraints repeat an earlier one's is dropped: TypeScript would never
choose it. An overload that only makes a parameter optional is named by it.
The plain name goes to an overload with no fixed value where there is one
(`on(type: string, fn)`), not to one literal; and of those, to one called
with fewer of the first's arguments that returns the same: `createEngine(canvas)`
before `createEngine(canvas, options)`.

## Generated names

An inline type is named by where it first is, by its shortest path
(`Renderer_getAnimationLoop_result`), keeping the owner: a name stays put
when a library adds an unrelated one. A callback used in several places is
named by what it takes and gives, if that is short and free
(`HTMLElement_MouseEvent_fn`, `Token_to_boolean_fn`). A union by its members
when a few short ones say it (`string_or_number`, member order aside), else
by its place; a place another union has, with what else it holds
(`FileHandle_readFile_options_or_string`), or `_union`. A declared alias keeps
its name where it is written, exported or not, inherited members included,
and for what a pass expanded to its structure. A union the library wrote as
a union (`string | HTMLElement` beside `type Content = string | HTMLElement`)
is named by its members where that applies: two declarations of one
structure, each place showing the name the library wrote there; where no
short name applies it has the alias's. A name made for an inherited member
is of the type that declares it (`Evented_addEventListener_type_click`),
flattened or not.

Inline types one place would name (overloads' options, a union's members)
are named apart: the first keeps the place's name, each other is named by
how it differs: its tag (`TrustContext_href`), a value it fixes
(`lstatSync_options_bigint_true`), a property only it has, has otherwise or
makes optional (`ColumnHelper_accessor_column_opt_id`), a parameter or result
(`stat_callback_opt_BigIntStats`), what it is (`ExpandedState_map`). Never
by a member of the first: in C `{Type}_{member}` is the first's own. Enums
of strings by their first value (`Evented_addEventListener_type_click`).
Names too long are cut in the middle; a number is the last resort.

Bundled declarations take their exported names: rollup's
`BarSeriesOption$1`, exported as `BarSeriesOption`, swaps names with the
local interface; an unused `$N` is pruned, a used one drops `$N` if free.
Its namespace of locals (`namespace color_d { export { color_d_parse as
parse } }`, exported as `color`) is `color`, with `parse` in it, reached as
`echarts.color.parse`: what a module exports itself stays its own (node's
`finished`, also `finished.__promisify__`). At global scope, C++ keeps such a
namespace clear of the C library (`time_ns`).
F# delegates are named by their place (`MkdirCallback`), then their types.

## Provenance

Lowering never silently loses the link to the source:

- `jsName` on renamed members: the real JavaScript name.
- `from` on inherited members: the declaring interface.
- `const` on widened literal properties: the value.
- `synthetic="true"` on declarations a pass created.
- `lossy="reason"` on every `any` a pass introduced.
- `<doc>` on a declaration or member: its JSDoc, read from the `/** */` block
  before it in the source text. The
  description, then a line each for `@param`, `@returns`, `@deprecated`,
  `@default`, `@see`; links as their text, other tags dropped. A copy a pass
  makes (flatten, overloads, accessors) keeps it; a declaration a pass made up
  has none.

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
iteration, and once made a call cost 5 ns in both. In the `be called back`
row, TypeScript's 6.0 ns is slightly slower than wasm's 5.0 ns only because all
six programs share one `each` helper whose call site sees six different
callbacks; alone, TypeScript takes about 4 ns, and adding 0.5 changed nothing,
so it is not the small-integer boxing effect.) A wasm target
pays 5 to 8 ns to cross the boundary, a handle-table slot per object, and a
decode or encode per string (a string at the same address with the same
bytes is decoded once). Code that stays inside wasm is as fast or faster.
So: bind coarse operations, not inner loops; keep strings out of hot paths.

## Tests

`cargo test` runs `tests/e2e.rs`: 65 tests in about 30 seconds, or 10 with
`-- --skip dom::`. Targets: F# (Fable), Gleam, C and Zig (wasm32), C++ (WASI
SDK), Rust (`rustup target add wasm32-unknown-unknown`); `mise.toml` pins the
versions. A missing toolchain or
Chrome skips its tests with a message.

**Fixtures in Node.** For each of `shapes`, `callbacks`, `data`, `globals`,
`unions`, `pts`, `exports` and `builtins` in `tests/fixtures/`, and each target
(Gleam's in a project from `tests/gleam`, with a `cell` module for what callbacks did): lower `api.d.ts`
with `examples/targets/<target>.toml`, generate bindings, compile them with
the real toolchain, and run `test.*` against `impl.mjs` in Node, with handle
checks on. A program may export `finish`, called once asynchronous work has
settled.

**A real library.** `pts` is Pts 1.0.3 (Apache-2.0), vendored unchanged: its
`index.d.ts` and its module. Every target compiles all of its bindings and
runs the same program: constructors with rest arguments, accessors, methods
that return `this`, statics, unions as arguments, a subclass of Array
indexed and iterated.

**A module across files.** `exports` is a package of three files: a default
export, functions and classes exported and re-exported under other names, a
constant and a variable, and a union told apart by the classes as exported.
`builtins` passes buffers, typed arrays, errors and dates both ways.

**Real libraries.** `tests/libraries/package.json` pins katex, marked,
date-fns, node (`fs`, `path`, `events`), leaflet, pixi.js, ECharts, konva,
three.js, rapier, TanStack Table and prosemirror (four packages bound
together); `tests/libraries/install.sh` installs them into
`target/libraries`. For each library and target, its declarations lower and
follow the schema, and its bindings compile. Slow, so ignored by default:
`cargo test --test e2e libraries -- --ignored`.

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
  `ArrayBufferLike | ArrayBufferView`): bound as untyped values, in C and
  Rust typed where a value goes in. A union with an `any` member is `any`.
- F# matches a union's cases no test tells apart (Pts' 7, Pixi's fills) by
  Fable's guess. Namespaced classes of a module library are still tested
  through `globalThis`.
- F# has no `SharedArrayBuffer` or `BigUint64Array` of Fable's: they are
  opaque. A built-in as a base type stays opaque too (`Pt` extends an opaque
  `Float32Array`): an F# interface cannot inherit Fable's.
- C: an empty call (`orUnit()`) is C23; under C11 `-pedantic` warns of it. A
  compound literal with a comma needs parentheses to pass through a macro:
  unions have `U_of` so it is seldom written. `_Generic` takes exact types,
  so a subtype is upcast first.
- Rust: a nullable union argument takes a member or `None`, not `Some(member)`; a
  nullable string takes `Some(&str)`, not `Some(&String)`, and an `any` takes
  `Some(&JsValue)`, not `Some(&handle)`: a bare `None` must infer, which one
  accepted type for `Some` allows. A type argument nothing can infer is no
  longer written (`em.on::<JsValue>(..)` is `em.on(..)`).
- The wasm glue takes a module library as `options.root`; it does not read
  `import` yet.
- Rust and Zig callbacks with a union argument receive it decoded; C's
  receive a handle.
- Gleam: no async iteration; builtins are opaque.
- All twelve real libraries compile in every target (72 of 72). A type
  that expands without end converts to a budget, then `any`.
- Strategies are strings, so more can be added (`generics = "monomorphize"`).
