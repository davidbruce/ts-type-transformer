// XML -> C header for a wasm32 module. No inheritance, generics, overloads,
// optional parameters or unions: the config must have lowered them all.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::Path;

use super::model::{Call, Class, Func, Model, NamedUnion, Ty};
use super::names::{Kind, Naming, identifiers};

/// A reference to a previously generated C binding directory.
pub struct ProjectRef {
    pub pkg_name: String,
    pub ts_types_include: String,
    pub umbrella_include: String,
    pub structs: HashSet<String>,
    pub external_structs: HashSet<String>,
    pub all_names: Vec<String>,
}

fn is_builtin_c_type(name: &str) -> bool {
    matches!(
        name,
        "Error" | "Date" | "Promise" | "JsValue" | "JsString" | "JsArray" | "JsIterator" | "JsOptionalDouble" | "JsOptionalBool" | "TsStr"
    ) || super::model::buffer_spec(name).is_some()
        || name.starts_with("JsArray_")
        || name.starts_with("JsTuple_")
}

fn collect_c_header_functions(dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            collect_c_header_functions(&p, out);
        } else if p.file_name().and_then(|s| s.to_str()) == Some("functions.h")
            && let Ok(src) = std::fs::read_to_string(&p)
        {
            for line in src.lines().map(str::trim) {
                if let Some(rest) = line.strip_prefix("static inline ")
                    && let Some((head, _)) = rest.split_once('(')
                    && let Some(name) = head.split_whitespace().last()
                    && !name.is_empty()
                {
                    out.push(name.to_string());
                } else if let Some(rest) = line.strip_prefix("#define ")
                    && let Some((name, _)) = rest.split_once('(')
                {
                    let name = name.trim();
                    if !name.is_empty() && !name.contains("__") {
                        out.push(name.to_string());
                    }
                }
            }
        }
    }
}

/// Loads a previously generated C binding directory (or `bindings.h`/`ts_types.h`)
/// so another generated C package can `#include` its `ts_types.h` and reserve
/// its global C identifiers against collisions.
pub fn load_project_ref(dir_or_header: &Path, from_dir: &Path) -> Result<ProjectRef, String> {
    let root_dir = if dir_or_header.is_file() {
        dir_or_header.parent().unwrap_or_else(|| Path::new("."))
    } else {
        dir_or_header
    };
    let types_path = root_dir.join("ts_types.h");
    let src = std::fs::read_to_string(&types_path).map_err(|e| format!("{}: {e}", types_path.display()))?;
    let pkg_name = super::snake_package_name(root_dir.file_name().and_then(|s| s.to_str()).unwrap_or("bindings"));
    let rel_dir = super::relative_path(from_dir, root_dir);
    let ts_types_include = format!("{rel_dir}/ts_types.h");
    let pkg_umbrella = root_dir.join(format!("{pkg_name}.h"));
    let umbrella_include = if pkg_umbrella.is_file() {
        format!("{rel_dir}/{pkg_name}.h")
    } else {
        format!("{rel_dir}/bindings.h")
    };

    let mut structs = HashSet::new();
    let mut external_structs = HashSet::new();
    let mut all_names = Vec::new();
    let mut prev_is_external = false;
    for line in src.lines().map(str::trim) {
        if line == "/* external */" {
            prev_is_external = true;
            continue;
        }
        if let Some(name) = line.strip_prefix("typedef struct { uint32_t h; } ").and_then(|r| r.strip_suffix(';')).map(str::trim) {
            if !is_builtin_c_type(name) {
                if prev_is_external {
                    external_structs.insert(name.to_string());
                } else {
                    structs.insert(name.to_string());
                }
                all_names.push(name.to_string());
            }
        } else if line.starts_with("typedef enum {")
            && let Some((inner, rhs)) = line.strip_prefix("typedef enum {").unwrap_or("").split_once("} ")
            && let Some(name) = rhs.strip_suffix(';').map(str::trim)
            && !is_builtin_c_type(name)
        {
            all_names.push(name.to_string());
            for item in inner.split(',') {
                let id = item.split('=').next().unwrap_or("").trim();
                if !id.is_empty() {
                    all_names.push(id.to_string());
                }
            }
        } else if line.starts_with("typedef struct { int32_t tag;")
            && let Some((_, rhs)) = line.rsplit_once("} ")
            && let Some(name) = rhs.strip_suffix(';').map(str::trim)
            && !is_builtin_c_type(name)
        {
            all_names.push(name.to_string());
        } else if line.starts_with("typedef ")
            && let Some((_, after)) = line.split_once("(*")
            && let Some(fn_name) = after.split(')').next()
            && let Some(name) = fn_name.strip_suffix("_fn")
            && !is_builtin_c_type(name)
        {
            all_names.push(name.to_string());
        }
        prev_is_external = false;
    }
    collect_c_header_functions(root_dir, &mut all_names);
    all_names.sort();
    all_names.dedup();
    Ok(ProjectRef { pkg_name, ts_types_include, umbrella_include, structs, external_structs, all_names })
}

fn import_attr(wasm_module: &str, name: &str) -> String {
    if wasm_module == "env" {
        format!("TS_IMPORT({name})")
    } else {
        format!("TS_IMPORT_MOD(\"{wasm_module}\", {name})")
    }
}

fn raw_fn(raw_prefix: &str, name: &str) -> String {
    format!("ts_raw_{raw_prefix}{name}")
}

thread_local! {
    // The unions of the model being generated that values can be decoded to.
    static DECODABLE: std::cell::RefCell<std::collections::HashSet<String>> = std::cell::RefCell::new(Default::default());
}

fn decodable(n: &str) -> bool {
    DECODABLE.with(|d| d.borrow().contains(n))
}

const T: &str = "C";

const KEYWORDS: &[&str] = &[
    "auto", "bool", "break", "case", "char", "const", "continue", "default", "do", "double", "else", "enum", "extern", "false", "float", "for",
    "goto", "if", "inline", "int", "long", "register", "restrict", "return", "self", "short", "signed", "sizeof", "static", "struct", "switch",
    "true", "typedef", "union", "unsigned", "void", "volatile", "while",
];

const LIBC_NAMES: &[&str] = &[
    "abort", "abs", "acos", "asin", "atan", "atan2", "atof", "atoi", "atol", "bsearch", "calloc", "ceil", "clock",
    "cos", "cosh", "div", "exit", "exp", "fabs", "fclose", "fflush", "floor", "fmod", "fopen", "fprintf", "fputc",
    "fputs", "fread", "free", "freopen", "fwrite", "getchar", "getenv", "hypot", "labs", "ldexp", "log", "log10",
    "log2", "malloc", "memchr", "memcmp", "memcpy", "memmove", "memset", "perror", "pow", "printf", "putchar",
    "puts", "qsort", "rand", "realloc", "remove", "rename", "round", "sin", "sinh", "snprintf", "sprintf", "sqrt",
    "srand", "sscanf", "strcat", "strchr", "strcmp", "strcpy", "strlen", "strncat", "strncmp", "strncpy", "strrchr",
    "strstr", "strtod", "strtol", "strtoul", "system", "tan", "tanh", "time", "tmpfile", "trunc", "vsnprintf",
    "vsprintf",
];

fn ident(name: &str) -> String {
    if KEYWORDS.contains(&name) { format!("{name}_") } else { name.to_string() }
}

// C has one namespace: a type, its helpers (`AS_Shape`, `Id_from_handle`,
// `Mode_fill`) and every function (`Shape_area`) are identifiers in it.
pub const NAMING: Naming = Naming {
    reserved: || KEYWORDS.iter().chain(LIBC_NAMES).map(|k| (*k).to_string()).chain(identifiers(RUNTIME)).collect(),
    type_names: |candidate, name, kind, m| {
        let mut ids = vec![candidate.to_string(), format!("AS_{candidate}")];
        match kind {
            Kind::Class => {
                if let Some(c) = m.classes.iter().find(|c| c.name == name) {
                    ids.extend(c.ancestors.iter().map(|a| format!("{candidate}_as_{a}")));
                    if c.iterates.is_some() || c.iterates_async.is_some() {
                        ids.extend(["iterate", "iterate_async", "iterate_next"].map(|s| format!("{candidate}_{s}")));
                    }
                }
            }
            Kind::Callback => ids.extend(["fn", "from"].map(|s| format!("{candidate}_{s}"))),
            Kind::Union => {
                ids.extend(["from_handle", "to_handle", "of"].map(|s| format!("{candidate}_{s}")));
                if let Some(u) = m.unions.iter().find(|u| u.name == name) {
                    if u.parts.contains(&Ty::Str) {
                        ids.push(format!("{candidate}_of_cstr"));
                    }
                    for p in u.parts.iter().filter(|p| matches!(p, Ty::Num | Ty::Str | Ty::Bool | Ty::Obj(_) | Ty::Enum(_) | Ty::Array | Ty::ArrayOf(_) | Ty::Tuple(_))) {
                        let f = union_field(p);
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
};

fn raw(t: &Ty) -> &'static str {
    match t {
        Ty::Void => "void",
        Ty::Num | Ty::NumEnum(_) => "double",
        Ty::Bool | Ty::Enum(_) => "int32_t",
        // An optional flag or enum is -1 when absent.
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Bool | Ty::Enum(_)) => "int32_t",
        _ => "uint32_t",
    }
}

fn typed(t: &Ty, param: bool) -> String {
    match t {
        Ty::Void => "void".into(),
        Ty::Num => "double".into(),
        Ty::Bool => "bool".into(),
        Ty::Str if param => "const char*".into(),
        Ty::Str => "JsString".into(),
        Ty::Any => "JsValue".into(),
        Ty::Array => "JsArray".into(),
        Ty::ArrayOf(e) => format!("JsArray_{}", element(e)),
        Ty::Tuple(es) => format!("JsTuple_{}", es.iter().map(element).collect::<Vec<_>>().join("_")),
        Ty::Obj(n) | Ty::Enum(n) | Ty::NumEnum(n) => n.clone(),
        // An optional number or flag says whether it is there; an optional enum is `E_absent` when it is not.
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Num | Ty::NumEnum(_)) => "JsOptionalDouble".into(),
        Ty::Nullable(inner, _) if **inner == Ty::Bool => "JsOptionalBool".into(),
        // NULL, or a null handle, is the absent value.
        Ty::Nullable(inner, _) => typed(inner, param),
        // A union a handle can be decoded to: a tagged struct. Any other is a value.
        Ty::NamedUnion(n) if decodable(n) => n.clone(),
        Ty::NamedUnion(_) => "JsValue".into(),
        Ty::Union(_) => panic!("{T} generator cannot express a union: the target config must name or erase unions"),
        Ty::Key(_) | Ty::Mapped(_) | Ty::Listener(_) => panic!("{T} generator cannot express keyed generics: the target config must expand them"),
        Ty::Param(_) | Ty::Generic(..) => panic!("{T} generator cannot express generics: the target config must erase them"),
    }
}

// An array's element type, as a name: `double`, `JsString`, `Element`, `JsArray_double`.
fn element(t: &Ty) -> String {
    match t {
        Ty::Str => "JsString".into(),
        t => typed(t, false),
    }
}

// The typed arrays and tuples a type names, innermost first.
fn arrays(t: &Ty, out: &mut Vec<Ty>) {
    match t {
        Ty::ArrayOf(e) => arrays(e, out),
        Ty::Tuple(es) => es.iter().for_each(|e| arrays(e, out)),
        Ty::Nullable(inner, _) => return arrays(inner, out),
        _ => return,
    }
    if !out.contains(t) {
        out.push(t.clone());
    }
}

// A union's member name: `number`, `string`, `Element`, `array`.
fn union_field(t: &Ty) -> String {
    match t {
        Ty::Num => "number".into(),
        Ty::Str => "string".into(),
        Ty::Bool => "boolean".into(),
        Ty::Obj(n) | Ty::Enum(n) => n.clone(),
        Ty::Array | Ty::ArrayOf(_) | Ty::Tuple(_) => "array".into(),
        t => panic!("{T} generator: a union member it cannot hold: {t:?}"),
    }
}

// A union: a struct with a tag and its members, tag constants, and
// conversions to and from handles: `Id_from_handle` takes a handle a function
// returned; `Id_to_handle` gives one the caller releases.
fn union_type(u: &NamedUnion, m: &Model, wasm_module: &str, raw_prefix: &str) -> (String, String) {
    // An enum of strings crosses as a handle by its place among the enums.
    let enum_id = |e: &str| m.enums.iter().position(|x| x.0 == e).unwrap_or(0);
    let n = &u.name;
    let has_str = u.parts.contains(&Ty::Str);
    let has_num = u.parts.contains(&Ty::Num);
    let mut fields: Vec<String> = u.parts.iter().map(|p| format!("{} {};", if *p == Ty::Str { "JsString".to_string() } else { typed(p, false) }, union_field(p))).collect();
    if has_str {
        fields.push("const char* ts_cstr;".into());
    }
    let tags: Vec<String> = u.parts.iter().enumerate().map(|(i, p)| format!("{n}_{} = {i}", union_field(p))).collect();
    let decl = format!(
        "// {n}: `tag` says which member it holds; -1 is none.\ntypedef struct {{ int32_t tag; union {{ {} }} as; }} {n};\nenum {{ {} }};\n",
        fields.join(" "),
        tags.join(", ")
    );
    let enum_value_fn = raw_fn(raw_prefix, "js_enum_value");
    let enum_index_fn = raw_fn(raw_prefix, "js_enum_index");
    let mut to: String = u
        .parts
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let f = union_field(p);
            match p {
                Ty::Num => format!(" case {i}: return ts_raw_js_from_number(u.as.{f});"),
                Ty::Bool => format!(" case {i}: return ts_raw_js_from_bool(u.as.{f});"),
                Ty::Enum(e) => format!(" case {i}: return {enum_value_fn}({}, u.as.{f});", enum_id(e)),
                _ => format!(" case {i}: ts_raw_js_retain(u.as.{f}.h); return u.as.{f}.h;"),
            }
        })
        .collect();
    if has_str {
        to.push_str(" case -2: return u.as.ts_cstr ? ts_raw_js_from_string(u.as.ts_cstr, ts_strlen(u.as.ts_cstr)) : 0;");
    }
    let from: String = u
        .parts
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let f = union_field(p);
            match p {
                Ty::Num => format!(" case {i}: u.as.{f} = ts_raw_js_to_number(h); ts_raw_js_release(h); break;"),
                Ty::Bool => format!(" case {i}: u.as.{f} = ts_raw_js_to_bool(h) != 0; ts_raw_js_release(h); break;"),
                Ty::Str => format!(" case {i}: u.as.{f} = (JsString){{ h }}; break;"),
                Ty::Enum(e) => format!(" case {i}: u.as.{f} = ({e}){enum_index_fn}({}, h); ts_raw_js_release(h); break;", enum_id(e)),
                p => format!(" case {i}: u.as.{f} = ({}){{ h }}; break;", typed(p, false)),
            }
        })
        .collect();
    let tag_attr = import_attr(wasm_module, &format!("{n}_tag"));
    let tag_raw = raw_fn(raw_prefix, &format!("{n}_tag"));
    let functions = format!(
        "{tag_attr} int32_t {tag_raw}(uint32_t, int32_t);\n\
         static inline uint32_t {n}_to_handle({n} u) {{ switch (u.tag) {{{to} }} return 0; }}\n\
         static inline {n} {n}_from_handle(uint32_t h) {{ {n} u; u.tag = {tag_raw}(h, 0); switch (u.tag) {{{from} }} return u; }}\n"
    );
    // Made from a member: `Value_of_number(42)`, or by its type, `Value_of(widget)`.
    let field_type = |p: &Ty| if *p == Ty::Str { "JsString".to_string() } else { typed(p, false) };
    let mut functions = functions;
    for p in &u.parts {
        let f = union_field(p);
        functions += &format!("static inline {n} {n}_of_{f}({} v) {{ return ({n}){{ .tag = {n}_{f}, .as.{f} = v }}; }}\n", field_type(p));
    }
    if has_str {
        functions += &format!("static inline {n} {n}_of_cstr(const char* v) {{ return ({n}){{ .tag = -2, .as.ts_cstr = v }}; }}\n");
    }
    let mut seen = std::collections::BTreeSet::new();
    let choices: Option<Vec<String>> = u
        .parts
        .iter()
        .map(|p| {
            if *p == Ty::Str {
                let pairs = [("JsString", "string"), ("const char*", "cstr"), ("char*", "cstr")];
                pairs
                    .iter()
                    .all(|(t, _)| seen.insert((*t).to_string()))
                    .then(|| pairs.iter().map(|(t, suffix)| format!("{t}: {n}_of_{suffix}")).collect::<Vec<_>>().join(", "))
            } else {
                let types = generic_types(p, has_num)?;
                types.iter().all(|t| seen.insert(t.clone())).then(|| types.iter().map(|t| format!("{t}: {n}_of_{}", union_field(p))).collect::<Vec<_>>().join(", "))
            }
        })
        .collect();
    if let Some(choices) = choices {
        functions += &format!("#define {n}_of(x) _Generic((x), {})(x)\n", choices.join(", "));
    }
    (decl, functions)
}

// A tuple: `JsTuple_double_JsString`, made by `_new(1, "one")`, read by `_get_0`.
// What a getter returns is the caller's to release; a number or flag is read and released.
fn tuple_type(es: &[Ty]) -> String {
    let name = typed(&Ty::Tuple(es.to_vec()), false);
    let params: Vec<String> = es.iter().enumerate().map(|(i, e)| format!("{} v{i}", if *e == Ty::Str { "TsStr".into() } else { typed(e, false) })).collect();
    let pushes: String = es
        .iter()
        .enumerate()
        .map(|(i, e)| match e {
            Ty::Num => format!(" {{ uint32_t h = ts_raw_js_from_number(v{i}); ts_raw_js_array_push(a.h, h); ts_raw_js_release(h); }}"),
            Ty::Bool => format!(" {{ uint32_t h = ts_raw_js_from_bool(v{i}); ts_raw_js_array_push(a.h, h); ts_raw_js_release(h); }}"),
            Ty::Str => format!(" {{ uint32_t h = ts_raw_js_from_string(v{i}.ptr, v{i}.len); ts_raw_js_array_push(a.h, h); ts_raw_js_release(h); }}"),
            // A union is a tagged struct: made a value, pushed, and released.
            Ty::NamedUnion(n) if decodable(n) => format!(" {{ uint32_t h = {n}_to_handle(v{i}); ts_raw_js_array_push(a.h, h); ts_raw_js_release(h); }}"),
            _ => format!(" ts_raw_js_array_push(a.h, v{i}.h);"),
        })
        .collect();
    let mut out = format!("typedef struct {{ uint32_t h; }} {name};\nstatic inline {name} {name}_new({}) {{ {name} a = {{ ts_raw_js_array_new() }};{pushes} return a; }}\n", params.join(", "));
    if es.contains(&Ty::Str) {
        let pnames: Vec<String> = (0..es.len()).map(|i| format!("v{i}")).collect();
        let pargs: Vec<String> = es.iter().enumerate().map(|(i, e)| if *e == Ty::Str { format!("TS_STR(v{i})") } else { format!("(v{i})") }).collect();
        out += &format!("#define {name}_new({}) ({name}_new)({})\n", pnames.join(", "), pargs.join(", "));
    }
    for (i, e) in es.iter().enumerate() {
        let elem = typed(e, false);
        let get = match e {
            Ty::Num => "double x = ts_raw_js_to_number(h); ts_raw_js_release(h); return x;".to_string(),
            Ty::Bool => "bool x = ts_raw_js_to_bool(h) != 0; ts_raw_js_release(h); return x;".to_string(),
            Ty::NamedUnion(n) if decodable(n) => format!("return {n}_from_handle(h);"),
            _ => format!("return ({elem}){{ h }};"),
        };
        out += &format!("static inline {elem} {name}_get_{i}({name} a) {{ uint32_t h = ts_raw_js_array_get(a.h, {i}); {get} }}\n");
    }
    out + &format!("static inline JsArray {name}_any({name} a) {{ return (JsArray){{ a.h }}; }}\n")
}

// A typed array: `JsArray_double`, with `_new`, `_length`, `_get`, `_push` and `_from`.
// What `_get` returns is the caller's to release; a number or flag is read and released.
fn array_type(e: &Ty) -> String {
    let (name, elem) = (format!("JsArray_{}", element(e)), typed(e, false));
    if *e == Ty::Str {
        return [
            format!("typedef struct {{ uint32_t h; }} {name};"),
            format!("static inline {name} {name}_new(void) {{ return ({name}){{ ts_raw_js_array_new() }}; }}"),
            format!("static inline uint32_t {name}_length({name} a) {{ return ts_raw_js_array_length(a.h); }}"),
            format!("static inline {elem} {name}_get({name} a, uint32_t i) {{ uint32_t h = ts_raw_js_array_get(a.h, i); return (JsString){{ h }}; }}"),
            format!("static inline void {name}_push({name} a, TsStr v) {{ uint32_t h = ts_raw_js_from_string(v.ptr, v.len); ts_raw_js_array_push(a.h, h); ts_raw_js_release(h); }}"),
            format!("static inline {name} {name}_from(const char* const* items, uint32_t len) {{ {name} a = {name}_new(); for (uint32_t i = 0; i < len; i++) {{ uint32_t h = ts_raw_js_from_string(items[i], items[i] ? ts_strlen(items[i]) : 0); ts_raw_js_array_push(a.h, h); ts_raw_js_release(h); }} return a; }}"),
            format!("#define {name}_push(a, v) {name}_push((a), TS_STR(v))"),
            format!("static inline JsArray {name}_any({name} a) {{ return (JsArray){{ a.h }}; }}"),
        ]
        .map(|l| l + "\n")
        .concat();
    }
    let (get, push_param, push) = match e {
        Ty::NamedUnion(n) if decodable(n) => (format!("return {n}_from_handle(h);"), format!("{n} v"), format!("uint32_t h = {n}_to_handle(v); ts_raw_js_array_push(a.h, h); ts_raw_js_release(h);")),
        Ty::Num => ("double x = ts_raw_js_to_number(h); ts_raw_js_release(h); return x;".to_string(), "double v".to_string(), "uint32_t h = ts_raw_js_from_number(v); ts_raw_js_array_push(a.h, h); ts_raw_js_release(h);".to_string()),
        Ty::Bool => ("bool x = ts_raw_js_to_bool(h) != 0; ts_raw_js_release(h); return x;".to_string(), "bool v".to_string(), "uint32_t h = ts_raw_js_from_bool(v); ts_raw_js_array_push(a.h, h); ts_raw_js_release(h);".to_string()),
        _ => (format!("return ({elem}){{ h }};"), format!("{elem} v"), "ts_raw_js_array_push(a.h, v.h);".to_string()),
    };
    [
        format!("typedef struct {{ uint32_t h; }} {name};"),
        format!("static inline {name} {name}_new(void) {{ return ({name}){{ ts_raw_js_array_new() }}; }}"),
        format!("static inline uint32_t {name}_length({name} a) {{ return ts_raw_js_array_length(a.h); }}"),
        format!("static inline {elem} {name}_get({name} a, uint32_t i) {{ uint32_t h = ts_raw_js_array_get(a.h, i); {get} }}"),
        format!("static inline void {name}_push({name} a, {push_param}) {{ {push} }}"),
        format!("static inline {name} {name}_from(const {elem}* items, uint32_t len) {{ {name} a = {name}_new(); for (uint32_t i = 0; i < len; i++) {name}_push(a, items[i]); return a; }}"),
        // The same array, untyped.
        format!("static inline JsArray {name}_any({name} a) {{ return (JsArray){{ a.h }}; }}"),
    ]
    .map(|l| l + "\n")
    .concat()
}

fn func_param_ty(t: &Ty) -> String {
    if *t == Ty::Str || matches!(t, Ty::Nullable(inner, _) if **inner == Ty::Str) {
        "TsStr".into()
    } else {
        typed(t, true)
    }
}

fn func(out: &mut String, owner: Option<&str>, f: &Func, wasm_module: &str, raw_prefix: &str) {
    let mut raw_params = Vec::new();
    let mut params = Vec::new();
    let mut args = Vec::new();
    if Class::has_self(f) {
        raw_params.push("uint32_t".to_string());
        params.push(format!("{} self", owner.unwrap()));
        args.push("self.h".to_string());
    }
    // A union argument is lent as a handle, released after the call.
    let mut lent = Vec::new();
    for (name, t) in &f.params {
        let name = ident(name);
        params.push(format!("{} {name}", func_param_ty(t)));
        let union = match t {
            Ty::NamedUnion(n) => Some(n),
            Ty::Nullable(inner, _) => match &**inner {
                Ty::NamedUnion(n) => Some(n),
                _ => None,
            },
            _ => None,
        };
        if let Some(n) = union.filter(|n| decodable(n)) {
            raw_params.push("uint32_t".into());
            args.push(format!("lent_{name}"));
            lent.push(format!("uint32_t lent_{name} = {n}_to_handle({name});"));
            continue;
        }
        match t {
            Ty::Str => {
                raw_params.extend(["const char*".to_string(), "uint32_t".to_string()]);
                args.extend([format!("{name}.ptr"), format!("{name}.len")]);
            }
            Ty::Nullable(inner, _) if **inner == Ty::Str => {
                raw_params.extend(["const char*".to_string(), "uint32_t".to_string()]);
                args.extend([format!("{name}.ptr"), format!("{name}.len")]);
            }
            Ty::Num | Ty::Bool | Ty::Enum(_) => {
                raw_params.push(raw(t).into());
                args.push(name);
            }
            Ty::NumEnum(_) => {
                raw_params.push(raw(t).into());
                args.push(format!("(double){name}"));
            }
            Ty::Nullable(inner, _) if matches!(**inner, Ty::Num | Ty::NumEnum(_)) => {
                raw_params.extend(["int32_t".to_string(), "double".to_string()]);
                args.extend([format!("{name}.has"), format!("{name}.value")]);
            }
            Ty::Nullable(inner, _) if **inner == Ty::Bool => {
                raw_params.push("int32_t".into());
                args.push(format!("({name}.has ? ({name}.value ? 1 : 0) : -1)"));
            }
            Ty::Nullable(inner, _) if matches!(**inner, Ty::Enum(_)) => {
                raw_params.push("int32_t".into());
                args.push(format!("(int32_t){name}"));
            }
            _ => {
                raw_params.push(raw(t).into());
                args.push(format!("{name}.h"));
            }
        }
    }
    let void = |v: &Vec<String>| if v.is_empty() { "void".to_string() } else { v.join(", ") };
    let name = match owner {
        Some(o) => format!("{o}_{}", f.name),
        None => f.name.clone(),
    };
    // A shared import takes the index of this function's fixed arguments after `self`.
    let import = match &f.shared {
        Some((shared, k)) => {
            let at = usize::from(Class::has_self(f));
            raw_params.insert(at, "int32_t".into());
            args.insert(at, k.to_string());
            shared.as_str()
        }
        None => f.import.as_str(),
    };
    let call_raw = raw_fn(raw_prefix, import);
    let mut call = format!("{call_raw}({})", args.join(", "));
    // With lent arguments: the call, their release, then the result.
    let mut pre = lent.concat();
    if !lent.is_empty() {
        let release: String = lent.iter().map(|l| format!(" ts_raw_js_release({});", l.split(' ').nth(1).unwrap())).collect();
        if f.ret == Ty::Void {
            pre += &format!(" {call};{release}");
            call = String::new();
        } else {
            pre += &format!(" {} raw_result = {call};{release}", raw(&f.ret));
            call = "raw_result".into();
        }
    }
    let union_ret = match &f.ret {
        Ty::NamedUnion(n) => Some((n, false)),
        Ty::Nullable(inner, _) => match &**inner {
            Ty::NamedUnion(n) => Some((n, true)),
            _ => None,
        },
        _ => None,
    };
    let body = match &f.ret {
        Ty::Void if call.is_empty() => String::new(),
        _ if union_ret.is_some_and(|(n, _)| decodable(n)) => {
            let (n, optional) = union_ret.unwrap();
            if optional { format!("uint32_t h = {call}; if (h) return {n}_from_handle(h); {n} none = {{ .tag = -1 }}; return none;") } else { format!("return {n}_from_handle({call});") }
        }
        Ty::Void => format!("{call};"),
        Ty::Num => format!("return {call};"),
        Ty::Bool => format!("return {call} != 0;"),
        Ty::Enum(e) => format!("return ({e}){call};"),
        Ty::NumEnum(e) => format!("return ({e})(int32_t){call};"),
        // An optional number comes back as a handle, read and released.
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Num | Ty::NumEnum(_)) => {
            format!("uint32_t h = {call}; JsOptionalDouble r = {{ h != 0, h ? ts_raw_js_to_number(h) : 0 }}; if (h) ts_raw_js_release(h); return r;")
        }
        Ty::Nullable(inner, _) if **inner == Ty::Bool => format!("int32_t r = {call}; return (JsOptionalBool){{ r >= 0, r > 0 }};"),
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Enum(_)) => format!("return ({}){call};", typed(inner, false)),
        t => format!("return ({}){{ {call} }};", typed(t, false)),
    };
    let decl = format!("{} {} {call_raw}({});\n", import_attr(wasm_module, import), raw(&f.ret), void(&raw_params));
    // Declared once per header, though several functions use it.
    // Only a shared import can repeat; checking others would be quadratic.
    if f.shared.is_none() || !out.contains(&decl) {
        *out += &decl;
    }
    *out += &format!("static inline {} {name}({}) {{ {pre}{body} }}\n", typed(&f.ret, false), void(&params));
}

const RUNTIME: &str = "// Generated.
#pragma once
#ifndef TS_RUNTIME_H
#define TS_RUNTIME_H
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#define TS_IMPORT_MOD(mod, name) __attribute__((import_module(mod), import_name(#name)))
#define TS_IMPORT(name) TS_IMPORT_MOD(\"env\", name)

// `name__N`, by the number of arguments: how trailing optional arguments are left out.
// An empty call counts as none: `orUnit()` (C23, or C11 without -pedantic).
#define TS_COUNT(...) TS_COUNT_(0 __VA_OPT__(,) __VA_ARGS__, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0)
#define TS_COUNT_(z, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13, a14, a15, n, ...) n
#define TS_PASTE(name, n) TS_PASTE_(name, n)
#define TS_PASTE_(name, n) name##__##n
#define TS_CALL(name, ...) TS_PASTE(name, TS_COUNT(__VA_ARGS__))(__VA_ARGS__)

// Handles to JavaScript values. Zero is null.
typedef struct { uint32_t h; } JsValue;
typedef struct { uint32_t h; } JsString;
typedef struct { uint32_t h; } JsArray;
static inline uint32_t ts_strlen(const char* s) { uint32_t n = 0; while (s[n]) n++; return n; }
typedef struct { const char* ptr; uint32_t len; } TsStr;
static inline TsStr ts_str_id(TsStr s) { return s; }
static inline TsStr ts_str_c(const char* s) { return (TsStr){ s, s ? ts_strlen(s) : 0 }; }
static inline TsStr ts_str_js(JsString s) { return (TsStr){ (const char*)(uintptr_t)s.h, 0xFFFFFFFFu }; }
#define TS_STR(x) _Generic((x), TsStr: ts_str_id, JsString: ts_str_js, default: ts_str_c)(x)
TS_IMPORT(js_string_len) uint32_t ts_raw_js_string_len(uint32_t);
TS_IMPORT(js_string_read) void ts_raw_js_string_read(uint32_t, char*);
TS_IMPORT(js_from_number) uint32_t ts_raw_js_from_number(double);
TS_IMPORT(js_from_bool) uint32_t ts_raw_js_from_bool(int32_t);
TS_IMPORT(js_from_string) uint32_t ts_raw_js_from_string(const char*, uint32_t);
TS_IMPORT(js_to_number) double ts_raw_js_to_number(uint32_t);
TS_IMPORT(js_to_bool) int32_t ts_raw_js_to_bool(uint32_t);
TS_IMPORT(js_equals) int32_t ts_raw_js_equals(uint32_t, uint32_t);
TS_IMPORT(js_array_new) uint32_t ts_raw_js_array_new(void);
TS_IMPORT(js_array_length) uint32_t ts_raw_js_array_length(uint32_t);
TS_IMPORT(js_array_get) uint32_t ts_raw_js_array_get(uint32_t, uint32_t);
TS_IMPORT(js_array_push) void ts_raw_js_array_push(uint32_t, uint32_t);
TS_IMPORT(js_object_new) uint32_t ts_raw_js_object_new(void);
TS_IMPORT(js_object_get) uint32_t ts_raw_js_object_get(uint32_t, const char*, uint32_t);
TS_IMPORT(js_object_set) void ts_raw_js_object_set(uint32_t, const char*, uint32_t, uint32_t);
TS_IMPORT(js_object_has) int32_t ts_raw_js_object_has(uint32_t, const char*, uint32_t);
TS_IMPORT(js_retain) void ts_raw_js_retain(uint32_t);
TS_IMPORT(js_release) void ts_raw_js_release(uint32_t);
TS_IMPORT(js_live) uint32_t ts_raw_js_live(void);
TS_IMPORT(js_callback_release) uint32_t ts_raw_js_callback_release(uint32_t);

// An optional number or flag: `has` says whether `value` is there.
typedef struct { bool has; double value; } JsOptionalDouble;
typedef struct { bool has; bool value; } JsOptionalBool;
static inline JsOptionalDouble js_some_double(double x) { return (JsOptionalDouble){ true, x }; }
static inline JsOptionalBool js_some_bool(bool x) { return (JsOptionalBool){ true, x }; }
static inline JsOptionalDouble js_none_double(void) { return (JsOptionalDouble){ false, 0 }; }
static inline JsOptionalBool js_none_bool(void) { return (JsOptionalBool){ false, false }; }
static inline JsOptionalDouble ts_opt_double_id(JsOptionalDouble x) { return x; }
static inline JsOptionalBool ts_opt_bool_id(JsOptionalBool x) { return x; }
#define TS_OPT_DOUBLE(x) _Generic((x), JsOptionalDouble: ts_opt_double_id, default: js_some_double)(x)
#define TS_OPT_BOOL(x) _Generic((x), JsOptionalBool: ts_opt_bool_id, default: js_some_bool)(x)

// A handle a function returns is the caller's: release it when done with it.
// Arguments, and a callback's arguments, are lent: retain one to keep it.
// What a callback returns is given to JavaScript: retain a handle to return it and keep it.
#define js_release(x) ts_raw_js_release((x).h)
#define js_retain(x) ts_raw_js_retain((x).h)
TS_IMPORT(js_take_error) uint32_t ts_raw_js_take_error(void);
TS_IMPORT(js_error_message) uint32_t ts_raw_js_error_message(uint32_t);
TS_IMPORT(js_error_name) uint32_t ts_raw_js_error_name(uint32_t);
TS_IMPORT(js_error_stack) uint32_t ts_raw_js_error_stack(uint32_t);
TS_IMPORT(js_error_new) uint32_t ts_raw_js_error_new(const char*, uint32_t);
TS_IMPORT(js_date_new) uint32_t ts_raw_js_date_new(double);
TS_IMPORT(js_date_now) double ts_raw_js_date_now(void);
TS_IMPORT(js_date_time) double ts_raw_js_date_time(uint32_t);
TS_IMPORT(js_date_to_iso_string) uint32_t ts_raw_js_date_to_iso_string(uint32_t);
TS_IMPORT(js_bytes_len) uint32_t ts_raw_js_bytes_len(uint32_t);
TS_IMPORT(js_bytes_read) uint32_t ts_raw_js_bytes_read(uint32_t, uint32_t, void*, uint32_t);
TS_IMPORT(js_bytes_write) uint32_t ts_raw_js_bytes_write(uint32_t, uint32_t, const void*, uint32_t);
TS_IMPORT(js_bytes_new) uint32_t ts_raw_js_bytes_new(uint32_t, const void*, uint32_t);
TS_IMPORT(js_bytes_buffer) uint32_t ts_raw_js_bytes_buffer(uint32_t);
TS_IMPORT(js_bytes_offset) uint32_t ts_raw_js_bytes_offset(uint32_t);
TS_IMPORT(js_has_error) int32_t ts_raw_js_has_error(void);
// Whether two handles are one JavaScript value.
static inline bool ts_same(uint32_t a, uint32_t b) { return a == b || ts_raw_js_equals(a, b) != 0; }
#define js_same(a, b) ts_same((a).h, (b).h)
// Whether the last call threw: its exception waits to be taken.
static inline bool js_failed(void) { return ts_raw_js_has_error() != 0; }
// The exception a call threw, taken, or a null handle. A call that throws
// returns a zero value, and an exception not taken by the time control
// returns to JavaScript is reported there.
static inline JsValue js_take_error(void) { return (JsValue){ ts_raw_js_take_error() }; }
// An exception's message, name or stack, the caller's to release.
static inline JsString js_error_message(JsValue e) { return (JsString){ ts_raw_js_error_message(e.h) }; }
static inline JsString js_error_name(JsValue e) { return (JsString){ ts_raw_js_error_name(e.h) }; }
static inline JsString js_error_stack(JsValue e) { return (JsString){ ts_raw_js_error_stack(e.h) }; }
static inline JsValue js_error_new(const char* message) { return (JsValue){ ts_raw_js_error_new(message, ts_strlen(message)) }; }
static inline double js_date_now(void) { return ts_raw_js_date_now(); }
// A JavaScript iterator: see `X_iterate`.
typedef struct { uint32_t h; } JsIterator;
TS_IMPORT(js_iterate) uint32_t ts_raw_js_iterate(uint32_t);
TS_IMPORT(js_iterator_next) uint32_t ts_raw_js_iterator_next(uint32_t);
TS_IMPORT(js_async_iterate) uint32_t ts_raw_js_async_iterate(uint32_t);
TS_IMPORT(js_async_iterator_next) uint32_t ts_raw_js_async_iterator_next(uint32_t);
// A scope: every handle a function returns while it is the innermost open
// scope is released when it closes. Retain one to keep it.
TS_IMPORT(js_scope_open) void js_scope_open(void);
TS_IMPORT(js_scope_close) void js_scope_close(void);
// How many JavaScript values wasm holds handles to.
static inline uint32_t js_live_handles(void) { return ts_raw_js_live(); }
// Forgets a callback made with `X_from`, and returns its context. Its handle is released as any other.
#define js_forget_callback(x) ((void*)(uintptr_t)ts_raw_js_callback_release((x).h))
static inline uint32_t JsString_len(JsString s) { return ts_raw_js_string_len(s.h); }
static inline void JsString_read(JsString s, char* buf) { ts_raw_js_string_read(s.h, buf); }
TS_IMPORT(js_string_copy) uint32_t ts_raw_js_string_copy(uint32_t, char*, uint32_t);
TS_IMPORT(js_string_equals) int32_t ts_raw_js_string_equals(uint32_t, const char*, uint32_t);
// Copies at most cap - 1 bytes and a terminating NUL; returns the string's whole length in bytes.
static inline uint32_t JsString_copy(JsString s, char* buf, uint32_t cap) {
    if (cap == 0) return JsString_len(s);
    uint32_t n = ts_raw_js_string_copy(s.h, buf, cap - 1);
    buf[n < cap - 1 ? n : cap - 1] = 0;
    return n;
}
// Compares in JavaScript, without copying.
static inline bool ts_string_equals(JsString s, TsStr text) { return ts_raw_js_string_equals(s.h, text.ptr, text.len) != 0; }
#define JsString_equals(s, text) ts_string_equals((s), TS_STR(text))
static inline JsValue js_number(double x) { return (JsValue){ ts_raw_js_from_number(x) }; }
static inline JsValue js_bool(bool x) { return (JsValue){ ts_raw_js_from_bool(x) }; }
static inline JsValue js_string(const char* s) { return (JsValue){ ts_raw_js_from_string(s, ts_strlen(s)) }; }
static inline JsString JsString_from(const char* s) { return (JsString){ ts_raw_js_from_string(s, ts_strlen(s)) }; }
static inline double js_to_number(JsValue v) { return ts_raw_js_to_number(v.h); }
static inline bool js_to_bool(JsValue v) { return ts_raw_js_to_bool(v.h) != 0; }
static inline JsString js_to_string(JsValue v) { return (JsString){ v.h }; }
static inline bool js_equals(JsValue a, JsValue b) { return ts_raw_js_equals(a.h, b.h) != 0; }
static inline JsArray JsArray_new(void) { return (JsArray){ ts_raw_js_array_new() }; }
static inline uint32_t JsArray_length(JsArray a) { return ts_raw_js_array_length(a.h); }
static inline JsValue JsArray_get(JsArray a, uint32_t i) { return (JsValue){ ts_raw_js_array_get(a.h, i) }; }
static inline void JsArray_push(JsArray a, JsValue v) { ts_raw_js_array_push(a.h, v.h); }
static inline JsValue js_object(void) { return (JsValue){ ts_raw_js_object_new() }; }
static inline JsValue js_object_get(JsValue o, const char* key) { return (JsValue){ ts_raw_js_object_get(o.h, key, ts_strlen(key)) }; }
static inline void js_object_set(JsValue o, const char* key, JsValue v) { ts_raw_js_object_set(o.h, key, ts_strlen(key), v.h); }
static inline bool js_object_has(JsValue o, const char* key) { return ts_raw_js_object_has(o.h, key, ts_strlen(key)) != 0; }
static inline JsArray js_to_array(JsValue v) { return (JsArray){ v.h }; }
#define js_value(x) ((JsValue){ (x).h })
static inline JsValue ts_get_prop(uint32_t o, const char* k) { return (JsValue){ ts_raw_js_object_get(o, k, ts_strlen(k)) }; }
static inline void ts_set_str(uint32_t o, const char* k, TsStr v) { uint32_t h = ts_raw_js_from_string(v.ptr, v.len); ts_raw_js_object_set(o, k, ts_strlen(k), h); ts_raw_js_release(h); }
static inline void ts_set_num(uint32_t o, const char* k, double v) { uint32_t h = ts_raw_js_from_number(v); ts_raw_js_object_set(o, k, ts_strlen(k), h); ts_raw_js_release(h); }
static inline void ts_set_bool(uint32_t o, const char* k, bool v) { uint32_t h = ts_raw_js_from_bool(v); ts_raw_js_object_set(o, k, ts_strlen(k), h); ts_raw_js_release(h); }
static inline void ts_set_val(uint32_t o, const char* k, uint32_t v) { ts_raw_js_object_set(o, k, ts_strlen(k), v); }
#define js_get_prop(o, k) ts_get_prop((o).h, (k))
#define js_set_str(o, k, v) ts_set_str((o).h, (k), TS_STR(v))
#define js_set_num(o, k, v) ts_set_num((o).h, (k), (v))
#define js_set_bool(o, k, v) ts_set_bool((o).h, (k), (v))
#define js_set_val(o, k, v) ts_set_val((o).h, (k), (v).h)
static inline void ts_push_str(uint32_t a, TsStr v) { uint32_t h = ts_raw_js_from_string(v.ptr, v.len); ts_raw_js_array_push(a, h); ts_raw_js_release(h); }
static inline void ts_push_num(uint32_t a, double v) { uint32_t h = ts_raw_js_from_number(v); ts_raw_js_array_push(a, h); ts_raw_js_release(h); }
static inline void ts_push_bool(uint32_t a, bool v) { uint32_t h = ts_raw_js_from_bool(v); ts_raw_js_array_push(a, h); ts_raw_js_release(h); }
static inline void ts_push_val(uint32_t a, uint32_t v) { ts_raw_js_array_push(a, v); }
#define js_push_str(a, v) ts_push_str((a).h, TS_STR(v))
#define js_push_num(a, v) ts_push_num((a).h, (v))
#define js_push_bool(a, v) ts_push_bool((a).h, (v))
#define js_push_val(a, v) ts_push_val((a).h, (v).h)

#endif
";

// What an argument left out is: `undefined`, as each type says it.
fn absent(t: &Ty) -> Option<String> {
    let Ty::Nullable(inner, true) = t else { return None };
    Some(match &**inner {
        Ty::Num | Ty::NumEnum(_) => "js_none_double()".into(),
        Ty::Bool => "js_none_bool()".into(),
        Ty::Enum(e) => format!("{e}_absent"),
        Ty::Str => "(const char*)0".into(),
        Ty::NamedUnion(n) if decodable(n) => format!("({n}){{ .tag = -1 }}"),
        _ => format!("({}){{ 0 }}", typed(t, true)),
    })
}

// The C types `_Generic` takes for a parameter of this type, where it can
// tell: an enumerator is an int, so an enum cannot be told from a number.
fn generic_types(t: &Ty, has_num: bool) -> Option<Vec<String>> {
    let list = |names: &[&str]| Some(names.iter().map(|n| n.to_string()).collect());
    match t {
        Ty::Str => list(&["const char*", "char*", "JsString"]),
        Ty::Num => list(&["double", "float", "int", "unsigned int", "long", "unsigned long"]),
        // `true` is an int in C11.
        Ty::Bool => list(if has_num { &["bool"] } else { &["bool", "int"] }),
        // Any value: `_Generic` matches only a `JsValue`, not the Shape it would take.
        Ty::Enum(_) | Ty::NumEnum(_) | Ty::Nullable(..) | Ty::Void | Ty::Any => None,
        t => Some(vec![typed(t, true)]),
    }
}

// The overloads `f` is the first of, where the C type of one parameter tells
// them apart: `Shape_label(s, "a")` and `Shape_label(s, 7)`.
fn family<'a>(f: &'a Func, all: &'a [Func]) -> Option<(usize, Vec<&'a Func>)> {
    let key = |f: &Func| match &f.call {
        Call::Method(n) => Some((0, n.clone(), f.fixed.clone())),
        Call::Static(n) => Some((1, n.clone(), f.fixed.clone())),
        _ => None,
    };
    let k = key(f)?;
    let members: Vec<&Func> = all.iter().filter(|o| key(o).as_ref() == Some(&k)).collect();
    if members.len() < 2 || !std::ptr::eq(members[0], f) || members.iter().any(|o| o.params.len() != f.params.len()) {
        return None;
    }
    let differing: Vec<usize> = (0..f.params.len()).filter(|&i| members.iter().any(|o| o.params[i].1 != f.params[i].1)).collect();
    let [at] = differing[..] else { return None };
    let has_num = members.iter().any(|o| o.params[at].1 == Ty::Num);
    let mut seen = std::collections::BTreeSet::new();
    for o in &members {
        if !generic_types(&o.params[at].1, has_num)?.into_iter().all(|t| seen.insert(t)) {
            return None;
        }
    }
    Some((at, members))
}

// A function's name as a macro, so a call may: pass any subtype
// (`AS_Element(x)` is applied), leave out trailing optional arguments, and
// for the first of a family of overloads, have `_Generic` choose among them.
// `earlier` are the functions declared before f beside it: where one of them,
// called with the same leading arguments, leaves out the same ones, f does not.
fn call_macros(name: &str, owner: Option<&str>, f: &Func, earlier: &[&Func], family: Option<(usize, Vec<String>, Vec<&Func>)>, descendants: &std::collections::BTreeMap<String, Vec<String>>, as_macro: &dyn Fn(&str) -> String) -> String {
    let upcast = |t: &Ty| match t {
        Ty::Obj(n) if descendants.contains_key(n) => Some(n.clone()),
        Ty::Nullable(inner, _) => match &**inner {
            Ty::Obj(n) if descendants.contains_key(n) => Some(n.clone()),
            _ => None,
        },
        _ => None,
    };
    let mut params = Vec::new();
    let mut args = Vec::new();
    let mut any = false;
    if let Some(o) = owner {
        params.push("self".to_string());
        let upcast_self = descendants.contains_key(o);
        any |= upcast_self;
        args.push(if upcast_self { format!("{}(self)", as_macro(o)) } else { "(self)".into() });
    }
    let chosen = family.as_ref().map(|f| f.0);
    for (i, (_, t)) in f.params.iter().enumerate() {
        let a = format!("a{}", i + 1);
        params.push(a.clone());
        if chosen == Some(i) {
            args.push(format!("({a})"));
        } else if let Some(n) = upcast(t) {
            any = true;
            args.push(format!("{}({a})", as_macro(&n)));
        } else if *t == Ty::Str || matches!(t, Ty::Nullable(inner, _) if **inner == Ty::Str) {
            any = true;
            args.push(format!("TS_STR({a})"));
        } else if matches!(t, Ty::Nullable(inner, _) if matches!(**inner, Ty::Num | Ty::NumEnum(_))) {
            any = true;
            args.push(format!("TS_OPT_DOUBLE({a})"));
        } else if matches!(t, Ty::Nullable(inner, _) if **inner == Ty::Bool) {
            any = true;
            args.push(format!("TS_OPT_BOOL({a})"));
        } else {
            args.push(format!("({a})"));
        }
    }
    let mut prefix = String::new();
    let callee = match &family {
        Some((at, names, members)) => {
            let has_num = members.iter().any(|o| o.params[*at].1 == Ty::Num);
            let mut choices = Vec::new();
            for (o_name, o) in names.iter().zip(members) {
                if o.params[*at].1 == Ty::Str {
                    let mut c_params = Vec::new();
                    let mut c_args = Vec::new();
                    let mut js_params = Vec::new();
                    let mut js_args = Vec::new();
                    if Class::has_self(o) {
                        let own_ty = owner.unwrap();
                        c_params.push(format!("{own_ty} self"));
                        js_params.push(format!("{own_ty} self"));
                        c_args.push("self".to_string());
                        js_args.push("self".to_string());
                    }
                    for (j, (_, pt)) in o.params.iter().enumerate() {
                        if j == *at {
                            c_params.push(format!("const char* p{j}"));
                            js_params.push(format!("JsString p{j}"));
                            c_args.push(format!("ts_str_c(p{j})"));
                            js_args.push(format!("ts_str_js(p{j})"));
                        } else {
                            let pty = func_param_ty(pt);
                            c_params.push(format!("{pty} p{j}"));
                            js_params.push(format!("{pty} p{j}"));
                            c_args.push(format!("p{j}"));
                            js_args.push(format!("p{j}"));
                        }
                    }
                    let ret_ty = typed(&o.ret, false);
                    let ret_kw = if o.ret == Ty::Void { "" } else { "return " };
                    let c_fn = format!("ts_fam_cstr_{o_name}");
                    let js_fn = format!("ts_fam_jsstr_{o_name}");
                    prefix += &format!(
                        "static inline {ret_ty} {c_fn}({}) {{ {ret_kw}({o_name})({}); }}\n\
                         static inline {ret_ty} {js_fn}({}) {{ {ret_kw}({o_name})({}); }}\n",
                        c_params.join(", "),
                        c_args.join(", "),
                        js_params.join(", "),
                        js_args.join(", "),
                    );
                    choices.extend([format!("const char*: {c_fn}"), format!("char*: {c_fn}"), format!("JsString: {js_fn}")]);
                } else {
                    choices.extend(generic_types(&o.params[*at].1, has_num).unwrap_or_default().into_iter().map(|t| format!("{t}: {o_name}")));
                }
            }
            format!("_Generic((a{}), {})", at + 1, choices.join(", "))
        }
        None => name.to_string(),
    };
    let param_absent = |g: &Func, idx: usize| -> Option<String> {
        if g.rest && idx + 1 == g.params.len() {
            Some(format!("({}){{ 0 }}", func_param_ty(&g.params[idx].1)))
        } else {
            absent(&g.params[idx].1)
        }
    };
    // A setter's value is never left out: that is not setting it.
    let setter = matches!(f.call, Call::Set(_) | Call::StaticSet(_) | Call::IndexSet);
    let n = params.len();
    let own = usize::from(owner.is_some());
    let optional = if setter || n > 15 { 0 } else { (0..f.params.len()).rev().take_while(|&idx| param_absent(f, idx).is_some()).count() };
    let shadowed = |given: usize| {
        let k = given - own;
        earlier.iter().any(|g| g.call == f.call && g.fixed == f.fixed && g.params.len() >= k && g.params[..k] == f.params[..k] && (k..g.params.len()).all(|j| param_absent(g, j).is_some()))
    };
    // The shorter calls this function answers: down to where an earlier one does.
    let shorter: Vec<usize> = (n - optional..n).rev().take_while(|g| !shadowed(*g)).collect();
    let forward = (any || family.is_some()).then(|| format!("{callee}({})", args.join(", ")));
    if shorter.is_empty() {
        return forward.map(|call| format!("{prefix}#define {name}({}) {call}\n", params.join(", "))).unwrap_or_default();
    }
    // By the number of arguments: each shorter call adds what is absent and
    // makes the next longer one. All of them is the function, upcast or chosen.
    let mut out = format!("{prefix}#define {name}(...) TS_CALL({name}, __VA_ARGS__)\n");
    out += &match forward {
        Some(call) => format!("#define {name}__{n}({}) {call}\n", params.join(", ")),
        None => format!("#define {name}__{n} {name}\n"),
    };
    for given in shorter {
        let mut next: Vec<String> = params[..given].to_vec();
        next.push(param_absent(f, given - own).unwrap());
        out += &format!("#define {name}__{given}({}) {name}__{}({})\n", params[..given].join(", "), given + 1, next.join(", "));
    }
    out
}

// Direct and indirect bases, nearest first.
fn all_ancestors(m: &Model, class: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut todo = vec![class.to_string()];
    while let Some(c) = todo.pop() {
        for a in m.classes.iter().filter(|k| k.name == c).flat_map(|k| &k.ancestors) {
            if !out.contains(a) {
                out.push(a.clone());
                todo.insert(0, a.clone());
            }
        }
    }
    out
}

// The bindings as files: `ts_runtime.h`; `ts_types.h` with every handle type,
// enum and callback (so headers never need each other); one header per
// declaration in a directory per module, `<module>/<Name>.h`, with the module's
// functions in `<module>/functions.h`; an umbrella `<module>.h` per module, and
// `bindings.h` for everything.
// What C cannot express of a model: inline unions, generics, keyed signatures.
fn check(m: &Model) -> super::Problems {
    let mut problems = super::Problems::default();
    super::model::each_type(m, &mut |at, t| {
        let what = match t {
            Ty::Union(_) => "a union: set unions = \"name\" or \"erase\"",
            Ty::Key(_) | Ty::Mapped(_) | Ty::Listener(_) => "a keyed signature: set keyed_params = \"expand\"",
            Ty::Param(_) | Ty::Generic(..) => "generics: set generics = \"erase\"",
            _ => return,
        };
        problems.add(format!("{T} generator cannot express {what}"), at.to_string());
    });
    problems
}

pub fn files(m: &Model) -> Result<Vec<(String, String)>, Vec<String>> {
    files_named(m, "bindings", &[])
}

pub fn files_named(m: &Model, pkg_name: &str, refs: &[ProjectRef]) -> Result<Vec<(String, String)>, Vec<String>> {
    let problems = check(m);
    if !problems.is_empty() {
        return Err(problems.into_messages());
    }
    let wasm_module = if pkg_name == "bindings" { "env" } else { pkg_name };
    let raw_prefix = if pkg_name == "bindings" { String::new() } else { format!("{pkg_name}_") };
    let mut resolved_opaque: HashMap<String, (usize, String)> = HashMap::new();
    for name in &m.opaque {
        if matches!(name.as_str(), "Error" | "Date" | "Promise") || super::model::buffer_spec(name).is_some() {
            continue;
        }
        let orig = m.opaque_names.get(name).map(String::as_str).unwrap_or(name.as_str());
        if let Some((idx, _)) = refs.iter().enumerate().find(|(_, r)| r.structs.contains(orig)) {
            resolved_opaque.insert(name.clone(), (idx, orig.to_string()));
        }
    }
    let used_refs: BTreeSet<usize> = resolved_opaque.values().map(|(i, _)| *i).collect();
    let as_macro = |base: &str| -> String {
        if pkg_name != "bindings" && (is_builtin_c_type(base) || resolved_opaque.contains_key(base)) {
            format!("AS_{pkg_name}_{base}")
        } else {
            format!("AS_{base}")
        }
    };

    DECODABLE.with(|d| *d.borrow_mut() = m.unions.iter().filter(|u| u.decodable() && u.type_params.is_empty()).map(|u| u.name.clone()).collect());
    let mut types = String::from("// Generated.\n#pragma once\n#include \"ts_runtime.h\"\n");
    for &idx in &used_refs {
        types += &format!("#include \"{}\"\n", refs[idx].ts_types_include);
    }
    types.push('\n');
    if !m.enums.is_empty() {
        types += &format!(
            "{} uint32_t {}(int32_t, int32_t);\n{} int32_t {}(int32_t, uint32_t);\n\n",
            import_attr(wasm_module, "js_enum_value"),
            raw_fn(&raw_prefix, "js_enum_value"),
            import_attr(wasm_module, "js_enum_index"),
            raw_fn(&raw_prefix, "js_enum_index"),
        );
    }
    for (name, values) in &m.enums {
        let mut seen = std::collections::HashSet::new();
        // `E_absent`, -1, is an optional enum that is not there.
        let absent = format!("{name}_absent");
        seen.insert(absent.clone());
        let members: Vec<String> = values
            .iter()
            .map(|v| {
                let base = format!("{name}_{}", super::model::ident(v));
                let mut id = base.clone();
                let mut n = 2;
                while !seen.insert(id.clone()) {
                    id = format!("{base}_{n}");
                    n += 1;
                }
                id
            })
            .collect();
        types += &format!("typedef enum {{ {absent} = -1, {} }} {name};\n", members.join(", "));
    }
    for (name, members) in &m.num_enums {
        let members: Vec<String> = members.iter().map(|(member, value)| format!("{name}_{member} = {value}")).collect();
        types += &format!("typedef enum {{ {} }} {name};\n", members.join(", "));
    }
    // Every object is an opaque handle; the struct keeps the types apart.
    for name in &m.opaque {
        if matches!(name.as_str(), "Error" | "Date" | "Promise")
            || super::model::buffer_spec(name).is_some()
        {
            continue;
        }
        if let Some((_, orig)) = resolved_opaque.get(name) {
            if orig != name {
                types += &format!("#ifndef TS_DEFINED_{name}\n#define TS_DEFINED_{name}\ntypedef {orig} {name};\n#endif\n");
            }
            continue;
        }
        types += &format!("#ifndef TS_DEFINED_{name}\n#define TS_DEFINED_{name}\n/* external */\ntypedef struct {{ uint32_t h; }} {name};\n#endif\n");
    }
    for name in m.classes.iter().map(|c| &c.name).chain(m.callbacks.iter().map(|c| &c.name)) {
        types += &format!("typedef struct {{ uint32_t h; }} {name};\n");
    }
    for name in &m.opaque {
        if name == "Error" {
            types += "#ifndef TS_DEFINED_Error\n#define TS_DEFINED_Error\n\
                      typedef struct { uint32_t h; } Error;\n\
                      static inline Error Error_new(const char* message) { return (Error){ ts_raw_js_error_new(message, ts_strlen(message)) }; }\n\
                      static inline JsString Error_name(Error e) { return (JsString){ ts_raw_js_error_name(e.h) }; }\n\
                      static inline JsString Error_message(Error e) { return (JsString){ ts_raw_js_error_message(e.h) }; }\n\
                      static inline JsString Error_stack(Error e) { return (JsString){ ts_raw_js_error_stack(e.h) }; }\n\
                      #endif\n";
        } else if name == "Date" {
            types += "#ifndef TS_DEFINED_Date\n#define TS_DEFINED_Date\n\
                      typedef struct { uint32_t h; } Date;\n\
                      static inline Date Date_fromTime(double ms) { return (Date){ ts_raw_js_date_new(ms) }; }\n\
                      static inline double Date_now(void) { return ts_raw_js_date_now(); }\n\
                      static inline double Date_time(Date d) { return ts_raw_js_date_time(d.h); }\n\
                      static inline JsString Date_toISOString(Date d) { return (JsString){ ts_raw_js_date_to_iso_string(d.h) }; }\n\
                      #endif\n";
        } else if let Some(s) = super::model::buffer_spec(name) {
            types += &format!("#ifndef TS_DEFINED_{name}\n#define TS_DEFINED_{name}\n");
            types += &format!("typedef struct {{ uint32_t h; }} {name};\n");
            types += &format!("static inline uint32_t {name}_byteLength({name} x) {{ return ts_raw_js_bytes_len(x.h); }}\n");
            types += &format!("static inline uint32_t {name}_readBytes({name} x, uint32_t offset, void* dst, uint32_t len) {{ return ts_raw_js_bytes_read(x.h, offset, dst, len); }}\n");
            types += &format!("static inline uint32_t {name}_writeBytes({name} x, uint32_t offset, const void* src, uint32_t len) {{ return ts_raw_js_bytes_write(x.h, offset, src, len); }}\n");
            if s.has_buffer() {
                types += &format!("static inline uint32_t {name}_byteOffset({name} x) {{ return ts_raw_js_bytes_offset(x.h); }}\n");
                types += &format!("static inline JsValue {name}_buffer({name} x) {{ return (JsValue){{ ts_raw_js_bytes_buffer(x.h) }}; }}\n");
            }
            if s.is_typed_array() {
                let kind = s.kind.unwrap();
                let elem = c_elem(s.elem);
                let eb = s.elem.bytes();
                types += &format!(
                    "static inline {name} {name}_new(uint32_t len) {{ uint32_t n = len > UINT32_MAX / {eb}u ? UINT32_MAX : len * {eb}u; return ({name}){{ ts_raw_js_bytes_new({kind}, (const void*)0, n) }}; }}\n\
                     static inline {name} {name}_from(const {elem}* data, uint32_t len) {{ uint32_t n = len > UINT32_MAX / {eb}u ? UINT32_MAX : len * {eb}u; return ({name}){{ ts_raw_js_bytes_new({kind}, data, n) }}; }}\n\
                     static inline uint32_t {name}_length({name} x) {{ return ts_raw_js_bytes_len(x.h) / {eb}; }}\n\
                     static inline uint32_t {name}_read({name} x, uint32_t elem_offset, {elem}* dst, uint32_t count) {{ if (elem_offset > UINT32_MAX / {eb}u) return 0; uint32_t n = count > UINT32_MAX / {eb}u ? UINT32_MAX : count * {eb}u; return ts_raw_js_bytes_read(x.h, elem_offset * {eb}u, dst, n) / {eb}; }}\n\
                     static inline uint32_t {name}_write({name} x, uint32_t elem_offset, const {elem}* src, uint32_t count) {{ if (elem_offset > UINT32_MAX / {eb}u) return 0; uint32_t n = count > UINT32_MAX / {eb}u ? UINT32_MAX : count * {eb}u; return ts_raw_js_bytes_write(x.h, elem_offset * {eb}u, src, n) / {eb}; }}\n"
                );
            } else {
                if let Some(kind) = s.kind {
                    types += &format!(
                        "static inline {name} {name}_new(uint32_t byte_len) {{ return ({name}){{ ts_raw_js_bytes_new({kind}, (const void*)0, byte_len) }}; }}\n\
                         static inline {name} {name}_from(const void* data, uint32_t byte_len) {{ return ({name}){{ ts_raw_js_bytes_new({kind}, data, byte_len) }}; }}\n"
                    );
                }
                types += &format!(
                    "static inline uint32_t {name}_read({name} x, uint32_t offset, void* dst, uint32_t len) {{ return ts_raw_js_bytes_read(x.h, offset, dst, len); }}\n\
                     static inline uint32_t {name}_write({name} x, uint32_t offset, const void* src, uint32_t len) {{ return ts_raw_js_bytes_write(x.h, offset, src, len); }}\n"
                );
            }
            types += "#endif\n";
        }
    }
    // A callback is made from a function and a context pointer passed back to it.
    // C11: one upcast per type with descendants. `AS_Element(div)` is `div` as
    // an Element, and a compile error for anything that is not one.
    let mut descendants: std::collections::BTreeMap<String, Vec<String>> = Default::default();
    for c in &m.classes {
        for a in all_ancestors(m, &c.name) {
            descendants.entry(a).or_default().push(c.name.clone());
        }
    }
    for (base, subs) in &descendants {
        let arms: Vec<String> = subs.iter().map(|s| format!("    {s}: ({base}){{ (x).h }}")).collect();
        types += &format!("#define {}(x) _Generic((x), \\\n    {base}: (x), \\\n{})\n", as_macro(base), arms.join(", \\\n"));
    }
    // A promise: `Promise_then(p, fn, ctx)` calls fn(ctx, ok, value) once it
    // settles, ok with its value or not with its reason; the value is lent.
    let iterates_async = m.classes.iter().any(|c| c.iterates_async.is_some());
    if m.opaque.iter().any(|n| n == "Promise") || iterates_async {
        types += "#ifndef TS_DEFINED_Promise\n#define TS_DEFINED_Promise\n\
                  typedef struct { uint32_t h; } Promise;\n\
                  typedef void (*Promise_fn)(void* ctx, bool ok, JsValue value);\n\
                  TS_IMPORT(js_promise_then) void ts_raw_js_promise_then(uint32_t, Promise_fn, void*);\n\
                  static inline void Promise_then(Promise p, Promise_fn fn, void* ctx) { ts_raw_js_promise_then(p.h, fn, ctx); }\n\
                  #endif\n";
    }
    if iterates_async {
        types += "#ifndef TS_DEFINED_JsIterator_next_async\n#define TS_DEFINED_JsIterator_next_async\n\
                  static inline Promise JsIterator_next_async(JsIterator it) { return (Promise){ ts_raw_js_async_iterator_next(it.h) }; }\n\
                  #endif\n";
    }
    // Each typed array, after its element type and before what uses it.
    let mut elements = Vec::new();
    for f in m.all_funcs() {
        f.params.iter().for_each(|p| arrays(&p.1, &mut elements));
        arrays(&f.ret, &mut elements);
    }
    for c in &m.callbacks {
        c.params.iter().for_each(|p| arrays(&p.1, &mut elements));
        arrays(&c.ret, &mut elements);
    }
    for u in &m.unions {
        u.parts.iter().for_each(|p| arrays(p, &mut elements));
    }
    for c in m.classes.iter().filter(|c| c.elements.is_some()) {
        arrays(&Ty::ArrayOf(Box::new(c.elements.clone().unwrap())), &mut elements);
    }
    // The structs first, then unions, which may hold arrays, then the
    // functions, which may read unions out of arrays.
    let containers: Vec<String> = elements
        .iter()
        .map(|t| match t {
            Ty::ArrayOf(e) => array_type(e),
            Ty::Tuple(es) => tuple_type(es),
            _ => unreachable!(),
        })
        .collect();
    for c in &containers {
        let first = c.lines().next().unwrap();
        let cname = first.rsplit_once("} ").and_then(|(_, r)| r.strip_suffix(';')).unwrap_or("JsContainer").trim();
        types += &format!("#ifndef TS_DEFINED_{cname}_T\n#define TS_DEFINED_{cname}_T\n{first}\n#endif\n");
    }
    let unions: Vec<(String, String)> = m.unions.iter().filter(|u| u.decodable() && u.type_params.is_empty()).map(|u| union_type(u, m, wasm_module, &raw_prefix)).collect();
    for (decl, _) in &unions {
        types += decl;
    }
    for (_, functions) in &unions {
        types += functions;
    }
    for c in &containers {
        let first = c.lines().next().unwrap();
        let cname = first.rsplit_once("} ").and_then(|(_, r)| r.strip_suffix(';')).unwrap_or("JsContainer").trim();
        let rest: String = c.lines().skip(1).map(|l| format!("{l}\n")).collect();
        types += &format!("#ifndef TS_DEFINED_{cname}_FN\n#define TS_DEFINED_{cname}_FN\n{rest}#endif\n");
    }
    for c in &m.callbacks {
        let mut params = vec!["void* ctx".to_string()];
        // A numeric enum arrives as the number it is.
        // An optional number arrives as a lent handle, an optional flag as -1, 0 or 1.
        let plain = |t: &Ty| match t {
            Ty::NumEnum(_) => "double".to_string(),
            Ty::Nullable(inner, _) if matches!(**inner, Ty::Num | Ty::NumEnum(_)) => "JsValue".to_string(),
            Ty::Nullable(inner, _) if matches!(**inner, Ty::Bool | Ty::Enum(_)) => "int32_t".to_string(),
            // A union arrives as a lent handle.
            Ty::NamedUnion(_) | Ty::Nullable(..) if matches!(t, Ty::NamedUnion(_)) || matches!(t, Ty::Nullable(inner, _) if matches!(**inner, Ty::NamedUnion(_))) => "JsValue".to_string(),
            t => typed(t, false),
        };
        // `ctx` is the callback's own: a parameter of that name is `ctx_`.
        params.extend(c.params.iter().map(|(n, t)| {
            let n = ident(n);
            format!("{} {}", plain(t), if n == "ctx" { "ctx_".to_string() } else { n })
        }));
        let cb_from = format!("{}_from", c.name);
        let cb_raw = raw_fn(&raw_prefix, &cb_from);
        types += &format!("\ntypedef {} (*{}_fn)({});\n", plain(&c.ret), c.name, params.join(", "));
        types += &format!("{} uint32_t {cb_raw}({}_fn, void*);\n", import_attr(wasm_module, &cb_from), c.name);
        types += &format!("static inline {0} {0}_from({0}_fn fn, void* ctx) {{ return ({0}){{ {cb_raw}(fn, ctx) }}; }}\n", c.name);
    }

    let mut out = vec![("ts_runtime.h".to_string(), RUNTIME.to_string()), ("ts_types.h".to_string(), types)];
    let mut modules: std::collections::BTreeMap<String, Vec<String>> = Default::default();
    // `areas/css/X.h` includes `../../ts_types.h`.
    let header = |module: &str, extra_includes: &[String], body: String| {
        let prefix = "../".repeat(module.split('/').count());
        let extras: String = extra_includes.iter().map(|p| format!("#include \"{prefix}{p}\"\n")).collect();
        format!("// Generated.\n#pragma once\n#include \"{prefix}ts_types.h\"\n{extras}\n{body}")
    };
    let mut taken = std::collections::HashMap::new();
    let class_paths: Vec<String> = m
        .classes
        .iter()
        .map(|c| format!("{}.h", super::model::file_name(&format!("{}/{}", c.module, c.name), &mut taken)))
        .collect();
    let class_path_map: HashMap<&str, &str> = m.classes.iter().zip(&class_paths).map(|(c, p)| (c.name.as_str(), p.as_str())).collect();
    let class_by_name: HashMap<&str, &Class> = m.classes.iter().map(|c| (c.name.as_str(), c)).collect();
    let mut global_taken: HashSet<String> = KEYWORDS
        .iter()
        .chain(LIBC_NAMES)
        .map(|k| (*k).to_string())
        .chain(identifiers(RUNTIME))
        .chain(refs.iter().flat_map(|r| r.all_names.iter().cloned()))
        .collect();
    for (name, values) in &m.enums {
        global_taken.insert(name.clone());
        global_taken.insert(format!("{name}_absent"));
        for v in values {
            global_taken.insert(format!("{name}_{}", super::model::ident(v)));
        }
    }
    for (name, members) in &m.num_enums {
        global_taken.insert(name.clone());
        for (member, _) in members {
            global_taken.insert(format!("{name}_{member}"));
        }
    }
    for name in &m.opaque {
        global_taken.insert(name.clone());
    }
    for c in &m.classes {
        global_taken.insert(c.name.clone());
        global_taken.insert(as_macro(&c.name));
        for a in all_ancestors(m, &c.name) {
            global_taken.insert(format!("{}_as_{a}", c.name));
        }
        if c.js.is_some() {
            global_taken.insert(format!("{}_cast", c.name));
            global_taken.insert(format!("{}_try_cast", c.name));
        }
        if c.elements.is_some() {
            global_taken.insert(format!("{}_as_array", c.name));
        }
        if c.iterates.is_some() || c.iterates_async.is_some() {
            global_taken.insert(format!("{}_iterate", c.name));
            global_taken.insert(format!("{}_iterate_async", c.name));
            global_taken.insert(format!("{}_iterate_next", c.name));
        }
        for (k, _) in &c.consts {
            global_taken.insert(format!("{}_{k}", c.name));
        }
        for f in &c.funcs {
            global_taken.insert(format!("{}_{}", c.name, f.name));
        }
    }
    for c in &m.callbacks {
        global_taken.insert(c.name.clone());
        global_taken.insert(format!("{}_fn", c.name));
        global_taken.insert(format!("{}_from", c.name));
    }
    for u in &m.unions {
        for id in (NAMING.type_names)(&u.name, &u.name, Kind::Union, m) {
            global_taken.insert(id);
        }
    }
    for f in &m.funcs {
        global_taken.insert(f.name.clone());
    }
    for (c, path) in m.classes.iter().zip(&class_paths) {
        let mut body = String::new();
        for (name, value) in &c.consts {
            body += &format!("#define {}_{name} {value}\n", c.name);
        }
        // A subclass of Array, as the array it is: the same handle.
        if let Some(elem) = &c.elements {
            let array = typed(&Ty::ArrayOf(Box::new(elem.clone())), false);
            body += &format!("static inline {array} {0}_as_array({0} self) {{ return ({array}){{ self.h }}; }}\n", c.name);
        }
        // Asynchronous iteration: `X_iterate_async(x)`, then `JsIterator_next_async(it)`, a
        // promise of the next value, or of a null handle at the end.
        if c.iterates_async.is_some() {
            let n = &c.name;
            body += &format!("static inline JsIterator {n}_iterate_async({n} self) {{ return (JsIterator){{ ts_raw_js_async_iterate(self.h) }}; }}\n");
        }
        // Iteration: `X_iterate(x)`, then `X_iterate_next(it, &value)` until false; each value is the caller's.
        if let Some(elem) = &c.iterates {
            let (ty, value) = match elem {
                Ty::Num => ("double".to_string(), "ts_raw_js_to_number(v); ts_raw_js_release(v)".to_string()),
                Ty::Obj(_) | Ty::Str | Ty::Array | Ty::ArrayOf(_) => (typed(elem, false), format!("({}){{ v }}", typed(elem, false))),
                _ => ("JsValue".to_string(), "(JsValue){ v }".to_string()),
            };
            let n = &c.name;
            body += &format!("static inline JsIterator {n}_iterate({n} self) {{ return (JsIterator){{ ts_raw_js_iterate(self.h) }}; }}\n");
            body += &format!("static inline bool {n}_iterate_next(JsIterator it, {ty}* out) {{ uint32_t v = ts_raw_js_iterator_next(it.h); if (v == 0xFFFFFFFFu) return false; *out = {value}; return true; }}\n");
            // Any subtype iterates: `HTMLCollectionBase_iterate(children)`.
            if descendants.contains_key(n) {
                body += &format!("#define {n}_iterate(self) {n}_iterate({}(self))\n", as_macro(n));
            }
        }
        // A cast to every ancestor: an inherited member is called on the type that declares it.
        for a in all_ancestors(m, &c.name) {
            body += &format!("static inline {a} {}_as_{a}({} self) {{ return ({a}){{ self.h }}; }}\n", c.name, c.name);
        }
        // Checked downcast: only a class has a constructor to test against.
        if c.js.is_some() {
            let cast_name = if c.funcs.iter().any(|f| f.name == "cast") { "try_cast" } else { "cast" };
            let is_name = format!("js_is__{}", c.name);
            let is_attr = import_attr(wasm_module, &is_name);
            let is_raw = raw_fn(&raw_prefix, &is_name);
            body += &format!(
                "{is_attr} int32_t {is_raw}(uint32_t);\n\
                 static inline {0} {0}_{cast_name}_h(uint32_t h) {{ return ({0}){{ (h && {is_raw}(h)) ? h : 0 }}; }}\n\
                 #define {0}_{cast_name}(v) {0}_{cast_name}_h((v).h)\n",
                c.name
            );
        }
        for f in &c.funcs {
            func(&mut body, Some(&c.name), f, wasm_module, &raw_prefix);
        }
        // After every function: a macro may choose a later one.
        for (i, f) in c.funcs.iter().enumerate() {
            let overloads = family(f, &c.funcs).map(|(at, members)| (at, members.iter().map(|o| format!("{}_{}", c.name, o.name)).collect(), members));
            let earlier: Vec<&Func> = c.funcs[..i].iter().collect();
            body += &call_macros(&format!("{}_{}", c.name, f.name), Class::has_self(f).then_some(c.name.as_str()), f, &earlier, overloads, &descendants, &as_macro);
        }
        let mut own_methods: HashSet<&str> = c.funcs.iter().map(|f| f.name.as_str()).chain(c.consts.iter().map(|(k, _)| k.as_str())).collect();
        own_methods.extend(["cast", "try_cast", "as_array", "iterate", "iterate_async", "iterate_next"]);
        let mut ancestor_includes = Vec::new();
        for a in all_ancestors(m, &c.name) {
            let Some(a_class) = class_by_name.get(a.as_str()) else { continue };
            for af in a_class.funcs.iter().filter(|af| Class::has_self(af)) {
                if !own_methods.insert(af.name.as_str()) {
                    continue;
                }
                let alias = format!("{}_{}", c.name, af.name);
                if !global_taken.insert(alias.clone()) {
                    continue;
                }
                let a_path = class_path_map[a.as_str()].to_string();
                if !ancestor_includes.contains(&a_path) {
                    ancestor_includes.push(a_path);
                }
                body += &format!("#define {alias} {a}_{}\n", af.name);
            }
        }
        modules.entry(c.module.clone()).or_default().push(path.clone());
        out.push((path.clone(), header(&c.module, &ancestor_includes, body)));
    }
    let mut functions: std::collections::BTreeMap<String, String> = Default::default();
    for f in &m.funcs {
        func(functions.entry(f.module.clone()).or_default(), None, f, wasm_module, &raw_prefix);
    }
    for (i, f) in m.funcs.iter().enumerate() {
        // Overloads in one module: in one header.
        let overloads = family(f, &m.funcs).filter(|(_, members)| members.iter().all(|o| o.module == f.module)).map(|(at, members)| (at, members.iter().map(|o| o.name.clone()).collect(), members));
        let earlier: Vec<&Func> = m.funcs[..i].iter().filter(|g| g.module == f.module).collect();
        *functions.entry(f.module.clone()).or_default() += &call_macros(&f.name, None, f, &earlier, overloads, &descendants, &as_macro);
    }
    for (module, body) in functions {
        let path = format!("{}.h", super::model::file_name(&format!("{module}/functions"), &mut taken));
        out.push((path.clone(), header(&module, &[], body)));
        modules.entry(module).or_default().push(path);
    }
    let mut all = String::from("// Generated.\n#pragma once\n");
    for (module, headers) in &modules {
        // `areas/css.h` includes `css/X.h`, next to it.
        let parent = module.rsplit_once('/').map_or("", |p| p.0);
        let includes: String = headers.iter().map(|h| format!("#include \"{}\"\n", h.strip_prefix(&format!("{parent}/")).unwrap_or(h))).collect();
        out.push((format!("{module}.h"), format!("// Generated.\n#pragma once\n{includes}")));
        all += &format!("#include \"{module}.h\"\n");
    }
    out.push(("bindings.h".into(), all.clone()));
    if pkg_name != "bindings" {
        out.push((format!("{pkg_name}.h"), all));
    }
    Ok(out)
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
