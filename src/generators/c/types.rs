use super::super::comments::{Style, comment};
use super::super::model::{Call, Class, Func, Model, NamedUnion, Ty};
use super::{Ctx, T, ident, import_attr, raw_fn};

pub(super) fn raw(t: &Ty) -> &'static str {
    use super::super::wasm::AbiSlot::*;
    match super::super::wasm::ret_abi(t) {
        Void => "void",
        F64 => "double",
        I32 => "int32_t",
        U32 => "uint32_t",
    }
}

pub(super) fn typed(t: &Ty, param: bool, cx: &Ctx) -> String {
    match t {
        Ty::Void => "void".into(),
        Ty::Num => "double".into(),
        Ty::Bool => "bool".into(),
        Ty::Str if param => "const char*".into(),
        Ty::Str => "JsString".into(),
        Ty::Any => "JsValue".into(),
        Ty::Array => "JsArray".into(),
        Ty::ArrayOf(e) => format!("JsArray_{}", element(e, cx)),
        Ty::Record(v) => format!("JsRecord_{}", element(v, cx)),
        Ty::Tuple(es) => tuple_name(es, cx),
        Ty::Obj(n) | Ty::Enum(n) | Ty::NumEnum(n) => n.clone(),
        // An optional number or flag says whether it is there; an optional enum is `E_absent` when it is not.
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Num | Ty::NumEnum(_)) => "JsOptionalDouble".into(),
        Ty::Nullable(inner, _) if **inner == Ty::Bool => "JsOptionalBool".into(),
        // NULL, or a null handle, is the absent value.
        Ty::Nullable(inner, _) => typed(inner, param, cx),
        // A union a handle can be decoded to: a tagged struct. Any other is a value,
        // but for what goes in, where the caller knows the member.
        Ty::NamedUnion(n) if cx.decodable(n) || param && cx.takes(n) => n.clone(),
        Ty::NamedUnion(_) => "JsValue".into(),
        Ty::Union(_) => panic!("{T} generator cannot express a union: the target config must name or erase unions"),
        Ty::Key(_) | Ty::Mapped(_) | Ty::Listener(_) => panic!("{T} generator cannot express keyed generics: the target config must expand them"),
        Ty::Param(_) | Ty::Generic(..) => panic!("{T} generator cannot express generics: the target config must erase them"),
    }
}

// An array's element type, as a name: `double`, `JsString`, `Element`, `JsArray_double`.
pub(super) fn element(t: &Ty, cx: &Ctx) -> String {
    match t {
        Ty::Str => "JsString".into(),
        t => typed(t, false, cx),
    }
}

pub(super) fn can_member_array(t: &Ty, cx: &Ctx) -> bool {
    match t {
        Ty::Str | Ty::Num | Ty::Bool | Ty::Enum(_) | Ty::NumEnum(_) | Ty::Array | Ty::ArrayOf(_) | Ty::Tuple(_) | Ty::Record(_) => true,
        Ty::Obj(n) => !cx.callbacks.contains(n),
        _ => false,
    }
}

pub(super) fn array_union_members<'a>(e: &Ty, m: &'a Model, cx: &Ctx) -> Option<(&'a NamedUnion, Vec<(usize, &'a Ty, String)>)> {
    let Ty::NamedUnion(n) = e else { return None };
    if !cx.takes(n) {
        return None;
    }
    let u = m.unions.iter().find(|u| u.name == *n)?;
    if u.parts.len() > 16 {
        return None;
    }
    let mut seen = std::collections::HashSet::from([format!("JsArray_{n}")]);
    let mut pairs = Vec::new();
    for (i, part) in u.parts.iter().enumerate() {
        if !can_member_array(part, cx) {
            continue;
        }
        let mem_arr_ty = format!("JsArray_{}", element(part, cx));
        if seen.insert(mem_arr_ty.clone()) {
            pairs.push((i, part, mem_arr_ty));
        }
    }
    (!pairs.is_empty()).then_some((u, pairs))
}

// The typed arrays, records and tuples a type names, innermost first.
pub(super) fn arrays(t: &Ty, out: &mut Vec<Ty>) {
    match t {
        Ty::ArrayOf(e) | Ty::Record(e) => arrays(e, out),
        Ty::Tuple(es) => es.iter().for_each(|e| arrays(e, out)),
        Ty::Nullable(inner, _) => return arrays(inner, out),
        _ => return,
    }
    if !out.contains(t) {
        out.push(t.clone());
    }
}

fn tuple_name(es: &[Ty], cx: &Ctx) -> String {
    let elems: Vec<String> = es.iter().map(|e| element(e, cx)).collect();
    if elems.len() >= 5 && elems.iter().all(|e| e == &elems[0]) {
        return format!("JsTuple{}_{}", elems.len(), elems[0]);
    }
    let joined = elems.join("_");
    if elems.len() > 3 && joined.len() > 40 {
        let full = format!("JsTuple_{joined}");
        return format!(
            "JsTuple{}_{}_{}_{:04x}",
            elems.len(),
            elems[0],
            elems.last().unwrap(),
            (crate::fnv1a64(&full) & 0xffff) as u16
        );
    }
    format!("JsTuple_{joined}")
}

// A union's member name: `number`, `string`, `Element`, `array`.
fn union_field(t: &Ty) -> String {
    match t {
        Ty::Num => "number".into(),
        Ty::Str => "string".into(),
        Ty::Bool => "boolean".into(),
        Ty::Obj(n) | Ty::Enum(n) => n.clone(),
        Ty::Array | Ty::ArrayOf(_) | Ty::Tuple(_) => "array".into(),
        Ty::Record(_) => "record".into(),
        t => panic!("{T} generator: a union member it cannot hold: {t:?}"),
    }
}

pub(in super::super) fn union_fields(u: &NamedUnion) -> Vec<String> {
    let raws: Vec<String> = u.parts.iter().map(union_field).collect();
    let union_words: std::collections::HashSet<String> = crate::generators::names::words(&u.name)
        .into_iter()
        .map(|w| w.to_lowercase())
        .collect();
    let mut out: Vec<String> = raws
        .iter()
        .map(|raw| {
            if u.name.len() + raw.len() <= 64 || !raw.contains('_') {
                return raw.clone();
            }
            let kept: Vec<&str> = raw
                .split('_')
                .filter(|seg| !seg.is_empty())
                .filter(|seg| {
                    let ws = crate::generators::names::words(seg);
                    ws.is_empty() || !ws.iter().all(|w| union_words.contains(&w.to_lowercase()))
                })
                .collect();
            let mut cand = kept.join("_");
            if !cand.is_empty() {
                while raws.iter().any(|r| *r == format!("{}_{cand}", u.name)) {
                    cand.push_str("_arm");
                }
            }
            let valid = !cand.is_empty()
                && cand.as_bytes()[0].is_ascii_alphabetic()
                && cand.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                && !super::KEYWORDS.contains(&cand.as_str())
                && !matches!(cand.as_str(), "ts_cstr" | "cstr");
            if valid { cand } else { raw.clone() }
        })
        .collect();
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for f in &out {
        *counts.entry(f.clone()).or_default() += 1;
    }
    for (i, f) in out.iter_mut().enumerate() {
        if counts.get(f).copied().unwrap_or(0) > 1 {
            *f = raws[i].clone();
        }
    }
    out
}

// A union: a struct with a tag and its members, tag constants, and
// conversions to and from handles: `Id_from_handle` takes a handle a function
// returned; `Id_to_handle` gives one the caller releases.
pub(super) fn union_type(u: &NamedUnion, m: &Model, wasm_module: &str, raw_prefix: &str, cx: &Ctx) -> (String, String) {
    // An enum of strings crosses as a handle by its place among the enums.
    let enum_id = |e: &str| m.enums.iter().position(|x| x.0 == e).unwrap_or(0);
    let n = &u.name;
    let has_str = u.parts.contains(&Ty::Str);
    let field_names = union_fields(u);
    let mut fields: Vec<String> = u
        .parts
        .iter()
        .zip(&field_names)
        .map(|(p, f)| format!("{} {f};", if *p == Ty::Str { "JsString".to_string() } else { typed(p, false, cx) }))
        .collect();
    if has_str {
        fields.push("const char* ts_cstr;".into());
    }
    // What no test tells apart is also taken as the value it is: tag -3.
    if !cx.decodable(n) {
        fields.push("JsValue ts_value;".into());
    }
    let tags: Vec<String> = field_names.iter().enumerate().map(|(i, f)| format!("{n}_{f} = {i}")).collect();
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
        .zip(&field_names)
        .enumerate()
        .map(|(i, (p, f))| match p {
            Ty::Num => format!(" case {i}: return ts_raw_js_from_number(u.as.{f});"),
            Ty::Bool => format!(" case {i}: return ts_raw_js_from_bool(u.as.{f});"),
            Ty::Enum(e) => format!(" case {i}: return {enum_value_fn}({}, u.as.{f});", enum_id(e)),
            _ => format!(" case {i}: ts_raw_js_retain(u.as.{f}.h); return u.as.{f}.h;"),
        })
        .collect();
    if has_str {
        to.push_str(" case -2: return u.as.ts_cstr ? ts_raw_js_from_string(u.as.ts_cstr, ts_strlen(u.as.ts_cstr)) : 0;");
    }
    if !cx.decodable(n) {
        to.push_str(" case -3: ts_raw_js_retain(u.as.ts_value.h); return u.as.ts_value.h;");
    }
    let from: String = u
        .parts
        .iter()
        .zip(&field_names)
        .enumerate()
        .map(|(i, (p, f))| match p {
            Ty::Num => format!(" case {i}: u.as.{f} = ts_raw_js_to_number(h); ts_raw_js_release(h); break;"),
            Ty::Bool => format!(" case {i}: u.as.{f} = ts_raw_js_to_bool(h) != 0; ts_raw_js_release(h); break;"),
            Ty::Str => format!(" case {i}: u.as.{f} = (JsString){{ h }}; break;"),
            Ty::Enum(e) => format!(" case {i}: u.as.{f} = ({e}){enum_index_fn}({}, h); ts_raw_js_release(h); break;", enum_id(e)),
            p => format!(" case {i}: u.as.{f} = ({}){{ h }}; break;", typed(p, false, cx)),
        })
        .collect();
    let release: String = u
        .parts
        .iter()
        .zip(&field_names)
        .enumerate()
        .filter(|(_, (p, _))| !matches!(p, Ty::Num | Ty::Bool | Ty::Enum(_) | Ty::NumEnum(_)))
        .map(|(i, (_, f))| format!(" case {i}: ts_raw_js_release(u.as.{f}.h); break;"))
        .collect();
    let mut functions = format!("static inline uint32_t {n}_to_handle({n} u) {{ switch (u.tag) {{{to} }} return 0; }}\n");
    // Telling the member of a handle is for what comes back from JavaScript.
    if cx.decodable(n) {
        let tag_attr = import_attr(wasm_module, &format!("{n}_tag"));
        let tag_raw = raw_fn(raw_prefix, &format!("{n}_tag"));
        functions = format!(
            "{tag_attr} int32_t {tag_raw}(uint32_t, int32_t);\n{functions}\
             static inline {n} {n}_from_handle(uint32_t h) {{ {n} u = {{ .tag = -1 }}; if (!h) return u; u.tag = {tag_raw}(h, 0); switch (u.tag) {{{from} default: ts_raw_js_release(h); break; }} return u; }}\n\
             static inline void {n}_release({n} u) {{ switch (u.tag) {{{release} default: break; }} }}\n"
        );
    }
    // Made from a member: `Value_of_number(42)`, or by its type, `Value_of(widget)`.
    let field_type = |p: &Ty| if *p == Ty::Str { "JsString".to_string() } else { typed(p, false, cx) };
    for (p, f) in u.parts.iter().zip(&field_names) {
        functions += &format!("static inline {n} {n}_of_{f}({} v) {{ return ({n}){{ .tag = {n}_{f}, .as.{f} = v }}; }}\n", field_type(p));
    }
    if has_str {
        functions += &format!("static inline {n} {n}_of_cstr(const char* v) {{ return ({n}){{ .tag = -2, .as.ts_cstr = v }}; }}\n");
    }
    if !cx.decodable(n) {
        functions += &format!("static inline {n} {n}_of_ts_value(JsValue v) {{ return ({n}){{ .tag = -3, .as.ts_value = v }}; }}\n");
    }
    if let Some(choices) = of_choices(u, cx) {
        functions += &format!("static inline {n} {n}_of_self({n} u) {{ return u; }}\n");
        functions += &format!("static inline {n} {n}_of_null(const void* p) {{ (void)p; return ({n}){{ .tag = -1 }}; }}\n");
        for (sub, base, f) in subtype_arms(u, cx) {
            let name = format!("{n}_of_{f}_from_{sub}");
            if choices.iter().any(|(_, c)| *c == name) {
                functions += &format!("static inline {n} {name}({sub} v) {{ return {n}_of_{f}(({base}){{ v.h }}); }}\n");
            }
        }
        let mut choices: Vec<String> = choices.iter().map(|(t, f)| format!("{t}: {f}")).collect();
        if !cx.decodable(n) {
            choices.push(format!("JsValue: {n}_of_ts_value"));
        }
        functions += &format!("#define {n}_of(x) _Generic((x), {n}: {n}_of_self, TS_NULL_ARMS({n}_of_null), {}, default: ts_no_member)(x)\n", choices.join(", "));
    }
    // A field every member has, read without a switch (marked's `Marked_Token_get_type(t)`).
    for (p, t) in m.shared_fields(u) {
        // A handle's type is none at a null handle.
        let handle = |t: &Ty| matches!(t, Ty::Str | Ty::Any | Ty::Array | Ty::ArrayOf(_) | Ty::Record(_) | Ty::Obj(_));
        let ret = typed(&t, false, cx);
        let tail = match &t {
            Ty::Num => "double v = ts_raw_js_to_number(r); ts_raw_js_release(r); return v;".to_string(),
            Ty::Bool => "bool v = ts_raw_js_to_bool(r) != 0; ts_raw_js_release(r); return v;".into(),
            Ty::Nullable(i, _) if **i == Ty::Num => "JsOptionalDouble v = { r != 0, r ? ts_raw_js_to_number(r) : 0 }; ts_raw_js_release(r); return v;".into(),
            Ty::Nullable(i, _) if **i == Ty::Bool => "JsOptionalBool v = { r != 0, r && ts_raw_js_to_bool(r) != 0 }; ts_raw_js_release(r); return v;".into(),
            Ty::Nullable(i, _) if handle(i) => format!("return ({ret}){{ r }};"),
            t if handle(t) => format!("return ({ret}){{ r }};"),
            Ty::NamedUnion(n) if cx.decodable(n) => format!("{n} v = {n}_from_handle(r); ts_raw_js_release(r); return v;"),
            _ => continue,
        };
        functions += &format!(
            "static inline {ret} {n}_get_{}({n} u) {{ uint32_t h = {n}_to_handle(u); uint32_t r = ts_raw_js_object_get(h, {p:?}, {}); ts_raw_js_release(h); {tail} }}\n",
            ident(&p),
            p.len()
        );
    }
    (decl, functions)
}

// What `U_of(x)` picks by the C type of `x`, as (type, function), where each member has types of its own.
pub(super) fn of_choices(u: &NamedUnion, cx: &Ctx) -> Option<Vec<(String, String)>> {
    let n = &u.name;
    let has_num = u.parts.contains(&Ty::Num);
    // An enum's values are ints, as a number's or a flag's may be: one enum
    // member, and no other that takes an int, is told apart by its type.
    let enums = u.parts.iter().filter(|p| matches!(p, Ty::Enum(_) | Ty::NumEnum(_))).count();
    let enum_ok = enums == 1 && !u.parts.iter().any(|p| *p == Ty::Num || *p == Ty::Bool);
    let mut seen = std::collections::BTreeSet::new();
    let per_member: Option<Vec<Vec<(String, String)>>> = u
        .parts
        .iter()
        .zip(union_fields(u))
        .map(|(p, f)| {
            let pairs: Vec<(String, String)> = if *p == Ty::Str {
                [("JsString", "string"), ("const char*", "cstr"), ("char*", "cstr")].iter().map(|(t, suffix)| ((*t).to_string(), format!("{n}_of_{suffix}"))).collect()
            } else {
                match p {
                    Ty::Enum(_) | Ty::NumEnum(_) if enum_ok => vec![(typed(p, true, cx), format!("{n}_of_{f}"))],
                    _ => generic_types(p, has_num, cx)?.into_iter().map(|t| (t, format!("{n}_of_{f}"))).collect(),
                }
            };
            pairs.iter().all(|(t, _)| seen.insert(t.clone())).then_some(pairs)
        })
        .collect();
    let mut out = per_member?.concat();
    // A subclass of a member is that member: `Mesh_new(geometry, basic_material)`.
    for (sub, _, f) in subtype_arms(u, cx) {
        if seen.insert(sub.clone()) {
            out.push((sub.clone(), format!("{n}_of_{f}_from_{sub}")));
        }
    }
    Some(out)
}

// A union's members that are classes with subclasses here, but not too many:
// (subclass, member, the member's field). Narrower bases come first so a
// subtype picks its nearest ancestor when a union names both.
pub(super) fn subtype_arms(u: &NamedUnion, cx: &Ctx) -> Vec<(String, String, String)> {
    const MAX_SUBTYPES: usize = 200;
    let mut by_base: Vec<(String, String, &Vec<String>)> = u
        .parts
        .iter()
        .zip(union_fields(u))
        .filter_map(|(p, f)| match p {
            Ty::Obj(base) => cx.subtypes.get(base).filter(|subs| subs.len() <= MAX_SUBTYPES).map(|subs| (base.clone(), f, subs)),
            _ => None,
        })
        .collect();
    by_base.sort_by_key(|(_, _, subs)| subs.len());
    by_base
        .into_iter()
        .flat_map(|(base, f, subs)| subs.iter().map(move |s| (s.clone(), base.clone(), f.clone())))
        .filter(|(s, ..)| !u.parts.contains(&Ty::Obj(s.clone())))
        .collect()
}

// A union a value can be made of: its members have C types and names of their own.
pub(super) fn typable(u: &NamedUnion) -> bool {
    if !u.type_params.is_empty() || !u.parts.iter().all(|p| matches!(p, Ty::Num | Ty::Str | Ty::Bool | Ty::Obj(_) | Ty::Enum(_) | Ty::Array | Ty::ArrayOf(_) | Ty::Tuple(_) | Ty::Record(_))) {
        return false;
    }
    let fields = union_fields(u);
    fields.iter().enumerate().all(|(i, f)| !fields[..i].contains(f))
}

// A tuple: `JsTuple_double_JsString`, made by `_new(1, "one")`, read by `_get_0`.
// What a getter returns is the caller's to release; a number or flag is read and released.
pub(super) fn tuple_type(es: &[Ty], cx: &Ctx) -> String {
    let name = typed(&Ty::Tuple(es.to_vec()), false, cx);
    let params: Vec<String> = es.iter().enumerate().map(|(i, e)| format!("{} v{i}", if *e == Ty::Str { "TsStr".into() } else { typed(e, false, cx) })).collect();
    let pushes: String = es
        .iter()
        .enumerate()
        .map(|(i, e)| match e {
            Ty::Num => format!(" {{ uint32_t h = ts_raw_js_from_number(v{i}); ts_raw_js_array_push(a.h, h); ts_raw_js_release(h); }}"),
            Ty::Bool => format!(" {{ uint32_t h = ts_raw_js_from_bool(v{i}); ts_raw_js_array_push(a.h, h); ts_raw_js_release(h); }}"),
            Ty::Str => format!(" {{ uint32_t h = ts_raw_js_from_string(v{i}.ptr, v{i}.len); ts_raw_js_array_push(a.h, h); ts_raw_js_release(h); }}"),
            // A union is a tagged struct: made a value, pushed, and released.
            Ty::NamedUnion(n) if cx.decodable(n) => format!(" {{ uint32_t h = {n}_to_handle(v{i}); ts_raw_js_array_push(a.h, h); ts_raw_js_release(h); }}"),
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
        let elem = typed(e, false, cx);
        let get = match e {
            Ty::Num => "double x = ts_raw_js_to_number(h); ts_raw_js_release(h); return x;".to_string(),
            Ty::Bool => "bool x = ts_raw_js_to_bool(h) != 0; ts_raw_js_release(h); return x;".to_string(),
            Ty::NamedUnion(n) if cx.decodable(n) => format!("return {n}_from_handle(h);"),
            _ => format!("return ({elem}){{ h }};"),
        };
        out += &format!("static inline {elem} {name}_get_{i}({name} a) {{ uint32_t h = ts_raw_js_array_get(a.h, {i}); {get} }}\n");
    }
    out + &format!("static inline JsArray {name}_any({name} a) {{ return (JsArray){{ a.h }}; }}\n")
}

// A typed array: `JsArray_double`, with `_new`, `_length`, `_get`, `_push`, `_from` and `_of`.
// What `_get` returns is the caller's to release; a number or flag is read and released.
pub(super) fn array_type(e: &Ty, m: &Model, raw_prefix: &str, cx: &Ctx, upcast_macro: Option<&str>, short_list_macro_ok: bool, member_arrays: &[String]) -> String {
    let (name, elem) = (format!("JsArray_{}", element(e, cx)), typed(e, false, cx));
    if *e == Ty::Str {
        return [
            format!("typedef struct {{ uint32_t h; }} {name};"),
            format!("static inline {name} {name}_new(void) {{ return ({name}){{ ts_raw_js_array_new() }}; }}"),
            format!("static inline uint32_t {name}_length({name} a) {{ return ts_raw_js_array_length(a.h); }}"),
            format!("static inline {elem} {name}_get({name} a, uint32_t i) {{ uint32_t h = ts_raw_js_array_get(a.h, i); return (JsString){{ h }}; }}"),
            format!("static inline void {name}_push({name} a, TsStr v) {{ uint32_t h = ts_raw_js_from_string(v.ptr, v.len); ts_raw_js_array_push(a.h, h); ts_raw_js_release(h); }}"),
            format!("static inline {name} {name}_from(const char* const* items, uint32_t len) {{ {name} a = {name}_new(); for (uint32_t i = 0; i < len; i++) {{ uint32_t h = ts_raw_js_from_string(items[i], items[i] ? ts_strlen(items[i]) : 0); ts_raw_js_array_push(a.h, h); ts_raw_js_release(h); }} return a; }}"),
            format!("static inline {name} ts_{name}_new(void) {{ return {name}_new(); }}"),
            format!("static inline {name} ts_{name}_from(const TsStr* items, uint32_t len) {{ {name} a = {name}_new(); for (uint32_t i = 0; i < len; i++) {{ uint32_t h = items[i].ptr ? ts_raw_js_from_string(items[i].ptr, items[i].len) : 0; ts_raw_js_array_push(a.h, h); if (h) ts_raw_js_release(h); }} return a; }}"),
            format!("#define {name}_push(a, v) ({name}_push)((a), TS_STR(v))"),
            format!("#define {name}_of(...) TS_LIST(ts_{name}, TsStr, TS_STR, __VA_ARGS__)"),
            format!("#ifndef js_strings\n#define js_strings(...) {name}_of(__VA_ARGS__)\n#endif"),
            format!("static inline JsArray {name}_any({name} a) {{ return (JsArray){{ a.h }}; }}"),
        ]
        .map(|l| l + "\n")
        .concat();
    }
    let enum_value_fn = raw_fn(raw_prefix, "js_enum_value");
    let enum_index_fn = raw_fn(raw_prefix, "js_enum_index");
    let enum_id = |en: &str| m.enums.iter().position(|x| x.0 == en).unwrap_or(0);
    let (get, push_param, push) = match e {
        Ty::NamedUnion(n) if cx.decodable(n) => (format!("return {n}_from_handle(h);"), format!("{n} v"), format!("uint32_t h = {n}_to_handle(v); ts_raw_js_array_push(a.h, h); ts_raw_js_release(h);")),
        Ty::Num => ("double x = ts_raw_js_to_number(h); ts_raw_js_release(h); return x;".to_string(), "double v".to_string(), "uint32_t h = ts_raw_js_from_number(v); ts_raw_js_array_push(a.h, h); ts_raw_js_release(h);".to_string()),
        Ty::Bool => ("bool x = ts_raw_js_to_bool(h) != 0; ts_raw_js_release(h); return x;".to_string(), "bool v".to_string(), "uint32_t h = ts_raw_js_from_bool(v); ts_raw_js_array_push(a.h, h); ts_raw_js_release(h);".to_string()),
        Ty::Enum(en) => (
            format!("{en} x = ({en}){enum_index_fn}({}, h); ts_raw_js_release(h); return x;", enum_id(en)),
            format!("{en} v"),
            format!("uint32_t h = {enum_value_fn}({}, v); ts_raw_js_array_push(a.h, h); ts_raw_js_release(h);", enum_id(en)),
        ),
        Ty::NumEnum(en) => (
            format!("{en} x = ({en})(int32_t)ts_raw_js_to_number(h); ts_raw_js_release(h); return x;"),
            format!("{en} v"),
            "uint32_t h = ts_raw_js_from_number((double)(int32_t)v); ts_raw_js_array_push(a.h, h); ts_raw_js_release(h);".to_string(),
        ),
        _ => (format!("return ({elem}){{ h }};"), format!("{elem} v"), "ts_raw_js_array_push(a.h, v.h);".to_string()),
    };
    let mut lines = vec![
        format!("typedef struct {{ uint32_t h; }} {name};"),
        format!("static inline {name} {name}_new(void) {{ return ({name}){{ ts_raw_js_array_new() }}; }}"),
        format!("static inline uint32_t {name}_length({name} a) {{ return ts_raw_js_array_length(a.h); }}"),
        format!("static inline {elem} {name}_get({name} a, uint32_t i) {{ uint32_t h = ts_raw_js_array_get(a.h, i); {get} }}"),
        format!("static inline void {name}_push({name} a, {push_param}) {{ {push} }}"),
        format!("static inline {name} {name}_from(const {elem}* items, uint32_t len) {{ {name} a = {name}_new(); for (uint32_t i = 0; i < len; i++) ({name}_push)(a, items[i]); return a; }}"),
        // The same array, untyped.
        format!("static inline JsArray {name}_any({name} a) {{ return (JsArray){{ a.h }}; }}"),
    ];
    if let Some(as_m) = upcast_macro {
        lines.push(format!("#define {name}_push(a, v) ({name}_push)((a), {as_m}(v))"));
        lines.push(format!("#define {name}_of(...) TS_LIST({name}, {elem}, {as_m}, __VA_ARGS__)"));
    } else if let Ty::NamedUnion(n) = e && cx.decodable(n) && cx.of.contains_key(n) {
        // A union element is pushed as any member: `push(a, widget)`.
        lines.push(format!("#define {name}_push(a, v) ({name}_push)((a), {n}_of(v))"));
        lines.push(format!("#define {name}_of(...) TS_LIST({name}, {n}, {n}_of, __VA_ARGS__)"));
    } else {
        lines.push(format!("#define {name}_of(...) TS_LIST_RAW({name}, {elem}, __VA_ARGS__)"));
    }
    if !member_arrays.is_empty() {
        lines.push(format!("static inline {name} ts_arr_{name}_self({name} a) {{ return a; }}"));
        lines.push(format!("static inline {name} ts_arr_{name}_null(const void* p) {{ (void)p; return ({name}){{ 0 }}; }}"));
        let mut arms = Vec::with_capacity(member_arrays.len());
        for (i, mem_ty) in member_arrays.iter().enumerate() {
            lines.push(format!("static inline {name} ts_arr_{name}_{i}({mem_ty} a) {{ return ({name}){{ a.h }}; }}"));
            arms.push(format!("{mem_ty}: ts_arr_{name}_{i}"));
        }
        lines.push(format!("#define {name}_arg(x) _Generic((x), {name}: ts_arr_{name}_self, TS_NULL_ARMS(ts_arr_{name}_null), {}, default: ts_no_member)(x)", arms.join(", ")));
    }
    match e {
        Ty::Num => lines.push(format!("#ifndef js_numbers\n#define js_numbers(...) {name}_of(__VA_ARGS__)\n#endif")),
        Ty::Bool => lines.push(format!("#ifndef js_bools\n#define js_bools(...) {name}_of(__VA_ARGS__)\n#endif")),
        Ty::Obj(n) | Ty::Enum(n) | Ty::NumEnum(n) | Ty::NamedUnion(n)
            if short_list_macro_ok && !super::is_builtin_c_type(n) =>
        {
            lines.push(format!("#ifndef {n}_list\n#define {n}_list(...) {name}_of(__VA_ARGS__)\n#endif"));
        }
        _ => {}
    }
    lines.into_iter().map(|l| l + "\n").collect()
}

// A typed record: `JsRecord_double`, with `_new`, `_create`, `_get`, `_set`, `_has`, `_from`, `_from_z` and `_of(...)`.
pub(super) fn record_type(v: &Ty, m: &Model, raw_prefix: &str, cx: &Ctx, short_macro_ok: bool) -> String {
    let name = format!("JsRecord_{}", element(v, cx));
    let elem = typed(v, false, cx);
    let enum_value_fn = raw_fn(raw_prefix, "js_enum_value");
    let enum_index_fn = raw_fn(raw_prefix, "js_enum_index");
    let enum_id = |e: &str| m.enums.iter().position(|x| x.0 == e).unwrap_or(0);
    let entry_val = if *v == Ty::Str { "const char*".to_string() } else { elem.clone() };
    let (get_ret, get_body) = match v {
        Ty::Num => (
            "JsOptionalDouble".to_string(),
            "if (!ts_raw_js_object_has(r.h, k.ptr, k.len)) return js_none_double(); uint32_t h = ts_raw_js_object_get(r.h, k.ptr, k.len); double x = ts_raw_js_to_number(h); ts_raw_js_release(h); return js_some_double(x);".to_string(),
        ),
        Ty::Bool => (
            "JsOptionalBool".to_string(),
            "if (!ts_raw_js_object_has(r.h, k.ptr, k.len)) return js_none_bool(); uint32_t h = ts_raw_js_object_get(r.h, k.ptr, k.len); bool x = ts_raw_js_to_bool(h) != 0; ts_raw_js_release(h); return js_some_bool(x);".to_string(),
        ),
        Ty::Enum(e) => (
            e.clone(),
            format!("if (!ts_raw_js_object_has(r.h, k.ptr, k.len)) return {e}_absent; uint32_t h = ts_raw_js_object_get(r.h, k.ptr, k.len); {e} x = ({e}){enum_index_fn}({}, h); ts_raw_js_release(h); return x;", enum_id(e)),
        ),
        Ty::NumEnum(e) => (
            e.clone(),
            format!("if (!ts_raw_js_object_has(r.h, k.ptr, k.len)) return ({e})0; uint32_t h = ts_raw_js_object_get(r.h, k.ptr, k.len); {e} x = ({e})(int32_t)ts_raw_js_to_number(h); ts_raw_js_release(h); return x;"),
        ),
        Ty::NamedUnion(u) if cx.decodable(u) => (
            u.clone(),
            format!("return {u}_from_handle(ts_raw_js_object_get(r.h, k.ptr, k.len));"),
        ),
        _ => (
            elem.clone(),
            format!("return ({elem}){{ ts_raw_js_object_get(r.h, k.ptr, k.len) }};"),
        ),
    };
    let (set_param, set_body) = match v {
        Ty::Str => (
            "TsStr v".to_string(),
            "uint32_t h = ts_raw_js_from_string(v.ptr, v.len); ts_raw_js_object_set(r.h, k.ptr, k.len, h); ts_raw_js_release(h);".to_string(),
        ),
        Ty::Num => (
            "double v".to_string(),
            "uint32_t h = ts_raw_js_from_number(v); ts_raw_js_object_set(r.h, k.ptr, k.len, h); ts_raw_js_release(h);".to_string(),
        ),
        Ty::Bool => (
            "bool v".to_string(),
            "uint32_t h = ts_raw_js_from_bool(v); ts_raw_js_object_set(r.h, k.ptr, k.len, h); ts_raw_js_release(h);".to_string(),
        ),
        Ty::Enum(e) => (
            format!("{e} v"),
            format!("uint32_t h = {enum_value_fn}({}, v); ts_raw_js_object_set(r.h, k.ptr, k.len, h); ts_raw_js_release(h);", enum_id(e)),
        ),
        Ty::NumEnum(e) => (
            format!("{e} v"),
            "uint32_t h = ts_raw_js_from_number((double)(int32_t)v); ts_raw_js_object_set(r.h, k.ptr, k.len, h); ts_raw_js_release(h);".to_string(),
        ),
        Ty::NamedUnion(u) if cx.decodable(u) => (
            format!("{u} v"),
            format!("uint32_t h = {u}_to_handle(v); ts_raw_js_object_set(r.h, k.ptr, k.len, h); ts_raw_js_release(h);"),
        ),
        _ => (
            format!("{elem} v"),
            "ts_raw_js_object_set(r.h, k.ptr, k.len, v.h);".to_string(),
        ),
    };
    let item_val = if *v == Ty::Str { "ts_str_c(items[i].value)" } else { "items[i].value" };
    let set_macro = match v {
        Ty::Str => format!("#define {name}_set(r, k, v) ({name}_set)((r), TS_STR(k), TS_STR(v))"),
        Ty::NamedUnion(u) if cx.decodable(u) && cx.of.contains_key(u) => {
            format!("#define {name}_set(r, k, v) ({name}_set)((r), TS_STR(k), {u}_of(v))")
        }
        _ => format!("#define {name}_set(r, k, v) ({name}_set)((r), TS_STR(k), (v))"),
    };
    let mut lines = vec![
        format!("typedef struct {{ uint32_t h; }} {name};"),
        format!("typedef struct {{ const char* key; {entry_val} value; }} {name}_entry;"),
        format!("static inline {name} {name}_new(void) {{ return ({name}){{ ts_raw_js_object_new() }}; }}"),
        format!("static inline {name} {name}_create(void) {{ return {name}_new(); }}"),
        format!("static inline bool {name}_has({name} r, TsStr k) {{ return ts_raw_js_object_has(r.h, k.ptr, k.len) != 0; }}"),
        format!("#define {name}_has(r, k) ({name}_has)((r), TS_STR(k))"),
        format!("static inline {get_ret} {name}_get({name} r, TsStr k) {{ {get_body} }}"),
        format!("#define {name}_get(r, k) ({name}_get)((r), TS_STR(k))"),
        format!("static inline void {name}_set({name} r, TsStr k, {set_param}) {{ {set_body} }}"),
        set_macro,
        format!("static inline {name} {name}_from(const {name}_entry* items, uint32_t len) {{ {name} r = {name}_new(); for (uint32_t i = 0; i < len; i++) ({name}_set)(r, ts_str_c(items[i].key), {item_val}); return r; }}"),
        format!("static inline {name} {name}_from_z(const {name}_entry* items) {{ {name} r = {name}_new(); for (uint32_t i = 0; items[i].key; i++) ({name}_set)(r, ts_str_c(items[i].key), {item_val}); return r; }}"),
        format!("static inline JsValue {name}_any({name} r) {{ return (JsValue){{ r.h }}; }}"),
        format!("#define {name}_of(...) TS_RECORD({name}, __VA_ARGS__)"),
    ];
    match v {
        Ty::Any => lines.push(format!("#ifndef js_record\n#define js_record(...) TS_RECORD({name}, __VA_ARGS__)\n#endif")),
        Ty::Str => lines.push(format!("#ifndef js_string_record\n#define js_string_record(...) TS_RECORD({name}, __VA_ARGS__)\n#endif")),
        Ty::Num => lines.push(format!("#ifndef js_number_record\n#define js_number_record(...) TS_RECORD({name}, __VA_ARGS__)\n#endif")),
        Ty::Bool => lines.push(format!("#ifndef js_bool_record\n#define js_bool_record(...) TS_RECORD({name}, __VA_ARGS__)\n#endif")),
        Ty::Obj(n) | Ty::Enum(n) | Ty::NumEnum(n) | Ty::NamedUnion(n) if short_macro_ok && !super::is_builtin_c_type(n) => {
            lines.push(format!("#ifndef {n}_record\n#define {n}_record(...) TS_RECORD({name}, __VA_ARGS__)\n#endif"));
        }
        _ => {}
    }
    lines.into_iter().map(|l| l + "\n").collect()
}

fn func_param_ty(t: &Ty, cx: &Ctx) -> String {
    if *t == Ty::Str || matches!(t, Ty::Nullable(inner, _) if **inner == Ty::Str) {
        "TsStr".into()
    } else {
        typed(t, true, cx)
    }
}

// A function's comment, above what the caller names: its macro where it has one, else it.
pub(super) fn documented(f: &Func) -> String {
    f.doc.as_deref().map(|d| comment(d, "//", "", Style::C)).unwrap_or_default()
}

// `macros` (see `call_macros`) with the function's comment before the one the caller names.
pub(super) fn with_doc(macros: String, name: &str, f: &Func) -> String {
    match macros.find(&format!("#define {name}(")) {
        Some(at) => format!("{}{}{}", &macros[..at], documented(f), &macros[at..]),
        None => macros,
    }
}

pub(super) fn func(out: &mut String, owner: Option<&str>, f: &Func, wasm_module: &str, raw_prefix: &str, cx: &Ctx, macro_documented: bool) {
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
        params.push(format!("{} {name}", func_param_ty(t, cx)));
        let union = match t {
            Ty::NamedUnion(n) => Some(n),
            Ty::Nullable(inner, _) => match &**inner {
                Ty::NamedUnion(n) => Some(n),
                _ => None,
            },
            _ => None,
        };
        if let Some(n) = union.filter(|n| cx.takes(n)) {
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
        _ if union_ret.is_some_and(|(n, _)| cx.decodable(n)) => {
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
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Enum(_)) => format!("return ({}){call};", typed(inner, false, cx)),
        t => format!("return ({}){{ {call} }};", typed(t, false, cx)),
    };
    let decl = format!("{} {} {call_raw}({});\n", import_attr(wasm_module, import), raw(&f.ret), void(&raw_params));
    // Declared once per header, though several functions use it.
    // Only a shared import can repeat; checking others would be quadratic.
    if f.shared.is_none() || !out.contains(&decl) {
        *out += &decl;
    }
    if !macro_documented {
        *out += &documented(f);
    }
    *out += &format!("static inline {} {name}({}) {{ {pre}{body} }}\n", typed(&f.ret, false, cx), void(&params));
}

// A bag's fields kind by kind (see `TS_BAG` in the runtime): one for each property that can be set.
fn field(t: &Ty, cx: &Ctx) -> Option<(char, String)> {
    let optional = matches!(t, Ty::Nullable(inner, _) if matches!(**inner, Ty::Num | Ty::NumEnum(_) | Ty::Bool));
    let t = match t {
        Ty::Nullable(inner, _) => inner,
        t => t,
    };
    Some(match t {
        Ty::Bool => (if optional { 'B' } else { 'i' }, "int8_t".into()),
        Ty::Num | Ty::NumEnum(_) => (if optional { 'D' } else { 'd' }, "double".into()),
        Ty::Str => ('s', "const char*".into()),
        Ty::Enum(e) => ('i', e.clone()),
        Ty::NamedUnion(n) if cx.takes(n) => ('u', n.clone()),
        Ty::NamedUnion(_) | Ty::Any | Ty::Obj(_) | Ty::Array | Ty::ArrayOf(_) | Ty::Tuple(_) | Ty::Record(_) => ('h', typed(t, true, cx)),
        _ => return None,
    })
}

fn array_member_suffix(field_name: &str, part: &Ty, raw_suffix: &str, obj_count: usize, m: &Model) -> String {
    match part {
        Ty::Str => "strings".into(),
        Ty::Num | Ty::NumEnum(_) => "numbers".into(),
        Ty::Bool => "booleans".into(),
        Ty::Array | Ty::ArrayOf(_) | Ty::Tuple(_) => "arrays".into(),
        Ty::Record(_) => "records".into(),
        Ty::Obj(n) if n == "Date" => "dates".into(),
        Ty::Obj(n) if n == "JsSymbol" => "symbols".into(),
        Ty::Obj(n) if obj_count == 1 => {
            let orig = m.original_type_names.get(n).map_or(n.as_str(), String::as_str);
            if raw_suffix == field_name || orig.ends_with("DataItemOption") || orig.ends_with("ItemOption") || orig.ends_with("DataItem") {
                "items".into()
            } else {
                raw_suffix.into()
            }
        }
        _ => raw_suffix.into(),
    }
}

// `X_create(.a = 1, .b = "x")`: a struct of the settable properties, and the object made from what is named.
// Each field is by the type that declares its setter. A property set in more than one way is left out.
// A union's is a field for each member: its primary member's by the property's
// name (`.left = 16`), each other's with the member's (`.left_string = "10%"`,
// `.series_bar`, `.title_list`), set through the union.
pub(super) fn bag(name: &str, setters: &[(&Func, &str)], cx: &Ctx, m: &Model) -> String {
    let js = |f: &Func| match &f.call {
        Call::Set(js) => js.clone(),
        _ => unreachable!(),
    };
    let eligible: Vec<(&Func, &str)> = setters
        .iter()
        .copied()
        .filter(|(f, _)| {
            let prop = js(f);
            f.params.len() == 1
                && setters.iter().filter(|(g, _)| js(g) == prop).count() == 1
                && prop.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
                && prop.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        })
        .collect();
    let reserved_fields: std::collections::HashSet<String> = eligible.iter().map(|(f, _)| ident(&js(f))).collect();
    let mut used_fields = reserved_fields.clone();
    let mut helpers = String::new();
    let mut entries: Vec<String> = Vec::new();
    let prefix = super::effective_prefix(m).map(|p| format!("{p}_")).unwrap_or_default();
    let strip_prefix = |t: &str| {
        let t = m.original_type_names.get(t).map_or(t, String::as_str);
        match t.get(..prefix.len()) {
            Some(head) if !prefix.is_empty() && t.len() > prefix.len() && head.eq_ignore_ascii_case(&prefix) => t[prefix.len()..].to_string(),
            _ => t.to_string(),
        }
    };
    let count_objs = |u: &NamedUnion| {
        u.parts
            .iter()
            .filter(|p| matches!(p, Ty::Obj(n) if !cx.callbacks.contains(n) && n != "Date" && n != "JsSymbol"))
            .count()
    };
    for (f, owner) in eligible {
        let prop = js(f);
        let field_name = ident(&prop);
        let setter_owner = owner;
        let owner = if owner == name { "C_" } else { owner };
        let value = match &f.params[0].1 {
            Ty::Nullable(inner, _) => &**inner,
            t => t,
        };
        if let Ty::NamedUnion(n) = value
            && cx.takes(n)
            && let Some(u) = m.unions.iter().find(|u| u.name == *n)
        {
            // Without the package's prefix: `CanvasGradient`, not `Konva_CanvasGradient`.
            let (primary, suffixes) = m.member_names(u, &strip_prefix);
            let members = union_fields(u);
            let mut seen = std::collections::HashSet::new();
            for (i, part) in u.parts.iter().enumerate() {
                let Some((kind, ty)) = field(part, cx) else { continue };
                let member_field = if Some(i) == primary { field_name.clone() } else { format!("{field_name}_{}", suffixes[i]) };
                if !seen.insert(member_field.clone()) {
                    continue;
                }
                used_fields.insert(member_field.clone());
                if let Ty::Obj(cb) = part && cx.callbacks.contains(cb) {
                    let of = &members[i];
                    helpers += &format!(
                        "static inline void {name}__{member_field}({name} o, {cb}_fn fn) {{ {cb} cb = {cb}_from(fn, (void*)0); ({setter_owner}_{})(({setter_owner}){{ o.h }}, {n}_of_{of}(cb)); ts_raw_js_release(cb.h); }}\n",
                        f.name
                    );
                    entries.push(format!("F_##_N(C_, p, {cb}_fn, {member_field}, _{member_field})"));
                    let base = member_field.strip_suffix("_fn").unwrap_or(&member_field);
                    let mut cb_field = format!("{base}_cb");
                    if reserved_fields.contains(&cb_field) || !used_fields.insert(cb_field.clone()) {
                        cb_field = format!("{member_field}_cb");
                    }
                    if !reserved_fields.contains(&cb_field) && used_fields.insert(cb_field.clone()) {
                        helpers += &format!(
                            "static inline void {name}__{cb_field}({name} o, {cb} v) {{ ({setter_owner}_{})(({setter_owner}){{ o.h }}, {n}_of_{of}(v)); }}\n",
                            f.name
                        );
                        entries.push(format!("F_##_N(C_, h, {cb}, {cb_field}, _{cb_field})"));
                    }
                    continue;
                }
                let (param, arg, of) = match (kind, part) {
                    ('s', _) => ("TsStr".to_string(), "v.ptr", "cstr".to_string()),
                    (_, _) => (ty.clone(), "v", members[i].clone()),
                };
                helpers += &format!(
                    "static inline void {name}__{member_field}({name} o, {param} v) {{ ({setter_owner}_{})(({setter_owner}){{ o.h }}, {n}_of_{of}({arg})); }}\n",
                    f.name
                );
                entries.push(format!("F_##_N(C_, {kind}, {ty}, {member_field}, _{member_field})"));
                if let Ty::ArrayOf(inner) = part
                    && cx.array_arg.contains_key(&ty)
                    && let Some((inner_u, arr_members)) = array_union_members(inner, m, cx)
                {
                    let (_, inner_suffixes) = m.member_names(inner_u, &strip_prefix);
                    let inner_obj_count = count_objs(inner_u);
                    for (j, arr_part, mem_arr_ty) in arr_members {
                        let sub_field = format!("{field_name}_{}", array_member_suffix(&field_name, arr_part, &inner_suffixes[j], inner_obj_count, m));
                        if !reserved_fields.contains(&sub_field) && used_fields.insert(sub_field.clone()) {
                            helpers += &format!(
                                "static inline void {name}__{sub_field}({name} o, {mem_arr_ty} v) {{ ({setter_owner}_{})(({setter_owner}){{ o.h }}, {n}_of_{of}(({ty}){{ v.h }})); }}\n",
                                f.name
                            );
                            entries.push(format!("F_##_N(C_, h, {mem_arr_ty}, {sub_field}, _{sub_field})"));
                        }
                    }
                }
            }
            continue;
        }
        let Some((kind, ty)) = field(&f.params[0].1, cx) else { continue };
        // Where the setter is not `set_` and the property (a keyword, a function assigned), it is named.
        entries.push(if f.name == format!("set_{field_name}") { format!("F_({owner}, {kind}, {ty}, {field_name})") } else { format!("F_##_N({owner}, {kind}, {ty}, {field_name}, {})", f.name) });
        if let Ty::ArrayOf(inner) = value
            && cx.array_arg.contains_key(&ty)
            && let Some((u, arr_members)) = array_union_members(inner, m, cx)
        {
            let (_, suffixes) = m.member_names(u, &strip_prefix);
            let obj_count = count_objs(u);
            for (i, part, mem_arr_ty) in arr_members {
                let sub_field = format!("{field_name}_{}", array_member_suffix(&field_name, part, &suffixes[i], obj_count, m));
                if !reserved_fields.contains(&sub_field) && used_fields.insert(sub_field.clone()) {
                    helpers += &format!(
                        "static inline void {name}__{sub_field}({name} o, {mem_arr_ty} v) {{ ({setter_owner}_{})(({setter_owner}){{ o.h }}, ({ty}){{ v.h }}); }}\n",
                        f.name
                    );
                    entries.push(format!("F_##_N(C_, h, {mem_arr_ty}, {sub_field}, _{sub_field})"));
                }
            }
        }
        if let Ty::Obj(cb) = value && cx.callbacks.contains(cb) {
            let fn_field = format!("{field_name}_fn");
            if !reserved_fields.contains(&fn_field) && used_fields.insert(fn_field.clone()) {
                helpers += &format!(
                    "static inline void {name}__{fn_field}({name} o, {cb}_fn fn) {{ {cb} cb = {cb}_from(fn, (void*)0); ({setter_owner}_{})(({setter_owner}){{ o.h }}, cb); ts_raw_js_release(cb.h); }}\n",
                    f.name
                );
                entries.push(format!("F_##_N(C_, p, {cb}_fn, {fn_field}, _{fn_field})"));
            }
        }
    }
    if entries.is_empty() {
        return String::new();
    }
    format!("{helpers}#define {name}_FIELDS(F_, C_) {}\nTS_BAG_DEF({name})\n#define {name}_create(...) TS_BAG({name}, __VA_ARGS__)\n", entries.join(" "))
}

// What an argument left out is: `undefined`, as each type says it.
fn absent(t: &Ty, cx: &Ctx) -> Option<String> {
    let Ty::Nullable(inner, a) = t else { return None };
    if !a.optional() {
        return None;
    }
    Some(match &**inner {
        Ty::Num | Ty::NumEnum(_) => "js_none_double()".into(),
        Ty::Bool => "js_none_bool()".into(),
        Ty::Enum(e) => format!("{e}_absent"),
        Ty::Str => "(const char*)0".into(),
        Ty::NamedUnion(n) if cx.takes(n) => format!("({n}){{ .tag = -1 }}"),
        _ => format!("({}){{ 0 }}", typed(t, true, cx)),
    })
}

// The C types `_Generic` takes for a parameter of this type, where it can
// tell: an enumerator is an int, so an enum cannot be told from a number.
fn generic_types(t: &Ty, has_num: bool, cx: &Ctx) -> Option<Vec<String>> {
    let list = |names: &[&str]| Some(names.iter().map(|n| n.to_string()).collect());
    match t {
        Ty::Str => list(&["const char*", "char*", "JsString"]),
        Ty::Num => list(&["double", "float", "int", "unsigned int", "long", "unsigned long"]),
        // `true` is an int in C11.
        Ty::Bool => list(if has_num { &["bool"] } else { &["bool", "int"] }),
        // Any value: `_Generic` matches only a `JsValue`, not the Shape it would take.
        Ty::Enum(_) | Ty::NumEnum(_) | Ty::Nullable(..) | Ty::Void | Ty::Any => None,
        t => Some(vec![typed(t, true, cx)]),
    }
}

// The overloads `f` is the first of, where the C type of one parameter tells
// them apart: `Shape_label(s, "a")` and `Shape_label(s, 7)`.
pub(super) fn family<'a>(f: &'a Func, all: &'a [Func], cx: &Ctx) -> Option<(usize, Vec<&'a Func>)> {
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
        if !generic_types(&o.params[at].1, has_num, cx)?.into_iter().all(|t| seen.insert(t)) {
            return None;
        }
    }
    Some((at, members))
}

// A function's name as a macro, so a call may: pass any subtype
// (`AS_Element(x)` is applied), pass `NULL` for nullable handles, pass bare
// function pointers for callbacks, leave out trailing optional arguments, pass
// variadic rest arguments, and for the first of a family of overloads, have
// `_Generic` choose among them. Returns `(macro_code, emitted_list_variant)`.
pub(super) fn call_macros(
    name: &str,
    owner: Option<&str>,
    f: &Func,
    earlier: &[&Func],
    family: Option<(usize, Vec<String>, Vec<&Func>)>,
    descendants: &std::collections::BTreeMap<String, Vec<String>>,
    remote_as: &std::collections::HashMap<String, String>,
    as_macro: &dyn Fn(&str) -> String,
    allow_list: bool,
    cx: &Ctx,
) -> (String, bool) {
    let cb_name = |t: &Ty| -> Option<String> {
        let inner = match t {
            Ty::Nullable(inner, _) => &**inner,
            t => t,
        };
        match inner {
            Ty::Obj(n) if cx.callbacks.contains(n) => Some(n.clone()),
            _ => None,
        }
    };
    let upcast = |t: &Ty| -> Option<String> {
        let inner = match t {
            Ty::Nullable(inner, _) => &**inner,
            t => t,
        };
        match inner {
            Ty::Obj(n) if descendants.contains_key(n) => Some(as_macro(n)),
            Ty::Obj(n) => remote_as.get(n).cloned(),
            _ => None,
        }
    };
    // A union argument is the union or any member `U_of` tells by its type.
    let made = |t: &Ty| match t {
        Ty::NamedUnion(n) if cx.of.contains_key(n) => Some(n.clone()),
        Ty::Nullable(inner, _) => match &**inner {
            Ty::NamedUnion(n) if cx.of.contains_key(n) => Some(n.clone()),
            _ => None,
        },
        _ => None,
    };
    let array_arg_name = |t: &Ty| -> Option<String> {
        let inner = match t {
            Ty::Nullable(inner, _) => &**inner,
            t => t,
        };
        let Ty::ArrayOf(elem) = inner else { return None };
        let arr = format!("JsArray_{}", element(elem, cx));
        cx.array_arg.contains_key(&arr).then_some(arr)
    };
    let nullable_handle_ty = |t: &Ty| -> Option<String> {
        let Ty::Nullable(inner, _) = t else { return None };
        match &**inner {
            Ty::Obj(_) | Ty::Record(_) | Ty::Array | Ty::ArrayOf(_) | Ty::Tuple(_) | Ty::Any | Ty::NamedUnion(_) => Some(typed(t, true, cx)),
            _ => None,
        }
    };
    let chosen = family.as_ref().map(|f| f.0);
    // Where an overload takes a union or an array of a union, the member types
    // not taken by other overloads choose it too, and convert into it.
    let mut conv: Vec<(String, String)> = Vec::new();
    if let Some((at, _, members)) = &family {
        let has_num = members.iter().any(|o| o.params[*at].1 == Ty::Num);
        let mut seen: std::collections::BTreeSet<String> = members.iter().flat_map(|o| generic_types(&o.params[*at].1, has_num, cx).unwrap_or_default()).collect();
        for o in members {
            if let Ty::NamedUnion(n) = &o.params[*at].1 {
                for t in cx.of.get(n).into_iter().flatten() {
                    if seen.insert(t.clone()) {
                        conv.push((t.clone(), format!("{n}_of")));
                    }
                }
            } else if let Some(arr) = array_arg_name(&o.params[*at].1) {
                for t in cx.array_arg.get(&arr).into_iter().flatten() {
                    if seen.insert(t.clone()) {
                        conv.push((t.clone(), format!("{arr}_arg")));
                    }
                }
            }
        }
    }
    let wrap_expr = |p_idx: usize, t: &Ty, expr: String| -> String {
        if chosen == Some(p_idx) && !conv.is_empty() {
            let arms: String = conv.iter().map(|(t, macro_name)| format!("{t}: {macro_name}({expr}), ")).collect();
            format!("_Generic(({expr}), {arms}default: ({expr}))")
        } else if chosen == Some(p_idx) {
            format!("({expr})")
        } else if let Some(cb) = cb_name(t) {
            format!("{cb}_arg({expr})")
        } else if let Some(m) = upcast(t) {
            format!("{m}({expr})")
        } else if let Some(n) = made(t) {
            format!("{n}_of({expr})")
        } else if let Some(arr) = array_arg_name(t) {
            format!("{arr}_arg({expr})")
        } else if *t == Ty::Str || matches!(t, Ty::Nullable(inner, _) if **inner == Ty::Str) {
            format!("TS_STR({expr})")
        } else if matches!(t, Ty::Nullable(inner, _) if matches!(**inner, Ty::Num | Ty::NumEnum(_))) {
            format!("TS_OPT_DOUBLE({expr})")
        } else if matches!(t, Ty::Nullable(inner, _) if **inner == Ty::Bool) {
            format!("TS_OPT_BOOL({expr})")
        } else if let Some(hty) = nullable_handle_ty(t) {
            format!("TS_NULL_H({hty}, {expr})")
        } else {
            format!("({expr})")
        }
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
    let mut has_cb = false;
    for (i, (_, t)) in f.params.iter().enumerate() {
        let a = format!("a{}", i + 1);
        params.push(a.clone());
        let is_cb = chosen != Some(i) && cb_name(t).is_some();
        has_cb |= is_cb;
        any |= chosen != Some(i)
            && (is_cb
                || upcast(t).is_some()
                || made(t).is_some()
                || array_arg_name(t).is_some()
                || *t == Ty::Str
                || matches!(t, Ty::Nullable(inner, _) if matches!(**inner, Ty::Str | Ty::Num | Ty::NumEnum(_) | Ty::Bool))
                || nullable_handle_ty(t).is_some());
        args.push(wrap_expr(i, t, a));
    }
    let own = usize::from(owner.is_some());
    let rest_idx = f.params.len().saturating_sub(1);
    let fixed_n = own + rest_idx;
    let rest_ok = f.rest
        && !f.params.is_empty()
        && chosen != Some(rest_idx)
        && fixed_n <= 12
        && family.as_ref().is_none_or(|(_, _, members)| {
            members.iter().all(|o| o.rest && o.params.len() == f.params.len() && o.params[rest_idx].1 == f.params[rest_idx].1)
        });
    let rest_ctor = if rest_ok {
        match &f.params[rest_idx].1 {
            Ty::Array => Some("js_values".to_string()),
            Ty::ArrayOf(e) => Some(format!("JsArray_{}_of", element(e, cx))),
            _ => None,
        }
    } else {
        None
    };

    let emit_wrap = |prefix: &mut String, fn_name: &str, o: &Func, o_name: &str, str_arm: Option<(usize, bool)>, release_rest: bool| {
        let mut w_params = Vec::new();
        let mut w_args = Vec::new();
        let mut w_release = Vec::new();
        if Class::has_self(o) {
            let own_ty = owner.unwrap();
            w_params.push(format!("{own_ty} self"));
            w_args.push("self".to_string());
        }
        for (j, (_, pt)) in o.params.iter().enumerate() {
            if let Some((at, is_js_str)) = str_arm && j == at {
                if is_js_str {
                    w_params.push(format!("JsString p{j}"));
                    w_args.push(format!("ts_str_js(p{j})"));
                } else {
                    w_params.push(format!("const char* p{j}"));
                    w_args.push(format!("ts_str_c(p{j})"));
                }
            } else if let Some(cb) = cb_name(pt).filter(|_| chosen != Some(j)) {
                w_params.push(format!("TsCb p{j}"));
                w_args.push(format!("({cb}){{ p{j}.h }}"));
                w_release.push(format!("if (p{j}.ts_own) ts_raw_js_release(p{j}.h);"));
            } else {
                w_params.push(format!("{} p{j}", func_param_ty(pt, cx)));
                w_args.push(format!("p{j}"));
            }
            if release_rest && j + 1 == o.params.len() {
                w_release.push(format!("ts_raw_js_release(p{j}.h);"));
            }
        }
        let ret_ty = typed(&o.ret, false, cx);
        let call_expr = format!("({o_name})({})", w_args.join(", "));
        let body = if w_release.is_empty() {
            if o.ret == Ty::Void { format!("{call_expr};") } else { format!("return {call_expr};") }
        } else if o.ret == Ty::Void {
            format!("{call_expr}; {}", w_release.join(" "))
        } else {
            format!("{ret_ty} raw_r = {call_expr}; {} return raw_r;", w_release.join(" "))
        };
        *prefix += &format!(
            "static inline {ret_ty} {fn_name}({}) {{ {body} }}\n",
            if w_params.is_empty() { "void".into() } else { w_params.join(", ") }
        );
    };

    let build_callee = |prefix: &mut String, release_rest: bool| -> String {
        match &family {
            Some((at, names, members)) => {
                let has_num = members.iter().any(|o| o.params[*at].1 == Ty::Num);
                let mut choices = Vec::new();
                let fam_prefix = if release_rest { "ts_rest_fam" } else { "ts_fam" };
                for (o_name, o) in names.iter().zip(members) {
                    if o.params[*at].1 == Ty::Str {
                        let c_fn = format!("{fam_prefix}_cstr_{o_name}");
                        let js_fn = format!("{fam_prefix}_jsstr_{o_name}");
                        emit_wrap(prefix, &c_fn, o, o_name, Some((*at, false)), release_rest);
                        emit_wrap(prefix, &js_fn, o, o_name, Some((*at, true)), release_rest);
                        choices.extend([format!("const char*: {c_fn}"), format!("char*: {c_fn}"), format!("JsString: {js_fn}")]);
                    } else {
                        let target_fn = if release_rest || o.params.iter().enumerate().any(|(j, (_, pt))| j != *at && cb_name(pt).is_some()) {
                            let w_fn = format!("{fam_prefix}_{o_name}");
                            emit_wrap(prefix, &w_fn, o, o_name, None, release_rest);
                            w_fn
                        } else {
                            o_name.clone()
                        };
                        choices.extend(generic_types(&o.params[*at].1, has_num, cx).unwrap_or_default().into_iter().map(|t| format!("{t}: {target_fn}")));
                        if let Ty::NamedUnion(n) = &o.params[*at].1 {
                            let target_macro = format!("{n}_of");
                            choices.extend(conv.iter().filter(|(_, m)| *m == target_macro).map(|(t, _)| format!("{t}: {target_fn}")));
                        } else if let Some(arr) = array_arg_name(&o.params[*at].1) {
                            let target_macro = format!("{arr}_arg");
                            choices.extend(conv.iter().filter(|(_, m)| *m == target_macro).map(|(t, _)| format!("{t}: {target_fn}")));
                        }
                    }
                }
                format!("_Generic((a{}), {})", at + 1, choices.join(", "))
            }
            None => {
                if release_rest {
                    let r_fn = format!("ts_rest_{name}");
                    emit_wrap(prefix, &r_fn, f, name, None, true);
                    r_fn
                } else if has_cb {
                    let w_fn = format!("ts_wrap_{name}");
                    emit_wrap(prefix, &w_fn, f, name, None, false);
                    w_fn
                } else {
                    format!("({name})")
                }
            }
        }
    };

    let mut prefix = String::new();
    let callee = build_callee(&mut prefix, false);
    let param_absent = |g: &Func, idx: usize| -> Option<String> {
        if g.rest && idx + 1 == g.params.len() {
            Some(format!("({}){{ 0 }}", func_param_ty(&g.params[idx].1, cx)))
        } else {
            absent(&g.params[idx].1, cx)
        }
    };
    // A setter's value is never left out: that is not setting it.
    let setter = matches!(f.call, Call::Set(_) | Call::StaticSet(_) | Call::IndexSet);
    let n = params.len();
    let optional = if setter || n > 15 { 0 } else { (0..f.params.len()).rev().take_while(|&idx| param_absent(f, idx).is_some()).count() };
    let shadowed = |given: usize| {
        let k = given - own;
        earlier.iter().any(|g| g.call == f.call && g.fixed == f.fixed && g.params.len() >= k && g.params[..k] == f.params[..k] && (k..g.params.len()).all(|j| param_absent(g, j).is_some()))
    };
    // The shorter calls this function answers: down to where an earlier one does.
    let shorter: Vec<usize> = (n - optional..n).rev().take_while(|g| !shadowed(*g)).collect();

    if let Some(ctor) = rest_ctor.filter(|_| shorter.contains(&fixed_n)) {
        let rest_callee = build_callee(&mut prefix, true);
        let forward_r0 = any || family.is_some();
        let mut out = prefix;
        if shorter.len() == 1 {
            let mut r0_args = if forward_r0 { args[..fixed_n].to_vec() } else { params[..fixed_n].to_vec() };
            r0_args.push(param_absent(f, rest_idx).unwrap());
            out += &format!("#define ts_r0_{name}({}) {callee}({})\n", params[..fixed_n].join(", "), r0_args.join(", "));
        } else {
            out += &format!("#define ts_r0_{name}(...) TS_CALL({name}, __VA_ARGS__)\n");
            for given in shorter {
                let mut call_args: Vec<String> = (0..given)
                    .map(|j| if forward_r0 { args[j].clone() } else { params[j].clone() })
                    .collect();
                for j in given..n {
                    let p_idx = j - own;
                    let abs = param_absent(f, p_idx).unwrap();
                    call_args.push(if forward_r0 && j + 1 < n {
                        wrap_expr(p_idx, &f.params[p_idx].1, abs)
                    } else {
                        abs
                    });
                }
                out += &format!("#define ts_arity_{name}_{given}({}) {callee}({})\n", params[..given].join(", "), call_args.join(", "));
            }
        }
        let mut r1_args = args[..fixed_n].to_vec();
        r1_args.push(format!("{ctor}(__VA_ARGS__)"));
        let r1_params = if fixed_n == 0 {
            "...".to_string()
        } else {
            format!("{}, ...", params[..fixed_n].join(", "))
        };
        out += &format!("#define ts_r1_{name}({r1_params}) {rest_callee}({})\n", r1_args.join(", "));
        out += &format!("#define {name}(...) TS_PASTE3(ts_r, TS_GT_{fixed_n}(__VA_ARGS__), _{name})(__VA_ARGS__)\n");
        let emitted_list = if allow_list {
            out += &format!("#define {name}_list({}) {callee}({})\n", params.join(", "), args.join(", "));
            true
        } else {
            false
        };
        return (out, emitted_list);
    }

    let forward = (any || family.is_some()).then(|| format!("{callee}({})", args.join(", ")));
    if shorter.is_empty() {
        return (
            forward.map(|call| format!("{prefix}#define {name}({}) {call}\n", params.join(", "))).unwrap_or_default(),
            false,
        );
    }
    // By the number of arguments: each shorter call supplies every omitted
    // trailing parameter directly in a single hop.
    let mut out = format!("{prefix}#define {name}(...) TS_CALL({name}, __VA_ARGS__)\n");
    out += &match &forward {
        Some(call) => format!("#define ts_arity_{name}_{n}({}) {call}\n", params.join(", ")),
        None => format!("#define ts_arity_{name}_{n} ({name})\n"),
    };
    for given in shorter {
        let mut call_args: Vec<String> = (0..given)
            .map(|j| if forward.is_some() { args[j].clone() } else { params[j].clone() })
            .collect();
        for j in given..n {
            let p_idx = j - own;
            let abs = param_absent(f, p_idx).unwrap();
            call_args.push(if forward.is_some() {
                wrap_expr(p_idx, &f.params[p_idx].1, abs)
            } else {
                abs
            });
        }
        out += &format!("#define ts_arity_{name}_{given}({}) {callee}({})\n", params[..given].join(", "), call_args.join(", "));
    }
    (out, false)
}

// Direct and indirect bases, nearest first.
pub(super) fn all_ancestors(m: &Model, class: &str) -> Vec<String> {
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

// What C cannot express of a model: inline unions, generics, keyed signatures.
pub(super) fn check(m: &Model) -> super::super::Problems {
    let mut problems = super::super::Problems::default();
    super::super::model::each_type(m, &mut |at, t| {
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
