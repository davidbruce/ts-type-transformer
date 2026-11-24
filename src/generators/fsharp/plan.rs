use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use roxmltree::{Node, NodeId};

use super::super::{attr, elements, flag, named, only_child};
use super::Gen;
use super::syntax::{ident, map_properties, only_child_type, pascal, type_ident, union_members, words};

pub(super) enum AliasForm {
    Interface,
    Delegate,
    Abbreviation(String),
}

pub(super) fn reaches_in(bases: &HashMap<String, Vec<String>>, from: &str, to: &str) -> bool {
    let mut todo = vec![from.to_string()];
    let mut seen = HashSet::new();
    while let Some(c) = todo.pop() {
        if c == to {
            return true;
        }
        if seen.insert(c.clone()) {
            todo.extend(bases.get(&c).cloned().unwrap_or_default());
        }
    }
    false
}

// The constraints each generic interface can keep, by position: those every
// reference to it satisfies, with a declared type that extends the
// constraint or a type parameter constrained to one. F# checks an
// interface's constraints wherever it is named.
pub(super) fn constrained_interfaces(root: Node, bases: &HashMap<String, Vec<String>>) -> HashMap<String, Vec<Option<String>>> {
    let constraint_of = |p: Node| elements(p).find(|c| c.has_tag_name("constraint")).map(only_child).filter(|c| c.has_tag_name("ref") && matches!(attr(*c, "kind"), "interface" | "class") && elements(*c).next().is_none()).and_then(|c| c.attribute("to")).map(str::to_string);
    let mut out: HashMap<String, Vec<Option<String>>> = HashMap::new();
    for d in elements(root).filter(|d| d.has_tag_name("interface") || d.has_tag_name("class")) {
        let cs: Vec<Option<String>> = named(d, "typeParameter").map(constraint_of).collect();
        if cs.iter().any(Option::is_some) {
            out.insert(d.attribute("id").unwrap_or(attr(d, "name")).to_string(), cs);
        }
    }
    // A type parameter in scope at a reference: the constraint it keeps. An
    // interface's is in `out`, once dropped there gone; a function's is its own.
    let param_constraint = |out: &HashMap<String, Vec<Option<String>>>, r: Node, name: &str| -> Option<String> {
        for a in r.ancestors() {
            let ps: Vec<Node> = named(a, "typeParameter").collect();
            let Some(i) = ps.iter().position(|p| attr(*p, "name") == name) else { continue };
            // A function value's own are `obj`, which meets nothing.
            if a.has_tag_name("functionType") {
                return None;
            }
            if a.has_tag_name("interface") || a.has_tag_name("class") || a.has_tag_name("typeAlias") {
                return out.get(a.attribute("id").unwrap_or(attr(a, "name"))).and_then(|cs| cs.get(i).cloned().flatten());
            }
            return constraint_of(ps[i]);
        }
        None
    };
    // An abbreviation (`Cell<F, D, V> = Cell_Core<F, D, V>`) has the
    // constraints its type needs of its parameters: F# wants them written.
    let aliases: Vec<Node> = root.descendants().filter(|d| d.has_tag_name("typeAlias") && named(*d, "typeParameter").next().is_some()).collect();
    let mut dropped_ever: HashSet<(String, usize)> = HashSet::new();
    // A constraint dropped can break one a type parameter met through it:
    // until nothing more is dropped.
    loop {
        for d in &aliases {
            let body = only_child_type(*d);
            let Some(target) = body.attribute("to").filter(|_| body.has_tag_name("ref")).and_then(|t| out.get(t)).cloned() else { continue };
            let id = d.attribute("id").unwrap_or(attr(*d, "name")).to_string();
            let cs: Vec<Option<String>> = named(*d, "typeParameter")
                .enumerate()
                .map(|(j, p)| {
                    let at = elements(body).position(|a| a.has_tag_name("ref") && a.attribute("kind") == Some("typeParameter") && attr(a, "name") == attr(p, "name"));
                    at.and_then(|i| target.get(i).cloned().flatten()).filter(|_| !dropped_ever.contains(&(id.clone(), j)))
                })
                .collect();
            if cs.iter().any(Option::is_some) {
                out.insert(id, cs);
            } else {
                out.remove(&id);
            }
        }
        let mut broken: HashSet<(String, usize)> = HashSet::new();
        for r in root.descendants().filter(|r| r.has_tag_name("ref")) {
            let Some(to) = r.attribute("to") else { continue };
            let Some(cs) = out.get(to) else { continue };
            for (i, arg) in elements(r).enumerate() {
                let Some(Some(c)) = cs.get(i) else { continue };
                let is = |t: Node| t.has_tag_name("ref") && matches!(attr(t, "kind"), "interface" | "class") && t.attribute("to").is_some_and(|to| reaches_in(bases, to, c));
                let ok = match (arg.tag_name().name(), arg.attribute("kind")) {
                    ("ref", Some("interface" | "class")) => is(arg),
                    ("ref", Some("typeParameter")) => param_constraint(&out, r, attr(arg, "name")).is_some_and(|pc| reaches_in(bases, &pc, c)),
                    // A union of such types: the reference names the constraint instead.
                    ("ref", Some("typeAlias")) => union_members(root, arg).is_some_and(|ms| ms.into_iter().all(is)),
                    // `M[K]`: one of a map's values, each of which must be one. The
                    // signature's type parameter then has the constraint.
                    ("indexedAccess", _) => {
                        let map = elements(arg).next().filter(|m| m.has_tag_name("ref")).and_then(|m| super::syntax::find_decl(root, m.attribute("to")?));
                        map.is_some_and(|m| map_properties(root, m).into_iter().all(|(_, t)| is(t)))
                    }
                    _ => false,
                };
                if !ok {
                    broken.insert((to.to_string(), i));
                }
            }
        }
        let mut dropped = false;
        for (name, cs) in out.iter_mut() {
            for (i, c) in cs.iter_mut().enumerate() {
                if c.is_some() && broken.contains(&(name.clone(), i)) {
                    *c = None;
                    dropped = true;
                    dropped_ever.insert((name.clone(), i));
                }
            }
        }
        if !dropped {
            break;
        }
    }
    out.retain(|_, cs| cs.iter().any(Option::is_some));
    out
}

impl Gen {
    // The unions F# cannot tell apart, bound as `obj`: known before any
    // member that takes one is written, to tell such members apart.
    pub(super) fn union_abbrevs(&mut self, root: Node) {
        for d in root.descendants().filter(|d| d.has_tag_name("typeAlias") && named(*d, "typeParameter").next().is_none()) {
            let body = only_child_type(d);
            if !body.has_tag_name("union") {
                continue;
            }
            let (out, skipped) = (std::mem::take(&mut self.out), self.skipped.clone());
            let told_apart = self.named_union("probe", body, "");
            let recursive = body.descendants().any(|r| r.has_tag_name("ref") && r.attribute("name") == Some(attr(d, "name")));
            let erased = if told_apart { None } else if recursive { Some("obj".to_string()) } else { Some(self.ty(body, "")) };
            (self.out, self.skipped) = (out, skipped);
            if let Some(t) = erased {
                self.abbrevs.insert(type_ident(attr(d, "name")), t);
            }
        }
    }

    // What an alias other than a non-generic union is in F#: an interface
    // where an abbreviation cannot be one, a delegate where a function has
    // optional parameters, else an abbreviation of this type.
    pub(super) fn alias_form(&mut self, d: Node) -> AliasForm {
        let raw = attr(d, "name");
        let body = only_child_type(d);
        // F# abbreviations cannot refer to themselves; an interface can.
        let recursive = body.descendants().any(|r| r.has_tag_name("ref") && r.attribute("name") == Some(raw));
        // So can one a type extends: F# inherits only interfaces.
        let inherited = d.attribute("id").is_some_and(|id| self.inherited.contains(id));
        // And one with more than properties, or a lone index signature, has members.
        let members = body.has_tag_name("object")
            && elements(body).next().is_some()
            && !elements(body).all(|m| m.has_tag_name("property"))
            && !(elements(body).count() == 1 && elements(body).all(|m| m.has_tag_name("index")));
        // An object with optional properties is an interface (with a ParamObject
        // `Create`), not an anonymous record where every field is required.
        let has_optional = body.has_tag_name("object") && elements(body).any(|m| m.has_tag_name("property") && flag(m, "optional"));
        if body.has_tag_name("object") && (recursive || inherited || members || has_optional) {
            return AliasForm::Interface;
        }
        if recursive && !(body.has_tag_name("functionType") && d.parent().is_some_and(|p| !p.has_tag_name("namespace"))) {
            self.skip("recursive type alias, bound as obj");
            return AliasForm::Abbreviation("obj".into());
        }
        if body.has_tag_name("functionType") && named(body, "parameter").any(|p| flag(p, "optional") || flag(p, "rest")) {
            return AliasForm::Delegate;
        }
        AliasForm::Abbreviation(self.ty(body, ""))
    }

    // Which type parameters each generic abbreviation shows: F# wants all it
    // declares in its type. Those its type does not show (in `this`, in a
    // union bound as `obj`, in an argument another drops) are dropped from
    // it and from references to it, until settled.
    pub(super) fn settle_alias_kept(&mut self, root: Node) {
        let aliases: Vec<Node> = root.descendants().filter(|d| d.has_tag_name("typeAlias") && named(*d, "typeParameter").next().is_some()).collect();
        self.probing = true;
        loop {
            let mut changed = false;
            for d in &aliases {
                let saved = (std::mem::take(&mut self.skipped), std::mem::take(&mut self.problems));
                let form = self.alias_form(*d);
                (self.skipped, self.problems) = saved;
                let AliasForm::Abbreviation(t) = form else { continue };
                let w = words(&t);
                let params: Vec<&str> = named(*d, "typeParameter").map(|p| attr(p, "name")).collect();
                let k: Vec<usize> = (0..params.len()).filter(|&i| w.contains(params[i])).collect();
                let id = d.attribute("id").unwrap_or(attr(*d, "name")).to_string();
                if k.len() < params.len() && self.alias_kept.get(&id) != Some(&k) {
                    self.alias_kept.insert(id, k);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        self.probing = false;
    }
}

// The files of types, in the order F# must see them: one per module, each
// after the modules it cannot do without. Those are the bases of its types,
// the bodies of its aliases and anything a namespace mentions: F# has no way
// to refer to them from an earlier file. Other references only decide the
// order where these allow a choice; a member that refers forward becomes an
// extension member (see `members`). Modules whose bases or aliases refer to
// each other share one file.
pub(super) fn file_plan(root: Node) -> (Vec<Vec<String>>, HashMap<String, String>) {
    let home = homes(root);
    let units = units(root, &home);
    let unit = |d: Node| -> Option<String> { d.attribute("name").and_then(|n| units.get(n)).cloned() };
    let mut modules: BTreeSet<String> = BTreeSet::new();
    let mut hard: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut soft: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    // An alias is where its parts are: a reference to one is to those.
    let alias_refs: HashMap<&str, Vec<&str>> = root
        .descendants()
        .filter(|d| d.has_tag_name("typeAlias"))
        .filter_map(|d| Some((d.attribute("id")?, d.descendants().filter(|r| r.has_tag_name("ref")).filter_map(|r| r.attribute("to")).collect())))
        .collect();
    let through = |to: &str| -> Vec<String> {
        let mut out = Vec::new();
        let mut todo = vec![to];
        let mut seen = HashSet::new();
        while let Some(t) = todo.pop() {
            if !seen.insert(t) {
                continue;
            }
            match alias_refs.get(t) {
                // Its own module too: one F# declares (`OpenMode`, a union) is there.
                Some(parts) => {
                    out.push(t.to_string());
                    todo.extend(parts.iter().copied());
                }
                _ => out.push(t.to_string()),
            }
        }
        out
    };
    let mut base_links: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut method_names: HashMap<&str, HashSet<&str>> = HashMap::new();
    for d in elements(root).filter(|d| d.has_tag_name("interface") || d.has_tag_name("class") || d.has_tag_name("typeAlias")) {
        let key = d.attribute("id").or_else(|| d.attribute("name")).unwrap_or_default();
        if key.is_empty() {
            continue;
        }
        if !d.has_tag_name("typeAlias") {
            let bases: Vec<&str> = named(d, "extends")
                .flat_map(|e| e.descendants())
                .filter(|r| r.has_tag_name("ref"))
                .filter_map(|r| r.attribute("to"))
                .collect();
            if !bases.is_empty() {
                base_links.insert(key, bases);
            }
        }
        let src = if d.has_tag_name("typeAlias") { only_child_type(d) } else { d };
        let ms: HashSet<&str> = elements(src)
            .filter(|m| !flag(*m, "static") && (m.has_tag_name("method") || (m.has_tag_name("property") && !flag(*m, "optional") && only_child(*m).has_tag_name("functionType"))))
            .filter_map(|m| m.attribute("name"))
            .collect();
        if !ms.is_empty() {
            method_names.insert(key, ms);
        }
    }
    for d in elements(root) {
        if d.attribute("module").is_none() || d.has_tag_name("var") || d.has_tag_name("function") {
            continue;
        }
        let Some(m) = unit(d) else { continue };
        let m = m.as_str();
        modules.insert(m.to_string());
        let target = |r: Node| -> Vec<String> {
            r.attribute("to").map(through).unwrap_or_default().iter().filter_map(|n| units.get(n.as_str()).or_else(|| home.get(n.as_str()))).filter(|t| t.as_str() != m).cloned().collect()
        };
        // An alias has no fixed place: it goes where its parts are (`alias_files`).
        let hard_scope: Vec<Node> = match d.tag_name().name() {
            "namespace" => vec![d],
            "typeAlias" => vec![],
            _ => named(d, "extends").collect(),
        };
        for scope in &hard_scope {
            hard.entry(m.to_string()).or_default().extend(scope.descendants().filter(|r| r.has_tag_name("ref")).flat_map(target));
        }
        // An interface overriding an inherited member whose signature mentions a `*.base`
        // type (e.g. `Document.getElementById -> HTMLElement` overriding `NonElementParentNode`)
        // cannot live as an extension in a later file without being shadowed by the base.
        if d.has_tag_name("interface") || d.has_tag_name("class") {
            let key = d.attribute("id").or_else(|| d.attribute("name")).unwrap_or_default();
            let mut inherited = HashSet::new();
            let mut seen_b = HashSet::new();
            let mut todo_b = base_links.get(key).cloned().unwrap_or_default();
            while let Some(b) = todo_b.pop() {
                if seen_b.insert(b) {
                    if let Some(ms) = method_names.get(b) {
                        inherited.extend(ms.iter().copied());
                    }
                    if let Some(next_b) = base_links.get(b) {
                        todo_b.extend(next_b.iter().copied());
                    }
                }
            }
            for mem in elements(d).filter(|mem| !flag(*mem, "static") && (mem.has_tag_name("method") || (mem.has_tag_name("property") && !flag(*mem, "optional") && only_child(*mem).has_tag_name("functionType")))) {
                if let Some(mname) = mem.attribute("name")
                    && inherited.contains(mname)
                {
                    for t in mem.descendants().filter(|r| r.has_tag_name("ref")).flat_map(target) {
                        if t.ends_with(".base") {
                            hard.entry(m.to_string()).or_default().insert(t);
                        }
                    }
                }
            }
        }
        soft.entry(m.to_string()).or_default().extend(d.descendants().filter(|r| r.has_tag_name("ref")).flat_map(target));
    }
    let names: Vec<String> = modules.into_iter().collect();
    let index: HashMap<&str, usize> = names.iter().enumerate().map(|(i, n)| (n.as_str(), i)).collect();
    let edges = |deps: &BTreeMap<String, BTreeSet<String>>| -> Vec<BTreeSet<usize>> {
        names.iter().map(|n| deps.get(n).into_iter().flatten().filter_map(|t| index.get(t.as_str()).copied()).collect()).collect()
    };
    let (hard, all) = (edges(&hard), edges(&soft));
    // Preferred order: dependencies first, counting every reference.
    let mut rank = vec![0; names.len()];
    for (r, group) in tarjan(names.len(), &all).into_iter().enumerate() {
        for i in group {
            rank[i] = r;
        }
    }
    // Groups that must share a file, then placed as soon as their bases are, in preferred order.
    let groups = tarjan(names.len(), &hard);
    let group_of: HashMap<usize, usize> = groups.iter().enumerate().flat_map(|(g, ms)| ms.iter().map(move |&m| (m, g))).collect();
    let mut placed = vec![false; groups.len()];
    let mut out = Vec::new();
    while out.len() < groups.len() {
        let ready = (0..groups.len())
            .filter(|&g| !placed[g] && groups[g].iter().all(|&m| hard[m].iter().all(|t| group_of[t] == g || placed[group_of[t]])))
            .min_by_key(|&g| groups[g].iter().map(|&m| rank[m]).min())
            .expect("an order exists");
        placed[ready] = true;
        let mut ms: Vec<String> = groups[ready].iter().map(|&m| names[m].clone()).collect();
        ms.sort();
        out.push(ms);
    }
    (out, units)
}

// The part of its module each declaration goes in. What other modules inherit
// from (transitively) is `<module>.base`, placed before the rest: `Window`
// inherits from other modules' types, which inherit from `EventTarget`, so
// both cannot share a file with the others in between.
fn units(root: Node, home: &HashMap<String, String>) -> HashMap<String, String> {
    let mut bases: HashMap<&str, Vec<String>> = HashMap::new();
    for d in elements(root).filter(|d| d.has_tag_name("interface") || d.has_tag_name("class")) {
        let refs = named(d, "extends").flat_map(|e| e.descendants()).filter(|r| r.has_tag_name("ref")).filter_map(|r| r.attribute("to")).map(str::to_string);
        bases.insert(attr(d, "name"), refs.collect());
    }
    let mut base: HashSet<String> = HashSet::new();
    for (d, refs) in &bases {
        let module = home.get(*d);
        // Everything a declaration of another module inherits, directly or not.
        let mut todo: Vec<&String> = refs.iter().filter(|r| home.get(*r) != module).collect();
        while let Some(r) = todo.pop() {
            if base.insert(r.clone()) {
                todo.extend(bases.get(r.as_str()).into_iter().flatten());
            }
        }
    }
    home.iter().filter(|(n, _)| !n.contains('.')).map(|(n, m)| (n.clone(), if base.contains(n) { format!("{m}.base") } else { m.clone() })).collect()
}

// Every declared name, qualified, with its module.
fn homes(root: Node) -> HashMap<String, String> {
    fn walk(scope: Node, prefix: &str, module: &str, home: &mut HashMap<String, String>) {
        for d in elements(scope) {
            let Some(n) = d.attribute("name") else { continue };
            let module = d.attribute("module").unwrap_or(module);
            // A namespace merged into its package's module adds to no path.
            if d.attribute("moduleKind") == Some("merged") {
                walk(d, prefix, module, home);
                continue;
            }
            let key = if prefix.is_empty() { n.to_string() } else { format!("{prefix}.{n}") };
            if d.has_tag_name("namespace") {
                walk(d, &key, module, home);
            }
            home.insert(key, module.to_string());
        }
    }
    let mut home = HashMap::new();
    walk(root, "", "main", &mut home);
    home
}

// The file of each top-level type name as it appears in F# source, its
// static side (`XStatic`) included.
pub(super) fn declared_files(root: Node, plan: &[Vec<String>], units: &HashMap<String, String>) -> HashMap<String, usize> {
    let file: HashMap<&str, usize> = plan.iter().enumerate().flat_map(|(i, g)| g.iter().map(move |m| (m.as_str(), i))).collect();
    let mut out = HashMap::new();
    for d in elements(root) {
        let (Some(n), Some(_)) = (d.attribute("name"), d.attribute("module")) else { continue };
        if d.has_tag_name("var") || d.has_tag_name("function") {
            continue;
        }
        // A namespace of values only (ECharts' `color`) is no type's: by its
        // name, a property `color` would place what has it after it.
        if d.has_tag_name("namespace") && !d.descendants().any(|t| ["interface", "class", "typeAlias", "enum"].iter().any(|k| t.has_tag_name(*k))) {
            continue;
        }
        if let Some(&f) = units.get(n).and_then(|u| file.get(u.as_str())) {
            out.insert(n.to_string(), f);
            out.insert(format!("{n}Static"), f);
        }
    }
    // An alias goes in the first file where everything it refers to exists.
    let aliases: Vec<(String, Vec<String>)> = elements(root)
        .filter(|d| d.has_tag_name("typeAlias"))
        .map(|d| (attr(d, "name").to_string(), d.descendants().filter(|r| r.has_tag_name("ref")).filter_map(|r| r.attribute("to")).map(str::to_string).collect()))
        .collect();
    for _ in 0..aliases.len().min(32) {
        let mut changed = false;
        for (name, refs) in &aliases {
            let own = out.get(name).copied().unwrap_or(0);
            // `Tokens.Generic`: where `Tokens` is.
            let needed = refs.iter().filter_map(|r| out.get(r).or_else(|| out.get(r.split('.').next().unwrap()))).copied().max().unwrap_or(0).max(own);
            if needed != own {
                out.insert(name.clone(), needed);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    out
}

// Strongly connected components, each after those it has edges to.
fn tarjan(n: usize, edges: &[BTreeSet<usize>]) -> Vec<Vec<usize>> {
    struct State<'a> {
        edges: &'a [BTreeSet<usize>],
        index: Vec<Option<usize>>,
        low: Vec<usize>,
        on: Vec<bool>,
        stack: Vec<usize>,
        next: usize,
        out: Vec<Vec<usize>>,
    }
    fn visit(v: usize, s: &mut State) {
        s.index[v] = Some(s.next);
        s.low[v] = s.next;
        s.next += 1;
        s.stack.push(v);
        s.on[v] = true;
        for &w in s.edges[v].iter() {
            match s.index[w] {
                None => {
                    visit(w, s);
                    s.low[v] = s.low[v].min(s.low[w]);
                }
                Some(iw) if s.on[w] => s.low[v] = s.low[v].min(iw),
                _ => {}
            }
        }
        if Some(s.low[v]) == s.index[v] {
            let mut group = Vec::new();
            loop {
                let w = s.stack.pop().unwrap();
                s.on[w] = false;
                group.push(w);
                if w == v {
                    break;
                }
            }
            s.out.push(group);
        }
    }
    let mut s = State { edges, index: vec![None; n], low: vec![0; n], on: vec![false; n], stack: Vec::new(), next: 0, out: Vec::new() };
    for v in 0..n {
        if s.index[v].is_none() {
            visit(v, &mut s);
        }
    }
    s.out
}

// With several packages bound together, the declarations as F# has them: in
// a module of their package (`Pb.Node`, `ProsemirrorModel.Node`,
// `Fs.Promises.FileHandle`), by the names they export (`pb_Node` is `pb`'s
// `Node`). A namespace of the package's name (node's `stream` `Stream`) is
// merged into the module, which takes its name. Each is a namespace
// element: `package`, adding to no JavaScript path, as it is imported;
// `merged`, adding to no F# one. References follow. None with one package.
pub(super) fn package_modules(root: Node) -> Option<String> {
    root.attribute("packages")?;
    let root_import = root.attribute("import");
    let alnum_lower = |s: &str| -> String { s.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect() };
    // Each package's module path, by its import.
    let segments = |p: &str| -> Vec<String> {
        let p = p.trim_start_matches("node:");
        let p = if p.starts_with('@') { p.split_once('/').map_or(p, |x| x.1) } else { p };
        p.split('/').filter(|s| !s.is_empty()).map(str::to_string).collect()
    };
    let packaged: Vec<Node> = elements(root).filter(|d| !d.has_tag_name("lowering") && d.attribute("import").is_some_and(|i| Some(i) != root_import)).collect();
    if packaged.is_empty() {
        return None;
    }
    // A package's module's F# names: its segments, the last the name of a
    // namespace merged into it.
    let mut module_names: BTreeMap<Vec<String>, Vec<String>> = BTreeMap::new();
    let mut merged: HashSet<NodeId> = HashSet::new();
    for d in &packaged {
        let segs = segments(d.attribute("import").unwrap());
        let names = module_names.entry(segs.clone()).or_insert_with(|| segs.iter().map(|s| pascal(s)).collect());
        if d.has_tag_name("namespace") && alnum_lower(attr(*d, "name")) == alnum_lower(segs.last().unwrap()) {
            *names.last_mut().unwrap() = type_ident(attr(*d, "name"));
            merged.insert(d.id());
        }
    }
    // Each declaration's name in its package's module: the one it exports,
    // apart from the module's others (node's `cluster` has a class `Worker`
    // and its merged namespace an interface `Worker`): else its own, else a
    // number. A merged namespace's members keep theirs.
    // A module and a type may share a name (events' class `EventEmitter`
    // and its namespace): F# tells them apart.
    // Variables of one name are two values (Konva's default export and its
    // internal `Konva`); functions of one name, overloads.
    let key = |d: Node, n: &str| match d.tag_name().name() {
        "namespace" => format!("module {}", type_ident(n)),
        "var" => format!("value {}", ident(n)),
        _ => type_ident(n),
    };
    let mut taken: HashMap<Vec<String>, HashSet<String>> = HashMap::new();
    for d in packaged.iter().filter(|d| merged.contains(&d.id())) {
        taken.entry(segments(d.attribute("import").unwrap())).or_default().extend(elements(*d).filter_map(|e| e.attribute("name").map(|n| key(e, n))));
    }
    let mut names: HashMap<NodeId, String> = HashMap::new();
    for d in packaged.iter().filter(|d| !merged.contains(&d.id())) {
        let t = taken.entry(segments(d.attribute("import").unwrap())).or_default();
        let exported = d.attribute("jsName").filter(|j| *j != "default" && !j.contains('.')).unwrap_or(attr(*d, "name")).to_string();
        let name = [exported.clone(), attr(*d, "name").to_string()].into_iter().find(|n| !t.contains(&key(*d, n))).unwrap_or_else(|| {
            (2..).map(|i| format!("{exported}_{i}")).find(|n| !t.contains(&key(*d, n))).unwrap()
        });
        // A type and a value of one name are one class (`Node` and its static side).
        if d.tag_name().name() != "function" {
            t.insert(key(*d, &name));
        }
        names.insert(d.id(), name);
    }
    let own = |d: Node| -> String { names.get(&d.id()).cloned().unwrap_or_else(|| attr(d, "name").to_string()) };
    // Each moved declaration's id, and its descendants', by the old.
    let mut ids: HashMap<String, String> = HashMap::new();
    for d in &packaged {
        if merged.contains(&d.id()) {
            continue;
        }
        let module = module_names[&segments(d.attribute("import").unwrap())].join(".");
        let old = d.attribute("id").unwrap_or(attr(*d, "name")).to_string();
        let new = format!("{module}.{}", own(*d));
        for e in d.descendants().filter(|e| e.is_element()) {
            if let Some(id) = e.attribute("id") {
                if id == old {
                    ids.insert(id.to_string(), new.clone());
                } else if let Some(rest) = id.strip_prefix(&format!("{old}.")) {
                    ids.insert(id.to_string(), format!("{new}.{rest}"));
                }
            }
        }
    }
    fn write(n: Node, out: &mut String, ids: &HashMap<String, String>, rename: Option<&str>, extra: &[(&str, String)], moved: bool) {
        let esc = |v: &str| v.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;");
        *out += &format!("<{}", n.tag_name().name());
        // In a package's module, its module is the package's.
        for a in n.attributes().filter(|a| !extra.iter().any(|(k, _)| *k == a.name()) && !(moved && a.name() == "module")) {
            // A reference to a moved declaration names it as it is now (`pb_Node` is `Node`).
            let moved_to = n.attribute("to").and_then(|t| ids.get(t));
            let v = match a.name() {
                "to" | "id" => ids.get(a.value()).cloned().unwrap_or_else(|| a.value().to_string()),
                // A union member tested to be an instance of a moved class: of it.
                "test" if a.value().strip_prefix("instanceof:").is_some_and(|c| ids.contains_key(c)) => format!("instanceof:{}", ids[a.value().strip_prefix("instanceof:").unwrap()]),
                "name" if n.has_tag_name("ref") && moved_to.is_some() => moved_to.unwrap().rsplit('.').next().unwrap_or_default().to_string(),
                "name" => rename.map_or_else(|| a.value().to_string(), str::to_string),
                _ => a.value().to_string(),
            };
            *out += &format!(" {}=\"{}\"", a.name(), esc(&v));
        }
        for (k, v) in extra {
            *out += &format!(" {k}=\"{}\"", esc(v));
        }
        *out += ">";
        if let Some(text) = super::super::doc(n) {
            *out += &format!("<doc>{}</doc>", text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;"));
        }
        for c in super::super::elements(n) {
            write(c, out, ids, None, &[], moved);
        }
        *out += &format!("</{}>\n", n.tag_name().name());
    }
    // The tree of package modules, each with what is directly in it.
    let mut members: BTreeMap<Vec<String>, Vec<Node>> = BTreeMap::new();
    for d in &packaged {
        members.entry(segments(d.attribute("import").unwrap())).or_default().push(*d);
    }
    #[allow(clippy::too_many_arguments)]
    fn module_text(at: &[String], members: &BTreeMap<Vec<String>, Vec<Node>>, module_names: &BTreeMap<Vec<String>, Vec<String>>, ids: &HashMap<String, String>, merged: &HashSet<NodeId>, own: &dyn Fn(Node) -> String, import: &str, out: &mut String) {
        let names = module_names.get(at).cloned().unwrap_or_else(|| at.to_vec());
        let name = names.last().cloned().unwrap_or_default();
        *out += &format!("<namespace module=\"{}\" import=\"{}\" moduleKind=\"package\" name=\"{}\">\n", at.join("/"), import, name);
        for d in members.get(at).into_iter().flatten() {
            if merged.contains(&d.id()) {
                write(*d, out, ids, None, &[("moduleKind", "merged".to_string())], true);
            } else {
                write(*d, out, ids, Some(&own(*d)), &[], true);
            }
        }
        let children: BTreeSet<&String> = members.keys().filter(|k| k.len() > at.len() && k[..at.len()] == *at).map(|k| &k[at.len()]).collect();
        for child in children {
            let mut path = at.to_vec();
            path.push(child.clone());
            let child_import = members.get(&path).and_then(|ds| ds.first()).and_then(|d| d.attribute("import")).unwrap_or(import).to_string();
            module_text(&path, members, module_names, ids, merged, own, &child_import, out);
        }
        *out += "</namespace>\n";
    }
    let mut out = String::from("<declarations");
    for a in root.attributes() {
        out += &format!(" {}=\"{}\"", a.name(), a.value().replace('&', "&amp;").replace('"', "&quot;"));
    }
    out += ">\n";
    for d in elements(root).filter(|d| !packaged.iter().any(|p| p.id() == d.id())) {
        write(d, &mut out, &ids, None, &[], false);
    }
    let tops: BTreeSet<&String> = members.keys().map(|k| &k[0]).collect();
    for top in tops {
        let path = vec![top.clone()];
        let import = members.get(&path).and_then(|ds| ds.first()).and_then(|d| d.attribute("import")).map(str::to_string).unwrap_or_else(|| top.clone());
        module_text(&path, &members, &module_names, &ids, &merged, &own, &import, &mut out);
    }
    out += "</declarations>\n";
    Some(out)
}
