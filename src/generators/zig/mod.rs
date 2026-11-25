// XML -> Zig module for a wasm32 module. As C, but unions of simple types
// become tagged unions, and nullable objects become optionals.

use std::collections::HashSet;
use std::path::Path;

use super::model::{Call, Func, Model};
use super::names::Naming;

mod emit;
mod types;

/// A reference to a previously generated Zig binding package.
pub struct ProjectRef {
    pub pkg_name: String,
    pub dep_path: String,
    pub types: HashSet<String>,
}

/// Loads a previously generated Zig binding directory (via `tsrs-meta.toml`)
/// so another generated package can import its types instead of stubbing them.
pub fn load_project_ref(dir_or_root: &Path, from_dir: &Path) -> Result<ProjectRef, String> {
    let loaded = super::pkg_meta::load(dir_or_root, from_dir, "zig")?;
    Ok(ProjectRef {
        pkg_name: loaded.meta.package,
        dep_path: loaded.rel_dir,
        types: loaded.meta.types.into_iter().collect(),
    })
}

fn is_zig_keyword_or_primitive(name: &str) -> bool {
    if matches!(
        name,
        "_"
            | "addrspace"
            | "align"
            | "allowzero"
            | "and"
            | "anyframe"
            | "anytype"
            | "asm"
            | "async"
            | "await"
            | "break"
            | "callconv"
            | "catch"
            | "comptime"
            | "const"
            | "continue"
            | "defer"
            | "else"
            | "enum"
            | "errdefer"
            | "error"
            | "export"
            | "extern"
            | "fn"
            | "for"
            | "if"
            | "inline"
            | "linksection"
            | "noalias"
            | "noinline"
            | "nosuspend"
            | "opaque"
            | "or"
            | "orelse"
            | "packed"
            | "pub"
            | "resume"
            | "return"
            | "struct"
            | "suspend"
            | "switch"
            | "test"
            | "threadlocal"
            | "try"
            | "union"
            | "unreachable"
            | "usingnamespace"
            | "var"
            | "volatile"
            | "while"
            | "bool"
            | "f16"
            | "f32"
            | "f64"
            | "f80"
            | "f128"
            | "isize"
            | "usize"
            | "c_char"
            | "c_short"
            | "c_ushort"
            | "c_int"
            | "c_uint"
            | "c_long"
            | "c_ulong"
            | "c_longlong"
            | "c_ulonglong"
            | "c_longdouble"
            | "anyopaque"
            | "void"
            | "noreturn"
            | "type"
            | "anyerror"
            | "comptime_int"
            | "comptime_float"
            | "true"
            | "false"
            | "null"
            | "undefined"
    ) {
        return true;
    }
    if let Some(rest) = name.strip_prefix(['i', 'u'])
        && !rest.is_empty()
        && rest.bytes().all(|b| b.is_ascii_digit())
    {
        return true;
    }
    false
}

// Bare identifier when valid and unreserved; `@"..."` otherwise.
pub(super) fn ident(name: &str) -> String {
    let mut bytes = name.bytes();
    let valid_bare = bytes.next().is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
        && !is_zig_keyword_or_primitive(name);
    if valid_bare {
        name.to_string()
    } else {
        format!("@\"{}\"", name.replace('\\', "\\\\").replace('"', "\\\""))
    }
}

// `get_x` -> `getX`, `addEventListener_with_string` -> `addEventListenerWithString`.
// Leading underscores (such as `$add` -> `_add`) stay so they do not clash with `add`.
fn method_name(name: &str) -> String {
    let lead_len = name.bytes().take_while(|&b| b == b'_').count();
    let rest = &name[lead_len..];
    if rest.is_empty() {
        return name.to_string();
    }
    let mut parts = rest.split('_');
    let mut out = format!("{}{}", &name[..lead_len], parts.next().unwrap_or_default());
    for part in parts {
        if part.is_empty() {
            out.push('_');
        } else {
            let mut cs = part.chars();
            if let Some(c) = cs.next() {
                out.extend(c.to_uppercase());
                out.push_str(cs.as_str());
            }
        }
    }
    out
}

pub(super) fn zig_func_name(name: &str, f: &Func) -> String {
    if matches!(f.call, Call::Get(_) | Call::GetBound(_) | Call::StaticGet(_))
        && let Some(rest) = name.strip_prefix("get_")
        && super::case::starts_with_word(rest, "get")
    {
        return method_name(rest);
    }
    method_name(name)
}

// What the runtime declares at the top of every file.
pub(super) const RUNTIME_NAMES: &[&str] = &[
    "JsValue", "JsString", "JsArray", "JsArrayOf", "JsTuple", "JsIteratorOf", "JsListener", "Lent", "jsRelease", "jsRetain", "jsLiveHandles", "jsForgetCallback",
    "jsTakeError", "jsErrorMessage", "jsErrorName", "jsErrorStack", "jsErrorNew", "jsDateNow", "jsEnumOf", "jsSame", "jsHasError", "jsTry", "JsScope", "jsTake",
];

// Types and functions share a file's namespace, and the root's that
// re-exports them; a type's functions are in its struct, beside what the
// struct has of its own.
pub const MODEL_OPTIONS: super::model::ModelOptions<'static> = super::model::ModelOptions {
    target: "Zig",
    counted: false,
    trap_uncaught: false,
    generics: false,
    overloading: false,
    retain_opaque_constraints: false,
    typed_union_arrays: false,
    args_bags: false,
};

pub const NAMING: Naming = Naming {
    // And what a struct declares inside (a map's `Key` and `Type`): Zig lets
    // no declaration shadow another, so no type of the file has those names.
    reserved: || RUNTIME_NAMES.iter().chain(&["raw", "std", "builtin", "JsAsyncIterator", "JsIterator", "Key", "Type", "Methods"]).map(|n| n.to_string()).collect(),
    type_names: |candidate, _, _, _| vec![candidate.to_string()],
    values_of: |_, _, _, _| Vec::new(),
    types_apart: false,
    overloads: false,
    func_name: |_, name, f| Some(zig_func_name(name, f)),
    funcs_flat: false,
    nests: true,
    members: |c, m| {
        let mut own = vec!["h".to_string(), "Methods".to_string(), "Key".to_string(), "Type".to_string()];
        own.extend(c.consts.iter().map(|(n, _)| n.clone()));
        let mut todo: Vec<&str> = c.ancestors.iter().map(String::as_str).collect();
        let mut seen: HashSet<&str> = HashSet::new();
        while let Some(a) = todo.pop() {
            if a != c.name && seen.insert(a) {
                own.push(format!("as{a}"));
                if let Some(ac) = m.classes.iter().find(|k| k.name == a) {
                    todo.extend(ac.ancestors.iter().map(String::as_str));
                }
            }
        }
        // Its own `iterate`, where it declares none.
        let declares = |n: &str| c.funcs.iter().any(|f| f.name == n);
        if c.iterates_async.is_some() && !declares("iterateAsync") {
            own.push("iterateAsync".into());
        }
        if c.iterates.is_some() && !declares("iterate") {
            own.push("iterate".into());
        }
        own
    },
};

pub(super) const RUNTIME: &str = include_str!("../runtime/zig_runtime.zig");

pub(super) const RUNTIME_EXTERNS: &str = include_str!("../runtime/zig_runtime_externs.zig");

pub fn files(m: &Model) -> Result<Vec<(String, String)>, Vec<String>> {
    files_named(m, "bindings", &[])
}

pub fn files_named(m: &Model, pkg_name: &str, refs: &[ProjectRef]) -> Result<Vec<(String, String)>, Vec<String>> {
    files_with_imports(m, pkg_name, refs).map(|(files, _)| files)
}

pub fn files_with_imports(m: &Model, pkg_name: &str, refs: &[ProjectRef]) -> Result<(Vec<(String, String)>, HashSet<String>), Vec<String>> {
    emit::files_with_imports(m, pkg_name, refs)
}
