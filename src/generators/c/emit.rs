use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use super::super::model::{Class, Func, Model, Ty};
use super::super::names::{Kind, identifiers};
use super::types::{
    all_ancestors, array_type, arrays, call_macros, check, family, func, tuple_type, typed, union_type,
};
use super::{
    Ctx, KEYWORDS, LIBC_NAMES, NAMING, ProjectRef, RUNTIME, c_elem, ident, import_attr, is_builtin_c_type, raw_fn,
};

// The bindings as files: `ts_runtime.h`; `ts_types.h` with every handle type,
// enum and callback (so headers never need each other); one header per
// declaration in a directory per module, `<module>/<Name>.h`, with the module's
// functions in `<module>/functions.h`; an umbrella `<module>.h` per module, and
// `bindings.h` for everything.
pub(super) fn files_named(m: &Model, pkg_name: &str, refs: &[ProjectRef]) -> Result<Vec<(String, String)>, Vec<String>> {
    let problems = check(m);
    if !problems.is_empty() {
        return Err(problems.into_messages());
    }
    let wasm_module = if pkg_name == "bindings" { "env" } else { pkg_name };
    let raw_prefix = if pkg_name == "bindings" { String::new() } else { format!("{pkg_name}_") };
    let mut resolved_opaque: HashMap<String, (usize, String)> = HashMap::new();
    for name in &m.opaque {
        if matches!(name.as_str(), "Error" | "Date" | "Promise") || super::super::model::buffer_spec(name).is_some() {
            continue;
        }
        let orig = m.opaque_names.get(name).map(String::as_str).unwrap_or(name.as_str());
        if let Some((idx, resolved_c_name)) = refs.iter().enumerate().find_map(|(idx, r)| {
            if r.structs.contains(orig) {
                Some((idx, orig.to_string()))
            } else {
                r.struct_aliases.get(orig).map(|c_name| (idx, c_name.clone()))
            }
        }) {
            resolved_opaque.insert(name.clone(), (idx, resolved_c_name));
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

    let cx = Ctx {
        decodable: m.unions.iter().filter(|u| u.decodable() && u.type_params.is_empty()).map(|u| u.name.clone()).collect(),
    };
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
    let mut meta_structs: Vec<String> = Vec::new();
    let mut meta_external_structs: Vec<String> = Vec::new();
    let mut all_names: Vec<String> = Vec::new();
    for (name, values) in &m.enums {
        let mut seen = HashSet::new();
        // `E_absent`, -1, is an optional enum that is not there.
        let absent = format!("{name}_absent");
        seen.insert(absent.clone());
        let members: Vec<String> = values
            .iter()
            .map(|v| {
                let base = format!("{name}_{}", super::super::model::ident(v));
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
        if !is_builtin_c_type(name) {
            all_names.push(name.clone());
            all_names.push(absent);
            all_names.extend(members);
        }
    }
    for (name, members) in &m.num_enums {
        if !is_builtin_c_type(name) {
            all_names.push(name.clone());
            for (member, _) in members {
                all_names.push(format!("{name}_{member}"));
            }
        }
        let members: Vec<String> = members.iter().map(|(member, value)| format!("{name}_{member} = {value}")).collect();
        types += &format!("typedef enum {{ {} }} {name};\n", members.join(", "));
    }
    // Every object is an opaque handle; the struct keeps the types apart.
    for name in &m.opaque {
        if matches!(name.as_str(), "Error" | "Date" | "Promise")
            || super::super::model::buffer_spec(name).is_some()
        {
            continue;
        }
        if let Some((_, orig)) = resolved_opaque.get(name) {
            if orig != name {
                types += &format!("#ifndef TS_DEFINED_{name}\n#define TS_DEFINED_{name}\ntypedef {orig} {name};\n#endif\n");
            }
            continue;
        }
        if !is_builtin_c_type(name) {
            meta_external_structs.push(name.clone());
            all_names.push(name.clone());
        }
        types += &format!("#ifndef TS_DEFINED_{name}\n#define TS_DEFINED_{name}\n/* external */\ntypedef struct {{ uint32_t h; }} {name};\n#endif\n");
    }
    for name in m.classes.iter().map(|c| &c.name).chain(m.callbacks.iter().map(|c| &c.name)) {
        if !is_builtin_c_type(name) {
            meta_structs.push(name.clone());
            all_names.push(name.clone());
        }
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
        } else if let Some(s) = super::super::model::buffer_spec(name) {
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
    let mut descendants: BTreeMap<String, Vec<String>> = Default::default();
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
            Ty::ArrayOf(e) => array_type(e, &cx),
            Ty::Tuple(es) => tuple_type(es, &cx),
            _ => unreachable!(),
        })
        .collect();
    for c in &containers {
        let first = c.lines().next().unwrap();
        let cname = first.rsplit_once("} ").and_then(|(_, r)| r.strip_suffix(';')).unwrap_or("JsContainer").trim();
        types += &format!("#ifndef TS_DEFINED_{cname}_T\n#define TS_DEFINED_{cname}_T\n{first}\n#endif\n");
    }
    let unions: Vec<(String, String)> = m
        .unions
        .iter()
        .filter(|u| u.decodable() && u.type_params.is_empty())
        .map(|u| {
            if !is_builtin_c_type(&u.name) {
                all_names.push(u.name.clone());
            }
            union_type(u, m, wasm_module, &raw_prefix, &cx)
        })
        .collect();
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
            t => typed(t, false, &cx),
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
    let mut modules: BTreeMap<String, Vec<String>> = Default::default();
    // `areas/css/X.h` includes `../../ts_types.h`.
    let header = |module: &str, extra_includes: &[String], body: String| {
        let prefix = "../".repeat(module.split('/').count());
        let extras: String = extra_includes.iter().map(|p| format!("#include \"{prefix}{p}\"\n")).collect();
        format!("// Generated.\n#pragma once\n#include \"{prefix}ts_types.h\"\n{extras}\n{body}")
    };
    let mut taken = HashMap::new();
    let class_paths: Vec<String> = m
        .classes
        .iter()
        .map(|c| format!("{}.h", super::super::model::file_name(&format!("{}/{}", c.module, c.name), &mut taken)))
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
            global_taken.insert(format!("{name}_{}", super::super::model::ident(v)));
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
            let array = typed(&Ty::ArrayOf(Box::new(elem.clone())), false, &cx);
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
                Ty::Obj(_) | Ty::Str | Ty::Array | Ty::ArrayOf(_) => (typed(elem, false, &cx), format!("({}){{ v }}", typed(elem, false, &cx))),
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
            func(&mut body, Some(&c.name), f, wasm_module, &raw_prefix, &cx);
        }
        // After every function: a macro may choose a later one.
        for (i, f) in c.funcs.iter().enumerate() {
            let overloads = family(f, &c.funcs, &cx).map(|(at, members)| (at, members.iter().map(|o| format!("{}_{}", c.name, o.name)).collect(), members));
            let earlier: Vec<&Func> = c.funcs[..i].iter().collect();
            body += &call_macros(&format!("{}_{}", c.name, f.name), Class::has_self(f).then_some(c.name.as_str()), f, &earlier, overloads, &descendants, &as_macro, &cx);
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
    let mut functions: BTreeMap<String, String> = Default::default();
    for f in &m.funcs {
        func(functions.entry(f.module.clone()).or_default(), None, f, wasm_module, &raw_prefix, &cx);
    }
    for (i, f) in m.funcs.iter().enumerate() {
        // Overloads in one module: in one header.
        let overloads = family(f, &m.funcs, &cx).filter(|(_, members)| members.iter().all(|o| o.module == f.module)).map(|(at, members)| (at, members.iter().map(|o| o.name.clone()).collect(), members));
        let earlier: Vec<&Func> = m.funcs[..i].iter().filter(|g| g.module == f.module).collect();
        *functions.entry(f.module.clone()).or_default() += &call_macros(&f.name, None, f, &earlier, overloads, &descendants, &as_macro, &cx);
    }
    for (module, body) in functions {
        for line in body.lines().map(str::trim) {
            if let Some(rest) = line.strip_prefix("static inline ")
                && let Some((head, _)) = rest.split_once('(')
                && let Some(name) = head.split_whitespace().last()
                && !name.is_empty()
            {
                all_names.push(name.to_string());
            } else if let Some(rest) = line.strip_prefix("#define ")
                && let Some((name, _)) = rest.split_once('(')
            {
                let name = name.trim();
                if !name.is_empty() && !name.contains("__") {
                    all_names.push(name.to_string());
                }
            }
        }
        let path = format!("{}.h", super::super::model::file_name(&format!("{module}/functions"), &mut taken));
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
    let entry = if pkg_name != "bindings" { format!("{pkg_name}.h") } else { "bindings.h".into() };
    let c_structs: BTreeMap<String, String> = meta_structs
        .iter()
        .filter_map(|s| m.original_type_names.get(s).map(|orig| (orig.clone(), s.clone())))
        .collect();
    let meta = super::super::pkg_meta::PkgMeta {
        target: "c".into(),
        package: pkg_name.to_string(),
        entry: Some(entry),
        types: all_names,
        structs: meta_structs,
        external_structs: meta_external_structs,
        c_structs,
        ..Default::default()
    };
    out.push((super::super::pkg_meta::META_FILE.into(), meta.to_toml()));
    Ok(out)
}
