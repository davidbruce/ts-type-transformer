use std::collections::HashSet;

use roxmltree::Node;

use super::super::{attr, elements, flag, named, only_child};
use super::{Gen, T};
use super::syntax::{
    add_opt, erased_union, fable_builtin, find_decl, ident, is_base, is_plain_ident, only_child_type, type_ident, type_path,
};

pub(super) struct CallbackSig {
    pub invoke: String,
    pub rest: Option<(Vec<String>, String, String)>,
}

impl Gen {
    pub(super) fn ty(&mut self, n: Node, owner: &str) -> String {
        match n.tag_name().name() {
            "primitive" => match attr(n, "name") {
                "string" => "string",
                "number" => "float",
                "boolean" => "bool",
                "bigint" => "bigint",
                "symbol" => "JsSymbol",
                "void" | "undefined" => "unit",
                _ => "obj",
            }
            .into(),
            "this" => owner.to_string(),
            "ref" if attr(n, "kind") == "typeParameter" && self.fn_params.iter().any(|p| p == attr(n, "name")) => "obj".into(),
            "ref" if attr(n, "kind") == "typeParameter" => format!("'{}", attr(n, "name")),
            "ref" => {
                // Declared here (`to`), or elsewhere, though one declared here may have its name.
                let to = n.attribute("to");
                let name = to.unwrap_or_else(|| n.attribute("qualified").unwrap_or(attr(n, "name")));
                if to.is_none() && matches!(name, "Promise" | "PromiseLike") && elements(n).count() == 1 {
                    return format!("JS.Promise<{}>", self.out_ty(only_child(n), owner));
                }
                if let Some(kept) = to.and_then(|t| self.alias_kept.get(t)).cloned() {
                    let args: Vec<Node> = elements(n).collect();
                    let args: Vec<String> = kept.iter().filter_map(|&i| args.get(i)).map(|a| self.ty(*a, owner)).collect();
                    let name = name.split('.').map(type_ident).collect::<Vec<_>>().join(".");
                    return if args.is_empty() { name } else { format!("{name}<{}>", args.join(", ")) };
                }
                // Where the interface keeps a constraint, a union of subtypes is named as the constraint: F# has no union that is one.
                let kept = to.and_then(|t| self.constrained.get(t)).cloned().unwrap_or_default();
                let args: Vec<String> = elements(n)
                    .enumerate()
                    .map(|(i, a)| match kept.get(i) {
                        Some(Some(c)) if a.has_tag_name("ref") && a.attribute("kind") == Some("typeAlias") => type_path(c),
                        _ => self.ty(a, owner),
                    })
                    .collect();
                let name = if to.is_some() {
                    name.split('.').map(type_ident).collect::<Vec<_>>().join(".")
                } else {
                    match (name, args.len()) {
                        ("Array" | "ReadonlyArray", 1) => "ResizeArray".into(),
                        ("Map" | "ReadonlyMap", 2) if !is_base(n) => "JS.Map".into(),
                        ("Set" | "ReadonlySet", 1) if !is_base(n) => "JS.Set".into(),
                        ("WeakMap", 2) if !is_base(n) => "JS.WeakMap".into(),
                        ("WeakSet", 1) if !is_base(n) => "JS.WeakSet".into(),
                        // Built-ins Fable has: its types, which have their members.
                        // Their buffer argument (`Uint8Array<ArrayBufferLike>`) it does not take.
                        // Not as a base: an interface cannot inherit them (`Pt extends Float32Array`).
                        (_, n_args) if !is_base(n) && (n_args == 0 || name != "Error") && let Some(t) = fable_builtin(name) => return t,
                        // A type from another library, an alias too: resolve through a
                        // referenced binding project if one defines it, else an opaque stub.
                        _ if matches!(attr(n, "kind"), "class" | "interface" | "typeAlias") => {
                            if let Some((ref_idx, qualified)) = self.resolve_ref_type(name, args.len(), is_base(n)) {
                                if !self.probing {
                                    self.used_refs.insert(ref_idx);
                                }
                                qualified
                            } else {
                                let ext = self.external_name(name);
                                let arity = self.external.entry(ext.clone()).or_default();
                                *arity = (*arity).max(args.len());
                                ext
                            }
                        }
                        // A value from another file or a dotted enum member (`ExtensionType.WebGLSystem`).
                        _ => {
                            if let Some((enum_path, _)) = name.rsplit_once('.')
                                && let Some(decl) = find_decl(n.document().root_element(), enum_path)
                                && decl.has_tag_name("enum")
                            {
                                return type_path(enum_path);
                            }
                            self.skip("reference to an undeclared value, bound as obj");
                            return "obj".into();
                        }
                    }
                };
                if args.is_empty() { name } else { format!("{name}<{}>", args.join(", ")) }
            }
            "nullable" => {
                let inner = only_child(n);
                let t = self.ty(inner, owner);
                // Fable sends `None` as `undefined`. Where only `null` is accepted, use
                // a type that has a null of its own, if there is one.
                if attr(n, "absent") != "null" || inner.has_tag_name("functionType") || t.starts_with('\'') || t == "unit" {
                    return format!("{t} option");
                }
                match t.as_str() {
                    "float" | "bool" => format!("System.Nullable<{t}>"),
                    _ => t,
                }
            }
            // Fable's erased unions.
            "union" => {
                let mut parts: Vec<String> = Vec::new();
                for p in elements(n) {
                    let t = self.ty(p, owner);
                    if !parts.contains(&t) {
                        parts.push(t);
                    }
                }
                erased_union(parts)
            }
            "array" => format!("ResizeArray<{}>", self.ty(only_child(n), owner)),
            // `M[K]` for a key parameter: the value type the key selects.
            "indexedAccess" => {
                let index = elements(n).nth(1).unwrap();
                match index.attribute("name").filter(|k| index.attribute("kind") == Some("typeParameter") && self.keyed.contains_key(*k)) {
                    Some(k) => format!("'{k}"),
                    None => {
                        self.problems.element(T, n);
                        "obj".into()
                    }
                }
            }
            // Fable compiles tuples to arrays.
            "tuple" => {
                let parts: Vec<String> = named(n, "element").map(|e| self.ty(only_child(e), owner)).collect();
                if parts.len() < 2 || named(n, "element").any(|e| flag(e, "rest") || flag(e, "optional")) {
                    // An array of the one type its elements have (`[A]`, `[A, ...A[]]`).
                    let elems: Vec<String> = named(n, "element")
                        .map(|e| match only_child(e) {
                            t if flag(e, "rest") && t.has_tag_name("array") => self.ty(only_child(t), owner),
                            t => self.ty(t, owner),
                        })
                        .collect();
                    if let Some(first) = elems.first().filter(|f| elems.iter().all(|e| e == *f)) {
                        return format!("ResizeArray<{first}>");
                    }
                    self.skip("irregular tuple, bound as an array");
                    return "ResizeArray<obj>".into();
                }
                format!("({})", parts.join(" * "))
            }
            // A call signature is one: an interface of only that is a function.
            "functionType" | "call" => self.fn_type(n, owner),
            // An object with only properties is an anonymous record, unless it has
            // optional properties and enough fields that writing every `None` is
            // unwieldy — then it is hoisted to an interface with a ParamObject `Create`.
            "object" if elements(n).all(|m| m.has_tag_name("property")) && elements(n).next().is_some() => {
                let props: Vec<Node> = elements(n).collect();
                let has_opt = props.iter().any(|m| flag(*m, "optional"));
                let in_tagged_union = n.parent().is_some_and(|p| p.has_tag_name("union") && p.has_attribute("discriminant"))
                    || n.attribute("test").is_some_and(|t| t.starts_with("tag:") || t.starts_with("tag_has:") || t.starts_with("has:"));
                if !self.probing
                    && (in_tagged_union || (has_opt && props.len() >= 4))
                    && props.iter().all(|m| is_plain_ident(attr(*m, "name")) && attr(*m, "name") != "Create")
                    && let Some(hoisted) = self.hoist_object(n, &props, owner)
                {
                    return hoisted;
                }
                let fields: Vec<String> = props
                    .iter()
                    .map(|m| {
                        let t = self.ty(only_child(*m), owner);
                        let t = if flag(*m, "optional") { add_opt(t) } else { t };
                        format!("{}: {t}", ident(attr(*m, "name")))
                    })
                    .collect();
                format!("{{| {} |}}", fields.join("; "))
            }
            // `{}`: any object.
            "object" if elements(n).next().is_none() => "obj".into(),
            // Only an index signature: a map.
            "object" if elements(n).count() == 1 && elements(n).all(|m| m.has_tag_name("index")) => {
                let m = elements(n).next().unwrap();
                let key = named(m, "parameter").next().map(|p| self.ty(only_child(p), owner)).unwrap_or("string".into());
                format!("JsRecord<{key}, {}>", self.returns(m, owner))
            }
            // A constructor: `new` on it makes a T. Its own type parameters are `obj`.
            "constructorType" if !named(n, "parameter").any(|p| flag(p, "rest") || flag(p, "optional")) => {
                let depth = self.fn_params.len();
                self.fn_params.extend(named(n, "typeParameter").map(|p| attr(p, "name").to_string()));
                let mut parts: Vec<String> = named(n, "parameter").map(|p| self.ty(only_child(p), owner)).collect();
                let k = parts.len();
                parts.push(named(n, "returns").next().map(|r| self.ty(only_child(r), owner)).unwrap_or("obj".into()));
                self.fn_params.truncate(depth);
                self.ctor_arities.insert(k);
                format!("JsConstructor{k}<{}>", parts.join(", "))
            }
            "object" => {
                if let Some(hoisted) = self.hoist_inline_interface(n, owner) {
                    return hoisted;
                }
                self.skip("inline type with methods, bound as obj");
                "obj".into()
            }
            "constructorType" => {
                self.skip("inline type with methods, bound as obj");
                "obj".into()
            }
            _ => {
                self.problems.element(T, n);
                "obj".into()
            }
        }
    }

    // Type in an output position (return value, read-only getter, Promise element, top-level var):
    // `| null` (`absent="null"`) becomes `option` rather than bare nullable / `System.Nullable`.
    pub(super) fn out_ty(&mut self, n: Node, owner: &str) -> String {
        match n.tag_name().name() {
            "nullable" => add_opt(self.out_ty(only_child(n), owner)),
            "ref"
                if n.attribute("to").is_none()
                    && matches!(n.attribute("qualified").unwrap_or(attr(n, "name")), "Promise" | "PromiseLike")
                    && elements(n).count() == 1 =>
            {
                format!("JS.Promise<{}>", self.out_ty(only_child(n), owner))
            }
            _ => self.ty(n, owner),
        }
    }

    // A type as F# sees it through aliases not declared yet (`OrdinalNumber`
    // is a float): what an abbreviation of it would stand for.
    pub(super) fn through_aliases<'a>(&mut self, mut t: Node<'a, 'a>) -> String {
        let root = t.document().root_element();
        for _ in 0..16 {
            let alias = t.has_tag_name("ref").then(|| t.attribute("to")).flatten().filter(|_| attr(t, "kind") == "typeAlias" && elements(t).next().is_none()).and_then(|to| find_decl(root, to));
            match alias.filter(|d| named(*d, "typeParameter").next().is_none()) {
                Some(d) => t = only_child_type(d),
                None => break,
            }
        }
        let saved = (std::mem::take(&mut self.skipped), std::mem::take(&mut self.problems));
        let out = self.ty(t, "");
        (self.skipped, self.problems) = saved;
        out
    }

    pub(super) fn callback_sig(&mut self, n: Node, owner: &str) -> CallbackSig {
        let saved_keyed = self.keyed.clone();
        for p in named(n, "typeParameter") {
            let map = elements(p).find(|c| c.has_tag_name("constraint")).map(only_child).filter(|k| k.has_tag_name("keyof")).map(only_child).filter(|r| r.has_tag_name("ref"));
            if let Some(map) = map {
                let name = map.attribute("to").unwrap_or_default().to_string();
                if self.maps.contains(&name) {
                    self.keyed.insert(attr(p, "name").to_string(), ident(&name));
                }
            }
        }
        let params: Vec<Node> = named(n, "parameter").filter(|p| !p.has_attribute("const")).collect();
        let has_trailing_rest = params.last().is_some_and(|p| flag(*p, "rest"));
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        let mut fixed_types = Vec::new();
        let mut rest_elem = None;
        for (i, &p) in params.iter().enumerate() {
            let t_node = only_child(p);
            let raw = attr(p, "name");
            // `_` names no parameter in F#.
            let mut pname = if raw.is_empty() || raw == "_" { format!("arg{i}") } else { raw.to_string() };
            while !seen.insert(pname.clone()) {
                pname.push('_');
            }
            let pname = ident(&pname);
            if flag(p, "rest") {
                let elem = if t_node.has_tag_name("array") { self.ty(only_child(t_node), owner) } else { "obj".into() };
                out.push(format!("[<System.ParamArray>] {pname}: {elem}[]"));
                if has_trailing_rest && i + 1 == params.len() {
                    rest_elem = Some(elem);
                }
                continue;
            }
            let t = if t_node.has_tag_name("ref") && attr(t_node, "kind") == "typeParameter" && self.keyed.contains_key(attr(t_node, "name")) {
                format!("TypedKey<{}, '{}>", self.keyed[attr(t_node, "name")], attr(t_node, "name"))
            } else {
                self.ty(t_node, owner)
            };
            if flag(p, "optional") {
                let trailing_opt = params[i + 1..].iter().all(|q| flag(*q, "optional") && !flag(*q, "rest"));
                if trailing_opt {
                    let base = t.strip_suffix(" option").unwrap_or(&t);
                    out.push(format!("?{pname}: {base}"));
                } else {
                    let opt = if t.ends_with(" option") { t } else { format!("{t} option") };
                    if has_trailing_rest {
                        fixed_types.push(opt.clone());
                    }
                    out.push(format!("{pname}: {opt}"));
                }
            } else {
                if has_trailing_rest {
                    fixed_types.push(t.clone());
                }
                out.push(format!("{pname}: {t}"));
            }
        }
        let params_str = if out.is_empty() { "unit".to_string() } else { out.join(" * ") };
        let ret = named(n, "returns").next().map(|r| self.ty(only_child(r), owner)).unwrap_or_else(|| "unit".into());
        self.keyed = saved_keyed;
        let rest = rest_elem.map(|elem| (fixed_types, elem, ret.clone()));
        CallbackSig { invoke: format!("{params_str} -> {ret}"), rest }
    }

    // Fable compiles a ParamArray delegate to a one-parameter function, so a
    // rest-parameter callback is an interface with `Invoke` (spreading on call)
    // and `Create` (gathering `...r` into a ResizeArray in JS).
    pub(super) fn callback_def(&self, name: &str, header: &str, ret_type: &str, sig: &CallbackSig, indent: &str) -> String {
        let Some((fixed, elem, ret)) = &sig.rest else {
            return format!("{indent}type {name}{header} = delegate of {}\n\n", sig.invoke);
        };
        let mut out = format!(
            "{indent}[<AllowNullLiteral; Interface>]\n\
             {indent}type {name}{header} =\n\
             {indent}    [<Emit(\"$0($1...)\")>] abstract Invoke: {}\n",
            sig.invoke
        );
        let rest_ty = format!("ResizeArray<{elem}>");
        let create_fn = if fixed.is_empty() && !super::syntax::is_fn_type(ret, &self.abbrevs) {
            Some(format!("{rest_ty} -> {ret}"))
        } else if fixed.len() < 16 {
            let mut args = fixed.clone();
            args.push(rest_ty);
            if ret == "unit" {
                Some(format!("System.Action<{}>", args.join(", ")))
            } else {
                args.push(ret.clone());
                Some(format!("System.Func<{}>", args.join(", ")))
            }
        } else {
            None
        };
        if let Some(create_fn) = create_fn {
            let mut js_params: Vec<String> = (0..fixed.len()).map(|i| format!("p{i}")).collect();
            let mut js_args = js_params.clone();
            js_params.push("...r".into());
            js_args.push("r".into());
            let emit = format!("(({}) => ($0)({}))", js_params.join(", "), js_args.join(", "));
            out += &format!(
                "{indent}    [<Emit({emit:?})>]\n\
                 {indent}    static member Create (f: {create_fn}) : {ret_type} = jsNative\n"
            );
        }
        out.push('\n');
        out
    }

    fn is_fn_return(&self, ret: &str, ret_node: Option<Node>) -> bool {
        if super::syntax::is_fn_type(ret, &self.abbrevs) {
            return true;
        }
        let Some(mut cur) = ret_node else { return false };
        let root = cur.document().root_element();
        for _ in 0..8 {
            if cur.has_tag_name("functionType") {
                return !named(cur, "parameter").any(|p| flag(p, "optional") || flag(p, "rest"));
            }
            if cur.has_tag_name("ref")
                && let Some(to) = cur.attribute("to")
                && let Some(decl) = find_decl(root, to)
                && decl.has_tag_name("typeAlias")
            {
                cur = only_child_type(decl);
                continue;
            }
            break;
        }
        false
    }

    pub(super) fn fn_type(&mut self, n: Node, owner: &str) -> String {
        self.fn_type_inner(n, owner, false)
    }

    fn fn_type_inner(&mut self, n: Node, owner: &str, in_delegate_ret: bool) -> String {
        let own: Vec<String> = named(n, "typeParameter").map(|p| attr(p, "name").to_string()).collect();
        let depth = self.fn_params.len();
        self.fn_params.extend(own);
        // Optional arguments, or the rest of them (`...args`), as
        // JavaScript calls it: a delegate's.
        let t = if named(n, "parameter").any(|p| flag(p, "optional") || flag(p, "rest")) {
            let fn_owner = if owner == "__HOIST_SELF__" { "obj" } else { owner };
            self.function_delegate(n, fn_owner)
        } else {
            let mut param_types: Vec<String> = named(n, "parameter").map(|p| self.ty(only_child(p), owner)).collect();
            let ret_node = named(n, "returns").next().map(only_child);
            // A listener (one argument, any result): its result is
            // nothing JavaScript reads, as the model has it.
            let listener = named(n, "parameter").count() == 1 && super::super::model::listener_result(n);
            let ret = match ret_node {
                _ if listener => "unit".to_string(),
                Some(r) if r.has_tag_name("functionType") && !named(r, "parameter").any(|p| flag(p, "optional") || flag(p, "rest")) => {
                    self.fn_type_inner(r, owner, true)
                }
                Some(r) => self.ty(r, owner),
                None => "unit".into(),
            };
            // Higher-order returns (`(t) => () => R`) would become curried
            // `obj -> (unit -> R)` arrows that Fable uncurries via `uncurry2` at
            // `[<ParamObject>]` boundaries; emit explicit delegates instead.
            let delegate_ret = (!listener && self.is_fn_return(&ret, ret_node)) || (in_delegate_ret && param_types.len() >= 2);
            if delegate_ret {
                if param_types.len() > 16 {
                    let fn_owner = if owner == "__HOIST_SELF__" { "obj" } else { owner };
                    self.function_delegate(n, fn_owner)
                } else if ret == "unit" {
                    if param_types.is_empty() {
                        "System.Action".into()
                    } else {
                        let j = param_types.join(", ");
                        format!("System.Action<{j}{}>", if j.ends_with('>') { " " } else { "" })
                    }
                } else {
                    param_types.push(ret);
                    let j = param_types.join(", ");
                    format!("System.Func<{j}{}>", if j.ends_with('>') { " " } else { "" })
                }
            } else {
                if param_types.is_empty() {
                    param_types.push("unit".into());
                }
                param_types.push(ret);
                format!("({})", param_types.join(" -> "))
            }
        };
        self.fn_params.truncate(depth);
        t
    }
}
