// XML -> Zig module for a wasm32 module. As C, but unions of simple types
// become tagged unions, and nullable objects become optionals.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::Path;

use super::model::{Call, Class, Func, Model, NamedUnion, Ty};
use super::names::Naming;

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
fn ident(name: &str) -> String {
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

// What the runtime declares at the top of every file.
const RUNTIME_NAMES: &[&str] = &[
    "JsValue", "JsString", "JsArray", "JsArrayOf", "JsTuple", "JsIteratorOf", "JsListener", "Lent", "jsRelease", "jsRetain", "jsLiveHandles", "jsForgetCallback",
    "jsTakeError", "jsErrorMessage", "jsErrorName", "jsErrorStack", "jsErrorNew", "jsDateNow", "jsEnumOf", "jsSame", "jsHasError", "jsTry", "JsScope", "jsTake",
];

// Types and functions share a file's namespace, and the root's that
// re-exports them; a type's functions are in its struct, beside what the
// struct has of its own.
pub const MODEL_OPTIONS: super::model::ModelOptions<'static> = super::model::ModelOptions {
    target: "Zig",
    counted: false,
    generics: false,
    overloading: false,
    retain_opaque_constraints: false,
};

pub const NAMING: Naming = Naming {
    // And what a struct declares inside (a map's `Key` and `Type`): Zig lets
    // no declaration shadow another, so no type of the file has those names.
    reserved: || RUNTIME_NAMES.iter().chain(&["raw", "std", "builtin", "JsAsyncIterator", "JsIterator", "Key", "Type", "Methods"]).map(|n| n.to_string()).collect(),
    type_names: |candidate, _, _, _| vec![candidate.to_string()],
    values_of: |_, _, _, _| Vec::new(),
    types_apart: false,
    overloads: false,
    func_name: |_, name, _| Some(method_name(name)),
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

// Each function's Zig name, by the model's: `names::settle` made them apart.
fn names_of<'a>(funcs: impl Iterator<Item = &'a Func>) -> HashMap<String, String> {
    funcs.map(|f| (f.name.clone(), method_name(&f.name))).collect()
}

// Parameters get a suffix: Zig forbids shadowing a declaration in scope.
fn param(name: &str) -> String {
    format!("{name}_")
}

fn raw(t: &Ty) -> &'static str {
    use super::wasm::{AbiSlot::*, WasmParamAbi::*};
    match super::wasm::param_abi(t) {
        None => "void",
        One(F64) => "f64",
        One(I32) => "i32",
        One(U32 | Void) | Str | OptNum => "u32",
    }
}

fn union_field(t: &Ty) -> String {
    match t {
        Ty::Num => "number".into(),
        Ty::Str => "string".into(),
        Ty::Bool => "boolean".into(),
        Ty::Obj(n) | Ty::Enum(n) => n.clone(),
        Ty::Array | Ty::ArrayOf(_) | Ty::Tuple(_) => "array".into(),
        _ => unreachable!(),
    }
}

// A named union: a Zig tagged union, the same for arguments and results.
// Strings in it are handles: `JsString.from("text")`.
fn union_decl(u: &NamedUnion) -> String {
    let fields: Vec<String> = u
        .parts
        .iter()
        .map(|p| {
            let t = match p {
                Ty::Str => "JsString".to_string(),
                p => typed(p, false),
            };
            format!("{}: {t}", ident(&union_field(p)))
        })
        .collect();
    let decode = if u.decodable() {
        format!(
            "    // The member a handle a function returned holds; takes the handle over.\n    pub fn fromHandle(h: u32) @This() {{ return {}; }}\n    \
             // The member a handle lent to a callback holds; it stays lent.\n    pub fn lent(h: u32) @This() {{ return {}; }}\n",
            decode_union(u, "h", true),
            decode_union(u, "h", false)
        )
    } else {
        String::new()
    };
    format!(
        "pub const {0} = union(enum) {{\n    {1},\n    // Which member a value is: its index.\n    pub fn tag(h: u32) i32 {{ return raw.{0}_tag(h, 0); }}\n{decode}}};\n",
        u.name,
        fields.join(",\n    ")
    )
}

// Turns a handle into the union's member: `raw.U_tag` says which. A number
// or flag is read from the handle, and with `take` the handle released.
fn decode_union(u: &NamedUnion, h: &str, take: bool) -> String {
    let arms: Vec<String> = u
        .parts
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let value = match (p, take) {
                (Ty::Num, true) => format!("JsValue.takeNumber({h})"),
                (Ty::Bool, true) => format!("JsValue.takeBool({h})"),
                (Ty::Num, false) => format!("(JsValue{{ .h = {h} }}).toNumber()"),
                (Ty::Bool, false) => format!("(JsValue{{ .h = {h} }}).toBool()"),
                (Ty::Enum(e), true) => format!("jsTake({e}, {h})"),
                (Ty::Enum(e), false) => format!("jsEnumOf({e}, {h})"),
                _ => format!(".{{ .h = {h} }}"),
            };
            format!("{i} => .{{ .{} = {value} }}", ident(&union_field(p)))
        })
        .collect();
    format!("switch (tag({h})) {{ {}, else => unreachable }}", arms.join(", "))
}

fn typed(t: &Ty, is_param: bool) -> String {
    match t {
        Ty::Void => "void".into(),
        Ty::Num => "f64".into(),
        Ty::Bool => "bool".into(),
        Ty::Str if is_param => "[]const u8".into(),
        Ty::Str => "JsString".into(),
        Ty::Array => "JsArray".into(),
        Ty::ArrayOf(e) => format!("JsArrayOf({})", typed(e, false)),
        Ty::Tuple(es) => format!("JsTuple(&.{{ {} }})", es.iter().map(|e| typed(e, false)).collect::<Vec<_>>().join(", ")),
        Ty::Obj(n) | Ty::Enum(n) | Ty::NumEnum(n) => n.clone(),
        Ty::Nullable(inner, _) => format!("?{}", typed(inner, is_param)),
        Ty::Union(parts) if !parts.is_empty() && is_param => {
            let fields: Vec<String> = parts.iter().map(|p| format!("{}: {}", ident(&union_field(p)), typed(p, true))).collect();
            format!("union(enum) {{ {} }}", fields.join(", "))
        }
        Ty::NamedUnion(n) => n.clone(),
        // `KEY` is replaced by the name of the signature's key parameter.
        Ty::Key(m) => format!("{m}.Key"),
        Ty::Mapped(m) => format!("{m}.Type(KEY)"),
        Ty::Listener(ev) => format!("JsListener({})", typed(ev, false)),
        Ty::Param(_) | Ty::Generic(..) => panic!("Zig generator cannot express generics: the target config must erase them"),
        Ty::Any | Ty::Union(_) => "JsValue".into(),
    }
}


// What a callback argument or result crosses as: the glue's `to_wasm`.
fn raw_callback(t: &Ty) -> &'static str {
    use super::wasm::AbiSlot::*;
    match super::wasm::ret_abi(t) {
        Void => "void",
        F64 => "f64",
        I32 => "i32",
        U32 => "u32",
    }
}

fn callback_param_ty(t: &Ty, unions: &[NamedUnion]) -> String {
    let decodable = |n: &str| unions.iter().any(|u| u.name == n && u.decodable());
    match t {
        Ty::NamedUnion(n) if !decodable(n) => "JsValue".into(),
        Ty::Nullable(inner, _) => match &**inner {
            Ty::NamedUnion(n) if !decodable(n) => "?JsValue".into(),
            _ => typed(t, false),
        },
        _ => typed(t, false),
    }
}

// A callback argument, lent, as its Zig type.
fn callback_arg(t: &Ty, a: &str, unions: &[NamedUnion]) -> String {
    let decodable = |n: &str| unions.iter().any(|u| u.name == n && u.decodable());
    match t {
        Ty::Num => a.into(),
        Ty::NumEnum(_) => format!("@enumFromInt(@as(i32, @intFromFloat({a})))"),
        Ty::Bool => format!("{a} != 0"),
        Ty::Enum(_) => format!("@enumFromInt({a})"),
        Ty::NamedUnion(n) if decodable(n) => format!("{n}.lent({a})"),
        Ty::NamedUnion(_) | Ty::Union(_) | Ty::Any => format!("JsValue{{ .h = {a} }}"),
        Ty::Nullable(inner, _) => {
            let some = match &**inner {
                Ty::Bool => return format!("if ({a} < 0) null else {a} != 0"),
                Ty::Enum(e) => return format!("if ({a} < 0) null else @as({e}, @enumFromInt({a}))"),
                Ty::Num => format!("(JsValue{{ .h = {a} }}).toNumber()"),
                Ty::NumEnum(e) => format!("@as({e}, @enumFromInt(@as(i32, @intFromFloat((JsValue{{ .h = {a} }}).toNumber()))))"),
                Ty::NamedUnion(n) if decodable(n) => format!("{n}.lent({a})"),
                Ty::NamedUnion(_) | Ty::Union(_) | Ty::Any => format!("JsValue{{ .h = {a} }}"),
                inner => format!("{}{{ .h = {a} }}", typed(inner, false)),
            };
            format!("@as({}, if ({a} == 0) null else {some})", callback_param_ty(t, unions))
        }
        t => format!("{}{{ .h = {a} }}", typed(t, false)),
    }
}

// Returns what a callback's function returned, given to JavaScript.
fn callback_result(t: &Ty, call: &str) -> String {
    match t {
        Ty::Void => format!("{call};"),
        Ty::Num => format!("return {call};"),
        Ty::NumEnum(_) => format!("return @floatFromInt(@intFromEnum({call}));"),
        Ty::Bool => format!("return @intFromBool({call});"),
        Ty::Enum(_) => format!("return @intFromEnum({call});"),
        Ty::Nullable(inner, _) if **inner == Ty::Bool => format!("return if ({call}) |v| @intFromBool(v) else -1;"),
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Enum(_)) => format!("return if ({call}) |v| @intFromEnum(v) else -1;"),
        Ty::Nullable(inner, _) if **inner == Ty::Num => format!("return if ({call}) |v| raw.js_from_number(v) else 0;"),
        Ty::Nullable(..) => format!("const r = {call}; return if (r) |v| Lent.of(v).h else 0;"),
        Ty::NamedUnion(_) => format!("const r = {call}; return switch (r) {{ inline else => |v| Lent.of(v).h }};"),
        _ => format!("return {call}.h;"),
    }
}

fn is_lent_ty(t: &Ty) -> bool {
    match t {
        Ty::Nullable(inner, _) => !matches!(**inner, Ty::Num | Ty::NumEnum(_) | Ty::Bool | Ty::Enum(_) | Ty::Str),
        Ty::Void | Ty::Num | Ty::Bool | Ty::Str | Ty::Enum(_) | Ty::NumEnum(_) | Ty::Key(_) => false,
        _ => true,
    }
}

fn lent_expr(expr: &str, t: &Ty, key: &str) -> String {
    let sub = |s: String| if key.is_empty() { s } else { s.replace("KEY", key) };
    match t {
        Ty::Obj(n) => format!("Lent.obj({n}, {expr})"),
        Ty::NamedUnion(n) => format!("Lent.unionVal({n}, {expr})"),
        Ty::Union(parts) if !parts.is_empty() => format!("Lent.unionVal({}, {expr})", sub(typed(t, true))),
        Ty::Any | Ty::Union(_) => format!("Lent.of({expr})"),
        Ty::Array => format!("Lent.array({expr})"),
        Ty::ArrayOf(e) => format!("Lent.arrayOf({}, {expr})", sub(typed(e, false))),
        Ty::Tuple(es) => format!(
            "Lent.tuple(&.{{ {} }}, {expr})",
            es.iter().map(|e| sub(typed(e, false))).collect::<Vec<_>>().join(", ")
        ),
        Ty::Listener(ev) => format!("Lent.listener(JsListener({}), {expr})", sub(typed(ev, false))),
        Ty::Mapped(m) => format!("Lent.value({m}.Type({key}), {expr})"),
        Ty::Nullable(inner, _) => match &**inner {
            Ty::Obj(n) => format!("Lent.objOpt({n}, {expr})"),
            Ty::NamedUnion(n) => format!("Lent.unionOpt({n}, {expr})"),
            Ty::Union(parts) if !parts.is_empty() => format!("Lent.unionOpt({}, {expr})", sub(typed(inner, true))),
            Ty::Any | Ty::Union(_) => format!("Lent.of({expr})"),
            Ty::Array => format!("Lent.arrayOpt({expr})"),
            Ty::ArrayOf(e) => format!("Lent.arrayOfOpt({}, {expr})", sub(typed(e, false))),
            Ty::Tuple(es) => format!(
                "Lent.tupleOpt(&.{{ {} }}, {expr})",
                es.iter().map(|e| sub(typed(e, false))).collect::<Vec<_>>().join(", ")
            ),
            Ty::Listener(ev) => format!("Lent.listenerOpt(JsListener({}), {expr})", sub(typed(ev, false))),
            Ty::Mapped(m) => format!("Lent.valueOpt({m}.Type({key}), {expr})"),
            _ => unreachable!(),
        },
        _ => unreachable!(),
    }
}

// Expression turning a typed value into what crosses the boundary.
fn lower_arg(name: &str, t: &Ty) -> String {
    if is_lent_ty(t) {
        return format!("lent_{name}.h");
    }
    match t {
        Ty::Num => name.into(),
        Ty::Bool => format!("@intFromBool({name})"),
        Ty::Enum(_) | Ty::Key(_) => format!("@intFromEnum({name})"),
        Ty::NumEnum(_) => format!("@floatFromInt(@intFromEnum({name}))"),
        Ty::Nullable(inner, _) if **inner == Ty::Bool => format!("if ({name}) |v| @as(i32, @intFromBool(v)) else -1"),
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Enum(_)) => format!("if ({name}) |v| @intFromEnum(v) else -1"),
        _ => unreachable!(),
    }
}

#[allow(clippy::too_many_arguments)]
fn func(out: &mut String, externs: &mut String, owner: Option<&str>, f: &Func, zig_name: &str, indent: &str, unions: &[NamedUnion], wasm_module: &str, emitted_imports: &mut HashSet<String>) {
    let mut raw_params = Vec::new();
    let mut params = Vec::new();
    let mut args = Vec::new();
    if Class::has_self(f) {
        raw_params.push("u32".to_string());
        params.push(format!("self: {}", owner.unwrap()));
        args.push("self.h".to_string());
    }
    // A key is known at compile time: the types it selects are computed from it.
    let key = f.params.iter().find(|p| matches!(p.1, Ty::Key(_))).map(|p| param(&p.0)).unwrap_or_default();
    // Trailing optional parameters are accepted via `options: anytype` (supporting
    // `.{}`, `null`, `.{ .field = val }`, or a direct single value / struct literal).
    let setter = matches!(f.call, Call::Set(_) | Call::StaticSet(_) | Call::IndexSet);
    let optional = if setter { 0 } else { f.params.iter().rev().take_while(|p| matches!(p.1, Ty::Nullable(_, true))).count() };
    // A lone parameter stays one: `orUnit(s)`, `orUnit(null)`.
    let optional = if optional == f.params.len() && optional == 1 { 0 } else { optional };
    let first_optional = f.params.len() - optional;
    let mut allowed = Vec::new();
    let mut unpack = String::new();
    let mut pre = String::new();
    for (i, (name, t)) in f.params.iter().enumerate() {
        let id = param(name);
        if i >= first_optional {
            let Ty::Nullable(inner, _) = t else { unreachable!() };
            if optional == 1 {
                let inner_ty = typed(inner, true);
                let inner_ty = if key.is_empty() { inner_ty } else { inner_ty.replace("KEY", &key) };
                let unwrap = format!("Lent.singleOpt({inner_ty}, {name:?}, options)");
                if is_lent_ty(t) {
                    pre += &format!("const lent_{id} = {}; defer lent_{id}.done(); ", lent_expr(&unwrap, t, &key));
                } else {
                    let val_expr = match &**inner {
                        Ty::Num => format!("Lent.optNum({unwrap})"),
                        Ty::Bool => format!("Lent.optBool({unwrap})"),
                        Ty::Str => format!("Lent.optStr({unwrap})"),
                        Ty::Enum(n) | Ty::NumEnum(n) => format!("Lent.optEnum({n}, {unwrap})"),
                        _ => unreachable!(),
                    };
                    unpack += &format!("const {id} = {val_expr}; ");
                }
            } else {
                allowed.push(format!("{name:?}"));
                let field_expr = format!("@field(options, {name:?})");
                let cond = format!("comptime Lent.hasOpt(@TypeOf(options), {name:?})");
                if is_lent_ty(t) {
                    pre += &format!("const lent_{id}: Lent = if ({cond}) {} else Lent.none; defer lent_{id}.done(); ", lent_expr(&field_expr, t, &key));
                } else {
                    let line = match &**inner {
                        Ty::Num => format!("const {id}: ?f64 = if ({cond}) Lent.optNum({field_expr}) else null; "),
                        Ty::Bool => format!("const {id}: ?bool = if ({cond}) Lent.optBool({field_expr}) else null; "),
                        Ty::Str => format!("const {id}: ?[]const u8 = if ({cond}) Lent.optStr({field_expr}) else null; "),
                        Ty::Enum(n) | Ty::NumEnum(n) => format!("const {id}: ?{n} = if ({cond}) Lent.optEnum({n}, {field_expr}) else null; "),
                        _ => unreachable!(),
                    };
                    unpack += &line;
                }
            }
        } else if matches!(t, Ty::Key(_)) {
            params.push(format!("comptime {id}: {}", typed(t, true).replace("KEY", &key)));
        } else if is_lent_ty(t) {
            params.push(format!("{id}: anytype"));
            pre += &format!("const lent_{id} = {}; defer lent_{id}.done(); ", lent_expr(&id, t, &key));
        } else {
            params.push(format!("{id}: {}", typed(t, true).replace("KEY", &key)));
        }
        if *t == Ty::Str {
            raw_params.extend(["[*]const u8".to_string(), "usize".to_string()]);
            args.extend([format!("{id}.ptr"), format!("{id}.len")]);
        } else if matches!(t, Ty::Nullable(inner, _) if matches!(**inner, Ty::Num | Ty::NumEnum(_))) {
            // An optional number: whether it is there, and the number.
            let value = if matches!(t, Ty::Nullable(inner, _) if **inner == Ty::Num) { format!("{id} orelse 0") } else { format!("if ({id}) |v| @floatFromInt(@intFromEnum(v)) else 0") };
            raw_params.extend(["i32".to_string(), "f64".to_string()]);
            args.extend([format!("@intFromBool({id} != null)"), value]);
        } else if matches!(t, Ty::Nullable(inner, _) if **inner == Ty::Str) {
            raw_params.extend(["?[*]const u8".to_string(), "usize".to_string()]);
            args.extend([format!("if ({id}) |v| v.ptr else null"), format!("if ({id}) |v| v.len else 0")]);
        } else {
            raw_params.push(raw(t).to_string());
            args.push(lower_arg(&id, t));
        }
    }
    if optional > 1 {
        unpack = format!("Lent.checkOpts(options, &.{{ {} }}); {unpack}", allowed.join(", "));
    }
    if optional > 0 {
        params.push("options: anytype".to_string());
    }
    // A shared import takes the index of this function's fixed arguments after `self`.
    let import = match &f.shared {
        Some((shared, k)) => {
            let at = usize::from(Class::has_self(f));
            raw_params.insert(at, "i32".into());
            args.insert(at, k.to_string());
            shared.as_str()
        }
        None => f.import.as_str(),
    };
    // Quoted: an import may be named as a keyword (`error`).
    let call = format!("raw.@\"{import}\"({})", args.join(", "));
    // A union result that cannot be told apart is an untyped value.
    let union = match &f.ret {
        Ty::NamedUnion(n) => unions.iter().find(|u| u.name == *n),
        Ty::Nullable(inner, _) => match &**inner {
            Ty::NamedUnion(n) => unions.iter().find(|u| u.name == *n),
            _ => None,
        },
        _ => None,
    };
    let ret_type = if f.returns_this && owner == Some("Self") {
        "Self".to_string()
    } else {
        match union {
            Some(u) if !u.decodable() => "JsValue".to_string(),
            _ => typed(&f.ret, false).replace("KEY", &key),
        }
    };
    let body = match &f.ret {
        Ty::NamedUnion(n) if union.is_some_and(|u| u.decodable()) => format!("const h = {call}; if (h == 0 and jsHasError()) return undefined; return {n}.fromHandle(h);"),
        Ty::Nullable(inner, _) if union.is_some_and(|u| u.decodable()) => format!("const h = {call}; return if (h == 0) null else {}.fromHandle(h);", typed(inner, false)),
        Ty::Nullable(..) if union.is_some() => format!("return .{{ .h = {call} }};"),
        Ty::Nullable(inner, _) if **inner == Ty::Num => format!("return jsTake(?f64, {call});"),
        Ty::Nullable(inner, _) if matches!(**inner, Ty::NumEnum(_)) => format!("const h = {call}; return if (h == 0) null else @enumFromInt(@as(i32, @intFromFloat(JsValue.takeNumber(h))));"),
        Ty::Nullable(inner, _) if **inner == Ty::Bool => format!("const r = {call}; return if (r < 0) null else r != 0;"),
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Enum(_)) => format!("const r = {call}; return if (r < 0) null else @enumFromInt(r);"),
        Ty::Void => format!("{call};"),
        Ty::Num => format!("return {call};"),
        Ty::Bool => format!("return {call} != 0;"),
        Ty::Enum(_) => format!("const r = {call}; if (jsHasError()) return undefined; return @enumFromInt(r);"),
        Ty::NumEnum(_) => format!("const r = {call}; if (jsHasError()) return undefined; return @enumFromInt(@as(i32, @intFromFloat(r)));"),
        Ty::Nullable(..) => format!("const h = {call}; return if (h == 0) null else .{{ .h = h }};"),
        _ => format!("return .{{ .h = {call} }};"),
    };
    emitted_imports.insert(import.to_string());
    let decl = format!("    extern \"{wasm_module}\" fn @\"{import}\"({}) {};\n", raw_params.join(", "), raw(&f.ret));
    // Declared once per file, though several functions use it.
    // Only a shared import can repeat; checking others would be quadratic.
    if f.shared.is_none() || !externs.contains(&decl) {
        *externs += &decl;
    }
    *out += &format!("{indent}pub fn {}({}) {ret_type} {{ {unpack}{pre}{body} }}\n", ident(zig_name), params.join(", "));
}

const RUNTIME: &str = include_str!("runtime/zig_runtime.zig");

const RUNTIME_EXTERNS: &str = include_str!("runtime/zig_runtime_externs.zig");

// The bindings as files: `runtime.zig`; `types.zig` with enums, unions,
// callbacks and handles declared elsewhere; one file per module with its
// types and functions; and `bindings.zig`, which re-exports every name. Each
// file imports by name what it uses from the others, and Zig only reads a
// file when one of its names is used.
// What Zig cannot express of a model: generics.
fn check(m: &Model) -> super::Problems {
    let mut problems = super::Problems::default();
    super::model::each_type(m, &mut |at, t| {
        if matches!(t, Ty::Param(_) | Ty::Generic(..)) {
            problems.add("Zig generator cannot express generics: set generics = \"erase\"".into(), at.to_string());
        }
    });
    problems
}

pub fn files(m: &Model) -> Result<Vec<(String, String)>, Vec<String>> {
    files_named(m, "bindings", &[])
}

pub fn files_named(m: &Model, pkg_name: &str, refs: &[ProjectRef]) -> Result<Vec<(String, String)>, Vec<String>> {
    files_with_imports(m, pkg_name, refs).map(|(files, _)| files)
}

pub fn files_with_imports(m: &Model, pkg_name: &str, refs: &[ProjectRef]) -> Result<(Vec<(String, String)>, HashSet<String>), Vec<String>> {
    let problems = check(m);
    if !problems.is_empty() {
        return Err(problems.into_messages());
    }
    let wasm_module = if pkg_name == "bindings" { "env" } else { pkg_name };
    let mut emitted_imports: HashSet<String> = HashSet::new();
    let mut resolved_opaque: HashMap<String, (usize, String)> = HashMap::new();
    let mut used_refs: BTreeSet<usize> = BTreeSet::new();
    for name in &m.opaque {
        if matches!(name.as_str(), "Promise" | "Error" | "Date") || super::model::buffer_spec(name).is_some() {
            continue;
        }
        let orig = m.opaque_names.get(name).map(String::as_str).unwrap_or(name.as_str());
        if let Some((idx, _)) = refs.iter().enumerate().find(|(_, r)| r.types.contains(orig)) {
            resolved_opaque.insert(name.clone(), (idx, orig.to_string()));
        }
    }
    // File -> (declarations, externs, names it declares).
    let mut files: BTreeMap<String, (String, String, Vec<String>)> = BTreeMap::new();
    // File -> the names of other files its declarations may use: the types
    // they name, from the model, not from the source written.
    let mut uses: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    files.insert("runtime".into(), (RUNTIME.into(), RUNTIME_EXTERNS.into(), RUNTIME_NAMES.iter().map(|n| n.to_string()).collect()));
    let mut types = (String::new(), String::new(), Vec::new());
    for (id, (name, values)) in m.enums.iter().enumerate() {
        let mut seen = std::collections::HashSet::new();
        let members: Vec<String> = values.iter().map(|v| if v.is_empty() { "empty" } else { v }).filter(|v| seen.insert(v.to_string())).map(ident).collect();
        // `js_id`: which enum of strings it is, for its values to cross as handles.
        types.0 += &format!(
            "pub const {name} = enum(i32) {{ {}, pub const js_id: i32 = {id}; pub fn toJsHandle(self: @This()) u32 {{ return raw.js_enum_value({id}, @intFromEnum(self)); }} pub fn fromJsHandle(h: u32) @This() {{ return @enumFromInt(raw.js_enum_index({id}, h)); }} }};\n",
            members.join(", ")
        );
        types.2.push(name.clone());
    }
    if !m.enums.is_empty() {
        types.1 += &format!(
            "    extern \"{wasm_module}\" fn js_enum_value(id: i32, i: i32) u32;\n    extern \"{wasm_module}\" fn js_enum_index(id: i32, h: u32) i32;\n"
        );
    }
    for (name, members) in &m.num_enums {
        // A value named twice (`LEFT = 0`, `ROTATE = 0`): the second is a constant of the first.
        let mut first: HashMap<&str, &str> = HashMap::new();
        let mut fields = Vec::new();
        let mut aliases = String::new();
        for (member, value) in members {
            match first.get(value.as_str()) {
                Some(f) => aliases += &format!(" pub const {} = @This().{};", ident(member), ident(f)),
                None => {
                    first.insert(value, member);
                    fields.push(format!("{} = {value}", ident(member)));
                }
            }
        }
        types.0 += &format!("pub const {name} = enum(i32) {{ {},{aliases} }};\n", fields.join(", "));
        types.2.push(name.clone());
    }
    let types_uses = uses.entry("types".into()).or_default();
    for u in &m.unions {
        u.parts.iter().for_each(|p| type_refs(p, types_uses));
        types.0 += &union_decl(u);
        emitted_imports.insert(format!("{}_tag", u.name));
        types.1 += &format!("    extern \"{wasm_module}\" fn {}_tag(h: u32, k: i32) i32;\n", u.name);
        types.2.push(u.name.clone());
    }
    let iterates_async = m.classes.iter().any(|c| c.iterates_async.is_some());
    let mut opaque = m.opaque.clone();
    if iterates_async && !opaque.iter().any(|n| n == "Promise") {
        opaque.push("Promise".into());
    }
    for c in &m.callbacks {
        c.params.iter().for_each(|p| type_refs(&p.1, types_uses));
        type_refs(&c.ret, types_uses);
    }
    if iterates_async {
        types.0 += "// A JavaScript async iterator: `next()` is a promise of the next value, or of a null handle at the end.\n\
                    pub const JsAsyncIterator = extern struct {\n    h: u32,\n    \
                    pub fn over(h: u32) JsAsyncIterator { return .{ .h = raw.js_async_iterate(h) }; }\n    \
                    pub fn next(self: JsAsyncIterator) Promise { return .{ .h = raw.js_async_iterator_next(self.h) }; }\n    \
                    pub fn deinit(self: JsAsyncIterator) void { jsRelease(self); }\n    \
                    pub fn retain(self: JsAsyncIterator) JsAsyncIterator { return jsRetain(self); }\n    \
                    pub fn as(self: JsAsyncIterator, comptime Target_: type) Target_ { return .{ .h = self.h }; }\n};\n";
        types.1 += "    extern \"env\" fn js_async_iterate(h: u32) u32;\n    extern \"env\" fn js_async_iterator_next(it: u32) u32;\n";
        types.2.push("JsAsyncIterator".into());
    }
    let mut need_error_externs = false;
    let mut need_date_externs = false;
    let mut need_bytes_externs = false;
    for name in &opaque {
        if name == "Promise" {
            // Calls f(ok, value) / f(ctx, ok, value) once it settles; the value is lent.
            types.0 += r#"pub const Promise = extern struct {
    h: u32,
    pub fn then(self: Promise, comptime f_: anytype, ctx_: anytype) void {
        const info_ = @typeInfo(@TypeOf(f_)).@"fn";
        if (info_.param_types.len == 2) {
            const T_ = struct { fn call(_: ?*anyopaque, ok_: i32, v_: u32) callconv(.c) void { f_(ok_ != 0, .{ .h = v_ }); } };
            raw.js_promise_then(self.h, @intFromPtr(&T_.call), null);
            return;
        }
        const Ptr_ = info_.param_types[0].?;
        if (Ptr_ == ?*anyopaque) {
            const T_ = struct { fn call(c_: ?*anyopaque, ok_: i32, v_: u32) callconv(.c) void { f_(c_, ok_ != 0, .{ .h = v_ }); } };
            raw.js_promise_then(self.h, @intFromPtr(&T_.call), Lent.toRawCtx(ctx_));
            return;
        }
        const typed_ctx_: Ptr_ = ctx_;
        const raw_ctx_ = Lent.toRawCtx(typed_ctx_);
        const T_ = struct {
            fn call(c_: ?*anyopaque, ok_: i32, v_: u32) callconv(.c) void {
                if (@typeInfo(Ptr_) == .optional) {
                    f_(if (c_) |p_| @ptrCast(@alignCast(p_)) else null, ok_ != 0, .{ .h = v_ });
                } else {
                    f_(@ptrCast(@alignCast(c_.?)), ok_ != 0, .{ .h = v_ });
                }
            }
        };
        raw.js_promise_then(self.h, @intFromPtr(&T_.call), raw_ctx_);
    }
    pub fn thenFn(self: Promise, comptime f_: anytype) void { self.then(f_, null); }
    pub fn deinit(self: Promise) void { jsRelease(self); }
    pub fn retain(self: Promise) Promise { return jsRetain(self); }
    pub fn as(self: Promise, comptime Target_: type) Target_ { return .{ .h = self.h }; }
};
"#;
            types.1 += "    extern \"env\" fn js_promise_then(p: u32, f: usize, ctx: ?*anyopaque) void;\n";
        } else if name == "Error" {
            need_error_externs = true;
            types.0 += "pub const Error = extern struct {\n    h: u32,\n    \
                        pub fn new(msg: []const u8) Error { return .{ .h = raw.js_error_new(msg.ptr, msg.len) }; }\n    \
                        pub fn name(self: Error) JsString { return .{ .h = raw.js_error_name(self.h) }; }\n    \
                        pub fn message(self: Error) JsString { return .{ .h = raw.js_error_message(self.h) }; }\n    \
                        pub fn stack(self: Error) JsString { return .{ .h = raw.js_error_stack(self.h) }; }\n    \
                        pub fn deinit(self: Error) void { jsRelease(self); }\n    \
                        pub fn retain(self: Error) Error { return jsRetain(self); }\n    \
                        pub fn as(self: Error, comptime Target_: type) Target_ { return .{ .h = self.h }; }\n};\n";
        } else if name == "Date" {
            need_date_externs = true;
            types.0 += "pub const Date = extern struct {\n    h: u32,\n    \
                        pub fn fromTime(ms: f64) Date { return .{ .h = raw.js_date_new(ms) }; }\n    \
                        pub fn now() f64 { return raw.js_date_now(); }\n    \
                        pub fn time(self: Date) f64 { return raw.js_date_time(self.h); }\n    \
                        pub fn toIsoString(self: Date) JsString { return .{ .h = raw.js_date_to_iso_string(self.h) }; }\n    \
                        pub fn deinit(self: Date) void { jsRelease(self); }\n    \
                        pub fn retain(self: Date) Date { return jsRetain(self); }\n    \
                        pub fn as(self: Date, comptime Target_: type) Target_ { return .{ .h = self.h }; }\n};\n";
        } else if let Some(s) = super::model::buffer_spec(name) {
            need_bytes_externs = true;
            types.0 += &format!(
                "pub const {name} = extern struct {{\n    h: u32,\n    \
                 pub fn byteLength(self: {name}) usize {{ return raw.js_bytes_len(self.h); }}\n    \
                 pub fn readBytes(self: {name}, offset: usize, dst: []u8) usize {{ return raw.js_bytes_read(self.h, offset, dst.ptr, dst.len); }}\n    \
                 pub fn writeBytes(self: {name}, offset: usize, src: []const u8) usize {{ return raw.js_bytes_write(self.h, offset, src.ptr, src.len); }}\n"
            );
            if s.has_buffer() {
                types.0 += &format!(
                    "    pub fn byteOffset(self: {name}) usize {{ return raw.js_bytes_offset(self.h); }}\n    \
                     pub fn buffer(self: {name}) JsValue {{ return .{{ .h = raw.js_bytes_buffer(self.h) }}; }}\n"
                );
            }
            if s.is_typed_array() {
                let kind = s.kind.unwrap();
                let elem = zig_elem(s.elem);
                let eb = s.elem.bytes();
                types.0 += &format!(
                    "    pub fn new(count: usize) {name} {{ const n = if (count > (~@as(usize, 0)) / {eb}) ~@as(usize, 0) else count * {eb}; return .{{ .h = raw.js_bytes_new({kind}, null, n) }}; }}\n    \
                     pub fn from(data: []const {elem}) {name} {{ const n = if (data.len > (~@as(usize, 0)) / {eb}) ~@as(usize, 0) else data.len * {eb}; return .{{ .h = raw.js_bytes_new({kind}, @ptrCast(data.ptr), n) }}; }}\n    \
                     pub fn len(self: {name}) usize {{ return raw.js_bytes_len(self.h) / {eb}; }}\n    \
                     pub fn read(self: {name}, elem_offset: usize, dst: []{elem}) usize {{ if (elem_offset > (~@as(usize, 0)) / {eb}) return 0; const n = if (dst.len > (~@as(usize, 0)) / {eb}) ~@as(usize, 0) else dst.len * {eb}; return raw.js_bytes_read(self.h, elem_offset * {eb}, @ptrCast(dst.ptr), n) / {eb}; }}\n    \
                     pub fn write(self: {name}, elem_offset: usize, src: []const {elem}) usize {{ if (elem_offset > (~@as(usize, 0)) / {eb}) return 0; const n = if (src.len > (~@as(usize, 0)) / {eb}) ~@as(usize, 0) else src.len * {eb}; return raw.js_bytes_write(self.h, elem_offset * {eb}, @ptrCast(src.ptr), n) / {eb}; }}\n"
                );
            } else {
                if let Some(kind) = s.kind {
                    types.0 += &format!(
                        "    pub fn new(byte_len: usize) {name} {{ return .{{ .h = raw.js_bytes_new({kind}, null, byte_len) }}; }}\n    \
                         pub fn from(data: []const u8) {name} {{ return .{{ .h = raw.js_bytes_new({kind}, data.ptr, data.len) }}; }}\n"
                    );
                }
                types.0 += &format!(
                    "    pub fn read(self: {name}, offset: usize, dst: []u8) usize {{ return raw.js_bytes_read(self.h, offset, dst.ptr, dst.len); }}\n    \
                     pub fn write(self: {name}, offset: usize, src: []const u8) usize {{ return raw.js_bytes_write(self.h, offset, src.ptr, src.len); }}\n"
                );
            }
            types.0 += &format!(
                "    pub fn deinit(self: {name}) void {{ jsRelease(self); }}\n    \
                 pub fn retain(self: {name}) {name} {{ return jsRetain(self); }}\n    \
                 pub fn as(self: {name}, comptime Target_: type) Target_ {{ return .{{ .h = self.h }}; }}\n}};\n"
            );
        } else if let Some((idx, orig)) = resolved_opaque.get(name) {
            used_refs.insert(*idx);
            types.0 += &format!("pub const {name} = @import(\"{}\").{orig};\n", refs[*idx].pkg_name);
        } else {
            types.0 += &format!("// external\npub const {name} = extern struct {{ h: u32, pub fn deinit(self: @This()) void {{ jsRelease(self); }} pub fn retain(self: @This()) @This() {{ return jsRetain(self); }} pub fn as(self: @This(), comptime Target_: type) Target_ {{ return .{{ .h = self.h }}; }} }};\n");
        }
        types.2.push(name.clone());
    }
    if need_error_externs {
        types.1 += "    extern \"env\" fn js_error_new(ptr: [*]const u8, len: usize) u32;\n    \
                    extern \"env\" fn js_error_name(h: u32) u32;\n    \
                    extern \"env\" fn js_error_message(h: u32) u32;\n    \
                    extern \"env\" fn js_error_stack(h: u32) u32;\n";
    }
    if need_date_externs {
        types.1 += "    extern \"env\" fn js_date_new(ms: f64) u32;\n    \
                    extern \"env\" fn js_date_now() f64;\n    \
                    extern \"env\" fn js_date_time(h: u32) f64;\n    \
                    extern \"env\" fn js_date_to_iso_string(h: u32) u32;\n";
    }
    if need_bytes_externs {
        types.1 += "    extern \"env\" fn js_bytes_len(h: u32) usize;\n    \
                    extern \"env\" fn js_bytes_read(h: u32, off: usize, dst: [*]u8, len: usize) usize;\n    \
                    extern \"env\" fn js_bytes_write(h: u32, off: usize, src: [*]const u8, len: usize) usize;\n    \
                    extern \"env\" fn js_bytes_new(kind: u32, src: ?[*]const u8, byte_len: usize) u32;\n    \
                    extern \"env\" fn js_bytes_buffer(h: u32) u32;\n    \
                    extern \"env\" fn js_bytes_offset(h: u32) usize;\n";
    }
    // A callback is made from a function and a context pointer passed back to it.
    // Supports stateless `fn (...) R`, typed ctx `fn (*T, ...) R`, or legacy `fn (?*anyopaque, ...) R`.
    for c in &m.callbacks {
        let mut params = vec!["?*anyopaque".to_string()];
        params.extend(c.params.iter().map(|(_, t)| callback_param_ty(t, &m.unions)));
        let raw_params: Vec<String> = c.params.iter().enumerate().map(|(i, (_, t))| format!("a{i}: {}", raw_callback(t))).collect();
        let args: Vec<String> = c.params.iter().enumerate().map(|(i, (_, t))| callback_arg(t, &format!("a{i}"), &m.unions)).collect();
        let args_str = args.join(", ");
        let n_params = c.params.len();
        let ret = typed(&c.ret, false);
        let raw_ret = raw_callback(&c.ret);
        let raw_suffix: String = raw_params.iter().map(|p| format!(", {p}")).collect();
        let call_stateless = format!("f_({args_str})");
        let call_c = if args_str.is_empty() { "f_(c_)".to_string() } else { format!("f_(c_, {args_str})") };
        let call_opt = if args_str.is_empty() {
            "f_(if (c_) |p_| @ptrCast(@alignCast(p_)) else null)".to_string()
        } else {
            format!("f_(if (c_) |p_| @ptrCast(@alignCast(p_)) else null, {args_str})")
        };
        let call_ptr = if args_str.is_empty() {
            "f_(@ptrCast(@alignCast(c_.?)))".to_string()
        } else {
            format!("f_(@ptrCast(@alignCast(c_.?)), {args_str})")
        };
        let body_stateless = callback_result(&c.ret, &call_stateless);
        let body_c = callback_result(&c.ret, &call_c);
        let body_opt = callback_result(&c.ret, &call_opt);
        let body_ptr = callback_result(&c.ret, &call_ptr);
        types.0 += &format!("\npub const {} = extern struct {{\n    h: u32,\n", c.name);
        types.0 += &format!("    pub const Fn = fn ({}) {ret};\n", params.join(", "));
        types.0 += &format!(
            "    pub fn from(comptime f_: anytype, ctx_: anytype) {0} {{\n        \
             const info_ = @typeInfo(@TypeOf(f_)).@\"fn\";\n        \
             if (info_.param_types.len == {n_params}) {{\n            \
                 const T_ = struct {{\n                fn call(_: ?*anyopaque{raw_suffix}) callconv(.c) {raw_ret} {{ {body_stateless} }}\n            }};\n            \
                 return .{{ .h = raw.{0}_from(@intFromPtr(&T_.call), null) }};\n        }}\n        \
             const Ptr_ = info_.param_types[0].?;\n        \
             if (Ptr_ == ?*anyopaque) {{\n            \
                 const T_ = struct {{\n                fn call(c_: ?*anyopaque{raw_suffix}) callconv(.c) {raw_ret} {{ {body_c} }}\n            }};\n            \
                 return .{{ .h = raw.{0}_from(@intFromPtr(&T_.call), Lent.toRawCtx(ctx_)) }};\n        }}\n        \
             const typed_ctx_: Ptr_ = ctx_;\n        \
             const raw_ctx_ = Lent.toRawCtx(typed_ctx_);\n        \
             const T_ = struct {{\n            \
                 fn call(c_: ?*anyopaque{raw_suffix}) callconv(.c) {raw_ret} {{\n                \
                     if (@typeInfo(Ptr_) == .optional) {{ {body_opt} }} else {{ {body_ptr} }}\n            \
                 }}\n        }};\n        \
             return .{{ .h = raw.{0}_from(@intFromPtr(&T_.call), raw_ctx_) }};\n    }}\n    \
             pub fn fromFn(comptime f_: anytype) {0} {{ return from(f_, null); }}\n",
            c.name
        );
        if let Some(apply_f) = m.funcs.iter().find(|f| f.call == Call::Apply && matches!(f.params.first(), Some((_, Ty::Obj(n))) if n == &c.name)) {
            for p in &apply_f.params[1..] {
                type_refs(&p.1, types_uses);
            }
            type_refs(&apply_f.ret, types_uses);
            let method = Func {
                type_params: Vec::new(),
                module: String::new(),
                import: apply_f.import.clone(),
                name: "call".into(),
                call: Call::Invoke,
                params: apply_f.params[1..].to_vec(),
                rest: apply_f.rest,
                fixed: apply_f.fixed.clone(),
                shared: apply_f.shared.clone(),
                ret: apply_f.ret.clone(),
                returns_this: false,
            };
            func(&mut types.0, &mut types.1, Some(&c.name), &method, "call", "    ", &m.unions, wasm_module, &mut emitted_imports);
        }
        types.0 += &format!(
            "    pub fn deinit(self: {0}) void {{ jsRelease(self); }}\n    \
             pub fn retain(self: {0}) {0} {{ return jsRetain(self); }}\n    \
             pub fn as(self: {0}, comptime Target_: type) Target_ {{ return .{{ .h = self.h }}; }}\n}};\n",
            c.name
        );
        emitted_imports.insert(format!("{}_from", c.name));
        types.1 += &format!("    extern \"{wasm_module}\" fn {}_from(f: usize, ctx: ?*anyopaque) u32;\n", c.name);
        types.2.push(c.name.clone());
    }
    files.insert("types".into(), types);
    // With inheritance flattened, a type has its ancestors' members. Each is
    // written once, by the furthest ancestor that declares it, in
    // `X.Methods(Self)`; a subtype's member of that name is an alias of it.
    let class_of: HashMap<&str, &Class> = m.classes.iter().map(|c| (c.name.as_str(), c)).collect();
    let mut ancestors: HashMap<&str, Vec<&Class>> = HashMap::new();
    for c in &m.classes {
        let mut up: Vec<&Class> = Vec::new();
        let mut todo: Vec<&Class> = vec![c];
        while let Some(k) = todo.pop() {
            for a in k.ancestors.iter().filter_map(|a| class_of.get(a.as_str())) {
                if !up.iter().any(|u| u.name == a.name) {
                    up.push(a);
                    todo.push(a);
                }
            }
        }
        up.sort_by(|a, b| a.name.cmp(&b.name));
        ancestors.insert(&c.name, up);
    }
    let same = |a: &Func, b: &Func| a.call == b.call && a.params == b.params && (a.ret == b.ret || (a.returns_this && b.returns_this)) && a.rest == b.rest && a.fixed == b.fixed && a.type_params == b.type_params;
    let members: HashMap<&str, HashMap<String, Vec<&Func>>> = m
        .classes
        .iter()
        .map(|c| {
            let mut by_call: HashMap<String, Vec<&Func>> = HashMap::new();
            c.funcs.iter().filter(|f| Class::has_self(f)).for_each(|f| by_call.entry(format!("{:?}", f.call)).or_default().push(f));
            (c.name.as_str(), by_call)
        })
        .collect();
    let declares = |k: &Class, f: &Func| members[k.name.as_str()].get(&format!("{:?}", f.call)).and_then(|fs| fs.iter().find(|g| same(g, f)).copied());
    // (type, member) -> (the ancestor that writes it, its name there).
    let mut homes: HashMap<(&str, &str), (&str, &str)> = HashMap::new();
    for c in &m.classes {
        for f in c.funcs.iter().filter(|f| Class::has_self(f)) {
            let home = ancestors[c.name.as_str()].iter().filter_map(|a| declares(a, f).map(|g| (*a, g))).find(|(a, _)| !ancestors[a.name.as_str()].iter().any(|b| declares(b, f).is_some()));
            if let Some((a, g)) = home {
                homes.insert((&c.name, &f.name), (&a.name, &g.name));
            }
        }
    }
    let mut shares: HashSet<&str> = homes.values().map(|h| h.0).collect();
    let class_names: HashMap<&str, HashMap<String, String>> = m.classes.iter().map(|c| (c.name.as_str(), names_of(c.funcs.iter()))).collect();
    // Alias methods from unflattened ancestors (e.g. generic mixins / Container / Node)
    // directly onto subclasses when the subclass does not already define that member.
    let inherited_consts = |c: &Class| -> Vec<(String, String)> {
        let mut out: Vec<(String, String)> = c.consts.clone();
        let mut seen_const: HashSet<String> = c.consts.iter().map(|(n, _)| n.clone()).collect();
        let mut cq: std::collections::VecDeque<&Class> = c.ancestors.iter().filter_map(|a| class_of.get(a.as_str()).copied()).collect();
        let mut seen_anc = HashSet::from([c.name.as_str()]);
        while let Some(a) = cq.pop_front() {
            if !seen_anc.insert(a.name.as_str()) {
                continue;
            }
            for (k, v) in &a.consts {
                if seen_const.insert(k.clone()) {
                    out.push((k.clone(), v.clone()));
                }
            }
            for pa in a.ancestors.iter().filter_map(|p| class_of.get(p.as_str()).copied()) {
                if !seen_anc.contains(pa.name.as_str()) {
                    cq.push_back(pa);
                }
            }
        }
        out
    };
    let inherited_iter = |c: &Class| -> (Option<Ty>, bool) {
        let mut iter = c.iterates.clone();
        let mut async_iter = c.iterates_async.is_some();
        let mut iq: std::collections::VecDeque<&Class> = c.ancestors.iter().filter_map(|a| class_of.get(a.as_str()).copied()).collect();
        let mut seen_anc = HashSet::from([c.name.as_str()]);
        while (iter.is_none() || !async_iter)
            && let Some(a) = iq.pop_front()
        {
            if !seen_anc.insert(a.name.as_str()) {
                continue;
            }
            if iter.is_none() {
                iter = a.iterates.clone();
            }
            if a.iterates_async.is_some() {
                async_iter = true;
            }
            for pa in a.ancestors.iter().filter_map(|p| class_of.get(p.as_str()).copied()) {
                if !seen_anc.contains(pa.name.as_str()) {
                    iq.push_back(pa);
                }
            }
        }
        (iter, async_iter)
    };
    let inherited_elements = |c: &Class| -> Option<Ty> {
        if let Some(e) = &c.elements {
            return Some(e.clone());
        }
        let mut eq: std::collections::VecDeque<&Class> = c.ancestors.iter().filter_map(|a| class_of.get(a.as_str()).copied()).collect();
        let mut seen_anc = HashSet::from([c.name.as_str()]);
        while let Some(a) = eq.pop_front() {
            if !seen_anc.insert(a.name.as_str()) {
                continue;
            }
            if let Some(e) = &a.elements {
                return Some(e.clone());
            }
            for pa in a.ancestors.iter().filter_map(|p| class_of.get(p.as_str()).copied()) {
                if !seen_anc.contains(pa.name.as_str()) {
                    eq.push_back(pa);
                }
            }
        }
        None
    };
    let consts_of: HashMap<&str, Vec<(String, String)>> = m.classes.iter().map(|c| (c.name.as_str(), inherited_consts(c))).collect();
    let iters_of: HashMap<&str, (Option<Ty>, bool)> = m.classes.iter().map(|c| (c.name.as_str(), inherited_iter(c))).collect();
    let elems_of: HashMap<&str, Option<Ty>> = m.classes.iter().map(|c| (c.name.as_str(), inherited_elements(c))).collect();
    let mut extra_inherited: HashMap<&str, Vec<(&str, &str, &str)>> = HashMap::new();
    for c in &m.classes {
        let own_names = &class_names[c.name.as_str()];
        let c_consts = &consts_of[c.name.as_str()];
        let (ref c_iter, c_iter_async) = iters_of[c.name.as_str()];
        let c_elem = &elems_of[c.name.as_str()];
        let c_iter = c_iter.as_ref().or(c_elem.as_ref());
        let mut taken: HashSet<String> = ["h", "Methods", "Key", "Type"].into_iter().map(String::from).collect();
        taken.extend(c_consts.iter().map(|(n, _)| n.clone()));
        for a in ancestors[c.name.as_str()].iter().map(|a| a.name.as_str()).chain(c.ancestors.iter().map(String::as_str)) {
            if a != c.name {
                taken.insert(format!("as{a}"));
            }
        }
        if c_iter_async && !c.funcs.iter().any(|f| f.name == "iterateAsync") {
            taken.insert("iterateAsync".into());
        }
        if c_iter.is_some() && !c.funcs.iter().any(|f| f.name == "iterate") {
            taken.insert("iterate".into());
        }
        if c.js.is_some() {
            let cast_name = if own_names.values().any(|n| n == "cast") || c.consts.iter().any(|(n, _)| n == "cast") { "tryCast" } else { "cast" };
            taken.insert(cast_name.into());
        }
        taken.extend(own_names.values().cloned());

        let mut extras = Vec::new();
        let mut queue: std::collections::VecDeque<&Class> = c.ancestors.iter().filter_map(|a| class_of.get(a.as_str()).copied()).collect();
        let mut seen_anc: HashSet<&str> = HashSet::from([c.name.as_str()]);
        while let Some(a) = queue.pop_front() {
            if !seen_anc.insert(a.name.as_str()) {
                continue;
            }
            for f in a.funcs.iter().filter(|f| Class::has_self(f)) {
                let zname = class_names[a.name.as_str()][&f.name].as_str();
                if taken.insert(zname.to_string()) {
                    let (home_cls, home_fname) = homes.get(&(a.name.as_str(), f.name.as_str())).copied().unwrap_or((a.name.as_str(), f.name.as_str()));
                    let home_zname = class_names[home_cls][home_fname].as_str();
                    shares.insert(home_cls);
                    extras.push((zname, home_cls, home_zname));
                }
            }
            for pa in a.ancestors.iter().filter_map(|p| class_of.get(p.as_str()).copied()) {
                if !seen_anc.contains(pa.name.as_str()) {
                    queue.push_back(pa);
                }
            }
        }
        extra_inherited.insert(c.name.as_str(), extras);
    }
    // Every object is an opaque handle; methods live in the struct's namespace.
    for c in &m.classes {
        let extras = &extra_inherited[c.name.as_str()];
        let c_consts = &consts_of[c.name.as_str()];
        let (ref c_iter, c_iter_async) = iters_of[c.name.as_str()];
        let c_elem = &elems_of[c.name.as_str()];
        let c_iter = c_iter.as_ref().or(c_elem.as_ref());
        let refs = uses.entry(c.module.clone()).or_default();
        for f in &c.funcs {
            f.params.iter().for_each(|p| type_refs(&p.1, refs));
            type_refs(&f.ret, refs);
            if let Some((from, _)) = homes.get(&(c.name.as_str(), f.name.as_str())) {
                refs.insert(from.to_string());
            }
        }
        refs.extend(extras.iter().map(|&(_, from, _)| from.to_string()));
        refs.extend(ancestors[c.name.as_str()].iter().map(|a| a.name.clone()));
        refs.extend(c.ancestors.iter().cloned());
        c_iter.into_iter().chain(c_elem).for_each(|t| type_refs(t, refs));
        if c_iter_async {
            refs.extend(["JsAsyncIterator".to_string(), "Promise".to_string()]);
        }
        if let Some(map) = m.maps.iter().find(|k| k.name == c.name) {
            map.values.iter().for_each(|v| type_refs(v, refs));
        }
        let (out, externs, names) = files.entry(c.module.clone()).or_default();
        names.push(c.name.clone());
        let own_names = &class_names[c.name.as_str()];
        let mut emitted: HashSet<String> = ["h", "Methods", "Key", "Type"].into_iter().map(String::from).collect();
        emitted.extend(c_consts.iter().map(|(n, _)| n.clone()));
        emitted.extend(own_names.values().cloned());

        *out += &format!("\npub const {} = extern struct {{\n    h: u32,\n", c.name);
        for (name, value) in c_consts {
            *out += &format!("    pub const {} = {value};\n", ident(name));
        }
        // Asynchronous iteration: `it.next()` is a promise of the next value, or of a null handle at the end.
        if c_iter_async && !c.funcs.iter().any(|f| f.name == "iterateAsync") {
            emitted.insert("iterateAsync".into());
            *out += &format!("    pub fn iterateAsync(self: {}) JsAsyncIterator {{ return JsAsyncIterator.over(self.h); }}\n", c.name);
        }
        // Iteration: `var it = list.iterate(); while (it.next()) |n| ...`.
        if let Some(elem) = c_iter
            && !c.funcs.iter().any(|f| f.name == "iterate")
        {
            emitted.insert("iterate".into());
            *out += &format!("    pub fn iterate(self: {}) JsIteratorOf({}) {{ return JsIteratorOf({1}).over(self.h); }}\n", c.name, typed(elem, false));
        }
        // A map's keys, and the type each selects, known at compile time.
        if let Some(map) = m.maps.iter().find(|k| k.name == c.name) {
            let keys: Vec<String> = map.keys.iter().map(|k| ident(k)).collect();
            *out += &format!("    pub const Key = enum(i32) {{ {} }};\n", keys.join(", "));
            let arms: Vec<String> = keys.iter().zip(&map.values).map(|(k, v)| format!(".{k} => {}", typed(v, false))).collect();
            *out += &format!("    pub fn Type(comptime key: Key) type {{ return switch (key) {{ {} }}; }}\n", arms.join(", "));
        }
        let mut seen_as: Vec<String> = Vec::new();
        for a in ancestors[c.name.as_str()].iter().map(|a| a.name.clone()).chain(c.ancestors.iter().cloned()) {
            if a != c.name && !seen_as.contains(&a) {
                seen_as.push(a.clone());
                let as_name = format!("as{a}");
                emitted.insert(as_name.clone());
                *out += &format!("    pub fn {}(self: {}) {a} {{ return .{{ .h = self.h }}; }}\n", ident(&as_name), c.name);
            }
        }
        // Checked downcast: only a class has a constructor to test against.
        if c.js.is_some() {
            let cast_name = if own_names.values().any(|n| n == "cast") || c.consts.iter().any(|(n, _)| n == "cast") { "tryCast" } else { "cast" };
            emitted.insert(cast_name.into());
            emitted_imports.insert(format!("js_is__{}", c.name));
            *externs += &format!("    extern \"{wasm_module}\" fn @\"js_is__{}\"(u32) i32;\n", c.name);
            *out += &format!("    pub fn {}(v: anytype) ?{} {{ return if (v.h != 0 and raw.@\"js_is__{1}\"(v.h) != 0) .{{ .h = v.h }} else null; }}\n", ident(cast_name), c.name);
        }
        let inherited = |f: &Func| if Class::has_self(f) { homes.get(&(c.name.as_str(), f.name.as_str())).copied() } else { None };
        let shared = |f: &Func| shares.contains(c.name.as_str()) && Class::has_self(f) && inherited(f).is_none();
        if c.funcs.iter().any(&shared) {
            *out += "    pub fn Methods(comptime Self: type) type {\n        return struct {\n";
            for f in c.funcs.iter().filter(|f| shared(f)) {
                func(out, externs, Some("Self"), f, &own_names[&f.name], "            ", &m.unions, wasm_module, &mut emitted_imports);
            }
            *out += "        };\n    }\n";
        }
        for f in &c.funcs {
            let own_name = &own_names[&f.name];
            match inherited(f) {
                Some((from, name)) => *out += &format!("    pub const {} = {from}.Methods(@This()).{};\n", ident(own_name), ident(&class_names[from][name])),
                None if shared(f) => *out += &format!("    pub const {0} = Methods(@This()).{0};\n", ident(own_name)),
                None => func(out, externs, Some(&c.name), f, own_name, "    ", &m.unions, wasm_module, &mut emitted_imports),
            }
        }
        for &(zname, from, home_zname) in extras {
            emitted.insert(zname.to_string());
            *out += &format!("    pub const {} = {from}.Methods(@This()).{};\n", ident(zname), ident(home_zname));
        }
        if let Some(elem) = c_elem {
            let elem_ty = typed(elem, false);
            if emitted.insert("asArray".into()) {
                *out += &format!("    pub fn asArray(self: @This()) JsArrayOf({elem_ty}) {{ return .{{ .h = self.h }}; }}\n");
            }
            if emitted.insert("getLength".into()) {
                *out += &format!("    pub fn getLength(self: @This()) f64 {{ return @floatFromInt(JsArrayOf({elem_ty}).len(.{{ .h = self.h }})); }}\n");
            }
            if emitted.insert("getAt".into()) {
                *out += &format!("    pub fn getAt(self: @This(), index_: f64) ?{elem_ty} {{ if (index_ < 0) return null; return JsArrayOf(?{elem_ty}).get(.{{ .h = self.h }}, @intFromFloat(index_)); }}\n");
            }
        }
        if c.funcs.iter().any(|f| matches!(f.call, Call::Create(_))) {
            *out += "    pub const _has_field_init = true;\n";
            let init_name = if emitted.contains("init") { "fromFields" } else { "init" };
            if emitted.insert(init_name.into()) {
                *out += &format!("    pub fn {init_name}(fields: anytype) @This() {{ return Lent.init(@This(), fields); }}\n");
            }
        }
        if emitted.insert("deinit".into()) {
            *out += "    pub fn deinit(self: @This()) void { jsRelease(self); }\n";
        }
        if emitted.insert("retain".into()) {
            *out += "    pub fn retain(self: @This()) @This() { return jsRetain(self); }\n";
        }
        if emitted.insert("as".into()) {
            *out += "    pub fn as(self: @This(), comptime Target_: type) Target_ { return .{ .h = self.h }; }\n";
        }
        *out += "};\n";
    }
    let top_names = names_of(m.funcs.iter());
    for f in &m.funcs {
        let refs = uses.entry(f.module.clone()).or_default();
        f.params.iter().for_each(|p| type_refs(&p.1, refs));
        type_refs(&f.ret, refs);
        let (out, externs, names) = files.entry(f.module.clone()).or_default();
        let zig_name = &top_names[&f.name];
        names.push(zig_name.clone());
        func(out, externs, None, f, zig_name, "", &m.unions, wasm_module, &mut emitted_imports);
    }

    // Which file declares each name.
    let home: HashMap<&str, &str> = files.iter().flat_map(|(file, (_, _, names))| names.iter().map(move |n| (n.as_str(), file.as_str()))).collect();
    let mut out = Vec::new();
    let mut root = format!("// Generated. Every name, from the file that declares it.\n// package: {pkg_name}\n");
    // A namespace's types and functions: their path, and where each is declared.
    let mut namespaces: Vec<(Vec<String>, String)> = Vec::new();
    // A namespace's function by its own name: `geometry.distance`.
    let func_paths: HashMap<&String, Vec<String>> = m
        .funcs
        .iter()
        .filter_map(|f| m.func_namespaces.get(&f.import).map(|(path, own, _, _)| (&top_names[&f.name], path.iter().cloned().chain([method_name(own)]).collect())))
        .collect();
    for (file, (body, externs, names)) in &files {
        let own: HashSet<&str> = names.iter().map(String::as_str).collect();
        // The runtime's names, and the types the file's declarations name.
        let wanted = RUNTIME_NAMES.iter().copied().chain(uses.get(file).into_iter().flatten().map(String::as_str));
        let used: BTreeSet<&str> = wanted.filter(|n| !own.contains(n) && home.contains_key(n)).collect();
        // Paths are relative to the importing file: `areas/css.zig` imports `../types.zig`.
        let up = "../".repeat(file.matches('/').count());
        let imports: String = used.iter().map(|n| format!("const {n} = @import(\"{up}{}.zig\").{n};\n", home[n])).collect();
        let body = if file == "runtime" { body.clone() } else { format!("// Generated.\n{imports}\n{body}") };
        out.push((format!("{file}.zig"), format!("{body}\nconst raw = struct {{\n{externs}}};\n")));
        for n in names {
            // A namespace's type is its namespace's, not the root's.
            match m.namespaced.get(n).or_else(|| func_paths.get(n)) {
                Some(path) => namespaces.push((path.clone(), format!("@import(\"{file}.zig\").{}", ident(n)))),
                None => root += &format!("pub const {} = @import(\"{file}.zig\").{};\n", ident(n), ident(n)),
            }
        }
    }
    root += &namespace_structs(&namespaces, &root);
    out.push(("bindings.zig".into(), root));
    let dep_imports = if used_refs.is_empty() {
        "    _ = mod;\n".to_string()
    } else {
        used_refs
            .iter()
            .map(|&idx| {
                format!(
                    "    mod.addImport(\"{}\", b.createModule(.{{\n        .root_source_file = b.path(\"{}/bindings.zig\"),\n    }}));\n",
                    refs[idx].pkg_name, refs[idx].dep_path
                )
            })
            .collect()
    };
    let build_zig = format!(
        "// Generated.\nconst std = @import(\"std\");\n\npub fn build(b: *std.Build) void {{\n    const mod = b.addModule(\"{pkg_name}\", .{{\n        .root_source_file = b.path(\"bindings.zig\"),\n    }});\n{dep_imports}}}\n"
    );
    out.push(("build.zig".into(), build_zig));
    let mut meta_types: Vec<String> = m
        .enums
        .iter()
        .map(|(n, _)| n.as_str())
        .chain(m.num_enums.iter().map(|(n, _)| n.as_str()))
        .chain(m.unions.iter().map(|u| u.name.as_str()))
        .chain(m.callbacks.iter().map(|c| c.name.as_str()))
        .chain(m.classes.iter().map(|c| c.name.as_str()))
        .chain(resolved_opaque.keys().map(String::as_str))
        .chain(iterates_async.then_some("JsAsyncIterator"))
        .filter(|n| !m.namespaced.contains_key(*n) && !func_paths.contains_key(&n.to_string()))
        .filter(|n| !matches!(*n, "Promise" | "Error" | "Date") && super::model::buffer_spec(n).is_none())
        .map(str::to_string)
        .collect();
    meta_types.sort();
    meta_types.dedup();
    let meta = super::pkg_meta::PkgMeta {
        target: "zig".into(),
        package: pkg_name.to_string(),
        entry: Some("bindings.zig".into()),
        types: meta_types,
        ..Default::default()
    };
    out.push((super::pkg_meta::META_FILE.into(), meta.to_toml()));
    Ok((out, emitted_imports))
}

// A namespace as a struct of its types, by their own names: `Stream`'s
// `PipeOptions` is `stream.PipeOptions`. Each refers to its definition
// through its file, never by a bare name another scope could also mean.
fn namespace_structs(types: &[(Vec<String>, String)], root: &str) -> String {
    let taken: HashSet<&str> = root.lines().filter_map(|l| l.strip_prefix("pub const ")?.split(' ').next()).collect();
    fn level(types: &[(Vec<String>, String)], at: &[String], depth: usize, taken: &HashSet<&str>) -> String {
        let indent = "    ".repeat(depth);
        let mut out = String::new();
        // Its own, once each: a function and a type of one name (`function Point`
        // beside `interface Point`) are apart.
        let mut own: HashSet<String> = HashSet::new();
        for (path, target) in types.iter().filter(|(p, _)| p.len() == at.len() + 1 && p[..at.len()] == *at) {
            let base = path.last().unwrap().clone();
            let mut name = base.clone();
            let mut i = 2;
            while !own.insert(name.clone()) {
                name = format!("{base}_{i}");
                i += 1;
            }
            out += &format!("{indent}pub const {} = {target};\n", ident(&name));
        }
        let children: BTreeSet<&String> = types.iter().filter(|(p, _)| p.len() > at.len() + 1 && p[..at.len()] == *at).map(|(p, _)| &p[at.len()]).collect();
        for child in children {
            // Apart from what its scope has: the root's (leaflet's function
            // `icon`), a namespace's own (`a.b()` beside `a.b.c()`), or another
            // namespace of the same name but its case (leaflet's `Control` and
            // `control`).
            let base = snake_case(child);
            let free = |n: &str| !own.contains(n) && !(depth == 0 && taken.contains(ident(n).as_str()));
            let mut name = if free(&base) { base.clone() } else { format!("{base}_ns") };
            let mut i = 2;
            while !free(&name) {
                name = format!("{base}_ns{i}");
                i += 1;
            }
            own.insert(name.clone());
            let name = ident(&name);
            let mut path = at.to_vec();
            path.push(child.clone());
            out += &format!("{indent}pub const {name} = struct {{\n{}{indent}}};\n", level(types, &path, depth + 1, taken));
        }
        out
    }
    level(types, &[], 0, &taken)
}

// `NodeJS` -> `node_js`, `Stream` -> `stream`: a namespace's name.
fn snake_case(name: &str) -> String {
    super::case::snake_case_after_digit(name)
}

// The model's types `t` names: what Zig code of it refers to.
fn type_refs(t: &Ty, out: &mut BTreeSet<String>) {
    match t {
        Ty::Obj(n) | Ty::Enum(n) | Ty::NumEnum(n) | Ty::NamedUnion(n) | Ty::Key(n) | Ty::Mapped(n) => {
            out.insert(n.clone());
        }
        Ty::Generic(n, args) => {
            out.insert(n.clone());
            args.iter().for_each(|a| type_refs(a, out));
        }
        Ty::Nullable(inner, _) | Ty::Listener(inner) | Ty::ArrayOf(inner) => type_refs(inner, out),
        Ty::Union(parts) | Ty::Tuple(parts) => parts.iter().for_each(|p| type_refs(p, out)),
        Ty::Void | Ty::Num | Ty::Bool | Ty::Str | Ty::Any | Ty::Array | Ty::Param(_) => {}
    }
}

// A buffer element as a Zig type.
fn zig_elem(e: super::model::Elem) -> &'static str {
    use super::model::Elem::*;
    match e {
        I8 => "i8",
        U8 => "u8",
        I16 => "i16",
        U16 => "u16",
        I32 => "i32",
        U32 => "u32",
        I64 => "i64",
        U64 => "u64",
        F32 => "f32",
        F64 => "f64",
    }
}
