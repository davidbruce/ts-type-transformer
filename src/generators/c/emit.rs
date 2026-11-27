use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use super::super::model::{Call, Class, Func, Model, Ty};
use super::super::names::Kind;
use super::super::comments::{Style, comment};
use super::types::{
    all_ancestors, array_type, array_union_members, arrays, bag, call_macros, check, element, family, func, of_choices, record_type, tuple_type, typable, typed, union_type, with_doc,
};
use super::{
    Ctx, KEYWORDS, LIBC_NAMES, NAMING, ProjectRef, RUNTIME, c_elem, ident, import_attr, is_builtin_c_type, raw_fn, runtime_names,
};

// The bindings as files: `ts_runtime.h`; `ts_types.h` with every handle type,
// enum and callback (so headers never need each other); one header per
// declaration in a directory per module, `<module>/<Name>.h`, with the module's
// functions in `<module>/functions.h`; an umbrella `<module>.h` per module, and
// `bindings.h` for everything.

// The most functions of one literal family a subclass repeats.
const MAX_INHERITED_FAMILY: usize = 150;

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

    let mut descendants: BTreeMap<String, Vec<String>> = Default::default();
    for c in &m.classes {
        for a in all_ancestors(m, &c.name) {
            descendants.entry(a).or_default().push(c.name.clone());
        }
    }
    let local_type_names: HashSet<&str> = m
        .classes
        .iter()
        .map(|c| c.name.as_str())
        .chain(m.callbacks.iter().map(|c| c.name.as_str()))
        .chain(m.unions.iter().map(|u| u.name.as_str()))
        .chain(m.type_aliases.iter().map(|a| a.0.as_str()))
        .chain(m.record_aliases.iter().map(|a| a.0.as_str()))
        .chain(m.enums.iter().map(|e| e.0.as_str()))
        .chain(m.num_enums.iter().map(|e| e.0.as_str()))
        .collect();
    let mut subtypes: HashMap<String, Vec<String>> = descendants.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    let mut opaque_keys: Vec<&String> = resolved_opaque.keys().collect();
    opaque_keys.sort();
    let mut remote_as: HashMap<String, String> = HashMap::new();
    for name in opaque_keys {
        let (idx, orig_c_name) = &resolved_opaque[name];
        if let Some(remote_subs) = refs[*idx].subtypes.get(orig_c_name) {
            let entry = subtypes.entry(name.clone()).or_default();
            for s in remote_subs {
                if refs[*idx].structs.contains(s) && !local_type_names.contains(s.as_str()) && !entry.contains(s) {
                    entry.push(s.clone());
                }
            }
            if !descendants.contains_key(name) {
                remote_as.insert(name.clone(), format!("AS_{orig_c_name}"));
            }
        }
    }
    let mut cx = Ctx {
        decodable: m.unions.iter().filter(|u| u.decodable() && u.type_params.is_empty()).map(|u| u.name.clone()).collect(),
        typed: HashSet::new(),
        of: HashMap::new(),
        array_arg: HashMap::new(),
        subtypes,
        callbacks: m.callbacks.iter().map(|c| c.name.clone()).collect(),
    };
    // A union only a value goes into is typed where a member can be told by its C type: else a value stays one.
    cx.of = m.unions.iter().filter(|u| cx.decodable(&u.name) || typable(u)).filter_map(|u| Some((u.name.clone(), of_choices(u, &cx)?.into_iter().map(|c| c.0).chain((!cx.decodable(&u.name)).then(|| "JsValue".to_string())).collect()))).collect();
    cx.typed = m.unions.iter().filter(|u| cx.decodable(&u.name) || cx.of.contains_key(&u.name)).map(|u| u.name.clone()).collect();
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
    for (at, (name, values)) in m.enums.iter().enumerate() {
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
        let docs = m.enum_docs.get(at);
        types += &docs.and_then(|d| d.doc.as_deref()).map(|d| comment(d, "//", "", Style::C)).unwrap_or_default();
        if docs.is_some_and(|d| d.members.iter().any(Option::is_some)) {
            types += &format!("typedef enum {{\n    {absent} = -1,\n");
            for (i, member) in members.iter().enumerate() {
                types += &docs.and_then(|d| d.members[i].as_deref()).map(|d| comment(d, "//", "    ", Style::C)).unwrap_or_default();
                types += &format!("    {member},\n");
            }
            types += &format!("}} {name};\n");
        } else {
            types += &format!("typedef enum {{ {absent} = -1, {} }} {name};\n", members.join(", "));
        }
        if !is_builtin_c_type(name) {
            all_names.push(name.clone());
            all_names.push(absent);
            all_names.extend(members);
        }
    }
    for (at, (name, members)) in m.num_enums.iter().enumerate() {
        if !is_builtin_c_type(name) {
            all_names.push(name.clone());
            for (member, _) in members {
                all_names.push(format!("{name}_{member}"));
            }
        }
        let docs = m.num_enum_docs.get(at);
        types += &docs.and_then(|d| d.doc.as_deref()).map(|d| comment(d, "//", "", Style::C)).unwrap_or_default();
        if docs.is_some_and(|d| d.members.iter().any(Option::is_some)) {
            types += "typedef enum {\n";
            for (i, (member, value)) in members.iter().enumerate() {
                types += &docs.and_then(|d| d.members[i].as_deref()).map(|d| comment(d, "//", "    ", Style::C)).unwrap_or_default();
                types += &format!("    {name}_{member} = {value},\n");
            }
            types += &format!("}} {name};\n");
        } else {
            let members: Vec<String> = members.iter().map(|(member, value)| format!("{name}_{member} = {value}")).collect();
            types += &format!("typedef enum {{ {} }} {name};\n", members.join(", "));
        }
        if m.flag_enums.contains(name) {
            types += &format!(
                "static inline {name} {name}_or({name} a, {name} b) {{ return ({name})((int32_t)a | (int32_t)b); }}\n\
                 static inline {name} {name}_and({name} a, {name} b) {{ return ({name})((int32_t)a & (int32_t)b); }}\n\
                 static inline {name} {name}_xor({name} a, {name} b) {{ return ({name})((int32_t)a ^ (int32_t)b); }}\n\
                 static inline {name} {name}_not({name} a) {{ return ({name})(~(int32_t)a); }}\n\
                 static inline bool {name}_has({name} flags, {name} bit) {{ return ((int32_t)flags & (int32_t)bit) == (int32_t)bit; }}\n"
            );
            if !is_builtin_c_type(name) {
                for s in ["or", "and", "xor", "not", "has"] {
                    all_names.push(format!("{name}_{s}"));
                }
            }
        }
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
    for (name, doc) in m.classes.iter().map(|c| (&c.name, c.doc.as_deref())).chain(m.callbacks.iter().map(|c| (&c.name, None))) {
        if !is_builtin_c_type(name) {
            meta_structs.push(name.clone());
            all_names.push(name.clone());
        }
        types += &doc.map(|d| comment(d, "//", "", Style::C)).unwrap_or_default();
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
    // C11: one upcast per type with descendants. `AS_Element(div)` is `div` as
    // an Element (or NULL), and a compile error for anything that is not one.
    for (base, local_subs) in &descendants {
        let mut all_subs = local_subs.clone();
        if let Some((idx, orig)) = resolved_opaque.get(base)
            && let Some(rsubs) = refs[*idx].subtypes.get(orig)
        {
            for s in rsubs {
                if refs[*idx].structs.contains(s) && !local_type_names.contains(s.as_str()) && !all_subs.contains(s) {
                    all_subs.push(s.clone());
                }
            }
        }
        let mname = as_macro(base);
        types += &format!("#ifndef TS_DEFINED_{mname}\n#define TS_DEFINED_{mname}\n");
        types += &format!("static inline {base} ts_up_{mname}_self({base} x) {{ return x; }}\n");
        types += &format!("static inline {base} ts_up_{mname}_null(const void* p) {{ (void)p; return ({base}){{ 0 }}; }}\n");
        let mut arms = vec![format!("    {base}: ts_up_{mname}_self"), format!("    TS_NULL_ARMS(ts_up_{mname}_null)")];
        for (i, s) in all_subs.iter().enumerate() {
            types += &format!("static inline {base} ts_up_{mname}_{i}({s} x) {{ return ({base}){{ x.h }}; }}\n");
            arms.push(format!("    {s}: ts_up_{mname}_{i}"));
        }
        types += &format!("#define {mname}(x) _Generic((x), \\\n{})((x))\n#endif\n", arms.join(", \\\n"));
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
    for (_, t) in &m.record_aliases {
        arrays(t, &mut elements);
    }
    let mut extra = Vec::new();
    for t in &elements {
        if let Ty::ArrayOf(inner) = t
            && let Some((_, pairs)) = array_union_members(inner, m, &cx)
        {
            let arr_name = format!("JsArray_{}", element(inner, &cx));
            let mut mem_tys = Vec::with_capacity(pairs.len());
            for (_, part, mem_arr_ty) in pairs {
                arrays(&Ty::ArrayOf(Box::new(part.clone())), &mut extra);
                mem_tys.push(mem_arr_ty);
            }
            cx.array_arg.insert(arr_name, mem_tys);
        }
    }
    for t in extra {
        if !elements.contains(&t) {
            elements.push(t);
        }
    }
    // The structs first, then unions, which may hold arrays or records, then the
    // functions, which may read unions out of arrays or records.
    let short_macro_ok = |t: &Ty, suffix: &str| match t {
        Ty::Obj(n) | Ty::Enum(n) | Ty::NumEnum(n) | Ty::NamedUnion(n) => {
            !m.classes.iter().any(|c| c.name == *n && c.funcs.iter().any(|f| f.name == suffix))
                && !m.funcs.iter().any(|f| f.name == format!("{n}_{suffix}"))
                && !m.enums.iter().any(|(en, vs)| en == n && vs.iter().any(|v| super::super::model::ident(v) == suffix))
                && !m.num_enums.iter().any(|(en, ms)| en == n && ms.iter().any(|(k, _)| k == suffix))
        }
        _ => false,
    };
    let elem_upcast_macro = |e: &Ty| -> Option<String> {
        let Ty::Obj(n) = e else { return None };
        if descendants.contains_key(n) {
            Some(as_macro(n))
        } else {
            remote_as.get(n).cloned()
        }
    };
    let mut emitted_arrays: HashSet<String> = HashSet::new();
    let containers: Vec<String> = elements
        .iter()
        .map(|t| match t {
            Ty::ArrayOf(e) => {
                let arr_name = format!("JsArray_{}", element(e, &cx));
                emitted_arrays.insert(arr_name.clone());
                let mems = cx.array_arg.get(&arr_name).map(Vec::as_slice).unwrap_or(&[]);
                array_type(e, m, &raw_prefix, &cx, elem_upcast_macro(e).as_deref(), short_macro_ok(e, "list"), mems)
            }
            Ty::Tuple(es) => tuple_type(es, &cx),
            Ty::Record(v) => record_type(v, m, &raw_prefix, &cx, short_macro_ok(v, "record")),
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
        .filter(|u| cx.typed.contains(&u.name))
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
    let mut alias_claimed: HashSet<String> = all_names
        .iter()
        .cloned()
        .chain(KEYWORDS.iter().chain(LIBC_NAMES).map(|s| (*s).to_string()))
        .chain(runtime_names())
        .chain(m.classes.iter().map(|c| c.name.clone()))
        .chain(m.callbacks.iter().map(|c| c.name.clone()))
        .chain(m.enums.iter().map(|e| e.0.clone()))
        .chain(m.num_enums.iter().map(|e| e.0.clone()))
        .chain(m.opaque.iter().cloned())
        .chain(m.funcs.iter().map(|f| f.name.clone()))
        .chain(emitted_arrays.iter().cloned())
        .chain(refs.iter().flat_map(|r| r.all_names.iter().cloned()))
        .collect();
    for u in m.unions.iter().filter(|u| cx.typed.contains(&u.name)) {
        for f in super::union_fields(u) {
            alias_claimed.insert(format!("{}_{f}", u.name));
        }
        alias_claimed.insert(format!("{}_of", u.name));
        alias_claimed.insert(format!("{}_from_handle", u.name));
        alias_claimed.insert(format!("{}_to_handle", u.name));
        alias_claimed.insert(format!("{}_release", u.name));
    }
    for (alias, to) in m.type_aliases.iter().filter(|(_, to)| cx.typed.contains(to)) {
        if !alias_claimed.insert(alias.clone()) {
            continue;
        }
        types += &format!("typedef {to} {alias};\n");
        all_names.push(alias.clone());
        if let Some(u) = m.unions.iter().find(|u| u.name == *to) {
            let mut tags = Vec::new();
            for f in super::union_fields(u) {
                let tag_id = format!("{alias}_{f}");
                if alias_claimed.insert(tag_id.clone()) {
                    tags.push(format!("{tag_id} = {to}_{f}"));
                }
            }
            if !tags.is_empty() {
                types += &format!("enum {{ {} }};\n", tags.join(", "));
            }
        }
        if cx.of.contains_key(to) && alias_claimed.insert(format!("{alias}_of")) {
            types += &format!("#define {alias}_of {to}_of\n");
        }
        if cx.decodable(to) {
            if alias_claimed.insert(format!("{alias}_from_handle")) {
                types += &format!("#define {alias}_from_handle {to}_from_handle\n");
            }
            if alias_claimed.insert(format!("{alias}_to_handle")) {
                types += &format!("#define {alias}_to_handle {to}_to_handle\n");
            }
            if alias_claimed.insert(format!("{alias}_release")) {
                types += &format!("#define {alias}_release {to}_release\n");
            }
        }
        let arr_to = format!("JsArray_{to}");
        let arr_alias = format!("JsArray_{alias}");
        if emitted_arrays.contains(&arr_to) && alias_claimed.insert(arr_alias) {
            types += &format!("typedef {arr_to} JsArray_{alias};\n");
            let to_ty = Ty::NamedUnion(to.clone());
            let alias_ty = Ty::NamedUnion(alias.clone());
            if !is_builtin_c_type(to)
                && short_macro_ok(&to_ty, "list")
                && short_macro_ok(&alias_ty, "list")
                && alias_claimed.insert(format!("{alias}_list"))
            {
                types += &format!("#ifndef {alias}_list\n#define {alias}_list {to}_list\n#endif\n");
            }
        }
    }
    for (alias, t) in &m.record_aliases {
        if !all_names.contains(alias) && let Ty::Record(_) = t {
            let rty = typed(t, false, &cx);
            types += &format!(
                "typedef {rty} {alias};\n\
                 #ifndef {alias}_record\n\
                 #define {alias}_record(...) TS_RECORD({rty}, __VA_ARGS__)\n\
                 #endif\n\
                 #ifndef {alias}_create\n\
                 #define {alias}_create() {rty}_new()\n\
                 #endif\n\
                 #ifndef {alias}_get_at\n\
                 #define {alias}_get_at(r, k) {rty}_get((r), k)\n\
                 #endif\n\
                 #ifndef {alias}_set_at\n\
                 #define {alias}_set_at(r, k, v) {rty}_set((r), k, v)\n\
                 #endif\n\
                 #ifndef {alias}_has\n\
                 #define {alias}_has(r, k) {rty}_has((r), k)\n\
                 #endif\n"
            );
            all_names.push(alias.clone());
        }
    }
    for c in &m.callbacks {
        let decodable_union = |t: &Ty| -> Option<String> {
            match t {
                Ty::NamedUnion(u) if cx.decodable(u) => Some(u.clone()),
                Ty::Nullable(inner, _) => match &**inner {
                    Ty::NamedUnion(u) if cx.decodable(u) => Some(u.clone()),
                    _ => None,
                },
                _ => None,
            }
        };
        let tramp = c.params.iter().any(|(_, t)| decodable_union(t).is_some());
        let mut params = vec!["void* ctx".to_string()];
        // A numeric enum arrives as the number it is.
        // An optional number arrives as a lent handle, an optional flag as -1, 0 or 1.
        let plain = |t: &Ty| match t {
            Ty::NumEnum(_) => "double".to_string(),
            Ty::Nullable(inner, _) if matches!(**inner, Ty::Num | Ty::NumEnum(_)) => "JsValue".to_string(),
            Ty::Nullable(inner, _) if matches!(**inner, Ty::Bool | Ty::Enum(_)) => "int32_t".to_string(),
            // A union arrives as a lent handle unless decoded by a C trampoline.
            Ty::NamedUnion(_) | Ty::Nullable(..) if matches!(t, Ty::NamedUnion(_)) || matches!(t, Ty::Nullable(inner, _) if matches!(**inner, Ty::NamedUnion(_))) => "JsValue".to_string(),
            t => typed(t, false, &cx),
        };
        // `ctx` is the callback's own: a parameter of that name is `ctx_`.
        params.extend(c.params.iter().map(|(n, t)| {
            let n = ident(n);
            let pty = decodable_union(t).unwrap_or_else(|| plain(t));
            format!("{pty} {}", if n == "ctx" { "ctx_".to_string() } else { n })
        }));
        let cb_from = format!("{}_from", c.name);
        let cb_raw = raw_fn(&raw_prefix, &cb_from);
        let fn_ret = plain(&c.ret);
        types += &format!("\ntypedef {fn_ret} (*{}_fn)({});\n", c.name, params.join(", "));
        if tramp {
            let mut raw_types = vec![format!("{}_fn ts_fn", c.name), "void* ts_ctx".to_string()];
            let mut tramp_param_types = vec![format!("{}_fn", c.name), "void*".to_string()];
            let mut decode_stmts = String::new();
            let mut call_args = vec!["ts_ctx".to_string()];
            let mut release_stmts = String::new();
            for (i, (_, t)) in c.params.iter().enumerate() {
                if let Some(u) = decodable_union(t) {
                    raw_types.push(format!("uint32_t a{i}"));
                    tramp_param_types.push("uint32_t".to_string());
                    decode_stmts += &format!(" if (a{i}) ts_raw_js_retain(a{i}); {u} ts_u{i} = {u}_from_handle(a{i});");
                    call_args.push(format!("ts_u{i}"));
                    release_stmts += &format!(" {u}_release(ts_u{i});");
                } else {
                    let pt = plain(t);
                    raw_types.push(format!("{pt} a{i}"));
                    tramp_param_types.push(pt);
                    call_args.push(format!("a{i}"));
                }
            }
            let call_line = if c.ret == Ty::Void {
                format!("ts_fn({});{release_stmts}", call_args.join(", "))
            } else {
                format!("{fn_ret} ts_r = ts_fn({});{release_stmts} return ts_r;", call_args.join(", "))
            };
            types += &format!(
                "typedef {fn_ret} (*ts_tramp_{name}_fn)({});\n\
                 static inline {fn_ret} ts_tramp_{name}({}) {{{decode_stmts} {call_line} }}\n\
                 {} uint32_t {cb_raw}(ts_tramp_{name}_fn, {name}_fn, void*);\n\
                 static inline {name} {name}_from({name}_fn fn, void* ctx) {{ return ({name}){{ {cb_raw}(ts_tramp_{name}, fn, ctx) }}; }}\n\
                 static inline TsCb {name}_arg_fn({name}_fn fn) {{ return (TsCb){{ fn ? {cb_raw}(ts_tramp_{name}, fn, (void*)0) : 0, fn != ({name}_fn)0 }}; }}\n",
                tramp_param_types.join(", "),
                raw_types.join(", "),
                import_attr(wasm_module, &cb_from),
                name = c.name,
            );
        } else {
            types += &format!("{} uint32_t {cb_raw}({}_fn, void*);\n", import_attr(wasm_module, &cb_from), c.name);
            types += &format!("static inline {0} {0}_from({0}_fn fn, void* ctx) {{ return ({0}){{ {cb_raw}(fn, ctx) }}; }}\n", c.name);
            types += &format!("static inline TsCb {0}_arg_fn({0}_fn fn) {{ return (TsCb){{ fn ? {cb_raw}(fn, (void*)0) : 0, fn != ({0}_fn)0 }}; }}\n", c.name);
        }
        types += &format!(
            "static inline TsCb {0}_arg_self({0} x) {{ return (TsCb){{ x.h, false }}; }}\n\
             static inline TsCb {0}_arg_null(const void* p) {{ (void)p; return (TsCb){{ 0, false }}; }}\n\
             #define {0}_arg(x) _Generic((x), {0}: {0}_arg_self, {0}_fn: {0}_arg_fn, TS_NULL_ARMS({0}_arg_null))(x)\n",
            c.name
        );
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
        .chain(runtime_names())
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
        for s in ["fn", "from", "arg", "arg_self", "arg_fn", "arg_null"] {
            global_taken.insert(format!("{}_{s}", c.name));
        }
    }
    for u in &m.unions {
        for id in (NAMING.type_names)(&u.name, &u.name, Kind::Union, m) {
            global_taken.insert(id);
        }
    }
    for f in &m.funcs {
        global_taken.insert(f.name.clone());
    }
    let mut rest_list_methods: HashSet<(String, String)> = HashSet::new();
    let mut class_bodies = Vec::with_capacity(m.classes.len());
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
        let macros: Vec<(String, bool)> = c
            .funcs
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let overloads = family(f, &c.funcs, &cx).map(|(at, members)| (at, members.iter().map(|o| format!("{}_{}", c.name, o.name)).collect(), members));
                let earlier: Vec<&Func> = c.funcs[..i].iter().collect();
                let list_name = format!("{}_{}_list", c.name, f.name);
                let allow_list = f.rest && !global_taken.contains(&list_name);
                let (m_out, emitted_list) = call_macros(&format!("{}_{}", c.name, f.name), Class::has_self(f).then_some(c.name.as_str()), f, &earlier, overloads, &descendants, &remote_as, &as_macro, allow_list, &cx);
                if emitted_list {
                    global_taken.insert(list_name);
                    rest_list_methods.insert((c.name.clone(), f.name.clone()));
                }
                (m_out, emitted_list)
            })
            .collect();
        for (f, (ms, _)) in c.funcs.iter().zip(&macros) {
            func(&mut body, Some(&c.name), f, wasm_module, &raw_prefix, &cx, !ms.is_empty());
        }
        // After every function: a macro may choose a later one.
        for (f, (ms, _)) in c.funcs.iter().zip(macros) {
            body += &with_doc(ms, &format!("{}_{}", c.name, f.name), f);
        }
        class_bodies.push((c, path, body));
    }
    // Emit inherited aliases in a second pass so `_list` macros exist regardless of class order.
    for (c, path, mut body) in class_bodies {
        let mut own_methods: HashSet<&str> = c.funcs.iter().map(|f| f.name.as_str()).chain(c.consts.iter().map(|(k, _)| k.as_str())).collect();
        own_methods.extend(["cast", "try_cast", "as_array", "iterate", "iterate_async", "iterate_next"]);
        let mut ancestor_includes = Vec::new();
        let mut setters: Vec<(&Func, &str)> = c.funcs.iter().filter(|f| matches!(f.call, Call::Set(_))).map(|f| (f, c.name.as_str())).collect();
        let mut methods = c.funcs.iter().any(|f| matches!(f.call, Call::Method(_)));
        let ancestors = all_ancestors(m, &c.name);
        for a in &ancestors {
            let Some(a_class) = class_by_name.get(a.as_str()) else { continue };
            methods |= a_class.funcs.iter().any(|f| matches!(f.call, Call::Method(_)));
            // A family of one function per literal too big to repeat in every
            // subclass (the DOM's 229 `querySelector_<tag>`) is named only by
            // a type that takes it directly: `Document_querySelector_canvas`,
            // and `ParentNode_querySelector_div(frameset)` for the rest.
            let mut families: HashMap<&str, usize> = HashMap::new();
            for f in &a_class.funcs {
                if let Some((import, _)) = &f.shared {
                    *families.entry(import.as_str()).or_default() += 1;
                }
            }
            let direct = c.ancestors.contains(a);
            for af in a_class.funcs.iter().filter(|af| Class::has_self(af)) {
                if !direct && af.shared.as_ref().is_some_and(|(import, _)| families[import.as_str()] > MAX_INHERITED_FAMILY) {
                    continue;
                }
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
                if rest_list_methods.contains(&(a.clone(), af.name.clone())) {
                    let list_alias = format!("{alias}_list");
                    if global_taken.insert(list_alias.clone()) {
                        body += &format!("#define {list_alias} {a}_{}_list\n", af.name);
                        rest_list_methods.insert((c.name.clone(), af.name.clone()));
                    }
                }
                if matches!(af.call, Call::Set(_)) {
                    setters.push((af, a));
                }
            }
        }
        // An object a caller fills in, not one with methods: made in one expression. A long list of
        // properties none optional is a map of keys to types (`ElementTagNameMap`), not made by hand.
        let optional = setters.iter().any(|(f, _)| matches!(f.params.first(), Some((_, Ty::Nullable(_, a))) if a.optional()));
        if !methods && (optional || setters.len() <= 32) && c.funcs.iter().any(|f| matches!(f.call, Call::Create(_))) {
            body += &bag(&c.name, &setters, &cx, m);
        }
        modules.entry(c.module.clone()).or_default().push(path.clone());
        out.push((path.clone(), header(&c.module, &ancestor_includes, body)));
    }
    let mut functions: BTreeMap<String, String> = Default::default();
    let macros: Vec<(String, bool)> = m
        .funcs
        .iter()
        .enumerate()
        .map(|(i, f)| {
            // Overloads in one module: in one header.
            let overloads = family(f, &m.funcs, &cx).filter(|(_, members)| members.iter().all(|o| o.module == f.module)).map(|(at, members)| (at, members.iter().map(|o| o.name.clone()).collect(), members));
            let earlier: Vec<&Func> = m.funcs[..i].iter().filter(|g| g.module == f.module).collect();
            let list_name = format!("{}_list", f.name);
            let allow_list = f.rest && !global_taken.contains(&list_name);
            let (ms, emitted_list) = call_macros(&f.name, None, f, &earlier, overloads, &descendants, &remote_as, &as_macro, allow_list, &cx);
            if emitted_list {
                global_taken.insert(list_name);
            }
            (ms, emitted_list)
        })
        .collect();
    for (f, (ms, _)) in m.funcs.iter().zip(&macros) {
        func(functions.entry(f.module.clone()).or_default(), None, f, wasm_module, &raw_prefix, &cx, !ms.is_empty());
    }
    for (f, (ms, _)) in m.funcs.iter().zip(macros) {
        *functions.entry(f.module.clone()).or_default() += &with_doc(ms, &f.name, f);
    }
    let is_internal_helper = |name: &str| {
        ["ts_arity_", "ts_fam_", "ts_wrap_", "ts_rest_", "ts_r0_", "ts_r1_"].iter().any(|p| name.starts_with(p))
    };
    for (module, body) in functions {
        for line in body.lines().map(str::trim) {
            if let Some(rest) = line.strip_prefix("static inline ")
                && let Some((head, _)) = rest.split_once('(')
                && let Some(name) = head.split_whitespace().last()
                && !name.is_empty()
                && !is_internal_helper(name)
            {
                all_names.push(name.to_string());
            } else if let Some(rest) = line.strip_prefix("#define ")
                && let Some((name, _)) = rest.split_once('(')
            {
                let name = name.trim();
                if !name.is_empty() && !is_internal_helper(name) {
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
    let meta_structs_set: HashSet<&str> = meta_structs.iter().map(String::as_str).collect();
    let c_subtypes: BTreeMap<String, Vec<String>> = descendants
        .iter()
        .filter(|(base, _)| meta_structs_set.contains(base.as_str()))
        .map(|(base, subs)| (base.clone(), subs.clone()))
        .collect();
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
        c_subtypes,
        ..Default::default()
    };
    out.push((super::super::pkg_meta::META_FILE.into(), meta.to_toml()));
    Ok(out)
}
