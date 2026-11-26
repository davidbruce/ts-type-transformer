use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use roxmltree::Document;

use super::bounds::{break_cycles, compute_class_bound, generic_args, parent, plan_folded_modules, take_secondary};
use super::classes::emit_class_and_func_modules;
use super::types::{visit_ty, Gen};
use super::{
    codec_var, lower, module_path, relative, snake, upper, upper_variant, Output, ProjectRef,
    KEYWORDS, MODEL_OPTIONS, NAMING, OPAQUE_BUILTINS, OWN, OWN_RUNTIME, PHANTOM,
};
use super::super::comments::{Style, comment};
use super::super::model::{model_with_options, subst, walk, Call, Class, EnumDoc, Model, TParam, Ty};

// A custom type of variants without fields, with its documentation.
fn enum_type(name: &str, variants: &[String], docs: Option<&EnumDoc>) -> String {
    let mut out = docs.and_then(|d| d.doc.as_deref()).map(|d| comment(d, "///", "", Style::Plain)).unwrap_or_default();
    out += &format!("pub type {name} {{\n");
    for (i, v) in variants.iter().enumerate() {
        out += &docs.and_then(|d| d.members.get(i)?.as_deref()).map(|d| comment(d, "///", "  ", Style::Plain)).unwrap_or_default();
        out += &format!("  {v}\n");
    }
    out + "}\n\n"
}

fn assign_type_path(
    m: &Model,
    tname: &str,
    model_name: &str,
    xml_module: &str,
    paths: &HashMap<String, String>,
    pkg: &str,
    taken: &mut HashSet<String>,
    out_paths: &mut HashMap<String, String>,
) {
    // A namespace's declaration by its own name: `ns.Node`'s is `namespaces/ns/node`.
    let own = m.namespaced.get(model_name).map(|p| {
        let depth = m.namespace_modules.get(model_name).map_or(1, |(_, d)| *d).min(p.len() - 1);
        p[depth..].iter().map(|s| snake(&upper(s))).collect::<Vec<_>>().join("/")
    });
    let mut segment = own.unwrap_or_else(|| snake(tname));
    if KEYWORDS.contains(&segment.as_str()) {
        segment.push('_');
    }
    // Its namespace's module, a package's in the package's (`areas/pb/util`).
    let module = m.namespace_modules.get(model_name).map_or(xml_module, |(m, _)| m.as_str());
    let base = format!("{}/{segment}", module_path(paths, pkg, module));
    let mut path = base.clone();
    let mut n = 2;
    while !taken.insert(path.clone()) {
        path = format!("{base}_{n}");
        n += 1;
    }
    out_paths.insert(model_name.to_string(), path);
}

fn expand_deps(
    t: &Ty,
    m: &Model,
    nodes: &HashSet<String>,
    out: &mut BTreeSet<String>,
    seen_cb: &mut HashSet<String>,
) {
    visit_ty(t, &mut |sub| {
        let (Ty::Obj(n) | Ty::Generic(n, _) | Ty::NamedUnion(n)) = sub else {
            return;
        };
        if let Some(cb) = m.callbacks.iter().find(|c| c.name == *n) {
            if !seen_cb.insert(n.clone()) {
                return;
            }
            let args = generic_args(sub);
            let resolved: Vec<Ty> = (0..cb.type_params.len())
                .map(|i| args.get(i).cloned().unwrap_or(Ty::Any))
                .collect();
            for (_, pt) in &cb.params {
                expand_deps(&subst(pt, &cb.type_params, &resolved), m, nodes, out, seen_cb);
            }
            expand_deps(&subst(&cb.ret, &cb.type_params, &resolved), m, nodes, out, seen_cb);
            seen_cb.remove(n);
            return;
        }
        if nodes.contains(n) {
            out.insert(n.clone());
        }
    });
}

fn tarjan_sccs(nodes: &[String], adj: &BTreeMap<String, BTreeSet<String>>) -> Vec<Vec<String>> {
    struct State<'a> {
        adj: &'a BTreeMap<String, BTreeSet<String>>,
        index: usize,
        indices: HashMap<String, usize>,
        lowlink: HashMap<String, usize>,
        stack: Vec<String>,
        on_stack: HashSet<String>,
        sccs: Vec<Vec<String>>,
    }
    fn strongconnect(v: &str, st: &mut State<'_>) {
        let v_idx = st.index;
        st.indices.insert(v.to_string(), v_idx);
        st.lowlink.insert(v.to_string(), v_idx);
        st.index += 1;
        st.stack.push(v.to_string());
        st.on_stack.insert(v.to_string());
        if let Some(nbrs) = st.adj.get(v) {
            for w in nbrs {
                if !st.indices.contains_key(w.as_str()) {
                    strongconnect(w, st);
                    let w_low = st.lowlink[w.as_str()];
                    let v_low = st.lowlink.get_mut(v).unwrap();
                    *v_low = (*v_low).min(w_low);
                } else if st.on_stack.contains(w.as_str()) {
                    let w_idx = st.indices[w.as_str()];
                    let v_low = st.lowlink.get_mut(v).unwrap();
                    *v_low = (*v_low).min(w_idx);
                }
            }
        }
        if st.lowlink[v] == v_idx {
            let mut scc = Vec::new();
            while let Some(w) = st.stack.pop() {
                st.on_stack.remove(&w);
                let done = w == v;
                scc.push(w);
                if done {
                    break;
                }
            }
            st.sccs.push(scc);
        }
    }
    let mut st = State {
        adj,
        index: 0,
        indices: HashMap::new(),
        lowlink: HashMap::new(),
        stack: Vec::new(),
        on_stack: HashSet::new(),
        sccs: Vec::new(),
    };
    for v in nodes {
        if !st.indices.contains_key(v.as_str()) {
            strongconnect(v, &mut st);
        }
    }
    st.sccs
}

// The bindings of the XML, or what Gleam cannot express of it.
pub(super) fn module_named(xml: &str, package_name: &str, refs: &[ProjectRef]) -> Result<Output, Vec<String>> {
    let pkg = if package_name.is_empty() { "bindings" } else { package_name };
    let runtime_mod = format!("{pkg}/runtime");
    let types_mod = format!("{pkg}/types");
    let callbacks_mod = format!("{pkg}/callbacks");
    let doc = Document::parse(xml).map_err(|e| vec![format!("not well-formed XML: {e}")])?;
    let root_import = doc.root_element().attribute("import").map(str::to_string);
    let mut m = model_with_options(xml, MODEL_OPTIONS)?;
    take_secondary(&mut m);
    // Give explicit methods priority over synthesized property setters when names collide.
    for c in &mut m.classes {
        c.funcs.sort_by_key(|f| matches!(f.call, Call::Set(_) | Call::StaticSet(_)));
    }
    super::super::names::settle(&mut m, &NAMING);
    break_cycles(&mut m);
    let mut g = Gen {
        m: &m,
        pkg: pkg.to_string(),
        types: HashMap::new(),
        callbacks: HashMap::new(),
        callback_codecs: HashMap::new(),
        union_codecs: HashMap::new(),
        values: HashSet::new(),
        reserved_values: HashSet::new(),
        unit: runtime_mod.clone(),
        out: String::new(),
        ffi: String::new(),
        used: RefCell::default(),
        ffi_used: RefCell::default(),
        used_key_ctor: false,
        uses_self_mjs: false,
        units: BTreeMap::new(),
        home: HashMap::new(),
        ffi_home: HashMap::new(),
        module_rank: HashMap::new(),
        classes: m.classes.iter().map(|c| (c.name.clone(), c.type_params.len())).collect(),
        parents: m.classes.iter().filter_map(|c| parent(&m, c).map(|p| (c.name.clone(), p))).collect(),
        class_bound: HashMap::new(),
        open: RefCell::new(None),
        going_in: Cell::new(false),
        inline_callbacks: Cell::new(false),
        inline_cb_depth: Cell::new(0),
        bound: RefCell::default(),
        fixed_params: RefCell::default(),
        bound_lists: RefCell::default(),
        key_vars: RefCell::default(),
        expanding_defaults: RefCell::default(),
        skipped: m.skipped.clone(),
        given_back: given_back(&m),
        unmakeable: HashSet::new(),
    };

    // Names: every type (`names::settle` made them apart).
    let mut type_names: HashSet<String> = OWN.iter().map(|s| s.to_string()).collect();
    let mut name_type = |n: &str, g: &mut Gen| {
        let name = upper(n);
        assert!(type_names.insert(name.clone()), "Gleam: the type `{name}` is named twice");
        g.types.insert(n.to_string(), name);
    };
    for c in &m.classes {
        name_type(&c.name, &mut g);
    }
    for c in &m.callbacks {
        name_type(&c.name, &mut g);
        g.callbacks.insert(c.name.clone(), (0..c.type_params.len()).collect());
    }
    // Gleam allows no unused type parameter: `this` of a callback has gone, and
    // what is `Dynamic` shows none. A callback may hold another: until settled.
    loop {
        let mut changed = false;
        for c in &m.callbacks {
            let used: Vec<usize> = (0..c.type_params.len()).filter(|&i| c.params.iter().map(|x| &x.1).chain([&c.ret]).any(|t| g.shows(t, &c.type_params[i].name))).collect();
            if used != g.callbacks[&c.name] {
                g.callbacks.insert(c.name.clone(), used);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    for u in &m.unions {
        name_type(&u.name, &mut g);
    }
    for (e, _) in &m.enums {
        name_type(e, &mut g);
    }
    for (e, _) in &m.num_enums {
        name_type(e, &mut g);
    }
    for o in m.opaque.iter().filter(|o| !OPAQUE_BUILTINS.contains(&o.as_str())) {
        name_type(o, &mut g);
    }
    g.types.insert("PromiseLike".into(), "Promise".into());
    // What the FFI's own helpers are called in the runtime module.
    for reserved in [
        "promise_then",
        "promise_try",
        "attempt",
        "js_error_message",
        "dynamic",
        "ref_new",
        "ref_get",
        "ref_set",
        "ref_update",
        "js_object",
        "js_get",
        "js_set",
        "js_has",
        "js_array",
        "js_to_list",
        "to_fixed",
    ] {
        g.values.insert((runtime_mod.clone(), reserved.into()));
    }

    // Each area's modules: `areas/main` is `{pkg}` (its functions)
    // and `{pkg}/<class>` (its classes).
    let mut resolved_opaque: HashMap<String, usize> = HashMap::new();
    let mut ref_aliases: HashMap<String, (String, String)> = HashMap::new();
    for o in m.opaque.iter().filter(|o| !OPAQUE_BUILTINS.contains(&o.as_str())) {
        let gname = g.tname(o);
        let orig = m.opaque_names.get(o).map(String::as_str).unwrap_or(o.as_str());
        let ref_gname = upper(orig);
        if m.opaque_arity.get(o).copied().unwrap_or(0) == 0 {
            for (idx, r) in refs.iter().enumerate() {
                if r.classes.contains(&ref_gname) {
                    let mod_path = format!("{}/types", r.package_name);
                    resolved_opaque.insert(o.clone(), idx);
                    g.classes.insert(o.clone(), 0);
                    g.home.insert(gname.clone(), mod_path.clone());
                    if gname != ref_gname {
                        ref_aliases.insert(gname.clone(), (mod_path, ref_gname));
                    }
                    break;
                }
            }
        }
    }
    // Nor does anything here give one back (echarts' `getDom()` gives an `HTMLElement`).
    let returned = returned_types(&m);
    g.unmakeable = m.opaque.iter().filter(|o| !resolved_opaque.contains_key(*o) && !OPAQUE_BUILTINS.contains(&o.as_str()) && !returned.contains(*o)).cloned().collect();
    g.class_bound = compute_class_bound(&m, &g.classes, &g.parents);
    // Compute exact crossing codec indices for callbacks and decodable unions.
    for c in &m.callbacks {
        g.callback_codecs.insert(c.name.clone(), Vec::new());
    }
    for u in m.unions.iter().filter(|u| u.decodable()) {
        g.union_codecs.insert(u.name.clone(), Vec::new());
    }
    loop {
        let mut changed = false;
        for c in &m.callbacks {
            let used: Vec<usize> = g.callbacks[&c.name]
                .iter()
                .copied()
                .filter(|&i| c.params.iter().map(|x| &x.1).chain([&c.ret]).any(|t| g.crosses(t, &c.type_params[i].name)))
                .collect();
            if used != g.callback_codecs[&c.name] {
                g.callback_codecs.insert(c.name.clone(), used);
                changed = true;
            }
        }
        for u in m.unions.iter().filter(|u| u.decodable()) {
            let used: Vec<usize> = (0..u.type_params.len())
                .filter(|&i| u.parts.iter().any(|p| g.crosses(p, &u.type_params[i].name)))
                .collect();
            if used != g.union_codecs[&u.name] {
                g.union_codecs.insert(u.name.clone(), used);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    let mut paths: HashMap<String, String> = HashMap::new();
    let mut taken: HashSet<String> = HashSet::from([runtime_mod.clone(), types_mod.clone()]);
    if !m.callbacks.is_empty() {
        taken.insert(callbacks_mod.clone());
    }
    let areas: BTreeSet<&String> = m
        .classes
        .iter()
        .map(|c| &c.module)
        .chain(m.funcs.iter().map(|f| &f.module))
        .chain(m.type_modules.values())
        .chain(m.namespace_modules.values().map(|(mod_name, _)| mod_name))
        .collect();
    for area in areas {
        let base = if *area == "areas/main" || area.is_empty() {
            pkg.to_string()
        } else {
            let segments: Vec<String> = area.split('/').filter(|s| !s.is_empty()).map(lower).collect();
            format!("{pkg}/{}", segments.join("/"))
        };
        let mut path = base.clone();
        let mut n = 2;
        while !taken.insert(path.clone()) {
            path = format!("{base}_{n}");
            n += 1;
        }
        paths.insert(area.clone(), path);
    }
    // Step A: assign module paths for all non-synthetic classes.
    let mut class_paths: HashMap<String, String> = HashMap::new();
    for c in &m.classes {
        if !m.synthetic.contains(&c.name) {
            let tname = g.tname(&c.name);
            assign_type_path(&m, &tname, &c.name, &c.module, &paths, pkg, &mut taken, &mut class_paths);
        }
    }
    // Step B: fold eligible synthetic micro-modules into their sole owning class/area module.
    let folded = plan_folded_modules(&m, &g, &paths, pkg, &class_paths);
    // Step C: assign module paths for remaining non-folded synthetic classes.
    for c in &m.classes {
        if m.synthetic.contains(&c.name) && !folded.contains_key(&c.name) {
            let tname = g.tname(&c.name);
            assign_type_path(&m, &tname, &c.name, &c.module, &paths, pkg, &mut taken, &mut class_paths);
        }
    }
    // Assign per-type module paths for enums and emitted unions.
    let mut enum_paths: HashMap<String, String> = HashMap::new();
    for e in m.enums.iter().map(|e| &e.0).chain(m.num_enums.iter().map(|e| &e.0)) {
        let tname = g.tname(e);
        let xml_mod = m.type_modules.get(e).map(String::as_str).unwrap_or("areas/main");
        assign_type_path(&m, &tname, e, xml_mod, &paths, pkg, &mut taken, &mut enum_paths);
    }
    let emitted_unions: BTreeSet<String> = m
        .unions
        .iter()
        .filter(|u| u.decodable() || u.typable())
        .map(|u| u.name.clone())
        .collect();
    let mut union_paths: HashMap<String, String> = HashMap::new();
    for u in m.unions.iter().filter(|u| emitted_unions.contains(&u.name)) {
        let tname = g.tname(&u.name);
        let xml_mod = m.type_modules.get(&u.name).map(String::as_str).unwrap_or("areas/main");
        assign_type_path(&m, &tname, &u.name, xml_mod, &paths, pkg, &mut taken, &mut union_paths);
    }

    // Classes. A class with no class for a base is an opaque type, its last
    // parameter a phantom: `Node(a)`. One with a base is its base's type,
    // with a marker of its own for the phantom: `Element(a) = Node(ElementTag(a))`.
    // A base's arguments may not name the class, nor one below it on its
    // chain (`BitmapFont extends AbstractBitmapFont<BitmapFont>`): an alias
    // in terms of itself. Those are `Dynamic`.
    let mut below: HashMap<String, HashSet<String>> = HashMap::new();
    for c in &m.classes {
        let mut at = c.name.clone();
        below.entry(at.clone()).or_default().insert(c.name.clone());
        while let Some((p, _)) = g.parents.get(&at) {
            below.entry(p.clone()).or_default().insert(c.name.clone());
            at = p.clone();
        }
    }

    // Stratify classes and unions into acyclic Gleam modules.
    let class_set: HashSet<String> = m.classes.iter().map(|c| c.name.clone()).collect();
    let graph_nodes_set: HashSet<String> = class_set.iter().cloned().chain(emitted_unions.iter().cloned()).collect();
    let mut full_deps: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for c in &m.classes {
        let entry = full_deps.entry(c.name.clone()).or_default();
        if let Some((base, args)) = g.parents.get(&c.name) {
            entry.insert(base.clone());
            let own = &below[&c.name];
            let mut seen_cb = HashSet::new();
            for a in args {
                let cut = walk(a, &|t| matches!(t, Ty::Obj(n) | Ty::Generic(n, _) if own.contains(n)).then_some(Ty::Any));
                expand_deps(&cut, &m, &graph_nodes_set, entry, &mut seen_cb);
            }
        }
    }
    for u in m.unions.iter().filter(|u| emitted_unions.contains(&u.name)) {
        let entry = full_deps.entry(u.name.clone()).or_default();
        let mut seen_cb = HashSet::new();
        for p in &u.parts {
            expand_deps(p, &m, &graph_nodes_set, entry, &mut seen_cb);
        }
    }
    let later_classes: BTreeSet<String> = m
        .classes
        .iter()
        .filter(|c| {
            let mut seen = HashSet::new();
            let mut stack = vec![c.name.clone()];
            while let Some(cur) = stack.pop() {
                if !seen.insert(cur.clone()) {
                    continue;
                }
                if emitted_unions.contains(&cur) {
                    return true;
                }
                if let Some(next) = full_deps.get(&cur) {
                    stack.extend(next.iter().cloned());
                }
            }
            false
        })
        .map(|c| c.name.clone())
        .collect();

    let sub_nodes: Vec<String> = emitted_unions.iter().chain(&later_classes).cloned().collect();
    let sub_set: HashSet<String> = sub_nodes.iter().cloned().collect();
    let sub_deps: BTreeMap<String, BTreeSet<String>> = sub_nodes
        .iter()
        .map(|n| {
            let deps = full_deps
                .get(n)
                .map(|d| d.iter().filter(|x| sub_set.contains(*x)).cloned().collect())
                .unwrap_or_default();
            (n.clone(), deps)
        })
        .collect();
    let sccs = tarjan_sccs(&sub_nodes, &sub_deps);
    let mut scc_of: HashMap<String, usize> = HashMap::new();
    for (si, scc) in sccs.iter().enumerate() {
        for n in scc {
            scc_of.insert(n.clone(), si);
        }
    }
    let is_single_class_scc = |si: usize| sccs[si].len() == 1 && class_set.contains(&sccs[si][0]);
    let mut scc_batch: Vec<usize> = vec![0; sccs.len()];
    let mut scc_class_grp: Vec<usize> = vec![0; sccs.len()];
    for (si, scc) in sccs.iter().enumerate() {
        let dep_sccs: BTreeSet<usize> = scc
            .iter()
            .flat_map(|n| sub_deps.get(n).into_iter().flatten())
            .filter_map(|d| scc_of.get(d).copied())
            .filter(|&dj| dj != si)
            .collect();
        if is_single_class_scc(si) {
            let grp = dep_sccs
                .iter()
                .map(|&dj| if is_single_class_scc(dj) { scc_class_grp[dj] } else { scc_batch[dj] + 1 })
                .max()
                .unwrap_or(1)
                .max(1);
            scc_class_grp[si] = grp;
        } else {
            let batch = dep_sccs
                .iter()
                .map(|&dj| if is_single_class_scc(dj) { scc_class_grp[dj] } else { scc_batch[dj] })
                .max()
                .unwrap_or(0);
            scc_batch[si] = batch;
        }
    }

    let mut union_batches: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    let mut class_groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for si in 0..sccs.len() {
        if is_single_class_scc(si) {
            class_groups.entry(scc_class_grp[si]).or_default().push(si);
        } else {
            union_batches.entry(scc_batch[si]).or_default().push(si);
        }
    }

    let mut class_group_mods: HashMap<usize, String> = HashMap::new();
    for &grp in class_groups.keys() {
        let mut n = grp + 1;
        let mut cand = format!("{pkg}/types_{n}");
        while !taken.insert(cand.clone()) {
            n += 1;
            cand = format!("{pkg}/types_{n}");
        }
        class_group_mods.insert(grp, cand);
    }

    #[derive(Clone)]
    enum ModGroup {
        TypesMod,
        Unions(Vec<String>, String),
        ClassLayer(Vec<String>, String),
    }
    let mut type_emit_order: Vec<ModGroup> = Vec::new();
    let layer0_classes: Vec<String> = m
        .classes
        .iter()
        .filter(|c| !later_classes.contains(&c.name))
        .map(|c| c.name.clone())
        .collect();
    type_emit_order.push(ModGroup::TypesMod);
    g.module_rank.insert(types_mod.clone(), 0);
    let mut rank_counter = 1usize;
    let max_batch = union_batches
        .keys()
        .copied()
        .max()
        .into_iter()
        .chain(class_groups.keys().map(|g| g.saturating_sub(1)))
        .max();
    let mut type_units: HashSet<String> = HashSet::from([types_mod.clone()]);
    if let Some(max_b) = max_batch {
        for b in 0..=max_b {
            if let Some(sis) = union_batches.get(&b) {
                for &si in sis {
                    let scc = &sccs[si];
                    let mod_path = if scc.len() == 1 {
                        union_paths[&scc[0]].clone()
                    } else {
                        let mut k = 1usize;
                        let mut cand = format!("{pkg}/types_scc_{k}");
                        while !taken.insert(cand.clone()) {
                            k += 1;
                            cand = format!("{pkg}/types_scc_{k}");
                        }
                        cand
                    };
                    g.module_rank.insert(mod_path.clone(), rank_counter);
                    type_units.insert(mod_path.clone());
                    rank_counter += 1;
                    for n in scc {
                        let gn = g.tname(n);
                        g.home.insert(gn.clone(), mod_path.clone());
                        if class_set.contains(n) {
                            g.home.insert(format!("{gn}Tag"), mod_path.clone());
                        } else {
                            g.ffi_home.insert(gn, mod_path.clone());
                        }
                    }
                    type_emit_order.push(ModGroup::Unions(scc.clone(), mod_path));
                }
            }
            if let Some(sis) = class_groups.get(&(b + 1)) {
                let mod_path = class_group_mods[&(b + 1)].clone();
                g.module_rank.insert(mod_path.clone(), rank_counter);
                type_units.insert(mod_path.clone());
                rank_counter += 1;
                let members: Vec<String> = sis.iter().flat_map(|&si| sccs[si].iter().cloned()).collect();
                for n in &members {
                    let gn = g.tname(n);
                    g.home.insert(gn.clone(), mod_path.clone());
                    g.home.insert(format!("{gn}Tag"), mod_path.clone());
                }
                type_emit_order.push(ModGroup::ClassLayer(members, mod_path));
            }
        }
    }

    for o in OWN_RUNTIME {
        g.home.insert(o.to_string(), runtime_mod.clone());
    }
    for n in &layer0_classes {
        g.home.insert(g.tname(n), types_mod.clone());
        g.home.insert(format!("{}Tag", g.tname(n)), types_mod.clone());
    }
    for (e, mod_path) in &enum_paths {
        let gn = g.tname(e);
        g.home.insert(gn.clone(), mod_path.clone());
        g.ffi_home.insert(gn, mod_path.clone());
        type_units.insert(mod_path.clone());
    }
    for c in &m.callbacks {
        let gn = g.tname(&c.name);
        g.home.insert(gn.clone(), callbacks_mod.clone());
        g.ffi_home.insert(gn, callbacks_mod.clone());
    }
    if !m.callbacks.is_empty() {
        type_units.insert(callbacks_mod.clone());
    }
    for n in m.opaque.iter().filter(|n| !OPAQUE_BUILTINS.contains(&n.as_str()) && !resolved_opaque.contains_key(*n)) {
        g.home.insert(g.tname(n), types_mod.clone());
    }
    for u in m.unions.iter().filter(|u| !emitted_unions.contains(&u.name)) {
        g.home.insert(g.tname(&u.name), types_mod.clone());
    }

    // The runtime: what every module may use.
    g.out += include_str!("../runtime/gleam_runtime.gleam");
    g.note("Dynamic");
    g.note("Option");
    // The modules the program reaches: not a type-only one (node's `stream/iter`).
    let used = m.runtime_packages();
    let mut packages: Vec<&String> = m.packages.values().filter(|p| used.contains(*p)).collect();
    packages.sort();
    packages.dedup();
    let mut imports = String::new();
    let mut modules = Vec::new();
    for (i, p) in packages.iter().enumerate() {
        imports += &format!("import * as P{i} from {p:?};\n");
        modules.push(format!("{p:?}: P{i}"));
    }
    if let Some(root) = root_import.as_ref().filter(|r| !packages.contains(r)) {
        imports += &format!("import * as ROOT from {root:?};\n");
        modules.push(format!("{root:?}: ROOT"));
    }
    g.ffi += &format!(
        "{imports}\n\
         // Where the API's names are: its module, or globals; with several packages, each one's.\n\
         export const PK = {{ {} }};\nexport const G = {};\n\n\
         {}",
        modules.join(", "),
        root_import.as_ref().map_or("globalThis".to_string(), |r| format!("PK[{r:?}]")),
        include_str!("../runtime/gleam_runtime_ffi.mjs")
    );

    // String and numeric enums: each in its own module with short variants and `codec()`.
    for (at, (e, values)) in m.enums.iter().enumerate() {
        g.switch(&enum_paths[e]);
        g.uses_self_mjs = true;
        let name = g.tname(e);
        let mut seen_v = HashSet::new();
        let variants: Vec<String> = values
            .iter()
            .map(|v| {
                let base = upper(if v.is_empty() { "empty" } else { v });
                let mut cand = base.clone();
                let mut n = 2;
                while !seen_v.insert(cand.clone()) {
                    cand = format!("{base}{n}");
                    n += 1;
                }
                cand
            })
            .collect();
        g.out += &enum_type(&name, &variants, m.enum_docs.get(at));
        let codec_id = g.value_name("codec");
        let js_codec = format!("{}_codec", snake(&name));
        g.note("Codec");
        g.out += &format!("@external(javascript, \"{}\", \"{js_codec}\")\npub fn {codec_id}() -> Codec({name})\n\n", g.own_ffi());
        g.ffi += &format!(
            "const {name}_values = {:?};\nconst {name}_cases = () => [{}];\n\
             export function {name}_to(v) {{\n  const i = {name}_values.indexOf(v);\n  if (i < 0) throw new TypeError(\"not a {name}: \" + v);\n  return new ({name}_cases()[i])();\n}}\n\
             export function {name}_from(v) {{\n  return {name}_values[{name}_cases().findIndex((c) => v instanceof c)];\n}}\n\
             export function {js_codec}() {{\n  return {{ to: (v) => {name}_to(v), from: (u) => {name}_from(u) }};\n}}\n",
            values,
            variants.iter().map(|v| format!("B.{v}")).collect::<Vec<_>>().join(", ")
        );
    }
    for (at, (e, members)) in m.num_enums.iter().enumerate() {
        g.switch(&enum_paths[e]);
        g.uses_self_mjs = true;
        let name = g.tname(e);
        let mut seen_v = HashSet::new();
        let variants: Vec<String> = members
            .iter()
            .map(|(member, _)| {
                let base = upper_variant(member);
                let mut cand = base.clone();
                let mut n = 2;
                while !seen_v.insert(cand.clone()) {
                    cand = format!("{base}{n}");
                    n += 1;
                }
                cand
            })
            .collect();
        g.out += &enum_type(&name, &variants, m.num_enum_docs.get(at));
        let codec_id = g.value_name("codec");
        let js_codec = format!("{}_codec", snake(&name));
        g.note("Codec");
        g.out += &format!("@external(javascript, \"{}\", \"{js_codec}\")\npub fn {codec_id}() -> Codec({name})\n\n", g.own_ffi());
        g.ffi += &format!(
            "const {name}_values = [{}];\nconst {name}_cases = () => [{}];\n\
             export function {name}_to(v) {{\n  return new ({name}_cases()[{name}_values.indexOf(v)])();\n}}\n\
             export function {name}_from(v) {{\n  return {name}_values[{name}_cases().findIndex((c) => v instanceof c)];\n}}\n\
             export function {js_codec}() {{\n  return {{ to: (v) => {name}_to(v), from: (u) => {name}_from(u) }};\n}}\n",
            members.iter().map(|(_, v)| v.clone()).collect::<Vec<_>>().join(", "),
            variants.iter().map(|v| format!("B.{v}")).collect::<Vec<_>>().join(", ")
        );
    }

    // Opaques and Layer-0 classes in `{pkg}/types`, then stratified unions and later classes.
    g.switch(&types_mod);
    for o in m.opaque.iter().filter(|o| !OPAQUE_BUILTINS.contains(&o.as_str()) && !resolved_opaque.contains_key(*o)) {
        let arity = m.opaque_arity.get(o).copied().unwrap_or(0);
        let params = if arity == 0 { String::new() } else { format!("({})", (1..=arity).map(|i| format!("t{i}")).collect::<Vec<_>>().join(", ")) };
        g.out += &format!("pub type {}{params}\n\n", g.tname(o));
    }

    struct CodecSpec {
        name: String,
        return_vars: Vec<TParam>,
        param_vars: Vec<TParam>,
        converts: bool,
        unit: String,
    }
    let mut codecs: Vec<CodecSpec> = Vec::new();
    codecs.extend(m.callbacks.iter().map(|c| CodecSpec {
        name: c.name.clone(),
        return_vars: g.callbacks[&c.name].iter().map(|&i| c.type_params[i].clone()).collect(),
        param_vars: g.callback_codecs[&c.name].iter().map(|&i| c.type_params[i].clone()).collect(),
        converts: true,
        unit: callbacks_mod.clone(),
    }));
    for o in m.opaque.iter().filter(|o| !OWN.contains(&o.as_str()) && *o != "PromiseLike" && !resolved_opaque.contains_key(*o)) {
        let arity = m.opaque_arity.get(o).copied().unwrap_or(0);
        let rvars = (1..=arity).map(|i| TParam { name: format!("t{i}"), default: None, constraint: None }).collect();
        codecs.push(CodecSpec { name: o.clone(), return_vars: rvars, param_vars: Vec::new(), converts: false, unit: types_mod.clone() });
    }

    let emit_class_decl = |g: &mut Gen<'_>, c: &Class, mod_path: &str, codecs: &mut Vec<CodecSpec>| {
        let name = g.tname(&c.name);
        let mut vars: Vec<String> = c.type_params.iter().map(|p| Gen::tvar(&p.name)).collect();
        vars.push(PHANTOM.into());
        let vars = vars.join(", ");
        g.out += &c.doc.as_deref().map(|d| comment(d, "///", "", Style::Plain)).unwrap_or_default();
        match g.parents.get(&c.name).cloned() {
            None => g.out += &format!("pub type {name}({vars})\n\n"),
            Some((base, args)) => {
                let own = &below[&c.name];
                let args: Vec<Ty> = args.iter().map(|a| walk(a, &|t| matches!(t, Ty::Obj(n) | Ty::Generic(n, _) if own.contains(n)).then_some(Ty::Any))).collect();
                let mut parts: Vec<String> = g.with_inline_callbacks(|| g.closed(|| args.iter().map(|a| g.ty(a)).collect()));
                while parts.len() < g.classes[&base] {
                    g.note("Dynamic");
                    parts.push("Dynamic".into());
                }
                parts.push(format!("{name}Tag({vars})"));
                g.out += &format!("pub type {name}Tag({vars})\n\npub type {name}({vars}) = {}({})\n\n", g.gt(&base), parts.join(", "));
            }
        }
        let mut params = c.type_params.clone();
        params.push(TParam { name: PHANTOM.into(), default: None, constraint: None });
        codecs.push(CodecSpec { name: c.name.clone(), return_vars: params, param_vars: Vec::new(), converts: false, unit: mod_path.to_string() });
    };

    for group in type_emit_order {
        match group {
            ModGroup::TypesMod => {
                g.switch(&types_mod);
                for name in &layer0_classes {
                    if let Some(c) = m.classes.iter().find(|c| &c.name == name) {
                        emit_class_decl(&mut g, c, &types_mod, &mut codecs);
                    }
                }
            }
            ModGroup::Unions(members, mod_path) => {
                g.switch(&mod_path);
                let single = members.len() == 1 && m.unions.iter().any(|u| u.name == members[0]);
                let mut used_variants = HashSet::new();
                for n in &members {
                    if let Some(u) = m.unions.iter().find(|u| u.name == *n) {
                        g.union(u, single, &mut used_variants);
                    } else if let Some(c) = m.classes.iter().find(|c| c.name == *n) {
                        emit_class_decl(&mut g, c, &mod_path, &mut codecs);
                    }
                }
            }
            ModGroup::ClassLayer(members, mod_path) => {
                g.switch(&mod_path);
                for n in &members {
                    if let Some(c) = m.classes.iter().find(|c| c.name == *n) {
                        emit_class_decl(&mut g, c, &mod_path, &mut codecs);
                    }
                }
            }
        }
    }

    // Callbacks: in `{pkg}/callbacks`, able to import all enums, unions, and classes.
    if !m.callbacks.is_empty() {
        g.switch(&callbacks_mod);
        for c in &m.callbacks {
            let name = g.tname(&c.name);
            let used: Vec<TParam> = g.callbacks[&c.name].iter().map(|&i| c.type_params[i].clone()).collect();
            let params = Gen::type_params(&used);
            let args: Vec<String> = c.params.iter().map(|(_, t)| g.ty(t)).collect();
            let ret = g.ty(&c.ret);
            g.out += &format!("pub type {name}{params} =\n  fn({}) -> {ret}\n\n", args.join(", "));
            let names: Vec<String> = (0..c.params.len()).map(|i| format!("a{i}")).collect();
            let into_gleam: Vec<String> = c.params.iter().zip(&names).map(|((_, t), a)| g.to_gleam(t, a)).collect();
            let mut into_js: Vec<String> = c.params.iter().zip(&names).map(|((_, t), a)| g.from_gleam(t, a)).collect();
            let codecs_str: String = g.callback_codecs[&c.name].iter().map(|&i| format!(", {}", codec_var(&c.type_params[i].name))).collect();
            let mut gathered = names.clone();
            if c.rest {
                if let Some(last) = gathered.last_mut() {
                    *last = format!("...{last}");
                }
                if let Some(last) = into_js.last_mut() {
                    *last = format!("...{last}");
                }
            }
            g.ffi += &format!(
                "export function {name}_from(f{codecs_str}) {{\n  return $wrap(f, (f) => ({}) => {});\n}}\n\
                 export function {name}_to(f{codecs_str}) {{\n  return typeof f === \"function\" ? ({}) => {} : f;\n}}\n",
                gathered.join(", "),
                g.from_gleam(&c.ret, &format!("f({})", into_gleam.join(", "))),
                names.join(", "),
                g.to_gleam(&c.ret, &format!("f({})", into_js.join(", "))),
            );
        }
    }

    for CodecSpec { name: n, return_vars, param_vars, converts, unit } in codecs {
        g.switch(&unit);
        let name = g.tname(&n);
        let id = g.value_name(&format!("{}_codec", snake(&name)));
        let vars = Gen::type_params(&return_vars);
        g.note("Codec");
        if !converts {
            g.out += &format!("@external(javascript, \"{}\", \"codec_same\")\npub fn {id}() -> Codec({name}{vars})\n\n", g.runtime_ffi());
            continue;
        }
        let gleam_params: Vec<String> = param_vars.iter().map(|p| format!("{}_codec: Codec({})", lower(&p.name), Gen::tvar(&p.name))).collect();
        let js: String = param_vars.iter().map(|p| format!(", {}", codec_var(&p.name))).collect();
        g.out += &format!("@external(javascript, \"{}\", \"{id}\")\npub fn {id}({}) -> Codec({name}{vars})\n\n", g.own_ffi(), gleam_params.join(", "));
        g.ffi += &format!(
            "export function {id}({}) {{\n  return {{ to: (v) => {name}_to(v{js}), from: (u) => {name}_from(u{js}) }};\n}}\n",
            js.trim_start_matches(", ")
        );
    }

    emit_class_and_func_modules(&mut g, &m, &class_paths, &folded, &paths, pkg);

    // A namespace's types by their own names, in its module: `ns.Node` is
    // `pub type Node(sub) = NsNode2(sub)` in `bindings/namespaces/ns`, as the
    // types themselves are in `bindings/types`.
    let headers: HashMap<String, String> = g
        .units
        .iter()
        .filter(|(p, _)| type_units.contains(*p))
        .flat_map(|(_, u)| {
            u.out
                .lines()
                .filter_map(|l| l.strip_prefix("pub opaque type ").or_else(|| l.strip_prefix("pub type ")))
                .map(|rest| {
                    let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
                    let params = rest[name.len()..].strip_prefix('(').and_then(|r| r.split_once(')')).map(|(p, _)| format!("({p})")).unwrap_or_default();
                    (name, params)
                })
        })
        .collect();
    let mut aliases: BTreeMap<String, Vec<(String, String, String)>> = BTreeMap::new();
    for (name, path) in &m.namespaced {
        let Some((module, depth)) = m.namespace_modules.get(name) else { continue };
        let gleam = g.tname(name);
        let Some(params) = headers.get(&gleam) else { continue };
        let base = module_path(&paths, pkg, module);
        // Past what its module stands for: a namespace's own module.
        let unit = std::iter::once(base).chain(path[(*depth).min(path.len() - 1)..path.len() - 1].iter().map(|s| snake(&upper(s)))).collect::<Vec<_>>().join("/");
        aliases.entry(unit).or_default().push((upper(path.last().unwrap()), gleam, params.clone()));
    }
    for (unit, mut list) in aliases {
        // In one order whatever `namespaced`'s: which of two same-named leaves is kept, too.
        list.sort();
        let u = g.units.entry(unit).or_default();
        let mut seen = HashSet::new();
        // What the module imports, these aliases' types included.
        let targets: HashSet<String> = list.iter().map(|(_, t, _)| t.clone()).collect();
        for (leaf, target, params) in list {
            // Once, and not where the module names another type so, nor
            // Gleam's own (echarts' `List` would hide `List`).
            if leaf == target || OWN.contains(&leaf.as_str()) || u.used.contains(&leaf) || targets.contains(&leaf) || !seen.insert(leaf.clone()) {
                continue;
            }
            u.out += &format!("pub type {leaf}{params} =\n  {target}{params}\n\n");
            u.used.insert(target);
        }
    }

    // Each module: what it imports of the others, of Gleam's, and its JavaScript's.
    let mut files = Vec::new();
    let ref_by_mod: HashMap<String, usize> = refs.iter().enumerate().map(|(i, r)| (format!("{}/types", r.package_name), i)).collect();
    let mut used_refs: BTreeSet<usize> = BTreeSet::new();
    // Not a module with nothing in it (one whose aliases all gave way).
    for (path, unit) in std::mem::take(&mut g.units).into_iter().filter(|(p, u)| !p.is_empty() && !(u.out.trim().is_empty() && u.ffi.is_empty())) {
        let mut by_module: BTreeMap<String, BTreeSet<&str>> = BTreeMap::new();
        for t in &unit.used {
            let module = match t.as_str() {
                "Dynamic" => "gleam/dynamic".to_string(),
                "Option" => "gleam/option".to_string(),
                t => match g.home.get(t) {
                    Some(home) => home.clone(),
                    None => continue,
                },
            };
            if let Some(&idx) = ref_by_mod.get(&module) {
                used_refs.insert(idx);
            }
            if module != path {
                by_module.entry(module).or_default().insert(t);
            }
        }
        let mut seg_counts: HashMap<&str, usize> = HashMap::new();
        for module in by_module.keys() {
            let seg = module.rsplit('/').next().unwrap_or(module.as_str());
            *seg_counts.entry(seg).or_default() += 1;
        }
        let imports: String = by_module
            .iter()
            .map(|(module, ts)| {
                let seg = module.rsplit('/').next().unwrap_or(module.as_str());
                let suffix = if seg_counts.get(seg).copied().unwrap_or(0) > 1 {
                    " as _"
                } else {
                    ""
                };
                let mut items: Vec<String> = ts
                    .iter()
                    .map(|t| match ref_aliases.get(*t) {
                        Some((ref_mod, orig)) if ref_mod == module => format!("type {orig} as {t}"),
                        _ => format!("type {t}"),
                    })
                    .collect();
                if unit.used_key_ctor && *module == runtime_mod {
                    items.push("Key".to_string());
                }
                format!("import {module}.{{{}}}{suffix}\n", items.join(", "))
            })
            .collect();
        let whole: String = unit.used.iter().filter_map(|t| t.strip_prefix(super::classes::MODULE_MARK)).map(|m| format!("import {m} as {}\n", super::classes::module_alias(m))).collect();
        files.push((format!("{path}.gleam"), format!("// Generated. Bindings for Gleam's JavaScript target.\n{imports}{whole}\n{}", unit.out)));
        if unit.ffi.is_empty() {
            continue;
        }
        let up = "../".repeat(path.matches('/').count());
        let prelude = if up.is_empty() { "./".to_string() } else { up.clone() };
        let mut header = format!(
            "// Generated. Calls the API, and converts between JavaScript's values and Gleam's.\n\
             import {{ Some, None }} from \"{up}../gleam_stdlib/gleam/option.mjs\";\n\
             import {{ Ok, Error as GleamError, toList }} from \"{prelude}gleam.mjs\";\n"
        );
        if path != runtime_mod {
            header += &format!("import {{ $key, $wrap, $same, $trim, PK, G }} from \"{}\";\n", relative(&path, &format!("{pkg}/runtime_ffi.mjs")));
            for dep in &unit.ffi_used {
                header += &format!("import * as {} from \"{}\";\n", Gen::ffi_alias(dep), relative(&path, &format!("{dep}_ffi.mjs")));
            }
        }
        if unit.uses_self_mjs {
            let seg = path.rsplit('/').next().unwrap_or(path.as_str());
            header += &format!("import * as B from \"./{seg}.mjs\";\n");
        }
        files.push((format!("{path}_ffi.mjs"), format!("{header}{}", unit.ffi)));
    }
    let project_refs = used_refs.iter().map(|&i| (refs[i].package_name.clone(), refs[i].dep_path.clone())).collect();
    let meta_classes: Vec<String> = m
        .classes
        .iter()
        .filter(|c| c.type_params.is_empty() && g.home.get(&g.tname(&c.name)) == Some(&types_mod))
        .map(|c| g.tname(&c.name))
        .collect();
    let meta = super::super::pkg_meta::PkgMeta {
        target: "gleam".into(),
        package: pkg.to_string(),
        types: meta_classes,
        ..Default::default()
    };
    Ok(Output { name: pkg.to_string(), files, project_refs, skipped: g.skipped, manifest: super::super::model::manifest(&m), meta_toml: meta.to_toml() })
}

// The unions a result or a callback's argument holds.
fn given_back(m: &Model) -> HashSet<String> {
    fn add(t: &Ty, out: &mut HashSet<String>) {
        match t {
            Ty::NamedUnion(n) => {
                out.insert(n.clone());
            }
            Ty::Nullable(t, _) | Ty::ArrayOf(t) | Ty::Listener(t) | Ty::Record(t) => add(t, out),
            Ty::Tuple(ts) | Ty::Union(ts) | Ty::Generic(_, ts) => ts.iter().for_each(|t| add(t, out)),
            _ => {}
        }
    }
    let mut out = HashSet::new();
    for f in m.funcs.iter().chain(m.classes.iter().flat_map(|c| &c.funcs)) {
        add(&f.ret, &mut out);
    }
    for c in &m.callbacks {
        c.params.iter().for_each(|(_, t)| add(t, &mut out));
    }
    out
}

// The named types a call's result or a callback's argument holds: not a
// property's, which gives back what was set (pts' `DefaultFormStyle.lineJoin`).
fn returned_types(m: &Model) -> HashSet<String> {
    fn add(t: &Ty, out: &mut HashSet<String>) {
        match t {
            Ty::Obj(n) => {
                out.insert(n.clone());
            }
            Ty::Generic(n, ts) => {
                out.insert(n.clone());
                ts.iter().for_each(|t| add(t, out));
            }
            Ty::Nullable(t, _) | Ty::ArrayOf(t) | Ty::Listener(t) | Ty::Record(t) => add(t, out),
            Ty::Tuple(ts) | Ty::Union(ts) => ts.iter().for_each(|t| add(t, out)),
            _ => {}
        }
    }
    let mut out = HashSet::new();
    for f in m.funcs.iter().chain(m.classes.iter().flat_map(|c| &c.funcs)).filter(|f| !matches!(f.call, Call::Get(_) | Call::GetBound(_) | Call::StaticGet(_) | Call::IndexGet)) {
        add(&f.ret, &mut out);
    }
    for c in &m.callbacks {
        c.params.iter().for_each(|(_, t)| add(t, &mut out));
    }
    out
}
