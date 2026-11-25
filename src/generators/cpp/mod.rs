// XML -> C++ bindings for a wasm32 module, built with the WASI SDK.
//
// Every JavaScript object is a value holding a handle, and every type
// inherits, virtually, from `ts::Handle`: inheritance and mixins are real C++
// inheritance, sharing one handle, and an HTMLDivElement is an Element.
// Strings go in as `std::string_view` and come back as `std::string`;
// nullables are `std::optional`, unions `std::variant`, arrays `ts::Array<T>`,
// callbacks lambdas, generic types class templates, and keyed methods
// templates over typed keys:
// `el.addEventListener(HTMLElementEventMapKeys::click, [](PointerEvent e) {...})`.
//
// Files:
// - `X.decl.hpp`: the struct, declaring its members. Includes only its
//   ancestors' structs, so a chain of them never meets a half-read header.
// - `X.hpp`: what a program includes. The struct, then the headers of every
//   type its members name, then the definitions of templates.
// - `<module>.cpp`: every other definition, compiled once and linked.
// - `types/X.hpp`: a callback or union.

use std::collections::{BTreeSet, HashSet};
use std::path::Path;

use super::model::{Call, Model};
use super::names::{Kind, Naming};

mod decl;
mod emit;
mod types;

pub(super) struct Ctx {
    // The unions of the model being generated that values can be decoded to.
    pub decodable: BTreeSet<String>,
    // Those that cannot, but whose members can be made of a value: typed where a value only goes in.
    pub typable: BTreeSet<String>,
    // Types a class's member is named as (`easingFunc easingFunc()`): within
    // the class the member hides the type, so it is written `::easingFunc`.
    pub qualified: BTreeSet<String>,
    pub ns_prefix: String,
    // Types imported from `--ref` project headers (may be handles, enums, or variants).
    pub resolved_ref: BTreeSet<String>,
}

impl Ctx {
    pub(super) fn is_resolved_ref(&self, n: &str) -> bool {
        self.resolved_ref.contains(n)
    }

    // A declared type's name where a type is written: global where a member hides it.
    pub(super) fn tn(&self, n: &str) -> String {
        if self.qualified.contains(n) {
            format!("::{}{n}", self.ns_prefix)
        } else {
            n.to_string()
        }
    }
}

/// A reference to a previously generated C++ binding directory.
pub struct ProjectRef {
    pub pkg_name: String,
    pub umbrella_include: String,
    pub types: HashSet<String>,
}

pub(super) fn is_builtin_cpp_type(name: &str) -> bool {
    matches!(name, "Error" | "Date" | "Promise" | "PromiseLike") || super::model::buffer_spec(name).is_some()
}

/// Loads a previously generated C++ binding directory (via `tsrs-meta.toml`)
/// so another generated C++ package can `#include` its umbrella header and
/// emit `using Type = ::ref_pkg::Type;` aliases instead of opaque stubs.
pub fn load_project_ref(dir_or_header: &Path, from_dir: &Path) -> Result<ProjectRef, String> {
    let loaded = super::pkg_meta::load(dir_or_header, from_dir, "cpp")?;
    let entry = loaded.meta.entry.as_deref().unwrap_or("bindings.hpp");
    let pkg_name = loaded.meta.package;
    let umbrella_include = format!("{}/{entry}", loaded.rel_dir);
    let types = loaded.meta.types.into_iter().collect();
    Ok(ProjectRef { pkg_name, umbrella_include, types })
}

pub(super) fn import_attr(wasm_module: &str, name: &str) -> String {
    if wasm_module == "env" {
        format!("TS_IMPORT({name})")
    } else {
        format!("TS_IMPORT_MOD(\"{wasm_module}\", {name})")
    }
}

pub(super) fn raw_fn(raw_prefix: &str, name: &str) -> String {
    format!("ts_raw_{raw_prefix}{name}")
}

pub(super) fn wrap_ns(ns: Option<&str>, body: &str) -> String {
    match ns {
        Some(pkg) if !body.trim().is_empty() => format!("namespace {pkg} {{\n{body}}}  // namespace {pkg}\n"),
        _ => body.to_string(),
    }
}

const KEYWORDS: &[&str] = &[
    "alignas", "alignof", "and", "and_eq", "asm", "auto", "bitand", "bitor", "bool", "break", "case", "catch", "char", "char8_t", "char16_t",
    "char32_t", "class", "compl", "concept", "const", "consteval", "constexpr", "constinit", "const_cast", "continue", "co_await", "co_return",
    "co_yield", "decltype", "default", "delete", "do", "double", "dynamic_cast", "else", "enum", "explicit", "export", "extern", "false", "float",
    "for", "friend", "goto", "if", "inline", "int", "long", "mutable", "namespace", "new", "noexcept", "not", "not_eq", "nullptr", "operator",
    "or", "or_eq", "private", "protected", "public", "register", "reinterpret_cast", "requires", "return", "short", "signed", "sizeof", "static",
    "static_assert", "static_cast", "struct", "switch", "template", "this", "thread_local", "throw", "true", "try", "typedef", "typeid",
    "typename", "union", "unsigned", "using", "virtual", "void", "volatile", "wchar_t", "while", "xor", "xor_eq", "errno", "assert", "NULL", "stdin", "stdout", "stderr",
    // The handle every type holds.
    "h",
];

// The C library's functions a WASI program has at global scope.
pub(super) const LIBC_GLOBALS: &[&str] = &[
    "abort", "abs", "acos", "asin", "atan", "atan2", "atexit", "bsearch", "calloc", "ceil", "clock", "cos", "ctime", "difftime", "div", "exit",
    "exp", "fabs", "floor", "fmod", "free", "getenv", "gmtime", "labs", "localtime", "log", "malloc", "memcpy", "memset", "mktime", "perror",
    "pow", "printf", "puts", "qsort", "raise", "rand", "realloc", "remove", "rename", "round", "signal", "sin", "sprintf", "sqrt", "srand",
    "strftime", "strlen", "system", "tan", "time", "tolower", "toupper", "trunc",
];

pub(super) fn ident(name: &str) -> String {
    let s = super::model::ident(name);
    if KEYWORDS.contains(&s.as_str()) { format!("{s}_") } else { s }
}

// Types, their helpers (`decode_Id`, `Mode_keys`) and free functions are in
// the global namespace; a type's functions in its own, where they overload.
pub const MODEL_OPTIONS: super::model::ModelOptions<'static> = super::model::ModelOptions {
    target: "C++",
    counted: true,
    trap_uncaught: false,
    generics: true,
    overloading: true,
    retain_opaque_constraints: false,
    typed_union_arrays: false,
    args_bags: false,
};

pub const NAMING: Naming = Naming {
    reserved: || KEYWORDS.iter().map(|k| k.to_string()).chain(["ts", "std"].map(String::from)).collect(),
    type_names: |candidate, name, kind, m| {
        let mut ids = vec![candidate.to_string()];
        match kind {
            Kind::Union => ids.push(format!("decode_{candidate}")),
            Kind::Class if m.maps.iter().any(|k| k.name == name) => ids.push(format!("{candidate}Keys")),
            _ => {}
        }
        ids
    },
    values_of: |_, _, _, _| Vec::new(),
    types_apart: false,
    overloads: true,
    // A getter or setter is its property's name, a constructor `create`.
    func_name: |owner, name, f| match &f.call {
        Call::Get(_) | Call::GetBound(_) | Call::Set(_) | Call::IndexGet | Call::IndexSet | Call::New(_) | Call::Create(_) => None,
        Call::StaticGet(_) | Call::StaticSet(_) if f.module.is_empty() => None,
        _ => Some(types::cpp_name_as(f, name, if owner.is_some() { "create" } else { "" })),
    },
    funcs_flat: false,
    nests: false,
    members: |c, _| vec![c.name.clone(), "h".into()],
};

pub(super) const RUNTIME: &str = include_str!("../runtime/cpp_runtime.hpp");

// The bindings as files: see the top of this file.
pub fn files(m: &Model) -> Result<Vec<(String, String)>, Vec<String>> {
    files_named(m, "bindings", &[])
}

pub fn files_named(m: &Model, pkg_name: &str, refs: &[ProjectRef]) -> Result<Vec<(String, String)>, Vec<String>> {
    emit::files_named(m, pkg_name, refs)
}
