use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use super::emit::module_file;
use super::types::{Site, 
    args_of, avoid_outer_type_params, bound, defaulted, families, function, function_raw, generics,
    handle_decl, hidden, indented, is_subtype, lower_callback, names_of, one_function,
    param, raise, raw, raw_ret, ty, upcast_in, Family, VALUE,
};
use super::{camel, ident, snake, Context, PRELUDE, RUNTIME_TYPES, TAKEN};
use super::super::model::{base_args, map_types, mentions, subst, with_self_args, Call, Class, Func, Model, NamedUnion, TParam, Ty};

fn valid_with_param(t: &Ty) -> bool {
    match t {
        Ty::Nullable(inner, _) => !matches!(**inner, Ty::Void | Ty::Key(_) | Ty::Mapped(_) | Ty::Nullable(..)),
        _ => !matches!(t, Ty::Void | Ty::Key(_) | Ty::Mapped(_)),
    }
}

fn setter_for_with<'a>(k: &'a Class, method: &str, names: &HashMap<&str, HashMap<String, String>>) -> Option<(&'a Func, String)> {
    let k_names = &names[k.name.as_str()];
    let matching: Vec<&Func> = k.funcs.iter().filter(|f| Class::has_self(f) && k_names[&f.name] == method).collect();
    if matching.len() != 1 {
        return None;
    }
    let f0 = matching[0];
    let Call::Set(js) = &f0.call else {
        return None;
    };
    let stem = if !super::super::case::has_dollar_js_name(&f0.call) && super::super::case::starts_with_word(js, "set") {
        snake(js)
    } else {
        method.strip_prefix("set_").or_else(|| method.strip_prefix("assign_"))?.to_string()
    };
    if stem.is_empty()
        || f0.params.len() != 1
        || !f0.type_params.is_empty()
        || f0.ret != Ty::Void
        || !f0.fixed.is_empty()
        || !valid_with_param(&f0.params[0].1)
    {
        return None;
    }
    Some((f0, ident(&format!("with_{stem}"))))
}

fn with_setter_item(
    f: &Func,
    orig: &Func,
    with_name: &str,
    vis: &str,
    where_sized: &str,
    imports: &mut BTreeMap<String, String>,
    cx: &Context,
) -> Option<String> {
    let orig_t = &orig.params[0].1;
    let (p_raw, t) = &f.params[0];
    if !valid_with_param(t) {
        return None;
    }
    let p = ident(&snake(p_raw));
    let (written, val_raw_args) = match t {
        Ty::Nullable(inner, _) => {
            let (w, inner_args) = param(inner, &p, cx);
            let ra = if raw(orig_t) != raw(t) {
                vec![format!("{p}.lend().h")]
            } else {
                match &**inner {
                    Ty::Num => vec!["1".into(), p.clone()],
                    Ty::NumEnum(_) => vec!["1".into(), format!("{p} as i32 as f64")],
                    Ty::Bool | Ty::Enum(_) => vec![format!("{p} as i32")],
                    _ => inner_args,
                }
            };
            (w, ra)
        }
        _ => {
            let (w, mut ra) = param(t, &p, cx);
            if raw(orig_t) != raw(t) {
                ra = vec![format!("{p}.lend().h")];
            }
            (w, ra)
        }
    };
    let this_expr = "AsRef::<JsHandle>::as_ref(&self).0";
    let (import, mut raw_params, mut call_args) = match &f.shared {
        Some((shared, k_idx)) => (shared.clone(), vec!["this: u32".into(), "k: i32".into()], vec![this_expr.into(), k_idx.to_string()]),
        None => (f.import.clone(), vec!["this: u32".into()], vec![this_expr.into()]),
    };
    call_args.extend(val_raw_args);
    raw_params.extend(raw(orig_t).into_iter().enumerate().map(|(j, r)| format!("a0_{j}: {r}")));
    let raw_name = if super::KEYWORDS.contains(&import.as_str()) { format!("{import}_") } else { import.clone() };
    imports.entry(import.clone()).or_insert_with(|| {
        let link = if raw_name != import { format!("#[link_name = \"{import}\"] ") } else { String::new() };
        format!("    {link}pub fn {raw_name}({}) -> ();\n", raw_params.join(", "))
    });
    Some(format!(
        "    {vis}fn {with_name}(self, {p}: {written}) -> Self{where_sized} {{ unsafe {{ raw::{raw_name}({}) }}; self }}\n",
        call_args.join(", ")
    ))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn emit_classes(
    m: &Model,
    cx: &Context,
    declared: &[&String],
    resolved_opaque: &HashMap<String, (usize, String, bool)>,
    module_of: &HashMap<String, Vec<String>>,
    namespaces: &mut BTreeMap<Vec<String>, String>,
    modules: &mut BTreeMap<String, String>,
    imports: &mut BTreeMap<String, String>,
) {
    // Each class: a struct, a trait of its members, and that trait for each subtype.
    let class_of = |n: &str| m.classes.iter().find(|k| k.name == n);
    let ancestors = |name: &str| -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut todo = vec![name.to_string()];
        while let Some(c) = todo.pop() {
            for a in m.classes.iter().filter(|k| k.name == c).flat_map(|k| &k.ancestors) {
                if !out.contains(a) {
                    out.push(a.clone());
                    todo.insert(0, a.clone());
                }
            }
        }
        out
    };
    // The Rust name of each class's functions, by the model's.
    let names: HashMap<&str, HashMap<String, String>> = m.classes.iter().map(|c| (c.name.as_str(), names_of(c.funcs.iter()))).collect();
    let type_names: HashSet<String> = declared.iter().map(|n| cx.tn(n)).collect();
    let grouped: HashMap<&str, Vec<Family>> = m.classes.iter().map(|c| (c.name.as_str(), families(c, &names[c.name.as_str()], &type_names, cx))).collect();
    // The family a member is one overload of.
    let family_of = |c: &Class, f: &Func| grouped[c.name.as_str()].iter().find(|g| g.members.iter().any(|o| o.name == f.name));
    let member_names = |c: &Class| -> BTreeSet<String> { c.funcs.iter().filter(|f| Class::has_self(f)).map(|f| family_of(c, f).map_or_else(|| names[c.name.as_str()][&f.name].clone(), |g| g.name.clone())).collect() };
    // Precompute every `{Class}Methods` trait's method set (including `*_as`, `iter`, `with_*`).
    let mut trait_names: HashMap<String, HashSet<String>> = HashMap::new();
    let mut trait_with_entries: HashMap<String, Vec<(String, String)>> = HashMap::new();
    for k in &m.classes {
        let mnames = member_names(k);
        let mut tset: HashSet<String> = mnames.iter().cloned().collect();
        for g in &grouped[k.name.as_str()] {
            if g.members.iter().any(|f| f.params[g.at].1 == Ty::Str && !hidden(f).is_empty()) {
                tset.insert(format!("{}_as", g.name));
            }
        }
        for f in k.funcs.iter().filter(|f| Class::has_self(f) && family_of(k, f).is_none()) {
            let method = &names[k.name.as_str()][&f.name];
            if !hidden(&with_self_args(f, k)).is_empty() {
                tset.insert(format!("{method}_as"));
            }
        }
        if k.iterates.is_some() {
            tset.insert("iter".into());
        }
        if k.iterates_async.is_some() {
            tset.insert("iterate_async".into());
        }
        let mut withs = Vec::new();
        let mut seen_with = HashSet::new();
        for method in &mnames {
            if let Some((_, with_name)) = setter_for_with(k, method, &names)
                && !tset.contains(&with_name)
                && !TAKEN.contains(&with_name.as_str())
                && seen_with.insert(with_name.clone())
            {
                withs.push((method.clone(), with_name));
            }
        }
        for (_, w) in &withs {
            tset.insert(w.clone());
        }
        trait_names.insert(k.name.clone(), tset);
        trait_with_entries.insert(k.name.clone(), withs);
    }
    let this = "AsRef::<JsHandle>::as_ref(self).0";
    // A class no other class here extends has no trait: its members are its
    // own inherent methods, which it has anyway.
    let extended: HashSet<&str> = m.classes.iter().flat_map(|k| k.ancestors.iter().map(String::as_str)).filter(|a| class_of(a).is_some()).collect();
    for c in &m.classes {
        let out = match module_of.get(&c.name) {
            Some(path) => namespaces.entry(path.clone()).or_default(),
            None => modules.entry(module_file(&c.module)).or_insert_with(|| PRELUDE.to_string()),
        };
        let (bounds, args) = (generics(&c.type_params, cx), args_of(&c.type_params));
        let name = cx.def_name(&c.name);
        let me = format!("{name}{args}");
        let own = &names[c.name.as_str()];
        *out += &format!("\n{}\n", handle_decl(&c.name, &name, &c.type_params, c.doc.as_deref(), cx));
        *out += &format!("impl{bounds} Is<{me}> for {me} {{}}\n");
        if let Some(f) = c.funcs.iter().find(|f| matches!(f.call, Call::Create(_)) && f.params.is_empty() && f.type_params.is_empty()) {
            *out += &format!("impl{bounds} ::core::default::Default for {me} {{ fn default() -> Self {{ Self::{}() }} }}\n", own[&f.name]);
        }
        let all = ancestors(&c.name);
        let primary_base: Option<&String> = c.ancestors.iter().find(|a| {
            *a != &c.name
                && !ancestors(a).contains(&c.name)
                && (class_of(a).is_some()
                    || (m.opaque.contains(*a)
                        && !RUNTIME_TYPES.iter().any(|r| r.0 == a.as_str())
                        && resolved_opaque.get(*a).is_none_or(|x| x.2)))
        });
        let primary_target = primary_base.map(|p| {
            if let Some(kp) = class_of(p) {
                if kp.type_params.is_empty() {
                    cx.tn(p)
                } else {
                    let b = base_args(m, c, p).unwrap_or_default();
                    let s = (0..kp.type_params.len()).map(|i| b.get(i).map(|t| ty(t, cx)).unwrap_or_else(|| VALUE.into())).collect::<Vec<_>>().join(", ");
                    format!("{}<{s}>", cx.tn(p))
                }
            } else {
                let arity = m.opaque_arity.get(p).copied().unwrap_or(0);
                if arity == 0 {
                    cx.tn(p)
                } else {
                    let b = base_args(m, c, p).unwrap_or_default();
                    let s = (0..arity).map(|i| b.get(i).map(|t| ty(t, cx)).unwrap_or_else(|| VALUE.into())).collect::<Vec<_>>().join(", ");
                    format!("{}<{s}>", cx.tn(p))
                }
            }
        });
        if let Some(ref target) = primary_target {
            *out += &format!(
                "impl{bounds} ::core::ops::Deref for {me} {{ type Target = {target}; fn deref(&self) -> &Self::Target {{ unsafe {{ &*(self as *const Self as *const Self::Target) }} }} }}\n"
            );
        }
        // The trait of its own members (including fluent `with_<prop>` builders).
        let mut trait_body = String::new();
        let own_names: HashSet<String> = own.values().cloned().collect();
        for f in c.funcs.iter().filter(|f| Class::has_self(f) && family_of(c, f).is_none()) {
            for item in function(&with_self_args(f, c), &own[&f.name], "", imports, Some(this), &Site { assoc: false, taken: &own_names }, cx) {
                trait_body += &indented(&item);
            }
        }
        let mut dispatch = String::new();
        for g in &grouped[c.name.as_str()] {
            for item in g.methods("", imports, this, None, true, cx) {
                trait_body += &indented(&item);
            }
            dispatch += &g.dispatch(imports, cx);
        }
        if let Some(elem) = &c.iterates {
            let elem = with_self_args(&Func { type_params: vec![], module: String::new(), import: String::new(), name: String::new(), call: Call::Create(String::new()), params: vec![], rest: false, fixed: vec![], shared: None, ret: elem.clone(), returns_this: false, doc: None }, c).ret;
            trait_body += &format!("    fn iter(&self) -> JsIterator<{}> {{ JsIterator::over({this}) }}\n", ty(&elem, cx));
        }
        if let Some(elem) = &c.iterates_async {
            trait_body += &format!("    fn iterate_async(&self) -> JsAsyncIterator<{}> {{ JsAsyncIterator::over({this}) }}\n", ty(elem, cx));
        }
        if let Some(withs) = trait_with_entries.get(&c.name) {
            for (method, with_name) in withs {
                if let Some((f0, _)) = setter_for_with(c, method, &names) {
                    let orig = with_self_args(f0, c);
                    if let Some(item) = with_setter_item(&orig, &orig, with_name, "", " where Self: Sized", imports, cx) {
                        trait_body += &item;
                    }
                }
            }
        }
        let leaf = !extended.contains(c.name.as_str());
        if leaf {
            *out += &dispatch;
        } else {
            *out += &format!("pub trait {name}Methods{bounds}: AsRef<JsHandle> {{\n{trait_body}}}\n");
            *out += &format!("impl{bounds} {name}Methods{args} for {me} {{}}\n{dispatch}");
        }
        // Statics, constants, a map's keys, non-primary/colliding/own inherent methods, and conversions.
        let mut inherent = String::new();
        let mut inherent_taken: HashSet<String> = HashSet::new();
        // A constructor of a generic class is in an impl where the class's
        // parameters it does not take are their defaults, as `HashMap::new`:
        // `Mesh::new_with(&geometry, &material)`, no `Mesh::<G, M>`. An option
        // bag's `create()` stays generic: where it goes says what it is.
        let mut defaulted_impls: BTreeMap<(String, String), String> = BTreeMap::new();
        for f in c.funcs.iter().filter(|f| !Class::has_self(f)) {
            let fname = &own[&f.name];
            let f_self = with_self_args(f, c);
            if !hidden(&f_self).is_empty() {
                inherent_taken.insert(format!("{fname}_as"));
            }
            inherent_taken.insert(fname.clone());
            let (spec_bounds, spec_me, f_spec) = if matches!(f.call, Call::New(_)) { defaulted_impl(c, &name, &f_self, cx) } else { (bounds.clone(), me.clone(), f_self.clone()) };
            let body = if spec_me == me { &mut inherent } else { defaulted_impls.entry((spec_bounds, spec_me)).or_default() };
            for item in function(&f_spec, fname, "pub ", imports, None, &Site { assoc: true, taken: &own_names }, cx) {
                *body += &indented(&item);
            }
        }
        for ((spec_bounds, spec_me), body) in &defaulted_impls {
            *out += &format!("impl{spec_bounds} {spec_me} {{\n{body}}}\n");
        }
        for (constant, value) in &c.consts {
            let t = if value.starts_with('"') { "&'static str" } else if value == "true" || value == "false" { "bool" } else { "f64" };
            // A whole number, decimal or hexadecimal, as an f64.
            let value = if t == "f64" && (value.starts_with("0x") || value.starts_with("0X")) { format!("{value}i64 as f64") } else if t == "f64" && !value.contains('.') && !value.contains('e') { format!("{value}.0") } else { value.clone() };
            let cname = snake(&super::super::model::ident(constant)).to_uppercase();
            inherent_taken.insert(cname.clone());
            inherent += &format!("    pub const {cname}: {t} = {value};\n");
        }
        if let Some(map) = m.maps.iter().find(|k| k.name == c.name) {
            let mut seen = HashSet::new();
            for (i, (key, v)) in map.keys.iter().zip(&map.values).enumerate() {
                let constant = snake(&super::super::model::ident(key)).to_uppercase();
                let constant = if seen.insert(constant.clone()) { constant } else { format!("{constant}_{i}") };
                inherent_taken.insert(constant.clone());
                inherent += &format!("    pub const {constant}: JsKey<{name}, {}> = JsKey::new({i});\n", ty(v, cx));
            }
        }
        // A subclass of Array, as the array it is.
        if let Some(elem) = &c.elements {
            inherent_taken.insert("as_array".into());
            inherent += &format!("    pub fn as_array(&self) -> JsArray<{}> {{ JsArray::from_handle(crate::runtime::give(self)) }}\n", ty(elem, cx));
        }
        let mut holders: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for h in std::iter::once(c.name.clone()).chain(all.iter().cloned()) {
            if let Some(k) = class_of(&h) {
                for n in member_names(k) {
                    holders.entry(n).or_default().push(h.clone());
                }
            }
        }
        let args_for = |h: &str, k: &Class| -> Vec<Ty> {
            if h == c.name {
                k.type_params.iter().map(|p| Ty::Param(p.name.clone())).collect()
            } else {
                let b = base_args(m, c, h).unwrap_or_default();
                (0..k.type_params.len()).map(|i| b.get(i).cloned().unwrap_or(Ty::Any)).collect()
            }
        };
        let mut primary_covered: HashSet<String> = HashSet::new();
        let mut primary_args: Vec<Ty> = Vec::new();
        let mut primary_kp: Option<&Class> = None;
        if let Some(p) = primary_base
            && let Some(kp) = class_of(p)
        {
            primary_covered.insert(p.clone());
            primary_covered.extend(ancestors(p));
            let b = base_args(m, c, p).unwrap_or_default();
            primary_args = (0..kp.type_params.len()).map(|i| b.get(i).cloned().unwrap_or(Ty::Any)).collect();
            primary_kp = Some(kp);
        }
        let specialized_vs_primary = |h: &str, k: &Class, h_args: &[Ty]| -> bool {
            if h == c.name || !primary_covered.contains(h) || k.type_params.is_empty() {
                return false;
            }
            let Some(kp) = primary_kp else { return false };
            let via_raw = if h == kp.name {
                Some(primary_args.clone())
            } else {
                base_args(m, kp, h).map(|v| v.into_iter().map(|t| subst(&t, &kp.type_params, &primary_args)).collect())
            };
            let Some(via_raw) = via_raw else { return false };
            let via: Vec<Ty> = (0..k.type_params.len()).map(|i| via_raw.get(i).cloned().unwrap_or(Ty::Any)).collect();
            via != h_args
        };
        let mut trait_counts: HashMap<String, usize> = HashMap::new();
        for h in std::iter::once(c.name.as_str()).chain(all.iter().map(String::as_str)) {
            if let Some(tset) = trait_names.get(h) {
                for n in tset {
                    *trait_counts.entry(n.clone()).or_insert(0) += 1;
                }
            }
        }
        let self_ty = if c.type_params.is_empty() {
            Ty::Obj(c.name.clone())
        } else {
            Ty::Generic(c.name.clone(), c.type_params.iter().map(|p| Ty::Param(p.name.clone())).collect())
        };
        // Every name the inherent block may have: a `_with` form keeps clear of them (ProseMirror's `replace_with`).
        let holder_names: HashSet<String> = holders.keys().cloned().collect();
        for (method, hs) in &holders {
            if inherent_taken.contains(method) {
                continue;
            }
            // The nearest holder: the type itself, then its nearest ancestor.
            let h = &hs[0];
            let k = class_of(h).unwrap();
            let h_args = args_for(h, k);
            let collision = trait_counts.get(method).copied().unwrap_or(0) > 1;
            if let Some(g) = grouped[k.name.as_str()].iter().find(|g| g.name == *method) {
                let has_returns_this = h != &c.name && g.members.iter().all(|f| f.returns_this);
                if h != &c.name && primary_covered.contains(h) && !collision && !has_returns_this && !specialized_vs_primary(h, k, &h_args) {
                    continue;
                }
                inherent_taken.insert(method.clone());
                if g.members.iter().any(|f| f.params[g.at].1 == Ty::Str && !hidden(f).is_empty()) {
                    inherent_taken.insert(format!("{method}_as"));
                }
                let ret_self = has_returns_this.then_some(&self_ty);
                for item in g.methods("pub ", imports, this, ret_self, h == &c.name, cx) {
                    inherent += &indented(&item);
                }
                continue;
            }
            let matching: Vec<&Func> = k.funcs.iter().filter(|f| Class::has_self(f) && names[k.name.as_str()][&f.name] == *method).collect();
            let has_returns_this = h != &c.name && matching.iter().any(|f| f.returns_this);
            if h != &c.name && primary_covered.contains(h) && !collision && !has_returns_this && !specialized_vs_primary(h, k, &h_args) {
                continue;
            }
            for f0 in matching {
                let mut orig = with_self_args(f0, k);
                if !c.type_params.is_empty() && !orig.type_params.is_empty() {
                    orig = avoid_outer_type_params(&orig, &c.type_params, &k.type_params);
                }
                let mut f = map_types(&orig, &|t| subst(t, &k.type_params, &h_args));
                if f.returns_this && h != &c.name {
                    f.ret = self_ty.clone();
                }
                // Documented where the trait has it, or the base.
                if h != &c.name {
                    f.doc = None;
                }
                inherent_taken.insert(method.clone());
                let as_name = format!("{method}_as");
                if !hidden(&f).is_empty() && !inherent_taken.contains(&as_name) && !holders.contains_key(&as_name) {
                    inherent_taken.insert(as_name);
                    let here: HashSet<String> = own_names.iter().chain(&inherent_taken).chain(&holder_names).cloned().collect();
                    for item in function_raw(&f, &orig, method, "pub ", imports, Some(this), &Site { assoc: false, taken: &here }, cx) {
                        inherent += &indented(&item);
                    }
                } else {
                    let f_def = if hidden(&f).is_empty() { f } else { defaulted(&f) };
                    let o_def = if hidden(&orig).is_empty() { orig } else { defaulted(&orig) };
                    let here: HashSet<String> = own_names.iter().chain(&inherent_taken).chain(&holder_names).cloned().collect();
                    for item in function_raw(&f_def, &o_def, method, "pub ", imports, Some(this), &Site { assoc: false, taken: &here }, cx) {
                        inherent += &indented(&item);
                    }
                }
            }
        }
        // Inherent `with_<prop>(self, v) -> Self` only for own/non-primary/colliding/specialized setters.
        for (method, hs) in &holders {
            let h = &hs[0];
            let k = class_of(h).unwrap();
            let Some((f0, with_name)) = setter_for_with(k, method, &names) else { continue };
            if inherent_taken.contains(&with_name) || holders.contains_key(&with_name) || TAKEN.contains(&with_name.as_str()) {
                continue;
            }
            let h_args = args_for(h, k);
            let collision = trait_counts.get(&with_name).copied().unwrap_or(0) > 1;
            if h != &c.name && primary_covered.contains(h) && !collision && !specialized_vs_primary(h, k, &h_args) {
                continue;
            }
            let orig = with_self_args(f0, k);
            let f = map_types(&orig, &|t| subst(t, &k.type_params, &h_args));
            if let Some(item) = with_setter_item(&f, &orig, &with_name, "pub ", "", imports, cx) {
                inherent_taken.insert(with_name);
                inherent += &item;
            }
        }
        if !inherent_taken.contains("iter")
            && let Some((k, elem)) = std::iter::once(c.name.as_str()).chain(all.iter().map(String::as_str)).find_map(|h| class_of(h).and_then(|k| k.iterates.as_ref().map(|e| (k, e))))
        {
            let h_args = args_for(&k.name, k);
            let collision = trait_counts.get("iter").copied().unwrap_or(0) > 1;
            if k.name == c.name || !primary_covered.contains(&k.name) || collision || specialized_vs_primary(&k.name, k, &h_args) {
                let elem = with_self_args(&Func { type_params: vec![], module: String::new(), import: String::new(), name: String::new(), call: Call::Create(String::new()), params: vec![], rest: false, fixed: vec![], shared: None, ret: elem.clone(), returns_this: false, doc: None }, k).ret;
                let elem = subst(&elem, &k.type_params, &h_args);
                inherent_taken.insert("iter".into());
                inherent += &format!("    pub fn iter(&self) -> JsIterator<{}> {{ JsIterator::over({this}) }}\n", ty(&elem, cx));
            }
        }
        if !inherent_taken.contains("iterate_async")
            && let Some((k, elem)) = std::iter::once(c.name.as_str()).chain(all.iter().map(String::as_str)).find_map(|h| class_of(h).and_then(|k| k.iterates_async.as_ref().map(|e| (k, e))))
        {
            let h_args = args_for(&k.name, k);
            let collision = trait_counts.get("iterate_async").copied().unwrap_or(0) > 1;
            if k.name == c.name || !primary_covered.contains(&k.name) || collision || specialized_vs_primary(&k.name, k, &h_args) {
                let elem = with_self_args(&Func { type_params: vec![], module: String::new(), import: String::new(), name: String::new(), call: Call::Create(String::new()), params: vec![], rest: false, fixed: vec![], shared: None, ret: elem.clone(), returns_this: false, doc: None }, k).ret;
                let elem = subst(&elem, &k.type_params, &h_args);
                inherent_taken.insert("iterate_async".into());
                inherent += &format!("    pub fn iterate_async(&self) -> JsAsyncIterator<{}> {{ JsAsyncIterator::over({this}) }}\n", ty(&elem, cx));
            }
        }
        // Covariant upcast for generic handles: `Texture<BufferImageSource>::upcast::<TextureSource<JsValue>>()`.
        if !c.type_params.is_empty() && !inherent_taken.contains("upcast") {
            let u_names: Vec<String> = (0..c.type_params.len()).map(|i| format!("U{i}_")).collect();
            let u_bounds: Vec<String> = u_names.iter().map(|u| format!("{u}: JsType")).collect();
            let wheres: Vec<String> = c.type_params.iter().zip(&u_names).map(|(p, u)| format!("{}: Is<{u}>", p.name)).collect();
            let u_args = u_names.join(", ");
            inherent_taken.insert("upcast".into());
            inherent += &format!(
                "    pub fn upcast<{}>(&self) -> {name}<{u_args}> where {} {{ {name}(self.0.clone(), core::marker::PhantomData) }}\n",
                u_bounds.join(", "),
                wheres.join(", ")
            );
        }
        // Checked downcast: only a class has a constructor to test against.
        if c.js.is_some() {
            let cast_name = if inherent_taken.contains("cast") { "try_cast" } else { "cast" };
            let is_imp = format!("js_is__{}", c.name);
            imports.entry(is_imp.clone()).or_insert_with(|| format!("    pub fn {is_imp}(h: u32) -> i32;\n"));
            let opt = cx.option();
            inherent_taken.insert(cast_name.into());
            inherent += &format!(
                "    pub fn {cast_name}(v: &impl AsRef<JsHandle>) -> {opt}<Self> {{ let h = v.as_ref().0; if h != 0 && unsafe {{ raw::{is_imp}(h) }} != 0 {{ let c = v.as_ref().clone(); core::mem::forget(c); Some(<Self>::from_handle(h)) }} else {{ None }} }}\n"
            );
        }
        if !inherent.is_empty() {
            *out += &format!("impl{bounds} {me} {{\n{inherent}}}\n");
        }
        // The traits of its ancestors, and conversions to them.
        for a in &all {
            let Some(k) = class_of(a) else {
                if m.opaque.contains(a) && !RUNTIME_TYPES.iter().any(|r| r.0 == a.as_str()) {
                    if let Some((_, _, is_handle)) = resolved_opaque.get(a) {
                        if *is_handle {
                            *out += &upcast_in(&bounds, &me, &cx.tn(a), true);
                        }
                    } else {
                        let arity = m.opaque_arity.get(a).copied().unwrap_or(0);
                        let base_args_str = if arity == 0 {
                            String::new()
                        } else {
                            let base = base_args(m, c, a).unwrap_or_default();
                            format!("<{}>", (0..arity).map(|i| base.get(i).map(|t| ty(t, cx)).unwrap_or_else(|| VALUE.into())).collect::<Vec<_>>().join(", "))
                        };
                        *out += &upcast_in(&bounds, &me, &format!("{}{base_args_str}", cx.tn(a)), false);
                    }
                }
                continue;
            };
            let base: Vec<Ty> = base_args(m, c, a).unwrap_or_default();
            let base_args_str = if k.type_params.is_empty() { String::new() } else { format!("<{}>", (0..k.type_params.len()).map(|i| base.get(i).map(|t| ty(t, cx)).unwrap_or_else(|| VALUE.into())).collect::<Vec<_>>().join(", ")) };
            *out += &format!("impl{bounds} {}Methods{base_args_str} for {me} {{}}\n", cx.tn(a));
            // Generic or not: `Marker<P>` is a `Layer`.
            *out += &upcast_in(&bounds, &me, &format!("{}{base_args_str}", cx.tn(a)), false);
            // Covariant default-instantiated base for concrete classes: `Rect` (`Shape<RectConfig>`) -> `Shape<ShapeConfig>`,
            // `BufferImageSource` (`TextureSource<...>`) -> `TextureSource<JsValue>`.
            if c.type_params.is_empty() && !k.type_params.is_empty() {
                let def_tys: Vec<Ty> = k.type_params.iter().map(|p| p.default.clone().unwrap_or(Ty::Any)).collect();
                let def_str = format!("<{}>", def_tys.iter().map(|t| ty(t, cx)).collect::<Vec<_>>().join(", "));
                let actual: Vec<Ty> = (0..k.type_params.len()).map(|i| base.get(i).cloned().unwrap_or(Ty::Any)).collect();
                if def_str != base_args_str && actual.iter().zip(&def_tys).all(|(x, y)| is_subtype(m, x, y, 0)) {
                    *out += &upcast_in(&bounds, &me, &format!("{}{def_str}", cx.tn(a)), false);
                }
            }
        }
    }
}

// A union's variant: `Number`, `String`, `Element`, `Array`, `TupleN`, or a type parameter.
fn variant(t: &Ty, cx: &Context, multi_tuple: bool) -> String {
    match t {
        Ty::Num => "Number".into(),
        Ty::Str => "String".into(),
        Ty::Bool => "Boolean".into(),
        // A namespace's type by its own name: `ExportValue::Global`.
        Ty::Obj(n) | Ty::Generic(n, _) | Ty::Enum(n) => cx.def_name(n),
        Ty::Param(n) => n.clone(),
        Ty::Tuple(es) if multi_tuple => format!("Tuple{}", es.len()),
        Ty::Array | Ty::ArrayOf(_) | Ty::Tuple(_) => "Array".into(),
        Ty::Record(_) => "Record".into(),
        t => panic!("Rust generator: a union member it cannot hold: {t:?}"),
    }
}

// A union as an enum: decoded from a handle by its tag, lent as its member,
// and made from any member: `describe(42.0)`. One whose members cannot be told
// apart is only made and lent, with a variant for any value.
pub(super) fn union_enum(u: &NamedUnion, m: &Model, imports: &mut BTreeMap<String, String>, cx: &Context) -> String {
    let n = cx.def_name(&u.name);
    let opt = cx.option();
    let import = &u.name;
    let decodable = u.decodable();
    let (bounds, args) = (generics(&u.type_params, cx), args_of(&u.type_params));
    // Each member's variant: by its type's own name, by its whole path
    // where two would be one (`stream::Readable` beside a `Readable`).
    let multi_tuple = u.parts.iter().filter(|q| matches!(q, Ty::Tuple(_))).count() > 1;
    let names: Vec<String> = u.parts.iter().map(|p| variant(p, cx, multi_tuple)).collect();
    let names: Vec<String> = u
        .parts
        .iter()
        .zip(&names)
        .map(|(p, v)| match p {
            Ty::Obj(n) | Ty::Generic(n, _) | Ty::Enum(n) if names.iter().filter(|w| *w == v).count() > 1 => cx.tn(n).trim_start_matches("crate::").split("::").map(camel).collect(),
            _ => v.clone(),
        })
        .collect();
    let variant = |p: &Ty| names[u.parts.iter().position(|q| q == p).unwrap()].clone();
    let mut variants: Vec<String> = u.parts.iter().map(|p| format!("{}({})", variant(p), ty(p, cx))).collect();
    let mut lend: String = u.parts.iter().map(|p| format!("{n}::{}(x) => x.lend(), ", variant(p))).collect();
    let mut clone: String = u.parts.iter().map(|p| format!("{n}::{0}(x) => {n}::{0}(Clone::clone(x)), ", variant(p))).collect();
    let mut debug_arms: String = u.parts.iter().map(|p| format!("{n}::{}(x) => f.debug_tuple({:?}).field(x).finish(), ", variant(p), variant(p))).collect();
    if !decodable {
        variants.push(format!("{VALUE}({VALUE})"));
        lend += &format!("{n}::{VALUE}(x) => x.lend(), ");
        clone += &format!("{n}::{VALUE}(x) => {n}::{VALUE}(Clone::clone(x)), ");
        debug_arms += &format!("{n}::{VALUE}(x) => f.debug_tuple({VALUE:?}).field(x).finish(), ");
    }
    let debug_where = if u.type_params.is_empty() {
        String::new()
    } else {
        format!(" where {}", u.type_params.iter().map(|p| format!("{}: core::fmt::Debug", p.name)).collect::<Vec<_>>().join(", "))
    };
    let mut out = format!(
        "pub enum {n}{bounds} {{ {} }}\n\
         impl{bounds} Clone for {n}{args} {{ fn clone(&self) -> Self {{ match self {{ {clone}}} }} }}\n\
         impl{bounds} ToJs for {n}{args} {{ fn lend(&self) -> Lent {{ match self {{ {lend}}} }} }}\n\
         impl{bounds} Optional<{n}{args}> for {opt}<{n}{args}> {{ fn into_option(self) -> {opt}<{n}{args}> {{ self }} }}\n\
         impl{bounds} Optional<{n}{args}> for JsNone {{ fn into_option(self) -> {opt}<{n}{args}> {{ None }} }}\n\
         impl{bounds} core::fmt::Debug for {n}{args}{debug_where} {{ fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {{ match self {{ {debug_arms}}} }} }}\n",
        variants.join(", ")
    );
    // A field every member has, read without a match (marked's `token.type_()`).
    let fields = if u.type_params.is_empty() { m.shared_fields(u) } else { Vec::new() };
    if !fields.is_empty() {
        let getters: String = fields.iter().map(|(p, t)| format!("    pub fn {}(&self) -> {} {{ js::field(self, {p:?}) }}\n", super::ident(&super::super::case::snake_case(p)), ty(t, cx))).collect();
        out += &format!("impl {n} {{\n{getters}}}\n");
    }
    if decodable {
        // A bare type parameter is told apart by what its argument is.
        let kind = u.parts.iter().find_map(|p| if let Ty::Param(t) = p { Some(format!("{t}::KIND")) } else { None }).unwrap_or("0".into());
        let decode: String = u.parts.iter().enumerate().map(|(i, p)| format!("{i} => {n}::{}(<{}>::from_handle(h)), ", variant(p), ty(p, cx))).collect();
        imports.insert(format!("{import}_tag"), format!("    pub fn {import}_tag(h: u32, k: i32) -> i32;\n"));
        // A union is what each of its members is.
        let is_bounds: Vec<String> = u.parts.iter().map(|p| format!("{}: Is<B_>", ty(p, cx))).collect();
        let is_params = std::iter::once("B_".to_string()).chain(u.type_params.iter().map(|p| bound(p, cx))).collect::<Vec<_>>().join(", ");
        out += &format!(
            "impl{bounds} FromJs for {n}{args} {{ fn from_handle(h: u32) -> Self {{ match unsafe {{ raw::{import}_tag(h, {kind}) }} {{ {decode}_ => unreachable!() }} }} }}\n\
             impl<{is_params}> Is<B_> for {n}{args} where {} {{}}\n",
            is_bounds.join(", ")
        );
    } else {
        out += &format!("impl From<{VALUE}> for {n} {{ fn from(x: {VALUE}) -> Self {{ {n}::{VALUE}(x) }} }}\nimpl From<&{VALUE}> for {n} {{ fn from(x: &{VALUE}) -> Self {{ {n}::{VALUE}(Clone::clone(x)) }} }}\n");
    }
    // Made from a member. A bare type parameter cannot be: it could be the union itself.
    let single_record = u.parts.iter().filter(|q| matches!(q, Ty::Record(_))).count() == 1;
    for p in u.parts.iter().filter(|p| !matches!(p, Ty::Param(_))) {
        let (v, t) = (variant(p), ty(p, cx));
        out += &format!("impl{bounds} From<{t}> for {n}{args} {{ fn from(x: {t}) -> Self {{ {n}::{v}(x) }} }}\n");
        match p {
            Ty::Str => out += &format!(
                "impl{bounds} From<&str> for {n}{args} {{ fn from(x: &str) -> Self {{ {n}::{v}(x.to_string()) }} }}\n\
                 impl{bounds} From<&::std::string::String> for {n}{args} {{ fn from(x: &::std::string::String) -> Self {{ {n}::{v}(x.clone()) }} }}\n"
            ),
            Ty::Record(inner) if single_record => {
                let vt = ty(inner, cx);
                let extra_bounds = |tail: String| {
                    if u.type_params.is_empty() {
                        format!("<{tail}>")
                    } else {
                        let bs: Vec<String> = u.type_params.iter().map(|tp| bound(tp, cx)).collect();
                        format!("<{}, {tail}>", bs.join(", "))
                    }
                };
                let where_js = if u.type_params.is_empty() { String::new() } else { format!(" where {vt}: JsType") };
                let b_arr = extra_bounds(format!("K_: AsRef<str>, V_: Into<{vt}>, const N_: usize"));
                let b_vec = extra_bounds(format!("K_: AsRef<str>, V_: Into<{vt}>"));
                let b_slice = extra_bounds(format!("K_: AsRef<str> + Clone, V_: Into<{vt}> + Clone"));
                out += &format!(
                    "impl{bounds} From<&{t}> for {n}{args} {{ fn from(x: &{t}) -> Self {{ {n}::{v}(Clone::clone(x)) }} }}\n\
                     impl{b_arr} From<[(K_, V_); N_]> for {n}{args}{where_js} {{ fn from(x: [(K_, V_); N_]) -> Self {{ {n}::{v}(x.into()) }} }}\n\
                     impl{b_vec} From<::std::vec::Vec<(K_, V_)>> for {n}{args}{where_js} {{ fn from(x: ::std::vec::Vec<(K_, V_)>) -> Self {{ {n}::{v}(x.into()) }} }}\n\
                     impl{b_slice} From<&[(K_, V_)]> for {n}{args}{where_js} {{ fn from(x: &[(K_, V_)]) -> Self {{ {n}::{v}(x.into()) }} }}\n"
                );
            }
            Ty::Num | Ty::Bool | Ty::Enum(_) => {}
            _ => out += &format!("impl{bounds} From<&{t}> for {n}{args} {{ fn from(x: &{t}) -> Self {{ {n}::{v}(Clone::clone(x)) }} }}\n"),
        }
    }
    // Made from concrete subtypes of an arm when unambiguous and small (e.g. Konva's `Rect` -> `GroupOrShape`).
    if u.type_params.is_empty() {
        let exact: HashSet<String> = u.parts.iter().map(|p| ty(p, cx)).collect();
        let mut sub_impls: Vec<(String, String, String)> = Vec::new();
        for c in &m.classes {
            if !c.type_params.is_empty() {
                continue;
            }
            let c_ty = Ty::Obj(c.name.clone());
            let c_rust = ty(&c_ty, cx);
            if exact.contains(&c_rust) {
                continue;
            }
            let matches: Vec<&Ty> = u.parts.iter().filter(|p| matches!(p, Ty::Obj(_) | Ty::Generic(..)) && is_subtype(m, &c_ty, p, 0)).collect();
            // Which of several does not matter where no variant is decoded.
            if matches.len() == 1 || !decodable && !matches.is_empty() {
                let p = matches[0];
                sub_impls.push((c_rust, variant(p), ty(p, cx)));
            }
        }
        if sub_impls.len() <= 40 {
            for (sub, v, target) in sub_impls {
                out += &format!(
                    "impl From<{sub}> for {n} {{ fn from(x: {sub}) -> Self {{ let h = x.0 .0; core::mem::forget(x); {n}::{v}(<{target}>::from_handle(h)) }} }}\n\
                     impl From<&{sub}> for {n} {{ fn from(x: &{sub}) -> Self {{ {n}::{v}(<{target}>::from_handle(crate::runtime::give(x))) }} }}\n"
                );
            }
        }
    }
    out
}

// A callback type, made from a closure: `CounterVisitF::new(|c: Counter| ..)`,
// or for the length of a call, `CounterVisitF::scoped(|c| .., |f| counter.visit(f))`.
pub(super) fn callback(name: &str, type_params: &[TParam], params: &[(String, Ty)], ret: &Ty, apply: Option<&Func>, imports: &mut BTreeMap<String, String>, cx: &Context) -> String {
    let import = name;
    let name = cx.def_name(name);
    let (bounds, args) = (generics(type_params, cx), args_of(type_params));
    let tps: Vec<String> = type_params.iter().map(|p| p.name.clone()).collect();
    let arg_types: Vec<String> = params.iter().map(|(_, t)| ty(t, cx)).collect();
    let fn_bound = if *ret == Ty::Void { format!("FnMut({})", arg_types.join(", ")) } else { format!("FnMut({}) -> {}", arg_types.join(", "), ty(ret, cx)) };
    let raw_args: String = params.iter().enumerate().map(|(i, (_, t))| format!(", a{i}: {}", raw_ret(t))).collect();
    let call_args: Vec<String> = params.iter().enumerate().map(|(i, (_, t))| raise(t, &format!("a{i}"), cx)).collect();
    let raw_result = if *ret == Ty::Void { String::new() } else { format!(" -> {}", raw_ret(ret)) };
    // The same trampoline twice: for a closure that is kept, and for one that is borrowed.
    let trampoline = |f: &str, lifetime: &str| {
        let mut generic: Vec<String> = type_params.iter().map(|p| bound(p, cx)).collect();
        generic.push(format!("F: {fn_bound}{lifetime}"));
        format!("extern \"C\" fn call<{}>(ctx: *mut core::ffi::c_void{raw_args}){raw_result} {{ {} }}", generic.join(", "), lower_callback(ret, &format!("(&mut *{f})({})", call_args.join(", "))))
    };
    let turbofish: Vec<String> = tps.iter().cloned().chain(std::iter::once("F".to_string())).collect();
    imports.insert(format!("{import}_from"), format!("    pub fn {import}_from(f: usize, ctx: *mut c_void) -> u32;\n"));
    let handle_params = if tps.is_empty() { String::new() } else { format!("<{}>", tps.join(", ")) };
    let is_self = if tps.is_empty() { format!("impl Is<{name}> for {name} {{}}\n") } else { String::new() };
    let call_method = if let Some(apply) = apply.filter(|a| !a.params.is_empty()) {
        let method_f = Func {
            type_params: Vec::new(),
            module: apply.module.clone(),
            import: apply.import.clone(),
            name: "call".into(),
            call: Call::Method("call".into()),
            params: apply.params[1..].to_vec(),
            rest: apply.rest,
            fixed: apply.fixed.clone(),
            shared: apply.shared.clone(),
            ret: apply.ret.clone(),
            returns_this: false,
            doc: None,
        };
        format!("\n{}", indented(&one_function(&method_f, "call", "pub ", imports, Some("self.0 .0"), cx)).trim_end_matches('\n'))
    } else {
        String::new()
    };
    format!(
        "handle!({name}{handle_params});\n{is_self}impl{bounds} JsCallback for {name}{args} {{}}\n\
         impl{bounds} {name}{args} {{\n    \
         pub fn new<F: {fn_bound} + 'static>(f: F) -> Self {{\n        {}\n        \
         Self::from_handle(unsafe {{ raw::{import}_from(call::<{2}> as *const () as usize, crate::runtime::closure(f)) }})\n    }}\n    \
         // For the length of `body`: its closure may borrow.\n    \
         pub fn scoped<F: {fn_bound}, R_>(f: F, body: impl FnOnce(&Self) -> R_) -> R_ {{\n        {}\n        \
         let f = core::cell::RefCell::new(f);\n        \
         let made = Self::from_handle(unsafe {{ raw::{import}_from(call::<{2}> as *const () as usize, &f as *const core::cell::RefCell<F> as *mut core::ffi::c_void) }});\n        \
         let _forget = crate::runtime::Forget(made.0 .0);\n        body(&made)\n    }}{call_method}\n}}\n",
        trampoline("unsafe { crate::runtime::closure_fn::<F>(ctx) }", " + 'static"),
        trampoline("unsafe { &*(ctx as *const core::cell::RefCell<F>) }.borrow_mut()", ""),
        turbofish.join(", ")
    )
}

// The impl a static goes in: the class's parameters it does not take, which
// have defaults, are those (`impl<G, M> Mesh<G, M, Object3DEventMap>`); its
// bounds, its type, and the static with them put in.
fn defaulted_impl(c: &Class, name: &str, f: &Func, cx: &Context) -> (String, String, Func) {
    let mut args: Vec<Ty> = c.type_params.iter().map(|p| Ty::Param(p.name.clone())).collect();
    let taken = |p: &TParam| f.params.iter().any(|(_, t)| mentions(t, &p.name)) || f.type_params.iter().any(|q| q.constraint.as_ref().is_some_and(|k| mentions(k, &p.name)));
    let mut fixed = vec![false; args.len()];
    for (i, p) in c.type_params.iter().enumerate() {
        if let Some(d) = p.default.as_ref().filter(|_| !taken(p)) {
            args[i] = subst(d, &c.type_params, &args);
            fixed[i] = true;
        }
    }
    // A default naming a parameter still open keeps it generic: nothing to fix.
    if !fixed.iter().any(|&x| x) || args.iter().zip(&fixed).any(|(a, &x)| x && c.type_params.iter().zip(&fixed).any(|(p, &y)| !y && mentions(a, &p.name))) {
        return (generics(&c.type_params, cx), format!("{name}{}", args_of(&c.type_params)), f.clone());
    }
    let open: Vec<TParam> = c.type_params.iter().zip(&fixed).filter(|(_, x)| !**x).map(|(p, _)| p.clone()).collect();
    let me = format!("{name}<{}>", args.iter().map(|a| ty(a, cx)).collect::<Vec<_>>().join(", "));
    (generics(&open, cx), me, map_types(f, &|t| subst(t, &c.type_params, &args)))
}
