use std::collections::HashSet;

use roxmltree::Node;

use super::super::{attr, elements, flag, named, only_child};
use super::Gen;
use super::member_ir::render_members;
use super::syntax::{
    add_opt, collect_ordered_tparams, const_js, ident, is_plain_ident, pascal, ret_type_word, strip_opt, type_ident, type_word, words,
};
use crate::lower::{SYNTH_NAME_MAX, trim_wrapper_suffixes};

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
    let mut out = Vec::new();
    for k in (2..=places.len().min(3)).chain(1..=1) {
        let mut segs: Vec<String> = places[..k.min(places.len())].iter().rev().cloned().collect();
        let raw_len: usize = segs.iter().map(String::len).sum();
        if segs.len() >= 3 && raw_len > SYNTH_NAME_MAX {
            let last_idx = segs.len() - 1;
            segs = segs
                .into_iter()
                .enumerate()
                .filter(|(i, s)| *i == 0 || *i == last_idx || !matches!(s.as_str(), "Result" | "Return" | "Value"))
                .map(|(_, s)| s)
                .collect();
            let trimmed_len: usize = segs.iter().map(String::len).sum();
            if trimmed_len > SYNTH_NAME_MAX {
                for i in 0..segs.len().saturating_sub(1) {
                    segs[i] = trim_wrapper_suffixes(&segs[i]).to_string();
                }
            }
        }
        let cand: String = segs.concat();
        if !cand.is_empty() && !out.contains(&cand) {
            out.push(cand);
        }
    }
    out
}

fn prop_desc(t: Node) -> Option<String> {
    match t.tag_name().name() {
        "functionType" => Some("Fn".into()),
        "object" => {
            if elements(t).any(|m| m.has_tag_name("call"))
                && elements(t).all(|m| m.has_tag_name("call") || m.has_tag_name("typeParameter"))
            {
                Some("Fn".into())
            } else {
                Some("Obj".into())
            }
        }
        "array" | "tuple" => Some("Array".into()),
        "union" => {
            let parts: Vec<String> = elements(t).filter_map(prop_desc).take(2).collect();
            Some(if parts.is_empty() { "Union".into() } else { parts.join("Or") })
        }
        "nullable" => prop_desc(only_child(t)),
        _ => ret_type_word(t),
    }
}

fn union_sibling_suffixes(n: Node) -> Vec<String> {
    let Some(parent) = n.parent().filter(|p| p.has_tag_name("union")) else {
        return Vec::new();
    };
    let siblings: Vec<Node> = elements(parent)
        .filter(|c| c.has_tag_name("object") || c.has_tag_name("functionType"))
        .collect();
    if siblings.len() < 2 {
        return Vec::new();
    }
    let obj_info = |obj: Node| -> (bool, Vec<(String, Option<String>)>) {
        if obj.has_tag_name("functionType") {
            return (true, Vec::new());
        }
        let has_call = elements(obj).any(|m| m.has_tag_name("call"));
        let mut props = Vec::new();
        for m in elements(obj).filter(|m| m.has_tag_name("property")) {
            let name = attr(m, "name").to_string();
            if !props.iter().any(|(pn, _)| pn == &name) {
                props.push((name, prop_desc(only_child(m))));
            }
        }
        (has_call, props)
    };
    let infos: Vec<(Node, bool, Vec<(String, Option<String>)>)> = siblings
        .into_iter()
        .map(|s| {
            let (hc, props) = obj_info(s);
            (s, hc, props)
        })
        .collect();
    let Some((_, own_has_call, own_props)) = infos.iter().find(|(s, _, _)| *s == n) else {
        return Vec::new();
    };
    let others: Vec<_> = infos.iter().filter(|(s, _, _)| *s != n).collect();

    let mut out = Vec::new();
    let mut push = |s: String| {
        if !s.is_empty() && !out.contains(&s) {
            out.push(s);
        }
    };

    if *own_has_call {
        if own_props.is_empty() {
            if others.iter().any(|(_, hc, op)| !hc || !op.is_empty()) {
                push("Fn".into());
            }
        } else {
            for (p, _) in own_props {
                if !others.iter().any(|(_, _, op)| op.iter().any(|(on, _)| on == p)) {
                    push(format!("With{}", pascal(p)));
                }
            }
            for (p, _) in own_props {
                push(format!("With{}", pascal(p)));
            }
        }
    }

    // Properties unique to this sibling among all union arms.
    let mut dist = Vec::new();
    for (p, _) in own_props {
        if !others.iter().any(|(_, _, op)| op.iter().any(|(on, _)| on == p)) {
            let w = pascal(p);
            if !w.is_empty() && !dist.contains(&w) {
                dist.push(w.clone());
            }
            push(w);
        }
    }
    // Properties present in this sibling that at least one sibling lacks.
    for (p, _) in own_props {
        if others.iter().any(|(_, _, op)| !op.iter().any(|(on, _)| on == p)) {
            let w = pascal(p);
            if !w.is_empty() && !dist.contains(&w) {
                dist.push(w.clone());
            }
            push(w);
        }
    }
    // Shared properties whose child type descriptor differs from another sibling.
    let mut typed_dist = Vec::new();
    for (p, desc) in own_props {
        let Some(d) = desc else { continue };
        if others
            .iter()
            .any(|(_, _, op)| op.iter().any(|(on, od)| on == p && od.as_deref() != Some(d.as_str())))
        {
            let w = format!("{}{d}", pascal(p));
            if !typed_dist.contains(&w) {
                typed_dist.push(w.clone());
            }
            push(w);
        }
    }
    // Combine a distinguishing property with a differing-type property when multiple arms share one.
    for d in &dist {
        for td in &typed_dist {
            push(format!("{d}{td}"));
        }
    }
    push("Base".into());
    push("Object".into());
    out
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
                .chain(named(n, "returns").next().and_then(|r| ret_type_word(only_child(r))).map(|w| format!("To{w}")))
                .collect();
            let mut cands = at.clone();
            for a in &at {
                cands.extend(said.iter().map(|w| format!("{a}{w}")));
                for (i, x) in said.iter().enumerate() {
                    cands.extend(said[i + 1..].iter().map(|y| format!("{a}{x}{y}")));
                }
            }
            cands.extend(at.iter().map(|a| format!("{a}Fn")));
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
        let sibling_suffixes = union_sibling_suffixes(n);
        if !sibling_suffixes.is_empty() {
            for base in &at {
                for suffix in &sibling_suffixes {
                    candidates.push(format!("{base}{suffix}"));
                }
            }
        }
        candidates.extend(at.iter().cloned());
        match candidates.into_iter().find(|c| self.is_hoist_name_free(c)) {
            Some(c) => c,
            None => {
                if let Some(base) = at.first() {
                    (1usize..)
                        .map(|i| {
                            if base.as_bytes().last().is_some_and(u8::is_ascii_digit) {
                                format!("{base}_{i}")
                            } else {
                                format!("{base}{i}")
                            }
                        })
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
