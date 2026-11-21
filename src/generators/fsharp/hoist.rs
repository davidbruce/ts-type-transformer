use std::collections::HashSet;

use roxmltree::Node;

use super::super::{attr, elements, flag, named, only_child};
use super::Gen;
use super::member_ir::render_members;
use super::syntax::{
    add_opt, collect_ordered_tparams, const_js, ident, is_plain_ident, pascal, strip_opt, type_ident, type_word, words,
};

fn ancestor_places(n: Node) -> Vec<String> {
    n.ancestors()
        .skip(1)
        .filter(|a| matches!(a.tag_name().name(), "parameter" | "property" | "method" | "function" | "interface" | "class" | "typeAlias" | "var" | "namespace"))
        .filter_map(|a| a.attribute("jsName").filter(|j| *j != "default").or(a.attribute("name")))
        .map(pascal)
        .filter(|p| !p.is_empty())
        .collect()
}

fn place_candidates(places: &[String]) -> Vec<String> {
    (2..=places.len().min(3))
        .chain(1..=1)
        .map(|k| places[..k.min(places.len())].iter().rev().cloned().collect::<String>())
        .filter(|c| !c.is_empty())
        .collect()
}

impl Gen {
    fn hoist_scope(&self) -> String {
        if !self.cur_indent.is_empty() {
            format!("ns:{}", self.cur_prefix)
        } else if self.in_globals {
            "globals".into()
        } else {
            "files".into()
        }
    }

    fn is_hoist_name_free(&self, c: &str) -> bool {
        !self.file_of.contains_key(c)
            && !self.declared.contains(c)
            && !self.type_names.contains(c)
            && !self.delegates.values().any(|d| d == c || d.rsplit('.').next() == Some(c))
            && !self.records.values().any(|d| d == c || d.rsplit('.').next() == Some(c))
    }

    fn place_hoisted_def(&mut self, mut name: String, def: &str, latest_src: &str, reject_ns_forward: bool) -> Option<String> {
        let target_file = words(latest_src)
            .iter()
            .filter_map(|w| self.file_of.get(*w))
            .copied()
            .max()
            .unwrap_or(0)
            .max(self.current_file);
        if reject_ns_forward && !self.cur_indent.is_empty() && target_file > self.current_file {
            return None;
        }
        if !self.cur_indent.is_empty() {
            self.pending_delegates += def;
        } else if self.in_globals {
            self.global_delegates += def;
        } else {
            self.file_of.insert(name.clone(), target_file);
            *self.alias_types.entry(target_file).or_default() += def;
        }
        // In a namespace, by its path: a companion type of the namespace's
        // functions, beside its module, names it too (`Fs.JsFn16`).
        if !self.cur_indent.is_empty() && !self.cur_prefix.is_empty() {
            name = format!("{}.{name}", self.cur_prefix.split('.').map(type_ident).collect::<Vec<_>>().join("."));
        }
        Some(name)
    }

    pub(super) fn function_delegate(&mut self, n: Node, owner: &str) -> String {
        let sig = self.callback_sig(n, owner);
        let sig_str = sig.invoke.clone();
        // The type parameters the signature shows, in order: erased ones it does not.
        let mut tparams: Vec<String> = Vec::new();
        let b = sig_str.as_bytes();
        for (i, _) in sig_str.match_indices('\'') {
            let end = b[i + 1..].iter().position(|c| !(c.is_ascii_alphanumeric() || *c == b'_')).map_or(b.len(), |p| i + 1 + p);
            let tp = sig_str[i..end].to_string();
            if end > i + 1 && !tparams.contains(&tp) {
                tparams.push(tp);
            }
        }
        let key = (self.hoist_scope(), tparams.clone(), sig_str.clone());
        let dname = if let Some(existing) = self.delegates.get(&key) {
            existing.clone()
        } else {
            // By where it is first: `mkdir`'s `callback` is `MkdirCallback`;
            // with what holds that, if another has the name; else numbered.
            let at = place_candidates(&ancestor_places(n));
            // Else with the types it takes, or gives (node's `pipeline` sources).
            let said: Vec<String> = named(n, "parameter")
                .filter_map(|p| type_word(only_child(p)))
                .chain(named(n, "returns").next().and_then(|r| type_word(only_child(r))).map(|w| format!("To{w}")))
                .collect();
            let mut cands = at.clone();
            if let Some(a) = at.first() {
                cands.extend(said.iter().map(|w| format!("{a}{w}")));
                for (i, x) in said.iter().enumerate() {
                    cands.extend(said[i + 1..].iter().map(|y| format!("{a}{x}{y}")));
                }
            }
            let by_place = cands.into_iter().find(|c| self.is_hoist_name_free(c));
            let dname = match by_place {
                Some(c) => c,
                None => loop {
                    self.delegate_counter += 1;
                    let candidate = format!("JsFn{}", self.delegate_counter);
                    if self.is_hoist_name_free(&candidate) {
                        break candidate;
                    }
                },
            };
            let tp_suffix = if tparams.is_empty() { String::new() } else { format!("<{}>", tparams.join(", ")) };
            let ret_type = format!("{dname}{tp_suffix}");
            let def = self.callback_def(&dname, &tp_suffix, &ret_type, &sig, &self.cur_indent);
            let mut dname = self.place_hoisted_def(dname, &def, &sig_str, false).unwrap();
            self.delegates.insert(key, dname.clone());
            if !tparams.is_empty() {
                dname = format!("{dname}{tp_suffix}");
            }
            dname
        };
        if !tparams.is_empty() && !dname.contains('<') {
            format!("{dname}<{}>", tparams.join(", "))
        } else {
            dname
        }
    }

    fn pick_hoist_name(&mut self, n: Node, string_tags: &[&str]) -> String {
        let at = place_candidates(&ancestor_places(n));
        let mut candidates: Vec<String> = Vec::new();
        if string_tags.len() == 1 {
            let tag = pascal(string_tags[0]);
            if !tag.is_empty() {
                for base in &at {
                    candidates.push(format!("{base}{tag}"));
                }
            }
        }
        candidates.extend(at.iter().cloned());
        match candidates.into_iter().find(|c| self.is_hoist_name_free(c)) {
            Some(c) => c,
            None => {
                if let Some(base) = at.first() {
                    (1usize..)
                        .map(|i| format!("{base}{i}"))
                        .find(|c| self.is_hoist_name_free(c))
                        .unwrap()
                } else {
                    loop {
                        self.record_counter += 1;
                        let candidate = format!("JsObj{}", self.record_counter);
                        if self.is_hoist_name_free(&candidate) {
                            break candidate;
                        }
                    }
                }
            }
        }
    }

    // Hoists an inline option-bag object (`{ a?: T; b?: U; ... }`) to a named
    // `[<AllowNullLiteral; Interface>]` with a `[<ParamObject>]` `Create`, so
    // callers pass only the fields they want instead of every `None`.
    pub(super) fn hoist_object(&mut self, n: Node, props: &[Node], owner: &str) -> Option<String> {
        let mut fields: Vec<(String, String, String, bool, bool, Option<String>)> = Vec::new();
        let mut seen_props = HashSet::new();
        for &m in props {
            let raw = attr(m, "name");
            if !seen_props.insert(raw.to_string()) {
                continue;
            }
            let t = self.ty(only_child(m), owner);
            let opt = flag(m, "optional");
            let ro = flag(m, "readonly");
            let fixed = const_js(m);
            fields.push((raw.to_string(), ident(raw), t, opt, ro, fixed));
        }
        let key_sig: String = fields
            .iter()
            .map(|(raw, _, t, opt, ro, fixed)| format!("{raw}:{t}:{}:{}:{}", u8::from(*opt), u8::from(*ro), fixed.as_deref().unwrap_or("")))
            .collect::<Vec<_>>()
            .join(";");
        let tparams = collect_ordered_tparams(&key_sig, &HashSet::new());
        // A constrained enclosing type parameter cannot be hoisted without its constraint.
        if tparams.iter().any(|tp| self.cur_constrained.contains(tp)) {
            return None;
        }
        let key = (self.hoist_scope(), tparams.clone(), key_sig);
        let rname = if let Some(existing) = self.records.get(&key) {
            existing.clone()
        } else {
            let string_tags: Vec<&str> = props
                .iter()
                .filter_map(|m| m.attribute("const").filter(|_| matches!(only_child(*m).attribute("name"), Some("string" | "boolean"))))
                .collect();
            let rname = self.pick_hoist_name(n, &string_tags);
            let tp_suffix = if tparams.is_empty() { String::new() } else { format!("<{}>", tparams.join(", ")) };
            let indent = self.cur_indent.clone();
            let mut def = format!("{indent}[<AllowNullLiteral; Interface>]\n{indent}type {rname}{tp_suffix} =\n");
            let mut fixed_entries = Vec::new();
            let mut req_args = Vec::new();
            let mut opt_args = Vec::new();
            for (raw, fname, t, opt, ro, fixed) in &fields {
                let prop_ty = if *opt { add_opt(t.clone()) } else { t.clone() };
                let acc = if !*ro && prop_ty != "unit" { " with get, set" } else { "" };
                def += &format!("{indent}    abstract {fname}: {prop_ty}{acc}\n");
                if let Some(val) = fixed {
                    fixed_entries.push(format!("{raw:?}: {val}"));
                }
                if *opt || fixed.is_some() {
                    opt_args.push(format!("?{fname}: {}", strip_opt(t)));
                } else {
                    req_args.push(format!("{fname}: {t}"));
                }
            }
            req_args.extend(opt_args);
            let create_args = req_args.join(", ");
            let emit = if fixed_entries.is_empty() { "$0".to_string() } else { format!("Object.assign({{{}}}, $0)", fixed_entries.join(", ")) };
            def += &format!("{indent}    [<ParamObject; Emit({emit:?})>]\n{indent}    static member Create ({create_args}) : {rname}{tp_suffix} = jsNative\n\n");
            let mut rname = self.place_hoisted_def(rname, &def, &def, false).unwrap();
            self.records.insert(key, rname.clone());
            if !tparams.is_empty() {
                rname = format!("{rname}{tp_suffix}");
            }
            rname
        };
        Some(if !tparams.is_empty() && !rname.contains('<') { format!("{rname}<{}>", tparams.join(", ")) } else { rname })
    }

    // Hoists inline object types that contain methods, indexers, or call/construct
    // signatures to named interfaces instead of falling back to `obj`.
    pub(super) fn hoist_inline_interface(&mut self, n: Node, _owner: &str) -> Option<String> {
        let members: Vec<Node> = elements(n)
            .filter(|m| !m.has_tag_name("typeParameter") && !m.has_tag_name("extends"))
            .collect();
        if members.is_empty() {
            return None;
        }
        if self.probing {
            let mut inner_tps = HashSet::new();
            for tp_decl in n.descendants().filter(|d| d.has_tag_name("typeParameter")) {
                inner_tps.insert(attr(tp_decl, "name"));
            }
            let mut free: Vec<String> = Vec::new();
            for r in n.descendants().filter(|r| r.has_tag_name("ref") && r.attribute("kind") == Some("typeParameter")) {
                let name = attr(r, "name");
                if !inner_tps.contains(name) {
                    let tv = format!("'{name}");
                    if self.cur_constrained.contains(&tv) {
                        return None;
                    }
                    if !free.contains(&tv) {
                        free.push(tv);
                    }
                }
            }
            return Some(if free.is_empty() { "obj".into() } else { format!("__Probe<{}>", free.join(", ")) });
        }

        let indent = self.cur_indent.clone();
        let saved_hoisting = std::mem::replace(&mut self.hoisting, true);
        let saved_inherited = std::mem::take(&mut self.inherited_methods);
        let body_members = self.members(&members, "__HOIST_SELF__", "", &indent);
        self.hoisting = saved_hoisting;
        self.inherited_methods = saved_inherited;
        if body_members.is_empty() {
            return None;
        }

        let body_str = render_members(&body_members);
        let mut bound_member_tps = HashSet::new();
        for &m in &members {
            let sig = if m.has_tag_name("property") && !flag(m, "optional") && only_child(m).has_tag_name("functionType") {
                Some(only_child(m))
            } else if matches!(m.tag_name().name(), "method" | "call" | "construct") {
                Some(m)
            } else {
                None
            };
            if let Some(s) = sig {
                for p in named(s, "typeParameter") {
                    bound_member_tps.insert(format!("'{}", attr(p, "name")));
                }
            }
        }
        let tparams = collect_ordered_tparams(&body_str, &bound_member_tps);
        if tparams.iter().any(|tp| self.cur_constrained.contains(tp)) {
            return None;
        }

        let key = (self.hoist_scope(), tparams.clone(), format!("iface:{body_str}"));
        let rname = if let Some(existing) = self.records.get(&key) {
            existing.clone()
        } else {
            let latest = words(&body_str).iter().filter_map(|w| self.file_of.get(*w)).copied().max().unwrap_or(0);
            if !self.cur_indent.is_empty() && latest > self.current_file {
                return None;
            }
            let non_index: Vec<Node> = members.iter().copied().filter(|m| !m.has_tag_name("index")).collect();
            let mut seen_create_names = HashSet::new();
            let can_factory = !non_index.is_empty()
                && non_index.iter().all(|m| {
                    let is_opt_method = m.has_tag_name("method")
                        && !flag(*m, "static")
                        && flag(*m, "optional")
                        && named(*m, "typeParameter").next().is_none()
                        && !named(*m, "parameter").any(|p| p.has_attribute("const"));
                    let is_prop = m.has_tag_name("property") && !flag(*m, "static");
                    let raw = m.attribute("name").unwrap_or_default();
                    (is_prop || is_opt_method) && is_plain_ident(raw) && raw != "Create" && seen_create_names.insert(raw)
                });
            let mut create_snippet: Option<String> = None;
            if can_factory {
                let (required, optional): (Vec<_>, Vec<_>) = non_index
                    .into_iter()
                    .partition(|m| m.has_tag_name("property") && !flag(*m, "optional") && !m.has_attribute("const"));
                let mut fixed = Vec::new();
                let mut args = Vec::new();
                for m in required.iter().chain(&optional) {
                    if let Some(val) = const_js(*m) {
                        fixed.push(format!("{:?}: {val}", attr(*m, "name")));
                    }
                    let raw_t = if m.has_tag_name("method") {
                        self.fn_type(*m, "__HOIST_SELF__")
                    } else {
                        self.ty(only_child(*m), "__HOIST_SELF__")
                    };
                    let is_opt = m.has_tag_name("method") || flag(*m, "optional") || m.has_attribute("const");
                    let q = if is_opt { "?" } else { "" };
                    let t = if is_opt { strip_opt(&raw_t) } else { raw_t.as_str() };
                    args.push(format!("{q}{}: {t}", ident(attr(*m, "name"))));
                }
                let create_args = args.join(", ");
                let emit = if fixed.is_empty() { "$0".to_string() } else { format!("Object.assign({{{}}}, $0)", fixed.join(", ")) };
                create_snippet = Some(format!("{indent}    [<ParamObject; Emit({emit:?})>]\n{indent}    static member Create ({create_args}) : __HOIST_SELF__ = jsNative\n"));
            }

            let rname = self.pick_hoist_name(n, &[]);
            let tp_suffix = if tparams.is_empty() { String::new() } else { format!("<{}>", tparams.join(", ")) };
            let self_ty = format!("{rname}{tp_suffix}");
            let body_filled = body_str.replace("__HOIST_SELF__", &self_ty);
            let create_filled = create_snippet.map(|s| s.replace("__HOIST_SELF__", &self_ty));
            let attr_line = if create_filled.is_some() { "AllowNullLiteral; Interface" } else { "AllowNullLiteral" };
            let def = format!(
                "{indent}[<{attr_line}>]\n{indent}type {rname}{tp_suffix} =\n{body_filled}{}\n",
                create_filled.as_deref().unwrap_or("")
            );
            let mut rname = self.place_hoisted_def(rname, &def, &def, true)?;
            self.records.insert(key, rname.clone());
            if !tparams.is_empty() {
                rname = format!("{rname}{tp_suffix}");
            }
            rname
        };
        Some(if !tparams.is_empty() && !rname.contains('<') { format!("{rname}<{}>", tparams.join(", ")) } else { rname })
    }
}
