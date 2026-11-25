use std::collections::HashSet;

use roxmltree::Node;

use super::build::{external, Builder};
use super::ops::{listener_result, mentions};
use super::{ident, Absence, Call, Class, Func, KeyMap, NamedUnion, TParam, Ty};
use super::super::{attr, elements, flag, named, only_child};

pub(super) type Sig = (Vec<(String, Ty)>, bool, Vec<(usize, String)>, Ty);

// What a declaration's functions say: its JSDoc, then where JavaScript is
// called where lowering renamed the member or fixed an argument.
pub(super) fn document(funcs: &mut [Func], d: Node) {
    let doc = crate::generators::member_doc(d);
    let renamed = d.attribute("jsName").is_some_and(|js| Some(js) != d.attribute("name"));
    for f in funcs {
        let origin = (renamed || !f.fixed.is_empty()).then(|| origin(f)).flatten().filter(|_| !named_by_its_call(f));
        f.doc = match (&doc, origin) {
            (Some(d), Some(o)) => Some(format!("{d}\n\n{o}")),
            (Some(d), None) => Some(d.clone()),
            (None, o) => o,
        };
    }
}

// A function whose name already says its call: `on_click` is `on("click", ...)`.
fn named_by_its_call(f: &Func) -> bool {
    let norm = |s: &str| s.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase();
    let name = norm(&f.name);
    let (Call::Method(js) | Call::Static(js)) = &f.call else { return false };
    !f.fixed.is_empty() && name.contains(&norm(js.rsplit("::").next().unwrap_or(js))) && f.fixed.iter().all(|x| name.contains(&norm(&x.1)))
}

// `JavaScript: setView(center, zoom)`: the call as written, with the arguments
// the glue fixes in place.
fn origin(f: &Func) -> Option<String> {
    let mut params = f.params.iter().map(|p| p.0.as_str());
    let args: Vec<String> = (0..f.params.len() + f.fixed.len())
        .map(|i| match f.fixed.iter().find(|x| x.0 == i) {
            Some(x) => x.1.clone(),
            None => format!("{}{}", if f.rest && i + 1 == f.params.len() + f.fixed.len() { "..." } else { "" }, params.next().unwrap_or_default()),
        })
        .collect();
    // Not the package it is in: `leaflet::transformation`.
    let bare = |js: &str| js.rsplit("::").next().unwrap_or(js).to_string();
    let call = match &f.call {
        Call::Method(js) => format!("{js}({})", args.join(", ")),
        Call::Static(js) => format!("{}({})", bare(js), args.join(", ")),
        Call::New(js) => format!("new {}({})", bare(js), args.join(", ")),
        Call::Get(js) | Call::GetBound(js) => format!(".{js}"),
        Call::Set(js) => format!(".{js} = {}", args.first().map_or("value", String::as_str)),
        Call::StaticGet(js) => bare(js),
        Call::StaticSet(js) => format!("{} = {}", bare(js), args.first().map_or("value", String::as_str)),
        _ => return None,
    };
    Some(format!("JavaScript: `{call}`"))
}

fn setter_name(raw_name: &str, is_fn_prop: bool) -> String {
    let id = ident(raw_name);
    if crate::generators::case::starts_with_word(raw_name, "set") {
        if is_fn_prop {
            if id == "set" { "set_fn".into() } else { format!("{id}_fn") }
        } else {
            format!("assign_{id}")
        }
    } else {
        format!("set_{id}")
    }
}

impl<'a> Builder<'a> {
    fn push_callable(
        &mut self,
        c: &mut Class,
        owner: &str,
        raw_name: &str,
        name: String,
        call: Call,
        sig: Sig,
        returns_this: bool,
    ) {
        let (mut params, rest, fixed, mut ret) = sig;
        let import = self.import(format!("{owner}_{name}"));
        let mut type_params = std::mem::take(&mut self.last_type_params);
        // A method's type parameter may not reuse its class's name in C++: it is renamed.
        for tp in type_params.iter_mut().filter(|tp| c.type_params.iter().any(|ctp| ctp.name == tp.name)) {
            let renamed = format!("{}_{}", tp.name, raw_name);
            params.iter_mut().for_each(|p| rename_param(&mut p.1, &tp.name, &renamed));
            rename_param(&mut ret, &tp.name, &renamed);
            for x in tp.default.iter_mut().chain(tp.constraint.iter_mut()) {
                rename_param(x, &tp.name, &renamed);
            }
            tp.name = renamed;
        }
        c.funcs.push(Func { type_params, module: String::new(), import, name, call, params, rest, fixed, shared: None, ret, returns_this, doc: None });
    }

    fn resolve_fn_prop(
        &mut self,
        ty_node: Node<'a, 'a>,
        owner: &str,
        depth: usize,
    ) -> Option<(Node<'a, 'a>, String, Vec<(Vec<TParam>, Vec<Ty>)>, bool)> {
        if depth > 8 {
            return None;
        }
        let no_this = |f: Node| !named(f, "thisParameter").any(|_| true);
        match ty_node.tag_name().name() {
            "functionType" if no_this(ty_node) => Some((ty_node, owner.to_string(), Vec::new(), true)),
            "ref" => {
                let key = self.ref_key(ty_node);
                let decl = *self.decls.get(&key)?;
                if !flag(decl, "synthetic") {
                    return None;
                }
                if !self.generics && named(decl, "typeParameter").next().is_some() {
                    return None;
                }
                if decl.has_tag_name("typeAlias") {
                    if self.resolving.contains(&key) {
                        return None;
                    }
                    let body = self.alias_body(decl);
                    let sig_kind = if body.has_tag_name("functionType") && no_this(body) {
                        Some(body)
                    } else if body.has_tag_name("object") {
                        let members: Vec<Node> = elements(body).collect();
                        if members.len() == 1 && members[0].has_tag_name("call") && no_this(members[0]) {
                            Some(members[0])
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    if sig_kind.is_none() && !body.has_tag_name("ref") {
                        return None;
                    }
                    let tparams: Vec<TParam> = named(decl, "typeParameter").map(|p| self.tparam(p)).collect();
                    let targs: Vec<Ty> = elements(ty_node).map(|a| self.ty(a, owner)).collect();
                    if self.bad.is_some() {
                        return None;
                    }
                    if let Some(sig_node) = sig_kind {
                        return Some((sig_node, owner.to_string(), vec![(tparams, targs)], false));
                    }
                    self.resolving.push(key);
                    let res = self.resolve_fn_prop(body, owner, depth + 1);
                    self.resolving.pop();
                    res.map(|(sig, sig_owner, mut layers, _)| {
                        for (_, inner_args) in &mut layers {
                            for arg in inner_args.iter_mut() {
                                *arg = super::ops::subst(arg, &tparams, &targs);
                            }
                        }
                        layers.push((tparams, targs));
                        (sig, sig_owner, layers, false)
                    })
                } else if decl.has_tag_name("interface") {
                    let members: Vec<Node> = elements(decl)
                        .filter(|c| !c.has_tag_name("typeParameter"))
                        .collect();
                    if members.len() != 1 || !members[0].has_tag_name("call") || !no_this(members[0]) {
                        return None;
                    }
                    let tparams: Vec<TParam> = named(decl, "typeParameter").map(|p| self.tparam(p)).collect();
                    let targs: Vec<Ty> = elements(ty_node).map(|a| self.ty(a, owner)).collect();
                    if self.bad.is_some() {
                        return None;
                    }
                    let sig_owner = self.type_ident(&key);
                    Some((members[0], sig_owner, vec![(tparams, targs)], false))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    // Whether a type is a function that knows no `this` of its own: one
    // written so, an alias of one, or a type of only call signatures.
    fn function_value(&self, t: Node) -> bool {
        let t = if t.has_tag_name("nullable") { only_child(t) } else { t };
        let no_this = |f: Node| !named(f, "thisParameter").any(|_| true);
        match t.tag_name().name() {
            "functionType" => no_this(t),
            "object" => elements(t).next().is_some() && elements(t).all(|m| m.has_tag_name("call") && no_this(m)),
            "ref" => match self.decls.get(&self.ref_key(t)) {
                Some(d) if d.has_tag_name("typeAlias") => elements(*d).find(|c| !c.has_tag_name("typeParameter")).is_some_and(|b| b.has_tag_name("functionType") && no_this(b) || b.has_tag_name("object") && elements(b).next().is_some() && elements(b).all(|m| m.has_tag_name("call") && no_this(m))),
                Some(d) if d.has_tag_name("interface") => elements(*d).filter(|m| !m.has_tag_name("typeParameter")).all(|m| m.has_tag_name("call") && no_this(m)) && named(*d, "call").next().is_some(),
                _ => false,
            },
            _ => false,
        }
    }

    pub(super) fn class(&mut self, d: Node<'a, 'a>, path: &str) -> Class {
        self.class_at(d, path, path)
    }

    // A class named for `path`, its constructor and statics reached at `js`.
    pub(super) fn class_at(&mut self, d: Node<'a, 'a>, path: &str, js: &str) -> Class {
        let runtime = d.has_tag_name("class").then(|| js.to_string());
        let path_js = js;
        let owner = self.type_ident(path);
        let mut c = Class { type_params: vec![], bases: vec![], module: self.module.clone(), name: owner.clone(), ancestors: Vec::new(), funcs: Vec::new(), consts: Vec::new(), iterates: None, iterates_async: None, elements: None, js: runtime, doc: None };
        let mut names = HashSet::new();
        if !d.has_tag_name("class") && !named(d, "call").any(|_| true) {
            names.insert("create".to_string());
            let import = self.import(format!("{owner}_create"));
            // What its type fixes, optional or not: echarts' `BarSeriesOption`,
            // `type?: "bar"`, is `{ type: "bar" }`, as echarts tells series apart.
            let fixed: Vec<String> = elements(d)
                .filter(|m| m.has_tag_name("property") && !flag(*m, "static"))
                .filter_map(|m| {
                    let value = m.attribute("const")?;
                    let key = m.attribute("jsName").unwrap_or(attr(m, "name"));
                    let quoted = only_child(m).attribute("name") == Some("string");
                    Some(format!("{key:?}: {}", if quoted { format!("{value:?}") } else { value.to_string() }))
                })
                .collect();
            let literal = format!("{{{}}}", fixed.join(", "));
            c.funcs.push(Func { type_params: vec![], module: String::new(), import, name: "create".into(), call: Call::Create(literal), params: vec![], rest: false, fixed: vec![], shared: None, ret: Ty::Obj(owner.clone()), returns_this: false, doc: None });
        }
        let is_option_bag = !d.has_tag_name("class")
            && !named(d, "call").any(|_| true)
            && !self.extended_by_class.contains(path)
            && !elements(d).any(|m| matches!(m.tag_name().name(), "method" | "construct" | "constructor") && !flag(m, "static"));
        // Its methods take their names first (a static `line` does not leave
        // the method `line_2`), and stay where they are declared.
        // What is not a member (its type parameters, its bases) comes first still.
        let members: Vec<Node<'a, 'a>> = elements(d).collect();
        let rank = |m: &Node| match m.tag_name().name() {
            "method" if !flag(*m, "static") => 1,
            "method" | "property" | "get" | "set" | "constructor" | "construct" | "call" | "index" => 2,
            _ => 0,
        };
        let mut order: Vec<(usize, &Node<'a, 'a>)> = members.iter().enumerate().collect();
        order.sort_by_key(|(_, m)| rank(m));
        let mut origin = vec![0; c.funcs.len()];
        for (i, m) in order {
            let is_static = m.has_tag_name("constructor") || flag(*m, "static");
            self.member(&mut c, &mut names, *m, path_js, is_static, !is_static && is_option_bag);
            origin.resize(c.funcs.len(), i + 1);
        }
        let mut funcs: Vec<(usize, Func)> = origin.into_iter().zip(std::mem::take(&mut c.funcs)).collect();
        funcs.sort_by_key(|(i, _)| *i);
        c.funcs = funcs.into_iter().map(|(_, f)| f).collect();
        c
    }

    pub(super) fn statics(&mut self, body: Node<'a, 'a>, path: &str, name: String) -> Class {
        let mut c = Class { type_params: vec![], bases: vec![], module: self.module.clone(), name, ancestors: Vec::new(), funcs: Vec::new(), consts: Vec::new(), iterates: None, iterates_async: None, elements: None, js: None, doc: None };
        let mut names = HashSet::new();
        for m in elements(body) {
            self.member(&mut c, &mut names, m, path, true, false);
        }
        c
    }

    pub(super) fn member(&mut self, c: &mut Class, names: &mut HashSet<String>, m: Node<'a, 'a>, path: &str, is_static: bool, allow_readonly_set: bool) {
        let before = c.funcs.len();
        self.member_funcs(c, names, m, path, is_static, allow_readonly_set);
        document(&mut c.funcs[before..], m);
    }

    fn member_funcs(&mut self, c: &mut Class, names: &mut HashSet<String>, m: Node<'a, 'a>, path: &str, is_static: bool, allow_readonly_set: bool) {
        let owner = c.name.clone();
        let raw_name = m.attribute("name").unwrap_or_default();
        let tag = m.tag_name().name();
        if matches!(tag, "ancestor") {
            // A base from another file is still a type to convert to.
            let key = self.ref_key(m);
            let out = self.type_ident(&key);
            if !self.decls.contains_key(&key) {
                self.model.opaque_names.insert(out.clone(), ident(external(&key)));
                self.opaque.insert(out.clone());
            }
            return c.ancestors.push(out);
        }
        // A direct base, when inheritance is kept: members stay on the type that declares them.
        if tag == "extends" {
            let base = only_child(m);
            if base.has_tag_name("ref") {
                let key = self.ref_key(base);
                let name = key.as_str();
                // A subclass of Array is an array of its elements.
                if matches!(external(name), "Array" | "ReadonlyArray") && !self.decls.contains_key(name) {
                    if let Some(e) = elements(base).next() {
                        let elem = self.ty(e, &owner);
                        if let Ty::ArrayOf(elem) = self.array_of(elem) {
                            c.elements = Some(*elem.clone());
                            // Where generics are kept, the array is its base.
                            if self.generics {
                                c.bases.push(Ty::ArrayOf(elem));
                            }
                        }
                    }
                    return;
                }
                // Where generics are erased, a generic base is the base without its arguments.
                let out = self.type_ident(name);
                let t = if elements(base).next().is_some() && !self.generics { Ty::Obj(out.clone()) } else { self.reference(base, &owner) };
                if !self.decls.contains_key(name) {
                    self.model.opaque_names.insert(out.clone(), ident(external(name)));
                    self.opaque.insert(out.clone());
                }
                c.bases.push(t);
                return c.ancestors.push(out);
            }
            return self.skip("generic base");
        }
        if tag == "typeParameter" {
            if !self.generics {
                return self.unsupported(m);
            }
            let p = self.tparam(m);
            return c.type_params.push(p);
        }
        self.bad = None;
        let before = c.funcs.len();
        if tag == "index" && !is_static {
            let (params, _, _, value) = self.signature(m, &owner);
            let key = params.into_iter().next().map(|p| p.1).unwrap_or(Ty::Str);
            if names.insert("get_at".into()) {
                // A missing key reads as `undefined`.
                let import = self.import(format!("{owner}_get_at"));
                c.funcs.push(Func { type_params: vec![], module: String::new(), import, name: "get_at".into(), call: Call::IndexGet, params: vec![("key".into(), key.clone())], rest: false, fixed: vec![], shared: None, ret: absent(value.clone(), Absence::Undefined), returns_this: false, doc: None });
                // A value of none (`[key: number]: never`) is never set.
                if value != Ty::Void {
                    let import = self.import(format!("{owner}_set_at"));
                    c.funcs.push(Func { type_params: vec![], module: String::new(), import, name: "set_at".into(), call: Call::IndexSet, params: vec![("key".into(), key), ("value".into(), value)], rest: false, fixed: vec![], shared: None, ret: Ty::Void, returns_this: false, doc: None });
                }
            }
            return;
        }
        // An accessor: `get classList(): DOMTokenList`, `set classList(value: string)`.
        if matches!(tag, "get" | "set") && !raw_name.starts_with('[') {
            let (params, _, _, ret) = self.signature(m, &owner);
            let js = m.attribute("jsName").unwrap_or(raw_name).to_string();
            let (prefix, call, ret) = if tag == "get" {
                ("get", if is_static { Call::StaticGet(format!("{path}.{js}")) } else { Call::Get(js) }, ret)
            } else {
                ("set", if is_static { Call::StaticSet(format!("{path}.{js}")) } else { Call::Set(js) }, Ty::Void)
            };
            let base = format!("{prefix}_{}", ident(raw_name));
            if !names.contains(&base) && self.bad.is_none() {
                names.insert(base.clone());
                let import = self.import(format!("{owner}_{base}"));
                c.funcs.push(Func { type_params: vec![], module: String::new(), import, name: base, call, params, rest: false, fixed: vec![], shared: None, ret, returns_this: false, doc: None });
            }
            self.bad = None;
            return;
        }
        // An object that is also a function: `invoke(...)`, `construct(...)`.
        if tag == "call" || tag == "construct" && !is_static {
            let (params, rest, fixed, ret) = self.signature(m, &owner);
            let (base, call) = if tag == "call" { ("invoke", Call::Invoke) } else { ("construct", Call::Construct) };
            let name = if self.opts.overloading { base.to_string() } else {
                // Named for the marks it has that its siblings do not all have
                // (`invoke_async`); the first without any is `invoke`, which
                // none before it took.
                let siblings: Vec<Node> = m.parent().map(|p| elements(p).filter(|s| s.tag_name() == m.tag_name() && !flag(*s, "static")).collect()).unwrap_or_default();
                let own = super::super::own_marks(&siblings.iter().map(|s| super::super::marks(*s)).collect::<Vec<_>>());
                let me = siblings.iter().position(|s| *s == m).unwrap_or(0);
                let plain = own.iter().position(Vec::is_empty).unwrap_or(0);
                let base = if me == plain || own[me].is_empty() { base.to_string() } else { format!("{base}_{}", own[me].join("_")) };
                let mut name = base.clone();
                let mut n = 2;
                while !names.insert(name.clone()) {
                    name = format!("{base}_{n}");
                    n += 1;
                }
                name
            };
            let import = self.import(format!("{owner}_{name}"));
            let type_params = std::mem::take(&mut self.last_type_params);
            c.funcs.push(Func { type_params, module: String::new(), import, name, call, params, rest, fixed, shared: None, ret, returns_this: false, doc: None });
            if let Some(why) = self.bad.take() {
                c.funcs.pop();
                self.skip(&why);
            }
            return;
        }
        if matches!(tag, "index" | "get" | "set") {
            return self.skip(&format!("<{tag}> member"));
        }
        // `[Symbol.iterator](): ArrayIterator<Node>` iterates Nodes.
        if raw_name == "[Symbol.iterator]" && !is_static {
            let elem = named(m, "returns").next().and_then(|r| elements(r).next()).and_then(|it| elements(it).next());
            c.iterates = Some(match elem {
                Some(e) => self.ty(e, &owner),
                None => Ty::Any,
            });
            self.bad = None;
            return;
        }
        if raw_name == "[Symbol.asyncIterator]" && !is_static {
            let elem = named(m, "returns").next().and_then(|r| elements(r).next()).and_then(|it| elements(it).next());
            c.iterates_async = Some(match elem {
                Some(e) => self.ty(e, &owner),
                None => Ty::Any,
            });
            self.bad = None;
            return;
        }
        // A member keyed by a well-known symbol is named by it: `[Symbol.toStringTag]` is `toStringTag`.
        let symbol = raw_name.strip_prefix("[Symbol.").and_then(|s| s.strip_suffix(']'));
        if raw_name.starts_with('[') && symbol.is_none() {
            return self.skip("member with a computed name");
        }
        let (raw_name, key) = (symbol.unwrap_or(raw_name), raw_name);
        // A target with overloading keeps one name for all of a method's overloads.
        let overloading = self.opts.overloading;
        let unique = |base: String, names: &mut HashSet<String>| {
            if overloading {
                return base;
            }
            let mut name = base.clone();
            let mut n = 2;
            while !names.insert(name.clone()) {
                name = format!("{base}_{n}");
                n += 1;
            }
            name
        };
        let js = m.attribute("jsName").unwrap_or(key).to_string();
        match tag {
            "constructor" | "construct" => {
                let (params, rest, fixed, sig_ret) = self.signature(m, &owner);
                let ret = if tag == "construct" && sig_ret != Ty::Void { sig_ret } else { Ty::Obj(owner.clone()) };
                let name = unique(if raw_name.is_empty() { "new".into() } else { ident(raw_name) }, names);
                let import = self.import(format!("{owner}_{name}"));
                // Its own type parameters (`new<T>(x: T)`), as a method's.
                let type_params = std::mem::take(&mut self.last_type_params);
                c.funcs.push(Func { type_params, module: String::new(), import, name, call: Call::New(path.to_string()), params, rest, fixed, shared: None, ret, returns_this: false, doc: None });
            }
            "method" => {
                let sig = self.signature(m, &owner);
                let returns_this = !is_static && named(m, "returns").next().is_some_and(|r| only_child(r).has_tag_name("this"));
                let call = if is_static { Call::Static(format!("{path}.{js}")) } else { Call::Method(js) };
                let name = unique(ident(raw_name), names);
                self.push_callable(c, &owner, raw_name, name, call, sig, returns_this);
            }
            // A constant needs no call across the boundary.
            "property" if m.has_attribute("const") => {
                let quoted = only_child(m).attribute("name") == Some("string");
                let value = attr(m, "const");
                // The same constant is often on both the class and its instances.
                if !c.consts.iter().any(|(n, _)| *n == ident(raw_name)) {
                    c.consts.push((ident(raw_name), if quoted { format!("{value:?}") } else { value.to_string() }));
                }
            }
            "property" => {
                let mut bound_fn_prop = false;
                if !flag(m, "optional")
                    && let Some((sig_node, sig_owner, layers, inline)) = self.resolve_fn_prop(only_child(m), &owner, 0)
                {
                    let saved_erased = self.erased.clone();
                    if !self.generics {
                        for p in named(sig_node, "typeParameter") {
                            let is_keyof = elements(p)
                                .find(|c| c.has_tag_name("constraint"))
                                .map(only_child)
                                .is_some_and(|k| k.has_tag_name("keyof"));
                            if !is_keyof {
                                self.erased.insert(attr(p, "name").to_string());
                            }
                        }
                    }
                    let (mut params, rest, fixed, mut ret) = self.signature(sig_node, &sig_owner);
                    self.erased = saved_erased;
                    if self.bad.is_none() {
                        for (tparams, targs) in layers {
                            let active: Vec<(TParam, Ty)> = tparams
                                .into_iter()
                                .enumerate()
                                .filter(|(_, tp)| !self.last_type_params.iter().any(|ip| ip.name == tp.name))
                                .map(|(i, tp)| {
                                    let arg = targs.get(i).cloned().or_else(|| tp.default.clone()).unwrap_or(Ty::Any);
                                    (tp, arg)
                                })
                                .collect();
                            if !active.is_empty() {
                                let active_params: Vec<TParam> = active.iter().map(|(p, _)| p.clone()).collect();
                                let active_args: Vec<Ty> = active.into_iter().map(|(_, a)| a).collect();
                                for (_, pt) in &mut params {
                                    *pt = super::ops::subst(pt, &active_params, &active_args);
                                }
                                ret = super::ops::subst(&ret, &active_params, &active_args);
                                for tp in &mut self.last_type_params {
                                    tp.default = tp.default.as_ref().map(|d| super::ops::subst(d, &active_params, &active_args));
                                    tp.constraint = tp.constraint.as_ref().map(|d| super::ops::subst(d, &active_params, &active_args));
                                }
                            }
                        }
                        let returns_this = inline && !is_static && named(sig_node, "returns").next().is_some_and(|r| only_child(r).has_tag_name("this"));
                        let call = if is_static { Call::Static(format!("{path}.{js}")) } else { Call::Method(js.clone()) };
                        let fn_name = unique(ident(raw_name), names);
                        self.push_callable(c, &owner, raw_name, fn_name, call, (params, rest, fixed, ret), returns_this);
                        if !flag(m, "readonly") || allow_readonly_set {
                            let prop_ty = value(self.ty(only_child(m), &owner));
                            if self.bad.is_none() {
                                let set = if is_static { Call::StaticSet(format!("{path}.{js}")) } else { Call::Set(js.clone()) };
                                let set_name = unique(setter_name(raw_name, true), names);
                                let import = self.import(format!("{owner}_{set_name}"));
                                c.funcs.push(Func { type_params: vec![], module: String::new(), import, name: set_name, call: set, params: vec![("value".into(), prop_ty)], rest: false, fixed: vec![], shared: None, ret: Ty::Void, returns_this: false, doc: None });
                            } else {
                                self.bad = None;
                            }
                        }
                        bound_fn_prop = true;
                    } else {
                        c.funcs.truncate(before);
                        self.bad = None;
                        self.last_type_params.clear();
                    }
                } else if self.bad.is_some() {
                    c.funcs.truncate(before);
                    self.bad = None;
                }
                if !bound_fn_prop {
                    let mut t = value(self.ty(only_child(m), &owner));
                    if flag(m, "optional") {
                        t = absent(t, Absence::Undefined);
                    }
                    let is_fn_prop = self.function_value(only_child(m));
                    let (get, set) = if is_static {
                        (Call::StaticGet(format!("{path}.{js}")), Call::StaticSet(format!("{path}.{js}")))
                    } else if is_fn_prop {
                        (Call::GetBound(js.clone()), Call::Set(js))
                    } else {
                        (Call::Get(js.clone()), Call::Set(js))
                    };
                    let name = unique(format!("get_{}", ident(raw_name)), names);
                    let import = self.import(format!("{owner}_{name}"));
                    c.funcs.push(Func { type_params: vec![], module: String::new(), import, name, call: get, params: vec![], rest: false, fixed: vec![], shared: None, ret: t.clone(), returns_this: false, doc: None });
                    if !flag(m, "readonly") || allow_readonly_set {
                        let name = unique(setter_name(raw_name, is_fn_prop), names);
                        let import = self.import(format!("{owner}_{name}"));
                        c.funcs.push(Func { type_params: vec![], module: String::new(), import, name, call: set, params: vec![("value".into(), t)], rest: false, fixed: vec![], shared: None, ret: Ty::Void, returns_this: false, doc: None });
                    }
                }
            }
            _ => self.unsupported(m),
        }
        if let Some(why) = self.bad.take() {
            c.funcs.truncate(before);
            self.skip(&why);
        }
    }

    // A callback's signature: its own type parameters (`<T>(x: T) => T`) are untyped values.
    pub(super) fn callback_signature(&mut self, n: Node<'a, 'a>, owner: &str) -> Sig {
        self.erased = named(n, "typeParameter").map(|p| attr(p, "name").to_string()).collect();
        let mut sig = self.signature(n, owner);
        self.erased.clear();
        // A listener (one argument, a declared `any` result) returns nothing
        // JavaScript reads: its result is void, as its writer would have it.
        if sig.0.len() == 1 && sig.3 == Ty::Any && listener_result(n) {
            sig.3 = Ty::Void;
        }
        sig
    }

    pub(super) fn signature(&mut self, n: Node<'a, 'a>, owner: &str) -> Sig {
        // Type parameters left after `generics = "erase"` range over a map's keys.
        self.keyed.clear();
        self.key_strings.clear();
        self.last_type_params.clear();
        let own: Vec<Node> = named(n, "typeParameter").filter(|p| !self.erased.contains(attr(*p, "name"))).collect();
        for p in own {
            let map = elements(p).find(|c| c.has_tag_name("constraint")).map(only_child).filter(|k| k.has_tag_name("keyof")).map(only_child);
            match map {
                Some(m) => {
                    let name = self.ref_key(m);
                    if self.key_map(&name) {
                        self.keyed.insert(attr(p, "name").to_string(), self.type_ident(&name));
                    } else {
                        // A key of a type that is not a map of keys: a string.
                        self.key_strings.insert(attr(p, "name").to_string());
                    }
                }
                None if self.generics => {
                    let p = self.tparam(p);
                    self.last_type_params.push(p);
                }
                None => self.unsupported(p),
            }
        }
        let mut params = Vec::new();
        let mut rest = false;
        let mut fixed = Vec::new();
        let mut ret = Ty::Void;
        for p in elements(n) {
            match p.tag_name().name() {
                // The receiver of a callback is not an argument.
                "thisParameter" => {}
                // An argument with one possible value is supplied by the glue.
                "parameter" if p.has_attribute("const") => {
                    let value = attr(p, "const");
                    let literal = if only_child(p).attribute("name") == Some("string") { format!("{value:?}") } else { value.to_string() };
                    fixed.push((params.len() + fixed.len(), literal));
                }
                // An optional parameter is an optional value: null, or a null handle, when absent.
                "parameter" if flag(p, "optional") => {
                    let t = value(self.ty(only_child(p), owner));
                    // One of any type: `undefined` when absent (leaflet's `on(type, fn,
                    // context?)`), where a null handle would be `null`.
                    let optional = if t == Ty::Any { Ty::Nullable(Box::new(Ty::Any), Absence::Undefined) } else { absent(t.clone(), Absence::Undefined) };
                    if optional == Ty::Any && t != Ty::Any {
                        self.skip("optional parameter of a type that cannot be absent, bound as an untyped value");
                    }
                    params.push((ident(attr(p, "name")), optional));
                }
                "parameter" => {
                    rest |= flag(p, "rest");
                    params.push((ident(attr(p, "name")), value(self.ty(only_child(p), owner))));
                }
                "returns" => ret = self.ty(only_child(p), owner),
                "typeParameter" => {}
                _ => self.unsupported(p),
            }
        }
        (params, rest, fixed, ret)
    }

    // Registers a map whose values are all handles; false, with a reason, if it is not one.
    fn key_map(&mut self, name: &str) -> bool {
        let id = self.type_ident(name);
        if self.model.maps.iter().any(|m| m.name == id) {
            return true;
        }
        let Some(decl) = self.decls.get(name).copied() else {
            self.bad = Some("map declared elsewhere".into());
            return false;
        };
        let mut keys = Vec::new();
        let mut values = Vec::new();
        // Its own properties and those of the maps it extends, the nearest first.
        let mut props: Vec<Node> = Vec::new();
        let mut todo = vec![decl];
        let mut seen_keys = HashSet::new();
        while let Some(d) = todo.pop() {
            props.extend(named(d, "property").filter(|p| seen_keys.insert(attr(*p, "name").to_string())));
            for e in named(d, "extends") {
                let base = only_child(e);
                if let Some(b) = self.decls.get(&self.ref_key(base)) {
                    todo.insert(0, *b);
                }
            }
        }
        for p in props {
            let t = self.ty(only_child(p), "");
            if !matches!(t, Ty::Obj(_) | Ty::Generic(..)) {
                self.bad = Some("map with values that are not objects".into());
                return false;
            }
            keys.push(attr(p, "name").to_string());
            values.push(t);
        }
        self.model.maps.push(KeyMap { name: id, keys, values });
        true
    }

    pub(super) fn ty(&mut self, n: Node<'a, 'a>, owner: &str) -> Ty {
        match n.tag_name().name() {
            "primitive" => match attr(n, "name") {
                "number" => Ty::Num,
                "boolean" => Ty::Bool,
                "string" => Ty::Str,
                "void" | "undefined" | "never" => Ty::Void,
                // A handle, as F#'s `JsSymbol`: `string | symbol` stays a union (node's event names).
                "symbol" => {
                    self.model.opaque_names.insert("JsSymbol".into(), "JsSymbol".into());
                    self.opaque.insert("JsSymbol".into());
                    Ty::Obj("JsSymbol".into())
                }
                _ => Ty::Any,
            },
            "this" => Ty::Obj(owner.to_string()),
            "ref" if attr(n, "kind") == "typeParameter" && self.key_strings.contains(attr(n, "name")) => Ty::Str,
            "ref" if attr(n, "kind") == "typeParameter" && self.erased.contains(attr(n, "name")) => {
                self.skip("type parameter of a generic callback, bound as an untyped value");
                Ty::Any
            }
            "ref" if attr(n, "kind") == "typeParameter" => match self.keyed.get(attr(n, "name")) {
                Some(map) => Ty::Key(map.clone()),
                None if self.generics => Ty::Param(attr(n, "name").to_string()),
                None => {
                    self.unsupported(n);
                    Ty::Any
                }
            },
            "ref" => self.reference(n, owner),
            // `M[K]`: the value the key selects.
            // `M[K]` where K is a key of a type that is not a map of keys: whatever M holds.
            "indexedAccess" if elements(n).nth(1).is_some_and(|k| k.has_tag_name("ref") && self.key_strings.contains(attr(k, "name"))) => {
                self.skip("value of a key of a type that is not a map, bound as an untyped value");
                Ty::Any
            }
            "indexedAccess" => {
                let index = elements(n).nth(1).unwrap();
                match index.attribute("name").and_then(|k| self.keyed.get(k)) {
                    Some(map) => Ty::Mapped(map.clone()),
                    None => {
                        self.unsupported(n);
                        Ty::Any
                    }
                }
            }
            // `new () => Material`: a class itself, which only JavaScript has to give.
            "constructorType" => {
                self.skip("constructor type, bound as an untyped value");
                Ty::Any
            }
            // A callback written inline: kept so only when it takes `M[K]`.
            "functionType" if self.keyed.is_empty() => {
                self.bad = Some("callback written inline".into());
                Ty::Any
            }
            "functionType" => {
                let params: Vec<Node> = named(n, "parameter").collect();
                let ret = named(n, "returns").next().map(|r| self.ty(only_child(r), owner)).unwrap_or(Ty::Void);
                match (params.as_slice(), ret) {
                    ([p], Ty::Void | Ty::Any) => match self.ty(only_child(*p), owner) {
                        t @ (Ty::Mapped(_) | Ty::Obj(_) | Ty::Generic(..)) => Ty::Listener(Box::new(t)),
                        _ => {
                            self.bad = Some("keyed callback of another shape".into());
                            Ty::Any
                        }
                    },
                    _ => {
                        self.bad = Some("keyed callback of another shape".into());
                        Ty::Any
                    }
                }
            }
            "nullable" => {
                let inner = self.ty(only_child(n), owner);
                absent(inner, match attr(n, "absent") {
                    "undefined" => Absence::Undefined,
                    "either" => Absence::Either,
                    "false" => Absence::False,
                    _ => Absence::Null,
                })
            }
            // Elements that cross as a number, a flag, a string, a handle or a
            // union that can be told apart.
            "array" => {
                let elem = self.ty(only_child(n), owner);
                self.array_of(elem)
            }
            // A tuple whose elements all cross, none optional or rest.
            "tuple" => {
                let elems: Vec<Node> = elements(n).collect();
                if elems.is_empty() || elems.iter().any(|e| e.attributes().len() > 0) {
                    return Ty::Array;
                }
                let ts: Vec<Ty> = elems.iter().map(|e| self.ty(only_child(*e), owner)).collect();
                let crosses = |t: &Ty| match t {
                    Ty::Num | Ty::Bool | Ty::Str | Ty::Obj(_) | Ty::ArrayOf(_) | Ty::Array | Ty::Tuple(_) => true,
                    Ty::Generic(..) | Ty::Param(_) => self.generics,
                    Ty::NamedUnion(u) => self.model.unions.iter().any(|x| x.name == *u && x.decodable()),
                    _ => false,
                };
                if ts.iter().all(crosses) { Ty::Tuple(ts) } else { Ty::Array }
            }
            "union" => {
                let parts: Vec<Ty> = elements(n).map(|p| self.ty(p, owner)).collect();
                // Only alternatives a tagged union can hold directly.
                if parts.iter().all(|p| matches!(p, Ty::Num | Ty::Str | Ty::Bool | Ty::Obj(_) | Ty::Enum(_))) && distinct(&parts) {
                    Ty::Union(parts)
                } else {
                    self.skip("union whose members cannot be told apart, bound as an untyped value");
                    Ty::Union(vec![])
                }
            }
            _ => {
                self.unsupported(n);
                Ty::Any
            }
        }
    }

    // An array of the given elements: typed, if they cross as a number, a
    // flag, a string, a handle or a union that can be told apart.
    fn array_of(&mut self, elem: Ty) -> Ty {
        match elem {
            e @ (Ty::Num | Ty::Bool | Ty::Str | Ty::Obj(_) | Ty::ArrayOf(_)) => Ty::ArrayOf(Box::new(e)),
            e @ (Ty::Generic(..) | Ty::Param(_)) if self.generics => Ty::ArrayOf(Box::new(e)),
            Ty::NamedUnion(u) if self.model.unions.iter().any(|x| x.name == u && x.decodable()) => Ty::ArrayOf(Box::new(Ty::NamedUnion(u))),
            _ => Ty::Array,
        }
    }

    // `owner` is what `this` is, in a type argument (`GetSet<number, this>`).
    fn reference(&mut self, n: Node<'a, 'a>, owner: &str) -> Ty {
        let key = self.ref_key(n);
        let declared = self.decls.get(&key).copied();
        // `Array<T>` is `T[]`.
        if declared.is_none() && matches!(external(&key), "Array" | "ReadonlyArray") {
            return match elements(n).next() {
                Some(e) => {
                    let elem = self.ty(e, owner);
                    self.array_of(elem)
                }
                None => Ty::Array,
            };
        }
        match (attr(n, "kind"), declared) {
            ("typeParameter", _) => {
                self.unsupported(n);
                Ty::Any
            }
            // By what is declared: an alias of string literals has become an enum.
            (_, Some(d)) if d.has_tag_name("enum") && named(d, "member").all(|m| attr(m, "type") == "string") => Ty::Enum(self.type_ident(&key)),
            (_, Some(d)) if d.has_tag_name("enum") && named(d, "member").all(|m| attr(m, "type") == "number") => Ty::NumEnum(self.type_ident(&key)),
            ("enum", _) => Ty::Num,
            (_, Some(d)) if d.has_tag_name("typeAlias") => {
                let body = self.alias_body(d);
                if body.has_tag_name("functionType") || body.has_tag_name("object") {
                    if self.generics && elements(n).next().is_some() {
                        let args: Vec<Ty> = elements(n).map(|a| self.ty(a, owner)).collect();
                        return Ty::Generic(self.type_ident(&key), args);
                    }
                    return Ty::Obj(self.type_ident(&key));
                }
                // A generic union alias, where generics are kept, is a generic union.
                let generic_union = self.generics && body.has_tag_name("union") && named(d, "typeParameter").next().is_some();
                // Another generic alias would need its arguments substituted
                // (`aliases = "expand"` does, but for one that refers to itself).
                if named(d, "typeParameter").next().is_some() && !generic_union {
                    self.skip("generic alias, bound as any");
                    return Ty::Any;
                }
                if self.resolving.contains(&key) || self.resolving.len() > 8 {
                    return Ty::Any;
                }
                // A union with a name of its own is declared once.
                if body.has_tag_name("union") {
                    let name = self.type_ident(&key);
                    let args: Vec<Ty> = if generic_union { elements(n).map(|a| self.ty(a, owner)).collect() } else { Vec::new() };
                    // `void` in a union is a value that may be absent: `void | Promise<void>`.
                    // So is a member that may itself be absent: `string | (number | null)`.
                    let is_void = |p: Node| p.has_tag_name("primitive") && attr(p, "name") == "void";
                    let has_void = elements(body).any(|p| is_void(p) || p.has_tag_name("nullable"));
                    let named_type = |name: String, kept: Option<&Vec<usize>>| {
                        let args: Vec<Ty> = args.iter().enumerate().filter(|(i, _)| kept.is_none_or(|k| k.contains(i))).map(|(_, a)| a.clone()).collect();
                        // One left with no type parameters is no generic union.
                        let t = if generic_union && !args.is_empty() { Ty::Generic(name, args) } else { Ty::NamedUnion(name) };
                        if has_void { absent(t, Absence::Undefined) } else { t }
                    };
                    if self.model.unions.iter().any(|u| u.name == name) {
                        return named_type(name.clone(), self.union_kept.get(&name));
                    }
                    let type_params: Vec<TParam> = named(d, "typeParameter").collect::<Vec<_>>().into_iter().map(|p| self.tparam(p)).collect();
                    let members: Vec<Node> = elements(body).filter(|p| !is_void(*p)).collect();
                    self.resolving.push(key.clone());
                    // Its own type parameters, not those of where it is named
                    // (a callback's erased `T` is not this `T`).
                    let own: Vec<String> = type_params.iter().map(|p| p.name.clone()).collect();
                    let saved = (self.erased.clone(), self.keyed.clone(), self.key_strings.clone());
                    self.erased.retain(|e| !own.contains(e));
                    self.keyed.retain(|k, _| !own.contains(k));
                    self.key_strings.retain(|k| !own.contains(k));
                    let parts: Vec<Ty> = members
                        .iter()
                        .map(|p| match self.ty(*p, "") {
                            Ty::Nullable(inner, _) => *inner,
                            t => t,
                        })
                        .collect();
                    // `T | T[]` of a union `T`, flattened (ECharts' `SeriesOption |
                    // SeriesOption[]`): the array is of `T`, whose members are all here.
                    let flat = parts.clone();
                    let parts: Vec<Ty> = members
                        .iter()
                        .zip(parts)
                        .map(|(m, p)| {
                            if self.opts.typed_union_arrays && p == Ty::Array && m.has_tag_name("array")
                                && let Ty::NamedUnion(n) = self.ty(only_child(*m), "")
                                && self.model.unions.iter().find(|u| u.name == n).is_some_and(|u| u.typable() && u.parts.iter().all(|q| flat.contains(q)))
                            {
                                return Ty::ArrayOf(Box::new(Ty::NamedUnion(n)));
                            }
                            p
                        })
                        .collect();
                    (self.erased, self.keyed, self.key_strings) = saved;
                    self.resolving.pop();
                    // Members that are one type here, as two generic types are once erased, are one member.
                    let mut kept = Vec::new();
                    let (members, parts): (Vec<Node>, Vec<Ty>) = members.into_iter().zip(parts).filter(|(_, p)| if kept.contains(p) { false } else { kept.push(p.clone()); true }).unzip();
                    // `X | any` is any.
                    if parts.contains(&Ty::Any) {
                        return Ty::Any;
                    }
                    if let [only] = parts.as_slice() {
                        return if has_void { absent(only.clone(), Absence::Undefined) } else { only.clone() };
                    }
                    let simple = |p: &Ty| matches!(p, Ty::Num | Ty::Str | Ty::Bool | Ty::Obj(_) | Ty::Enum(_) | Ty::Array | Ty::ArrayOf(_) | Ty::Tuple(_)) || self.generics && matches!(p, Ty::Generic(..) | Ty::Param(_));
                    if !parts.iter().all(simple) || !distinct(&parts) {
                        if std::env::var_os("TS_UNION_DEBUG").is_some() {
                            eprintln!("UNION {name}: distinct={} {:?}", distinct(&parts), parts);
                        }
                        self.skip("union whose members cannot be told apart, bound as an untyped value");
                        return Ty::Any;
                    }
                    let tests = members.iter().map(|p| p.attribute("test").map(str::to_string)).collect();
                    let discriminant = body.attribute("discriminant").map(str::to_string);
                    // A type parameter no member uses (`Number<TNum> = NodeClass | number`)
                    // is no parameter of it: a target may not declare one unused.
                    let kept: Vec<usize> = (0..type_params.len()).filter(|&i| parts.iter().any(|p| mentions(p, &type_params[i].name))).collect();
                    let type_params: Vec<TParam> = if kept.len() < type_params.len() {
                        self.union_kept.insert(name.clone(), kept.clone());
                        kept.iter().map(|&i| type_params[i].clone()).collect()
                    } else {
                        type_params
                    };
                    self.model.unions.push(NamedUnion { name: name.clone(), type_params, parts, tests, discriminant });
                    return named_type(name.clone(), self.union_kept.get(&name));
                }
                self.resolving.push(key.clone());
                let t = self.ty(body, "");
                self.resolving.pop();
                t
            }
            ("class" | "interface", declared) if self.generics && elements(n).next().is_some() => {
                let args: Vec<Ty> = elements(n).map(|a| self.ty(a, owner)).collect();
                let out = self.type_ident(&key);
                if declared.is_none() {
                    self.model.opaque_names.insert(out.clone(), ident(external(&key)));
                    self.opaque.insert(out.clone());
                    let arity = self.model.opaque_arity.entry(out.clone()).or_default();
                    *arity = (*arity).max(args.len());
                }
                Ty::Generic(out, args)
            }
            ("class" | "interface", Some(_)) => Ty::Obj(self.type_ident(&key)),
            // Declared in another library: a handle, by name. An alias may be of anything, and a handle can hold anything.
            ("class" | "interface" | "typeAlias", None) => {
                let out = self.type_ident(&key);
                self.model.opaque_names.insert(out.clone(), ident(external(&key)));
                self.opaque.insert(out.clone());
                Ty::Obj(out)
            }
            _ => Ty::Any,
        }
    }
}

pub(super) fn is_function_interface(d: Node) -> bool {
    let mut members = elements(d);
    members.next().is_some_and(|m| m.has_tag_name("call")) && members.next().is_none()
}

// No two members alike. Arrays are alike whatever they hold: a test sees only an array.
fn distinct(parts: &[Ty]) -> bool {
    let shape = |p: &Ty| if matches!(p, Ty::ArrayOf(_) | Ty::Tuple(_)) { Ty::Array } else { p.clone() };
    let shapes: Vec<Ty> = parts.iter().map(shape).collect();
    shapes.iter().enumerate().all(|(i, p)| !shapes[..i].contains(p))
}

// `void` is only a result. A parameter or property of type `undefined` holds a value.
pub(super) fn value(t: Ty) -> Ty {
    if t == Ty::Void { Ty::Any } else { t }
}

// Renames a type parameter wherever a type mentions it.
fn rename_param(t: &mut Ty, from: &str, to: &str) {
    match t {
        Ty::Param(n) if n == from => *n = to.to_string(),
        Ty::Nullable(inner, _) | Ty::Listener(inner) | Ty::ArrayOf(inner) => rename_param(inner, from, to),
        Ty::Generic(_, args) | Ty::Tuple(args) => args.iter_mut().for_each(|a| rename_param(a, from, to)),
        _ => {}
    }
}

// What may be absent. A handle can be null; a string is a null pointer going
// in and a null handle coming back; a number is a flag and the number going
// in, and a handle coming back; a flag or an enum is -1 when absent.
fn absent(t: Ty, a: Absence) -> Ty {
    match t {
        Ty::Obj(_) | Ty::Array | Ty::ArrayOf(_) | Ty::Tuple(_) | Ty::Str | Ty::Param(_) | Ty::Generic(..) => Ty::Nullable(Box::new(t), a),
        Ty::Num | Ty::NumEnum(_) | Ty::Bool | Ty::Enum(_) | Ty::Mapped(_) | Ty::NamedUnion(_) | Ty::Listener(_) => Ty::Nullable(Box::new(t), a),
        // `x?: T | null`: omitted too, as `x?: T` is.
        Ty::Nullable(inner, u) => Ty::Nullable(inner, match (a, u) {
            (Absence::Undefined, Absence::Null | Absence::Either) => Absence::Either,
            (Absence::Undefined, _) => Absence::Undefined,
            (_, u) => u,
        }),
        Ty::Void => Ty::Void,
        _ => Ty::Any,
    }
}

impl Builder<'_> {
    // More than four optional arguments are an option bag of them, made as
    // any is (three's `CylinderGeometry(radiusTop?, .., thetaLength?)`):
    // `cylinder_geometry.new_with(cylinder_geometry_args.create() |> ..)`.
    // The function takes the bag, optional, for them; `Model::args_bags` says
    // how it spreads.
    pub(super) fn args_bags(&mut self) {
        let mut taken: HashSet<String> = self.model.classes.iter().map(|c| c.name.clone()).chain(self.model.callbacks.iter().map(|c| c.name.clone())).collect();
        let mut bags = Vec::new();
        for ci in 0..self.model.classes.len() {
            let (owner, module) = (self.model.classes[ci].name.clone(), self.model.classes[ci].module.clone());
            for fi in 0..self.model.classes[ci].funcs.len() {
                let mut f = self.model.classes[ci].funcs[fi].clone();
                if let Some(bag) = self.args_bag(&owner, &module, &mut f, &mut taken) {
                    self.model.classes[ci].funcs[fi] = f;
                    bags.push(bag);
                }
            }
        }
        for fi in 0..self.model.funcs.len() {
            let mut f = self.model.funcs[fi].clone();
            let module = f.module.clone();
            if let Some(bag) = self.args_bag("", &module, &mut f, &mut taken) {
                self.model.funcs[fi] = f;
                bags.push(bag);
            }
        }
        self.model.classes.extend(bags);
    }

    fn args_bag(&mut self, owner: &str, module: &str, f: &mut Func, taken: &mut HashSet<String>) -> Option<Class> {
        let start = super::tail_start(f);
        if f.rest || f.params.len() - start <= 4 || !matches!(f.call, Call::Method(_) | Call::Static(_) | Call::New(_)) {
            return None;
        }
        let pascal = |s: &str| s.chars().take(1).flat_map(char::to_uppercase).chain(s.chars().skip(1)).collect::<String>();
        let base = match &f.call {
            Call::New(_) => format!("{owner}Args"),
            _ => format!("{owner}{}Args", pascal(&f.name)),
        };
        let mut name = base.clone();
        let mut n = 2;
        while !taken.insert(name.clone()) {
            name = format!("{base}{n}");
            n += 1;
        }
        let func = |name: String, import: String, call: Call, params: Vec<(String, Ty)>, ret: Ty| Func { type_params: vec![], module: String::new(), import, name, call, params, rest: false, fixed: vec![], shared: None, ret, returns_this: false, doc: None };
        let mut funcs = vec![func("create".into(), self.import(format!("{name}_create")), Call::Create("{}".into()), vec![], Ty::Obj(name.clone()))];
        for (p, t) in &f.params[start..] {
            let setter = format!("set_{}", ident(p));
            funcs.push(func(setter.clone(), self.import(format!("{name}_{setter}")), Call::Set(p.clone()), vec![("value".into(), t.clone())], Ty::Void));
        }
        self.model.args_bags.insert(f.import.clone(), f.params[start..].iter().map(|(p, _)| p.clone()).collect());
        self.model.synthetic.insert(name.clone());
        f.params.truncate(start);
        f.params.push(("args".into(), Ty::Nullable(Box::new(Ty::Obj(name.clone())), Absence::Undefined)));
        Some(Class { type_params: vec![], bases: vec![], module: module.to_string(), name, ancestors: Vec::new(), funcs, consts: Vec::new(), iterates: None, iterates_async: None, elements: None, js: None, doc: None })
    }
}
