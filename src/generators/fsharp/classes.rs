use std::collections::{HashMap, HashSet};

use roxmltree::Node;

use super::super::{attr, elements, flag, marks, named, only_child};
use super::Gen;
use super::member_ir::{FsMember, FsMemberKind, FsParam, has_member_conflict, push_member, render_members};
use super::syntax::{
    collect_type_vars, const_js, fable_builtin, find_decl, ident, is_plain_ident, only_child_type, pascal, strip_opt, subst_params, type_ident,
    type_path, words,
};

pub(super) enum FactoryResult {
    None,
    Inline { args: String, emit: String },
    Extension { latest: usize, args: String, emit: String },
}

impl FactoryResult {
    fn is_some(&self) -> bool {
        !matches!(self, Self::None)
    }
}

impl Gen {
    fn inheritance_depth(&self, id: &str) -> usize {
        fn go(bases: &HashMap<String, Vec<String>>, cur: &str, seen: &mut HashSet<String>) -> usize {
            if !seen.insert(cur.to_string()) {
                return 0;
            }
            let d = bases.get(cur).map_or(0, |bs| {
                bs.iter()
                    .filter(|b| bases.contains_key(b.as_str()))
                    .map(|b| 1 + go(bases, b, seen))
                    .max()
                    .unwrap_or(0)
            });
            seen.remove(cur);
            d
        }
        go(&self.bases, id, &mut HashSet::new())
    }

    fn is_arg_subtype(&self, root: Node, decl: Node, arg: Node, default: Node, depth: usize) -> bool {
        if depth > 8 {
            return false;
        }
        if default.has_tag_name("primitive") && matches!(default.attribute("name"), Some("any" | "unknown")) {
            return true;
        }
        if arg.has_tag_name("union") {
            return elements(arg).next().is_some()
                && elements(arg).all(|arm| self.is_arg_subtype(root, decl, arm, default, depth + 1));
        }
        if arg.has_tag_name("ref") && arg.attribute("kind") == Some("typeParameter") {
            let tname = attr(arg, "name");
            let Some(tp_decl) = named(decl, "typeParameter").find(|p| attr(*p, "name") == tname) else {
                return false;
            };
            let Some(constraint) = named(tp_decl, "constraint").next() else {
                return false;
            };
            return self.is_arg_subtype(root, decl, only_child(constraint), default, depth + 1);
        }
        let Some(ato) = arg.attribute("to") else { return false };
        let Some(dto) = default.attribute("to") else { return false };
        ato == dto || self.reaches(ato, dto)
    }

    fn can_widen_to_defaults(&self, root: Node, decl: Node, gdecl: Node, actual: &[Node]) -> bool {
        let gparams: Vec<Node> = named(gdecl, "typeParameter").collect();
        if gparams.is_empty()
            || gparams.len() != actual.len()
            || !gparams.iter().all(|p| named(*p, "default").next().is_some())
        {
            return false;
        }
        gparams.iter().enumerate().all(|(i, p)| {
            let def = only_child(named(*p, "default").next().unwrap());
            self.is_arg_subtype(root, decl, actual[i], def, 0)
        })
    }

    fn widen_candidates<'a>(&mut self, root: Node<'a, 'a>, decl: Node<'a, 'a>, owner: &str, max_file: usize) -> Vec<(String, String)> {
        let mut out: Vec<(String, String)> = Vec::new();
        let mut seen_decls: HashSet<String> = HashSet::new();
        let mut seen_methods: HashSet<String> = HashSet::new();
        let start_id = decl.attribute("id").unwrap_or(attr(decl, "name")).to_string();
        seen_decls.insert(start_id);

        let mut queue: std::collections::VecDeque<(Node<'a, 'a>, HashMap<String, Node<'a, 'a>>, bool)> =
            std::collections::VecDeque::from([(decl, HashMap::new(), true)]);

        while let Some((cur, subst, is_direct)) = queue.pop_front() {
            if cur.has_tag_name("typeAlias") {
                continue;
            }
            for e in named(cur, "extends") {
                let target = only_child(e);
                let Some(to) = target.attribute("to") else { continue };
                let Some(gdecl) = find_decl(root, to) else { continue };

                let raw_actual: Vec<Node<'a, 'a>> = elements(target).collect();
                let actual: Vec<Node<'a, 'a>> = raw_actual
                    .iter()
                    .map(|a| {
                        if a.has_tag_name("ref") && a.attribute("kind") == Some("typeParameter") {
                            subst.get(attr(*a, "name")).copied().unwrap_or(*a)
                        } else {
                            *a
                        }
                    })
                    .collect();

                if !actual.is_empty() && self.can_widen_to_defaults(root, decl, gdecl, &actual) {
                    let gparams: Vec<Node> = named(gdecl, "typeParameter").collect();
                    let def_tys: Vec<String> = gparams
                        .iter()
                        .map(|p| self.ty(only_child(named(*p, "default").next().unwrap()), owner))
                        .collect();
                    let base_name_ty = type_path(to);
                    let def_base = format!("{base_name_ty}<{}>", def_tys.join(", "));
                    let same_direct = is_direct && def_base == self.ty(target, owner);
                    if !same_direct
                        && collect_type_vars(&def_base).is_empty()
                        && words(&def_base).iter().filter_map(|w| self.file_of.get(*w)).all(|f| *f <= max_file)
                    {
                        let mname = format!("as{}", pascal(attr(gdecl, "name")));
                        if seen_methods.insert(mname.clone()) {
                            out.push((mname, def_base));
                        }
                    }
                }

                let gid = gdecl.attribute("id").unwrap_or(to).to_string();
                if seen_decls.insert(gid) {
                    let next_subst: HashMap<String, Node<'a, 'a>> = named(gdecl, "typeParameter")
                        .zip(actual.iter())
                        .map(|(gp, a)| (attr(gp, "name").to_string(), *a))
                        .collect();
                    queue.push_back((gdecl, next_subst, false));
                }
            }
        }
        out
    }

    pub(super) fn interface(&mut self, d: Node, prefix: &str, indent: &str) {
        let name = type_ident(attr(d, "name"));
        // Reached in JavaScript by what it is exported as: `default`.
        let js_name = d.attribute("jsName").unwrap_or(attr(d, "name"));
        let path = super::syntax::js_path(&self.js_prefix, js_name);
        let params = self.type_params(d);
        let owner = format!("{name}{params}");
        // The declaration, and its extensions, state the constraints it keeps.
        let saved_constrained = self.cur_constrained.clone();
        let header = match d.attribute("id").and_then(|id| self.constrained.get(id)) {
            Some(cs) => {
                let ps: Vec<String> = named(d, "typeParameter").map(|p| format!("'{}", attr(p, "name"))).collect();
                for (p, c) in ps.iter().zip(cs) {
                    if c.is_some() {
                        self.cur_constrained.insert(p.clone());
                    }
                }
                // The constraint as F# names it: a built-in Fable has is its type (`JS.Date`).
                let when: Vec<String> = ps.iter().zip(cs).filter_map(|(p, c)| c.as_ref().map(|c| format!("{p} :> {}", fable_builtin(c).unwrap_or_else(|| type_path(c))))).collect();
                format!("{name}<{} when {} >", ps.join(", "), when.join(" and "))
            }
            None => owner.clone(),
        };
        let is_member = |m: &Node| matches!(m.tag_name().name(), "property" | "method" | "call" | "construct" | "index" | "get" | "set");
        // An alias of an object type has its members in the object.
        let src = if d.has_tag_name("typeAlias") { only_child_type(d) } else { d };
        let instance: Vec<Node> = elements(src).filter(|m| is_member(m) && !flag(*m, "static")).collect();
        let statics: Vec<Node> = elements(src).filter(|m| m.has_tag_name("constructor") || is_member(m) && flag(*m, "static")).collect();
        // A class of an imported library that unions tell apart: Fable tests it with instanceof.
        let origin = match self.tested_classes.get(d.attribute("id").unwrap_or(attr(d, "name"))) {
            Some(module) => format!("{indent}[<Import({js_name:?}, {module:?})>]\n"),
            _ => String::new(),
        };
        let extend = if prefix.is_empty() { owner.clone() } else { format!("{}.{owner}", prefix.split('.').map(type_ident).collect::<Vec<_>>().join(".")) };
        if header != owner {
            let qualified = if prefix.is_empty() { header.clone() } else { format!("{}.{header}", prefix.split('.').map(type_ident).collect::<Vec<_>>().join(".")) };
            self.constrained_owner.insert(extend.clone(), qualified);
        }
        let decl_id = d.attribute("id").unwrap_or(attr(d, "name"));
        let is_base = self.extended_bases.contains(decl_id);
        // A static member makes F# take it for a class unless told.
        let factory = if d.has_tag_name("class") { FactoryResult::None } else { self.factory(d, &owner, &extend, is_base) };
        let kind = if factory.is_some() { "AllowNullLiteral; Interface" } else { "AllowNullLiteral" };
        self.out += &format!("{origin}{indent}[<{kind}>]\n{indent}type {header} =\n");
        let mut prefix_body = Vec::new();
        // A subclass of Array is a seq with an indexer and a length: an F#
        // interface cannot inherit ResizeArray, which is a class.
        let mut is_array = false;
        let direct: Vec<Option<&str>> = if d.has_tag_name("typeAlias") { Vec::new() } else { named(d, "extends").map(|e| only_child(e).attribute("to")).collect() };
        let mut left: Vec<Node> = Vec::new();
        for (i, e) in named(d, "extends").enumerate() {
            if d.has_tag_name("typeAlias") {
                break;
            }
            // A generic base another base reaches already, maybe with other
            // arguments (`GPUDataOwner<'D>` beside a Container that is a
            // `GPUDataOwner<obj>`): F# allows one instantiation; the other path has it.
            let target = only_child(e);
            if elements(target).next().is_some()
                && let Some(to) = target.attribute("to")
                && direct.iter().enumerate().any(|(j, other)| j != i && other.is_some_and(|o| self.reaches(o, to)))
            {
                // Its members are declared again, with these arguments.
                left.push(target);
                continue;
            }
            let base = self.ty(target, &owner);
            match base.strip_prefix("ResizeArray<").and_then(|b| b.strip_suffix('>')) {
                Some(elem) => {
                    is_array = true;
                    prefix_body.push(format!("{indent}    inherit seq<{elem}>\n"));
                    prefix_body.push(format!("{indent}    [<EmitIndexer>] abstract Item: index: int -> {elem} with get, set\n"));
                    prefix_body.push(format!("{indent}    [<Emit(\"$0.length\")>] abstract length: int\n"));
                }
                None => prefix_body.push(format!("{indent}    inherit {base}\n")),
            }
        }
        // `[Symbol.iterator](): ArrayIterator<Node>`: a seq of Nodes, which Fable iterates by that method.
        let iterator = elements(src).find(|m| m.attribute("name") == Some("[Symbol.iterator]") && !flag(*m, "static"));
        // A type iterates as its ancestor does when that one is a seq already:
        // F# allows one seq per type (`HTMLCollectionOf<'T>` stays a seq of Elements).
        let mut todo: Vec<String> = self.bases.get(d.attribute("id").unwrap_or(attr(d, "name"))).cloned().unwrap_or_default();
        let mut seen = HashSet::new();
        let mut inherited = false;
        while let Some(b) = todo.pop() {
            if seen.insert(b.clone()) {
                inherited |= self.iterable.contains(&b);
                todo.extend(self.bases.get(&b).cloned().unwrap_or_default());
            }
        }
        if let Some(elem) = iterator.filter(|_| !inherited && !is_array).and_then(|m| named(m, "returns").next()).and_then(|r| elements(r).next()).and_then(|it| elements(it).next()) {
            let line = format!("inherit seq<{}>", self.ty(elem, &owner));
            // F# files cannot refer forward, and an inherited interface cannot be an extension.
            let latest = words(&line).iter().filter_map(|w| self.file_of.get(*w)).copied().max().unwrap_or(0);
            if latest > self.current_file && indent.is_empty() {
                self.skip("iterable over a type of a later file");
            } else {
                prefix_body.push(format!("{indent}    {line}\n"));
            }
        }
        let root = d.document().root_element();
        let mut inherited_methods: HashSet<String> = HashSet::new();
        for b in &seen {
            if let Some(bd) = find_decl(root, b) {
                let bsrc = if bd.has_tag_name("typeAlias") { only_child_type(bd) } else { bd };
                for bm in elements(bsrc) {
                    if !flag(bm, "static")
                        && (bm.has_tag_name("method") || (bm.has_tag_name("property") && !flag(bm, "optional") && only_child(bm).has_tag_name("functionType")))
                        && let Some(n) = bm.attribute("name")
                    {
                        inherited_methods.insert(n.to_string());
                    }
                }
            }
        }
        let saved_inherited_methods = std::mem::replace(&mut self.inherited_methods, inherited_methods);
        let mut ancestor_widens: HashSet<String> = HashSet::new();
        if !d.has_tag_name("typeAlias") {
            let saved_probing = std::mem::replace(&mut self.probing, true);
            for b in &seen {
                if let Some(bd) = find_decl(root, b) {
                    let b_file = if let Some(module) = bd.attribute("module") {
                        let unit = self.units.get(attr(bd, "name")).cloned().unwrap_or_else(|| module.to_string());
                        self.file_of.get(&format!("module:{unit}")).copied().unwrap_or(self.current_file)
                    } else {
                        self.file_of.get(attr(bd, "name")).copied().unwrap_or(self.current_file)
                    };
                    let b_owner = format!("{}{}", type_ident(attr(bd, "name")), self.type_params(bd));
                    for (mname, _) in self.widen_candidates(root, bd, &b_owner, b_file) {
                        ancestor_widens.insert(mname);
                    }
                }
            }
            self.probing = saved_probing;
        }
        let member_list = self.members(&instance, &owner, &extend, indent);
        let extra_own: &[(&str, &'static str)] = if is_array { &[("Item", ""), ("length", "")] } else { &[] };
        let redecl_list = self.redeclared(d, &left, &member_list, extra_own, &owner, &extend, indent);
        let mut suffix_body = Vec::new();
        if !d.has_tag_name("typeAlias") {
            for (mname, def_base) in self.widen_candidates(root, d, &owner, self.current_file) {
                if !instance.iter().any(|m| attr(*m, "name") == mname)
                    && !self.inherited_methods.contains(&mname)
                    && !ancestor_widens.contains(&mname)
                {
                    suffix_body.push(format!("{indent}    [<Emit(\"$0\")>] abstract {mname}: unit -> {def_base}\n"));
                }
            }
        }
        self.inherited_methods = saved_inherited_methods;

        let depth = self.inheritance_depth(decl_id);
        let disambiguated = |args: &str| {
            let mut sentinel = "_base".to_string();
            let arg_words = words(args);
            while arg_words.contains(sentinel.as_str()) {
                sentinel.push('_');
            }
            if args.is_empty() { format!("?{sentinel}: unit") } else { format!("{args}, ?{sentinel}: unit") }
        };
        match factory {
            FactoryResult::Inline { args, emit } => {
                if is_base && !self.in_globals {
                    let bargs = disambiguated(&args);
                    let ext = format!("        [<ParamObject; Emit({emit:?})>]\n        static member Create ({bargs}) : {extend} = jsNative\n");
                    let lines = self.factories.entry(self.current_file).or_default().entry(depth).or_default().entry(extend.clone()).or_default();
                    if !lines.contains(&ext) {
                        lines.push(ext);
                    }
                } else {
                    suffix_body.push(format!("{indent}    [<ParamObject; Emit({emit:?})>]\n{indent}    static member Create ({args}) : {owner} = jsNative\n"));
                }
            }
            FactoryResult::Extension { latest, args, emit } => {
                if !self.in_globals {
                    let fargs = if is_base { disambiguated(&args) } else { args };
                    let ext = format!("        [<ParamObject; Emit({emit:?})>]\n        static member Create ({fargs}) : {extend} = jsNative\n");
                    let lines = self.factories.entry(latest).or_default().entry(depth).or_default().entry(extend.clone()).or_default();
                    if !lines.contains(&ext) {
                        lines.push(ext);
                    }
                }
            }
            FactoryResult::None => {}
        }
        let mut rendered = prefix_body.concat();
        rendered += &render_members(&member_list);
        rendered += &render_members(&redecl_list);
        rendered += &suffix_body.concat();
        if rendered.is_empty() {
            rendered = format!("{indent}    interface end\n");
        }
        self.out += &rendered;
        self.out += "\n";
        if d.has_tag_name("class") {
            self.static_side(&name, &path, &statics, Some(&owner), Some(d), indent);
        }
        self.cur_constrained = saved_constrained;
    }

    // An options bag (properties + optional methods, its bases' too) is made with named
    // arguments: `AddEventListenerOptions.Create (once = true)`, the object
    // they make, by Fable's ParamObject. Not where a value of its name has
    // a `Create` (a class). When an argument refers to a later file, emits as an
    // extension `Create` in that file instead.
    fn factory(&mut self, d: Node, owner: &str, extend: &str, is_base: bool) -> FactoryResult {
        let initial_owner = if is_base && !self.in_globals { extend } else { owner };
        match self.factory_build(d, initial_owner, extend) {
            Some((latest, line, emit)) if latest > self.current_file => {
                if !self.in_globals && initial_owner != extend {
                    let Some((ext_latest, ext_line, ext_emit)) = self.factory_build(d, extend, extend) else {
                        return FactoryResult::None;
                    };
                    return FactoryResult::Extension { latest: ext_latest.max(self.current_file + 1), args: ext_line, emit: ext_emit };
                }
                FactoryResult::Extension { latest, args: line, emit }
            }
            Some((_, line, emit)) => FactoryResult::Inline { args: line, emit },
            None => FactoryResult::None,
        }
    }

    fn factory_build(&mut self, d: Node, owner: &str, extend: &str) -> Option<(usize, String, String)> {
        let root = d.document().root_element();
        let raw = attr(d, "name");
        let constrained = d.attribute("id").is_some_and(|id| self.constrained.contains_key(id));
        if constrained || d.parent().is_some_and(|p| elements(p).any(|v| v.has_tag_name("var") && attr(v, "name") == raw)) {
            return None;
        }
        // Discover all members and bases first before mutating delegate/hoist caches.
        let mut collected: Vec<(Node, HashMap<String, String>)> = Vec::new();
        let mut seen_names: HashSet<String> = HashSet::new();
        let mut todo: Vec<(Node, HashMap<String, String>)> = vec![(d, HashMap::new())];
        let mut seen = HashSet::new();
        while let Some((t, outer)) = todo.pop() {
            if !seen.insert(t.attribute("id").unwrap_or(attr(t, "name")).to_string()) {
                continue;
            }
            let src = if t.has_tag_name("typeAlias") { only_child_type(t) } else { t };
            for m in elements(src).filter(|m| !m.has_tag_name("typeParameter") && !m.has_tag_name("extends")) {
                if m.has_tag_name("index") {
                    continue;
                }
                let is_opt_method = m.has_tag_name("method")
                    && !flag(m, "static")
                    && flag(m, "optional")
                    && named(m, "typeParameter").next().is_none()
                    && !named(m, "parameter").any(|p| p.has_attribute("const"));
                let is_prop = m.has_tag_name("property") && !flag(m, "static");
                if !is_prop && !is_opt_method {
                    return None;
                }
                let mname = attr(m, "name").to_string();
                if seen_names.insert(mname) {
                    collected.push((m, outer.clone()));
                }
            }
            if !t.has_tag_name("typeAlias") {
                for e in named(t, "extends") {
                    let r = only_child(e);
                    let b = r.attribute("to").and_then(|to| find_decl(root, to))?;
                    if !(b.has_tag_name("interface") || (b.has_tag_name("typeAlias") && only_child_type(b).has_tag_name("object"))) {
                        return None;
                    }
                    let bparams: Vec<Node> = named(b, "typeParameter").collect();
                    let rargs: Vec<Node> = elements(r).collect();
                    let mut map = HashMap::new();
                    for (i, p) in bparams.iter().enumerate() {
                        let raw_arg = if let Some(a) = rargs.get(i) {
                            self.ty(*a, owner)
                        } else if let Some(def) = named(*p, "default").next() {
                            self.ty(only_child(def), owner)
                        } else {
                            return None;
                        };
                        map.insert(attr(*p, "name").to_string(), subst_params(&raw_arg, &outer));
                    }
                    todo.push((b, map));
                }
            }
        }
        if collected.is_empty() || collected.iter().any(|(m, _)| !is_plain_ident(attr(*m, "name")) || attr(*m, "name") == "Create") {
            return None;
        }
        let own_tparams: HashSet<String> = named(d, "typeParameter").map(|p| attr(p, "name").to_string()).collect();
        let (required, optional): (Vec<_>, Vec<_>) =
            collected.into_iter().partition(|(m, _)| m.has_tag_name("property") && !flag(*m, "optional") && !m.has_attribute("const"));
        let mut fixed = Vec::new();
        let mut args = Vec::new();
        for (m, map) in required.iter().chain(&optional) {
            if let Some(val) = const_js(*m) {
                fixed.push(format!("{:?}: {val}", attr(*m, "name")));
            }
            let raw_t = if m.has_tag_name("method") { self.fn_type(*m, owner) } else { self.ty(only_child(*m), owner) };
            let t = subst_params(&raw_t, map);
            if collect_type_vars(&t).iter().any(|v| !own_tparams.contains(v)) {
                return None;
            }
            let is_opt = m.has_tag_name("method") || flag(*m, "optional") || m.has_attribute("const");
            let q = if is_opt { "?" } else { "" };
            let t = if is_opt { strip_opt(&t) } else { t.as_str() };
            args.push(format!("{q}{}: {t}", ident(attr(*m, "name"))));
        }
        let line = args.join(", ");
        let emit = if fixed.is_empty() { "$0".to_string() } else { format!("Object.assign({{{}}}, $0)", fixed.join(", ")) };
        let latest = words(&line).iter().chain(words(extend).iter()).filter_map(|w| self.file_of.get(*w)).copied().max().unwrap_or(0);
        Some((latest, line, emit))
    }

    // Resolves default type arguments for `tparams` on a signature (`<constructor>` or `<function>`).
    // Returns `(ret_only_map, all_tparams_are_ret_only_with_defaults, all_defaults_with_constraint_fallback)`.
    pub(super) fn defaulted_tp_maps(&mut self, sig_node: Node, owner: &str, tparams: &[Node], constrained_flags: Option<&[Option<String>]>, allow_zero_arg_fallback: bool) -> (HashMap<String, String>, bool, Option<HashMap<String, String>>) {
        let own_names: HashSet<&str> = if sig_node.has_tag_name("function") {
            HashSet::new()
        } else {
            named(sig_node, "typeParameter").map(|p| attr(p, "name")).collect()
        };
        let tp_names: HashSet<String> = tparams.iter().map(|p| attr(*p, "name").to_string()).chain(own_names.iter().map(|s| (*s).to_string())).collect();
        let mut in_params: HashSet<&str> = HashSet::new();
        for p in named(sig_node, "parameter").filter(|p| !p.has_attribute("const")) {
            for r in p.descendants().filter(|r| r.has_tag_name("ref") && attr(*r, "kind") == "typeParameter") {
                in_params.insert(attr(r, "name"));
            }
        }
        let resolve_one = |g: &mut Gen, i: usize, tp: Node, use_constraint: bool, acc: &HashMap<String, String>| -> Option<String> {
            let name = attr(tp, "name");
            if own_names.contains(name) {
                return None;
            }
            let has_keyof = elements(tp).find(|c| c.has_tag_name("constraint")).is_some_and(|c| only_child(c).has_tag_name("keyof"));
            if has_keyof {
                return None;
            }
            let def_node = named(tp, "default").next().map(only_child).or_else(|| if use_constraint { named(tp, "constraint").next().map(only_child) } else { None })?;
            let raw = subst_params(&g.ty(def_node, owner), acc);
            let is_values_fn = sig_node.has_tag_name("function") && g.cur_indent.is_empty();
            if !is_values_fn && words(&raw).iter().filter_map(|w| g.file_of.get(*w)).any(|f| *f > g.current_file) {
                return None;
            }
            if collect_type_vars(&raw).iter().any(|v| tp_names.contains(v)) {
                return None;
            }
            // Check F# constraint compatibility when F# keeps a `:>` constraint on this type parameter.
            if let Some(c) = named(tp, "constraint").next().map(only_child) {
                let has_fs_constraint = match constrained_flags {
                    Some(cs) => cs.get(i).and_then(|x| x.as_ref()).is_some(),
                    None => c.has_tag_name("ref") && matches!(attr(c, "kind"), "interface" | "class") && g.ty(c, "") != "obj",
                };
                if has_fs_constraint {
                    if raw == "obj" {
                        return None;
                    }
                    if let Some(cid) = c.attribute("to") {
                        let ok = def_node.attribute("to") == Some(cid) || def_node.attribute("to").is_some_and(|did| g.reaches(did, cid));
                        if !ok {
                            return None;
                        }
                    }
                }
            }
            Some(raw)
        };
        let mut all_def_only = HashMap::new();
        let mut all_resolved = true;
        for (i, &tp) in tparams.iter().enumerate() {
            if let Some(ty) = resolve_one(self, i, tp, false, &all_def_only) {
                all_def_only.insert(attr(tp, "name").to_string(), ty);
            } else {
                all_resolved = false;
            }
        }
        let ret_only_map: HashMap<String, String> = all_def_only.iter().filter(|(k, _)| !in_params.contains(k.as_str())).map(|(k, v)| (k.clone(), v.clone())).collect();
        let all_ret_only = all_resolved && ret_only_map.len() == tparams.len() && !tparams.is_empty();
        let zero_arg_map = if !all_ret_only && allow_zero_arg_fallback && !tparams.is_empty() {
            let mut m = HashMap::new();
            let ok = tparams.iter().enumerate().all(|(i, &tp)| {
                if let Some(ty) = resolve_one(self, i, tp, true, &m) {
                    m.insert(attr(tp, "name").to_string(), ty);
                    true
                } else {
                    false
                }
            });
            ok.then_some(m)
        } else {
            None
        };
        (ret_only_map, all_ret_only, zero_arg_map)
    }

    // Formats `<'A, 'B when 'A :> Foo >` for a subset/all of a class's type parameters using `self.constrained`.
    fn format_class_tps(&self, d: Node, skip: &HashMap<String, String>) -> String {
        let cs = d.attribute("id").and_then(|id| self.constrained.get(id));
        let mut ps = Vec::new();
        let mut when = Vec::new();
        for (i, tp) in named(d, "typeParameter").enumerate() {
            let raw_name = attr(tp, "name");
            if skip.contains_key(raw_name) {
                continue;
            }
            let pname = format!("'{raw_name}");
            ps.push(pname.clone());
            if let Some(Some(c)) = cs.and_then(|cs| cs.get(i)) {
                when.push(format!("{pname} :> {}", fable_builtin(c).unwrap_or_else(|| type_path(c))));
            }
        }
        if ps.is_empty() {
            String::new()
        } else if when.is_empty() {
            format!("<{}>", ps.join(", "))
        } else {
            let sp = if when.last().unwrap().ends_with('>') { " " } else { "" };
            format!("<{} when {}{sp}>", ps.join(", "), when.join(" and "))
        }
    }

    // A value with constructors and static members: a type for its shape and a global of that type.
    pub(super) fn static_side(&mut self, name: &str, path: &str, members: &[Node], instance: Option<&str>, class_node: Option<Node>, indent: &str) {
        let static_name = type_ident(&format!("{}Static", name.trim_matches('`')));
        self.out += &format!("{indent}[<AllowNullLiteral>]\n{indent}type {static_name} =\n");
        let mut lines: Vec<FsMember> = Vec::new();
        let mut ctor_extras: Vec<FsMember> = Vec::new();
        let mut non_ctors = Vec::new();
        for &m in members {
            if (m.has_tag_name("constructor") || m.has_tag_name("construct")) && let Some(inst) = instance {
                let ret = inst.to_string();
                let own_tps = self.type_params(m);
                let params = self.params(m, "");
                let all_opt_or_rest = named(m, "parameter").filter(|p| !p.has_attribute("const")).all(|p| flag(p, "optional") || flag(p, "rest"));
                let (ret_only_map, all_ret_only, zero_arg_map) = match class_node.filter(|_| own_tps.is_empty()) {
                    Some(cd) => {
                        let ctps: Vec<Node> = named(cd, "typeParameter").collect();
                        if ctps.is_empty() {
                            (HashMap::new(), false, None)
                        } else {
                            let cflags = cd.attribute("id").and_then(|id| self.constrained.get(id)).cloned();
                            self.defaulted_tp_maps(m, "", &ctps, cflags.as_deref(), all_opt_or_rest && !params.is_empty())
                        }
                    }
                    None => (HashMap::new(), false, None),
                };
                let m_marks = marks(m);
                let emit_ctor = |g: &mut Gen, lines: &mut Vec<FsMember>, ctor_extras: &mut Vec<FsMember>, tps: &str, cparams: &[FsParam], cret: &str, expand: bool| {
                    let rendered_params: Vec<String> = cparams.iter().map(FsParam::render).collect();
                    let sig = format!("{} -> {cret}", if rendered_params.is_empty() { "unit".to_string() } else { rendered_params.join(" * ") });
                    let latest = words(&sig).iter().filter_map(|w| g.file_of.get(*w)).copied().max().unwrap_or(0);
                    if latest > g.current_file && indent.is_empty() {
                        let ext = format!("        [<Emit(\"new $0($1...)\")>] member this.Create{tps}({}): {cret} = jsNative\n", rendered_params.join(", "));
                        let exts = g.extensions.entry(latest).or_default().entry(static_name.clone()).or_default();
                        if !exts.contains(&ext) {
                            exts.push(ext);
                        }
                        return;
                    }
                    push_member(
                        lines,
                        FsMember {
                            indent: indent.to_string(),
                            attrs: vec!["[<EmitConstructor>]".into()],
                            name: "Create".into(),
                            kind: FsMemberKind::AbstractMethod {
                                tparams: tps.to_string(),
                                params: cparams.to_vec(),
                                ret: cret.to_string(),
                            },
                            marks: m_marks.clone(),
                        },
                        &g.abbrevs,
                    );
                    if expand {
                        for ep in g.union_param_expansions(m, "", cparams) {
                            ctor_extras.push(FsMember {
                                indent: indent.to_string(),
                                attrs: vec!["[<EmitConstructor>]".into()],
                                name: "Create".into(),
                                kind: FsMemberKind::AbstractMethod {
                                    tparams: tps.to_string(),
                                    params: ep,
                                    ret: cret.to_string(),
                                },
                                marks: m_marks.clone(),
                            });
                        }
                    }
                };
                if all_ret_only {
                    // Rule A1: emit both a non-generic defaulted Create and an explicitly generic Create<...>.
                    let cd = class_node.unwrap();
                    let def_params: Vec<FsParam> = params.iter().map(|p| p.subst(&ret_only_map)).collect();
                    let def_ret = subst_params(&ret, &ret_only_map);
                    emit_ctor(self, &mut lines, &mut ctor_extras, "", &def_params, &def_ret, true);
                    let explicit_tps = self.format_class_tps(cd, &HashMap::new());
                    emit_ctor(self, &mut lines, &mut ctor_extras, &explicit_tps, &params, &ret, true);
                } else {
                    // Rule A2: if all params are optional/rest and all class tps resolve, add a 0-arg defaulted Create.
                    if let Some(zmap) = &zero_arg_map {
                        let zret = subst_params(&ret, zmap);
                        emit_ctor(self, &mut lines, &mut ctor_extras, "", &[], &zret, false);
                    }
                    // Rule B: drop return-only defaulted type params from the constructor return type.
                    if !ret_only_map.is_empty() {
                        let cd = class_node.unwrap();
                        let bparams: Vec<FsParam> = params.iter().map(|p| p.subst(&ret_only_map)).collect();
                        let bret = subst_params(&ret, &ret_only_map);
                        let rem_tps = if zero_arg_map.is_some() { self.format_class_tps(cd, &ret_only_map) } else { own_tps };
                        emit_ctor(self, &mut lines, &mut ctor_extras, &rem_tps, &bparams, &bret, true);
                    } else {
                        let rem_tps = match (class_node, zero_arg_map.is_some()) {
                            (Some(cd), true) => self.format_class_tps(cd, &HashMap::new()),
                            _ => own_tps,
                        };
                        emit_ctor(self, &mut lines, &mut ctor_extras, &rem_tps, &params, &ret, true);
                    }
                }
            } else {
                non_ctors.push(m);
            }
        }
        for line_m in self.members(&non_ctors, "obj", &static_name, indent) {
            push_member(&mut lines, line_m, &self.abbrevs);
        }
        for e in ctor_extras {
            if !has_member_conflict(&lines, &e, &self.abbrevs) {
                lines.push(e);
            }
        }
        if lines.is_empty() {
            self.out += &format!("{indent}    interface end\n");
        } else {
            self.out += &render_members(&lines);
        }
        self.out += "\n";
        let value = format!("{indent}{}\n{indent}let {name}: {static_name} = jsNative\n\n", self.at(path, None));
        if indent.is_empty() {
            self.values += &value;
        } else {
            self.out += &value;
        }
    }
}
