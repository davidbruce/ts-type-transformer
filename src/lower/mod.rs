// Lowering passes. Each removes one TypeScript construct from the IR, for
// targets that cannot express it. Passes that need semantics ask the checker
// through `Front`; the rest are plain IR rewrites.

mod classes;
mod eval;
mod generics;
mod hoist;
mod names;
mod overloads;
mod unions;
mod walk;

pub use generics::keyed_map;
pub(crate) use generics::{SYNTH_NAME_MAX, trim_wrapper_suffixes};
pub use names::{pascal_types, rename, rename_everywhere};
pub use walk::visit;

use std::collections::{HashMap, HashSet};

use crate::front::Front;
use crate::ir::*;

use classes::{flatten, mark_class_refs, pair_classes, structural_extends};
use eval::{drop_unreachable_overloads, evaluate, expand_aliases, expand_keyed, inline_object_bases, nominal_constraints, split_literal_overloads, splice_union_aliases};
use generics::{erase_generics, literals, nullable};
use hoist::{cut_recursive_dictionary, hoist};
use overloads::{expand_functions, expand_members, mark_overloads, rename_functions, rename_members};
use unions::{absorb_enums, annotate_unions, collect_declared_names, flatten_unions, merge_array_members, name_unions, written_unions};
use walk::{each_member_list, walk, walk_signature};

// What to lower. Built from a config file; see config.rs.
#[derive(Clone, Copy)]
pub struct Options {
    pub classes: bool,
    pub typeops: bool,
    pub intersections: bool,
    pub inheritance: bool,
    pub generics: bool,
    // Only type parameters TypeScript's checking alone uses (`generics = "checking"`).
    pub checking_generics: bool,
    pub aliases: bool,
    pub constraints: bool,
    // A type parameter meets a constraint only with that one (`constraints = "exact"`).
    pub exact_constraints: bool,
    pub nullable: bool,
    pub literals: bool,
    pub anonymous: bool,
    pub optionals: bool,
    pub param_unions: bool,
    // Of union parameters, only those with a member of no type of its own.
    pub untyped_param_unions: bool,
    pub unions: bool,
    pub overloads: bool,
    pub keyed: bool,
    pub redeclared: bool,
    // Keep keyed signatures generic, with their map (`keyed_params = "map"`).
    pub map_keyed: bool,
    // Unions become named declarations (`unions = "name"`).
    pub name_unions: bool,
    // Most keys a keyed signature is expanded over.
    pub keyed_limit: usize,
    // Most overloads one signature may expand into through union parameters.
    pub union_param_limit: usize,
    // Largest merged intersection that is written out inline.
    pub intersection_limit: usize,
    // Joins the parts of generated names.
    pub separator: &'static str,
    // Overload names: `f_2`, `f_3` instead of `f_with_x`.
    pub numbered_overloads: bool,
    // Generated type names in PascalCase: `BooleanOrAddEventListenerOptions`.
    pub pascal_types: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            classes: false, typeops: false, intersections: false, inheritance: false, generics: false, checking_generics: false, aliases: false, constraints: false, exact_constraints: false, nullable: false,
            literals: false, anonymous: false, optionals: false, param_unions: false, untyped_param_unions: false, unions: false, overloads: false, keyed: false, redeclared: false, map_keyed: false, name_unions: false, keyed_limit: 256,
            union_param_limit: 16, intersection_limit: 64, separator: "_", numbered_overloads: false, pascal_types: false,
        }
    }
}

impl Options {
    // Whether the construct with this config key is lowered.
    pub fn slot(&mut self, key: &str) -> Option<&mut bool> {
        Some(match key {
            "classes" => &mut self.classes,
            "type_ops" => &mut self.typeops,
            "intersections" => &mut self.intersections,
            "inheritance" => &mut self.inheritance,
            "generics" => &mut self.generics,
            "aliases" => &mut self.aliases,
            "constraints" => &mut self.constraints,
            "nullable" => &mut self.nullable,
            "literals" => &mut self.literals,
            "anonymous" => &mut self.anonymous,
            "optional_params" => &mut self.optionals,
            "union_params" => &mut self.param_unions,
            "unions" => &mut self.unions,
            "overloads" => &mut self.overloads,
            "keyed_params" => &mut self.keyed,
            "redeclarations" => &mut self.redeclared,
            _ => return None,
        })
    }
}

pub fn lower(front: &mut Front, decls: &mut Vec<Decl>, o: Options) {
    front.merge_intersections = o.intersections;
    front.intersection_limit = o.intersection_limit;
    front.expand_keyed = o.keyed;
    front.map_keyed = o.map_keyed;
    front.keyed_limit = o.keyed_limit;
    if o.keyed || o.map_keyed {
        split_literal_overloads(decls, o.keyed_limit);
    }
    let mut written = if o.name_unions { written_unions(decls) } else { HashSet::new() };
    callable_properties(decls);
    // Before widening and erasing lose what marks them.
    mark_overloads(decls);
    // An inherited member serves every subtype, where a target has subtypes.
    if o.redeclared && !o.inheritance {
        let dropped = drop_redeclared(decls);
        *front.stats.entry("dropped: member redeclared as inherited".into()).or_default() += dropped;
    }
    // `keyed_params = "map"` keeps a map these bindings declare; one they do
    // not (the DOM's tag map, for a library) has no keys here: expanded.
    if o.keyed || o.map_keyed {
        expand_keyed(front, decls, o);
    }
    if o.classes {
        pair_classes(decls);
    }
    if o.inheritance {
        flatten(front, decls);
        // Members are the checker's again.
        callable_properties(decls);
    }
    if o.classes {
        structural_extends(decls, o.inheritance);
    }
    if o.keyed || o.map_keyed {
        drop_unreachable_overloads(decls);
    }
    if o.typeops || o.intersections {
        evaluate(front, decls, o);
        flatten_unions(decls);
        inline_object_bases(front, decls);
        // And the overloads evaluating made (`typeof merge`).
        mark_overloads(decls);
    }
    // And those `typeof` made (marked's `let parseInline: typeof markedInstance.parseInline`).
    callable_properties(decls);
    literal_option_overloads(decls, o.separator);
    // `(...args: [style?: StrokeInput])` is `(style?: StrokeInput)`, as
    // TypeScript has it: pixi's `stroke(...args: Parameters<GraphicsContext['stroke']>)`.
    generics::each_signature(decls, &mut spread_tuple_rest);
    if o.aliases {
        expand_aliases(front, decls);
    }
    if o.generics {
        erase_generics(decls, o.map_keyed, o.checking_generics);
    }
    if o.nullable {
        walk(decls, &mut |t, _| {
            nullable(t);
            true
        });
    }
    if o.literals {
        let widened = literals(decls, o.separator);
        *front.stats.entry("widened: literal -> primitive".into()).or_default() += widened;
    }
    // Whether hoisted or kept inline.
    walk(decls, &mut |t, _| {
        cut_recursive_dictionary(t);
        true
    });
    if o.anonymous {
        hoist(decls, o.separator);
    }
    if o.optionals || o.param_unions {
        let mut unions = HashSet::new();
        overloads::union_aliases(decls, &mut unions);
        each_member_list(decls, &mut |members| expand_members(members, o, &unions));
        expand_functions(decls, o, &unions);
    }
    if o.unions {
        walk(decls, &mut |t, _| {
            match t {
                Type::Union(parts) => match front.common_supertype(parts) {
                    Some(sup) => {
                        front.count("widened: union -> common supertype");
                        *t = sup;
                    }
                    None => {
                        front.count("erased: union");
                        *t = Type::Erased("union");
                    }
                },
                Type::Intersection(..) => {
                    front.count("erased: intersection");
                    *t = Type::Erased("intersection");
                }
                _ => {}
            }
            true
        });
    }
    if o.name_unions {
        // Merging arrays can bring a union alias next to its own members: a few rounds settle it.
        for _ in 0..3 {
            splice_union_aliases(decls);
            flatten_unions(decls);
            merge_array_members(decls);
        }
        absorb_enums(decls);
        written.extend(front.written_members.drain());
        name_unions(decls, o.separator, &written);
    }
    if o.overloads {
        let mut synth_enums = HashMap::new();
        collect_synth_enum_firsts(decls, "", o.separator, &mut synth_enums);
        each_member_list(decls, &mut |members| rename_members(members, o, &synth_enums));
        // A renamed overload is of its package (leaflet's `circle_with_radius`).
        for (old, new) in rename_functions(decls, o, &synth_enums) {
            if let Some(p) = front.package_of_name.get(&old).cloned() {
                front.package_of_name.insert(new.clone(), p);
            }
            if let Some(origin) = front.origin_of_name.get(&old).cloned() {
                front.origin_of_name.insert(new, origin);
            }
        }
    }
    if o.classes {
        mark_class_refs(decls);
    }
    if o.constraints {
        nominal_constraints(front, decls, o.exact_constraints);
    }
    prune_bundled(decls);
    readonly_globals(decls);
    annotate_unions(front, decls);
}

// A global `var` that the global object has read-only is read-only:
// lib.dom's `declare var document: Document` is `Window`'s `readonly
// document`, the global object's interface being what `window` or `self` is
// (`declare var self: Window & typeof globalThis`), with what it extends.
// A namespace's variables are its object's, read-only (`CSS.highlights`).
// A class's `prototype` is read-only.
fn readonly_globals(decls: &mut [Decl]) {
    let interfaces: HashMap<String, &Interface> = decls.iter().filter_map(|d| if let Decl::Interface(i) = d { Some((i.name.clone(), i)) } else { None }).collect();
    let mut scopes: Vec<String> = Vec::new();
    for d in decls.iter() {
        if let Decl::Var(v) = d
            && matches!(v.name.as_str(), "window" | "self")
        {
            let mut t = v.ty.clone();
            visit(&mut t, &mut |x| {
                if let Type::Ref(r) = x
                    && interfaces.contains_key(&r.name)
                {
                    scopes.push(r.name.clone());
                }
                true
            });
        }
    }
    let mut readonly: HashSet<String> = HashSet::new();
    let mut seen: HashSet<String> = HashSet::new();
    while let Some(name) = scopes.pop() {
        let Some(i) = interfaces.get(&name).filter(|_| seen.insert(name.clone())) else { continue };
        readonly.extend(i.members.iter().filter(|m| m.kind == MemberKind::Property && m.readonly).map(|m| m.name.clone()));
        scopes.extend(i.ancestors.iter().cloned());
        for e in &i.extends {
            if let Type::Ref(r) = e {
                scopes.push(r.name.clone());
            }
        }
    }
    for d in decls.iter_mut() {
        // A class's static side has its `prototype` read-only (`declare var
        // SVGMatrix: { prototype: DOMMatrix; new(): DOMMatrix }`).
        if let Decl::Var(v) = d
            && let Type::Object(members) = &mut v.ty
        {
            members.iter_mut().filter(|m| m.name == "prototype" && m.kind == MemberKind::Property).for_each(|m| m.readonly = true);
        }
        match d {
            Decl::Var(v) if readonly.contains(&v.name) => v.readonly = true,
            Decl::Namespace(n) => {
                for d in n.decls.iter_mut() {
                    if let Decl::Var(v) = d {
                        v.readonly = true;
                    }
                }
            }
            _ => {}
        }
    }
}

// A declaration a bundler renamed (rollup's `BarSeriesOption$1`), once what
// it was for has its members (the export, merged with it), is gone; so are
// the types made of its own, once only it had them.
fn prune_bundled(decls: &mut Vec<Decl>) {
    let mut gone: Vec<String> = Vec::new();
    loop {
        let mut used: HashSet<String> = HashSet::new();
        walk(decls, &mut |t, _| {
            if let Type::Ref(r) = t {
                used.insert(r.name.clone());
            }
            true
        });
        for d in decls.iter() {
            if let Decl::Interface(i) = d {
                used.extend(i.ancestors.iter().cloned());
            }
        }
        let before = decls.len();
        decls.retain(|d| {
            let (name, synthetic) = match d {
                Decl::Interface(i) => (&i.name, i.synthetic),
                Decl::Alias(a) => (&a.name, a.synthetic),
                _ => return true,
            };
            let bundled = name.contains('$') && !synthetic;
            let made_of_gone = synthetic && gone.iter().any(|g| name.starts_with(&format!("{g}_")));
            if used.contains(name) || !(bundled || made_of_gone) {
                return true;
            }
            if bundled {
                gone.push(name.clone());
            }
            false
        });
        if decls.len() == before {
            break;
        }
    }
    // One still used has its own name if no other has it: `NullUndefined$1`
    // is `NullUndefined`, and `string_or_LabelFormatter$1` `string_or_LabelFormatter`.
    let mut taken: HashSet<String> = HashSet::new();
    collect_declared_names(decls, "", "_", &mut taken);
    let bundled: Vec<String> = decls
        .iter()
        .filter_map(|d| match d {
            Decl::Interface(i) if !i.synthetic => Some(&i.name),
            Decl::Alias(a) if !a.synthetic => Some(&a.name),
            _ => None,
        })
        .filter(|n| n.contains('$'))
        .cloned()
        .collect();
    let referrers = collect_bundled_referrers(decls, &bundled);
    let mut bundled_map: HashMap<String, String> = HashMap::new();
    let mut renames: Vec<(String, String)> = Vec::new();
    for b in &bundled {
        let bare = b.split('$').next().unwrap_or(b);
        let candidate = if !bare.contains('$') && !taken.contains(bare) {
            Some(bare.to_string())
        } else {
            referrers
                .get(b)
                .and_then(|refs| disambiguate_bundled(refs, bare, &taken))
        };
        if let Some(new) = candidate
            && new != *b
            && !new.contains('$')
            && taken.insert(new.clone())
        {
            bundled_map.insert(b.clone(), new.clone());
            renames.push((b.clone(), new));
        }
    }
    let mut replacements: Vec<(&str, &str)> = bundled_map.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    // Replace longer bundled names first so `Foo$1` cannot partially match `Foo$10`.
    replacements.sort_by(|a, b| b.0.len().cmp(&a.0.len()).then_with(|| a.0.cmp(b.0)));
    let mut names: Vec<String> = Vec::new();
    collect_names(decls, &mut names);
    for n in names.into_iter().filter(|n| n.contains('$') && !bundled_map.contains_key(n)) {
        let mut new = n.clone();
        for (old_b, new_b) in &replacements {
            new = new.replace(*old_b, *new_b);
        }
        if new != n && !new.contains('$') && taken.insert(new.clone()) {
            renames.push((n, new));
        }
    }
    rename_everywhere(decls, &renames);
}

fn collect_synth_enum_firsts(decls: &[Decl], prefix: &str, sep: &str, out: &mut HashMap<String, String>) {
    for d in decls {
        match d {
            Decl::Enum(e)
                if e.synthetic
                    && !sep.is_empty()
                    && e.name.contains(sep)
                    && !e.members.is_empty()
                    && e.members.iter().all(|m| m.kind == "string") =>
            {
                let first = overloads::name_part(&e.members[0].value);
                out.insert(e.name.clone(), first.clone());
                if !prefix.is_empty() {
                    out.insert(format!("{prefix}.{}", e.name), first);
                }
            }
            Decl::Namespace(ns) => {
                let next = if prefix.is_empty() { ns.name.clone() } else { format!("{prefix}.{}", ns.name) };
                collect_synth_enum_firsts(&ns.decls, &next, sep, out);
            }
            _ => {}
        }
    }
}

fn collect_bundled_referrers(decls: &mut [Decl], bundled: &[String]) -> HashMap<String, Vec<String>> {
    fn index_decls(decls: &[Decl], ns: &mut HashSet<String>, synth: &mut HashSet<String>) {
        for d in decls {
            match d {
                Decl::Namespace(n) => {
                    ns.insert(n.name.clone());
                    index_decls(&n.decls, ns, synth);
                }
                Decl::Interface(i) if i.synthetic => {
                    synth.insert(i.name.clone());
                }
                Decl::Alias(a) if a.synthetic => {
                    synth.insert(a.name.clone());
                }
                _ => {}
            }
        }
    }
    if bundled.is_empty() {
        return HashMap::new();
    }
    let bundled_set: HashSet<&str> = bundled.iter().map(String::as_str).collect();
    let (mut ns_names, mut synth_names) = (HashSet::new(), HashSet::new());
    index_decls(decls, &mut ns_names, &mut synth_names);

    let mut non_synth: HashMap<String, Vec<String>> = HashMap::new();
    let mut fallback: HashMap<String, Vec<String>> = HashMap::new();
    walk(decls, &mut |t, path| {
        if let Type::Ref(r) = t {
            let short = r.name.rsplit('.').next().unwrap_or(&r.name);
            let key = bundled_set.get(r.name.as_str()).or_else(|| bundled_set.get(short)).copied();
            if let Some(target) = key {
                let enclosing = path
                    .iter()
                    .find(|p| !ns_names.contains(p.as_str()))
                    .or_else(|| path.first())
                    .map(String::as_str)
                    .unwrap_or_default();
                if !enclosing.is_empty() && enclosing != target && !enclosing.contains('$') {
                    let fb = fallback.entry(target.to_string()).or_default();
                    if !fb.iter().any(|x| x == enclosing) {
                        fb.push(enclosing.to_string());
                    }
                    if !synth_names.contains(enclosing) {
                        let ns = non_synth.entry(target.to_string()).or_default();
                        if !ns.iter().any(|x| x == enclosing) {
                            ns.push(enclosing.to_string());
                        }
                    }
                }
            }
        }
        true
    });
    bundled
        .iter()
        .filter_map(|b| {
            let refs = non_synth
                .remove(b)
                .filter(|v| !v.is_empty())
                .or_else(|| fallback.remove(b));
            refs.map(|v| (b.clone(), v))
        })
        .collect()
}

fn upper_first(w: &str) -> String {
    let mut cs = w.chars();
    cs.next()
        .map(|c| c.to_uppercase().collect::<String>() + cs.as_str())
        .unwrap_or_default()
}

fn disambiguate_bundled(referrers: &[String], bare: &str, taken: &HashSet<String>) -> Option<String> {
    use crate::generators::names::{stem, words};

    let bare_stems: HashSet<String> = words(bare).into_iter().map(|w| stem(&w)).collect();
    let word_vecs: Vec<Vec<String>> = referrers
        .iter()
        .map(|r| words(r))
        .filter(|w| !w.is_empty())
        .collect();
    if word_vecs.is_empty() {
        return None;
    }
    let mut common_len = 0;
    while common_len < word_vecs[0].len()
        && word_vecs[1..].iter().all(|wv| {
            wv.get(common_len)
                .is_some_and(|w| w.eq_ignore_ascii_case(&word_vecs[0][common_len]))
        })
    {
        common_len += 1;
    }

    let make_cand = |prefix: String| -> Option<String> {
        if prefix.is_empty() {
            return None;
        }
        let cand = format!("{prefix}{}", upper_first(bare));
        (!taken.contains(&cand)).then_some(cand)
    };

    if common_len > 0 {
        let deduped: String = word_vecs[0][..common_len]
            .iter()
            .filter(|w| !bare_stems.contains(&stem(w)))
            .map(|w| upper_first(w))
            .collect();
        if let Some(c) = make_cand(deduped) {
            return Some(c);
        }
        let raw_common: String = word_vecs[0][..common_len].iter().map(|w| upper_first(w)).collect();
        if let Some(c) = make_cand(raw_common) {
            return Some(c);
        }
    }

    // Fallback when referrers start with different leading words: try the first referrer's
    // first distinct leading word(s).
    let first_words: Vec<String> = word_vecs[0]
        .iter()
        .filter(|w| !bare_stems.contains(&stem(w)))
        .map(|w| upper_first(w))
        .collect();
    (1..=first_words.len().min(2)).find_map(|k| make_cand(first_words[..k].concat()))
}

fn collect_names(decls: &[Decl], out: &mut Vec<String>) {
    for d in decls {
        match d {
            Decl::Namespace(n) => collect_names(&n.decls, out),
            Decl::Function(_) | Decl::Var(_) => {}
            d => out.push(d.name().to_string()),
        }
    }
}

// A method a type redeclares as its ancestor declares it, but for the types
// of `this`: `HTMLDivElement.addEventListener` is `HTMLElement.addEventListener`.
fn drop_redeclared(decls: &mut [Decl]) -> usize {
    let key = |m: &Member| -> Option<String> {
        let mut sig = m.sig.clone()?;
        sig.this = None;
        let mut path = Vec::new();
        walk_signature(&mut sig, &mut path, &mut |t, _| {
            if let Type::Function(s) | Type::Constructor(s) = t {
                s.this = None;
            }
            true
        });
        let params: Vec<String> = sig.params.iter().map(|p| format!("{}{}{}", p.optional, p.rest, crate::xml::type_key(&p.ty))).collect();
        let tps: Vec<String> = sig.type_params.iter().map(|t| format!("{}:{}", t.name, t.constraint.as_ref().map(crate::xml::type_key).unwrap_or_default())).collect();
        Some(format!("{}({})<{}>{}", m.name, params.join(","), tps.join(","), sig.ret.as_ref().map(crate::xml::type_key).unwrap_or_default()))
    };
    let mut own: HashMap<String, HashSet<String>> = HashMap::new();
    let mut bases: HashMap<String, Vec<String>> = HashMap::new();
    for d in decls.iter() {
        if let Decl::Interface(i) = d {
            own.entry(i.name.clone()).or_default().extend(i.members.iter().filter(|m| m.kind == MemberKind::Method).filter_map(key));
            bases.entry(i.name.clone()).or_default().extend(i.extends.iter().filter_map(|t| if let Type::Ref(r) = t { Some(r.name.clone()) } else { None }));
        }
    }
    let inherited = |name: &str| -> HashSet<String> {
        let mut out = HashSet::new();
        let mut todo: Vec<String> = bases.get(name).cloned().unwrap_or_default();
        let mut seen = HashSet::new();
        while let Some(b) = todo.pop() {
            if seen.insert(b.clone()) {
                out.extend(own.get(&b).cloned().unwrap_or_default());
                todo.extend(bases.get(&b).cloned().unwrap_or_default());
            }
        }
        out
    };
    let mut dropped = 0;
    for d in decls.iter_mut() {
        if let Decl::Interface(i) = d {
            let from_bases = inherited(&i.name);
            if from_bases.is_empty() {
                continue;
            }
            // Only whole overload sets: a set split between a type and its
            // ancestor is resolved differently, or hidden, in some targets.
            let redeclared = |m: &Member| m.kind == MemberKind::Method && key(m).is_some_and(|k| from_bases.contains(&k));
            let names: HashSet<String> = i.members.iter().filter(|m| m.kind == MemberKind::Method).map(|m| m.name.clone()).collect();
            let whole: HashSet<String> = names.into_iter().filter(|n| i.members.iter().filter(|m| m.kind == MemberKind::Method && m.name == *n).all(redeclared)).collect();
            let before = i.members.len();
            i.members.retain(|m| !(m.kind == MemberKind::Method && whole.contains(&m.name)));
            dropped += before - i.members.len();
        }
    }
    dropped
}

// A rest parameter of a tuple of fixed length: its elements, each a parameter.
fn spread_tuple_rest(s: &mut Signature) {
    let Some(last) = s.params.last() else { return };
    let Type::Tuple(elems) = &last.ty else { return };
    if !last.rest || elems.iter().any(|e| e.rest) {
        return;
    }
    let (base, elems) = (last.name.clone(), elems.clone());
    s.params.pop();
    let mut taken: HashSet<String> = s.params.iter().map(|p| p.name.clone()).collect();
    for (i, e) in elems.into_iter().enumerate() {
        let mut name = e.name.unwrap_or_else(|| format!("{base}{i}"));
        while !taken.insert(name.clone()) {
            name.push('_');
        }
        s.params.push(Param { name, optional: e.optional, rest: false, constant: None, ty: e.ty });
    }
}

// A property whose type is only call signatures is a method of each
// (marked's `Marked#parse: { (src, options & { async: true }): Promise<string>; .. }`),
// as a caller sees it: `marked.parse(src)`; a constant so is a function of
// each (marked's `parseInline`).
fn callable_properties(decls: &mut Vec<Decl>) {
    fn constants(decls: &mut Vec<Decl>) {
        let mut out = Vec::with_capacity(decls.len());
        for d in decls.drain(..) {
            match d {
                Decl::Var(v) if matches!(&v.ty, Type::Object(ms) if !ms.is_empty() && ms.iter().all(|c| c.kind == MemberKind::Call)) => {
                    let Type::Object(ms) = v.ty else { unreachable!() };
                    out.extend(ms.into_iter().filter_map(|c| c.sig).map(|sig| Decl::Function(Function { name: v.name.clone(), js_name: v.js_name.clone(), sig, doc: v.doc.clone() })));
                }
                Decl::Namespace(mut n) => {
                    constants(&mut n.decls);
                    out.push(Decl::Namespace(n));
                }
                d => out.push(d),
            }
        }
        *decls = out;
    }
    constants(decls);
    each_member_list(decls, &mut |members| {
        let mut out = Vec::with_capacity(members.len());
        for m in members.drain(..) {
            let calls: Option<Vec<Signature>> = match (&m.kind, &m.ty) {
                (MemberKind::Property, Some(Type::Object(ms))) if !ms.is_empty() && ms.iter().all(|c| c.kind == MemberKind::Call) => ms.iter().map(|c| c.sig.clone()).collect(),
                _ => None,
            };
            match calls {
                Some(sigs) => out.extend(sigs.into_iter().map(|sig| Member { kind: MemberKind::Method, sig: Some(sig), ty: None, readonly: false, ..m.clone() })),
                None => out.push(m),
            }
        }
        *members = out;
    });
}

// Overloads an options object's boolean literal tells apart (marked's
// `{ async: false }` gives a string, `{ async: true }` a promise, and leaving
// it out either): each passes its literal itself, the one of `true` named
// for its property (`marked(src)`, `marked_async(src)`) in every target, so
// what it gives is sound whatever the library is set to; the one that
// leaves it open goes.
fn literal_option_overloads(decls: &mut Vec<Decl>, sep: &str) {
    // The parameter and property a literal of this signature fixes, and its value.
    fn fixed(s: &Signature) -> Option<(usize, String, String)> {
        s.params.iter().enumerate().find_map(|(i, p)| match &p.ty {
            Type::Object(ms) => ms.iter().find_map(|m| match &m.ty {
                Some(Type::Literal { kind: "boolean", value }) if m.kind == MemberKind::Property && !m.optional => Some((i, m.name.clone(), value.clone())),
                _ => None,
            }),
            _ => None,
        })
    }
    fn group(sigs: Vec<&mut Signature>, renamed: &mut Vec<(usize, String)>) -> Vec<usize> {
        let found: Vec<Option<(usize, String, String)>> = sigs.iter().map(|s| fixed(s)).collect();
        let values: HashSet<&str> = found.iter().flatten().map(|(_, _, v)| v.as_str()).collect();
        let Some((at, prop, _)) = found.iter().flatten().next().cloned() else { return Vec::new() };
        if !(values.contains("true") && values.contains("false")) || found.iter().flatten().any(|(i, p, _)| *i != at || *p != prop) {
            return Vec::new();
        }
        let mut dropped = Vec::new();
        for (k, (s, f)) in sigs.into_iter().zip(&found).enumerate() {
            match f {
                Some((_, _, v)) => {
                    if v == "true" {
                        renamed.push((k, prop.clone()));
                    }
                    let p = &mut s.params[at];
                    p.constant = Some(format!("{{{prop:?}: {v}}}"));
                    p.ty = Type::prim("object");
                    p.optional = false;
                }
                // The one that leaves it open, with the same parameters before it.
                None if s.params.len() > at => dropped.push(k),
                None => {}
            }
        }
        dropped
    }
    each_member_list(decls, &mut |members| {
        let names: Vec<String> = members.iter().filter(|m| m.kind == MemberKind::Method).map(|m| m.js_name.clone().unwrap_or_else(|| m.name.clone())).collect();
        for name in names.into_iter().collect::<HashSet<_>>() {
            let idx: Vec<usize> = members.iter().enumerate().filter(|(_, m)| m.kind == MemberKind::Method && m.js_name.as_ref().unwrap_or(&m.name) == &name).map(|(i, _)| i).collect();
            if idx.len() < 3 {
                continue;
            }
            let sigs: Vec<&mut Signature> = members.iter_mut().enumerate().filter(|(i, _)| idx.contains(i)).filter_map(|(_, m)| m.sig.as_mut()).collect();
            let mut renamed = Vec::new();
            let dropped: HashSet<usize> = group(sigs, &mut renamed).into_iter().map(|k| idx[k]).collect();
            for (k, prop) in renamed {
                let m = &mut members[idx[k]];
                m.js_name.get_or_insert_with(|| m.name.clone());
                m.name = format!("{}{sep}{prop}", m.name);
            }
            let mut i = 0;
            members.retain(|_| {
                i += 1;
                !dropped.contains(&(i - 1))
            });
        }
    });
    fn functions(decls: &mut Vec<Decl>, sep: &str) {
        let names: HashSet<String> = decls.iter().filter_map(|d| if let Decl::Function(f) = d { Some(f.js_name.clone().unwrap_or_else(|| f.name.clone())) } else { None }).collect();
        for name in names {
            let idx: Vec<usize> = decls.iter().enumerate().filter(|(_, d)| matches!(d, Decl::Function(f) if f.js_name.as_ref().unwrap_or(&f.name) == &name)).map(|(i, _)| i).collect();
            if idx.len() < 3 {
                continue;
            }
            let sigs: Vec<&mut Signature> = decls.iter_mut().enumerate().filter(|(i, _)| idx.contains(i)).filter_map(|(_, d)| if let Decl::Function(f) = d { Some(&mut f.sig) } else { None }).collect();
            let mut renamed = Vec::new();
            let dropped: HashSet<usize> = group(sigs, &mut renamed).into_iter().map(|k| idx[k]).collect();
            for (k, prop) in renamed {
                if let Decl::Function(f) = &mut decls[idx[k]] {
                    f.js_name.get_or_insert_with(|| f.name.clone());
                    f.name = format!("{}{sep}{prop}", f.name);
                }
            }
            let mut i = 0;
            decls.retain(|_| {
                i += 1;
                !dropped.contains(&(i - 1))
            });
        }
        for d in decls.iter_mut() {
            if let Decl::Namespace(n) = d {
                functions(&mut n.decls, sep);
            }
        }
    }
    functions(decls, sep);
}
