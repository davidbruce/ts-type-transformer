// What a wasm target needs from the XML, and the JavaScript glue both wasm
// generators share.
//
// The boundary: numbers are f64, booleans i32, strings a pointer and a length
// going in. Everything else (objects, strings coming back, arrays, callbacks,
// `any`) is an integer handle into a table the glue keeps; 0 is null.
// A callback is a handle made from a wasm function pointer and a context pointer.

use std::collections::{BTreeMap, HashMap, HashSet};

use roxmltree::{Document, Node};

use super::{attr, elements, flag, named, only_child, unsupported};

#[derive(Clone, PartialEq, Debug)]
pub enum Ty {
    Void,
    Num,
    Bool,
    Str,
    // A JavaScript value of unknown type.
    Any,
    // A handle with a named type: class, interface, callback or external type.
    Obj(String),
    // A string enum crosses as the index of its value.
    Enum(String),
    // A numeric enum crosses as its number.
    NumEnum(String),
    Array,
    // The flag says the absent value is `undefined`, not `null`.
    Nullable(Box<Ty>, bool),
    // Zig only: a parameter that takes one of several simple types.
    Union(Vec<Ty>),
    // A union declared by name, in `Model::unions`.
    NamedUnion(String),
    // A key of a map (`K extends keyof M`): crosses as its index in `Model::maps`.
    Key(String),
    // The value a key selects (`M[K]`): a handle.
    Mapped(String),
    // A callback taking one handle, whose type may depend on a key.
    Listener(Box<Ty>),
    // A type parameter, and a generic type with its arguments (`Box<T>`).
    // Only where generics are kept (C++); its values cross as handles.
    Param(String),
    Generic(String, Vec<Ty>),
    // An array of a known element type, where generics are kept.
    ArrayOf(Box<Ty>),
}

#[derive(Clone)]
pub enum Call {
    Method(String),
    Get(String),
    Set(String),
    // The rest are reached from a global path such as `WebAssembly.Module`.
    New(String),
    Static(String),
    StaticGet(String),
    StaticSet(String),
    // An empty object, for interfaces used as option bags.
    Create,
    // `object[key]`, from an index signature.
    IndexGet,
    IndexSet,
}

pub struct Func {
    // Its own type parameters, where generics are kept.
    pub type_params: Vec<TParam>,
    // The module a top-level function is in; empty for members.
    pub module: String,
    // Name of the wasm import, unique in the module.
    pub import: String,
    // Name in the target language, unique in its class.
    pub name: String,
    pub call: Call,
    pub params: Vec<(String, Ty)>,
    // The last parameter is an array spread into the call.
    pub rest: bool,
    // Arguments with one possible value: position in the call, JavaScript literal.
    pub fixed: Vec<(usize, String)>,
    // An import shared with functions that differ only in their fixed
    // arguments, and this function's index among them.
    pub shared: Option<(String, usize)>,
    pub ret: Ty,
}

pub struct Class {
    // Its type parameters, and its direct bases with their arguments, where generics are kept.
    pub type_params: Vec<TParam>,
    pub bases: Vec<Ty>,
    // The module it is in, from the XML.
    pub module: String,
    pub name: String,
    pub ancestors: Vec<String>,
    pub funcs: Vec<Func>,
    // Name, value as source text.
    pub consts: Vec<(String, String)>,
    // What iterating it yields, from `[Symbol.iterator]()`.
    pub iterates: Option<Ty>,
}

pub struct NamedUnion {
    pub name: String,
    // Where generics are kept, a union alias can be generic: `Result<T> = Ok<T> | Failed`.
    pub type_params: Vec<TParam>,
    pub parts: Vec<Ty>,
    // How each member is recognised at runtime, from the XML.
    pub tests: Vec<Option<String>>,
    pub discriminant: Option<String>,
}

impl NamedUnion {
    // A value coming back can be told apart: every member has a test, but
    // one. A bare type parameter is told apart where its argument is known,
    // if it is a number, a string or a flag.
    pub fn decodable(&self) -> bool {
        let untested = self.tests.iter().zip(&self.parts).filter(|(t, p)| t.is_none() && !matches!(p, Ty::Param(_))).count();
        untested <= 1 && self.params() <= 1
    }

    pub fn params(&self) -> usize {
        self.parts.iter().filter(|p| matches!(p, Ty::Param(_))).count()
    }
}

// A map whose keys select types: `HTMLElementEventMap`.
pub struct KeyMap {
    pub name: String,
    pub keys: Vec<String>,
    pub values: Vec<Ty>,
}

// A type parameter: `E extends Element = Element`.
#[derive(Clone, PartialEq, Debug)]
pub struct TParam {
    pub name: String,
    pub default: Option<Ty>,
    pub constraint: Option<Ty>,
}

pub struct Callback {
    pub type_params: Vec<TParam>,
    pub name: String,
    pub params: Vec<(String, Ty)>,
    pub ret: Ty,
}

// One import for functions that differ only in their fixed arguments:
// `addEventListener("click", ...)`, `addEventListener("keydown", ...)`.
pub struct Shared {
    pub name: String,
    // The glue: its parameters after the index, and its body, in which
    // `$F<j>` stands for the j-th fixed argument.
    pub params: Vec<String>,
    pub body: String,
    // The fixed arguments of each function, by index.
    pub values: Vec<Vec<String>>,
}

pub struct Model {
    pub enums: Vec<(String, Vec<String>)>,
    // Name, then member names and values.
    pub num_enums: Vec<(String, Vec<(String, String)>)>,
    pub classes: Vec<Class>,
    pub callbacks: Vec<Callback>,
    pub unions: Vec<NamedUnion>,
    pub maps: Vec<KeyMap>,
    pub shared: Vec<Shared>,
    pub funcs: Vec<Func>,
    // Types declared elsewhere: handles with no members.
    pub opaque: Vec<String>,
    // How many type arguments those that are generic take.
    pub opaque_arity: BTreeMap<String, usize>,
    // What the generator does not bind yet, by reason.
    pub skipped: BTreeMap<String, usize>,
    // Handles are counted references: wasm retains and releases them, and
    // owns what a callback returns. Else a handle lives as long as the page.
    pub counted: bool,
}

impl Class {
    pub fn has_self(f: &Func) -> bool {
        matches!(f.call, Call::Method(_) | Call::Get(_) | Call::Set(_) | Call::IndexGet | Call::IndexSet)
    }
}

// A file or folder name that no other one in `taken` equals ignoring letter
// case, and that is not reserved on Windows: `foo` after `Foo` is `foo_2`.
pub fn file_name(path: &str, taken: &mut HashMap<String, String>) -> String {
    let (dir, name) = path.rsplit_once('/').map_or(("", path), |(d, n)| (d, n));
    let reserved = |n: &str| {
        let n = n.to_ascii_uppercase();
        matches!(n.as_str(), "CON" | "PRN" | "AUX" | "NUL") || (n.len() == 4 && (n.starts_with("COM") || n.starts_with("LPT")) && n.as_bytes()[3].is_ascii_digit())
    };
    let mut candidate = if reserved(name) { format!("{name}_") } else { name.to_string() };
    let mut n = 2;
    loop {
        let full = if dir.is_empty() { candidate.clone() } else { format!("{dir}/{candidate}") };
        match taken.get(&full.to_lowercase()) {
            Some(existing) if *existing != full => {
                candidate = format!("{name}_{n}");
                n += 1;
            }
            _ => {
                taken.insert(full.to_lowercase(), full.clone());
                return full;
            }
        }
    }
}

// A name usable in C, Zig and JavaScript.
pub fn ident(name: &str) -> String {
    let mut s: String = name.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' }).collect();
    if s.is_empty() || s.as_bytes()[0].is_ascii_digit() {
        s.insert(0, '_');
    }
    s
}

struct Builder<'a> {
    target: &'a str,
    // Every declaration by qualified name.
    decls: HashMap<String, Node<'a, 'a>>,
    model: Model,
    opaque: HashSet<String>,
    imports: HashSet<String>,
    // Aliases being resolved, to stop on recursive ones.
    resolving: Vec<String>,
    // In the signature being read: type parameters over a map's keys, and their map.
    keyed: HashMap<String, String>,
    // Why the member being read cannot be bound, if it cannot.
    bad: Option<String>,
    // The module of the declaration being read.
    module: String,
    // Whether generics are kept (C++), and the type parameters of the signature just read.
    generics: bool,
    last_type_params: Vec<TParam>,
    // Module folders given out, by lowercased path: names that differ only in case get a suffix.
    folders: HashMap<String, String>,
    folder_names: HashMap<(String, String), String>,
}

pub fn model(xml: &str, target: &str) -> Model {
    let doc = Document::parse(xml).expect("well-formed XML");
    let mut b = Builder {
        target,
        decls: HashMap::new(),
        model: Model { enums: vec![], num_enums: vec![], classes: vec![], callbacks: vec![], unions: vec![], maps: vec![], shared: vec![], funcs: vec![], opaque: vec![], opaque_arity: BTreeMap::new(), skipped: BTreeMap::new(), counted: target == "C++" },
        opaque: HashSet::new(),
        imports: HashSet::new(),
        resolving: Vec::new(),
        keyed: HashMap::new(),
        bad: None,
        module: String::new(),
        generics: target == "C++",
        last_type_params: Vec::new(),
        folders: HashMap::new(),
        folder_names: HashMap::new(),
    };
    b.index(doc.root_element(), "");
    b.decls_of(doc.root_element(), "");
    b.share_imports();
    let mut opaque: Vec<String> = b.opaque.into_iter().collect();
    opaque.sort();
    b.model.opaque = opaque;
    b.model
}

fn join(prefix: &str, name: &str) -> String {
    if prefix.is_empty() { name.to_string() } else { format!("{prefix}.{name}") }
}

impl<'a> Builder<'a> {
    fn skip(&mut self, why: &str) {
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
                    self.decls.insert(key, d);
                }
            }
        }
    }

    fn import(&mut self, base: String) -> String {
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
    fn tparam(&mut self, p: Node<'a, 'a>) -> TParam {
        let mut part = |tag: &str| elements(p).find(|c| c.has_tag_name(tag)).map(|d| only_child(d)).map(|t| self.ty(t, ""));
        let default = part("default");
        let constraint = part("constraint");
        TParam { name: attr(p, "name").to_string(), default, constraint }
    }

    // The type of an alias. Its type parameters come first; only where generics
    // are kept can it have any.
    fn alias_body(&self, d: Node<'a, 'a>) -> Node<'a, 'a> {
        if !self.generics {
            return only_child_after_params(d, self.target);
        }
        elements(d).find(|c| !c.has_tag_name("typeParameter")).unwrap_or_else(|| only_child(d))
    }

    // Functions whose glue is the same but for their fixed arguments share one
    // import, which takes the index of the function's arguments.
    fn share_imports(&mut self) {
        let mut groups: BTreeMap<String, Vec<(usize, usize)>> = BTreeMap::new();
        for (ci, c) in self.model.classes.iter().enumerate() {
            for (fi, f) in c.funcs.iter().enumerate().filter(|(_, f)| !f.fixed.is_empty()) {
                let (params, body) = entry(f, &|j, _| format!("$F{j}"));
                groups.entry(format!("{}|{body}", params.join(","))).or_default().push((ci, fi));
            }
        }
        for (_, members) in groups.into_iter().filter(|g| g.1.len() > 1) {
            let (ci, fi) = members[0];
            let first = &self.model.classes[ci].funcs[fi];
            let (params, body) = entry(first, &|j, _| format!("$F{j}"));
            // Named after what it calls: `addEventListener_keyed`.
            let what = match &first.call {
                Call::Method(js) => js.clone(),
                Call::New(path) | Call::Static(path) => path.rsplit('.').next().unwrap_or_default().to_string(),
                _ => first.import.clone(),
            };
            let name = self.import(format!("{}_keyed", ident(&what)));
            let mut values = Vec::new();
            for (k, (ci, fi)) in members.into_iter().enumerate() {
                let f = &mut self.model.classes[ci].funcs[fi];
                values.push(f.fixed.iter().map(|x| x.1.clone()).collect());
                f.shared = Some((name.clone(), k));
            }
            self.model.shared.push(Shared { name, params, body, values });
        }
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
            match d.tag_name().name() {
                "lowering" => {}
                "namespace" => self.decls_of(d, &path),
                "enum" => {
                    // Only string enums cross as an index; numeric ones are numbers.
                    if named(d, "member").all(|m| attr(m, "type") == "string") {
                        self.model.enums.push((ident(&path), named(d, "member").map(|v| attr(v, "value").to_string()).collect()));
                    } else if named(d, "member").all(|m| attr(m, "type") == "number") {
                        let members = named(d, "member").map(|v| (ident(attr(v, "name")), attr(v, "value").to_string())).collect();
                        self.model.num_enums.push((ident(&path), members));
                    }
                }
                // An interface that is only a call signature is a function type.
                "interface" if is_function_interface(d) => {
                    let owner = ident(&path);
                    let (params, _, _, ret) = self.signature(only_child(d), &owner);
                    self.model.callbacks.push(Callback { name: owner, params, ret, type_params: std::mem::take(&mut self.last_type_params) });
                }
                "interface" | "class" => {
                    let c = self.class(d, &path);
                    self.model.classes.push(c);
                }
                "typeAlias" => {
                    let body = self.alias_body(d);
                    let alias_params: Vec<TParam> = named(d, "typeParameter").collect::<Vec<_>>().into_iter().map(|p| self.tparam(p)).collect();
                    if body.has_tag_name("functionType") {
                        let owner = ident(&path);
                        let (params, _, _, ret) = self.signature(body, &owner);
                        self.last_type_params = alias_params.clone();
                        self.model.callbacks.push(Callback { name: owner, params, ret, type_params: std::mem::take(&mut self.last_type_params) });
                    }
                    // A named object type is an interface in all but syntax.
                    if body.has_tag_name("object") {
                        let mut c = self.class(body, &path);
                        c.type_params = alias_params;
                        self.model.classes.push(c);
                    }
                    // Other aliases are resolved where they are used.
                }
                "function" => {
                    let (params, rest, fixed, ret) = self.signature(d, "");
                    let js = join(prefix, d.attribute("jsName").unwrap_or(name));
                    let name = ident(&join(prefix, name));
                    let import = self.import(name.clone());
                    let type_params = std::mem::take(&mut self.last_type_params);
                    self.model.funcs.push(Func { type_params, module: self.module.clone(), import, name, call: Call::Static(js), params, rest, fixed, shared: None, ret });
                }
                "var" => {
                    let body = only_child(d);
                    if body.has_tag_name("object") {
                        // A value with members of its own: bound like a class with only statics.
                        // A type may have the same name (`type NodeFilter` and `var NodeFilter`).
                        let name = if self.decls.contains_key(&path) { format!("{}_statics", ident(&path)) } else { ident(&path) };
                        let c = self.statics(body, &path, name);
                        self.model.classes.push(c);
                        continue;
                    }
                    let ret = self.ty(body, "");
                    let name = format!("get_{}", ident(&path));
                    let import = self.import(name.clone());
                    self.model.funcs.push(Func { type_params: vec![], module: self.module.clone(), import, name, call: Call::StaticGet(path), params: vec![], rest: false, fixed: vec![], shared: None, ret });
                }
                _ => unsupported(self.target, d),
            }
        }
    }

    fn class(&mut self, d: Node<'a, 'a>, path: &str) -> Class {
        let owner = ident(path);
        let mut c = Class { type_params: vec![], bases: vec![], module: self.module.clone(), name: owner.clone(), ancestors: Vec::new(), funcs: Vec::new(), consts: Vec::new(), iterates: None };
        let mut names = HashSet::new();
        if !d.has_tag_name("class") && !named(d, "call").any(|_| true) {
            names.insert("create".to_string());
            let import = self.import(format!("{owner}_create"));
            c.funcs.push(Func { type_params: vec![], module: String::new(), import, name: "create".into(), call: Call::Create, params: vec![], rest: false, fixed: vec![], shared: None, ret: Ty::Obj(owner.clone()) });
        }
        for m in elements(d) {
            self.member(&mut c, &mut names, m, path, m.has_tag_name("constructor") || flag(m, "static"));
        }
        c
    }

    fn statics(&mut self, body: Node<'a, 'a>, path: &str, name: String) -> Class {
        let mut c = Class { type_params: vec![], bases: vec![], module: self.module.clone(), name, ancestors: Vec::new(), funcs: Vec::new(), consts: Vec::new(), iterates: None };
        let mut names = HashSet::new();
        for m in elements(body) {
            self.member(&mut c, &mut names, m, path, true);
        }
        c
    }

    fn member(&mut self, c: &mut Class, names: &mut HashSet<String>, m: Node<'a, 'a>, path: &str, is_static: bool) {
        let owner = c.name.clone();
        let raw_name = m.attribute("name").unwrap_or_default();
        let tag = m.tag_name().name();
        if matches!(tag, "ancestor") {
            // A base from another file is still a type to convert to.
            if !self.decls.contains_key(raw_name) {
                self.opaque.insert(ident(raw_name));
            }
            return c.ancestors.push(ident(raw_name));
        }
        // A direct base, when inheritance is kept: members stay on the type that declares them.
        if tag == "extends" {
            let base = only_child(m);
            if base.has_tag_name("ref") && (elements(base).next().is_none() || self.generics) {
                let name = base.attribute("qualified").unwrap_or(attr(base, "name"));
                let t = self.reference(base);
                if !self.decls.contains_key(name) {
                    self.opaque.insert(ident(name));
                }
                c.bases.push(t);
                return c.ancestors.push(ident(name));
            }
            return self.skip("generic base");
        }
        if tag == "typeParameter" {
            if !self.generics {
                unsupported(self.target, m);
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
                c.funcs.push(Func { type_params: vec![], module: String::new(), import, name: "get_at".into(), call: Call::IndexGet, params: vec![("key".into(), key.clone())], rest: false, fixed: vec![], shared: None, ret: absent(value.clone(), true) });
                let import = self.import(format!("{owner}_set_at"));
                c.funcs.push(Func { type_params: vec![], module: String::new(), import, name: "set_at".into(), call: Call::IndexSet, params: vec![("key".into(), key), ("value".into(), value)], rest: false, fixed: vec![], shared: None, ret: Ty::Void });
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
                c.funcs.push(Func { type_params: vec![], module: String::new(), import, name: base, call, params, rest: false, fixed: vec![], shared: None, ret });
            }
            self.bad = None;
            return;
        }
        if matches!(tag, "call" | "index" | "get" | "set") || tag == "construct" && !is_static {
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
        if raw_name == "[Symbol.asyncIterator]" {
            return self.skip("async iterator");
        }
        if raw_name.starts_with('[') {
            return self.skip("symbol-named member");
        }
        // A target with overloading keeps one name for all of a method's overloads.
        let overloading = self.target == "C++";
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
        let js = m.attribute("jsName").unwrap_or(raw_name).to_string();
        match tag {
            "constructor" | "construct" => {
                let (params, rest, fixed, _) = self.signature(m, &owner);
                let name = unique(if raw_name.is_empty() { "new".into() } else { ident(raw_name) }, names);
                let import = self.import(format!("{owner}_{name}"));
                c.funcs.push(Func { type_params: vec![], module: String::new(), import, name, call: Call::New(path.to_string()), params, rest, fixed, shared: None, ret: Ty::Obj(owner) });
            }
            "method" => {
                let (params, rest, fixed, ret) = self.signature(m, &owner);
                let call = if is_static { Call::Static(format!("{path}.{js}")) } else { Call::Method(js) };
                let name = unique(ident(raw_name), names);
                let import = self.import(format!("{owner}_{name}"));
                let mut type_params = std::mem::take(&mut self.last_type_params);
                let (mut params, mut ret) = (params, ret);
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
                c.funcs.push(Func { type_params, module: String::new(), import, name, call, params, rest, fixed, shared: None, ret });
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
                let mut t = value(self.ty(only_child(m), &owner));
                if flag(m, "optional") {
                    t = absent(t, true);
                }
                let (get, set) = if is_static {
                    (Call::StaticGet(format!("{path}.{js}")), Call::StaticSet(format!("{path}.{js}")))
                } else {
                    (Call::Get(js.clone()), Call::Set(js))
                };
                let name = unique(format!("get_{}", ident(raw_name)), names);
                let import = self.import(format!("{owner}_{name}"));
                c.funcs.push(Func { type_params: vec![], module: String::new(), import, name, call: get, params: vec![], rest: false, fixed: vec![], shared: None, ret: t.clone() });
                if !flag(m, "readonly") {
                    let name = unique(format!("set_{}", ident(raw_name)), names);
                    let import = self.import(format!("{owner}_{name}"));
                    c.funcs.push(Func { type_params: vec![], module: String::new(), import, name, call: set, params: vec![("value".into(), t)], rest: false, fixed: vec![], shared: None, ret: Ty::Void });
                }
            }
            _ => unsupported(self.target, m),
        }
        if let Some(why) = self.bad.take() {
            c.funcs.truncate(before);
            self.skip(&why);
        }
    }

    fn signature(&mut self, n: Node<'a, 'a>, owner: &str) -> (Vec<(String, Ty)>, bool, Vec<(usize, String)>, Ty) {
        // Type parameters left after `generics = "erase"` range over a map's keys.
        self.keyed.clear();
        self.last_type_params.clear();
        for p in named(n, "typeParameter") {
            let map = elements(p).find(|c| c.has_tag_name("constraint")).map(only_child).filter(|k| k.has_tag_name("keyof")).map(only_child);
            match map {
                Some(m) => {
                    let name = m.attribute("qualified").unwrap_or(attr(m, "name")).to_string();
                    if self.key_map(&name) {
                        self.keyed.insert(attr(p, "name").to_string(), ident(&name));
                    }
                }
                None if self.generics => {
                    let p = self.tparam(p);
                    self.last_type_params.push(p);
                }
                None => unsupported(self.target, p),
            }
        }
        let mut params = Vec::new();
        let mut rest = false;
        let mut fixed = Vec::new();
        let mut ret = Ty::Void;
        for p in elements(n) {
            match p.tag_name().name() {
                // The receiver of a callback is not an argument.
                "this" => {}
                // An argument with one possible value is supplied by the glue.
                "parameter" if p.has_attribute("const") => {
                    let value = attr(p, "const");
                    let literal = if only_child(p).attribute("name") == Some("string") { format!("{value:?}") } else { value.to_string() };
                    fixed.push((params.len() + fixed.len(), literal));
                }
                "parameter" if flag(p, "optional") => unsupported(self.target, p),
                "parameter" => {
                    rest |= flag(p, "rest");
                    params.push((ident(attr(p, "name")), value(self.ty(only_child(p), owner))));
                }
                "returns" => ret = self.ty(only_child(p), owner),
                "typeParameter" => {}
                _ => unsupported(self.target, p),
            }
        }
        (params, rest, fixed, ret)
    }

    // Registers a map whose values are all handles; false, with a reason, if it is not one.
    fn key_map(&mut self, name: &str) -> bool {
        let id = ident(name);
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
                if let Some(b) = self.decls.get(base.attribute("qualified").unwrap_or(attr(base, "name"))) {
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

    fn ty(&mut self, n: Node<'a, 'a>, owner: &str) -> Ty {
        match n.tag_name().name() {
            "primitive" => match attr(n, "name") {
                "number" => Ty::Num,
                "boolean" => Ty::Bool,
                "string" => Ty::Str,
                "void" | "undefined" | "never" => Ty::Void,
                _ => Ty::Any,
            },
            "this" => Ty::Obj(owner.to_string()),
            "ref" if attr(n, "kind") == "typeParameter" => match self.keyed.get(attr(n, "name")) {
                Some(map) => Ty::Key(map.clone()),
                None if self.generics => Ty::Param(attr(n, "name").to_string()),
                None => unsupported(self.target, n),
            },
            "ref" => self.reference(n),
            // `M[K]`: the value the key selects.
            "indexedAccess" => {
                let index = elements(n).nth(1).unwrap();
                match index.attribute("name").and_then(|k| self.keyed.get(k)) {
                    Some(map) => Ty::Mapped(map.clone()),
                    None => unsupported(self.target, n),
                }
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
                absent(inner, attr(n, "absent") == "undefined")
            }
            // Elements that cross as a number, a flag, a string, a handle or a
            // union that can be told apart.
            "array" => match self.ty(only_child(n), owner) {
                e @ (Ty::Num | Ty::Bool | Ty::Str | Ty::Obj(_) | Ty::ArrayOf(_)) => Ty::ArrayOf(Box::new(e)),
                e @ (Ty::Generic(..) | Ty::Param(_)) if self.generics => Ty::ArrayOf(Box::new(e)),
                Ty::NamedUnion(u) if self.model.unions.iter().any(|x| x.name == u && x.decodable()) => Ty::ArrayOf(Box::new(Ty::NamedUnion(u))),
                _ => Ty::Array,
            },
            "array" | "tuple" => Ty::Array,
            "union" => {
                let parts: Vec<Ty> = elements(n).map(|p| self.ty(p, owner)).collect();
                // Only alternatives a tagged union can hold directly.
                if parts.iter().all(|p| matches!(p, Ty::Num | Ty::Str | Ty::Bool | Ty::Obj(_))) && distinct(&parts) {
                    Ty::Union(parts)
                } else {
                    self.skip("union whose members cannot be told apart, bound as an untyped value");
                    Ty::Union(vec![])
                }
            }
            _ => unsupported(self.target, n),
        }
    }

    fn reference(&mut self, n: Node<'a, 'a>) -> Ty {
        let key = n.attribute("qualified").unwrap_or(attr(n, "name")).to_string();
        let declared = self.decls.get(&key).copied();
        match (attr(n, "kind"), declared) {
            ("typeParameter", _) => unsupported(self.target, n),
            // By what is declared: an alias of string literals has become an enum.
            (_, Some(d)) if d.has_tag_name("enum") && named(d, "member").all(|m| attr(m, "type") == "string") => Ty::Enum(ident(&key)),
            (_, Some(d)) if d.has_tag_name("enum") && named(d, "member").all(|m| attr(m, "type") == "number") => Ty::NumEnum(ident(&key)),
            ("enum", _) => Ty::Num,
            (_, Some(d)) if d.has_tag_name("typeAlias") => {
                let body = self.alias_body(d);
                if body.has_tag_name("functionType") || body.has_tag_name("object") {
                    if self.generics && elements(n).next().is_some() {
                        let args: Vec<Ty> = elements(n).map(|a| self.ty(a, "")).collect();
                        return Ty::Generic(ident(&key), args);
                    }
                    return Ty::Obj(ident(&key));
                }
                // A generic union alias, where generics are kept, is a generic union.
                let generic_union = self.generics && body.has_tag_name("union") && named(d, "typeParameter").next().is_some();
                // Another generic alias would need its arguments substituted.
                if named(d, "typeParameter").next().is_some() && !generic_union {
                    return Ty::Any;
                }
                if self.resolving.contains(&key) || self.resolving.len() > 8 {
                    return Ty::Any;
                }
                // A union with a name of its own is declared once.
                if body.has_tag_name("union") && self.target != "C" {
                    let name = ident(&key);
                    let args: Vec<Ty> = if generic_union { elements(n).map(|a| self.ty(a, "")).collect() } else { Vec::new() };
                    // `void` in a union is a value that may be absent: `void | Promise<void>`.
                    let is_void = |p: Node| p.has_tag_name("primitive") && attr(p, "name") == "void";
                    let has_void = elements(body).any(is_void);
                    let named_type = |name: String| {
                        let t = if generic_union { Ty::Generic(name, args.clone()) } else { Ty::NamedUnion(name) };
                        if has_void { absent(t, true) } else { t }
                    };
                    if self.model.unions.iter().any(|u| u.name == name) {
                        return named_type(name);
                    }
                    let type_params: Vec<TParam> = named(d, "typeParameter").collect::<Vec<_>>().into_iter().map(|p| self.tparam(p)).collect();
                    let members: Vec<Node> = elements(body).filter(|p| !is_void(*p)).collect();
                    self.resolving.push(key.clone());
                    let parts: Vec<Ty> = members.iter().map(|p| self.ty(*p, "")).collect();
                    self.resolving.pop();
                    // `X | any` is any.
                    if parts.contains(&Ty::Any) {
                        self.skip("union with an any member, which is any");
                        return Ty::Any;
                    }
                    if let [only] = parts.as_slice() {
                        return if has_void { absent(only.clone(), true) } else { only.clone() };
                    }
                    let simple = |p: &Ty| matches!(p, Ty::Num | Ty::Str | Ty::Bool | Ty::Obj(_) | Ty::Array | Ty::ArrayOf(_)) || self.generics && matches!(p, Ty::Generic(..) | Ty::Param(_));
                    if !parts.iter().all(simple) || !distinct(&parts) {
                        self.skip("union whose members cannot be told apart, bound as an untyped value");
                        return Ty::Any;
                    }
                    let tests = members.iter().map(|p| p.attribute("test").map(str::to_string)).collect();
                    let discriminant = body.attribute("discriminant").map(str::to_string);
                    self.model.unions.push(NamedUnion { name: name.clone(), type_params, parts, tests, discriminant });
                    return named_type(name);
                }
                self.resolving.push(key.clone());
                let t = self.ty(body, "");
                self.resolving.pop();
                t
            }
            ("class" | "interface", declared) if self.generics && elements(n).next().is_some() => {
                let args: Vec<Ty> = elements(n).map(|a| self.ty(a, "")).collect();
                if declared.is_none() {
                    self.opaque.insert(ident(&key));
                    let arity = self.model.opaque_arity.entry(ident(&key)).or_default();
                    *arity = (*arity).max(args.len());
                }
                Ty::Generic(ident(&key), args)
            }
            ("class" | "interface", Some(_)) => Ty::Obj(ident(&key)),
            // Declared in another library: a handle, by name. An alias may be of anything, and a handle can hold anything.
            ("class" | "interface" | "typeAlias", None) => {
                self.opaque.insert(ident(&key));
                Ty::Obj(ident(&key))
            }
            _ => Ty::Any,
        }
    }
}

fn is_function_interface(d: Node) -> bool {
    let mut members = elements(d);
    members.next().is_some_and(|m| m.has_tag_name("call")) && members.next().is_none()
}

// The single type of an alias, which may be preceded by type parameters.
fn only_child_after_params<'a>(d: Node<'a, 'a>, target: &str) -> Node<'a, 'a> {
    if let Some(p) = named(d, "typeParameter").next() {
        unsupported(target, p);
    }
    only_child(d)
}

// No two members alike. Arrays are alike whatever they hold: a test sees only an array.
fn distinct(parts: &[Ty]) -> bool {
    let shape = |p: &Ty| if matches!(p, Ty::ArrayOf(_)) { Ty::Array } else { p.clone() };
    let shapes: Vec<Ty> = parts.iter().map(shape).collect();
    shapes.iter().enumerate().all(|(i, p)| !shapes[..i].contains(p))
}

// `void` is only a result. A parameter or property of type `undefined` holds a value.
fn value(t: Ty) -> Ty {
    if t == Ty::Void { Ty::Any } else { t }
}

// Renames a type parameter wherever a type mentions it.
fn rename_param(t: &mut Ty, from: &str, to: &str) {
    match t {
        Ty::Param(n) if n == from => *n = to.to_string(),
        Ty::Nullable(inner, _) | Ty::Listener(inner) | Ty::ArrayOf(inner) => rename_param(inner, from, to),
        Ty::Generic(_, args) => args.iter_mut().for_each(|a| rename_param(a, from, to)),
        _ => {}
    }
}

// What may be absent. A handle can be null; a string is a null pointer going
// in and a null handle coming back; a number is a flag and the number going
// in, and a handle coming back; a flag or an enum is -1 when absent.
fn absent(t: Ty, undefined: bool) -> Ty {
    match t {
        Ty::Obj(_) | Ty::Array | Ty::ArrayOf(_) | Ty::Str | Ty::Param(_) | Ty::Generic(..) => Ty::Nullable(Box::new(t), undefined),
        Ty::Num | Ty::NumEnum(_) | Ty::Bool | Ty::Enum(_) | Ty::Mapped(_) | Ty::NamedUnion(_) => Ty::Nullable(Box::new(t), undefined),
        Ty::Nullable(..) => t,
        Ty::Void => Ty::Void,
        _ => Ty::Any,
    }
}

impl Model {
    pub fn all_funcs(&self) -> impl Iterator<Item = &Func> {
        self.classes.iter().flat_map(|c| c.funcs.iter()).chain(self.funcs.iter())
    }
}

// JavaScript expression turning a value from wasm into a JavaScript value.
fn from_wasm(p: &str, t: &Ty) -> String {
    match t {
        Ty::Num | Ty::NumEnum(_) => p.to_string(),
        Ty::Bool => format!("!!{p}"),
        Ty::Enum(e) => format!("enums.{e}[{p}]"),
        // A flag or an enum that may be absent is -1 when it is.
        Ty::Nullable(inner, u) if matches!(**inner, Ty::Bool) => format!("({p} < 0 ? {} : !!{p})", if *u { "undefined" } else { "null" }),
        Ty::Nullable(inner, u) if matches!(**inner, Ty::Enum(_)) => {
            let Ty::Enum(e) = &**inner else { unreachable!() };
            format!("({p} < 0 ? {} : enums.{e}[{p}])", if *u { "undefined" } else { "null" })
        }
        Ty::Nullable(_, true) => format!("({p} === 0 ? undefined : get({p}))"),
        Ty::Key(m) => format!("keys.{m}[{p}]"),
        _ => format!("get({p})"),
    }
}

// JavaScript expression turning a JavaScript value into one wasm can take.
fn to_wasm(x: &str, t: &Ty) -> String {
    match t {
        Ty::Void => format!("void ({x})"),
        Ty::Num | Ty::NumEnum(_) => x.to_string(),
        Ty::Bool => format!("(({x}) ? 1 : 0)"),
        Ty::Enum(e) => format!("enums.{e}.indexOf({x})"),
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Bool) => format!("(v => (v == null ? -1 : v ? 1 : 0))({x})"),
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Enum(_)) => {
            let Ty::Enum(e) = &**inner else { unreachable!() };
            format!("(v => (v == null ? -1 : enums.{e}.indexOf(v)))({x})")
        }
        _ => format!("put({x})"),
    }
}

// A union member's runtime test as a JavaScript condition on `v`.
fn js_test(test: &str, discriminant: Option<&str>) -> String {
    let (kind, arg) = test.split_once(':').unwrap_or((test, ""));
    match kind {
        "typeof" => format!("typeof v === {arg:?}"),
        "array" => "Array.isArray(v)".into(),
        "instanceof" => format!("v instanceof globalThis{}", access_path(arg)),
        "has" => format!("typeof v === 'object' && v !== null && {arg:?} in v"),
        "tag" => format!("v != null && String(v[{:?}]) === {arg:?}", discriminant.unwrap_or_default()),
        _ => "false".into(),
    }
}

// The glue of one import: its parameters and body. `fixed` writes the j-th
// fixed argument, given its literal.
fn entry(f: &Func, fixed: &dyn Fn(usize, &str) -> String) -> (Vec<String>, String) {
    let mut wasm_params = Vec::new();
    let mut args = Vec::new();
    if Class::has_self(f) {
        wasm_params.push("self".to_string());
    }
    for (i, (_, t)) in f.params.iter().enumerate() {
        let p = format!("a{i}");
        // A string is a pointer and a length; everything else is one value.
        if *t == Ty::Str {
            wasm_params.push(format!("{p}_ptr"));
            wasm_params.push(format!("{p}_len"));
            args.push(format!("str({p}_ptr, {p}_len)"));
            continue;
        }
        if let Ty::Nullable(inner, undefined) = t
            && **inner == Ty::Str
        {
            wasm_params.push(format!("{p}_ptr"));
            wasm_params.push(format!("{p}_len"));
            args.push(format!("({p}_ptr === 0 ? {} : str({p}_ptr, {p}_len))", if *undefined { "undefined" } else { "null" }));
            continue;
        }
        if let Ty::Nullable(inner, undefined) = t
            && matches!(**inner, Ty::Num | Ty::NumEnum(_))
        {
            wasm_params.push(format!("{p}_has"));
            wasm_params.push(format!("{p}_value"));
            args.push(format!("({p}_has ? {p}_value : {})", if *undefined { "undefined" } else { "null" }));
            continue;
        }
        let spread = if f.rest && i == f.params.len() - 1 { "..." } else { "" };
        args.push(format!("{spread}{}", from_wasm(&p, t)));
        wasm_params.push(p);
    }
    for (j, (position, literal)) in f.fixed.iter().enumerate() {
        args.insert((*position).min(args.len()), fixed(j, literal));
    }
    let (first, second) = (args.first().cloned().unwrap_or_default(), args.get(1).cloned().unwrap_or_default());
    let args = args.join(", ");
    let call = match &f.call {
        Call::Method(js) => format!("get(self){}({args})", access(js)),
        Call::Get(prop) => format!("get(self){}", access(prop)),
        Call::Set(prop) => format!("get(self){} = {args}", access(prop)),
        Call::New(path) => format!("new globalThis{}({args})", access_path(path)),
        Call::Static(path) => format!("globalThis{}({args})", access_path(path)),
        Call::StaticGet(path) => format!("globalThis{}", access_path(path)),
        Call::StaticSet(path) => format!("globalThis{} = {args}", access_path(path)),
        Call::Create => "({})".to_string(),
        Call::IndexGet => format!("get(self)[{args}]"),
        Call::IndexSet => format!("get(self)[{first}] = {second}"),
    };
    let body = match &f.ret {
        Ty::Void => format!("{{ {call}; }}"),
        t => to_wasm(&call, t),
    };
    (wasm_params, body)
}

// `.name`, or `["some-name"]` where the name is not an identifier.
fn access(name: &str) -> String {
    let plain = !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '$') && !name.as_bytes()[0].is_ascii_digit();
    if plain { format!(".{name}") } else { format!("[{name:?}]") }
}

fn access_path(path: &str) -> String {
    path.split('.').map(access).collect()
}

// JavaScript that implements every import by calling the real API.
// The functions a compiled module imports from `env`: what the glue must provide.
pub fn imports(wasm: &[u8]) -> HashSet<String> {
    fn leb(b: &[u8], i: &mut usize) -> usize {
        let (mut v, mut shift) = (0usize, 0);
        loop {
            let byte = b[*i];
            *i += 1;
            v |= ((byte & 0x7f) as usize) << shift;
            if byte & 0x80 == 0 {
                return v;
            }
            shift += 7;
        }
    }
    fn name(b: &[u8], i: &mut usize) -> String {
        let n = leb(b, i);
        let s = String::from_utf8_lossy(&b[*i..*i + n]).into_owned();
        *i += n;
        s
    }
    let mut out = HashSet::new();
    let mut i = 8;
    while i < wasm.len() {
        let id = wasm[i];
        i += 1;
        let size = leb(wasm, &mut i);
        let end = i + size;
        if id == 2 {
            for _ in 0..leb(wasm, &mut i) {
                let module = name(wasm, &mut i);
                let field = name(wasm, &mut i);
                let kind = wasm[i];
                i += 1;
                match kind {
                    0 => {
                        leb(wasm, &mut i);
                    }
                    1 => {
                        i += 1;
                        let flags = leb(wasm, &mut i);
                        leb(wasm, &mut i);
                        if flags & 1 != 0 {
                            leb(wasm, &mut i);
                        }
                    }
                    2 => {
                        let flags = leb(wasm, &mut i);
                        leb(wasm, &mut i);
                        if flags & 1 != 0 {
                            leb(wasm, &mut i);
                        }
                    }
                    3 => i += 2,
                    _ => {
                        i += 1;
                        leb(wasm, &mut i);
                    }
                }
                if module == "env" {
                    out.insert(field);
                }
            }
        }
        i = end;
    }
    out
}

// The glue for every import in the model, or with `used`, only those a compiled module needs.
pub fn glue(m: &Model, used: Option<&HashSet<String>>) -> String {
    let wanted = |name: &str| used.is_none_or(|u| u.contains(name));
    let mut out = format!("const COUNTED = {};\n", m.counted);
    out += &String::from(
        "// Generated. Implements the wasm imports by calling the JavaScript API.
export function makeEnv(getInstance) {
  const objs = [null];
  // One handle per value, so handles can be compared for identity. Each
  // `put` is a reference wasm owns; a handle is freed when none are left.
  const ids = new Map();
  const refs = [0];
  const free = [];
  const put = v => {
    if (v === null || v === undefined) return 0;
    let h = ids.get(v);
    if (h === undefined) {
      h = free.length ? free.pop() : objs.length;
      objs[h] = v;
      refs[h] = 0;
      ids.set(v, h);
    }
    refs[h]++;
    return h;
  };
  const release = h => {
    if (h && --refs[h] === 0) {
      ids.delete(objs[h]);
      objs[h] = undefined;
      free.push(h);
    }
  };
  const get = h => objs[h];
  // A handle lent to a callback, released when it returns. Where wasm counts
  // handles, it releases them itself: the handle is its own.
  const lend = (lent, v) => {
    const h = put(v);
    if (!COUNTED && h) lent.push(h);
    return h;
  };
  // A handle wasm gave up: what a callback returns.
  const take = h => {
    const v = get(h);
    release(h);
    return v;
  };
  const memory = () => getInstance().exports.memory.buffer;
  const table = () => getInstance().exports.__indirect_function_table;
  const enc = new TextEncoder(), dec = new TextDecoder();
  const str = (ptr, len) => dec.decode(new Uint8Array(memory(), ptr, len));
  // One JavaScript function per wasm function and context, so that a
  // callback passed twice is the same function (removeEventListener).
  const callbacks = new Map();
  // Callbacks made here: their key and context, until released.
  const owned = new Map();
  const callback = (kind, fn, ctx, make) => {
    const key = `${kind}:${fn}:${ctx}`;
    if (!callbacks.has(key)) {
      const f = make(table().get(fn));
      const state = { key, ctx, alive: true };
      const live = (...args) => {
        if (!state.alive) throw new Error('callback called after release');
        return f(...args);
      };
      owned.set(live, state);
      callbacks.set(key, live);
    }
    return put(callbacks.get(key));
  };
  const enums = {
",
    );
    for (name, values) in &m.enums {
        out += &format!("    {name}: {values:?},\n");
    }
    // The keys of each map, by index.
    out += "  };\n  const keys = {\n";
    for k in &m.maps {
        out += &format!("    {}: {:?},\n", k.name, k.keys);
    }
    out += "  };
  // The first exception a call threw, until wasm takes it: a handle, or 0.
  let error = 0;
  const env = {
    js_string_len: h => enc.encode(String(get(h) ?? '')).length,
    js_string_read: (h, ptr) => { new Uint8Array(memory()).set(enc.encode(String(get(h) ?? '')), ptr); },
    js_from_number: x => put(x),
    js_from_bool: x => put(!!x),
    js_from_string: (ptr, len) => put(str(ptr, len)),
    js_to_number: h => Number(get(h)),
    js_to_bool: h => (get(h) ? 1 : 0),
    js_equals: (a, b) => (get(a) === get(b) ? 1 : 0),
    js_array_new: () => put([]),
    js_array_length: h => get(h).length,
    js_array_get: (h, i) => put(get(h)[i]),
    js_array_push: (h, v) => { get(h).push(get(v)); },
    js_listener_from: (fn, ctx) => callback('listener', fn, ctx, f => ev => { const lent = []; try { f(ctx, lend(lent, ev)); } finally { lent.forEach(release); } }),
    // Forgets a callback made here; returns its context, for wasm to free, or 0.
    js_callback_release: h => {
      const o = owned.get(get(h));
      if (!o) return 0;
      owned.delete(get(h));
      callbacks.delete(o.key);
      o.alive = false;
      return o.ctx;
    },
    js_retain: h => { if (h) refs[h]++; },
    js_release: release,
    // Values wasm holds handles to.
    js_live: () => ids.size,
    // Iteration: an iterator over a value, and its next value, or -1 at the end.
    js_iterate: h => put(get(h)[Symbol.iterator]()),
    js_iterator_next: it => { const r = get(it).next(); return r.done ? -1 : put(r.value); },
    // The pending exception, which wasm now holds, or 0.
    js_take_error: () => { const e = error; error = 0; return e; },
    js_error_message: h => { const e = get(h); return put(String(e instanceof Error ? e.message : e)); },
    // Calls fn(ctx, ok, value) once the promise settles: ok 1 with its value, 0 with its reason.
    js_promise_then: (p, fn, ctx) => {
      const settle = ok => v => { const lent = []; try { table().get(fn)(ctx, ok, lend(lent, v)); } finally { lent.forEach(release); } };
      Promise.resolve(get(p)).then(settle(1), settle(0));
    },
";
    // Which member of a union a value is: its index, or -1.
    for u in m.unions.iter().filter(|u| wanted(&format!("{}_tag", u.name))) {
        let mut arms = Vec::new();
        let mut default = None;
        // A bare type parameter: `k` says what its argument is, 1 a number, 2 a string, 3 a flag, 0 anything else.
        let mut param = None;
        for (i, (test, part)) in u.tests.iter().zip(&u.parts).enumerate() {
            match test {
                _ if matches!(part, Ty::Param(_)) => {
                    arms.insert(0, format!("if (typeof v === [, 'number', 'string', 'boolean'][k]) return {i};"));
                    param = Some(i);
                }
                Some(t) => arms.push(format!("if ({}) return {i};", js_test(t, u.discriminant.as_deref()))),
                None => default = Some(i),
            }
        }
        let default = match (default, param) {
            (Some(d), _) => d.to_string(),
            (None, Some(p)) => format!("(k === 0 ? {p} : -1)"),
            (None, None) => "-1".into(),
        };
        out += &format!("    {}_tag: (h, k) => {{ const v = get(h); {} return {default}; }},\n", u.name, arms.join(" "));
    }
    for c in m.callbacks.iter().filter(|c| wanted(&format!("{}_from", c.name))) {
        let names: Vec<String> = (0..c.params.len()).map(|i| format!("a{i}")).collect();
        let mut args = vec!["ctx".to_string()];
        args.extend(c.params.iter().zip(&names).map(|((_, t), a)| to_wasm(a, t).replace("put(", "lend(lent, ")));
        let call = format!("f({})", args.join(", "));
        // What a callback returns, wasm gives up.
        let ret = from_wasm("r", &c.ret).replace("get(r)", "take(r)");
        let body = if c.ret == Ty::Void { format!("{call};") } else { format!("const r = {call}; return {ret};") };
        let body = format!("{{ const lent = []; try {{ {body} }} finally {{ lent.forEach(release); }} }}");
        out += &format!("    {}_from: (fn, ctx) => callback({:?}, fn, ctx, f => ({}) => {body}),\n", c.name, c.name, names.join(", "));
    }
    for f in m.all_funcs().filter(|f| f.shared.is_none() && wanted(&f.import)) {
        let (params, body) = entry(f, &|_, literal| literal.to_string());
        out += &format!("    {}: ({}) => {body},\n", f.import, params.join(", "));
    }
    // A shared import takes the index of the fixed arguments after `self`.
    for s in m.shared.iter().filter(|s| wanted(&s.name)) {
        let mut params = s.params.clone();
        let at = usize::from(params.first().is_some_and(|p| p == "self"));
        params.insert(at, "k".into());
        let mut body = s.body.clone();
        for j in 0..s.values.first().map_or(0, Vec::len) {
            body = body.replace(&format!("$F{j}"), &format!("F[k][{j}]"));
        }
        let table: Vec<String> = s.values.iter().map(|v| format!("[{}]", v.join(", "))).collect();
        out += &format!("    {}: (() => {{ const F = [{}]; return ({}) => {body}; }})(),\n", s.name, table.join(", "), params.join(", "));
    }
    // A call that throws returns zero, and its exception is kept for wasm to take.
    out + "  };
  const calls = ['js_take_error', 'js_error_message', 'js_promise_then', 'js_retain', 'js_release', 'js_live'];
  for (const [name, f] of Object.entries(env)) {
    if (calls.includes(name)) continue;
    env[name] = (...args) => {
      try {
        return f(...args);
      } catch (e) {
        if (error === 0) error = put(e);
        return 0;
      }
    };
  }
  return env;
}
"
}
