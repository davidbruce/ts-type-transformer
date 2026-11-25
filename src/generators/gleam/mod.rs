// XML -> Gleam bindings for Gleam's JavaScript target: modules of opaque
// types and external functions, each with an FFI module that calls the API
// and converts between JavaScript's values and Gleam's.
//
// Gleam modules may not import each other in a cycle. Every type is in
// `bindings/types`; `Promise`, `JsError` and `Codec` in `bindings/runtime`.
// An area's functions, in `bindings/<area>`, import only types, so no
// module imports one that imports it.
//
// Subtypes are phantom types along each class's chain of first bases:
// `Element(a) = Node(ElementTag(a))`. Parameters are open (`Node(a)`, any
// subtype); results, callbacks' parameters and what lists and variants hold
// are closed (`Node(Nil)`). See DESIGN.md.
//
// Gleam has functions and custom types; no methods, subtypes, overloads or
// optional arguments. So, from the shared model (`generators::model`): a class
// is an opaque type and its members functions named for it
// (`shape_get_size(s)`), an ancestor is reached by `shape_as_named(s)`, an
// optional argument or a nullable value is an `Option(a)`, an array a
// `List(a)`, a tuple a tuple, a union or string enum a custom type, a
// callback a Gleam function, a promise a `Promise(a)` to `promise_then`.
// Gleam's values are JavaScript's, but for these: the FFI converts them.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use super::model::{Call, Func, Model};
use super::names::{Kind, Naming};

mod bounds;
mod classes;
mod emit;
mod types;

#[derive(Clone, Debug)]
pub struct ProjectRef {
    pub package_name: String,
    pub dep_path: String,
    /// Non-generic class/interface phantom types (`Name(sub)`) exported in `<pkg>/types.gleam`.
    pub classes: HashSet<String>,
}

/// Loads a previously generated Gleam binding package (via `tsrs-meta.toml`)
/// so another generated package can import its class types directly.
pub fn load_project_ref(pkg_or_toml: &Path, from_dir: &Path) -> Result<ProjectRef, String> {
    let loaded = super::pkg_meta::load(pkg_or_toml, from_dir, "gleam")?;
    Ok(ProjectRef {
        package_name: loaded.meta.package,
        dep_path: loaded.rel_dir,
        classes: loaded.meta.types.into_iter().collect(),
    })
}

pub(super) const KEYWORDS: &[&str] = &[
    "as", "assert", "auto", "case", "const", "delegate", "derive", "echo", "else", "fn", "if", "implement", "import", "let", "macro", "opaque", "panic", "pub", "test", "todo", "type", "use",
];

// Types the bindings declare whatever the API: what a module of Gleam can
// name of JavaScript's values. Not opaque types of the API.
pub(super) const OWN: &[&str] = &["Promise", "JsError", "Dynamic", "Option", "List", "Result", "Nil", "Int", "Float", "String", "Bool", "BitArray", "Codec", "Ref", "Key"];
pub(super) const OPAQUE_BUILTINS: &[&str] = &["Promise", "PromiseLike"];
pub(super) const MAX_INHERITED_ALIASES: usize = 40_000;

// What JavaScript reserves, as a function's name.
pub(super) const JS_RESERVED: &[&str] = &[
    "await", "break", "case", "catch", "class", "const", "continue", "debugger", "default", "delete", "do", "else", "enum", "export", "extends", "false",
    "finally", "for", "function", "if", "implements", "import", "in", "instanceof", "interface", "let", "new", "null", "package", "private", "protected",
    "public", "return", "static", "super", "switch", "this", "throw", "true", "try", "typeof", "var", "void", "while", "with", "yield",
    // Not names in a module's strict code.
    "arguments", "eval",
];

// The modules every library's bindings have.
pub(super) const OWN_RUNTIME: &[&str] = &["Promise", "JsError", "Codec", "Ref", "Key"];

// A class type's phantom parameter, where it is declared.
pub(super) const PHANTOM: &str = "sub";

// Types (and variants) and values are apart; a type's functions, constants,
// upcasts and `to_list` are values named for it: `shape_area`.
pub const MODEL_OPTIONS: super::model::ModelOptions<'static> = super::model::ModelOptions {
    target: "Gleam",
    counted: false,
    trap_uncaught: false,
    generics: true,
    overloading: false,
    retain_opaque_constraints: true,
    typed_union_arrays: true,
    args_bags: true,
};

pub const NAMING: Naming = Naming {
    reserved: || KEYWORDS.iter().chain(OWN).map(|k| k.to_string()).collect(),
    // An external `Promise` is Gleam's own; a class the API declares as
    // `Option` is not, and gives way.
    type_names: |candidate, _, kind, _| match kind {
        Kind::Opaque if OPAQUE_BUILTINS.contains(&candidate) => Vec::new(),
        // A class's marker for the phantom of its subtypes.
        Kind::Class => vec![upper(candidate), format!("{}Tag", upper(candidate))],
        _ => vec![upper(candidate)],
    },
    // Every type has a codec in the types module: `shape_codec`.
    values_of: |own, _, _, _| vec![lower(&format!("{}_codec", snake(own)))],
    types_apart: true,
    overloads: false,
    // A class's functions are in its own module: `shape.area`.
    func_name: |_, name, f| Some(lower(&function_name(f, name))),
    funcs_flat: false,
    nests: false,
    // Beside its functions there: its constants, list helpers and casts.
    members: |c, m| {
        let mut out: Vec<String> = c.consts.iter().map(|(k, _)| lower(k)).collect();
        out.push("upcast".into());
        if c.iterates.is_some() || c.elements.is_some() {
            out.push("to_list".into());
        }
        if c.elements.is_some() {
            out.extend(["get_at", "length"].map(String::from));
        }
        if c.js.is_some() {
            let has = |target: &str| c.funcs.iter().any(|f| lower(&function_name(f, &f.name)) == target);
            if bounds::parent(m, c).is_some() && !has("from") {
                out.push("from".into());
            }
            if !has("cast") {
                out.push("cast".into());
            }
        }
        out.extend(bounds::secondary(m, c).iter().map(|a| lower(&format!("as_{}", snake(&upper(a))))));
        out
    },
};

pub struct Output {
    pub name: String,
    pub files: Vec<(String, String)>,
    pub project_refs: Vec<(String, String)>,
    pub skipped: BTreeMap<String, usize>,
    // What each binding reaches, for a page to check (`wasm::manifest`).
    pub manifest: String,
    pub meta_toml: String,
}

impl Output {
    pub fn gleam_toml(&self) -> String {
        let deps: String = self
            .project_refs
            .iter()
            .map(|(name, path)| format!("{name} = {{ path = {:?} }}\n", path))
            .collect();
        format!(
            "name = {:?}\n\
             version = \"0.1.0\"\n\
             target = \"javascript\"\n\n\
             [dependencies]\n\
             gleam_stdlib = \">= 1.0.0 and < 2.0.0\"\n\
             {deps}",
            self.name
        )
    }
}

// `getElementById` -> `get_element_by_id`, `HTMLElement` -> `html_element`.
pub(super) fn snake(name: &str) -> String {
    super::case::snake_case_alnum(name)
}

// A Gleam name for a value: snake_case, starting with a letter, not a keyword.
pub(super) fn lower(name: &str) -> String {
    let s = snake(name).trim_matches('_').to_string();
    let s = if s.starts_with(|c: char| c.is_ascii_lowercase()) { s } else { format!("x{s}") };
    if KEYWORDS.contains(&s.as_str()) { format!("{s}_") } else { s }
}

// A Gleam name for a type or a variant: UpperCamelCase, letters and digits only.
pub(super) fn upper(name: &str) -> String {
    let out = super::case::pascal_ascii_alnum(name);
    if out.starts_with(|c: char| c.is_ascii_uppercase()) { out } else { format!("X{out}") }
}

pub(super) fn upper_variant(name: &str) -> String {
    super::case::screaming_to_pascal(name, upper)
}

// The JavaScript name of the codec of type parameter `n`.
pub(super) fn codec_var(n: &str) -> String {
    format!("$c_{}", n.replace(|c: char| !c.is_alphanumeric() && c != '_', "_"))
}

// `to` (a path from the package's root), relative to module `from`.
pub(super) fn relative(from: &str, to: &str) -> String {
    let from: Vec<&str> = from.split('/').collect();
    let from = &from[..from.len() - 1];
    let to: Vec<&str> = to.split('/').collect();
    let common = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
    let up = "../".repeat(from.len() - common);
    let rest = to[common..].join("/");
    if up.is_empty() { format!("./{rest}") } else { format!("{up}{rest}") }
}

// A function's name, as the model has it; a `$` method (`$add`, which
// returns a new value where `add` changes this one) is `add_new`, while a `$`
// property accessor (`$from`) is `get_dollar_from` / `set_dollar_from`.
pub(super) fn function_name(f: &Func, name: &str) -> String {
    let dollar = super::case::has_dollar_js_name(&f.call);
    if !dollar {
        if matches!(&f.call, Call::Get(_) | Call::GetBound(_) | Call::StaticGet(_))
            && let Some(rest) = name.strip_prefix("get_")
            && super::case::starts_with_word(rest, "get")
        {
            return rest.to_string();
        }
        return name.to_string();
    }
    let is_prop = matches!(&f.call, Call::Get(_) | Call::GetBound(_) | Call::Set(_) | Call::StaticGet(_) | Call::StaticSet(_));
    let is_set = matches!(&f.call, Call::Set(_) | Call::StaticSet(_));
    if is_prop {
        let stripped = if is_set { name.strip_prefix("set_") } else { name.strip_prefix("get_") }.unwrap_or(name);
        let stem = stripped.trim_start_matches('_');
        let prefix = if is_set { "set_dollar" } else { "get_dollar" };
        return if stem.is_empty() { prefix.to_string() } else { format!("{prefix}_{stem}") };
    }
    let stem = name.trim_start_matches('_');
    match stem.strip_prefix("set_") {
        Some(rest) => format!("set_{}_new", rest.trim_start_matches('_')),
        None => format!("{}_new", if stem.is_empty() { "dollar" } else { stem }),
    }
}

// A module's Gleam path: as given out, with `areas/main` mapped to pkg root.
pub(super) fn module_path(paths: &HashMap<String, String>, pkg: &str, module: &str) -> String {
    paths.get(module).cloned().unwrap_or_else(|| {
        if module == "areas/main" {
            pkg.to_string()
        } else {
            format!("{pkg}/{}", module.split('/').map(lower).collect::<Vec<_>>().join("/"))
        }
    })
}

pub(super) fn func_unit(m: &Model, paths: &HashMap<String, String>, pkg: &str, f: &Func) -> String {
    match m.func_namespaces.get(&f.import) {
        Some((path, _, module, depth)) => std::iter::once(module_path(paths, pkg, module))
            .chain(path[(*depth).min(path.len())..].iter().map(|s| snake(&upper(s))))
            .collect::<Vec<_>>()
            .join("/"),
        None => paths[&f.module].clone(),
    }
}

pub(super) fn gleam_const_literal(value: &str) -> Option<String> {
    if value.starts_with('"') {
        Some(value.to_string())
    } else if value == "true" || value == "false" {
        Some(upper(value))
    } else if let Ok(n) = value.parse::<f64>() {
        Some(format!("{n:?}"))
    } else if let Some(hex) = value.strip_prefix("0x").or(value.strip_prefix("0X")) {
        Some(format!("{:?}", i64::from_str_radix(hex, 16).unwrap_or(0) as f64))
    } else {
        None
    }
}

// The bindings of the XML, or what Gleam cannot express of it.
pub fn module(xml: &str) -> Result<Output, Vec<String>> {
    module_named(xml, "bindings", &[])
}

pub fn module_named(xml: &str, package_name: &str, refs: &[ProjectRef]) -> Result<Output, Vec<String>> {
    emit::module_named(xml, package_name, refs)
}
