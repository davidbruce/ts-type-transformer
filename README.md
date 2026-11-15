# TypeScript Type Transformer
> [!CAUTION]  
> I am new to Rust and this is not in a usable state!

The goal of this project is to experiment with converting TypeScript type definitions to a format that can be parsed by other languages easier than .d.ts, such as XML. Alternatively, it may be used to evaluate the effort of emitting JavaScript wrapper code for other languages directly from TypeScript. 

## The Problem

Many languages support compiling to JavaScript, but consistently run into the problem of wrapping its APIs.
TypeScript already does the leg work of adding types to JavaScript, making it a natural target of these other languages.
Projects consistently popup to consume .d.ts files or the TypeScript AST. Many of these projects start off very promising,
but in the end they generally fall off due to the complexities of TypeScripts type system. 

Examples:
* F#: https://github.com/fable-compiler/ts2fable
* F#: https://github.com/glutinum-org/cli
* Ocaml and ReScript: https://github.com/ocsigen/ts2ocaml
* ReScript/ReasonML: https://github.com/jsiebern/re-typescript 
* Nim: https://github.com/mcclure/dts2nim
* Haxe: https://github.com/haxiomic/dts2hx
* Gleam: One will probably show up soon

## Usage

`tsrs_proto` lowers a `.d.ts` entry file, and the files it imports, to XML by a target's config, and can emit bindings for F#, C, C++, Zig or Rust with their JavaScript glue:

```sh
cargo run --bin tsrs_proto -- path/to/index.d.ts --config examples/targets/rust.toml
cargo run --bin tsrs_proto -- path/to/index.d.ts --config examples/targets/zig.toml --emit zig --out bindings
cargo run --bin tsrs_proto -- lib.dom.d.ts --config examples/targets/c.toml --config examples/modules/dom.toml
```

A bare `lib.*.d.ts` name is one of TypeScript's own lib files. `--lib es2020,dom` chooses the lib files a package is checked with, and `--include Intl` adds a lib namespace to the output. Target configs are in `examples/targets/`, the XML schema in `schema/`, and the design, benchmarks and tests are in [DESIGN.md](DESIGN.md).

## Tests

`cargo test` compiles the generated bindings with each language's toolchain and runs them in Node, and the DOM bindings in headless Chrome. A missing toolchain (`fable`, `clang`, `zig`, the WASI SDK, `rustc` with `wasm32-unknown-unknown`, `node`, Chrome, `xmllint`) skips its tests.

## Notes
If things are going well, it would be interesting to go further and test some server side APIs, such as Deno and Playwright.
