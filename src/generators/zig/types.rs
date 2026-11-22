use std::collections::{BTreeSet, HashMap, HashSet};

use super::{ident, zig_func_name};
use super::super::comments::{Style, comment};
use super::super::model::{Call, Callback, Class, Func, NamedUnion, Ty};

// Each function's Zig name, by the model's: `names::settle` made them apart.
pub(super) fn names_of<'a>(funcs: impl Iterator<Item = &'a Func>) -> HashMap<String, String> {
    funcs.map(|f| (f.name.clone(), zig_func_name(&f.name, f))).collect()
}

// Parameters get a suffix: Zig forbids shadowing a declaration in scope.
fn param(name: &str) -> String {
    format!("{name}_")
}

fn raw(t: &Ty) -> &'static str {
    use super::super::wasm::{AbiSlot::*, WasmParamAbi::*};
    match super::super::wasm::param_abi(t) {
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
pub(super) fn union_decl(u: &NamedUnion) -> String {
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

pub(super) fn typed(t: &Ty, is_param: bool) -> String {
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
pub(super) fn raw_callback(t: &Ty) -> &'static str {
    use super::super::wasm::AbiSlot::*;
    match super::super::wasm::ret_abi(t) {
        Void => "void",
        F64 => "f64",
        I32 => "i32",
        U32 => "u32",
    }
}

pub(super) fn callback_param_ty(t: &Ty, unions: &[NamedUnion]) -> String {
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
pub(super) fn callback_arg(t: &Ty, a: &str, unions: &[NamedUnion]) -> String {
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
pub(super) fn callback_result(t: &Ty, call: &str) -> String {
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

// What a parameter of `anytype` takes, in words.
fn describe(t: &Ty, unions: &[NamedUnion], callbacks: &[Callback]) -> String {
    let list = |parts: &[Ty]| {
        let shown: Vec<String> = parts.iter().take(6).map(|p| describe(p, unions, callbacks).trim_start_matches("a ").to_string()).collect();
        format!("one of {}{}", shown.join(", "), if parts.len() > 6 { ", ..." } else { "" })
    };
    match t {
        Ty::Str => "a string".into(),
        Ty::Num => "a number".into(),
        Ty::Bool => "a bool".into(),
        Ty::Any => "any value".into(),
        Ty::Array => "an array".into(),
        Ty::ArrayOf(e) => format!("an array of {}", describe(e, unions, callbacks).trim_start_matches("a ")),
        Ty::Tuple(es) => format!("a tuple of ({})", es.iter().map(|e| describe(e, unions, callbacks).trim_start_matches("a ").to_string()).collect::<Vec<_>>().join(", ")),
        Ty::Obj(n) => match callbacks.iter().find(|c| c.name == *n) {
            Some(c) => format!("a function `fn ({}) {}`", c.params.iter().map(|p| typed(&p.1, true)).collect::<Vec<_>>().join(", "), typed(&c.ret, false)),
            None => format!("`{n}`"),
        },
        Ty::Enum(n) | Ty::NumEnum(n) => format!("`{n}`"),
        Ty::NamedUnion(n) => match unions.iter().find(|u| u.name == *n) {
            Some(u) => list(&u.parts),
            None => format!("`{n}`"),
        },
        Ty::Union(parts) => list(parts),
        Ty::Listener(ev) => format!("a function taking a `{}`", typed(ev, false)),
        Ty::Mapped(m) => format!("the type `{m}` selects"),
        Ty::Nullable(inner, _) => describe(inner, unions, callbacks),
        Ty::Key(m) => format!("a `{m}.Key`"),
        Ty::Void | Ty::Param(_) | Ty::Generic(..) => String::new(),
    }
}

fn says_more_of(t: &Ty, callbacks: &[Callback]) -> bool {
    match t {
        Ty::NamedUnion(_) | Ty::Union(_) | Ty::Listener(_) | Ty::Tuple(_) => true,
        Ty::Obj(n) => callbacks.iter().any(|c| c.name == *n),
        _ => false,
    }
}

// The one line that says what the `anytype` parameters of a function take.
fn accepts(f: &Func, first_optional: usize, optional: usize, opts_param: &str, unions: &[NamedUnion], callbacks: &[Callback]) -> Option<String> {
    let mut items: Vec<String> = Vec::new();
    // What a name or a type alone does not say: a union's members, a function's signature.
    let says_more = |t: &Ty| match t {
        Ty::Nullable(inner, _) => says_more_of(inner, callbacks),
        t => says_more_of(t, callbacks),
    };
    for (i, (name, t)) in f.params.iter().enumerate().take(first_optional) {
        if *t == Ty::Num {
            items.push(format!("`{name}`: an integer or a float"));
            continue;
        }
        if !says_more(t) || (i == f.params.len() - 1 && f.rest) {
            continue;
        }
        let or_null = if matches!(t, Ty::Nullable(..)) { ", or null" } else { "" };
        items.push(format!("`{name}`: {}{or_null}", describe(t, unions, callbacks)));
    }
    if optional > 1 || optional == 1 && says_more(&f.params[first_optional].1) {
        let fields: Vec<String> = f.params[first_optional..].iter().map(|(n, t)| format!(".{n}: {}", describe(t, unions, callbacks))).collect();
        items.push(if optional == 1 { format!("`{opts_param}`: the optional `{}`", fields[0].trim_start_matches('.')) } else { format!("`{opts_param}`: null or a struct literal of {}", fields.join(", ")) });
    }
    (!items.is_empty()).then(|| format!("Takes {}.", items.join("; ")))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn func(out: &mut String, externs: &mut String, owner: Option<&str>, f: &Func, zig_name: &str, indent: &str, unions: &[NamedUnion], callbacks: &[Callback], wasm_module: &str, emitted_imports: &mut HashSet<String>) {
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
    let rest_opt = !setter
        && f.rest
        && matches!(f.params.last().map(|p| &p.1), Some(Ty::Array | Ty::ArrayOf(_) | Ty::Tuple(_)));
    let optional = if setter {
        0
    } else {
        let before_rest = if rest_opt { f.params.len() - 1 } else { f.params.len() };
        let opt_before = f.params[..before_rest].iter().rev().take_while(|p| matches!(p.1, Ty::Nullable(_, true))).count();
        opt_before + usize::from(rest_opt)
    };
    // A lone parameter stays one: `orUnit(s)`, `orUnit(null)`.
    let optional = if optional == f.params.len() && optional == 1 { 0 } else { optional };
    let first_optional = f.params.len() - optional;
    let opts_param = if f.params.iter().any(|(n, _)| param(n) == "opts_") { "ts_opts_" } else { "opts_" };
    let mut allowed = Vec::new();
    let mut unpack = String::new();
    let mut pre = String::new();
    for (i, (name, t)) in f.params.iter().enumerate() {
        let id = param(name);
        let is_rest_param = rest_opt && i == f.params.len() - 1;
        let effective_t = if is_rest_param && !matches!(t, Ty::Nullable(..)) {
            Ty::Nullable(Box::new(t.clone()), true)
        } else {
            t.clone()
        };
        if i >= first_optional {
            let inner = match t {
                Ty::Nullable(inner, _) => inner.as_ref(),
                other => other,
            };
            if optional == 1 {
                let inner_ty = typed(inner, true);
                let inner_ty = if key.is_empty() { inner_ty } else { inner_ty.replace("KEY", &key) };
                let unwrap = format!("Lent.singleOpt({inner_ty}, {name:?}, {opts_param})");
                if is_lent_ty(&effective_t) {
                    pre += &format!("const lent_{id} = {}; defer lent_{id}.done(); ", lent_expr(&unwrap, &effective_t, &key));
                } else if *inner == Ty::Str {
                    unpack += &format!("const str_{id} = Lent.optStr({unwrap}); ");
                } else {
                    let val_expr = match inner {
                        Ty::Num => format!("Lent.optNum({unwrap})"),
                        Ty::Bool => format!("Lent.optBool({unwrap})"),
                        Ty::Enum(n) | Ty::NumEnum(n) => format!("Lent.optEnum({n}, {unwrap})"),
                        _ => unreachable!(),
                    };
                    unpack += &format!("const {id} = {val_expr}; ");
                }
            } else {
                allowed.push(format!("{name:?}"));
                let field_expr = format!("@field({opts_param}, {name:?})");
                let cond = format!("comptime Lent.hasOpt(@TypeOf({opts_param}), {name:?})");
                if is_lent_ty(&effective_t) {
                    pre += &format!("const lent_{id}: Lent = if ({cond}) {} else Lent.none; defer lent_{id}.done(); ", lent_expr(&field_expr, &effective_t, &key));
                } else if *inner == Ty::Str {
                    unpack += &format!("const str_{id}: Lent.OptStrArg = if ({cond}) Lent.optStr({field_expr}) else .{{ .ptr = null, .len = 0 }}; ");
                } else {
                    let line = match inner {
                        Ty::Num => format!("const {id}: ?f64 = if ({cond}) Lent.optNum({field_expr}) else null; "),
                        Ty::Bool => format!("const {id}: ?bool = if ({cond}) Lent.optBool({field_expr}) else null; "),
                        Ty::Enum(n) | Ty::NumEnum(n) => format!("const {id}: ?{n} = if ({cond}) Lent.optEnum({n}, {field_expr}) else null; "),
                        _ => unreachable!(),
                    };
                    unpack += &line;
                }
            }
        } else if matches!(t, Ty::Key(_)) {
            params.push(format!("comptime {id}: {}", typed(t, true).replace("KEY", &key)));
        } else if *t == Ty::Num {
            params.push(format!("{id}: anytype"));
        } else if *t == Ty::Str {
            params.push(format!("{id}: anytype"));
            unpack += &format!("const str_{id} = Lent.str({id}); ");
        } else if matches!(t, Ty::Nullable(inner, _) if **inner == Ty::Str) {
            params.push(format!("{id}: anytype"));
            unpack += &format!("const str_{id} = Lent.optStr({id}); ");
        } else if is_lent_ty(&effective_t) {
            params.push(format!("{id}: anytype"));
            pre += &format!("const lent_{id} = {}; defer lent_{id}.done(); ", lent_expr(&id, &effective_t, &key));
        } else {
            params.push(format!("{id}: {}", typed(t, true).replace("KEY", &key)));
        }
        if *t == Ty::Str {
            raw_params.extend(["[*]const u8".to_string(), "usize".to_string()]);
            args.extend([format!("str_{id}.ptr"), format!("str_{id}.len")]);
        } else if matches!(t, Ty::Nullable(inner, _) if matches!(**inner, Ty::Num | Ty::NumEnum(_))) {
            // An optional number: whether it is there, and the number.
            let value = if matches!(t, Ty::Nullable(inner, _) if **inner == Ty::Num) { format!("{id} orelse 0") } else { format!("if ({id}) |v| @floatFromInt(@intFromEnum(v)) else 0") };
            raw_params.extend(["i32".to_string(), "f64".to_string()]);
            args.extend([format!("@intFromBool({id} != null)"), value]);
        } else if matches!(t, Ty::Nullable(inner, _) if **inner == Ty::Str) {
            raw_params.extend(["?[*]const u8".to_string(), "usize".to_string()]);
            args.extend([format!("str_{id}.ptr"), format!("str_{id}.len")]);
        } else if *t == Ty::Num {
            raw_params.push("f64".to_string());
            args.push(format!("Lent.num({id}, {name:?})"));
        } else {
            raw_params.push(raw(t).to_string());
            args.push(lower_arg(&id, &effective_t));
        }
    }
    if optional > 1 {
        unpack = format!("Lent.checkOpts({opts_param}, &.{{ {} }}); {unpack}", allowed.join(", "));
    }
    if optional > 0 {
        params.push(format!("{opts_param}: anytype"));
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
    let takes = accepts(f, first_optional, optional, opts_param, unions, callbacks);
    let docs: Vec<&str> = f.doc.as_deref().into_iter().chain(takes.as_deref()).collect();
    if !docs.is_empty() {
        *out += &comment(&docs.join("\n\n"), "///", indent, Style::Plain);
    }
    *out += &format!("{indent}pub fn {}({}) {ret_type} {{ {unpack}{pre}{body} }}\n", ident(zig_name), params.join(", "));
}

// The model's types `t` names: what Zig code of it refers to.
pub(super) fn type_refs(t: &Ty, out: &mut BTreeSet<String>) {
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
pub(super) fn zig_elem(e: super::super::model::Elem) -> &'static str {
    use super::super::model::Elem::*;
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
