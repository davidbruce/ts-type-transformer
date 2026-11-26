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

// What a nullable value is in JavaScript when absent.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Absence {
    Undefined,
    Null,
    // `x?: T | null`: left out, or null as a value of its own.
    Either,
    False,
}

impl Absence {
    pub fn js(self) -> &'static str {
        match self {
            Absence::Undefined | Absence::Either => "undefined",
            Absence::Null => "null",
            Absence::False => "false",
        }
    }

    // May be left out of a call.
    pub fn optional(self) -> bool {
        matches!(self, Absence::Undefined | Absence::Either)
    }
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
    // What the absent value is: `undefined`, `null`, or `false` (`T | false`).
    Nullable(Box<Ty>, Absence),
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

    // The member a value going in is by default, where a language names the
    // others (Gleam's `with_left(16.0)` beside `with_left_string("10%")`):
    // of `T | T[]` the one; a string beside objects (Konva's `string |
    // CanvasGradient`); a record beside other objects (prosemirror's `{
    // [name]: NodeSpec } | OrderedMap<NodeSpec>`); none where every member is
    // an object (ECharts' series); else the first as declared (`number |
    // string`, `Date | number | string`). An inline union's order is the
    // checker's, `string` before `number` always: there the number comes first
    // (ECharts' `fontSize`, `min`, as its `left` through `PositionSizeOption`),
    // but for a key's (`string | number | symbol`).
    pub fn primary(&self, record: impl Fn(&str) -> bool, inline: bool) -> Option<usize> {
        let object = |t: &Ty| !matches!(t, Ty::Num | Ty::Bool | Ty::Str | Ty::Enum(_) | Ty::NumEnum(_) | Ty::Any | Ty::Void);
        if let Some(i) = self.parts.iter().position(|p| self.parts.iter().any(|q| matches!(q, Ty::ArrayOf(e) if **e == *p))) {
            return Some(i);
        }
        let records: Vec<usize> = self.parts.iter().enumerate().filter(|(_, p)| matches!(p, Ty::Obj(n) if record(n))).map(|(i, _)| i).collect();
        if let [i] = records.as_slice()
            && self.parts.iter().all(object)
        {
            return Some(*i);
        }
        if let Some(i) = self.parts.iter().position(|p| *p == Ty::Str)
            && self.parts.iter().enumerate().all(|(j, q)| j == i || object(q))
        {
            return Some(i);
        }
        if self.parts.iter().all(object) {
            return None;
        }
        if inline
            && self.parts.first() == Some(&Ty::Str)
            && !self.parts.iter().any(|p| matches!(p, Ty::Obj(n) if n == "JsSymbol"))
            && let Some(i) = self.parts.iter().position(|p| matches!(p, Ty::Num | Ty::NumEnum(_)))
        {
            return Some(i);
        }
        (!self.parts.is_empty()).then_some(0)
    }

    // A value can be made of a member without telling members apart: each is a
    // number, a flag, a string, a handle, an array or a tuple.
    pub fn typable(&self) -> bool {
        self.type_params.is_empty() && self.parts.iter().all(|p| matches!(p, Ty::Num | Ty::Str | Ty::Bool | Ty::Obj(_) | Ty::Enum(_) | Ty::Array | Ty::ArrayOf(_) | Ty::Tuple(_)))
    }
}

// How an overload that differs from others only in its optional tail is
// named (ECharts' `setOption(opt, notMerge?, lazyUpdate?)` beside
// `setOption(opt, opts?)`): the one whose tail ends in an options object
// leads, the plain name and `name_with`; each other is `name_with_<its
// first optional>` alone. `name` is the overload whose name they take: the
// first, as lowering named it (`setOption`'s `set_option`).
pub enum TailRole {
    Leader { name: usize },
    Follower { name: usize, first: String },
}

// Where a function's optional tail starts: its trailing optional parameters.
pub fn tail_start(f: &Func) -> usize {
    f.params.len() - f.params.iter().rev().take_while(|(_, t)| matches!(t, Ty::Nullable(_, a) if a.optional())).count()
}

impl Model {
    pub fn tail_overloads(&self, funcs: &[Func]) -> HashMap<usize, TailRole> {
        let mut groups: Vec<Vec<usize>> = Vec::new();
        let key = |f: &Func| (format!("{:?}", f.call), f.fixed.clone(), f.params[..tail_start(f)].iter().map(|(_, t)| t.clone()).collect::<Vec<Ty>>());
        for (i, f) in funcs.iter().enumerate() {
            if f.rest || tail_start(f) == f.params.len() || !matches!(f.call, Call::Method(_) | Call::Static(_) | Call::New(_) | Call::Invoke | Call::Construct) {
                continue;
            }
            match groups.iter_mut().find(|g| key(&funcs[g[0]]) == key(f)) {
                Some(g) => g.push(i),
                None => groups.push(vec![i]),
            }
        }
        let options = |f: &Func| matches!(f.params.last(), Some((_, Ty::Nullable(inner, _))) if matches!(&**inner, Ty::Obj(n) if self.classes.iter().any(|c| c.name == *n && c.funcs.iter().any(|g| matches!(g.call, Call::Create(_))))));
        let mut out = HashMap::new();
        for g in groups.into_iter().filter(|g| g.len() > 1) {
            let leader = g.iter().copied().find(|&i| options(&funcs[i])).unwrap_or(g[0]);
            out.insert(leader, TailRole::Leader { name: g[0] });
            for &i in g.iter().filter(|&&i| i != leader) {
                let first = funcs[i].params[tail_start(&funcs[i])].0.clone();
                out.insert(i, TailRole::Follower { name: g[0], first });
            }
        }
        out
    }

    // The fields every member of a union of objects has, each read by a
    // getter of the union's own (marked's `Token`: `type` on all 23, a
    // constant on 22, `"heading"`, a property on `Generic`): a member's own
    // or its ancestors', getters and constants. Of one type, or of one
    // primitive under constants and enums of it, nullable where any member's
    // is; not where the members' disagree, nor a constant all members share
    // (`Node.ELEMENT_NODE`), nor one of a member's type parameters (`T`), nor
    // a name no target writes as is (`$action`).
    pub fn shared_fields(&self, u: &NamedUnion) -> Vec<(String, Ty)> {
        let classes: Option<Vec<&Class>> = u.parts.iter().map(|p| if let Ty::Obj(n) = p { self.classes.iter().find(|c| c.name == *n) } else { None }).collect();
        let Some(classes) = classes.filter(|c| c.len() > 1) else { return Vec::new() };
        let plain = |n: &str| n.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_') && n.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
        let fields = |c: &Class| -> Vec<(String, Ty, Option<String>)> {
            let mut out: Vec<(String, Ty, Option<String>)> = Vec::new();
            // Its bases' bases too, where inheritance is kept.
            let mut chain = vec![c];
            let mut i = 0;
            while i < chain.len() {
                for a in &chain[i].ancestors {
                    if let Some(x) = self.classes.iter().find(|x| x.name == *a)
                        && !chain.iter().any(|y| y.name == x.name)
                    {
                        chain.push(x);
                    }
                }
                i += 1;
            }
            for k in chain {
                for f in &k.funcs {
                    if let Call::Get(p) = &f.call
                        && !out.iter().any(|(n, ..)| n == p)
                        && !k.type_params.iter().any(|t| mentions(&f.ret, &t.name))
                        && plain(p)
                    {
                        out.push((p.clone(), f.ret.clone(), None));
                    }
                }
                for (n, v) in &k.consts {
                    let t = if v.starts_with('"') { Ty::Str } else if v == "true" || v == "false" { Ty::Bool } else { Ty::Num };
                    if !out.iter().any(|(m, ..)| m == n) && plain(n) {
                        out.push((n.clone(), t, Some(v.clone())));
                    }
                }
            }
            out
        };
        let per: Vec<Vec<(String, Ty, Option<String>)>> = classes.iter().map(|c| fields(c)).collect();
        let primitive = |t: &Ty| match t {
            Ty::Enum(_) => Ty::Str,
            Ty::NumEnum(_) => Ty::Num,
            t => t.clone(),
        };
        let mut out = Vec::new();
        for (name, ..) in &per[0] {
            let Some(found) = per.iter().map(|fs| fs.iter().find(|(n, ..)| n == name)).collect::<Option<Vec<_>>>() else { continue };
            if found[0].2.is_some() && found.iter().all(|f| f.2 == found[0].2) {
                continue;
            }
            let tys: Vec<&Ty> = found.iter().map(|f| &f.1).collect();
            let absent: Vec<Absence> = tys.iter().filter_map(|t| if let Ty::Nullable(_, a) = t { Some(*a) } else { None }).collect();
            let inner: Vec<&Ty> = tys.iter().map(|t| if let Ty::Nullable(i, _) = t { &**i } else { *t }).collect();
            let one = if inner.iter().all(|t| *t == inner[0]) {
                inner[0].clone()
            } else if inner.iter().all(|t| primitive(t) == primitive(inner[0])) {
                primitive(inner[0])
            } else {
                continue;
            };
            if matches!(one, Ty::Void | Ty::Any | Ty::Param(_) | Ty::Key(_) | Ty::Mapped(_) | Ty::Listener(_)) || absent.contains(&Absence::False) {
                continue;
            }
            // Read, null and undefined are both none.
            let a = if absent.is_empty() {
                None
            } else if absent.iter().all(|a| *a == absent[0]) {
                Some(absent[0])
            } else {
                Some(Absence::Either)
            };
            out.push((name.clone(), match a { Some(a) => Ty::Nullable(Box::new(one), a), None => one }));
        }
        out
    }

    pub fn is_callback(&self, name: &str) -> bool {
        self.callbacks.iter().any(|c| c.name == name)
    }

    // An object of string keys: `{ [name: string]: NodeSpec }`.
    pub fn is_record(&self, class: &str) -> bool {
        self.classes.iter().find(|c| c.name == class).is_some_and(|c| c.funcs.iter().any(|f| f.call == Call::IndexSet))
    }

    // The `type` an option object's `create` fixes: `{"type": "category", ..}`.
    pub fn fixed_type(&self, class: &str) -> Option<String> {
        let c = self.classes.iter().find(|c| c.name == class)?;
        c.funcs.iter().find_map(|f| match &f.call {
            Call::Create(lit) => {
                let at = lit.find("\"type\": \"")? + "\"type\": \"".len();
                Some(lit[at..].split('"').next()?.to_string())
            }
            _ => None,
        })
    }

    // An array element of the union's own members: ECharts' `SeriesOption` in
    // `BarSeriesOption | LineSeriesOption | .. | SeriesOption[]`.
    pub fn of_members(&self, e: &Ty, u: &NamedUnion) -> bool {
        match e {
            Ty::NamedUnion(n) => self.unions.iter().find(|v| v.name == *n).is_some_and(|v| v.parts.iter().all(|p| u.parts.contains(p))),
            t => u.parts.iter().filter(|p| *p == t).count() == 1,
        }
    }

    // The member a union's plain name takes (see `NamedUnion::primary`; where
    // every member is an object, the one named by just what they all share:
    // Leaflet's `Icon` beside `DivIcon`), and each member's part of a name for it.
    pub fn member_names(&self, u: &NamedUnion, type_name: &dyn Fn(&str) -> String) -> (Option<usize>, Vec<String>) {
        let primary = u.primary(|n| self.is_record(n), self.synthetic.contains(&u.name));
        let (names, base) = self.member_suffixes(u, primary, type_name);
        // Of objects only, a lone array of something else is what a user most
        // readily makes (Konva's `points`, `number[]` beside typed arrays).
        let arrays: Vec<usize> = u.parts.iter().enumerate().filter(|(_, p)| matches!(p, Ty::Array | Ty::ArrayOf(_) | Ty::Tuple(_))).map(|(i, _)| i).collect();
        let lone_array = match arrays.as_slice() {
            [a] if names[*a] != "list" => Some(*a),
            _ => None,
        };
        match (primary, base.or(lone_array)) {
            (None, Some(b)) => (Some(b), self.member_suffixes(u, Some(b), type_name).0),
            _ => (primary, names),
        }
    }

    // Each member's part of a name for it (Gleam's `with_left_string`, C's
    // `.left_string`): `string`, `number`, `list` (of `T | T[]`), `fn`; an
    // option object by the `type` its `create` fixes (ECharts' axes:
    // `category`, `value`); one lowering made up by the last of its name
    // (`EditorView_new_place_object`); else its type's name, as `type_name`
    // has it, without the words every such member has (`BarSeriesOption |
    // LineSeriesOption`: `bar`, `line`). And the one member those words
    // alone name, if any.
    fn member_suffixes(&self, u: &NamedUnion, primary: Option<usize>, type_name: &dyn Fn(&str) -> String) -> (Vec<String>, Option<usize>) {
        let snake = super::case::snake_case_alnum;
        let typed: Vec<Option<String>> = u
            .parts
            .iter()
            .map(|p| match p {
                Ty::Obj(n) if self.is_callback(n) => None,
                Ty::Obj(n) | Ty::Generic(n, _) | Ty::Enum(n) | Ty::NamedUnion(n) => Some(type_name(n)),
                _ => None,
            })
            .collect();
        let fixed: Vec<Option<String>> = u.parts.iter().map(|p| if let Ty::Obj(n) = p { self.fixed_type(n) } else { None }).collect();
        let objects = typed.iter().filter(|t| t.is_some()).count();
        let mut values: Vec<&String> = fixed.iter().flatten().collect();
        values.sort();
        values.dedup();
        let by_type = objects > 1 && fixed.iter().zip(&typed).all(|(f, t)| t.is_none() || f.is_some()) && values.len() == objects;
        let words: Vec<Vec<String>> = typed.iter().flatten().map(|n| camel_words(n)).collect();
        let (mut head, mut tail) = (0, 0);
        if words.len() > 1 {
            let shortest = words.iter().map(Vec::len).min().unwrap_or(0);
            while head < shortest && words.iter().all(|w| w[head] == words[0][head]) {
                head += 1;
            }
            while tail < shortest - head && words.iter().all(|w| w[w.len() - 1 - tail] == words[0][words[0].len() - 1 - tail]) {
                tail += 1;
            }
        }
        let bare: Vec<usize> = if words.len() > 1 && !by_type {
            typed.iter().enumerate().filter(|(_, n)| n.as_ref().is_some_and(|n| camel_words(n).len() == head + tail)).map(|(i, _)| i).collect()
        } else {
            Vec::new()
        };
        let base = if let [b] = bare.as_slice() { Some(*b) } else { None };
        let names = u.parts
            .iter()
            .zip(&typed)
            .enumerate()
            .map(|(m, (p, name))| match p {
                Ty::Str => "string".into(),
                Ty::Num | Ty::NumEnum(_) => "number".into(),
                Ty::Bool => "boolean".into(),
                Ty::ArrayOf(e) if primary.is_some_and(|q| **e == u.parts[q]) || self.of_members(e, u) => "list".into(),
                Ty::Obj(n) if self.is_callback(n) => "fn".into(),
                Ty::Obj(n) if n == "JsSymbol" => "symbol".into(),
                // Untyped, the only array beside objects: `T | T[]` of a union `T`, flattened.
                Ty::Array if primary.is_none() && u.parts.iter().filter(|q| matches!(q, Ty::Array | Ty::ArrayOf(_) | Ty::Tuple(_))).count() == 1 => "list".into(),
                Ty::Array | Ty::ArrayOf(_) | Ty::Tuple(_) => "array".into(),
                _ if by_type => snake(&fixed[m].clone().unwrap_or_default().replace(['-', ' ', '.'], "_")),
                Ty::Obj(n) | Ty::NamedUnion(n) if self.synthetic.contains(n) && n.contains('_') => snake(n.rsplit('_').next().unwrap_or(n)),
                _ => match name {
                    Some(n) => {
                        let w = camel_words(n);
                        let kept = &w[head.min(w.len())..w.len().saturating_sub(tail).max(head.min(w.len()))];
                        snake(&if kept.is_empty() { n.clone() } else { kept.concat() })
                    }
                    None => format!("case{m}"),
                },
            })
            .collect();
        (names, base)
    }
}

// `XAXisComponentOption` as `XA`, `Xis`, `Component`, `Option`: a word starts
// at a capital after a lowercase letter or a digit, at an acronym's last
// capital before a lowercase letter (`GLBuffer`: `GL`, `Buffer`), or after an
// underscore (C's `Echarts_BarSeriesOption`).
pub fn camel_words(n: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let chars: Vec<char> = n.chars().collect();
    let mut fresh = true;
    for (i, &c) in chars.iter().enumerate() {
        if c == '_' {
            fresh = true;
            continue;
        }
        let prev = (i > 0).then(|| chars[i - 1]).filter(|p| *p != '_');
        let next_lower = chars.get(i + 1).is_some_and(|n| n.is_ascii_lowercase());
        // A capital after a lowercase letter or a digit, or the last of an acronym before a word (`GLBuffer`).
        let starts = c.is_ascii_uppercase() && prev.is_some_and(|p| p.is_ascii_lowercase() || p.is_ascii_digit() || (p.is_ascii_uppercase() && next_lower));
        if fresh || starts {
            out.push(String::new());
            fresh = false;
        }
        out.last_mut().unwrap().push(c);
    }
    out
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
    // Functions whose optional arguments are an option bag, by import: the
    // bag's properties, in the order the function takes them.
    pub args_bags: HashMap<String, Vec<String>>,
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
    // `T | T[]` of a union `T` keeps its array's element (a Gleam list of `T` going in).
    pub typed_union_arrays: bool,
    // More than four optional arguments are an option bag of them (see `Builder::args_bags`).
    pub args_bags: bool,
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
            typed_union_arrays: target == "Gleam",
            args_bags: target == "Gleam",
        }
    }
}

// A function's name with a member's suffix, the words they share said once:
// `node` with `node_type` is `node_type`, not `node_node_type`. A suffix all
// of whose words the name ends with is kept whole (`set_node` with `node`).
pub fn suffixed(name: &str, suffix: &str) -> String {
    let n: Vec<&str> = name.split('_').collect();
    let s: Vec<&str> = suffix.split('_').collect();
    let shared = (1..s.len()).rev().find(|&k| k <= n.len() && n[n.len() - k..] == s[..k]).unwrap_or(0);
    format!("{name}_{}", s[shared..].join("_"))
}

#[cfg(test)]
mod tests {
    use super::suffixed;

    #[test]
    fn suffixes_say_shared_words_once() {
        assert_eq!(suffixed("node", "node_type"), "node_type");
        assert_eq!(suffixed("create_mark", "mark_type"), "create_mark_type");
        assert_eq!(suffixed("set_node", "node"), "set_node_node");
        assert_eq!(suffixed("add_days", "string"), "add_days_string");
    }
}
