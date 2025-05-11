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

## The Plan 

Use oxc.rs to convert the following projects into XML so that they may be consumed by other languages:

* @types/web https://github.com/microsoft/TypeScript-DOM-Lib-Generator: TypeScripts definitions for the Web API. For testing .d.ts files 
* Pts https://github.com/williamngan/pts: A relatively small drawing library. For testing .ts files 

Use the output and attempt to produce F# bindings or Rust bindings from them. 


## Notes
If things are going well, it would be interesting to go further and test some server side APIs, such as Deno and Playwright.
