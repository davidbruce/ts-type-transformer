use std::collections::{BTreeMap, HashMap, HashSet};

use roxmltree::{Document, Node};

use super::build_types::{is_function_interface, value};
use super::{file_name, ident, Call, Callback, Func, Model, ModelOptions, TParam, Ty};
use super::super::{attr, elements, flag, named, only_child, Problems};

pub(super) struct Builder<'a> {
    pub opts: ModelOptions<'a>,
    // Several packages bound together: each is a module of its declarations.
    pub multi: bool,
    // Each top-level namespace's own name in its package, by its name.
    pub namespace_names: HashMap<String, String>,
    // Every declaration by qualified name.
    pub decls: HashMap<String, Node<'a, 'a>>,
    pub model: Model,
    pub opaque: HashSet<String>,
    pub imports: HashSet<String>,
    // Aliases being resolved, to stop on recursive ones.
    pub resolving: Vec<String>,
    // A generic union's type parameters its members use, where not all:
    // the others are dropped, and the arguments given for them.
    pub union_kept: HashMap<String, Vec<usize>>,
    // In the signature being read: type parameters over a map's keys, and their map.
    pub keyed: HashMap<String, String>,
    // Why the member being read cannot be bound, if it cannot.
    pub bad: Option<String>,
    // What the target cannot express at all.
    pub problems: std::cell::RefCell<Problems>,
    // The module of the declaration being read.
    pub module: String,
    // Whether generics are kept (C++), and the type parameters of the signature just read.
    pub generics: bool,
    pub last_type_params: Vec<TParam>,
    // Module folders given out, by lowercased path: names that differ only in case get a suffix.
    pub folders: HashMap<String, String>,
    pub folder_names: HashMap<(String, String), String>,
    // The package of the top-level declaration being read, when several are bound.
    pub package: Option<String>,
    // The package of declarations that do not name theirs: one bound alone.
    pub root_import: Option<String>,
    // The JavaScript path of the namespaces being read: `Konva`, or `util`
    // for the namespace the output calls `pb_util`.
    pub js_prefix: String,
    // See `type_ident`.
    pub declared: Vec<String>,
    // The XML says which declaration each reference is (`to`).
    pub has_ids: bool,
    pub idents: std::cell::RefCell<HashMap<String, String>>,
    pub ident_taken: std::cell::RefCell<HashSet<String>>,
    // A generic callback's own type parameters, bound as untyped values: a
    // closure cannot be generic.
    pub erased: HashSet<String>,
    // Type parameters over the keys of a type that is not a map: strings.
    pub key_strings: HashSet<String>,
    // Interfaces / classes extended (directly or transitively) by at least one `class`.
    pub extended_by_class: HashSet<String>,
}

// The model of the XML for a target, or what the target cannot express.
pub fn model(xml: &str, target: &str) -> Result<Model, Vec<String>> {
    model_with_options(xml, ModelOptions::for_target(target))
}

pub fn model_with_options(xml: &str, opts: ModelOptions<'_>) -> Result<Model, Vec<String>> {
    let doc = Document::parse(xml).map_err(|e| vec![format!("not well-formed XML: {e}")])?;
    let mut b = Builder {
        opts,
        decls: HashMap::new(),
        model: Model { enums: vec![], num_enums: vec![], classes: vec![], callbacks: vec![], unions: vec![], type_aliases: vec![], maps: vec![], shared: vec![], funcs: vec![], opaque: vec![], opaque_names: BTreeMap::new(), opaque_arity: BTreeMap::new(), skipped: BTreeMap::new(), counted: opts.counted, trap_uncaught: opts.trap_uncaught, packages: BTreeMap::new(), js_refs: BTreeMap::new(), synthetic: HashSet::new(), origins: HashMap::new(), namespaced: HashMap::new(), namespace_modules: HashMap::new(), func_namespaces: HashMap::new(), requested_packages: Vec::new(), prefix: doc.root_element().attribute("prefix").map(str::to_string), root_import: doc.root_element().attribute("import").map(str::to_string), original_type_names: BTreeMap::new() },
        opaque: HashSet::new(),
        imports: HashSet::new(),
        resolving: Vec::new(),
        union_kept: HashMap::new(),
        keyed: HashMap::new(),
        bad: None,
        problems: Default::default(),
        module: String::new(),
        generics: opts.generics,
        last_type_params: Vec::new(),
        folders: HashMap::new(),
        folder_names: HashMap::new(),
        package: None,
        root_import: doc.root_element().attribute("import").map(str::to_string),
        multi: doc.root_element().has_attribute("packages"),
        namespace_names: HashMap::new(),
        js_prefix: String::new(),
        declared: Vec::new(),
        has_ids: doc.descendants().any(|d| d.has_attribute("id")),
        idents: Default::default(),
        ident_taken: Default::default(),
        erased: HashSet::new(),
        key_strings: HashSet::new(),
        extended_by_class: HashSet::new(),
    };
    b.index(doc.root_element(), "");
    let mut extended = HashSet::new();
    let mut todo = Vec::new();
    let collect_edges = |d: Node<'_, '_>, out: &mut Vec<String>| {
        for e in elements(d) {
            let key = if e.has_tag_name("ancestor") {
                Some(b.ref_key(e))
            } else if e.has_tag_name("extends") {
                let base = only_child(e);
                base.has_tag_name("ref").then(|| b.ref_key(base))
            } else {
                None
            };
            if let Some(k) = key {
                out.push(k);
            }
        }
    };
    for &d in b.decls.values() {
        if d.has_tag_name("class") {
            let mut edges = Vec::new();
            collect_edges(d, &mut edges);
            for k in edges {
                if extended.insert(k.clone()) {
                    todo.push(k);
                }
            }
        }
    }
    while let Some(cur) = todo.pop() {
        let Some(&decl) = b.decls.get(&cur) else { continue };
        let mut edges = Vec::new();
        collect_edges(decl, &mut edges);
        for k in edges {
            if extended.insert(k.clone()) {
                todo.push(k);
            }
        }
    }
    b.extended_by_class = extended;
    for nested in [false, true] {
        for path in b.declared.iter().filter(|p| p.contains('.') == nested) {
            b.type_ident(path);
        }
    }
    b.model.requested_packages = doc.root_element().attribute("packages").map(|p| p.split(' ').map(str::to_string).collect()).unwrap_or_default();
    b.decls_of(doc.root_element(), "");
    b.nest_generated(doc.root_element());
    // JavaScript's built-in `Function` (used by `TimerHandler`, Konva, etc.)
    // is a callable variadic callback rather than an unconstructible opaque handle.
    if b.opaque.remove("Function") && !b.model.callbacks.iter().any(|c| c.name == "Function") && !b.model.classes.iter().any(|c| c.name == "Function") {
        let params = vec![("args".into(), Ty::Array)];
        b.module = b.model.funcs.first().map_or_else(|| b.model.classes.first().map_or_else(|| "areas/main".into(), |c| c.module.clone()), |f| f.module.clone());
        b.caller("Function", &params, &Ty::Any, &[], true);
        b.model.callbacks.push(Callback { type_params: vec![], name: "Function".into(), params, rest: true, ret: Ty::Any });
    }
    let mut opaque: Vec<String> = b.opaque.into_iter().collect();
    opaque.sort();
    b.model.opaque = opaque;
    let m = &mut b.model;
    // A constraint that is an opaque type nothing is known to extend
    // (`T extends ArrayBufferLike`) holds for no type: none.
    // Targets such as Gleam retain opaque constraints so `$same` opaque types like `Date` are recognized.
    if !opts.retain_opaque_constraints {
        let opaque: HashSet<String> = m.opaque.iter().cloned().collect();
        let unconstrain = |ps: &mut Vec<TParam>| {
            for p in ps.iter_mut() {
                if matches!(&p.constraint, Some(Ty::Obj(n)) if opaque.contains(n)) {
                    let c = p.constraint.take();
                    if p.default.is_none() {
                        p.default = c;
                    }
                }
            }
        };
        m.classes.iter_mut().for_each(|c| {
            unconstrain(&mut c.type_params);
            c.funcs.iter_mut().for_each(|f| unconstrain(&mut f.type_params));
        });
        m.funcs.iter_mut().for_each(|f| unconstrain(&mut f.type_params));
        m.unions.iter_mut().for_each(|u| unconstrain(&mut u.type_params));
        m.callbacks.iter_mut().for_each(|c| unconstrain(&mut c.type_params));
    }
    b.problems.into_inner().result(b.model)
}

// A reference to a type declared elsewhere, by this and its name.
const EXTERNAL: &str = "external:";

// The name of a type declared elsewhere, or the key as it is.
pub(super) fn external(key: &str) -> &str {
    key.strip_prefix(EXTERNAL).unwrap_or(key)
}

fn join(prefix: &str, name: &str) -> String {
    if prefix.is_empty() { name.to_string() } else { format!("{prefix}.{name}") }
}

impl<'a> Builder<'a> {
    pub(super) fn skip(&mut self, why: &str) {
        *self.model.skipped.entry(why.to_string()).or_default() += 1;
    }

    fn index(&mut self, scope: Node<'a, 'a>, prefix: &str) {
        for d in elements(scope) {
            let Some(name) = d.attribute("name") else { continue };
            let key = join(prefix, name);
            match d.tag_name().name() {
                "namespace" => self.index(d, &key),
                // A paired class also has a `var`; the type wins.
                "var" | "function" => {}
                _ => {
                    self.declared.push(key.clone());
                    self.decls.insert(key, d);
                }
            }
        }
    }

    // A function calling a callback, given it: a callback a value holds
    // (`view.dispatch`, `table.getRowModel`) is called with it.
    fn caller(&mut self, owner: &str, params: &[(String, Ty)], ret: &Ty, type_params: &[TParam], rest: bool) {
        let f = if type_params.is_empty() { Ty::Obj(owner.to_string()) } else { Ty::Generic(owner.to_string(), type_params.iter().map(|p| Ty::Param(p.name.clone())).collect()) };
        let import = self.import(format!("{owner}_call"));
        let params = std::iter::once(("f".to_string(), f)).chain(params.iter().cloned()).collect();
        self.model.funcs.push(Func { type_params: type_params.to_vec(), module: self.module.clone(), import, name: format!("call_{owner}"), call: Call::Apply, params, rest, fixed: vec![], shared: None, ret: ret.clone(), returns_this: false });
    }

    // Where a reference leads: the declaration its `to` names; with none, a
    // type declared elsewhere, apart from any declared here of its name
    // (GeoJSON's `Point` beside leaflet's). XML without ids is by name.
    pub(super) fn ref_key(&self, n: Node) -> String {
        let name = n.attribute("qualified").unwrap_or(attr(n, "name"));
        match n.attribute("to") {
            Some(to) => to.to_string(),
            None if self.has_ids => format!("{EXTERNAL}{name}"),
            None => name.to_string(),
        }
    }

    // A type's identifier, by its path in the XML: the path flattened
    // (`cluster_Worker` for `cluster.Worker`), numbered where another path
    // flattens alike. Declared types have theirs first: a top-level one its
    // own name, before a namespace's flattened.
    pub(super) fn type_ident(&self, path: &str) -> String {
        if let Some(i) = self.idents.borrow().get(path) {
            return i.clone();
        }
        let base = ident(external(path));
        let mut name = base.clone();
        let mut n = 2;
        while !self.ident_taken.borrow_mut().insert(name.clone()) {
            name = format!("{base}_{n}");
            n += 1;
        }
        self.idents.borrow_mut().insert(path.to_string(), name.clone());
        name
    }

    pub(super) fn import(&mut self, base: String) -> String {
        let mut name = base.clone();
        let mut n = 2;
        while !self.imports.insert(name.clone()) {
            name = format!("{base}_{n}");
            n += 1;
        }
        name
    }

    // The folder of a module, unique even where letter case does not count.
    fn folder(&mut self, kind: &str, module: &str) -> String {
        if let Some(f) = self.folder_names.get(&(kind.to_string(), module.to_string())) {
            return f.clone();
        }
        let path = file_name(&format!("{kind}/{module}"), &mut self.folders);
        self.folder_names.insert((kind.to_string(), module.to_string()), path.clone());
        path
    }

    // A type parameter, with its default and constraint: `E extends Element = Element`.
    pub(super) fn tparam(&mut self, p: Node<'a, 'a>) -> TParam {
        let mut part = |tag: &str| elements(p).find(|c| c.has_tag_name(tag)).map(|d| only_child(d)).map(|t| self.ty(t, ""));
        let default = part("default");
        let constraint = part("constraint");
        TParam { name: attr(p, "name").to_string(), default, constraint }
    }

    // Notes an element the target has no form for.
    pub(super) fn unsupported(&self, n: Node) {
        self.problems.borrow_mut().element(self.opts.target, n);
    }

    // The type of an alias. Its type parameters come first; only where generics
    // are kept can it have any.
    pub(super) fn alias_body(&self, d: Node<'a, 'a>) -> Node<'a, 'a> {
        if !self.generics && let Some(p) = named(d, "typeParameter").next() {
            self.unsupported(p);
        }
        elements(d).find(|c| !c.has_tag_name("typeParameter") && !c.has_tag_name("extends")).unwrap_or_else(|| only_child(d))
    }

    fn decls_of(&mut self, scope: Node<'a, 'a>, prefix: &str) {
        for d in elements(scope) {
            // Members of a namespace are in the namespace's module. The module is
            // a folder: `areas/css`, or `namespaces/CSS` for one from the library.
            if let Some(m) = d.attribute("module") {
                let kind = if d.attribute("moduleKind") == Some("namespace") { "namespaces" } else { "areas" };
                self.module = self.folder(kind, m);
            }
            let name = d.attribute("name").unwrap_or_default();
            let path = join(prefix, name);
            // A namespace's members are in its package.
            if prefix.is_empty() {
                self.package = d.attribute("import").map(str::to_string).or_else(|| self.root_import.clone());
                if let Some(package) = &self.package {
                    self.model.packages.insert(path.clone(), package.clone());
                }
            }
            if !d.has_tag_name("lowering") {
                let js = self.js(d, &path);
                self.model.js_refs.insert(path.clone(), js);
            }
            if flag(d, "synthetic") {
                self.model.synthetic.insert(self.type_ident(&path));
            }
            // Declared in a namespace (`Stream.PipeOptions`): it gives way to a
            // top-level declaration a target would name the same.
            if !flag(d, "synthetic")
                && matches!(d.tag_name().name(), "interface" | "class" | "typeAlias" | "enum")
                && let Some((full, depth, module)) = self.scope(&path)
            {
                let full = self.exported_as(d, full);
                self.model.namespaced.insert(self.type_ident(&path), full);
                self.model.namespace_modules.insert(self.type_ident(&path), (module, depth));
            }
            if let Some(origin) = d.attribute("origin") {
                self.model.origins.insert(self.type_ident(&path), origin.to_string());
            }
            match d.tag_name().name() {
                "lowering" => {}
                "namespace" => {
                    // A package's namespace by the name it exports (`pb`'s `util`).
                    if prefix.is_empty() {
                        let own = self.exported_name(d, name);
                        self.namespace_names.insert(name.to_string(), own);
                    }
                    let inner = join(&self.js_prefix, d.attribute("jsName").unwrap_or(name));
                    let outer = std::mem::replace(&mut self.js_prefix, inner);
                    self.decls_of(d, &path);
                    self.js_prefix = outer;
                }
                "enum" => {
                    // Only string enums cross as an index; numeric ones are numbers.
                    if named(d, "member").all(|m| attr(m, "type") == "string") {
                        self.model.enums.push((self.type_ident(&path), named(d, "member").map(|v| attr(v, "value").to_string()).collect()));
                    } else if named(d, "member").all(|m| attr(m, "type") == "number") {
                        let members = named(d, "member").map(|v| (ident(attr(v, "name")), attr(v, "value").to_string())).collect();
                        self.model.num_enums.push((self.type_ident(&path), members));
                    }
                }
                // An interface that is only a call signature is a function type.
                "interface" if is_function_interface(d) => {
                    let owner = self.type_ident(&path);
                    let (params, _, _, ret) = self.callback_signature(only_child(d), &owner);
                    let rest = named(only_child(d), "parameter").last().is_some_and(|p| flag(p, "rest"));
                    let type_params = std::mem::take(&mut self.last_type_params);
                    self.caller(&owner, &params, &ret, &type_params, rest);
                    self.model.callbacks.push(Callback { name: owner, params, ret, type_params, rest });
                }
                "interface" | "class" => {
                    let c = self.class_at(d, &path, &self.js(d, &path));
                    self.model.classes.push(c);
                }
                "typeAlias" => {
                    let body = self.alias_body(d);
                    let alias_params: Vec<TParam> = named(d, "typeParameter").collect::<Vec<_>>().into_iter().map(|p| self.tparam(p)).collect();
                    if body.has_tag_name("functionType") {
                        let owner = self.type_ident(&path);
                        let (params, _, _, ret) = self.callback_signature(body, &owner);
                        let rest = named(body, "parameter").last().is_some_and(|p| flag(p, "rest"));
                        self.caller(&owner, &params, &ret, &alias_params, rest);
                        self.model.callbacks.push(Callback { name: owner, params, ret, type_params: alias_params.clone(), rest });
                    }
                    // A named object type is an interface in all but syntax.
                    if body.has_tag_name("object") {
                        let mut c = self.class(body, &path);
                        c.type_params = alias_params;
                        let mut ext_names = HashSet::new();
                        for ext in named(d, "extends") {
                            self.member(&mut c, &mut ext_names, ext, &path, false, false);
                        }
                        self.model.classes.push(c);
                    }
                    // A library's name for a named union is declared beside it; other aliases are resolved where they are used.
                    if prefix.is_empty() && named(d, "typeParameter").next().is_none() && body.has_tag_name("ref") && !flag(d, "synthetic") && self.decls.get(&self.ref_key(body)).is_some_and(|t| t.has_tag_name("typeAlias") && elements(*t).any(|c| c.has_tag_name("union"))) {
                        let alias = self.type_ident(&path);
                        if let Ty::NamedUnion(to) = self.ty(body, "") {
                            self.model.type_aliases.push((alias, to));
                        }
                    }
                }
                "function" => {
                    let (params, rest, fixed, ret) = self.signature(d, "");
                    let js = self.js(d, &path);
                    let own = self.exported_name(d, name);
                    let name = ident(&join(prefix, name));
                    let import = self.import(name.clone());
                    self.namespace_func(prefix, &import, &own);
                    let type_params = std::mem::take(&mut self.last_type_params);
                    self.model.funcs.push(Func { type_params, module: self.module.clone(), import, name, call: Call::Static(js), params, rest, fixed, shared: None, ret, returns_this: false });
                }
                "var" => {
                    let body = only_child(d);
                    if body.has_tag_name("object") {
                        // A value with members of its own: bound like a class with only statics.
                        // A type may have the same name (`type NodeFilter` and `var NodeFilter`).
                        let name = if self.decls.contains_key(&path) { format!("{}_statics", self.type_ident(&path)) } else { self.type_ident(&path) };
                        // A namespace's, as its types are: `Intl.Locale`'s statics, `intl.Locale_statics`.
                        if let Some((mut full, depth, module)) = self.scope(&path) {
                            if self.decls.contains_key(&path) {
                                let own = full.pop().unwrap_or_default();
                                full.push(format!("{own}_statics"));
                            }
                            self.model.namespaced.insert(name.clone(), full);
                            self.model.namespace_modules.insert(name.clone(), (module, depth));
                        }
                        let c = self.statics(body, &self.js(d, &path), name);
                        self.model.classes.push(c);
                        continue;
                    }
                    let ret = self.ty(body, "");
                    let own = &self.exported_name(d, name);
                    let name = format!("get_{}", ident(&path));
                    let import = self.import(name.clone());
                    self.namespace_func(prefix, &import, &format!("get_{}", ident(own)));
                    let js = self.js(d, &path);
                    self.model.funcs.push(Func { type_params: vec![], module: self.module.clone(), import, name, call: Call::StaticGet(js.clone()), params: vec![], rest: false, fixed: vec![], shared: None, ret: ret.clone(), returns_this: false });
                    // A `const`, or a module's export, cannot be assigned.
                    if flag(d, "readonly") {
                        continue;
                    }
                    let path = js;
                    let set_name = format!("set_{}", ident(&path));
                    let set_import = self.import(set_name.clone());
                    self.namespace_func(prefix, &set_import, &format!("set_{}", ident(own)));
                    self.model.funcs.push(Func { type_params: vec![], module: self.module.clone(), import: set_import, name: set_name, call: Call::StaticSet(path), params: vec![("value".into(), value(ret))], rest: false, fixed: vec![], shared: None, ret: Ty::Void, returns_this: false });
                }
                _ => self.unsupported(d),
            }
        }
    }

    // A type the lowering made from a namespace's declarations is that
    // namespace's (`Intl.DateTimeFormatOptions_day`): where every reference to
    // it is in the namespace, or in another such type of it, and its name is
    // made from there (`Intl_...`). One made from a class the namespace shares
    // its name with (node's `Stream`) is referred to from the class: not.
    fn nest_generated(&mut self, root: Node<'a, 'a>) {
        let generated: Vec<Node> = elements(root).filter(|d| flag(*d, "synthetic") && matches!(d.tag_name().name(), "interface" | "class" | "typeAlias" | "enum")).collect();
        if generated.is_empty() {
            return;
        }
        let ids: HashSet<&str> = generated.iter().filter_map(|d| d.attribute("id")).collect();
        let mut refs: HashMap<&str, Vec<Node>> = HashMap::new();
        for r in root.descendants().filter(|r| r.has_tag_name("ref")) {
            if let Some(to) = r.attribute("to").filter(|t| ids.contains(t)) {
                refs.entry(to).or_default().push(r);
            }
        }
        // Where a reference is: the namespaces it is in, or the generated
        // declaration it is in (whose owner it then shares).
        enum At<'b> {
            Namespaces(Vec<String>),
            Generated(&'b str),
        }
        let at = |r: Node<'a, 'a>| -> At<'a> {
            let decl = r.ancestors().find(|a| a.parent().is_some_and(|p| p == root || p.has_tag_name("namespace")));
            if let Some(d) = decl.filter(|d| d.parent() == Some(root) && flag(*d, "synthetic")) {
                return At::Generated(d.attribute("id").unwrap_or_default());
            }
            At::Namespaces(r.ancestors().filter(|a| a.has_tag_name("namespace")).map(|a| attr(a, "name").to_string()).collect::<Vec<_>>().into_iter().rev().collect())
        };
        let mut owner: HashMap<&str, Vec<String>> = HashMap::new();
        for _ in 0..8 {
            let mut changed = false;
            for d in &generated {
                let id = d.attribute("id").unwrap_or_default();
                let places: Vec<Vec<String>> = refs.get(id).into_iter().flatten().map(|r| match at(*r) {
                    At::Namespaces(ns) => ns,
                    At::Generated(g) => owner.get(g).cloned().unwrap_or_default(),
                }).collect();
                let common = match places.split_first() {
                    Some((first, rest)) if !first.is_empty() && rest.iter().all(|p| p == first) => first.clone(),
                    _ => Vec::new(),
                };
                if owner.get(id).cloned().unwrap_or_default() != common {
                    owner.insert(id, common);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        for d in &generated {
            let id = d.attribute("id").unwrap_or_default();
            let Some(ns) = owner.get(id).filter(|ns| !ns.is_empty()) else { continue };
            let Some(own) = id.strip_prefix(&format!("{}_", ns.join("_"))) else { continue };
            // In the namespace's module, as its own declarations are; with
            // several packages, in its package's (`fs`'s `readFile`).
            let Some(top) = root.descendants().find(|n| n.has_tag_name("namespace") && n.parent() == Some(root) && attr(*n, "name") == ns[0]) else { continue };
            let Some(module) = top.attribute("module") else { continue };
            let package = top.attribute("import").map(str::to_string).or_else(|| self.root_import.clone());
            let module = self.folder("namespaces", module);
            let Some((full, depth, module)) = self.scope_in(package.as_deref(), &ns.iter().cloned().chain([own.to_string()]).collect::<Vec<_>>().join("."), module) else { continue };
            let name = self.type_ident(id);
            self.model.namespaced.insert(name.clone(), full);
            self.model.namespace_modules.insert(name, (module, depth));
        }
    }

    // A namespace's function, or a variable's accessor: its namespace and its
    // own name (`Geometry`, `distance`), by its import.
    fn namespace_func(&mut self, prefix: &str, import: &str, own: &str) {
        if let Some((mut full, depth, module)) = self.scope(&join(prefix, own)) {
            full.pop();
            self.model.func_namespaces.insert(import.to_string(), (full, own.to_string(), module, depth));
        }
    }

    // A package's declaration by the name it exports, not one it was given
    // apart from another package's (`pb_Node` is `pb`'s `Node`); not
    // `default`, which names no declaration.
    fn exported_name(&self, d: Node, name: &str) -> String {
        // A path (`default.Stage`) is where it is, not what it is exported as.
        match d.attribute("jsName").filter(|j| *j != "default" && !j.contains('.') && self.multi && self.package.is_some() && !d.ancestors().skip(1).any(|a| a.has_tag_name("namespace"))) {
            // An overload, named for what sets it apart (`circle_with_radius`
            // of `circle`), keeps that name; a name given apart from another
            // package's (`pb_pick`) is the one exported.
            Some(js) if name.starts_with(&format!("{js}_")) => name.to_string(),
            Some(js) => js.to_string(),
            None => name.to_string(),
        }
    }

    fn exported_as(&self, d: Node, mut full: Vec<String>) -> Vec<String> {
        if let Some(last) = full.last_mut() {
            *last = self.exported_name(d, last);
        }
        full
    }

    // Where a declaration of this path is, if not at the top level: its
    // namespaces (`Stream.PipeOptions`), and with several packages its
    // package's module first (`prosemirror-model` is `prosemirror_model`,
    // `fs/promises` `fs`, `promises`, `@dimforge/rapier3d-compat`
    // `rapier3d_compat`). A namespace of its package's name (node's `stream`
    // `Stream`, `path`) is that module. With how many of the segments its
    // module stands for.
    fn scope(&mut self, path: &str) -> Option<(Vec<String>, usize, String)> {
        let package = self.package.clone();
        self.scope_in(package.as_deref(), path, self.module.clone())
    }

    // As `scope`, with the module a declaration is in where not its
    // package's: a package's own is its area (`areas/pb`), standing for its
    // segments, its namespaces' in it (`pb`'s `util`).
    fn scope_in(&mut self, package: Option<&str>, path: &str, module: String) -> Option<(Vec<String>, usize, String)> {
        let (full, depth, package) = self.scope_parts(package, path)?;
        let module = match package {
            Some(p) => self.folder("areas", &p),
            None => module,
        };
        Some((full, depth, module))
    }

    fn scope_parts(&self, package: Option<&str>, path: &str) -> Option<(Vec<String>, usize, Option<String>)> {
        let mut parts: Vec<String> = path.split('.').map(str::to_string).collect();
        if parts.len() > 1
            && let Some(own) = self.namespace_names.get(&parts[0])
        {
            parts[0] = own.clone();
        }
        let scoped = package.filter(|p| self.multi && Some(*p) != self.root_import.as_deref());
        let segments: Vec<String> = match scoped {
            Some(p) => {
                let p = p.trim_start_matches("node:");
                let p = if p.starts_with('@') { p.split_once('/').map_or(p, |x| x.1) } else { p };
                p.split('/').filter(|s| !s.is_empty()).map(ident).collect()
            }
            None => Vec::new(),
        };
        if segments.is_empty() {
            return (parts.len() > 1).then_some((parts, 1, None));
        }
        let same = |a: &str, b: &str| a.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).eq(b.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase));
        let depth = segments.len();
        let package = scoped.map(str::to_string);
        if parts.len() > 1 && same(&parts[0], segments.last().unwrap()) {
            return Some((segments.into_iter().chain(parts.into_iter().skip(1)).collect(), depth, package));
        }
        Some((segments.into_iter().chain(parts).collect(), depth, package))
    }

    // Where a declaration is reached: its JavaScript path, in its package when
    // it is in one (`prosemirror-model::Node`).
    fn js(&self, d: Node, path: &str) -> String {
        let own = d.attribute("jsName").unwrap_or_else(|| path.rsplit('.').next().unwrap_or(path));
        let js = join(&self.js_prefix, own);
        match &self.package {
            Some(package) => format!("{package}::{js}"),
            None => js,
        }
    }
}
