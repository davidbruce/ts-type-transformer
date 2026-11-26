// XML -> a Rust crate of bindings for wasm32-unknown-unknown.
//
// Every JavaScript object is a value holding a counted handle: `Clone`
// retains it and `Drop` releases it. Inheritance is traits: each type's own
// members are default methods of `XMethods`, implemented for the type and
// each of its subtypes, so `div.set_attribute(..)` works on an
// HTMLDivElement. Where two of a type's traits share a name, the type has an
// inherent method of that name, which Rust prefers.
//
// Names are Rust's: methods and functions in snake_case, a property `x` as
// `x()` and `set_x(..)`, generated type names and enum variants in
// CamelCase, a map's keys as constants of the map
// (`HTMLElementEventMap::CLICK`). Arguments are borrowed: an object is
// `&impl AsRef<T>`, so any subtype goes in; an array or a union is
// `impl Into<..>`, so `["a", "b"]` or `42.0` do; `any` is `impl Into<JsValue>`;
// a nullable or optional string or object is `impl Nullable<T>`, the value by
// reference, `Some` of it or `None`, and a number or flag `impl Into<Option<..>>`;
// a value of a type parameter goes by value. Unions are enums, callbacks
// closures, promises futures.
//
// The crate: `Cargo.toml`, `src/lib.rs`, `src/runtime.rs`, `src/types.rs`
// (enums, unions, callbacks), a file per module, and `src/raw.rs` (the
// imports).

use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::Path;

use super::model::{Func, Model};
use super::names::{Kind, Naming, identifiers};

mod classes;
mod emit;
mod types;

#[derive(Clone, Debug)]
pub struct ProjectRef {
    pub crate_name: String,
    pub dep_path: String,
    /// Exported top-level types: Rust name -> `(arity, is_handle)`.
    pub types: HashMap<String, (usize, bool)>,
}

/// Loads a previously generated Rust binding crate (via `tsrs-meta.toml`)
/// so another generated crate can reference its types directly instead of
/// emitting duplicate opaque handles.
pub fn load_project_ref(crate_or_toml: &Path, from_dir: &Path) -> Result<ProjectRef, String> {
    let loaded = super::pkg_meta::load(crate_or_toml, from_dir, "rust")?;
    Ok(ProjectRef {
        crate_name: loaded.meta.package,
        dep_path: loaded.rel_dir,
        types: loaded.meta.rust_types.into_iter().collect(),
    })
}

pub(super) const KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "else", "enum", "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub",
    "ref", "return", "static", "struct", "trait", "true", "type", "unsafe", "use", "where", "while", "async", "await", "dyn", "abstract", "become", "box",
    "do", "final", "macro", "override", "priv", "typeof", "unsized", "virtual", "yield", "try", "gen", "self", "Self", "super", "crate", "_",
];

// Method names a handle type already has, from the traits every one implements.
pub(super) const TAKEN: &[&str] = &["clone", "as_ref", "eq", "ne", "into", "lend", "release", "iter", "iterate_async", "as_array", "default", "deref"];

// A value's name: a keyword gets a trailing underscore.
pub(super) fn ident(name: &str) -> String {
    let s = super::model::ident(name);
    if KEYWORDS.contains(&s.as_str()) { format!("{s}_") } else { s }
}

// `getElementById` -> `get_element_by_id`, `innerHTML` -> `inner_html`.
pub(super) fn snake(name: &str) -> String {
    super::case::snake_case(name)
}

// `Options_mode` -> `OptionsMode`, `fill` -> `Fill`: for generated type names and variants.
pub(super) fn camel(name: &str) -> String {
    let out = super::case::pascal_underscore(name);
    if out.chars().next().is_none_or(|c| c.is_ascii_digit()) {
        format!("_{out}")
    } else if KEYWORDS.contains(&out.as_str()) {
        // `"self"` as a variant is `Self_`: `Self` is Rust's.
        format!("{out}_")
    } else {
        out
    }
}

pub(super) fn camel_variant(name: &str) -> String {
    super::case::screaming_to_pascal(name, camel)
}

#[derive(Default)]
pub(super) struct Context {
    // Unions values can be decoded to, and all unions.
    pub decodable: BTreeSet<String>,
    pub unions: BTreeSet<String>,
    // Unions an argument is typed as: the decodable ones, and those only a
    // value goes into, made from a member.
    pub typed: BTreeSet<String>,
    // Types declared without type parameters: `Is` is implemented for them.
    pub classes: BTreeSet<String>,
    // The Rust name of each type the model names: a namespace's as its
    // path, `stream::PipeOptions`.
    pub types: HashMap<String, String>,
    // A namespace's types' own names (`PipeOptions`), by the model's; and
    // all of them, which a top-level type's name may be too.
    pub own: HashMap<String, String>,
    pub leaves: HashSet<String>,
}

impl Context {
    pub(super) fn decodable(&self, n: &str) -> bool {
        self.decodable.contains(n)
    }

    pub(super) fn takes(&self, n: &str) -> bool {
        self.typed.contains(n)
    }

    pub(super) fn is_union(&self, n: &str) -> bool {
        self.unions.contains(n)
    }

    pub(super) fn is_class(&self, n: &str) -> bool {
        self.classes.contains(n)
    }

    // `Option`, in full where the API declares a type of that name.
    pub(super) fn option(&self) -> &'static str {
        if self.classes.contains("Option") || self.types.contains_key("Option") { "::core::option::Option" } else { "Option" }
    }

    // A type's name in Rust, where it is named: a namespace's by its path
    // (`stream::PipeOptions`); a top-level one a namespace's type is also
    // named, from the crate (`crate::PipeOptions`), as that would hide it there.
    pub(super) fn tn(&self, n: &str) -> String {
        let t = self.types.get(n).cloned().unwrap_or_else(|| n.to_string());
        if !self.own.contains_key(n) && self.leaves.contains(&t) { format!("crate::{t}") } else { t }
    }

    // A type's name where it is defined: a namespace's own (`PipeOptions`).
    pub(super) fn def_name(&self, n: &str) -> String {
        self.own.get(n).cloned().or_else(|| self.types.get(n).cloned()).unwrap_or_else(|| n.to_string())
    }
}

pub(super) const RUNTIME: &str = include_str!("../runtime/rust_runtime.rs");

// The runtime's crate, `ts_runtime`, beside a package's: paths are from the package.
pub fn runtime_crate() -> Vec<(String, String)> {
    vec![
        ("../ts_runtime/Cargo.toml".into(), "# Generated.\n[package]\nname = \"ts_runtime\"\nversion = \"0.1.0\"\nedition = \"2024\"\n".into()),
        ("../ts_runtime/src/lib.rs".into(), format!("#![allow(non_camel_case_types, non_snake_case, clippy::all)]\n{RUNTIME}")),
    ]
}

// What the model names that the runtime declares under another name.
pub(super) const RUNTIME_TYPES: &[(&str, &str)] = &[("Promise", "JsPromise"), ("PromiseLike", "JsPromise")];

// A model type's Rust name: CamelCase (a type named in snake_case would
// share its constructor's name with a function), or the runtime's.
pub(super) fn rust_type(n: &str) -> String {
    // A bundler's `SeriesOption$1` is `SeriesOption1`.
    RUNTIME_TYPES.iter().find(|r| r.0 == n).map_or_else(|| camel(&ident(n)), |r| r.1.to_string())
}

pub const MODEL_OPTIONS: super::model::ModelOptions<'static> = super::model::ModelOptions {
    target: "Rust",
    counted: true,
    trap_uncaught: true,
    generics: true,
    overloading: false,
    retain_opaque_constraints: false,
    typed_union_arrays: false,
    args_bags: false,
    records: true,
};

// A tuple struct is also a function of its name: types and functions share
// a namespace. A type's functions are in its `impl`, beside what every
// handle has (`clone`, `lend`); a class's inherited ones in `XMethods`.
pub const NAMING: Naming = Naming {
    // The runtime's own types; std's (`Box`) a type of the crate may shadow.
    reserved: || {
        let words = identifiers(RUNTIME);
        let mut declared: Vec<String> = Vec::new();
        // At the top of the file, not an associated `type Item` in an `impl`.
        for line in RUNTIME.lines().filter(|l| !l.starts_with(char::is_whitespace)) {
            let mut it = line.split(|c: char| !(c.is_alphanumeric() || c == '_')).filter(|w| !w.is_empty());
            while let Some(w) = it.next() {
                if matches!(w, "struct" | "trait" | "enum" | "type")
                    && let Some(name) = it.next()
                {
                    declared.push(name.to_string());
                }
            }
        }
        KEYWORDS.iter().map(|k| k.to_string()).chain(declared.into_iter().filter(|d| words.contains(d))).collect()
    },
    type_names: |candidate, _, kind, _| {
        if RUNTIME_TYPES.iter().any(|r| r.0 == candidate) {
            return Vec::new();
        }
        let t = rust_type(candidate);
        if kind == Kind::Class { vec![t.clone(), format!("{t}Methods")] } else { vec![t] }
    },
    values_of: |_, _, _, _| Vec::new(),
    types_apart: false,
    overloads: false,
    func_name: |_, name, f| Some(rust_function(f, name)),
    funcs_flat: false,
    nests: true,
    members: |_, _| TAKEN.iter().map(|t| t.to_string()).collect(),
};

// A function's Rust name, were its model name `name`: what every handle has
// already (`clone`) is `js_clone`.
pub(super) fn rust_function(f: &Func, name: &str) -> String {
    let base = types::function_name_as(f, name);
    if TAKEN.contains(&base.as_str()) { format!("js_{base}") } else { base }
}

// What every file of the crate starts with.
pub(super) const PRELUDE: &str = "// Generated.\n#![allow(unused_imports)]\nuse crate::raw;\nuse crate::runtime::handle;\nuse crate::*;\nuse core::marker::PhantomData;\n\n";

// The crate: see the top of this file.
pub fn files(m: &Model) -> Result<Vec<(String, String)>, Vec<String>> {
    files_named(m, "bindings", &[])
}

pub fn files_named(m: &Model, crate_name: &str, refs: &[ProjectRef]) -> Result<Vec<(String, String)>, Vec<String>> {
    emit::files_named(m, crate_name, refs)
}
