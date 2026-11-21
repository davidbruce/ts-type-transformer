use std::collections::{HashMap, HashSet};

use roxmltree::Node;

use super::super::{attr, elements, flag, marks, named, only_child, value_name};
use super::super::js::access as js_access;
use super::{Gen, T};
use super::member_ir::{FsMember, FsMemberKind, FsParam, PropAccessor, has_member_conflict, push_member};
use super::syntax::{
    add_opt, collect_type_vars, expand, find_decl, ident, is_absent_null, map_properties, only_child_type, strip_opt, subst_params, type_ident,
    type_path, words,
};

impl Gen {
    // Notes the type parameters of a signature that range over a map's keys.
    pub(super) fn enter_keyed(&mut self, n: Node) {
        self.keyed.clear();
        for p in named(n, "typeParameter") {
            let map = elements(p).find(|c| c.has_tag_name("constraint")).map(only_child).filter(|k| k.has_tag_name("keyof")).map(only_child).filter(|r| r.has_tag_name("ref"));
            if let Some(map) = map {
                let Some(name) = map.attribute("to").map(str::to_string) else { continue };
                self.maps.insert(name.clone());
                self.keyed.insert(attr(p, "name").to_string(), name.split('.').map(type_ident).collect::<Vec<_>>().join("."));
            }
        }
    }

    // For each map whose keys are used: a module with one typed key per property.
    pub(super) fn key_modules(&mut self, root: Node) {
        for map in std::mem::take(&mut self.maps) {
            let Some(decl) = find_decl(root, &map) else {
                self.skip("map declared elsewhere, no keys");
                continue;
            };
            let map_type = map.split('.').map(type_ident).collect::<Vec<_>>().join(".");
            // By its whole path: two packages' `WriteStreamEventMap` are two.
            let module = ident(&format!("{}Keys", map.split('.').collect::<String>()));
            self.out += &format!("/// The keys of {map}.\nmodule {module} =\n");
            let mut seen = HashSet::new();
            for (key, ty) in map_properties(root, decl) {
                if seen.insert(key.clone()) {
                    let t = self.ty(ty, "");
                    self.out += &format!("    let {}: TypedKey<{map_type}, {t}> = TypedKey {key:?}\n", ident(&key));
                }
            }
            self.out += "\n";
        }
    }

    // A method's or function's type parameters, with the constraints F# can
    // state: `<'E when 'E :> Element>`, checked where it is called. An
    // interface's are left unconstrained: F# would check them wherever the
    // interface is named, erased unions and `obj` arguments included.
    pub(super) fn constrained_type_params_with_names(&mut self, n: Node) -> (String, Vec<String>) {
        let mut params = Vec::new();
        let mut constraints = Vec::new();
        let mut constrained_names = Vec::new();
        for p in named(n, "typeParameter") {
            let name = format!("'{}", attr(p, "name"));
            let mut is_constrained = false;
            let c = elements(p).find(|c| c.has_tag_name("constraint")).map(only_child);
            if let Some(c) = c.filter(|c| c.has_tag_name("ref") && matches!(attr(*c, "kind"), "interface" | "class")) {
                let t = self.ty(c, "");
                if t != "obj" {
                    constraints.push(format!("{name} :> {t}"));
                    is_constrained = true;
                }
            }
            // `K extends keyof M`: 'K is the value the key selects. Where the
            // signature passes `M[K]` to a constrained interface, 'K has that constraint.
            for r in n.descendants().filter(|r| r.has_tag_name("ref")) {
                let Some(kept) = r.attribute("to").and_then(|t| self.constrained.get(t)) else { continue };
                for (i, arg) in elements(r).enumerate() {
                    let index = elements(arg).nth(1).filter(|_| arg.has_tag_name("indexedAccess"));
                    if let (Some(Some(c)), Some(k)) = (kept.get(i), index)
                        && k.attribute("name") == p.attribute("name")
                    {
                        let bound = format!("{name} :> {}", type_path(c));
                        if !constraints.contains(&bound) {
                            constraints.push(bound);
                        }
                        is_constrained = true;
                    }
                }
            }
            if is_constrained {
                constrained_names.push(name.clone());
            }
            params.push(name);
        }
        let formatted = match (params.is_empty(), constraints.is_empty()) {
            (true, _) => String::new(),
            (false, true) => format!("<{}>", params.join(", ")),
            // `X<Y> >`: F# would read `>>` as an operator.
            (false, false) => format!("<{} when {}{}>", params.join(", "), constraints.join(" and "), if constraints.last().unwrap().ends_with('>') { " " } else { "" }),
        };
        (formatted, constrained_names)
    }

    pub(super) fn type_params(&self, n: Node) -> String {
        let params: Vec<String> = named(n, "typeParameter").map(|p| format!("'{}", attr(p, "name"))).collect();
        if params.is_empty() { String::new() } else { format!("<{}>", params.join(", ")) }
    }

    // `a: float * ?b: string -> Shape`, or the same as arguments of a static member.
    pub(super) fn params(&mut self, n: Node, owner: &str) -> Vec<FsParam> {
        self.enter_keyed(n);
        let mut out = Vec::new();
        let has_optional = named(n, "parameter").any(|p| flag(p, "optional"));
        for p in named(n, "parameter") {
            // An argument with one possible value is written into the call instead.
            if p.has_attribute("const") {
                continue;
            }
            let t = only_child(p);
            let name = match attr(p, "name") {
                "_" | "" => "arg".to_string(),
                n => ident(n),
            };
            // F# wants optional arguments last and a parameter array last: not
            // both. A method is two overloads (see `members`); elsewhere the rest is left out.
            if flag(p, "rest") && has_optional && self.split_rest != Some(true) {
                if self.split_rest.is_none() {
                    self.skip("rest parameter after optional ones");
                }
                continue;
            }
            let optional = flag(p, "optional") && self.split_rest != Some(true);
            // A key of a map: typed by the value it selects.
            if t.has_tag_name("ref") && attr(t, "kind") == "typeParameter" && let Some(map) = self.keyed.get(attr(t, "name")) {
                out.push(FsParam {
                    name,
                    ty: format!("TypedKey<{map}, '{}>", attr(t, "name")),
                    optional,
                    param_array: false,
                });
                continue;
            }
            if flag(p, "rest") {
                let rest_array = t.has_tag_name("array")
                    || (t.has_tag_name("ref")
                        && t.attribute("to").is_none()
                        && matches!(attr(t, "name"), "Array" | "ReadonlyArray")
                        && elements(t).next().is_some());
                if rest_array {
                    out.push(FsParam {
                        name,
                        ty: format!("{}[]", self.ty(only_child(t), owner)),
                        optional: false,
                        param_array: true,
                    });
                } else if t.has_tag_name("tuple")
                    && !has_optional
                    && named(t, "element").next().is_some()
                    && !named(t, "element").any(|e| flag(e, "rest"))
                {
                    let mut tuple_opt = false;
                    for (ei, elem) in named(t, "element").enumerate() {
                        tuple_opt |= flag(elem, "optional");
                        let ename = match elem.attribute("name").filter(|s| !s.is_empty() && *s != "_") {
                            Some(s) => ident(s),
                            None => format!("arg{}", ei + 1),
                        };
                        let et = self.ty(only_child(elem), owner);
                        let ty = if tuple_opt { strip_opt(&et).to_string() } else { et };
                        out.push(FsParam {
                            name: ename,
                            ty,
                            optional: tuple_opt,
                            param_array: false,
                        });
                    }
                } else {
                    out.push(FsParam {
                        name,
                        ty: "obj[]".into(),
                        optional: false,
                        param_array: true,
                    });
                }
            } else {
                let raw_t = self.ty(t, owner);
                let ty = if optional { strip_opt(&raw_t).to_string() } else { raw_t };
                out.push(FsParam { name, ty, optional, param_array: false });
            }
        }
        out
    }

    // Extra parameter lists produced by expanding small union parameters into
    // their arm types (always required, so 0-argument calls still unambiguously
    // hit the original optional overload). Never partially expands: if any union
    // parameter would remain unexpanded, returns empty (otherwise F# treats
    // 1-op_Implicit and 2-op_Implicit overloads as equally ambiguous, FS0041).
    pub(super) fn union_param_expansions(&mut self, n: Node, owner: &str, base_params: &[FsParam]) -> Vec<Vec<FsParam>> {
        const MAX_OVERLOADS: usize = 9;
        let root = n.document().root_element();
        let has_optional = named(n, "parameter").any(|p| flag(p, "optional"));
        let mut req_choices: Vec<(usize, Vec<FsParam>)> = Vec::new();
        let mut opt_choice: Option<(usize, Vec<FsParam>)> = None;
        let mut idx = 0usize;
        let mut seen_optional = false;
        for p in named(n, "parameter") {
            if p.has_attribute("const") {
                continue;
            }
            if flag(p, "rest") && has_optional && self.split_rest != Some(true) {
                continue;
            }
            let cur = idx;
            idx += 1;
            let optional = flag(p, "optional") && self.split_rest != Some(true);
            let is_first_optional = optional && !seen_optional;
            if optional {
                seen_optional = true;
            }
            if cur >= base_params.len() {
                continue;
            }
            let mut t = only_child(p);
            if t.has_tag_name("nullable") && attr(t, "absent") == "null" {
                t = only_child(t);
            }
            let union_node = if t.has_tag_name("union") {
                Some(t)
            } else if t.has_tag_name("ref") && attr(t, "kind") == "typeAlias" && elements(t).next().is_none() {
                t.attribute("to")
                    .and_then(|to| find_decl(root, to))
                    .filter(|d| d.has_tag_name("typeAlias") && named(*d, "typeParameter").next().is_none())
                    .map(only_child_type)
                    .filter(|b| b.has_tag_name("union"))
            } else {
                None
            };
            let Some(u) = union_node else { continue };
            // Leaving any union parameter unexpanded would cause FS0041 whenever
            // another parameter is expanded.
            if flag(p, "rest") || (optional && !is_first_optional) {
                return Vec::new();
            }
            let members: Vec<Node> = elements(u).collect();
            if members.len() < 2 || members.len() > 6 {
                return Vec::new();
            }
            let pname = match attr(p, "name") {
                "_" | "" => "arg".to_string(),
                s => ident(s),
            };
            let mut arms = Vec::new();
            let mut seen = HashSet::new();
            for m in members {
                let arm_ty = self.ty(m, owner);
                // Do not expand `obj`, `unit`, or nested erased `U2<..>`..`U9<..>` arms.
                let is_erased_u = arm_ty.starts_with('U')
                    && arm_ty.as_bytes().get(1).is_some_and(u8::is_ascii_digit)
                    && arm_ty.as_bytes().get(2) == Some(&b'<');
                if matches!(arm_ty.as_str(), "obj" | "unit") || is_erased_u {
                    return Vec::new();
                }
                // Skip if arm references a later file.
                if words(&arm_ty).iter().filter_map(|w| self.file_of.get(*w)).any(|f| *f > self.current_file) {
                    return Vec::new();
                }
                let norm = expand(&self.through_aliases(m), &self.abbrevs);
                if seen.insert(norm) {
                    // Always required so omitting an optional arg hits the original union overload.
                    arms.push(FsParam {
                        name: pname.clone(),
                        ty: arm_ty,
                        optional: false,
                        param_array: false,
                    });
                }
            }
            if arms.len() < 2 {
                return Vec::new();
            }
            if optional {
                opt_choice = Some((cur, arms));
            } else {
                req_choices.push((cur, arms));
            }
        }
        if req_choices.is_empty() && opt_choice.is_none() {
            return Vec::new();
        }
        let expand_over = |seed: Vec<FsParam>, choices: &[(usize, Vec<FsParam>)]| -> Vec<Vec<FsParam>> {
            let mut rows = vec![seed];
            for (pos, arms) in choices {
                let mut next = Vec::with_capacity(rows.len() * arms.len());
                for row in &rows {
                    for arm in arms {
                        let mut copy = row.clone();
                        copy[*pos] = arm.clone();
                        next.push(copy);
                    }
                }
                rows = next;
            }
            rows
        };
        let req_prod: usize = req_choices.iter().try_fold(1usize, |acc, (_, a)| acc.checked_mul(a.len())).unwrap_or(usize::MAX);
        match opt_choice {
            None => {
                if req_prod > MAX_OVERLOADS {
                    return Vec::new();
                }
                expand_over(base_params.to_vec(), &req_choices)
            }
            Some((opt_pos, opt_arms)) => {
                let full_prod = req_prod.saturating_mul(opt_arms.len());
                if req_choices.is_empty() {
                    if full_prod > MAX_OVERLOADS {
                        return Vec::new();
                    }
                    expand_over(base_params.to_vec(), &[(opt_pos, opt_arms)])
                } else {
                    if req_prod.saturating_add(full_prod) > MAX_OVERLOADS {
                        return Vec::new();
                    }
                    // Prefix overloads (omitting the optional union and later optionals)
                    // plus full overloads (passing both required and optional union arms).
                    let mut out = expand_over(base_params[..opt_pos].to_vec(), &req_choices);
                    req_choices.push((opt_pos, opt_arms));
                    out.extend(expand_over(base_params.to_vec(), &req_choices));
                    out
                }
            }
        }
    }

    // For a signature with fixed arguments: a name suffix made of their values,
    // and the argument list of the call with them written in. `first` is the
    // placeholder of the first real argument.
    pub(super) fn fixed(&mut self, n: Node, first: usize) -> Option<(String, String)> {
        if !named(n, "parameter").any(|p| p.has_attribute("const")) {
            return None;
        }
        let mut suffix = Vec::new();
        let mut args = Vec::new();
        let mut next = first;
        for p in named(n, "parameter") {
            if let Some(value) = p.attribute("const") {
                suffix.push(value_name(value));
                args.push(if only_child(p).attribute("name") == Some("string") { format!("{value:?}") } else { value.to_string() });
            } else if flag(p, "optional") || flag(p, "rest") {
                // Whatever is left is passed through as given.
                args.push(format!("${next}..."));
                break;
            } else {
                args.push(format!("${next}"));
                next += 1;
            }
        }
        Some((suffix.join("_"), args.join(", ")))
    }

    pub(super) fn returns(&mut self, n: Node, owner: &str) -> String {
        self.enter_keyed(n);
        named(n, "returns").next().map(|r| self.out_ty(only_child(r), owner)).unwrap_or("unit".into())
    }

    // The members of one side of a type, as `abstract` lines. A member that
    // mentions a type from a later file cannot be abstract (F# files cannot
    // refer forward): it becomes an extension of `extend` in that file.
    pub(super) fn members(&mut self, members: &[Node], owner: &str, extend: &str, indent: &str) -> Vec<FsMember> {
        let mut list: Vec<FsMember> = Vec::new();
        let mut extras: Vec<FsMember> = Vec::new();
        let getters: HashSet<&str> = members.iter().filter(|m| m.has_tag_name("get") || m.has_tag_name("property")).map(|m| attr(*m, "name")).collect();
        let setters: HashSet<&str> = members.iter().filter(|m| m.has_tag_name("set")).map(|m| attr(*m, "name")).collect();
        for &m in members {
            let raw = m.attribute("name").unwrap_or_default();
            // `[Symbol.iterator]` makes the type a seq: see `interface`.
            if raw == "[Symbol.iterator]" || (m.has_tag_name("set") && getters.contains(raw)) {
                continue;
            }
            let fn_prop = m.has_tag_name("property") && !flag(m, "optional") && only_child(m).has_tag_name("functionType");
            let sig_node = if fn_prop { Some(only_child(m)) } else if m.has_tag_name("method") { Some(m) } else { None };
            let pairs: Vec<(FsMember, String, Vec<FsMember>)> = if raw == "[Symbol.asyncIterator]" {
                // `x.asyncIterator ()`, whose `next ()` is a promise of the next value.
                let elem = named(m, "returns").next().and_then(|r| elements(r).next()).and_then(|it| elements(it).next());
                let t = elem.map_or("obj".to_string(), |e| self.ty(e, owner));
                let emit = "[<Emit(\"$0[Symbol.asyncIterator]()\")>]";
                vec![(
                    FsMember {
                        indent: indent.to_string(),
                        attrs: vec![emit.into()],
                        name: "asyncIterator".into(),
                        kind: FsMemberKind::AbstractMethod { tparams: String::new(), params: Vec::new(), ret: format!("JsAsyncIterator<{t}>") },
                        marks: marks(m),
                    },
                    format!("{emit} member this.asyncIterator(): JsAsyncIterator<{t}> = jsNative"),
                    Vec::new(),
                )]
            } else if raw == "[Symbol.dispose]" {
                let clash = members.iter().any(|x| x.attribute("name") == Some("dispose")) || self.inherited_methods.contains("dispose");
                let dname = if clash { "symbolDispose" } else { "dispose" };
                let emit = "[<Emit(\"$0[Symbol.dispose]()\")>]";
                vec![(
                    FsMember {
                        indent: indent.to_string(),
                        attrs: vec![emit.into()],
                        name: dname.into(),
                        kind: FsMemberKind::AbstractMethod { tparams: String::new(), params: Vec::new(), ret: "unit".into() },
                        marks: marks(m),
                    },
                    format!("{emit} member this.{dname}(): unit = jsNative"),
                    Vec::new(),
                )]
            } else if raw == "[Symbol.asyncDispose]" {
                let clash = members.iter().any(|x| x.attribute("name") == Some("asyncDispose")) || self.inherited_methods.contains("asyncDispose");
                let dname = if clash { "symbolAsyncDispose" } else { "asyncDispose" };
                let emit = "[<Emit(\"$0[Symbol.asyncDispose]()\")>]";
                vec![(
                    FsMember {
                        indent: indent.to_string(),
                        attrs: vec![emit.into()],
                        name: dname.into(),
                        kind: FsMemberKind::AbstractMethod { tparams: String::new(), params: Vec::new(), ret: "JS.Promise<unit>".into() },
                        marks: marks(m),
                    },
                    format!("{emit} member this.{dname}(): JS.Promise<unit> = jsNative"),
                    Vec::new(),
                )]
            } else if raw.starts_with('[') {
                self.skip("symbol-named member");
                Vec::new()
            } else if sig_node.is_some_and(|s| named(s, "parameter").any(|p| flag(p, "rest")) && named(s, "parameter").any(|p| flag(p, "optional"))) {
                // `f(a, b?, ...rest)`: `f(a, ?b)` and `f(a, b, [<ParamArray>] rest)`.
                let mut both = Vec::new();
                for with_rest in [false, true] {
                    self.split_rest = Some(with_rest);
                    both.extend(self.member(m, owner, setters.contains(raw), indent));
                }
                self.split_rest = None;
                both
            } else {
                self.member(m, owner, setters.contains(raw), indent)
            };
            for (cand, ext, exp) in pairs {
                let rendered = cand.render_unindented();
                // The latest file a type in the member is declared in.
                let latest = words(&rendered).iter().filter_map(|w| self.file_of.get(*w)).copied().max().unwrap_or(0);
                if !self.hoisting && latest > self.current_file && indent.is_empty() {
                    if !ext.is_empty() {
                        let lines = self.extensions.entry(latest).or_default().entry(extend.to_string()).or_default();
                        let ext = format!("        {ext}\n");
                        if !lines.contains(&ext) {
                            lines.push(ext);
                        }
                    }
                    continue;
                }
                // Distinct TypeScript overloads can map to the same F# signature.
                push_member(&mut list, cand, &self.abbrevs);
                extras.extend(exp);
            }
        }
        for e in extras {
            if !has_member_conflict(&list, &e, &self.abbrevs) {
                list.push(e);
            }
        }
        list
    }

    // One member, as `FsMember`s, an extension member string, and any union-expanded overloads.
    fn member(&mut self, m: Node, owner: &str, has_setter: bool, indent: &str) -> Vec<(FsMember, String, Vec<FsMember>)> {
        let raw = m.attribute("name").unwrap_or_default();
        let name = ident(raw);
        let js = js_access(m.attribute("jsName").unwrap_or(raw));
        let m_marks = marks(m);
        let saved_constrained = self.cur_constrained.clone();
        for p in named(m, "typeParameter") {
            self.cur_constrained.remove(&format!("'{}", attr(p, "name")));
        }
        let (tps, m_constrained) = self.constrained_type_params_with_names(m);
        self.cur_constrained.extend(m_constrained);
        let property = |name: &str, t: &str, settable: bool, null_set: bool, js: &str, marks: &Vec<String>| {
            // A `unit` cannot be set (`static protoInitialize: void`).
            let settable = settable && t != "unit";
            let js_esc = js.replace('"', "\\\"");
            let get = format!("[<Emit(\"$0{js_esc}\")>] get (): {t} = jsNative");
            let set_emit = if null_set { format!("$0{js_esc} = ($1 ?? null)") } else { format!("$0{js_esc} = $1") };
            let set = format!("[<Emit(\"{set_emit}\")>] set (v: {t}) = jsNative");
            if settable && null_set {
                vec![
                    (
                        FsMember {
                            indent: indent.to_string(),
                            attrs: Vec::new(),
                            name: name.to_string(),
                            kind: FsMemberKind::Property { ty: t.to_string(), accessor: PropAccessor::Get },
                            marks: marks.clone(),
                        },
                        format!("member this.{name} with {get} and {set}"),
                        Vec::new(),
                    ),
                    (
                        FsMember {
                            indent: indent.to_string(),
                            attrs: vec![format!("[<Emit(\"{set_emit}\")>]")],
                            name: name.to_string(),
                            kind: FsMemberKind::Property { ty: t.to_string(), accessor: PropAccessor::Set },
                            marks: marks.clone(),
                        },
                        String::new(),
                        Vec::new(),
                    ),
                ]
            } else {
                let accessor = if settable { PropAccessor::GetSet } else { PropAccessor::ReadOnly };
                let ext = if settable { format!("member this.{name} with {get} and {set}") } else { format!("member this.{name} with {get}") };
                vec![(
                    FsMember {
                        indent: indent.to_string(),
                        attrs: Vec::new(),
                        name: name.to_string(),
                        kind: FsMemberKind::Property { ty: t.to_string(), accessor },
                        marks: marks.clone(),
                    },
                    ext,
                    Vec::new(),
                )]
            }
        };
        let method_from = |g: &mut Gen, sig_node: Node, initial_tps: &str, name: &str, emit: String, member_attr: Option<String>| {
            let mut params = g.params(sig_node, owner);
            let ret = g.returns(sig_node, owner);
            let raw_name = sig_node.attribute("name").unwrap_or(raw);
            let overrides_base = g.inherited_methods.contains(raw_name);
            let mut tps = initial_tps.to_string();
            if m.has_tag_name("method") && tps.is_empty() && !overrides_base && g.split_rest != Some(false) {
                let root = m.document().root_element();
                let non_const: Vec<Node> = named(sig_node, "parameter").filter(|p| !p.has_attribute("const")).collect();
                if let Some(&last_p) = non_const.last()
                    && flag(last_p, "rest")
                {
                    let pt = only_child(last_p);
                    let el = if pt.has_tag_name("array") {
                        Some(only_child(pt))
                    } else if pt.has_tag_name("ref")
                        && pt.attribute("to").is_none()
                        && matches!(attr(pt, "name"), "Array" | "ReadonlyArray")
                    {
                        elements(pt).next()
                    } else {
                        None
                    };
                    if let Some(el) = el
                        && el.has_tag_name("ref")
                        && matches!(attr(el, "kind"), "class" | "interface")
                        && let Some(to) = el.attribute("to")
                        && let Some(target_decl) = find_decl(root, to)
                        && g.is_extended(target_decl)
                    {
                        let elem_ty = g.ty(el, owner);
                        let valid = elem_ty != "obj"
                            && !elem_ty.contains('\'')
                            && !elem_ty.starts_with("ResizeArray<")
                            && words(&elem_ty).iter().filter_map(|w| g.file_of.get(*w)).all(|f| *f <= g.current_file);
                        if valid {
                            let rendered_params: Vec<String> = params.iter().map(FsParam::render).collect();
                            let used_text = format!("{owner} {}", rendered_params.join(" "));
                            let used_vars = collect_type_vars(&used_text);
                            let tp = std::iter::once("TArg".to_string())
                                .chain((1..).map(|i| format!("TArg{i}")))
                                .find(|cand| !used_vars.contains(cand))
                                .unwrap();
                            let pname = match attr(last_p, "name") {
                                "_" | "" => "arg".to_string(),
                                s => ident(s),
                            };
                            if let Some(last) = params.last_mut() {
                                *last = FsParam {
                                    name: pname,
                                    ty: format!("'{tp}[]"),
                                    optional: false,
                                    param_array: true,
                                };
                                let sp = if elem_ty.ends_with('>') { " " } else { "" };
                                tps = format!("<'{tp} when '{tp} :> {elem_ty}{sp}>");
                            }
                        }
                    }
                }
            }
            let attrs: Vec<String> = member_attr.into_iter().collect();
            let rendered_params: Vec<String> = params.iter().map(FsParam::render).collect();
            let ext = format!("[<Emit(\"{}\")>] member this.{name}{tps}({}): {ret} = jsNative", emit.replace('"', "\\\""), rendered_params.join(", "));
            let mut expanded = Vec::new();
            for ep in g.union_param_expansions(sig_node, owner, &params) {
                let first_opt = ep.iter().position(|p| p.optional);
                let has_param_array = ep.iter().any(|p| p.param_array);
                if overrides_base
                    && !has_param_array
                    && let Some(first_opt) = first_opt
                    && ep[first_opt..].iter().all(|p| p.optional)
                    && ep.len() - first_opt <= 4
                {
                    // When overriding an inherited method whose trailing optional
                    // parameter types differ (e.g. `VisualForm.strokeOnly` vs
                    // `CanvasForm.strokeOnly`), F# does not shadow the base's
                    // optional overload unless exact-arity overloads are provided.
                    for arity in first_opt..=ep.len() {
                        let prefix: Vec<FsParam> = ep[..arity].iter().map(|p| FsParam { optional: false, ..p.clone() }).collect();
                        expanded.push(FsMember {
                            indent: indent.to_string(),
                            attrs: attrs.clone(),
                            name: name.to_string(),
                            kind: FsMemberKind::AbstractMethod { tparams: tps.clone(), params: prefix, ret: ret.clone() },
                            marks: m_marks.clone(),
                        });
                    }
                } else {
                    expanded.push(FsMember {
                        indent: indent.to_string(),
                        attrs: attrs.clone(),
                        name: name.to_string(),
                        kind: FsMemberKind::AbstractMethod { tparams: tps.clone(), params: ep, ret: ret.clone() },
                        marks: m_marks.clone(),
                    });
                }
            }
            let main = FsMember {
                indent: indent.to_string(),
                attrs,
                name: name.to_string(),
                kind: FsMemberKind::AbstractMethod { tparams: tps, params, ret },
                marks: m_marks.clone(),
            };
            (main, ext, expanded)
        };
        let res: Vec<(FsMember, String, Vec<FsMember>)> = match m.tag_name().name() {
            "property" if !flag(m, "optional") && only_child(m).has_tag_name("functionType") => {
                let fn_node = only_child(m);
                for p in named(fn_node, "typeParameter") {
                    self.cur_constrained.remove(&format!("'{}", attr(p, "name")));
                }
                let (fn_tps, fn_constrained) = self.constrained_type_params_with_names(fn_node);
                self.cur_constrained.extend(fn_constrained);
                vec![match self.fixed(fn_node, 1) {
                    Some((suffix, args)) => {
                        let emit = format!("$0{js}({args})");
                        let fixed_name = ident(&format!("{raw}_{suffix}"));
                        let attr = format!("[<Emit(\"{}\")>]", emit.replace('"', "\\\""));
                        method_from(self, fn_node, &fn_tps, &fixed_name, emit, Some(attr))
                    }
                    None => method_from(self, fn_node, &fn_tps, &name, format!("$0{js}($1...)"), None),
                }]
            }
            "property" => {
                let child = only_child(m);
                let opt = flag(m, "optional");
                let t = self.out_ty(child, owner);
                let t = if opt { add_opt(t) } else { t };
                let null_set = !opt && is_absent_null(child);
                property(&name, &t, !flag(m, "readonly"), null_set, &js, &m_marks)
            }
            "get" => {
                let ret_child = named(m, "returns").next().map(only_child);
                let t = self.returns(m, owner);
                let null_set = has_setter && ret_child.is_some_and(is_absent_null);
                property(&name, &t, has_setter, null_set, &js, &m_marks)
            }
            "set" => {
                let t = named(m, "parameter").next().map(|p| self.ty(only_child(p), owner)).unwrap_or("obj".into());
                if t == "unit" {
                    Vec::new()
                } else {
                    let set = format!("[<Emit(\"$0{} = $1\")>] set (v: {t}) = jsNative", js.replace('"', "\\\""));
                    vec![(
                        FsMember {
                            indent: indent.to_string(),
                            attrs: Vec::new(),
                            name: name.clone(),
                            kind: FsMemberKind::Property { ty: t, accessor: PropAccessor::Set },
                            marks: m_marks,
                        },
                        format!("member this.{name} with {set}"),
                        Vec::new(),
                    )]
                }
            }
            "method" => vec![match self.fixed(m, 1) {
                Some((suffix, args)) => {
                    let emit = format!("$0{js}({args})");
                    let fixed_name = ident(&format!("{raw}_{suffix}"));
                    let attr = format!("[<Emit(\"{}\")>]", emit.replace('"', "\\\""));
                    method_from(self, m, &tps, &fixed_name, emit, Some(attr))
                }
                None => method_from(self, m, &tps, &name, format!("$0{js}($1...)"), None),
            }],
            "call" => vec![method_from(self, m, &tps, "Invoke", "$0($1...)".into(), Some("[<Emit(\"$0($1...)\")>]".into()))],
            "construct" | "constructor" => vec![method_from(self, m, &tps, "Create", "new $0($1...)".into(), Some("[<EmitConstructor>]".into()))],
            "index" => {
                let key = named(m, "parameter").next().map(|p| self.ty(only_child(p), owner)).unwrap_or("string".into());
                let value = self.returns(m, owner);
                let settable = !flag(m, "readonly");
                let get = format!("[<Emit(\"$0[$1]\")>] get (key: {key}): {value} = jsNative");
                let ext = if settable {
                    format!("member this.Item with {get} and [<Emit(\"$0[$1] = $2\")>] set (key: {key}) (v: {value}) = jsNative")
                } else {
                    format!("member this.Item with {get}")
                };
                vec![(
                    FsMember {
                        indent: indent.to_string(),
                        attrs: vec!["[<EmitIndexer>]".into()],
                        name: "Item".into(),
                        kind: FsMemberKind::Indexer { key, value, settable },
                        marks: m_marks,
                    },
                    ext,
                    Vec::new(),
                )]
            }
            _ => {
                self.problems.element(T, m);
                Vec::new()
            }
        };
        self.cur_constrained = saved_constrained;
        res
    }

    // The members of generic bases left to another path (which has them at
    // other arguments, `GPUDataOwner<obj>`), and of their bases, at the
    // arguments given here: F# lets an interface declare again a member it
    // inherits, at a type of its own. Those whose type is the same either way,
    // or that it declares already, it inherits.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn redeclared(
        &mut self,
        d: Node,
        left: &[Node],
        body_members: &[FsMember],
        extra_own: &[(&str, &'static str)],
        owner: &str,
        extend: &str,
        indent: &str,
    ) -> Vec<FsMember> {
        let root = d.document().root_element();
        let mut own: HashSet<(String, &'static str)> = body_members.iter().map(FsMember::redeclared_key).collect();
        for &(n, acc) in extra_own {
            own.insert((n.to_string(), acc));
        }
        let mut out = Vec::new();
        let mut todo: Vec<(Node, HashMap<String, String>)> = Vec::new();
        for r in left {
            todo.push((*r, HashMap::new()));
        }
        let mut seen = HashSet::new();
        while let Some((r, outer)) = todo.pop() {
            let Some(base) = r.attribute("to").and_then(|to| find_decl(root, to)) else { continue };
            if !seen.insert(r.attribute("to").unwrap_or_default().to_string()) {
                continue;
            }
            // Its type parameters, as the arguments given here.
            let mut map = HashMap::new();
            for (p, a) in named(base, "typeParameter").zip(elements(r)) {
                let t = self.ty(a, owner);
                map.insert(attr(p, "name").to_string(), subst_params(&t, &outer));
            }
            for e in named(base, "extends") {
                todo.push((only_child(e), map.clone()));
            }
            let src = if base.has_tag_name("typeAlias") { only_child_type(base) } else { base };
            let members: Vec<Node> = elements(src).filter(|m| matches!(m.tag_name().name(), "property" | "method" | "get" | "set") && !flag(*m, "static")).collect();
            // Rendered apart: what goes to later files' extensions, and what is skipped, is the base's.
            let saved = (self.extensions.clone(), self.skipped.clone());
            let base_members = self.members(&members, owner, extend, indent);
            (self.extensions, self.skipped) = saved;
            for bm in base_members {
                let again = bm.subst(&map);
                let key = again.redeclared_key();
                if again != bm && own.insert(key) {
                    out.push(again);
                }
            }
        }
        out
    }
}
