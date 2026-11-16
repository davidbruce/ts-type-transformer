// The binding model: what generators for targets without methods, overloads,
// optional arguments or subtyping (C, C++, Zig, Rust, Gleam) need from the
// lowered XML. Properties are functions (`get_x`, `set_x`), option bags
// `create()`, overloads named apart; each function says how JavaScript is
// called (`Call`), so a generator only writes its side. The wasm boundary and
// its glue are in `wasm`.

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
    // Where its constructor is, when it is a class at runtime (not only an
    // interface): what `instanceof` tests against.
    pub js: Option<String>,
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
    // Types lowering made up (`synthetic` in the XML): they give way when
    // a name is wanted twice.
    pub synthetic: HashSet<String>,
    // Where each declared type comes from (`readline`, `fs/promises`): what
    // it is named by where its name must give way, `ReadlineKey`.
    pub origins: HashMap<String, String>,
    // Types declared in a namespace, by name: their path, the namespaces
    // and their own name (`Stream`, `PipeOptions`).
    pub namespaced: HashMap<String, Vec<String>>,
    // And the module of their namespace (`namespaces/Stream`).
    pub namespace_modules: HashMap<String, String>,
    // A namespace's functions, and its variables' accessors, by their
    // import: the namespaces, their own name, their namespace's module.
    pub func_namespaces: HashMap<String, (Vec<String>, String, String)>,
}

impl Class {
    pub fn has_self(f: &Func) -> bool {
        matches!(f.call, Call::Method(_) | Call::Get(_) | Call::Set(_) | Call::IndexGet | Call::IndexSet | Call::Invoke | Call::Construct)
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
    // A generic union's type parameters its members use, where not all:
    // the others are dropped, and the arguments given for them.
    union_kept: HashMap<String, Vec<usize>>,
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
    // See `type_ident`.
    declared: Vec<String>,
    // The XML says which declaration each reference is (`to`).
    has_ids: bool,
    idents: std::cell::RefCell<HashMap<String, String>>,
    ident_taken: std::cell::RefCell<HashSet<String>>,
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
        model: Model { enums: vec![], num_enums: vec![], classes: vec![], callbacks: vec![], unions: vec![], maps: vec![], shared: vec![], funcs: vec![], opaque: vec![], opaque_arity: BTreeMap::new(), skipped: BTreeMap::new(), counted: target == "C++" || target == "Rust", packages: BTreeMap::new(), js_refs: BTreeMap::new(), synthetic: HashSet::new(), origins: HashMap::new(), namespaced: HashMap::new(), namespace_modules: HashMap::new(), func_namespaces: HashMap::new() },
        opaque: HashSet::new(),
        imports: HashSet::new(),
        resolving: Vec::new(),
        union_kept: HashMap::new(),
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
        declared: Vec::new(),
        has_ids: doc.descendants().any(|d| d.has_attribute("id")),
        idents: Default::default(),
        ident_taken: Default::default(),
        erased: HashSet::new(),
        key_strings: HashSet::new(),
    };
    b.index(doc.root_element(), "");
    for nested in [false, true] {
        for path in b.declared.iter().filter(|p| p.contains('.') == nested) {
            b.type_ident(path);
        }
    }
    b.decls_of(doc.root_element(), "");
    let mut opaque: Vec<String> = b.opaque.into_iter().collect();
    opaque.sort();
    b.model.opaque = opaque;
    let m = &mut b.model;
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

// A reference to a type declared elsewhere, by this and its name.
const EXTERNAL: &str = "external:";

// The name of a type declared elsewhere, or the key as it is.
fn external(key: &str) -> &str {
    key.strip_prefix(EXTERNAL).unwrap_or(key)
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
                    self.declared.push(key.clone());
                    self.decls.insert(key, d);
                }
            }
        }
    }

    // Where a reference leads: the declaration its `to` names; with none, a
    // type declared elsewhere, apart from any declared here of its name
    // (GeoJSON's `Point` beside leaflet's). XML without ids is by name.
    fn ref_key(&self, n: Node) -> String {
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
    fn type_ident(&self, path: &str) -> String {
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
            if path.contains('.') && !flag(d, "synthetic") {
                self.model.namespaced.insert(self.type_ident(&path), path.split('.').map(str::to_string).collect());
                self.model.namespace_modules.insert(self.type_ident(&path), self.module.clone());
            }
            if let Some(origin) = d.attribute("origin") {
                self.model.origins.insert(self.type_ident(&path), origin.to_string());
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
                        let owner = self.type_ident(&path);
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
                    let own = name;
                    let name = ident(&join(prefix, name));
                    let import = self.import(name.clone());
                    self.namespace_func(prefix, &import, own);
                    let type_params = std::mem::take(&mut self.last_type_params);
                    self.model.funcs.push(Func { type_params, module: self.module.clone(), import, name, call: Call::Static(js), params, rest, fixed, shared: None, ret });
                }
                "var" => {
                    let body = only_child(d);
                    if body.has_tag_name("object") {
                        // A value with members of its own: bound like a class with only statics.
                        // A type may have the same name (`type NodeFilter` and `var NodeFilter`).
                        let name = if self.decls.contains_key(&path) { format!("{}_statics", self.type_ident(&path)) } else { self.type_ident(&path) };
                        // A namespace's, as its types are: `Intl.Locale`'s statics, `intl.Locale_statics`.
                        if let Some((ns, own)) = path.rsplit_once('.') {
                            let own = if self.decls.contains_key(&path) { format!("{own}_statics") } else { own.to_string() };
                            self.model.namespaced.insert(name.clone(), ns.split('.').map(str::to_string).chain([own]).collect());
                            self.model.namespace_modules.insert(name.clone(), self.module.clone());
                        }
                        let c = self.statics(body, &self.js(d, &path), name);
                        self.model.classes.push(c);
                        continue;
                    }
                    let ret = self.ty(body, "");
                    let own = name;
                    let name = format!("get_{}", ident(&path));
                    let import = self.import(name.clone());
                    self.namespace_func(prefix, &import, &format!("get_{}", ident(own)));
                    let js = self.js(d, &path);
                    self.model.funcs.push(Func { type_params: vec![], module: self.module.clone(), import, name, call: Call::StaticGet(js.clone()), params: vec![], rest: false, fixed: vec![], shared: None, ret: ret.clone() });
                    // A `const`, or a module's export, cannot be assigned.
                    if flag(d, "readonly") {
                        continue;
                    }
                    let path = js;
                    let set_name = format!("set_{}", ident(&path));
                    let set_import = self.import(set_name.clone());
                    self.namespace_func(prefix, &set_import, &format!("set_{}", ident(own)));
                    self.model.funcs.push(Func { type_params: vec![], module: self.module.clone(), import: set_import, name: set_name, call: Call::StaticSet(path), params: vec![("value".into(), value(ret))], rest: false, fixed: vec![], shared: None, ret: Ty::Void });
                }
                _ => self.unsupported(d),
            }
        }
    }

    // A namespace's function, or a variable's accessor: its namespace and its
    // own name (`Geometry`, `distance`), by its import.
    fn namespace_func(&mut self, prefix: &str, import: &str, own: &str) {
        if !prefix.is_empty() {
            self.model.func_namespaces.insert(import.to_string(), (prefix.split('.').map(str::to_string).collect(), own.to_string(), self.module.clone()));
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
        let runtime = d.has_tag_name("class").then(|| js.to_string());
        let path_js = js;
        let owner = self.type_ident(path);
        let mut c = Class { type_params: vec![], bases: vec![], module: self.module.clone(), name: owner.clone(), ancestors: Vec::new(), funcs: Vec::new(), consts: Vec::new(), iterates: None, iterates_async: None, elements: None, js: runtime };
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
        let mut c = Class { type_params: vec![], bases: vec![], module: self.module.clone(), name, ancestors: Vec::new(), funcs: Vec::new(), consts: Vec::new(), iterates: None, iterates_async: None, elements: None, js: None };
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
            let key = self.ref_key(m);
            if !self.decls.contains_key(&key) {
                self.opaque.insert(self.type_ident(&key));
            }
            return c.ancestors.push(self.type_ident(&key));
        }
        // A direct base, when inheritance is kept: members stay on the type that declares them.
        if tag == "extends" {
            let base = only_child(m);
            if base.has_tag_name("ref") {
                let key = self.ref_key(base);
                let name = key.as_str();
                // A subclass of Array is an array of its elements.
                if matches!(external(name), "Array" | "ReadonlyArray") && !self.decls.contains_key(name) {
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
                let t = if elements(base).next().is_some() && !self.generics { Ty::Obj(self.type_ident(name)) } else { self.reference(base, &owner) };
                if !self.decls.contains_key(name) {
                    self.opaque.insert(self.type_ident(name));
                }
                c.bases.push(t);
                return c.ancestors.push(self.type_ident(name));
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
                // A value of none (`[key: number]: never`) is never set.
                if value != Ty::Void {
                    let import = self.import(format!("{owner}_set_at"));
                    c.funcs.push(Func { type_params: vec![], module: String::new(), import, name: "set_at".into(), call: Call::IndexSet, params: vec![("key".into(), key), ("value".into(), value)], rest: false, fixed: vec![], shared: None, ret: Ty::Void });
                }
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
                // Named for the marks it has that its siblings do not all have
                // (`invoke_async`); the first without any is `invoke`, which
                // none before it took.
                let siblings: Vec<Node> = m.parent().map(|p| elements(p).filter(|s| s.tag_name() == m.tag_name() && !flag(*s, "static")).collect()).unwrap_or_default();
                let own = super::own_marks(&siblings.iter().map(|s| super::marks(*s)).collect::<Vec<_>>());
                let me = siblings.iter().position(|s| *s == m).unwrap_or(0);
                let plain = own.iter().position(Vec::is_empty).unwrap_or(0);
                let base = if me == plain || own[me].is_empty() { base.to_string() } else { format!("{base}_{}", own[me].join("_")) };
                let mut name = base.clone();
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
                // Its own type parameters (`new<T>(x: T)`), as a method's.
                let type_params = std::mem::take(&mut self.last_type_params);
                c.funcs.push(Func { type_params, module: String::new(), import, name, call: Call::New(path.to_string()), params, rest, fixed, shared: None, ret });
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
        let mut sig = self.signature(n, owner);
        self.erased.clear();
        // A listener (one argument, a declared `any` result) returns nothing
        // JavaScript reads: its result is void, as its writer would have it.
        if sig.0.len() == 1 && sig.3 == Ty::Any && listener_result(n) {
            sig.3 = Ty::Void;
        }
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
                    let name = self.ref_key(m);
                    if self.key_map(&name) {
                        self.keyed.insert(attr(p, "name").to_string(), self.type_ident(&name));
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
        let id = self.type_ident(name);
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
                if let Some(b) = self.decls.get(&self.ref_key(base)) {
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
            "ref" => self.reference(n, owner),
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

    // `owner` is what `this` is, in a type argument (`GetSet<number, this>`).
    fn reference(&mut self, n: Node<'a, 'a>, owner: &str) -> Ty {
        let key = self.ref_key(n);
        let declared = self.decls.get(&key).copied();
        // `Array<T>` is `T[]`.
        if declared.is_none() && matches!(external(&key), "Array" | "ReadonlyArray") {
            return match elements(n).next() {
                Some(e) => {
                    let elem = self.ty(e, owner);
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
            (_, Some(d)) if d.has_tag_name("enum") && named(d, "member").all(|m| attr(m, "type") == "string") => Ty::Enum(self.type_ident(&key)),
            (_, Some(d)) if d.has_tag_name("enum") && named(d, "member").all(|m| attr(m, "type") == "number") => Ty::NumEnum(self.type_ident(&key)),
            ("enum", _) => Ty::Num,
            (_, Some(d)) if d.has_tag_name("typeAlias") => {
                let body = self.alias_body(d);
                if body.has_tag_name("functionType") || body.has_tag_name("object") {
                    if self.generics && elements(n).next().is_some() {
                        let args: Vec<Ty> = elements(n).map(|a| self.ty(a, owner)).collect();
                        return Ty::Generic(self.type_ident(&key), args);
                    }
                    return Ty::Obj(self.type_ident(&key));
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
                    let name = self.type_ident(&key);
                    let args: Vec<Ty> = if generic_union { elements(n).map(|a| self.ty(a, owner)).collect() } else { Vec::new() };
                    // `void` in a union is a value that may be absent: `void | Promise<void>`.
                    // So is a member that may itself be absent: `string | (number | null)`.
                    let is_void = |p: Node| p.has_tag_name("primitive") && attr(p, "name") == "void";
                    let has_void = elements(body).any(|p| is_void(p) || p.has_tag_name("nullable"));
                    let named_type = |name: String, kept: Option<&Vec<usize>>| {
                        let args: Vec<Ty> = args.iter().enumerate().filter(|(i, _)| kept.is_none_or(|k| k.contains(i))).map(|(_, a)| a.clone()).collect();
                        // One left with no type parameters is no generic union.
                        let t = if generic_union && !args.is_empty() { Ty::Generic(name, args) } else { Ty::NamedUnion(name) };
                        if has_void { absent(t, true) } else { t }
                    };
                    if self.model.unions.iter().any(|u| u.name == name) {
                        return named_type(name.clone(), self.union_kept.get(&name));
                    }
                    let type_params: Vec<TParam> = named(d, "typeParameter").collect::<Vec<_>>().into_iter().map(|p| self.tparam(p)).collect();
                    let members: Vec<Node> = elements(body).filter(|p| !is_void(*p)).collect();
                    self.resolving.push(key.clone());
                    // Its own type parameters, not those of where it is named
                    // (a callback's erased `T` is not this `T`).
                    let own: Vec<String> = type_params.iter().map(|p| p.name.clone()).collect();
                    let saved = (self.erased.clone(), self.keyed.clone(), self.key_strings.clone());
                    self.erased.retain(|e| !own.contains(e));
                    self.keyed.retain(|k, _| !own.contains(k));
                    self.key_strings.retain(|k| !own.contains(k));
                    let parts: Vec<Ty> = members
                        .iter()
                        .map(|p| match self.ty(*p, "") {
                            Ty::Nullable(inner, _) => *inner,
                            t => t,
                        })
                        .collect();
                    (self.erased, self.keyed, self.key_strings) = saved;
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
                    // A type parameter no member uses (`Number<TNum> = NodeClass | number`)
                    // is no parameter of it: a target may not declare one unused.
                    let kept: Vec<usize> = (0..type_params.len()).filter(|&i| parts.iter().any(|p| mentions(p, &type_params[i].name))).collect();
                    let type_params: Vec<TParam> = if kept.len() < type_params.len() {
                        self.union_kept.insert(name.clone(), kept.clone());
                        kept.iter().map(|&i| type_params[i].clone()).collect()
                    } else {
                        type_params
                    };
                    self.model.unions.push(NamedUnion { name: name.clone(), type_params, parts, tests, discriminant });
                    return named_type(name.clone(), self.union_kept.get(&name));
                }
                self.resolving.push(key.clone());
                let t = self.ty(body, "");
                self.resolving.pop();
                t
            }
            ("class" | "interface", declared) if self.generics && elements(n).next().is_some() => {
                let args: Vec<Ty> = elements(n).map(|a| self.ty(a, owner)).collect();
                if declared.is_none() {
                    self.opaque.insert(self.type_ident(&key));
                    let arity = self.model.opaque_arity.entry(self.type_ident(&key)).or_default();
                    *arity = (*arity).max(args.len());
                }
                Ty::Generic(self.type_ident(&key), args)
            }
            ("class" | "interface", Some(_)) => Ty::Obj(self.type_ident(&key)),
            // Declared in another library: a handle, by name. An alias may be of anything, and a handle can hold anything.
            ("class" | "interface" | "typeAlias", None) => {
                self.opaque.insert(self.type_ident(&key));
                Ty::Obj(self.type_ident(&key))
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

// Whether a function type's result is an `any` the library declared, not
// one a lowering made (an erased `U`, which the caller reads).
pub fn listener_result(f: Node) -> bool {
    named(f, "returns").next().map(only_child).is_some_and(|r| r.has_tag_name("primitive") && attr(r, "name") == "any" && !r.has_attribute("lossy"))
}

// Whether a type names the type parameter `p`.
pub fn mentions(t: &Ty, p: &str) -> bool {
    match t {
        Ty::Param(n) => n == p,
        Ty::Nullable(inner, _) | Ty::ArrayOf(inner) | Ty::Listener(inner) => mentions(inner, p),
        Ty::Tuple(es) | Ty::Union(es) | Ty::Generic(_, es) => es.iter().any(|e| mentions(e, p)),
        _ => false,
    }
}
