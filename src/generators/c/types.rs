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
        Ty::Tuple(es) => tuple_name(es, cx),
        Ty::Obj(n) | Ty::Enum(n) | Ty::NumEnum(n) => n.clone(),
        // An optional number or flag says whether it is there; an optional enum is `E_absent` when it is not.
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Num | Ty::NumEnum(_)) => "JsOptionalDouble".into(),
        Ty::Nullable(inner, _) if **inner == Ty::Bool => "JsOptionalBool".into(),
        // NULL, or a null handle, is the absent value.
        Ty::Nullable(inner, _) => typed(inner, param, cx),
        // A union a handle can be decoded to: a tagged struct. Any other is a value.
        Ty::NamedUnion(n) if cx.decodable(n) => n.clone(),
        Ty::NamedUnion(_) => "JsValue".into(),
        Ty::Union(_) => panic!("{T} generator cannot express a union: the target config must name or erase unions"),
        Ty::Key(_) | Ty::Mapped(_) | Ty::Listener(_) => panic!("{T} generator cannot express keyed generics: the target config must expand them"),
        Ty::Param(_) | Ty::Generic(..) => panic!("{T} generator cannot express generics: the target config must erase them"),
    }
}

// An array's element type, as a name: `double`, `JsString`, `Element`, `JsArray_double`.
fn element(t: &Ty, cx: &Ctx) -> String {
    match t {
        Ty::Str => "JsString".into(),
        t => typed(t, false, cx),
    }
}

// The typed arrays and tuples a type names, innermost first.
pub(super) fn arrays(t: &Ty, out: &mut Vec<Ty>) {
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
    let has_num = u.parts.contains(&Ty::Num);
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
    let tag_attr = import_attr(wasm_module, &format!("{n}_tag"));
    let tag_raw = raw_fn(raw_prefix, &format!("{n}_tag"));
    let mut functions = format!(
        "{tag_attr} int32_t {tag_raw}(uint32_t, int32_t);\n\
         static inline uint32_t {n}_to_handle({n} u) {{ switch (u.tag) {{{to} }} return 0; }}\n\
         static inline {n} {n}_from_handle(uint32_t h) {{ {n} u = {{ .tag = -1 }}; if (!h) return u; u.tag = {tag_raw}(h, 0); switch (u.tag) {{{from} default: ts_raw_js_release(h); break; }} return u; }}\n\
         static inline void {n}_release({n} u) {{ switch (u.tag) {{{release} default: break; }} }}\n"
    );
    // Made from a member: `Value_of_number(42)`, or by its type, `Value_of(widget)`.
    let field_type = |p: &Ty| if *p == Ty::Str { "JsString".to_string() } else { typed(p, false, cx) };
    for (p, f) in u.parts.iter().zip(&field_names) {
        functions += &format!("static inline {n} {n}_of_{f}({} v) {{ return ({n}){{ .tag = {n}_{f}, .as.{f} = v }}; }}\n", field_type(p));
    }
    if has_str {
        functions += &format!("static inline {n} {n}_of_cstr(const char* v) {{ return ({n}){{ .tag = -2, .as.ts_cstr = v }}; }}\n");
    }
    let mut seen = std::collections::BTreeSet::new();
    let choices: Option<Vec<String>> = u
        .parts
        .iter()
        .zip(&field_names)
        .map(|(p, f)| {
            if *p == Ty::Str {
                let pairs = [("JsString", "string"), ("const char*", "cstr"), ("char*", "cstr")];
                pairs
                    .iter()
                    .all(|(t, _)| seen.insert((*t).to_string()))
                    .then(|| pairs.iter().map(|(t, suffix)| format!("{t}: {n}_of_{suffix}")).collect::<Vec<_>>().join(", "))
            } else {
                let types = generic_types(p, has_num, cx)?;
                types.iter().all(|t| seen.insert(t.clone())).then(|| types.iter().map(|t| format!("{t}: {n}_of_{f}")).collect::<Vec<_>>().join(", "))
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

// A typed array: `JsArray_double`, with `_new`, `_length`, `_get`, `_push` and `_from`.
// What `_get` returns is the caller's to release; a number or flag is read and released.
pub(super) fn array_type(e: &Ty, cx: &Ctx) -> String {
    let (name, elem) = (format!("JsArray_{}", element(e, cx)), typed(e, false, cx));
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
        Ty::NamedUnion(n) if cx.decodable(n) => (format!("return {n}_from_handle(h);"), format!("{n} v"), format!("uint32_t h = {n}_to_handle(v); ts_raw_js_array_push(a.h, h); ts_raw_js_release(h);")),
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

fn func_param_ty(t: &Ty, cx: &Ctx) -> String {
    if *t == Ty::Str || matches!(t, Ty::Nullable(inner, _) if **inner == Ty::Str) {
        "TsStr".into()
    } else {
        typed(t, true, cx)
    }
}

pub(super) fn func(out: &mut String, owner: Option<&str>, f: &Func, wasm_module: &str, raw_prefix: &str, cx: &Ctx) {
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
        if let Some(n) = union.filter(|n| cx.decodable(n)) {
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
    *out += &format!("static inline {} {name}({}) {{ {pre}{body} }}\n", typed(&f.ret, false, cx), void(&params));
}

// What an argument left out is: `undefined`, as each type says it.
fn absent(t: &Ty, cx: &Ctx) -> Option<String> {
    let Ty::Nullable(inner, true) = t else { return None };
    Some(match &**inner {
        Ty::Num | Ty::NumEnum(_) => "js_none_double()".into(),
        Ty::Bool => "js_none_bool()".into(),
        Ty::Enum(e) => format!("{e}_absent"),
        Ty::Str => "(const char*)0".into(),
        Ty::NamedUnion(n) if cx.decodable(n) => format!("({n}){{ .tag = -1 }}"),
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
// (`AS_Element(x)` is applied), leave out trailing optional arguments, and
// for the first of a family of overloads, have `_Generic` choose among them.
// `earlier` are the functions declared before f beside it: where one of them,
// called with the same leading arguments, leaves out the same ones, f does not.
pub(super) fn call_macros(
    name: &str,
    owner: Option<&str>,
    f: &Func,
    earlier: &[&Func],
    family: Option<(usize, Vec<String>, Vec<&Func>)>,
    descendants: &std::collections::BTreeMap<String, Vec<String>>,
    as_macro: &dyn Fn(&str) -> String,
    cx: &Ctx,
) -> String {
    let upcast = |t: &Ty| match t {
        Ty::Obj(n) if descendants.contains_key(n) => Some(n.clone()),
        Ty::Nullable(inner, _) => match &**inner {
            Ty::Obj(n) if descendants.contains_key(n) => Some(n.clone()),
            _ => None,
        },
        _ => None,
    };
    let chosen = family.as_ref().map(|f| f.0);
    let wrap_expr = |p_idx: usize, t: &Ty, expr: String| -> String {
        if chosen == Some(p_idx) {
            format!("({expr})")
        } else if let Some(n) = upcast(t) {
            format!("{}({expr})", as_macro(&n))
        } else if *t == Ty::Str || matches!(t, Ty::Nullable(inner, _) if **inner == Ty::Str) {
            format!("TS_STR({expr})")
        } else if matches!(t, Ty::Nullable(inner, _) if matches!(**inner, Ty::Num | Ty::NumEnum(_))) {
            format!("TS_OPT_DOUBLE({expr})")
        } else if matches!(t, Ty::Nullable(inner, _) if **inner == Ty::Bool) {
            format!("TS_OPT_BOOL({expr})")
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
    for (i, (_, t)) in f.params.iter().enumerate() {
        let a = format!("a{}", i + 1);
        params.push(a.clone());
        any |= chosen != Some(i)
            && (upcast(t).is_some()
                || *t == Ty::Str
                || matches!(t, Ty::Nullable(inner, _) if matches!(**inner, Ty::Str | Ty::Num | Ty::NumEnum(_) | Ty::Bool)));
        args.push(wrap_expr(i, t, a));
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
                            let pty = func_param_ty(pt, cx);
                            c_params.push(format!("{pty} p{j}"));
                            js_params.push(format!("{pty} p{j}"));
                            c_args.push(format!("p{j}"));
                            js_args.push(format!("p{j}"));
                        }
                    }
                    let ret_ty = typed(&o.ret, false, cx);
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
                    choices.extend(generic_types(&o.params[*at].1, has_num, cx).unwrap_or_default().into_iter().map(|t| format!("{t}: {o_name}")));
                }
            }
            format!("_Generic((a{}), {})", at + 1, choices.join(", "))
        }
        None => format!("({name})"),
    };
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
    // By the number of arguments: each shorter call supplies every omitted
    // trailing parameter directly in a single hop.
    let mut out = format!("{prefix}#define {name}(...) TS_CALL({name}, __VA_ARGS__)\n");
    out += &match &forward {
        Some(call) => format!("#define ts_arity_{name}_{n}({}) {call}\n", params.join(", ")),
        None => format!("#define ts_arity_{name}_{n} ({name})\n"),
    };
    let head = if forward.is_some() { callee.as_str() } else { "" };
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
        let target = if forward.is_some() { head.to_string() } else { format!("({name})") };
        out += &format!("#define ts_arity_{name}_{given}({}) {target}({})\n", params[..given].join(", "), call_args.join(", "));
    }
    out
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
