// XML -> C header for a wasm32 module. No inheritance, generics, overloads,
// optional parameters or unions: the config must have lowered them all.

mod emit;
mod types;

use std::collections::{HashMap, HashSet};
use std::path::Path;

use super::model::{Model, Ty};
use super::names::{Kind, Naming, identifiers};

pub(super) use self::types::{union_fields, union_fields_for};

pub(super) const T: &str = "C";
pub(super) const RUNTIME: &str = include_str!("../runtime/c_runtime.h");

pub(super) struct Ctx {
    pub decodable: HashSet<String>,
    // Unions with a C type: the decodable ones, and those only a value goes into.
    pub typed: HashSet<String>,
    // Unions made from a member by `U_of(x)`.
    // with the C types it tells members by.
    pub of: HashMap<String, Vec<String>>,
    // Typed arrays of unions (`JsArray_<U>`) that accept member arrays via `_Generic`.
    pub array_arg: HashMap<String, Vec<String>>,
    // Each class's subclasses here, for a union to take one as the member it is.
    pub subtypes: HashMap<String, Vec<String>>,
    // Callback handle types declared in this module.
    pub callbacks: HashSet<String>,
}

impl Ctx {
    pub fn decodable(&self, n: &str) -> bool {
        self.decodable.contains(n)
    }

    // Whether a parameter of this union is typed as it.
    pub fn takes(&self, n: &str) -> bool {
        self.typed.contains(n)
    }
}

/// A reference to a previously generated C binding directory.
pub struct ProjectRef {
    pub pkg_name: String,
    pub ts_types_include: String,
    pub umbrella_include: String,
    pub structs: HashSet<String>,
    pub external_structs: HashSet<String>,
    pub struct_aliases: HashMap<String, String>,
    pub subtypes: HashMap<String, Vec<String>>,
    pub all_names: Vec<String>,
}

// The names the runtime takes. What a pragma or an attribute says is not one.
pub(super) fn runtime_names() -> Vec<String> {
    let code: Vec<&str> = RUNTIME.lines().filter(|l| !l.contains("_Pragma") && !l.contains("#pragma") && !l.contains("__attribute__((error")).collect();
    identifiers(&code.join("\n"))
}

pub(super) fn is_builtin_c_type(name: &str) -> bool {
    matches!(
        name,
        "Error" | "Date" | "Promise" | "JsValue" | "JsString" | "JsArray" | "JsIterator" | "JsOptionalDouble" | "JsOptionalBool" | "TsStr"
    ) || super::model::buffer_spec(name).is_some()
        || name.starts_with("JsArray_")
        || name.starts_with("JsTuple_")
        || name.starts_with("JsRecord_")
}

/// Loads a previously generated C binding directory (via `tsrs-meta.toml`)
/// so another generated C package can `#include` its `ts_types.h` and reserve
/// its global C identifiers against collisions.
pub fn load_project_ref(dir_or_header: &Path, from_dir: &Path) -> Result<ProjectRef, String> {
    let loaded = super::pkg_meta::load(dir_or_header, from_dir, "c")?;
    let entry = loaded.meta.entry.as_deref().unwrap_or("bindings.h");
    let pkg_name = loaded.meta.package;
    let ts_types_include = format!("{}/ts_types.h", loaded.rel_dir);
    let umbrella_include = format!("{}/{entry}", loaded.rel_dir);
    let structs = loaded.meta.structs.into_iter().collect();
    let external_structs = loaded.meta.external_structs.into_iter().collect();
    let struct_aliases = loaded.meta.c_structs.into_iter().collect();
    let subtypes = loaded.meta.c_subtypes.into_iter().collect();
    let all_names = loaded.meta.types;
    Ok(ProjectRef { pkg_name, ts_types_include, umbrella_include, structs, external_structs, struct_aliases, subtypes, all_names })
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

pub(super) const KEYWORDS: &[&str] = &[
    "auto", "bool", "break", "case", "char", "const", "continue", "default", "do", "double", "else", "enum", "extern", "false", "float", "for",
    "goto", "if", "inline", "int", "long", "register", "restrict", "return", "self", "short", "signed", "sizeof", "static", "struct", "switch",
    "true", "typedef", "union", "unsigned", "void", "volatile", "while",
];

pub(super) const LIBC_NAMES: &[&str] = &[
    "abort", "abs", "acos", "asin", "atan", "atan2", "atof", "atoi", "atol", "bsearch", "calloc", "ceil", "clock",
    "cos", "cosh", "div", "exit", "exp", "fabs", "fclose", "fflush", "floor", "fmod", "fopen", "fprintf", "fputc",
    "fputs", "fread", "free", "freopen", "fwrite", "getchar", "getenv", "hypot", "labs", "ldexp", "log", "log10",
    "log2", "malloc", "memchr", "memcmp", "memcpy", "memmove", "memset", "perror", "pow", "printf", "putchar",
    "puts", "qsort", "rand", "realloc", "remove", "rename", "round", "sin", "sinh", "snprintf", "sprintf", "sqrt",
    "srand", "sscanf", "strcat", "strchr", "strcmp", "strcpy", "strlen", "strncat", "strncmp", "strncpy", "strrchr",
    "strstr", "strtod", "strtol", "strtoul", "system", "tan", "tanh", "time", "tmpfile", "trunc", "vsnprintf",
    "vsprintf",
];

pub(super) fn ident(name: &str) -> String {
    if KEYWORDS.contains(&name) { format!("{name}_") } else { name.to_string() }
}

// C has one namespace: a type, its helpers (`AS_Shape`, `Id_from_handle`,
// `Mode_fill`) and every function (`Shape_area`) are identifiers in it.
pub const MODEL_OPTIONS: super::model::ModelOptions<'static> = super::model::ModelOptions {
    target: "C",
    counted: false,
    trap_uncaught: false,
    generics: false,
    overloading: false,
    retain_opaque_constraints: false,
    typed_union_arrays: false,
    args_bags: false,
    records: true,
    c_union_trampoline: true,
};

fn default_c_prefix(import: &str) -> Option<String> {
    fn strip_pkg_noise(raw: &str) -> String {
        let mut s = raw.to_ascii_lowercase();
        loop {
            let next = s
                .strip_suffix("-compat")
                .or_else(|| s.strip_suffix("-core"))
                .or_else(|| s.strip_suffix(".js"))
                .or_else(|| s.strip_suffix("-js"))
                .or_else(|| s.strip_suffix("-lib"));
            match next {
                Some(rest) => s = rest.to_string(),
                None => break,
            }
        }
        if s.len() >= 5 && s.ends_with("js") {
            s.truncate(s.len() - 2);
        }
        let clean: String = s
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect();
        clean.trim_matches('_').to_string()
    }
    let chosen = if let Some((scope, pkg)) = import.strip_prefix('@').and_then(|s| s.split_once('/')) {
        let scope_s = strip_pkg_noise(scope);
        let pkg_s = strip_pkg_noise(pkg);
        if pkg_s.is_empty()
            || matches!(
                pkg_s.as_str(),
                "lite" | "core" | "compat" | "js" | "client" | "runtime" | "browser" | "web" | "wasm"
            )
        {
            scope_s
        } else {
            pkg_s
        }
    } else {
        strip_pkg_noise(import)
    };
    (!chosen.is_empty()).then_some(chosen)
}

pub(super) fn effective_prefix(m: &Model) -> Option<String> {
    match m.prefix.as_deref() {
        Some("") => None,
        Some(p) => Some(p.to_string()),
        None => m.root_import.as_deref().and_then(default_c_prefix),
    }
}

fn apply_prefix(m: &mut Model, pfx: &str, opaque_kept: &mut HashSet<String>) {
    let type_pfx = super::case::pascal_ascii_alnum(pfx);
    let fn_pfx: String = pfx
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '_' })
        .collect::<String>()
        .trim_matches('_')
        .to_string();
    if type_pfx.is_empty() || fn_pfx.is_empty() {
        return;
    }
    let all_types: Vec<String> = m
        .classes
        .iter()
        .map(|c| c.name.clone())
        .chain(m.callbacks.iter().map(|c| c.name.clone()))
        .chain(m.unions.iter().map(|u| u.name.clone()))
        .chain(m.type_aliases.iter().map(|a| a.0.clone()))
        .chain(m.record_aliases.iter().map(|a| a.0.clone()))
        .chain(m.enums.iter().map(|e| e.0.clone()))
        .chain(m.num_enums.iter().map(|e| e.0.clone()))
        .chain(m.opaque.iter().cloned())
        .collect();
    let mut claimed: HashSet<String> = all_types.iter().cloned().collect();
    let mut renames: HashMap<String, String> = HashMap::new();
    for name in all_types {
        if is_builtin_c_type(&name)
            || opaque_kept.contains(&name)
            || super::names::has_prefix_word(pfx, &name)
            || renames.contains_key(&name)
        {
            continue;
        }
        let base = format!("{type_pfx}_{name}");
        let mut candidate = base.clone();
        let mut n = 2;
        while claimed.contains(&candidate) {
            candidate = format!("{base}_{n}");
            n += 1;
        }
        claimed.insert(candidate.clone());
        renames.insert(name, candidate);
    }
    super::names::rename_types(m, &renames);
    let mut occupied: HashSet<String> = m
        .classes
        .iter()
        .map(|c| c.name.clone())
        .chain(m.callbacks.iter().map(|c| c.name.clone()))
        .chain(m.unions.iter().map(|u| u.name.clone()))
        .chain(m.enums.iter().map(|e| e.0.clone()))
        .chain(m.num_enums.iter().map(|e| e.0.clone()))
        .chain(m.opaque.iter().cloned())
        .collect();
    let mut kept_sorted: Vec<String> = opaque_kept.iter().cloned().collect();
    kept_sorted.sort();
    let mut restore: HashMap<String, String> = HashMap::new();
    for name in kept_sorted {
        let orig = m.opaque_names.get(&name).cloned().unwrap_or_else(|| name.clone());
        if orig != name && !occupied.contains(&orig) {
            occupied.insert(orig.clone());
            restore.insert(name.clone(), orig.clone());
            opaque_kept.remove(&name);
            opaque_kept.insert(orig);
        }
    }
    super::names::rename_types(m, &restore);
    let fn_pfx_under = format!("{fn_pfx}_");
    for f in &mut m.funcs {
        if !f.name.eq_ignore_ascii_case(&fn_pfx) && !f.name.to_ascii_lowercase().starts_with(&fn_pfx_under) {
            f.name = format!("{fn_pfx}_{}", f.name);
        }
    }
}

pub fn model_with_refs(xml: &str, refs: &[ProjectRef]) -> Result<Model, Vec<String>> {
    let mut m = super::model::model_with_options(xml, MODEL_OPTIONS)?;
    let pre_struct_names: Vec<String> = m.classes.iter().map(|c| c.name.clone()).chain(m.callbacks.iter().map(|c| c.name.clone())).collect();
    let ref_structs: HashSet<&str> = refs
        .iter()
        .flat_map(|r| r.structs.iter().chain(r.external_structs.iter()).chain(r.struct_aliases.keys()).map(String::as_str))
        .collect();
    let mut opaque_kept: HashSet<String> = m
        .opaque
        .iter()
        .filter(|o| {
            let orig = m.opaque_names.get(*o).map(String::as_str).unwrap_or(o.as_str());
            ref_structs.contains(orig)
        })
        .cloned()
        .collect();
    if let Some(pfx) = effective_prefix(&m) {
        apply_prefix(&mut m, &pfx, &mut opaque_kept);
    }
    let extra_reserved: Vec<String> = refs
        .iter()
        .flat_map(|r| r.all_names.iter())
        .filter(|r| {
            let base = r.strip_prefix("AS_").unwrap_or(r.as_str());
            !opaque_kept.contains(base)
        })
        .cloned()
        .collect();
    super::names::settle_with(&mut m, &NAMING, &extra_reserved);
    for (pre, post) in pre_struct_names.into_iter().zip(m.classes.iter().map(|c| &c.name).chain(m.callbacks.iter().map(|c| &c.name))) {
        if pre != *post {
            m.original_type_names.insert(post.clone(), pre);
        }
    }
    super::wasm::share_imports(&mut m);
    Ok(m)
}

pub const NAMING: Naming = Naming {
    // And what ts_types.h declares for promises.
    reserved: || KEYWORDS.iter().chain(LIBC_NAMES).chain(&["Promise_fn", "Promise_then"]).map(|k| (*k).to_string()).chain(runtime_names()).collect(),
    type_names: |candidate, name, kind, m| {
        let mut ids = vec![candidate.to_string(), format!("AS_{candidate}")];
        match kind {
            Kind::Class => {
                if let Some(c) = m.classes.iter().find(|c| c.name == name) {
                    ids.extend(["fields", "FIELDS", "from_fields"].map(|s| format!("{candidate}_{s}")));
                    if c.iterates.is_some() || c.iterates_async.is_some() {
                        ids.extend(["iterate", "iterate_async", "iterate_next"].map(|s| format!("{candidate}_{s}")));
                    }
                }
            }
            Kind::Callback => ids.extend(["fn", "from", "arg", "arg_self", "arg_fn", "arg_null"].map(|s| format!("{candidate}_{s}"))),
            Kind::Union => {
                ids.extend(["from_handle", "to_handle", "of", "of_self", "of_null", "of_ts_value", "release"].map(|s| format!("{candidate}_{s}")));
                if let Some(u) = m.unions.iter().find(|u| u.name == name) {
                    if u.parts.contains(&Ty::Str) {
                        ids.push(format!("{candidate}_of_cstr"));
                    }
                    for (_, f) in u
                        .parts
                        .iter()
                        .zip(union_fields_for(candidate, u))
                        .filter(|(p, _)| matches!(p, Ty::Num | Ty::Str | Ty::Bool | Ty::Obj(_) | Ty::Enum(_) | Ty::Array | Ty::ArrayOf(_) | Ty::Tuple(_) | Ty::Record(_)))
                    {
                        ids.extend([format!("{candidate}_{f}"), format!("{candidate}_of_{f}")]);
                    }
                }
            }
            Kind::Enum => {
                ids.push(format!("{candidate}_absent"));
                if let Some(e) = m.enums.iter().find(|e| e.0 == name) {
                    ids.extend(e.1.iter().map(|v| format!("{candidate}_{}", super::model::ident(if v.is_empty() { "empty" } else { v }))));
                }
            }
            Kind::NumEnum => {
                if let Some(e) = m.num_enums.iter().find(|e| e.0 == name) {
                    ids.extend(e.1.iter().map(|(member, _)| format!("{candidate}_{member}")));
                }
            }
            Kind::Opaque => {}
        }
        // An enum's members may share a name: the enum numbers them itself.
        ids.sort();
        ids.dedup();
        let own = ids.iter().position(|i| i == candidate).unwrap();
        ids.swap(0, own);
        ids
    },
    values_of: |_, _, _, _| Vec::new(),
    types_apart: false,
    overloads: false,
    func_name: |owner, name, _| Some(match owner {
        Some(o) => format!("{o}_{name}"),
        None => name.to_string(),
    }),
    funcs_flat: true,
    nests: false,
    members: |_, _| Vec::new(),
    semantic_fallbacks: false,
};

pub fn files(m: &Model) -> Result<Vec<(String, String)>, Vec<String>> {
    files_named(m, "bindings", &[])
}

pub fn files_named(m: &Model, pkg_name: &str, refs: &[ProjectRef]) -> Result<Vec<(String, String)>, Vec<String>> {
    emit::files_named(m, pkg_name, refs)
}

// A buffer element as a C type; C++ uses the same.
pub(super) fn c_elem(e: super::model::Elem) -> &'static str {
    use super::model::Elem::*;
    match e {
        I8 => "int8_t",
        U8 => "uint8_t",
        I16 => "int16_t",
        U16 => "uint16_t",
        I32 => "int32_t",
        U32 => "uint32_t",
        I64 => "int64_t",
        U64 => "uint64_t",
        F32 => "float",
        F64 => "double",
    }
}

#[cfg(test)]
mod tests {
    use super::default_c_prefix;

    #[test]
    fn default_prefixes() {
        assert_eq!(default_c_prefix("@babylonjs/lite").as_deref(), Some("babylon"));
        assert_eq!(default_c_prefix("@dimforge/rapier3d-compat").as_deref(), Some("rapier3d"));
        assert_eq!(default_c_prefix("@tanstack/table-core").as_deref(), Some("table"));
        assert_eq!(default_c_prefix("pixi.js").as_deref(), Some("pixi"));
        assert_eq!(default_c_prefix("date-fns").as_deref(), Some("date_fns"));
        assert_eq!(default_c_prefix("exports-lib").as_deref(), Some("exports"));
        assert_eq!(default_c_prefix("pts").as_deref(), Some("pts"));
        assert_eq!(default_c_prefix("katex").as_deref(), Some("katex"));
        assert_eq!(default_c_prefix("marked").as_deref(), Some("marked"));
        assert_eq!(default_c_prefix("echarts").as_deref(), Some("echarts"));
        assert_eq!(default_c_prefix("konva").as_deref(), Some("konva"));
        assert_eq!(default_c_prefix("leaflet").as_deref(), Some("leaflet"));
        assert_eq!(default_c_prefix("three").as_deref(), Some("three"));
    }
}
