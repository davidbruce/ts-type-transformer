// The binding model: what generators for targets without methods, overloads,
// optional arguments or subtyping (C, C++, Zig, Rust, Gleam) need from the
// lowered XML. Properties are functions (`get_x`, `set_x`), option bags
// `create()`, overloads named apart; each function says how JavaScript is
// called (`Call`), so a generator only writes its side. The wasm boundary and
// its glue are in `wasm`.

use std::collections::{BTreeMap, HashMap, HashSet};

mod build;
mod build_types;
mod ops;

pub use build::{model, model_with_options};
pub use ops::{
    base_args, each_type, listener_result, manifest, map_types, mentions, subst, walk,
    with_self_args,
};

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
    // A property holding a function, read bound to its object: called later,
    // it is its method still (Konva's `node.x`, TanStack's `table.getRowModel`).
    GetBound(String),
    Set(String),
    // The rest are reached from a global path such as `WebAssembly.Module`.
    New(String),
    Static(String),
    StaticGet(String),
    StaticSet(String),
    // An object, for interfaces used as option bags: empty but for the
    // values its type fixes (`{ type: "bar" }`), as a JavaScript literal.
    Create(String),
    // The object called, or constructed with `new`: a callable interface.
    Invoke,
    Construct,
    // `object[key]`, from an index signature.
    IndexGet,
    IndexSet,
    // Its first argument, a function, called with the rest: what calls a
    // callback a value holds (`call_Transaction_fn(view.dispatch, tr)`).
    Apply,
}

#[derive(Clone)]
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
    // Declared `: this` in TypeScript.
    pub returns_this: bool,
    // What its JSDoc says, then where it was called in JavaScript, when lowering changed that.
    pub doc: Option<String>,
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
    pub doc: Option<String>,
}

impl Class {
    pub fn has_self(f: &Func) -> bool {
        matches!(f.call, Call::Method(_) | Call::Get(_) | Call::GetBound(_) | Call::Set(_) | Call::IndexGet | Call::IndexSet | Call::Invoke | Call::Construct)
    }
}

// An enum's documentation and its members', in the order of `Model::enums` or `num_enums`.
#[derive(Default)]
pub struct EnumDoc {
    pub doc: Option<String>,
    pub members: Vec<Option<String>>,
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

    // A value can be made of a member without telling members apart: each is a
    // number, a flag, a string, a handle, an array or a tuple.
    pub fn typable(&self) -> bool {
        self.type_params.is_empty() && self.parts.iter().all(|p| matches!(p, Ty::Num | Ty::Str | Ty::Bool | Ty::Obj(_) | Ty::Enum(_) | Ty::Array | Ty::ArrayOf(_) | Ty::Tuple(_)))
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
    // Its last parameter gathers the rest of the arguments (`...args: any[]`):
    // JavaScript calls it with them one by one.
    pub rest: bool,
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
    pub enum_docs: Vec<EnumDoc>,
    pub num_enum_docs: Vec<EnumDoc>,
    pub classes: Vec<Class>,
    pub callbacks: Vec<Callback>,
    pub unions: Vec<NamedUnion>,
    // A library's name for a union, and the union.
    pub type_aliases: Vec<(String, String)>,
    pub maps: Vec<KeyMap>,
    pub shared: Vec<Shared>,
    pub funcs: Vec<Func>,
    // Types declared elsewhere: handles with no members, and their original external names before collision renaming.
    pub opaque: Vec<String>,
    pub opaque_names: BTreeMap<String, String>,
    // How many type arguments those that are generic take.
    pub opaque_arity: BTreeMap<String, usize>,
    // What the generator does not bind yet, by reason.
    pub skipped: BTreeMap<String, usize>,
    // Handles are counted references: wasm retains and releases them, and
    // owns what a callback returns. Else a handle lives as long as the page.
    pub counted: bool,
    // Uncaught JS exceptions throw out of the wasm import call immediately
    // unless inside `attempt` (`attemptDepth > 0`).
    pub trap_uncaught: bool,
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
    // And their module (`namespaces/Stream`), with how many of the path's
    // first segments it stands for.
    pub namespace_modules: HashMap<String, (String, usize)>,
    // A namespace's functions, and its variables' accessors, by their
    // import: the namespaces, their own name, their module, and how many of
    // the namespaces it stands for.
    pub func_namespaces: HashMap<String, (Vec<String>, String, String, usize)>,
    // With several packages bound together, those asked for: a program must
    // give their modules; others only where used.
    pub requested_packages: Vec<String>,
    // Optional explicit symbol prefix from `<declarations prefix="...">`.
    pub prefix: Option<String>,
    // ES module from `<declarations import="...">`, if this is a module library.
    pub root_import: Option<String>,
    // Emitted C struct name -> original unprefixed model type name (when prefixed/disambiguated).
    pub original_type_names: BTreeMap<String, String>,
}

// A file or folder name that no other one in `taken` equals ignoring letter
// case, and that is not reserved on Windows: `foo` after `Foo` is `foo_2`.
pub fn file_name(path: &str, taken: &mut HashMap<String, String>) -> String {
    let (dir, name) = path.rsplit_once('/').map_or(("", path), |(d, n)| (d, n));
    let mut candidate = if crate::config::reserved(name) { format!("{name}_") } else { name.to_string() };
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

#[derive(Clone, Copy)]
pub struct ModelOptions<'a> {
    pub target: &'a str,
    pub counted: bool,
    pub trap_uncaught: bool,
    pub generics: bool,
    pub overloading: bool,
    pub retain_opaque_constraints: bool,
}

impl<'a> ModelOptions<'a> {
    pub fn for_target(target: &'a str) -> Self {
        Self {
            target,
            counted: matches!(target, "C++" | "Rust"),
            trap_uncaught: target == "Rust",
            generics: matches!(target, "C++" | "Rust" | "Gleam"),
            overloading: target == "C++",
            retain_opaque_constraints: target == "Gleam",
        }
    }
}
