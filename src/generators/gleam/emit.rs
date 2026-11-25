use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use roxmltree::Document;

use super::bounds::{break_cycles, compute_class_bound, parent, plan_folded_modules, take_secondary};
use super::classes::emit_class_and_func_modules;
use super::types::Gen;
use super::{
    codec_var, lower, module_path, relative, snake, upper, upper_variant, Output, ProjectRef,
    KEYWORDS, MODEL_OPTIONS, NAMING, OPAQUE_BUILTINS, OWN, OWN_RUNTIME, PHANTOM,
};
use super::super::comments::{Style, comment};
use super::super::model::{model_with_options, walk, Call, Class, EnumDoc, Model, TParam, Ty};

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

fn assign_class_path(
    m: &Model,
    tname: &str,
    c: &Class,
    paths: &HashMap<String, String>,
    pkg: &str,
    taken: &mut HashSet<String>,
    class_paths: &mut HashMap<String, String>,
) {
    // A namespace's class by its own name: `ns.Node`'s is `namespaces/ns/node`.
    let own = m.namespaced.get(&c.name).map(|p| {
        let depth = m.namespace_modules.get(&c.name).map_or(1, |(_, d)| *d).min(p.len() - 1);
        p[depth..].iter().map(|s| snake(&upper(s))).collect::<Vec<_>>().join("/")
    });
    let mut segment = own.unwrap_or_else(|| snake(tname));
    if KEYWORDS.contains(&segment.as_str()) {
        segment.push('_');
    }
    // Its namespace's module, a package's in the package's (`areas/pb/util`).
    let module = m.namespace_modules.get(&c.name).map_or(&c.module, |(m, _)| m);
    let base = format!("{}/{segment}", module_path(paths, pkg, module));
    let mut path = base.clone();
    let mut n = 2;
    while !taken.insert(path.clone()) {
        path = format!("{base}_{n}");
        n += 1;
    }
    class_paths.insert(c.name.clone(), path);
}

// The bindings of the XML, or what Gleam cannot express of it.
pub(super) fn module_named(xml: &str, package_name: &str, refs: &[ProjectRef]) -> Result<Output, Vec<String>> {
    let pkg = if package_name.is_empty() { "bindings" } else { package_name };
    let runtime_mod = format!("{pkg}/runtime");
    let types_mod = format!("{pkg}/types");
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
        variants: HashMap::new(),
        callbacks: HashMap::new(),
        callback_codecs: HashMap::new(),
        union_codecs: HashMap::new(),
        values: HashSet::new(),
        reserved_values: HashSet::new(),
        unit: runtime_mod.clone(),
        out: String::new(),
        ffi: String::new(),
        used: RefCell::default(),
        used_key_ctor: false,
        units: BTreeMap::new(),
        home: HashMap::new(),
        classes: m.classes.iter().map(|c| (c.name.clone(), c.type_params.len())).collect(),
        parents: m.classes.iter().filter_map(|c| parent(&m, c).map(|p| (c.name.clone(), p))).collect(),
        class_bound: HashMap::new(),
        open: RefCell::new(None),
        going_in: Cell::new(false),
        bound: RefCell::default(),
        fixed_params: RefCell::default(),
        bound_lists: RefCell::default(),
        key_vars: RefCell::default(),
        expanding_defaults: RefCell::default(),
        skipped: m.skipped.clone(),
        given_back: given_back(&m),
    };

    // Names: every type (`names::settle` made them apart), then every
    // variant of a union or enum, apart from them and each other.
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
    // A variant by its type's name and its own.
    let mut variant_names: Vec<(String, usize, String)> = Vec::new();
    for u in m.unions.iter().filter(|u| u.decodable() || u.typable()) {
        for (i, p) in u.parts.iter().enumerate() {
            let own = match p {
                Ty::Num => "Number".to_string(),
                Ty::Str => "String".to_string(),
                Ty::Bool => "Boolean".to_string(),
                Ty::Array | Ty::ArrayOf(_) | Ty::Tuple(_) => "Array".to_string(),
                Ty::Obj(n) | Ty::Generic(n, _) | Ty::Enum(n) => g.tname(n),
                Ty::Param(n) => upper(n),
                _ => format!("Case{i}"),
            };
            variant_names.push((u.name.clone(), i, format!("{}{own}", g.tname(&u.name))));
        }
    }
    for (e, values) in &m.enums {
        for (i, v) in values.iter().enumerate() {
            variant_names.push((e.clone(), i, format!("{}{}", g.tname(e), upper(if v.is_empty() { "empty" } else { v }))));
        }
    }
    for (e, members) in &m.num_enums {
        for (i, (member, _)) in members.iter().enumerate() {
            variant_names.push((e.clone(), i, format!("{}{}", g.tname(e), upper_variant(member))));
        }
    }
    for (owner, i, base) in variant_names {
        let mut name = base.clone();
        let mut n = 2;
        while !type_names.insert(name.clone()) {
            name = format!("{base}{n}");
            n += 1;
        }
        g.variants.insert((owner, i), name);
    }
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
    let areas: BTreeSet<&String> = m.classes.iter().map(|c| &c.module).chain(m.funcs.iter().map(|f| &f.module)).collect();
    for area in areas {
        let base = if *area == "areas/main" {
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
            assign_class_path(&m, &tname, c, &paths, pkg, &mut taken, &mut class_paths);
        }
    }
    // Step B: fold eligible synthetic micro-modules into their sole owning class/area module.
    let folded = plan_folded_modules(&m, &g, &paths, pkg, &class_paths);
    // Step C: assign module paths for remaining non-folded synthetic classes.
    for c in &m.classes {
        if m.synthetic.contains(&c.name) && !folded.contains_key(&c.name) {
            let tname = g.tname(&c.name);
            assign_class_path(&m, &tname, c, &paths, pkg, &mut taken, &mut class_paths);
        }
    }
    for o in OWN_RUNTIME {
        g.home.insert(o.to_string(), runtime_mod.clone());
    }
    for c in &m.classes {
        g.home.insert(g.tname(&c.name), types_mod.clone());
        g.home.insert(format!("{}Tag", g.tname(&c.name)), types_mod.clone());
    }
    let others = m.enums.iter().map(|e| &e.0).chain(m.num_enums.iter().map(|e| &e.0)).chain(m.callbacks.iter().map(|c| &c.name)).chain(m.unions.iter().map(|u| &u.name)).chain(&m.opaque);
    for n in others.filter(|n| !OPAQUE_BUILTINS.contains(&n.as_str()) && !resolved_opaque.contains_key(*n)) {
        g.home.insert(g.tname(n), types_mod.clone());
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

    // The types that may refer to each other: externals, enums, unions, callbacks.
    g.switch(&types_mod);
    for o in m.opaque.iter().filter(|o| !OPAQUE_BUILTINS.contains(&o.as_str()) && !resolved_opaque.contains_key(*o)) {
        let arity = m.opaque_arity.get(o).copied().unwrap_or(0);
        let params = if arity == 0 { String::new() } else { format!("({})", (1..=arity).map(|i| format!("t{i}")).collect::<Vec<_>>().join(", ")) };
        g.out += &format!("pub type {}{params}\n\n", g.tname(o));
    }
    // String and numeric enums: custom types without fields.
    for (at, (e, values)) in m.enums.iter().enumerate() {
        let name = g.tname(e);
        let variants: Vec<String> = (0..values.len()).map(|i| g.variants[&(e.clone(), i)].clone()).collect();
        g.out += &enum_type(&name, &variants, m.enum_docs.get(at));
        g.ffi += &format!(
            "const {name}_values = {:?};\nconst {name}_cases = () => [{}];\n\
             export function {name}_to(v) {{\n  const i = {name}_values.indexOf(v);\n  if (i < 0) throw new TypeError(\"not a {name}: \" + v);\n  return new ({name}_cases()[i])();\n}}\n\
             export function {name}_from(v) {{\n  return {name}_values[{name}_cases().findIndex((c) => v instanceof c)];\n}}\n",
            values,
            variants.iter().map(|v| format!("B.{v}")).collect::<Vec<_>>().join(", ")
        );
    }
    for (at, (e, members)) in m.num_enums.iter().enumerate() {
        let name = g.tname(e);
        let variants: Vec<String> = (0..members.len()).map(|i| g.variants[&(e.clone(), i)].clone()).collect();
        g.out += &enum_type(&name, &variants, m.num_enum_docs.get(at));
        g.ffi += &format!(
            "const {name}_values = [{}];\nconst {name}_cases = () => [{}];\n\
             export function {name}_to(v) {{\n  return new ({name}_cases()[{name}_values.indexOf(v)])();\n}}\n\
             export function {name}_from(v) {{\n  return {name}_values[{name}_cases().findIndex((c) => v instanceof c)];\n}}\n",
            members.iter().map(|(_, v)| v.clone()).collect::<Vec<_>>().join(", "),
            variants.iter().map(|v| format!("B.{v}")).collect::<Vec<_>>().join(", ")
        );
    }
    // Callbacks: Gleam functions, made JavaScript's and back.
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
        let codecs: String = g.callback_codecs[&c.name].iter().map(|&i| format!(", {}", codec_var(&c.type_params[i].name))).collect();
        // The rest of the arguments: gathered into a list, and spread back.
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
            "export function {name}_from(f{codecs}) {{\n  return $wrap(f, (f) => ({}) => {});\n}}\n\
             export function {name}_to(f{codecs}) {{\n  return typeof f === \"function\" ? ({}) => {} : f;\n}}\n",
            gathered.join(", "),
            g.from_gleam(&c.ret, &format!("f({})", into_gleam.join(", "))),
            names.join(", "),
            g.to_gleam(&c.ret, &format!("f({})", into_js.join(", "))),
        );
    }
    for u in m.unions.iter().filter(|u| u.decodable() || u.typable()) {
        g.union(u);
    }
    // Each type's codec, beside it. A handle crosses as it is; an enum, a
    // union or a callback converts, a generic one by its type parameters' codecs.
    struct CodecSpec {
        name: String,
        return_vars: Vec<TParam>,
        param_vars: Vec<TParam>,
        converts: bool,
        unit: String,
    }
    let mut codecs: Vec<CodecSpec> = Vec::new();
    codecs.extend(m.enums.iter().map(|e| CodecSpec { name: e.0.clone(), return_vars: Vec::new(), param_vars: Vec::new(), converts: true, unit: types_mod.clone() }));
    codecs.extend(m.num_enums.iter().map(|e| CodecSpec { name: e.0.clone(), return_vars: Vec::new(), param_vars: Vec::new(), converts: true, unit: types_mod.clone() }));
    codecs.extend(m.unions.iter().filter(|u| u.decodable()).map(|u| CodecSpec {
        name: u.name.clone(),
        return_vars: u.type_params.clone(),
        param_vars: g.union_codecs[&u.name].iter().map(|&i| u.type_params[i].clone()).collect(),
        converts: true,
        unit: types_mod.clone(),
    }));
    codecs.extend(m.callbacks.iter().map(|c| CodecSpec {
        name: c.name.clone(),
        return_vars: g.callbacks[&c.name].iter().map(|&i| c.type_params[i].clone()).collect(),
        param_vars: g.callback_codecs[&c.name].iter().map(|&i| c.type_params[i].clone()).collect(),
        converts: true,
        unit: types_mod.clone(),
    }));
    for o in m.opaque.iter().filter(|o| !OWN.contains(&o.as_str()) && *o != "PromiseLike" && !resolved_opaque.contains_key(*o)) {
        let arity = m.opaque_arity.get(o).copied().unwrap_or(0);
        let rvars = (1..=arity).map(|i| TParam { name: format!("t{i}"), default: None, constraint: None }).collect();
        codecs.push(CodecSpec { name: o.clone(), return_vars: rvars, param_vars: Vec::new(), converts: false, unit: types_mod.clone() });
    }
    // Classes. A class with no class for a base is an opaque type, its last
    // parameter a phantom: `Node(a)`. One with a base is its base's type,
    // with a marker of its own for the phantom: `Element(a) = Node(ElementTag(a))`.
    // So an `Element(Nil)` is a `Node(ElementTag(Nil))`: a Node to any function
    // that takes a `Node(a)`, and nothing more than an Element.
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
    for c in &m.classes {
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
                let mut parts: Vec<String> = g.closed(|| args.iter().map(|a| g.ty(a)).collect());
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
        codecs.push(CodecSpec { name: c.name.clone(), return_vars: params, param_vars: Vec::new(), converts: false, unit: types_mod.clone() });
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
        .get(&types_mod)
        .map(|u| {
            u.out
                .lines()
                .filter_map(|l| l.strip_prefix("pub opaque type ").or_else(|| l.strip_prefix("pub type ")))
                .map(|rest| {
                    let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
                    let params = rest[name.len()..].strip_prefix('(').and_then(|r| r.split_once(')')).map(|(p, _)| format!("({p})")).unwrap_or_default();
                    (name, params)
                })
                .collect()
        })
        .unwrap_or_default();
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
    for (unit, list) in aliases {
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
    let has_types_ffi = g.units.get(&types_mod).is_some_and(|u| !u.ffi.is_empty());
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
                let suffix = if ref_by_mod.contains_key(module) && seg_counts.get(seg).copied().unwrap_or(0) > 1 {
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
            if has_types_ffi && unit.ffi.contains("$T.") {
                header += &format!("import * as $T from \"{}\";\n", relative(&path, &format!("{pkg}/types_ffi.mjs")));
            }
        }
        if path == types_mod {
            header += "import * as B from \"./types.mjs\";\n";
        }
        files.push((format!("{path}_ffi.mjs"), format!("{header}{}", unit.ffi)));
    }
    let project_refs = used_refs.iter().map(|&i| (refs[i].package_name.clone(), refs[i].dep_path.clone())).collect();
    let meta_classes: Vec<String> = m.classes.iter().filter(|c| c.type_params.is_empty()).map(|c| g.tname(&c.name)).collect();
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
            Ty::Nullable(t, _) | Ty::ArrayOf(t) | Ty::Listener(t) => add(t, out),
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
