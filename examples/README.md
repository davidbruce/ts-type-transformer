# `tsrs_proto` Multi-Target Examples Showcase

This directory contains **438 runnable demos** (**73 demos × 6 target languages** across **14 npm & Node libraries**: 68 interactive/rendered browser demos and 5 Node CLI demos):

- **WebAssembly targets**: [C (`c/`)](c/), [C++20 (`cpp/`)](cpp/), [Zig (`zig/`)](zig/), [Rust (`rust/`)](rust/)
- **Native JavaScript targets**: [Gleam (`gleam/`)](gleam/), [F# via Fable (`fsharp/`)](fsharp/)

Libraries covered: `pts`, `@babylonjs/lite`, `katex`, `marked`, `date-fns`, `echarts`, `konva`, `pixi.js`, `leaflet`, `@tanstack/table-core`, `prosemirror-*`, `three`, `@dimforge/rapier3d-compat` (both 2D/3D and coupled `babylon_lite` 3D scenes), and Node built-ins (`fs`, `path`, `events`).

## Quickstart

From the repository root:

```sh
# 1. Build the binding generator
cargo build --bin tsrs_proto

# 2. Install JS/TypeScript dependencies for the showcase (npm workspaces)
cd examples
npm install

# 3. Launch the interactive showcase dev server (compiles demos lazily on demand)
npm run dev
```

The unified gallery at [examples/index.html](index.html) lets you switch between all 6 languages on the same demo, inspect live build status (`/__api/status`), and view syntax-highlighted source (`code_viewer.mjs`) across the target entrypoint, `main.mjs` loader, and `index.html`.

Each target also exposes its own standalone gallery (`examples/<target>/index.html` powered by [target_gallery.mjs](target_gallery.mjs)).

## Pre-building & CLI Usage

All commands below run from `examples/` (or pass filters from a target's own `build.mjs`):

```sh
npm run status                  # Show compiled / total counts for all 6 targets
npm run precompile              # Generate bindings and build all 438 demos
npm run precompile:browser      # Pre-build all browser demos (skip Node CLI demos)
npm run precompile:rust         # Build only one target (c, cpp, zig, rust, gleam, fsharp)
node build.mjs c:pts zig:konva  # Build specific package or demo slices
node build.mjs --force rust     # Force regeneration of bindings + rebuild
```

To run a Node CLI demo after building it:

```sh
cd examples/rust
node ./node/event_bus/main.mjs
```

## Toolchains & Layout

- **Toolchains**: Versions used for Node, Rust, Zig, Gleam, and .NET/Fable are pinned in [mise.toml](../mise.toml), alongside `clang` / WASI SDK (`~/.wasi-sdk`) for C/C++ Wasm builds.
- **Target configs**: Lowering/emission configs live in [targets/](targets/) and [modules/](modules/), orchestrated by [targets.mjs](targets.mjs) and [build.mjs](build.mjs).
- **Bindings**: Running any build generates bindings under `examples/<target>/bindings/` (ignored by git). A wasm demo loads only the glue its module imports, written to `glue/` beside its `app.wasm` (`--glue-for`, `--glue-out`).
- **Standalone Rust Wasm builds**: [rust/.cargo/config.toml](rust/.cargo/config.toml) configures `--export-table` for `wasm32-unknown-unknown` so standalone `cargo build --target wasm32-unknown-unknown` works alongside `node examples/rust/build.mjs`.
