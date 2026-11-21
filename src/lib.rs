pub mod config;
pub mod front;
// Generators: the lowered XML to bindings for C, C++, Zig (wasm32) and F# (Fable).
pub mod generators;
pub mod ir;
pub mod lower;
pub mod xml;

pub(crate) fn fnv1a64(s: &str) -> u64 {
    s.bytes().fold(0xcbf29ce484222325u64, |h, b| (h ^ u64::from(b)).wrapping_mul(0x100000001b3))
}
