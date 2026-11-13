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

#[derive(Clone, PartialEq)]
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
}

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
    // Name of the wasm import, unique in the module.
    pub import: String,
    // Name in the target language, unique in its class.
    pub name: String,
    pub call: Call,
    pub params: Vec<(String, Ty)>,
    // The last parameter is an array spread into the call.
    pub rest: bool,
    pub ret: Ty,
}

pub struct Class {
    pub name: String,
    pub ancestors: Vec<String>,
    pub funcs: Vec<Func>,
    // Name, value as source text.
    pub consts: Vec<(String, String)>,
}

pub struct Callback {
    pub name: String,
    pub params: Vec<(String, Ty)>,
    pub ret: Ty,
}

pub struct Model {
    pub enums: Vec<(String, Vec<String>)>,
    // Name, then member names and values.
    pub num_enums: Vec<(String, Vec<(String, String)>)>,
    pub classes: Vec<Class>,
    pub callbacks: Vec<Callback>,
    pub funcs: Vec<Func>,
    // Types declared elsewhere: handles with no members.
    pub opaque: Vec<String>,
    // What the generator does not bind yet, by reason.
    pub skipped: BTreeMap<String, usize>,
}

impl Class {
    pub fn has_self(f: &Func) -> bool {
        matches!(f.call, Call::Method(_) | Call::Get(_) | Call::Set(_) | Call::IndexGet | Call::IndexSet)
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
}

pub fn model(xml: &str, target: &str) -> Model {
    let doc = Document::parse(xml).expect("well-formed XML");
    let mut b = Builder {
        target,
        decls: HashMap::new(),
        model: Model { enums: vec![], num_enums: vec![], classes: vec![], callbacks: vec![], funcs: vec![], opaque: vec![], skipped: BTreeMap::new() },
        opaque: HashSet::new(),
        imports: HashSet::new(),
        resolving: Vec::new(),
    };
    b.index(doc.root_element(), "");
    b.decls_of(doc.root_element(), "");
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

    fn decls_of(&mut self, scope: Node<'a, 'a>, prefix: &str) {
        for d in elements(scope) {
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
                "interface" | "class" => {
                    let c = self.class(d, &path);
                    self.model.classes.push(c);
                }
                "typeAlias" => {
                    let body = only_child_after_params(d, self.target);
                    if body.has_tag_name("functionType") {
                        let owner = ident(&path);
                        let (params, _, ret) = self.signature(body, &owner);
                        self.model.callbacks.push(Callback { name: owner, params, ret });
                    }
                    // A named object type is an interface in all but syntax.
                    if body.has_tag_name("object") {
                        let c = self.class(body, &path);
                        self.model.classes.push(c);
                    }
                    // Other aliases are resolved where they are used.
                }
                "function" => {
                    let (params, rest, ret) = self.signature(d, "");
                    let js = join(prefix, d.attribute("jsName").unwrap_or(name));
                    let name = ident(&join(prefix, name));
                    let import = self.import(name.clone());
                    self.model.funcs.push(Func { import, name, call: Call::Static(js), params, rest, ret });
                }
                "var" => {
                    let body = only_child(d);
                    if body.has_tag_name("object") {
                        // A value with members of its own: bound like a class with only statics.
                        let c = self.statics(body, &path);
                        self.model.classes.push(c);
                        continue;
                    }
                    let ret = self.ty(body, "");
                    let name = format!("get_{}", ident(&path));
                    let import = self.import(name.clone());
                    self.model.funcs.push(Func { import, name, call: Call::StaticGet(path), params: vec![], rest: false, ret });
                }
                _ => unsupported(self.target, d),
            }
        }
    }

    fn class(&mut self, d: Node<'a, 'a>, path: &str) -> Class {
        let owner = ident(path);
        let mut c = Class { name: owner.clone(), ancestors: Vec::new(), funcs: Vec::new(), consts: Vec::new() };
        let mut names = HashSet::new();
        if !d.has_tag_name("class") && !named(d, "call").any(|_| true) {
            names.insert("create".to_string());
            let import = self.import(format!("{owner}_create"));
            c.funcs.push(Func { import, name: "create".into(), call: Call::Create, params: vec![], rest: false, ret: Ty::Obj(owner.clone()) });
        }
        for m in elements(d) {
            self.member(&mut c, &mut names, m, path, m.has_tag_name("constructor") || flag(m, "static"));
        }
        c
    }

    fn statics(&mut self, body: Node<'a, 'a>, path: &str) -> Class {
        let mut c = Class { name: ident(path), ancestors: Vec::new(), funcs: Vec::new(), consts: Vec::new() };
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
            return c.ancestors.push(ident(raw_name));
        }
        if matches!(tag, "extends" | "typeParameter") {
            unsupported(self.target, m);
        }
        if tag == "index" && !is_static {
            let (params, _, value) = self.signature(m, &owner);
            let key = params.into_iter().next().map(|p| p.1).unwrap_or(Ty::Str);
            if names.insert("get_at".into()) {
                // A missing key reads as `undefined`.
                let import = self.import(format!("{owner}_get_at"));
                c.funcs.push(Func { import, name: "get_at".into(), call: Call::IndexGet, params: vec![("key".into(), key.clone())], rest: false, ret: absent(value.clone(), true) });
                let import = self.import(format!("{owner}_set_at"));
                c.funcs.push(Func { import, name: "set_at".into(), call: Call::IndexSet, params: vec![("key".into(), key), ("value".into(), value)], rest: false, ret: Ty::Void });
            }
            return;
        }
        if matches!(tag, "call" | "index" | "get" | "set") || tag == "construct" && !is_static {
            return self.skip(&format!("<{tag}> member"));
        }
        if raw_name.starts_with('[') {
            return self.skip("symbol-named member");
        }
        let unique = |base: String, names: &mut HashSet<String>| {
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
                let (params, rest, _) = self.signature(m, &owner);
                let name = unique(if raw_name.is_empty() { "new".into() } else { ident(raw_name) }, names);
                let import = self.import(format!("{owner}_{name}"));
                c.funcs.push(Func { import, name, call: Call::New(path.to_string()), params, rest, ret: Ty::Obj(owner) });
            }
            "method" => {
                let (params, rest, ret) = self.signature(m, &owner);
                let call = if is_static { Call::Static(format!("{path}.{js}")) } else { Call::Method(js) };
                let name = unique(ident(raw_name), names);
                let import = self.import(format!("{owner}_{name}"));
                c.funcs.push(Func { import, name, call, params, rest, ret });
            }
            // A constant needs no call across the boundary.
            "property" if m.has_attribute("const") => {
                let quoted = only_child(m).attribute("name") == Some("string");
                let value = attr(m, "const");
                c.consts.push((ident(raw_name), if quoted { format!("{value:?}") } else { value.to_string() }));
            }
            "property" => {
                let mut t = self.ty(only_child(m), &owner);
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
                c.funcs.push(Func { import, name, call: get, params: vec![], rest: false, ret: t.clone() });
                if !flag(m, "readonly") {
                    let name = unique(format!("set_{}", ident(raw_name)), names);
                    let import = self.import(format!("{owner}_{name}"));
                    c.funcs.push(Func { import, name, call: set, params: vec![("value".into(), t)], rest: false, ret: Ty::Void });
                }
            }
            _ => unsupported(self.target, m),
        }
    }

    fn signature(&mut self, n: Node<'a, 'a>, owner: &str) -> (Vec<(String, Ty)>, bool, Ty) {
        let mut params = Vec::new();
        let mut rest = false;
        let mut ret = Ty::Void;
        for p in elements(n) {
            match p.tag_name().name() {
                // The receiver of a callback is not an argument.
                "this" => {}
                "parameter" if flag(p, "optional") => unsupported(self.target, p),
                "parameter" => {
                    rest |= flag(p, "rest");
                    params.push((ident(attr(p, "name")), self.ty(only_child(p), owner)));
                }
                "returns" => ret = self.ty(only_child(p), owner),
                _ => unsupported(self.target, p),
            }
        }
        (params, rest, ret)
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
            "ref" => self.reference(n),
            "nullable" => {
                let inner = self.ty(only_child(n), owner);
                absent(inner, attr(n, "absent") == "undefined")
            }
            "array" | "tuple" => Ty::Array,
            "union" => {
                let parts: Vec<Ty> = elements(n).map(|p| self.ty(p, owner)).collect();
                // Only alternatives a tagged union can hold directly.
                if parts.iter().all(|p| matches!(p, Ty::Num | Ty::Str | Ty::Bool | Ty::Obj(_))) && distinct(&parts) {
                    Ty::Union(parts)
                } else {
                    self.skip("union bound as an untyped value");
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
            ("enum", Some(d)) if d.has_tag_name("enum") && named(d, "member").all(|m| attr(m, "type") == "string") => Ty::Enum(ident(&key)),
            ("enum", Some(d)) if d.has_tag_name("enum") && named(d, "member").all(|m| attr(m, "type") == "number") => Ty::NumEnum(ident(&key)),
            ("enum", _) => Ty::Num,
            (_, Some(d)) if d.has_tag_name("typeAlias") => {
                let body = only_child_after_params(d, self.target);
                if body.has_tag_name("functionType") || body.has_tag_name("object") {
                    return Ty::Obj(ident(&key));
                }
                if self.resolving.contains(&key) || self.resolving.len() > 8 {
                    return Ty::Any;
                }
                self.resolving.push(key.clone());
                let t = self.ty(body, "");
                self.resolving.pop();
                t
            }
            ("class" | "interface", Some(_)) => Ty::Obj(ident(&key)),
            ("class" | "interface", None) => {
                self.opaque.insert(ident(&key));
                Ty::Obj(ident(&key))
            }
            _ => Ty::Any,
        }
    }
}

// The single type of an alias, which may be preceded by type parameters.
fn only_child_after_params<'a>(d: Node<'a, 'a>, target: &str) -> Node<'a, 'a> {
    if let Some(p) = named(d, "typeParameter").next() {
        unsupported(target, p);
    }
    only_child(d)
}

fn distinct(parts: &[Ty]) -> bool {
    parts.iter().enumerate().all(|(i, p)| !parts[..i].contains(p))
}

// A handle can be null; anything else that may be absent is an untyped value.
fn absent(t: Ty, undefined: bool) -> Ty {
    match t {
        Ty::Obj(_) | Ty::Array => Ty::Nullable(Box::new(t), undefined),
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
        Ty::Nullable(_, true) => format!("({p} === 0 ? undefined : get({p}))"),
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
        _ => format!("put({x})"),
    }
}

// JavaScript that implements every import by calling the real API.
pub fn glue(m: &Model) -> String {
    let mut out = String::from(
        "// Generated. Implements the wasm imports by calling the JavaScript API.
export function makeEnv(getInstance) {
  const objs = [null];
  // One handle per value, so handles can be compared for identity.
  const ids = new Map();
  const put = v => {
    if (v === null || v === undefined) return 0;
    if (!ids.has(v)) ids.set(v, objs.push(v) - 1);
    return ids.get(v);
  };
  const get = h => objs[h];
  const memory = () => getInstance().exports.memory.buffer;
  const table = () => getInstance().exports.__indirect_function_table;
  const enc = new TextEncoder(), dec = new TextDecoder();
  const str = (ptr, len) => dec.decode(new Uint8Array(memory(), ptr, len));
  // One JavaScript function per wasm function and context, so that a
  // callback passed twice is the same function (removeEventListener).
  const callbacks = new Map();
  const callback = (kind, fn, ctx, make) => {
    const key = `${kind}:${fn}:${ctx}`;
    if (!callbacks.has(key)) callbacks.set(key, make(table().get(fn)));
    return put(callbacks.get(key));
  };
  const enums = {
",
    );
    for (name, values) in &m.enums {
        out += &format!("    {name}: {values:?},\n");
    }
    out += "  };
  return {
    js_string_len: h => enc.encode(get(h)).length,
    js_string_read: (h, ptr) => { new Uint8Array(memory()).set(enc.encode(get(h)), ptr); },
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
";
    for c in &m.callbacks {
        let names: Vec<String> = (0..c.params.len()).map(|i| format!("a{i}")).collect();
        let mut args = vec!["ctx".to_string()];
        args.extend(c.params.iter().zip(&names).map(|((_, t), a)| to_wasm(a, t)));
        let call = format!("f({})", args.join(", "));
        let body = if c.ret == Ty::Void { format!("{{ {call}; }}") } else { from_wasm(&call, &c.ret) };
        out += &format!("    {}_from: (fn, ctx) => callback({:?}, fn, ctx, f => ({}) => {body}),\n", c.name, c.name, names.join(", "));
    }
    for f in m.all_funcs() {
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
            let spread = if f.rest && i == f.params.len() - 1 { "..." } else { "" };
            args.push(format!("{spread}{}", from_wasm(&p, t)));
            wasm_params.push(p);
        }
        let (first, second) = (args.first().cloned().unwrap_or_default(), args.get(1).cloned().unwrap_or_default());
        let args = args.join(", ");
        let call = match &f.call {
            Call::Method(js) => format!("get(self).{js}({args})"),
            Call::Get(prop) => format!("get(self).{prop}"),
            Call::Set(prop) => format!("get(self).{prop} = {args}"),
            Call::New(path) => format!("new globalThis.{path}({args})"),
            Call::Static(path) => format!("globalThis.{path}({args})"),
            Call::StaticGet(path) => format!("globalThis.{path}"),
            Call::StaticSet(path) => format!("globalThis.{path} = {args}"),
            Call::Create => "({})".to_string(),
            Call::IndexGet => format!("get(self)[{args}]"),
            Call::IndexSet => format!("get(self)[{first}] = {second}"),
        };
        let body = match &f.ret {
            Ty::Void => format!("{{ {call}; }}"),
            t => to_wasm(&call, t),
        };
        out += &format!("    {}: ({}) => {body},\n", f.import, wasm_params.join(", "));
    }
    out + "  };\n}\n"
}
