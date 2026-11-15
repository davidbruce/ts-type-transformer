// What a wasm target needs from the XML, and the JavaScript glue both wasm
// generators share.
//
// The boundary: numbers are f64, booleans i32, strings a pointer and a length
// going in. Everything else (objects, strings coming back, arrays, callbacks,
// `any`) is an integer handle into a table the glue keeps; 0 is null.
// A callback is a handle made from a wasm function pointer and a context pointer.

use std::collections::{BTreeMap, HashMap, HashSet};

use roxmltree::{Document, Node};

use super::{Problems, attr, elements, flag, named, only_child};

// The element of a typed array, a view's bytes, or a buffer's: each
// generator names it in its own language.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Elem {
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    F32,
    F64,
}

impl Elem {
    pub fn bytes(self) -> usize {
        match self {
            Elem::I8 | Elem::U8 => 1,
            Elem::I16 | Elem::U16 => 2,
            Elem::I32 | Elem::U32 | Elem::F32 => 4,
            Elem::I64 | Elem::U64 | Elem::F64 => 8,
        }
    }
}

pub struct BufferSpec {
    pub name: &'static str,
    pub kind: Option<u32>,
    pub elem: Elem,
}

impl BufferSpec {
    pub fn is_typed_array(&self) -> bool {
        self.kind.is_some_and(|k| (2..=12).contains(&k))
    }
    pub fn has_buffer(&self) -> bool {
        self.kind != Some(0) && self.kind != Some(1) && self.name != "ArrayBufferLike"
    }
}

pub const BUFFER_SPECS: &[BufferSpec] = &[
    BufferSpec { name: "ArrayBuffer", kind: Some(0), elem: Elem::U8 },
    BufferSpec { name: "SharedArrayBuffer", kind: Some(1), elem: Elem::U8 },
    BufferSpec { name: "ArrayBufferLike", kind: None, elem: Elem::U8 },
    BufferSpec { name: "ArrayBufferView", kind: None, elem: Elem::U8 },
    BufferSpec { name: "DataView", kind: Some(13), elem: Elem::U8 },
    BufferSpec { name: "Int8Array", kind: Some(2), elem: Elem::I8 },
    BufferSpec { name: "Uint8Array", kind: Some(3), elem: Elem::U8 },
    BufferSpec { name: "Uint8ClampedArray", kind: Some(4), elem: Elem::U8 },
    BufferSpec { name: "Int16Array", kind: Some(5), elem: Elem::I16 },
    BufferSpec { name: "Uint16Array", kind: Some(6), elem: Elem::U16 },
    BufferSpec { name: "Int32Array", kind: Some(7), elem: Elem::I32 },
    BufferSpec { name: "Uint32Array", kind: Some(8), elem: Elem::U32 },
    BufferSpec { name: "Float32Array", kind: Some(9), elem: Elem::F32 },
    BufferSpec { name: "Float64Array", kind: Some(10), elem: Elem::F64 },
    BufferSpec { name: "BigInt64Array", kind: Some(11), elem: Elem::I64 },
    BufferSpec { name: "BigUint64Array", kind: Some(12), elem: Elem::U64 },
];

pub fn buffer_spec(name: &str) -> Option<&'static BufferSpec> {
    BUFFER_SPECS.iter().find(|s| s.name == name)
}

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
    // An array of a known element type.
    ArrayOf(Box<Ty>),
    // An array of fixed length and element types: `[number, string]`.
    Tuple(Vec<Ty>),
}

#[derive(Clone, PartialEq, Debug)]
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
    // The object called, or constructed with `new`: a callable interface.
    Invoke,
    Construct,
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
    // What iterating it yields, from `[Symbol.iterator]()`, and asynchronously, from `[Symbol.asyncIterator]()`.
    pub iterates: Option<Ty>,
    pub iterates_async: Option<Ty>,
    // What it is an array of, when it extends Array: `class Group extends Array<Pt>`.
    pub elements: Option<Ty>,
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
        // Two members with one test are not told apart by it: two enums of strings, say.
        let tests: Vec<&String> = self.tests.iter().flatten().filter(|t| !t.starts_with("tag:")).collect();
        let unique = tests.iter().enumerate().all(|(i, t)| !tests[..i].contains(t));
        untested <= 1 && self.params() <= 1 && unique
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
    // Each top-level declaration's package, by its name in the output, when it
    // is in one (`pb_Node`: `pb`).
    pub packages: BTreeMap<String, String>,
    // Where each declaration is reached, by its name in the output:
    // `pb::Node` for `pb_Node`, `Konva.Shape` for one in a namespace.
    pub js_refs: BTreeMap<String, String>,
}

impl Class {
    pub fn has_self(f: &Func) -> bool {
        matches!(f.call, Call::Method(_) | Call::Get(_) | Call::Set(_) | Call::IndexGet | Call::IndexSet | Call::Invoke | Call::Construct)
    }
}

// Where a declaration is reached in JavaScript: its path, with the name it
// is exported as (`jsName`) in place of its own.

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
    // What the target cannot express at all.
    problems: std::cell::RefCell<Problems>,
    // The module of the declaration being read.
    module: String,
    // Whether generics are kept (C++), and the type parameters of the signature just read.
    generics: bool,
    last_type_params: Vec<TParam>,
    // Module folders given out, by lowercased path: names that differ only in case get a suffix.
    folders: HashMap<String, String>,
    folder_names: HashMap<(String, String), String>,
    // The package of the top-level declaration being read, when several are bound.
    package: Option<String>,
    // The package of declarations that do not name theirs: one bound alone.
    root_import: Option<String>,
    // The JavaScript path of the namespaces being read: `Konva`, or `util`
    // for the namespace the output calls `pb_util`.
    js_prefix: String,
    // A generic callback's own type parameters, bound as untyped values: a
    // closure cannot be generic.
    erased: HashSet<String>,
    // Type parameters over the keys of a type that is not a map: strings.
    key_strings: HashSet<String>,
}

// The model of the XML for a target, or what the target cannot express.
pub fn model(xml: &str, target: &str) -> Result<Model, Vec<String>> {
    let doc = Document::parse(xml).map_err(|e| vec![format!("not well-formed XML: {e}")])?;
    let mut b = Builder {
        target,
        decls: HashMap::new(),
        model: Model { enums: vec![], num_enums: vec![], classes: vec![], callbacks: vec![], unions: vec![], maps: vec![], shared: vec![], funcs: vec![], opaque: vec![], opaque_arity: BTreeMap::new(), skipped: BTreeMap::new(), counted: target == "C++" || target == "Rust", packages: BTreeMap::new(), js_refs: BTreeMap::new() },
        opaque: HashSet::new(),
        imports: HashSet::new(),
        resolving: Vec::new(),
        keyed: HashMap::new(),
        bad: None,
        problems: Default::default(),
        module: String::new(),
        generics: matches!(target, "C++" | "Rust" | "Gleam"),
        last_type_params: Vec::new(),
        folders: HashMap::new(),
        folder_names: HashMap::new(),
        package: None,
        root_import: doc.root_element().attribute("import").map(str::to_string),
        js_prefix: String::new(),
        erased: HashSet::new(),
        key_strings: HashSet::new(),
    };
    b.index(doc.root_element(), "");
    b.decls_of(doc.root_element(), "");
    b.share_imports();
    let mut opaque: Vec<String> = b.opaque.into_iter().collect();
    opaque.sort();
    b.model.opaque = opaque;
    // A function named as a type (`function WebGLShader(): WebGLShader`) is `WebGLShader_fn`.
    let m = &mut b.model;
    let types: HashSet<String> = m.classes.iter().map(|c| c.name.clone())
        .chain(m.enums.iter().map(|e| e.0.clone()))
        .chain(m.num_enums.iter().map(|e| e.0.clone()))
        .chain(m.unions.iter().map(|u| u.name.clone()))
        .chain(m.callbacks.iter().map(|c| c.name.clone()))
        .chain(m.opaque.iter().cloned())
        .collect();
    for f in m.funcs.iter_mut().filter(|f| types.contains(&f.name)) {
        f.name += "_fn";
    }
    // A constraint that is an opaque type nothing is known to extend
    // (`T extends ArrayBufferLike`) holds for no type: none.
    let opaque: HashSet<String> = m.opaque.iter().cloned().collect();
    let unconstrain = |ps: &mut Vec<TParam>| ps.iter_mut().filter(|p| matches!(&p.constraint, Some(Ty::Obj(n)) if opaque.contains(n))).for_each(|p| p.constraint = None);
    m.classes.iter_mut().for_each(|c| {
        unconstrain(&mut c.type_params);
        c.funcs.iter_mut().for_each(|f| unconstrain(&mut f.type_params));
    });
    m.funcs.iter_mut().for_each(|f| unconstrain(&mut f.type_params));
    m.unions.iter_mut().for_each(|u| unconstrain(&mut u.type_params));
    m.callbacks.iter_mut().for_each(|c| unconstrain(&mut c.type_params));
    b.problems.into_inner().result(b.model)
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

    // Notes an element the target has no form for.
    fn unsupported(&self, n: Node) {
        self.problems.borrow_mut().element(self.target, n);
    }

    // The type of an alias. Its type parameters come first; only where generics
    // are kept can it have any.
    fn alias_body(&self, d: Node<'a, 'a>) -> Node<'a, 'a> {
        if !self.generics {
            if let Some(p) = named(d, "typeParameter").next() {
                self.unsupported(p);
            }
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
            match d.tag_name().name() {
                "lowering" => {}
                "namespace" => {
                    let inner = join(&self.js_prefix, d.attribute("jsName").unwrap_or(name));
                    let outer = std::mem::replace(&mut self.js_prefix, inner);
                    self.decls_of(d, &path);
                    self.js_prefix = outer;
                }
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
                    let (params, _, _, ret) = self.callback_signature(only_child(d), &owner);
                    self.model.callbacks.push(Callback { name: owner, params, ret, type_params: std::mem::take(&mut self.last_type_params) });
                }
                "interface" | "class" => {
                    let c = self.class_at(d, &path, &self.js(d, &path));
                    self.model.classes.push(c);
                }
                "typeAlias" => {
                    let body = self.alias_body(d);
                    let alias_params: Vec<TParam> = named(d, "typeParameter").collect::<Vec<_>>().into_iter().map(|p| self.tparam(p)).collect();
                    if body.has_tag_name("functionType") {
                        let owner = ident(&path);
                        let (params, _, _, ret) = self.callback_signature(body, &owner);
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
                    let js = self.js(d, &path);
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
                        let c = self.statics(body, &self.js(d, &path), name);
                        self.model.classes.push(c);
                        continue;
                    }
                    let ret = self.ty(body, "");
                    let name = format!("get_{}", ident(&path));
                    let import = self.import(name.clone());
                    let js = self.js(d, &path);
                    self.model.funcs.push(Func { type_params: vec![], module: self.module.clone(), import, name, call: Call::StaticGet(js.clone()), params: vec![], rest: false, fixed: vec![], shared: None, ret: ret.clone() });
                    // A `const`, or a module's export, cannot be assigned.
                    if flag(d, "readonly") {
                        continue;
                    }
                    let path = js;
                    let set_name = format!("set_{}", ident(&path));
                    let set_import = self.import(set_name.clone());
                    self.model.funcs.push(Func { type_params: vec![], module: self.module.clone(), import: set_import, name: set_name, call: Call::StaticSet(path), params: vec![("value".into(), value(ret))], rest: false, fixed: vec![], shared: None, ret: Ty::Void });
                }
                _ => self.unsupported(d),
            }
        }
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

    fn class(&mut self, d: Node<'a, 'a>, path: &str) -> Class {
        self.class_at(d, path, path)
    }

    // A class named for `path`, its constructor and statics reached at `js`.
    fn class_at(&mut self, d: Node<'a, 'a>, path: &str, js: &str) -> Class {
        let path_js = js;
        let owner = ident(path);
        let mut c = Class { type_params: vec![], bases: vec![], module: self.module.clone(), name: owner.clone(), ancestors: Vec::new(), funcs: Vec::new(), consts: Vec::new(), iterates: None, iterates_async: None, elements: None };
        let mut names = HashSet::new();
        if !d.has_tag_name("class") && !named(d, "call").any(|_| true) {
            names.insert("create".to_string());
            let import = self.import(format!("{owner}_create"));
            c.funcs.push(Func { type_params: vec![], module: String::new(), import, name: "create".into(), call: Call::Create, params: vec![], rest: false, fixed: vec![], shared: None, ret: Ty::Obj(owner.clone()) });
        }
        for m in elements(d) {
            self.member(&mut c, &mut names, m, path_js, m.has_tag_name("constructor") || flag(m, "static"));
        }
        c
    }

    fn statics(&mut self, body: Node<'a, 'a>, path: &str, name: String) -> Class {
        let mut c = Class { type_params: vec![], bases: vec![], module: self.module.clone(), name, ancestors: Vec::new(), funcs: Vec::new(), consts: Vec::new(), iterates: None, iterates_async: None, elements: None };
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
            if base.has_tag_name("ref") {
                let name = base.attribute("qualified").unwrap_or(attr(base, "name"));
                // A subclass of Array is an array of its elements.
                if matches!(name, "Array" | "ReadonlyArray") && !self.decls.contains_key(name) {
                    if let Some(e) = elements(base).next() {
                        let elem = self.ty(e, &owner);
                        if let Ty::ArrayOf(elem) = self.array_of(elem) {
                            c.elements = Some(*elem.clone());
                            // Where generics are kept, the array is its base.
                            if self.generics {
                                c.bases.push(Ty::ArrayOf(elem));
                            }
                        }
                    }
                    return;
                }
                // Where generics are erased, a generic base is the base without its arguments.
                let t = if elements(base).next().is_some() && !self.generics { Ty::Obj(ident(name)) } else { self.reference(base) };
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
                return self.unsupported(m);
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
        // An object that is also a function: `invoke(...)`, `construct(...)`.
        if tag == "call" || tag == "construct" && !is_static {
            let (params, rest, fixed, ret) = self.signature(m, &owner);
            let (base, call) = if tag == "call" { ("invoke", Call::Invoke) } else { ("construct", Call::Construct) };
            let name = if self.target == "C++" { base.to_string() } else {
                let mut name = base.to_string();
                let mut n = 2;
                while !names.insert(name.clone()) {
                    name = format!("{base}_{n}");
                    n += 1;
                }
                name
            };
            let import = self.import(format!("{owner}_{name}"));
            let type_params = std::mem::take(&mut self.last_type_params);
            c.funcs.push(Func { type_params, module: String::new(), import, name, call, params, rest, fixed, shared: None, ret });
            if let Some(why) = self.bad.take() {
                c.funcs.pop();
                self.skip(&why);
            }
            return;
        }
        if matches!(tag, "index" | "get" | "set") {
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
        if raw_name == "[Symbol.asyncIterator]" && !is_static {
            let elem = named(m, "returns").next().and_then(|r| elements(r).next()).and_then(|it| elements(it).next());
            c.iterates_async = Some(match elem {
                Some(e) => self.ty(e, &owner),
                None => Ty::Any,
            });
            self.bad = None;
            return;
        }
        // A member keyed by a well-known symbol is named by it: `[Symbol.toStringTag]` is `toStringTag`.
        let symbol = raw_name.strip_prefix("[Symbol.").and_then(|s| s.strip_suffix(']'));
        if raw_name.starts_with('[') && symbol.is_none() {
            return self.skip("member with a computed name");
        }
        let (raw_name, key) = (symbol.unwrap_or(raw_name), raw_name);
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
        let js = m.attribute("jsName").unwrap_or(key).to_string();
        match tag {
            "constructor" | "construct" => {
                let (params, rest, fixed, sig_ret) = self.signature(m, &owner);
                let ret = if tag == "construct" && sig_ret != Ty::Void { sig_ret } else { Ty::Obj(owner.clone()) };
                let name = unique(if raw_name.is_empty() { "new".into() } else { ident(raw_name) }, names);
                let import = self.import(format!("{owner}_{name}"));
                c.funcs.push(Func { type_params: vec![], module: String::new(), import, name, call: Call::New(path.to_string()), params, rest, fixed, shared: None, ret });
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
            _ => self.unsupported(m),
        }
        if let Some(why) = self.bad.take() {
            c.funcs.truncate(before);
            self.skip(&why);
        }
    }

    // A callback's signature: its own type parameters (`<T>(x: T) => T`) are untyped values.
    fn callback_signature(&mut self, n: Node<'a, 'a>, owner: &str) -> (Vec<(String, Ty)>, bool, Vec<(usize, String)>, Ty) {
        self.erased = named(n, "typeParameter").map(|p| attr(p, "name").to_string()).collect();
        let sig = self.signature(n, owner);
        self.erased.clear();
        sig
    }

    fn signature(&mut self, n: Node<'a, 'a>, owner: &str) -> (Vec<(String, Ty)>, bool, Vec<(usize, String)>, Ty) {
        // Type parameters left after `generics = "erase"` range over a map's keys.
        self.keyed.clear();
        self.key_strings.clear();
        self.last_type_params.clear();
        let own: Vec<Node> = named(n, "typeParameter").filter(|p| !self.erased.contains(attr(*p, "name"))).collect();
        for p in own {
            let map = elements(p).find(|c| c.has_tag_name("constraint")).map(only_child).filter(|k| k.has_tag_name("keyof")).map(only_child);
            match map {
                Some(m) => {
                    let name = m.attribute("qualified").unwrap_or(attr(m, "name")).to_string();
                    if self.key_map(&name) {
                        self.keyed.insert(attr(p, "name").to_string(), ident(&name));
                    } else {
                        // A key of a type that is not a map of keys: a string.
                        self.key_strings.insert(attr(p, "name").to_string());
                    }
                }
                None if self.generics => {
                    let p = self.tparam(p);
                    self.last_type_params.push(p);
                }
                None => self.unsupported(p),
            }
        }
        let mut params = Vec::new();
        let mut rest = false;
        let mut fixed = Vec::new();
        let mut ret = Ty::Void;
        for p in elements(n) {
            match p.tag_name().name() {
                // The receiver of a callback is not an argument.
                "thisParameter" => {}
                // An argument with one possible value is supplied by the glue.
                "parameter" if p.has_attribute("const") => {
                    let value = attr(p, "const");
                    let literal = if only_child(p).attribute("name") == Some("string") { format!("{value:?}") } else { value.to_string() };
                    fixed.push((params.len() + fixed.len(), literal));
                }
                // An optional parameter is an optional value: null, or a null handle, when absent.
                "parameter" if flag(p, "optional") => {
                    let t = value(self.ty(only_child(p), owner));
                    let optional = absent(t.clone(), true);
                    if optional == Ty::Any && t != Ty::Any {
                        self.skip("optional parameter of a type that cannot be absent, bound as an untyped value");
                    }
                    params.push((ident(attr(p, "name")), optional));
                }
                "parameter" => {
                    rest |= flag(p, "rest");
                    params.push((ident(attr(p, "name")), value(self.ty(only_child(p), owner))));
                }
                "returns" => ret = self.ty(only_child(p), owner),
                "typeParameter" => {}
                _ => self.unsupported(p),
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
            "ref" if attr(n, "kind") == "typeParameter" && self.key_strings.contains(attr(n, "name")) => Ty::Str,
            "ref" if attr(n, "kind") == "typeParameter" && self.erased.contains(attr(n, "name")) => {
                self.skip("type parameter of a generic callback, bound as an untyped value");
                Ty::Any
            }
            "ref" if attr(n, "kind") == "typeParameter" => match self.keyed.get(attr(n, "name")) {
                Some(map) => Ty::Key(map.clone()),
                None if self.generics => Ty::Param(attr(n, "name").to_string()),
                None => {
                    self.unsupported(n);
                    Ty::Any
                }
            },
            "ref" => self.reference(n),
            // `M[K]`: the value the key selects.
            // `M[K]` where K is a key of a type that is not a map of keys: whatever M holds.
            "indexedAccess" if elements(n).nth(1).is_some_and(|k| k.has_tag_name("ref") && self.key_strings.contains(attr(k, "name"))) => {
                self.skip("value of a key of a type that is not a map, bound as an untyped value");
                Ty::Any
            }
            "indexedAccess" => {
                let index = elements(n).nth(1).unwrap();
                match index.attribute("name").and_then(|k| self.keyed.get(k)) {
                    Some(map) => Ty::Mapped(map.clone()),
                    None => {
                        self.unsupported(n);
                        Ty::Any
                    }
                }
            }
            // `new () => Material`: a class itself, which only JavaScript has to give.
            "constructorType" => {
                self.skip("constructor type, bound as an untyped value");
                Ty::Any
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
            "array" => {
                let elem = self.ty(only_child(n), owner);
                self.array_of(elem)
            }
            // A tuple whose elements all cross, none optional or rest.
            "tuple" => {
                let elems: Vec<Node> = elements(n).collect();
                if elems.is_empty() || elems.iter().any(|e| e.attributes().len() > 0) {
                    return Ty::Array;
                }
                let ts: Vec<Ty> = elems.iter().map(|e| self.ty(only_child(*e), owner)).collect();
                let crosses = |t: &Ty| match t {
                    Ty::Num | Ty::Bool | Ty::Str | Ty::Obj(_) | Ty::ArrayOf(_) | Ty::Array | Ty::Tuple(_) => true,
                    Ty::Generic(..) | Ty::Param(_) => self.generics,
                    Ty::NamedUnion(u) => self.model.unions.iter().any(|x| x.name == *u && x.decodable()),
                    _ => false,
                };
                if ts.iter().all(crosses) { Ty::Tuple(ts) } else { Ty::Array }
            }
            "union" => {
                let parts: Vec<Ty> = elements(n).map(|p| self.ty(p, owner)).collect();
                // Only alternatives a tagged union can hold directly.
                if parts.iter().all(|p| matches!(p, Ty::Num | Ty::Str | Ty::Bool | Ty::Obj(_) | Ty::Enum(_))) && distinct(&parts) {
                    Ty::Union(parts)
                } else {
                    self.skip("union whose members cannot be told apart, bound as an untyped value");
                    Ty::Union(vec![])
                }
            }
            _ => {
                self.unsupported(n);
                Ty::Any
            }
        }
    }

    // An array of the given elements: typed, if they cross as a number, a
    // flag, a string, a handle or a union that can be told apart.
    fn array_of(&mut self, elem: Ty) -> Ty {
        match elem {
            e @ (Ty::Num | Ty::Bool | Ty::Str | Ty::Obj(_) | Ty::ArrayOf(_)) => Ty::ArrayOf(Box::new(e)),
            e @ (Ty::Generic(..) | Ty::Param(_)) if self.generics => Ty::ArrayOf(Box::new(e)),
            Ty::NamedUnion(u) if self.model.unions.iter().any(|x| x.name == u && x.decodable()) => Ty::ArrayOf(Box::new(Ty::NamedUnion(u))),
            _ => Ty::Array,
        }
    }

    fn reference(&mut self, n: Node<'a, 'a>) -> Ty {
        let key = n.attribute("qualified").unwrap_or(attr(n, "name")).to_string();
        let declared = self.decls.get(&key).copied();
        // `Array<T>` is `T[]`.
        if declared.is_none() && matches!(key.as_str(), "Array" | "ReadonlyArray") {
            return match elements(n).next() {
                Some(e) => {
                    let elem = self.ty(e, "");
                    self.array_of(elem)
                }
                None => Ty::Array,
            };
        }
        match (attr(n, "kind"), declared) {
            ("typeParameter", _) => {
                self.unsupported(n);
                Ty::Any
            }
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
                // Another generic alias would need its arguments substituted
                // (`aliases = "expand"` does, but for one that refers to itself).
                if named(d, "typeParameter").next().is_some() && !generic_union {
                    self.skip("generic alias, bound as any");
                    return Ty::Any;
                }
                if self.resolving.contains(&key) || self.resolving.len() > 8 {
                    return Ty::Any;
                }
                // A union with a name of its own is declared once.
                if body.has_tag_name("union") {
                    let name = ident(&key);
                    let args: Vec<Ty> = if generic_union { elements(n).map(|a| self.ty(a, "")).collect() } else { Vec::new() };
                    // `void` in a union is a value that may be absent: `void | Promise<void>`.
                    // So is a member that may itself be absent: `string | (number | null)`.
                    let is_void = |p: Node| p.has_tag_name("primitive") && attr(p, "name") == "void";
                    let has_void = elements(body).any(|p| is_void(p) || p.has_tag_name("nullable"));
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
                    let parts: Vec<Ty> = members
                        .iter()
                        .map(|p| match self.ty(*p, "") {
                            Ty::Nullable(inner, _) => *inner,
                            t => t,
                        })
                        .collect();
                    self.resolving.pop();
                    // Members that are one type here, as two generic types are once erased, are one member.
                    let mut kept = Vec::new();
                    let (members, parts): (Vec<Node>, Vec<Ty>) = members.into_iter().zip(parts).filter(|(_, p)| if kept.contains(p) { false } else { kept.push(p.clone()); true }).unzip();
                    // `X | any` is any.
                    if parts.contains(&Ty::Any) {
                        self.skip("union with an any member, which is any");
                        return Ty::Any;
                    }
                    if let [only] = parts.as_slice() {
                        return if has_void { absent(only.clone(), true) } else { only.clone() };
                    }
                    let simple = |p: &Ty| matches!(p, Ty::Num | Ty::Str | Ty::Bool | Ty::Obj(_) | Ty::Enum(_) | Ty::Array | Ty::ArrayOf(_) | Ty::Tuple(_)) || self.generics && matches!(p, Ty::Generic(..) | Ty::Param(_));
                    if !parts.iter().all(simple) || !distinct(&parts) {
                        if std::env::var_os("TS_UNION_DEBUG").is_some() {
                            eprintln!("UNION {name}: distinct={} {:?}", distinct(&parts), parts);
                        }
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

// No two members alike. Arrays are alike whatever they hold: a test sees only an array.
fn distinct(parts: &[Ty]) -> bool {
    let shape = |p: &Ty| if matches!(p, Ty::ArrayOf(_) | Ty::Tuple(_)) { Ty::Array } else { p.clone() };
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
        Ty::Generic(_, args) | Ty::Tuple(args) => args.iter_mut().for_each(|a| rename_param(a, from, to)),
        _ => {}
    }
}

// What may be absent. A handle can be null; a string is a null pointer going
// in and a null handle coming back; a number is a flag and the number going
// in, and a handle coming back; a flag or an enum is -1 when absent.
fn absent(t: Ty, undefined: bool) -> Ty {
    match t {
        Ty::Obj(_) | Ty::Array | Ty::ArrayOf(_) | Ty::Tuple(_) | Ty::Str | Ty::Param(_) | Ty::Generic(..) => Ty::Nullable(Box::new(t), undefined),
        Ty::Num | Ty::NumEnum(_) | Ty::Bool | Ty::Enum(_) | Ty::Mapped(_) | Ty::NamedUnion(_) | Ty::Listener(_) => Ty::Nullable(Box::new(t), undefined),
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
        Ty::Enum(e) => format!("(places.{e}.get({x}) ?? -1)"),
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Bool) => format!("(v => (v == null ? -1 : v ? 1 : 0))({x})"),
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Enum(_)) => {
            let Ty::Enum(e) = &**inner else { unreachable!() };
            format!("(places.{e}.get({x}) ?? -1)")
        }
        _ => format!("put({x})"),
    }
}

// A union member's runtime test as a JavaScript condition on `v`.
// `js_refs` says where a class the output declares is reached.
pub(crate) fn js_test(test: &str, discriminant: Option<&str>, js_refs: &BTreeMap<String, String>) -> String {
    let (kind, arg) = test.split_once(':').unwrap_or((test, ""));
    match kind {
        "typeof" => format!("typeof v === {arg:?}"),
        "array" => "Array.isArray(v)".into(),
        "instanceof" => format!("v instanceof {}", global(js_refs.get(arg).map_or(arg, String::as_str))),
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
        // A rest parameter's array is spread; a null one is empty.
        if f.rest && i == f.params.len() - 1 {
            args.push(format!("...({} ?? [])", from_wasm(&p, t)));
        } else {
            args.push(from_wasm(&p, t));
        }
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
        Call::New(path) => format!("new ({})({args})", global(path)),
        Call::Static(path) => format!("{}({args})", global(path)),
        Call::StaticGet(path) => global(path),
        // A variable of the library's own module cannot be assigned from outside it.
        Call::StaticSet(path) if !path.contains('.') => format!("globalThis{} = {args}", access_path(path)),
        Call::StaticSet(path) => format!("{} = {args}", global(path)),
        Call::Create => "({})".to_string(),
        Call::Invoke => format!("get(self)({args})"),
        Call::Construct => format!("new (get(self))({args})"),
        Call::IndexGet => format!("get(self)[{args}]"),
        Call::IndexSet => format!("get(self)[{first}] = {second}"),
    };
    let body = match &f.ret {
        Ty::Void => format!("{{ {call}; }}"),
        t => to_wasm(&call, t),
    };
    (wasm_params, body)
}

// A name the API declares at its top level. In a package, only in its module:
// `prosemirror-model::Node` is `PK["prosemirror-model"].Node`, and not a
// global `Node` should the module lack it. Else in `options.root` if there,
// else a global: `Pt.make` is `(G.Pt ?? globalThis.Pt).make`.
pub(crate) fn global(path: &str) -> String {
    if let Some((package, path)) = path.split_once("::") {
        return format!("PK[{package:?}]{}", access_path(path));
    }
    let (first, rest) = path.split_once('.').map_or((path, String::new()), |(f, r)| (f, access_path(r)));
    format!("(G{0} ?? globalThis{0}){rest}", access(first))
}

// `.name`, or `["some-name"]` where the name is not an identifier.
pub(crate) fn access(name: &str) -> String {
    // A symbol's key is written as it is: `[Symbol.toStringTag]`.
    if name.starts_with("[Symbol.") {
        return name.to_string();
    }
    let plain = !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '$') && !name.as_bytes()[0].is_ascii_digit();
    if plain { format!(".{name}") } else { format!("[{name:?}]") }
}

pub(crate) fn access_path(path: &str) -> String {
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
    // A value handed to a callback: wasm's own where it counts handles, else
    // lent, and released when the callback returns.
    let lend = |args: &[String]| -> (String, String) {
        if m.counted {
            return (args.iter().map(|a| a.replace("put(", "ref(")).collect::<Vec<_>>().join(", "), String::new());
        }
        let mut pre = String::new();
        let mut post = String::new();
        let mut out = Vec::new();
        for (i, a) in args.iter().enumerate() {
            match a.strip_prefix("put(") {
                Some(_) => {
                    pre += &format!("const h{i} = {}; ", a.replace("put(", "ref("));
                    post += &format!("release(h{i}); ");
                    out.push(format!("h{i}"));
                }
                None => out.push(a.clone()),
            }
        }
        (format!("{pre}\u{1}{}", out.join(", ")), post)
    };
    // `call` with its lent arguments released afterwards.
    let lending = |call: &dyn Fn(&str) -> String, args: &[String]| -> String {
        let (args, post) = lend(args);
        match args.split_once('\u{1}') {
            Some((pre, args)) if !post.is_empty() => format!("{pre}try {{ {} }} finally {{ {post}}}", call(args)),
            Some((_, args)) => call(args),
            None => call(&args),
        }
    };
    let mut out = String::from(
        "// Generated. Implements the wasm imports by calling the JavaScript API.
//
// makeEnv(getInstance, options): `options.root` is where the API's names
// are, when not all are globals: a library's module, say; `options.modules`,
// with several packages, each one's module by its name. A package bound
// must be given. `options.check`
// reports mistakes with handles (one used after its release, or released
// twice), at some cost in speed; `options.unchecked(e)` is called with an
// exception no one took, and by default writes it to the console.
export function makeEnv(getInstance, options = {}) {
  const G = options.root ?? globalThis;
  // With several packages bound together, each one's module by its name.
  const M = options.modules ?? {};
  const CHECK = !!options.check;
  const RELEASED = Symbol('released');
  // Values by handle. Each `ref` is a reference wasm owns, in a slot of its
  // own; the slot is freed when its count reaches zero. Zero is null.
  const objs = [null];
  const refs = [0];
  const free = [];
  let live = 0;
  const ref = v => {
    if (v === null || v === undefined) return 0;
    // When checking, a slot is never reused: a stale handle stays stale.
    const h = free.length ? free.pop() : objs.length;
    objs[h] = v;
    refs[h] = 1;
    live++;
    return h;
  };
  // Open scopes: each holds the handles given to wasm while it is innermost,
  // released when it closes.
  const scopes = [];
  const put = v => {
    const h = ref(v);
    if (h !== 0 && scopes.length !== 0) scopes[scopes.length - 1].push(h);
    return h;
  };
  const release = h => {
    if (h === 0) return;
    if (!(refs[h] > 0)) return fail(new Error(`handle ${h} released twice`));
    if (--refs[h] === 0) {
      live--;
      if (CHECK) objs[h] = RELEASED;
      else {
        objs[h] = undefined;
        free.push(h);
      }
    }
  };
  const get = CHECK
    ? h => {
        const v = objs[h];
        if (v === RELEASED) throw new Error(`handle ${h} used after its release`);
        return v;
      }
    : h => objs[h];
  // A handle wasm gave up: what a callback returns.
  const take = h => {
    const v = get(h);
    release(h);
    return v;
  };
  // The exception a call threw, until wasm takes it: a handle, or 0. One that
  // is not taken by the time control returns to JavaScript, or before the
  // next one, is reported.
  let error = 0;
  const unchecked = options.unchecked ?? (e => console.error('Uncaught in a wasm call:', e));
  const fail = e => {
    if (error !== 0) {
      const old = objs[error];
      release(error);
      unchecked(old);
    }
    const mine = (error = ref(e));
    queueMicrotask(() => {
      if (error === mine && objs[mine] === e) {
        error = 0;
        release(mine);
        unchecked(e);
      }
    });
    return 0;
  };
  // Runtime calls that can throw, with up to four arguments.
  const guard = f => (a, b, c, d) => {
    try {
      return f(a, b, c, d);
    } catch (e) {
      return fail(e);
    }
  };
  // The module's memory, as bytes: a new view when it has grown.
  let bytes = new Uint8Array(0);
  const mem = () => (bytes.byteLength !== 0 ? bytes : (bytes = new Uint8Array(getInstance().exports.memory.buffer)));
  const table = () => getInstance().exports.__indirect_function_table;
  const enc = new TextEncoder(), dec = new TextDecoder();
  // A string from memory. Short ones are usually ASCII, read faster by hand.
  const decode = (m, ptr, len) => {
    if (len <= 16) {
      let s = '';
      for (let i = 0; i < len; i++) {
        const c = m[ptr + i];
        if (c > 127) return dec.decode(m.subarray(ptr, ptr + len));
        s += String.fromCharCode(c);
      }
      return s;
    }
    return dec.decode(m.subarray(ptr, ptr + len));
  };
  // Strings seen at an address, reused while the bytes there are the same:
  // most strings a program passes are its literals, passed again and again.
  const seen = new Map();
  const str = (ptr, len) => {
    const m = mem();
    const key = ptr * 65536 + len;
    const e = seen.get(key);
    if (e !== undefined) {
      const b = e.bytes;
      let i = 0;
      while (i < len && m[ptr + i] === b[i]) i++;
      if (i === len) return e.s;
      // A buffer that keeps changing is not worth remembering.
      if (e.misses++ > 2) return decode(m, ptr, len);
    }
    const s = decode(m, ptr, len);
    if (len > 256) return s;
    if (seen.size > 1024) seen.clear();
    seen.set(key, { s, bytes: m.slice(ptr, ptr + len), misses: e === undefined ? 0 : e.misses });
    return s;
  };
  // The length of a string in UTF-8.
  const utf8Length = s => {
    const n = s.length;
    for (let i = 0; i < n; i++) if (s.charCodeAt(i) > 127) return enc.encode(s).length;
    return n;
  };
  // Writes at most `cap` bytes of a string to memory; returns its whole length in UTF-8.
  const write = (s, ptr, cap) => {
    const m = mem();
    const n = s.length;
    const fits = n <= cap ? n : cap;
    let i = 0;
    for (; i < fits; i++) {
      const c = s.charCodeAt(i);
      if (c > 127) break;
      m[ptr + i] = c;
    }
    if (i === n) return n;
    if (i === fits && n > cap) return utf8Length(s);
    const r = enc.encodeInto(s, m.subarray(ptr, ptr + cap));
    return r.read === n ? r.written : utf8Length(s);
  };
  const text = h => String(get(h) ?? '');
  const viewOf = (h, off, len) => {
    const v = get(h);
    if (v == null) return new Uint8Array(0);
    const buf = ArrayBuffer.isView(v) ? v.buffer : v;
    const base = ArrayBuffer.isView(v) ? v.byteOffset : 0;
    const total = v.byteLength >>> 0;
    const start = Math.min(off >>> 0, total);
    const count = len === undefined ? total - start : Math.min(len >>> 0, total - start);
    return new Uint8Array(buf, base + start, count);
  };
  const bufferCtors = [
    null,
    typeof SharedArrayBuffer !== 'undefined' ? SharedArrayBuffer : null,
    Int8Array,
    Uint8Array,
    Uint8ClampedArray,
    Int16Array,
    Uint16Array,
    Int32Array,
    Uint32Array,
    Float32Array,
    Float64Array,
    typeof BigInt64Array !== 'undefined' ? BigInt64Array : null,
    typeof BigUint64Array !== 'undefined' ? BigUint64Array : null,
    DataView,
  ];
  // One JavaScript function per wasm function and context, so that a
  // callback passed twice is the same function (removeEventListener).
  const callbacks = new Map();
  // Callbacks made here: their key and context, until released.
  const owned = new Map();
  const dead = () => {
    throw new Error('callback called after release');
  };
  const callback = (kind, fn, ctx, make) => {
    const key = `${kind}:${fn}:${ctx}`;
    let f = callbacks.get(key);
    if (f === undefined) {
      const state = { key, ctx, alive: true };
      f = make(table().get(fn), state);
      owned.set(f, state);
      callbacks.set(key, f);
    }
    return put(f);
  };
  const enums = {
",
    );
    // Each package's module: given, or makeEnv fails here, not at a call.
    let mut packages: Vec<&String> = m.packages.values().collect();
    packages.sort();
    packages.dedup();
    let alone = packages.len() == 1;
    let mut pk = String::from("  const PK = {};\n");
    for p in packages {
        let given = if alone { format!("M[{p:?}] ?? options.root") } else { format!("M[{p:?}]") };
        pk += &format!("  PK[{p:?}] = {given};\n  if (PK[{p:?}] === undefined) throw new Error({:?});\n", format!("makeEnv: no module for the package {p:?}: give it in options.modules{}", if alone { " or options.root" } else { "" }));
    }
    out = out.replacen("  const CHECK = !!options.check;\n", &format!("{pk}  const CHECK = !!options.check;\n"), 1);
    for (name, values) in &m.enums {
        out += &format!("    {name}: {values:?},\n");
    }
    // The keys of each map, by index.
    out += "  };\n  const keys = {\n";
    for k in &m.maps {
        out += &format!("    {}: {:?},\n", k.name, k.keys);
    }
    let listener = lending(&|args| format!("f(ctx, {args});"), &["put(ev)".to_string()]);
    let settle = lending(&|args| format!("table().get(fn)(ctx, ok, {args});"), &["put(v)".to_string()]);
    out += &format!(
        "  }};
  const enumList = Object.values(enums);
  // Each enum's values by name, to their place: looked up, not searched for.
  const places = {{}};
  for (const name in enums) places[name] = new Map(enums[name].map((v, i) => [v, i]));
  const placeList = Object.values(places);
  const env = {{
    js_string_len: guard(h => utf8Length(text(h))),
    js_string_read: guard((h, ptr) => {{ write(text(h), ptr, 0x7fffffff); }}),
    // At most `cap` bytes of the string; returns its whole length.
    js_string_copy: guard((h, ptr, cap) => write(text(h), ptr, cap)),
    // As js_string_copy, and if it fit, the handle is released.
    js_string_take: guard((h, ptr, cap) => {{ const n = write(text(h), ptr, cap); if (n <= cap) release(h); return n; }}),
    js_string_equals: guard((h, ptr, len) => (text(h) === str(ptr, len) ? 1 : 0)),
    js_from_number: x => put(x),
    js_from_bool: x => put(!!x),
    js_from_string: (ptr, len) => put(str(ptr, len)),
    js_to_number: guard(h => Number(get(h))),
    js_to_bool: guard(h => (get(h) ? 1 : 0)),
    // Whether two handles are one JavaScript value.
    js_equals: guard((a, b) => (a === b || get(a) === get(b) ? 1 : 0)),
    js_array_new: () => put([]),
    js_array_length: guard(h => get(h).length),
    js_array_get: guard((h, i) => put(get(h)[i])),
    js_array_push: guard((h, v) => {{ get(h).push(get(v)); }}),
    js_listener_from: (fn, ctx) => callback('listener', fn, ctx, (f, s) => ev => {{ if (!s.alive) dead(); {listener} }}),
    // Forgets a callback made here; returns its context, for wasm to free, or 0.
    js_callback_release: guard(h => {{
      const o = owned.get(get(h));
      if (!o) return 0;
      owned.delete(get(h));
      callbacks.delete(o.key);
      o.alive = false;
      return o.ctx;
    }}),
    js_retain: guard(h => {{ if (h !== 0 && get(h) !== undefined) refs[h]++; }}),
    js_release: release,
    js_scope_open: () => {{ scopes.push([]); }},
    js_scope_close: () => {{ (scopes.pop() ?? []).forEach(release); }},
    // References wasm holds.
    js_live: () => live,
    // An enum's value as a handle, and back as its index: enums by the order they are declared in.
    js_enum_value: (id, i) => put(enumList[id][i]),
    js_enum_index: guard((id, h) => placeList[id].get(get(h)) ?? -1),
    // Iteration: an iterator over a value, and its next value, or -1 at the end.
    js_iterate: guard(h => put(get(h)[Symbol.iterator]())),
    js_iterator_next: guard(it => {{ const r = get(it).next(); return r.done ? -1 : put(r.value); }}),
    // Asynchronous iteration: a promise of the next value, or of nothing at the end.
    js_async_iterate: guard(h => put(get(h)[Symbol.asyncIterator]())),
    js_async_iterator_next: guard(it => put(Promise.resolve(get(it).next()).then(r => (r.done ? undefined : r.value)))),
    // Whether an exception waits to be taken; the exception, which wasm now holds, or 0.
    js_has_error: () => (error !== 0 ? 1 : 0),
    js_take_error: () => {{ const e = error; error = 0; if (e !== 0 && scopes.length !== 0) scopes[scopes.length - 1].push(e); return e; }},
    js_error_message: guard(h => {{ const e = get(h); return put(String(e instanceof Error ? e.message : (e && typeof e === 'object' && 'message' in e ? e.message : e))); }}),
    js_error_name: guard(h => {{ const e = get(h); return put(String(e instanceof Error ? e.name : (e && typeof e === 'object' && 'name' in e ? e.name : typeof e))); }}),
    js_error_stack: guard(h => {{ const e = get(h); return put(String(e && typeof e === 'object' && e.stack != null ? e.stack : '')); }}),
    js_error_new: guard((ptr, len) => put(new Error(str(ptr, len)))),
    js_date_new: ms => put(new Date(ms)),
    js_date_now: () => Date.now(),
    js_date_time: guard(h => {{ const d = get(h); return d instanceof Date ? d.getTime() : Number(d); }}),
    js_date_to_iso_string: guard(h => put(get(h).toISOString())),
    js_bytes_len: guard(h => {{ const v = get(h); return v == null ? 0 : (v.byteLength >>> 0); }}),
    js_bytes_read: guard((h, srcOff, dstPtr, len) => {{ const s = viewOf(h, srcOff, len); if (s.byteLength) mem().set(s, dstPtr >>> 0); return s.byteLength; }}),
    js_bytes_write: guard((h, dstOff, srcPtr, len) => {{ const d = viewOf(h, dstOff, len); const p = srcPtr >>> 0; if (d.byteLength) d.set(mem().subarray(p, p + d.byteLength)); return d.byteLength; }}),
    js_bytes_new: guard((kind, srcPtr, byteLen) => {{
      const n = byteLen >>> 0;
      const p = srcPtr >>> 0;
      if (kind === 0) {{
        const ab = new ArrayBuffer(n);
        if (p !== 0 && n > 0) new Uint8Array(ab).set(mem().subarray(p, p + n));
        return put(ab);
      }}
      if (kind === 1) {{
        const Ctor = bufferCtors[1];
        if (!Ctor) throw new TypeError('SharedArrayBuffer is not available');
        const sab = new Ctor(n);
        if (p !== 0 && n > 0) new Uint8Array(sab).set(mem().subarray(p, p + n));
        return put(sab);
      }}
      const Ctor = bufferCtors[kind];
      if (!Ctor) throw new TypeError(`unsupported buffer kind ${{kind}}`);
      if (kind === 13) {{
        const ab = new ArrayBuffer(n);
        if (p !== 0 && n > 0) new Uint8Array(ab).set(mem().subarray(p, p + n));
        return put(new DataView(ab));
      }}
      const count = (n / Ctor.BYTES_PER_ELEMENT) >>> 0;
      const ta = new Ctor(count);
      if (p !== 0 && count > 0) new Uint8Array(ta.buffer, ta.byteOffset, ta.byteLength).set(mem().subarray(p, p + ta.byteLength));
      return put(ta);
    }}),
    js_bytes_buffer: guard(h => {{ const v = get(h); return put(ArrayBuffer.isView(v) ? v.buffer : v); }}),
    js_bytes_offset: guard(h => {{ const v = get(h); return ArrayBuffer.isView(v) ? (v.byteOffset >>> 0) : 0; }}),
    // Calls fn(ctx, ok, value) once the promise settles: ok 1 with its value, 0 with its reason.
    js_promise_then: guard((p, fn, ctx) => {{
      const settle = ok => v => {{ {settle} }};
      Promise.resolve(get(p)).then(settle(1), settle(0));
    }}),
"
    );
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
                Some(t) => arms.push(format!("if ({}) return {i};", js_test(t, u.discriminant.as_deref(), &m.js_refs))),
                None => default = Some(i),
            }
        }
        let default = match (default, param) {
            (Some(d), _) => d.to_string(),
            (None, Some(p)) => format!("(k === 0 ? {p} : -1)"),
            (None, None) => "-1".into(),
        };
        out += &format!("    {}_tag: guard((h, k) => {{ const v = get(h); {} return {default}; }}),\n", u.name, arms.join(" "));
    }
    for c in m.callbacks.iter().filter(|c| wanted(&format!("{}_from", c.name))) {
        let names: Vec<String> = (0..c.params.len()).map(|i| format!("a{i}")).collect();
        let args: Vec<String> = c.params.iter().zip(&names).map(|((_, t), a)| to_wasm(a, t)).collect();
        // What a callback returns, wasm gives up.
        let ret = from_wasm("r", &c.ret).replace("get(r)", "take(r)");
        let body = lending(
            &|args| {
                let call = if args.is_empty() { "f(ctx)".to_string() } else { format!("f(ctx, {args})") };
                if c.ret == Ty::Void { format!("{call};") } else { format!("const r = {call}; return {ret};") }
            },
            &args,
        );
        out += &format!("    {}_from: (fn, ctx) => callback({:?}, fn, ctx, (f, s) => ({}) => {{ if (!s.alive) dead(); {body} }}),\n", c.name, c.name, names.join(", "));
    }
    // A call that throws returns zero, and its exception is kept for wasm to take.
    let guarded = |body: &str| match body.strip_prefix('{') {
        Some(_) => format!("{{ try {body} catch (e) {{ fail(e); }} }}"),
        None => format!("{{ try {{ return {body}; }} catch (e) {{ return fail(e); }} }}"),
    };
    for f in m.all_funcs().filter(|f| f.shared.is_none() && wanted(&f.import)) {
        let (params, body) = entry(f, &|_, literal| literal.to_string());
        out += &format!("    {}: ({}) => {},\n", f.import, params.join(", "), guarded(&body));
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
        out += &format!("    {}: (() => {{ const F = [{}]; return ({}) => {}; }})(),\n", s.name, table.join(", "), params.join(", "), guarded(&body));
    }
    out + "  };\n  return env;\n}\n"
}

// A function with its types rewritten.
pub fn map_types(f: &Func, fix: &dyn Fn(&Ty) -> Ty) -> Func {
    Func {
        type_params: f.type_params.clone(),
        module: f.module.clone(),
        import: f.import.clone(),
        name: f.name.clone(),
        call: f.call.clone(),
        params: f.params.iter().map(|(n, t)| (n.clone(), fix(t))).collect(),
        rest: f.rest,
        fixed: f.fixed.clone(),
        shared: f.shared.clone(),
        ret: fix(&f.ret),
    }
}

// Rewrites a type and every type inside it.
pub fn walk(t: &Ty, f: &dyn Fn(&Ty) -> Option<Ty>) -> Ty {
    if let Some(t) = f(t) {
        return t;
    }
    match t {
        Ty::Nullable(inner, u) => Ty::Nullable(Box::new(walk(inner, f)), *u),
        Ty::Listener(inner) => Ty::Listener(Box::new(walk(inner, f))),
        Ty::ArrayOf(inner) => Ty::ArrayOf(Box::new(walk(inner, f))),
        Ty::Tuple(es) => Ty::Tuple(es.iter().map(|e| walk(e, f)).collect()),
        Ty::Generic(n, args) => Ty::Generic(n.clone(), args.iter().map(|a| walk(a, f)).collect()),
        t => t.clone(),
    }
}

// A function of a class template, where the class's own name means the
// class with its own parameters: `Box` is `Box<T>`.
pub fn with_self_args(f: &Func, c: &Class) -> Func {
    if c.type_params.is_empty() {
        return map_types(f, &|t| t.clone());
    }
    let own = Ty::Generic(c.name.clone(), c.type_params.iter().map(|p| Ty::Param(p.name.clone())).collect());
    map_types(f, &|t| walk(t, &|t| matches!(t, Ty::Obj(n) if *n == c.name).then(|| own.clone())))
}

// A type with type parameters replaced by arguments, or their defaults.
pub fn subst(t: &Ty, params: &[TParam], args: &[Ty]) -> Ty {
    walk(t, &|t| match t {
        Ty::Param(n) => params.iter().position(|p| p.name == *n).map(|i| args.get(i).cloned().or_else(|| params[i].default.clone()).unwrap_or(Ty::Any)),
        _ => None,
    })
}

// The type arguments `c` gives an ancestor, in terms of its own parameters:
// for `Pipe extends Source<number>`, Source's are `[number]`.
pub fn base_args(m: &Model, c: &Class, ancestor: &str) -> Option<Vec<Ty>> {
    for b in &c.bases {
        let (name, args) = match b {
            Ty::Obj(n) => (n, Vec::new()),
            Ty::Generic(n, args) => (n, args.clone()),
            _ => continue,
        };
        if name == ancestor {
            return Some(args);
        }
        if let Some(k) = m.classes.iter().find(|k| k.name == *name)
            && let Some(inner) = base_args(m, k, ancestor)
        {
            return Some(inner.iter().map(|t| subst(t, &k.type_params, &args)).collect());
        }
    }
    None
}

// Calls `f` with every type the model names, and the types inside it, and where: for a generator to check what it can express.
pub fn each_type(m: &Model, f: &mut dyn FnMut(&str, &Ty)) {
    fn visit(at: &str, t: &Ty, f: &mut dyn FnMut(&str, &Ty)) {
        f(at, t);
        match t {
            Ty::Nullable(inner, _) | Ty::Listener(inner) | Ty::ArrayOf(inner) => visit(at, inner, f),
            Ty::Generic(_, args) | Ty::Tuple(args) | Ty::Union(args) => args.iter().for_each(|a| visit(at, a, f)),
            _ => {}
        }
    }
    let func = |owner: &str, func: &Func, f: &mut dyn FnMut(&str, &Ty)| {
        let at = if owner.is_empty() { func.name.clone() } else { format!("{owner}.{}", func.name) };
        func.params.iter().for_each(|p| visit(&at, &p.1, f));
        visit(&at, &func.ret, f);
        if let Some(p) = func.type_params.first() {
            f(&at, &Ty::Param(p.name.clone()));
        }
    };
    for c in &m.classes {
        if let Some(p) = c.type_params.first() {
            f(&c.name, &Ty::Param(p.name.clone()));
        }
        c.bases.iter().chain(&c.iterates).chain(&c.iterates_async).for_each(|t| visit(&c.name, t, f));
        c.funcs.iter().for_each(|x| func(&c.name, x, f));
    }
    m.funcs.iter().for_each(|x| func("", x, f));
    for c in &m.callbacks {
        c.params.iter().for_each(|p| visit(&c.name, &p.1, f));
        visit(&c.name, &c.ret, f);
    }
    for u in &m.unions {
        u.parts.iter().for_each(|p| visit(&u.name, p, f));
    }
    for k in &m.maps {
        k.values.iter().for_each(|v| visit(&k.name, v, f));
    }
}
