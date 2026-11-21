// XML -> Gleam bindings for Gleam's JavaScript target: modules of opaque
// types and external functions, each with an FFI module that calls the API
// and converts between JavaScript's values and Gleam's.
//
// Gleam modules may not import each other in a cycle. Every type is in
// `bindings/types`; `Promise`, `JsError` and `Codec` in `bindings/runtime`.
// An area's functions, in `bindings/<area>`, import only types, so no
// module imports one that imports it.
//
// Subtypes are phantom types along each class's chain of first bases:
// `Element(a) = Node(ElementTag(a))`. Parameters are open (`Node(a)`, any
// subtype); results, callbacks' parameters and what lists and variants hold
// are closed (`Node(Nil)`). See DESIGN.md.
//
// Gleam has functions and custom types; no methods, subtypes, overloads or
// optional arguments. So, from the shared model (`generators::model`): a class
// is an opaque type and its members functions named for it
// (`shape_get_size(s)`), an ancestor is reached by `shape_as_named(s)`, an
// optional argument or a nullable value is an `Option(a)`, an array a
// `List(a)`, a tuple a tuple, a union or string enum a custom type, a
// callback a Gleam function, a promise a `Promise(a)` to `promise_then`.
// Gleam's values are JavaScript's, but for these: the FFI converts them.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::Path;

use roxmltree::Document;

use super::js::{access, call_expr, global, js_test};
use super::model::{Call, Class, Func, Model, NamedUnion, TParam, Ty, base_args, map_types, mentions, model_with_options, subst, walk, with_self_args};
use super::names::{Kind, Naming};

#[derive(Clone, Debug)]
pub struct ProjectRef {
    pub package_name: String,
    pub dep_path: String,
    /// Non-generic class/interface phantom types (`Name(sub)`) exported in `<pkg>/types.gleam`.
    pub classes: HashSet<String>,
}

/// Loads a previously generated Gleam binding package (via `tsrs-meta.toml`)
/// so another generated package can import its class types directly.
pub fn load_project_ref(pkg_or_toml: &Path, from_dir: &Path) -> Result<ProjectRef, String> {
    let loaded = super::pkg_meta::load(pkg_or_toml, from_dir, "gleam")?;
    Ok(ProjectRef {
        package_name: loaded.meta.package,
        dep_path: loaded.rel_dir,
        classes: loaded.meta.types.into_iter().collect(),
    })
}

const KEYWORDS: &[&str] = &[
    "as", "assert", "auto", "case", "const", "delegate", "derive", "echo", "else", "fn", "if", "implement", "import", "let", "macro", "opaque", "panic", "pub", "test", "todo", "type", "use",
];

// Types the bindings declare whatever the API: what a module of Gleam can
// name of JavaScript's values. Not opaque types of the API.
const OWN: &[&str] = &["Promise", "JsError", "Dynamic", "Option", "List", "Result", "Nil", "Int", "Float", "String", "Bool", "BitArray", "Codec", "Ref"];

// Types (and variants) and values are apart; a type's functions, constants,
// upcasts and `to_list` are values named for it: `shape_area`.
pub const MODEL_OPTIONS: super::model::ModelOptions<'static> = super::model::ModelOptions {
    target: "Gleam",
    counted: false,
    generics: true,
    overloading: false,
    retain_opaque_constraints: true,
};

pub const NAMING: Naming = Naming {
    reserved: || KEYWORDS.iter().chain(OWN).map(|k| k.to_string()).collect(),
    // An external `Promise` is Gleam's own; a class the API declares as
    // `Option` is not, and gives way.
    type_names: |candidate, _, kind, _| match kind {
        Kind::Opaque if OWN.contains(&candidate) || candidate == "PromiseLike" => Vec::new(),
        // A class's marker for the phantom of its subtypes.
        Kind::Class => vec![upper(candidate), format!("{}Tag", upper(candidate))],
        _ => vec![upper(candidate)],
    },
    // Every type has a codec in the types module: `shape_codec`.
    values_of: |own, _, _, _| vec![lower(&format!("{}_codec", snake(own)))],
    types_apart: true,
    overloads: false,
    // A class's functions are in its own module: `shape.area`.
    func_name: |_, name, f| Some(lower(&function_name(f, name))),
    funcs_flat: false,
    nests: false,
    // Beside its functions there: its constants, list helpers and casts.
    members: |c, m| {
        let mut out: Vec<String> = c.consts.iter().map(|(k, _)| lower(k)).collect();
        out.push("upcast".into());
        if c.iterates.is_some() || c.elements.is_some() {
            out.push("to_list".into());
        }
        if c.elements.is_some() {
            out.extend(["get_at", "length"].map(String::from));
        }
        if c.js.is_some() {
            let has = |target: &str| c.funcs.iter().any(|f| lower(&function_name(f, &f.name)) == target);
            if parent(m, c).is_some() && !has("from") {
                out.push("from".into());
            }
            if !has("cast") {
                out.push("cast".into());
            }
        }
        out.extend(secondary(m, c).iter().map(|a| lower(&format!("as_{}", snake(&upper(a))))));
        out
    },
};

pub struct Output {
    pub name: String,
    pub files: Vec<(String, String)>,
    pub project_refs: Vec<(String, String)>,
    pub skipped: BTreeMap<String, usize>,
    // What each binding reaches, for a page to check (`wasm::manifest`).
    pub manifest: String,
    pub meta_toml: String,
}

impl Output {
    pub fn gleam_toml(&self) -> String {
        let deps: String = self
            .project_refs
            .iter()
            .map(|(name, path)| format!("{name} = {{ path = {:?} }}\n", path))
            .collect();
        format!(
            "name = {:?}\n\
             version = \"0.1.0\"\n\
             target = \"javascript\"\n\n\
             [dependencies]\n\
             gleam_stdlib = \">= 1.0.0 and < 2.0.0\"\n\
             {deps}",
            self.name
        )
    }
}

// `getElementById` -> `get_element_by_id`, `HTMLElement` -> `html_element`.
fn snake(name: &str) -> String {
    super::case::snake_case_alnum(name)
}

// A Gleam name for a value: snake_case, starting with a letter, not a keyword.
fn lower(name: &str) -> String {
    let s = snake(name).trim_matches('_').to_string();
    let s = if s.starts_with(|c: char| c.is_ascii_lowercase()) { s } else { format!("x{s}") };
    if KEYWORDS.contains(&s.as_str()) { format!("{s}_") } else { s }
}

// A Gleam name for a type or a variant: UpperCamelCase, letters and digits only.
fn upper(name: &str) -> String {
    let out = super::case::pascal_ascii_alnum(name);
    if out.starts_with(|c: char| c.is_ascii_uppercase()) { out } else { format!("X{out}") }
}

fn upper_variant(name: &str) -> String {
    super::case::screaming_to_pascal(name, upper)
}

struct Gen<'m> {
    m: &'m Model,
    pkg: String,
    // Gleam's name of each type the model names, and of each union's or enum's variants.
    types: HashMap<String, String>,
    variants: HashMap<(String, usize), String>,
    // Each callback, by the places of the type parameters it uses in Gleam, and those needing JS codecs.
    callbacks: HashMap<String, Vec<usize>>,
    callback_codecs: HashMap<String, Vec<usize>>,
    union_codecs: HashMap<String, Vec<usize>>,
    // Values given out, by module; and names no module's value may have.
    values: HashSet<(String, String)>,
    reserved_values: HashSet<String>,
    // The module being written: its Gleam, its JavaScript, and the Gleam
    // types its Gleam names (for its imports). Those written, by path.
    unit: String,
    out: String,
    ffi: String,
    used: RefCell<BTreeSet<String>>,
    units: BTreeMap<String, Unit>,
    // The module each Gleam type is declared in.
    home: HashMap<String, String>,
    // Each class's type parameter count; and the class its chain continues
    // from, with the arguments it gives it.
    classes: HashMap<String, usize>,
    parents: HashMap<String, (String, Vec<Ty>)>,
    // Class type parameters proven strictly class-bound across the entire model.
    class_bound: HashMap<(String, usize), (String, Vec<Ty>)>,
    // Where a class's type is open, `Element(a)`, any subtype (a function's
    // parameters): the variables given out. None where it is closed,
    // `Element(Nil)`, that type and no more (what comes back, what a callback
    // is given, what a list or variant holds).
    open: RefCell<Option<usize>>,
    // A function's own or class-bound type parameters bound to a class (`T extends Node`):
    // that class, open in the parameter's variable.
    bound: RefCell<HashMap<String, (String, Vec<Ty>)>>,
    // Function type parameters constrained to concrete `$same` types (primitives or non-inherited opaque types).
    fixed_params: RefCell<HashMap<String, Ty>>,
    // Function type parameters constrained to `ArrayOf(Class)` (`U extends ContainerChild[]`).
    bound_lists: RefCell<HashMap<String, (String, Vec<Ty>)>>,
    // Recursion guard while expanding default type arguments in `class_type`.
    expanding_defaults: RefCell<HashSet<String>>,
    skipped: BTreeMap<String, usize>,
}

#[derive(Default)]
struct Unit {
    out: String,
    ffi: String,
    used: BTreeSet<String>,
}

impl<'m> Gen<'m> {
    fn skip(&mut self, why: &str) {
        *self.skipped.entry(why.to_string()).or_default() += 1;
    }

    // A module-level value's name: `get_size` in a class's module.
    // `names::settle` made them apart; one given out twice in a module is a
    // mistake in its naming.
    fn value_name(&mut self, base: &str) -> String {
        let name = lower(base);
        assert!(!self.reserved_values.contains(&name) && self.values.insert((self.unit.clone(), name.clone())), "Gleam: the value `{name}` is named twice in {}", self.unit);
        name
    }

    // A value's name in the module being written, the first of `base`,
    // `base_2`, ... it has not given out, and no keyword.
    fn fresh_value(&mut self, base: &str) -> String {
        let mut base = lower(base);
        if KEYWORDS.contains(&base.as_str()) {
            base.push('_');
        }
        let mut name = base.clone();
        let mut i = 2;
        while self.reserved_values.contains(&name) || self.values.contains(&(self.unit.clone(), name.clone())) {
            name = format!("{base}_{i}");
            i += 1;
        }
        self.values.insert((self.unit.clone(), name.clone()));
        name
    }

    fn tname(&self, n: &str) -> String {
        self.types.get(n).cloned().unwrap_or_else(|| upper(n))
    }

    // A type's name, written in the module's Gleam: noted, for its import.
    fn gt(&self, n: &str) -> String {
        let t = self.tname(n);
        self.note(&t);
        t
    }

    fn note(&self, gleam_type: &str) {
        self.used.borrow_mut().insert(gleam_type.to_string());
    }

    // Writes to another module from here on.
    fn switch(&mut self, unit: &str) {
        let done = Unit { out: std::mem::take(&mut self.out), ffi: std::mem::take(&mut self.ffi), used: self.used.take() };
        let was = self.units.entry(std::mem::replace(&mut self.unit, unit.to_string())).or_default();
        was.out += &done.out;
        was.ffi += &done.ffi;
        was.used.extend(done.used);
        let next = self.units.remove(unit).unwrap_or_default();
        self.out = next.out;
        self.ffi = next.ffi;
        *self.used.borrow_mut() = next.used;
    }

    // The module's own JavaScript, as an external names it.
    fn own_ffi(&self) -> String {
        format!("./{}_ffi.mjs", self.unit.rsplit('/').next().unwrap_or_default())
    }

    // The runtime's JavaScript, from the module being written.
    fn runtime_ffi(&self) -> String {
        relative(&self.unit, &format!("{}/runtime_ffi.mjs", self.pkg))
    }

    // A generic type named without its arguments: `Polygon(Dynamic)`.
    fn bare(&self, n: &str) -> String {
        if self.classes.contains_key(n) {
            return self.class_type(n, &[]);
        }
        let arity = self.m.classes.iter().find(|c| c.name == n).map(|c| c.type_params.len())
            .or_else(|| self.m.opaque_arity.get(n).copied())
            .or_else(|| self.m.unions.iter().find(|u| u.name == n).map(|u| u.type_params.len()))
            .or_else(|| self.callbacks.get(n).map(Vec::len))
            .unwrap_or(0);
        let name = self.gt(n);
        if arity == 0 {
            name
        } else {
            self.note("Dynamic");
            format!("{name}({})", vec!["Dynamic"; arity].join(", "))
        }
    }

    fn tvar(n: &str) -> String {
        lower(n)
    }

    // A class type's last argument: a fresh variable where open, else `Nil`.
    fn phantom(&self) -> String {
        let mut open = self.open.borrow_mut();
        match open.as_mut() {
            Some(k) => {
                *k += 1;
                format!("sub_{k}")
            }
            None => "Nil".into(),
        }
    }

    // `f`, with class types closed.
    fn closed<T>(&self, f: impl FnOnce() -> T) -> T {
        let was = self.open.replace(None);
        let out = f();
        *self.open.borrow_mut() = was;
        out
    }

    // A class's type with its arguments (using parameter defaults, or `Dynamic`
    // when unavailable/cyclic) and its last, phantom one.
    fn class_type(&self, n: &str, args: &[Ty]) -> String {
        let count = self.classes[n];
        let cls = self.m.classes.iter().find(|c| c.name == n);
        let mut parts: Vec<String> = Vec::with_capacity(count + 1);
        let mut resolved: Vec<Ty> = Vec::with_capacity(count);
        for i in 0..count {
            if let Some(a) = args.get(i) {
                resolved.push(a.clone());
                parts.push(self.closed(|| self.ty(a)));
                continue;
            }
            if let Some(c) = cls
                && let Some(tp) = c.type_params.get(i)
                && let Some(def) = &tp.default
                && !self.expanding_defaults.borrow().contains(n)
            {
                let def_sub = subst(def, &c.type_params[..i], &resolved);
                let unresolved = c.type_params[i..].iter().any(|rem| mentions(&def_sub, &rem.name));
                if !unresolved {
                    self.expanding_defaults.borrow_mut().insert(n.to_string());
                    let s = self.closed(|| self.ty(&def_sub));
                    self.expanding_defaults.borrow_mut().remove(n);
                    resolved.push(def_sub);
                    parts.push(s);
                    continue;
                }
            }
            self.note("Dynamic");
            resolved.push(Ty::Any);
            parts.push("Dynamic".to_string());
        }
        parts.push(self.phantom());
        format!("{}({})", self.gt(n), parts.join(", "))
    }

    fn decodable(&self, n: &str) -> bool {
        self.m.unions.iter().any(|u| u.name == n && u.decodable())
    }

    fn is_callback(&self, n: &str) -> bool {
        self.callbacks.contains_key(n)
    }

    // Whether Gleam's type of a value names a type parameter.
    fn shows(&self, t: &Ty, param: &str) -> bool {
        match t {
            Ty::Param(n) => n == param,
            Ty::Nullable(inner, _) | Ty::ArrayOf(inner) => self.shows(inner, param),
            Ty::Tuple(es) => es.iter().any(|e| self.shows(e, param)),
            Ty::Generic(n, _) if self.m.unions.iter().any(|u| u.name == *n) && !self.decodable(n) => false,
            Ty::Generic(n, args) if self.callbacks.contains_key(n) => self.callbacks[n].iter().filter_map(|&i| args.get(i)).any(|a| self.shows(a, param)),
            Ty::Generic(_, args) => args.iter().any(|a| self.shows(a, param)),
            _ => false,
        }
    }

    fn bound_class_ty(&self, class: &str, args: &[Ty], param: &str) -> String {
        let tvar = Self::tvar(param);
        let count = self.classes[class];
        let mut parts: Vec<String> = (0..count)
            .map(|i| match args.get(i) {
                Some(a) if !matches!(a, Ty::Any) => self.closed(|| self.ty(a)),
                _ => format!("{tvar}_a{i}"),
            })
            .collect();
        parts.push(tvar);
        format!("{}({})", self.gt(class), parts.join(", "))
    }

    // Gleam's type of a value.
    fn ty(&self, t: &Ty) -> String {
        let dynamic = || {
            self.note("Dynamic");
            "Dynamic".to_string()
        };
        match t {
            Ty::Void => "Nil".into(),
            Ty::Num => "Float".into(),
            Ty::Bool => "Bool".into(),
            Ty::Str => "String".into(),
            Ty::Any | Ty::Union(_) | Ty::Key(_) | Ty::Mapped(_) | Ty::Listener(_) => dynamic(),
            Ty::Array => format!("List({})", dynamic()),
            Ty::ArrayOf(e) => format!("List({})", self.ty(e)),
            Ty::Tuple(es) => format!("#({})", es.iter().map(|e| self.ty(e)).collect::<Vec<_>>().join(", ")),
            // A promise of what the model does not say.
            Ty::Obj(n) if n == "Promise" || n == "PromiseLike" => {
                self.note("Promise");
                format!("Promise({})", dynamic())
            }
            Ty::Obj(n) => self.bare(n),
            Ty::Enum(n) | Ty::NumEnum(n) => self.gt(n),
            Ty::Nullable(inner, _) => {
                self.note("Option");
                format!("Option({})", self.ty(inner))
            }
            Ty::NamedUnion(n) if self.decodable(n) => self.bare(n),
            Ty::NamedUnion(_) => dynamic(),
            Ty::Param(n) => {
                if let Some((class, args)) = self.bound.borrow().get(n).cloned() {
                    return self.bound_class_ty(&class, &args, n);
                }
                if let Some(fixed_ty) = self.fixed_params.borrow().get(n).cloned() {
                    return self.ty(&fixed_ty);
                }
                if let Some((class, args)) = self.bound_lists.borrow().get(n).cloned() {
                    return format!("List({})", self.bound_class_ty(&class, &args, n));
                }
                Self::tvar(n)
            }
            Ty::Generic(n, args) if self.classes.contains_key(n) => self.class_type(n, args),
            Ty::Generic(n, _) if self.m.unions.iter().any(|u| u.name == *n) && !self.decodable(n) => dynamic(),
            // A callback takes the type arguments it uses.
            Ty::Generic(n, args) if self.callbacks.contains_key(n) => {
                let args: Vec<String> = self.closed(|| self.callbacks[n].iter().filter_map(|&i| args.get(i)).map(|a| self.ty(a)).collect());
                if args.is_empty() { self.gt(n) } else { format!("{}({})", self.gt(n), args.join(", ")) }
            }
            Ty::Generic(n, args) if args.is_empty() => self.bare(n),
            Ty::Generic(n, args) => {
                let args = self.closed(|| args.iter().map(|a| self.ty(a)).collect::<Vec<_>>());
                format!("{}({})", self.gt(n), args.join(", "))
            }
        }
    }

    // A JavaScript expression that is `x`, JavaScript's value, as Gleam has it.
    fn to_gleam(&self, t: &Ty, x: &str) -> String {
        match t {
            // A type parameter's values cross by the codec the function was given,
            // except when bound to a class, concrete `$same` type, or class list.
            Ty::Param(n) if self.bound.borrow().contains_key(n) || self.fixed_params.borrow().contains_key(n) => x.to_string(),
            Ty::Param(n) if self.bound_lists.borrow().contains_key(n) => format!("toList(Array.from({x} ?? []))"),
            Ty::Param(n) => format!("{}.to({x})", codec_var(n)),
            Ty::Void => format!("({x}, undefined)"),
            Ty::Nullable(inner, _) => format!("((v) => v == null ? new None() : new Some({}))({x})", self.to_gleam(inner, "v")),
            Ty::Array => format!("toList(Array.from({x} ?? []))"),
            Ty::ArrayOf(e) => format!("toList(Array.from({x} ?? [], (v) => {}))", self.to_gleam(e, "v")),
            Ty::Tuple(es) => format!("((v) => [{}])({x})", es.iter().enumerate().map(|(i, e)| self.to_gleam(e, &format!("v[{i}]"))).collect::<Vec<_>>().join(", ")),
            Ty::Enum(n) | Ty::NumEnum(n) => format!("$T.{}_to({x})", self.tname(n)),
            Ty::NamedUnion(n) if self.decodable(n) => format!("$T.{}_to({x})", self.tname(n)),
            Ty::Generic(n, args) if self.decodable(n) && !args.is_empty() => format!("$T.{}_to({x}{})", self.tname(n), self.codec_args(n, args)),
            Ty::Obj(n) | Ty::Generic(n, _) if self.is_callback(n) => format!("$T.{}_to({x}{})", self.tname(n), self.codec_args(n, generic_args(t))),
            Ty::Generic(n, args) if (n == "Promise" || n == "PromiseLike") && args.len() == 1 => {
                format!("Promise.resolve({x}).then((v) => {})", self.to_gleam(&args[0], "v"))
            }
            _ => x.to_string(),
        }
    }

    // The codecs a generic union's or callback's conversions take, for the
    // type parameters they use: `, $same, $c_T`.
    fn codec_args(&self, n: &str, args: &[Ty]) -> String {
        let used: &[usize] = self
            .callback_codecs
            .get(n)
            .or_else(|| self.union_codecs.get(n))
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        // Named without its arguments, it is of `Dynamic`: values as they are.
        used.iter().map(|&i| format!(", {}", args.get(i).map_or("$same".to_string(), |a| self.codec_expr(a)))).collect()
    }

    // A JavaScript codec for values of `t`: how they cross each way.
    fn codec_expr(&self, t: &Ty) -> String {
        match t {
            Ty::Param(n) if self.bound.borrow().contains_key(n) || self.fixed_params.borrow().contains_key(n) => "$same".into(),
            Ty::Param(n) if self.bound_lists.borrow().contains_key(n) => "{ to: (v) => toList(Array.from(v ?? [])), from: (v) => v.toArray() }".into(),
            Ty::Param(n) => codec_var(n),
            t if self.to_gleam(t, "v") == "v" && self.from_gleam(t, "v") == "v" => "$same".into(),
            t => format!("{{ to: (v) => {}, from: (v) => {} }}", self.to_gleam(t, "v"), self.from_gleam(t, "v")),
        }
    }

    // Whether a type parameter's values cross where `t` is: bare, in an
    // option, list, tuple or promise, through a callback or a union. Not
    // inside a handle (`Box(t)`), which crosses as it is.
    fn crosses(&self, t: &Ty, p: &str) -> bool {
        if self.bound.borrow().contains_key(p) || self.fixed_params.borrow().contains_key(p) || self.bound_lists.borrow().contains_key(p) {
            return false;
        }
        match t {
            Ty::Param(n) => n == p,
            Ty::Nullable(inner, _) | Ty::ArrayOf(inner) => self.crosses(inner, p),
            Ty::Tuple(es) => es.iter().any(|e| self.crosses(e, p)),
            Ty::Generic(n, args) if n == "Promise" || n == "PromiseLike" => args.first().is_some_and(|a| self.crosses(a, p)),
            Ty::Generic(n, args) if self.is_callback(n) => self.callback_codecs.get(n).is_some_and(|used| used.iter().filter_map(|&i| args.get(i)).any(|a| self.crosses(a, p))),
            Ty::Generic(n, args) if self.decodable(n) => self.union_codecs.get(n).is_some_and(|used| used.iter().filter_map(|&i| args.get(i)).any(|a| self.crosses(a, p))),
            _ => false,
        }
    }

    // A JavaScript expression that is `x`, Gleam's value, as JavaScript has it.
    #[allow(clippy::wrong_self_convention)]
    fn from_gleam(&self, t: &Ty, x: &str) -> String {
        match t {
            Ty::Param(n) if self.bound.borrow().contains_key(n) || self.fixed_params.borrow().contains_key(n) => x.to_string(),
            Ty::Param(n) if self.bound_lists.borrow().contains_key(n) => format!("{x}.toArray()"),
            Ty::Param(n) => format!("{}.from({x})", codec_var(n)),
            Ty::Nullable(inner, undefined) => format!("((v) => v instanceof Some ? {} : {})({x})", self.from_gleam(inner, "v[0]"), if *undefined { "undefined" } else { "null" }),
            Ty::Array => format!("{x}.toArray()"),
            Ty::ArrayOf(e) => format!("{x}.toArray().map((v) => {})", self.from_gleam(e, "v")),
            Ty::Tuple(es) => format!("((v) => [{}])({x})", es.iter().enumerate().map(|(i, e)| self.from_gleam(e, &format!("v[{i}]"))).collect::<Vec<_>>().join(", ")),
            Ty::Enum(n) | Ty::NumEnum(n) => format!("$T.{}_from({x})", self.tname(n)),
            Ty::NamedUnion(n) if self.decodable(n) => format!("$T.{}_from({x})", self.tname(n)),
            Ty::Generic(n, args) if self.decodable(n) && !args.is_empty() => format!("$T.{}_from({x}{})", self.tname(n), self.codec_args(n, args)),
            Ty::Obj(n) | Ty::Generic(n, _) if self.is_callback(n) => format!("$T.{}_from({x}{})", self.tname(n), self.codec_args(n, generic_args(t))),
            Ty::Generic(n, args) if (n == "Promise" || n == "PromiseLike") && args.len() == 1 => {
                format!("Promise.resolve({x}).then((v) => {})", self.from_gleam(&args[0], "v"))
            }
            _ => x.to_string(),
        }
    }

    fn type_params(params: &[TParam]) -> String {
        if params.is_empty() { String::new() } else { format!("({})", params.iter().map(|p| Self::tvar(&p.name)).collect::<Vec<_>>().join(", ")) }
    }

    fn is_fixed_opaque(&self, o: &str) -> bool {
        !self.classes.contains_key(o)
            && self.m.opaque.iter().any(|x| x == o)
            && !OWN.contains(&o)
            && o != "PromiseLike"
            && self.m.opaque_arity.get(o).copied().unwrap_or(0) == 0
    }

    fn clear_fn_bounds(&self) {
        self.bound.borrow_mut().clear();
        self.fixed_params.borrow_mut().clear();
        self.bound_lists.borrow_mut().clear();
    }

    fn setup_fn_bounds(&self, f: &Func, owner: Option<(&str, &[TParam])>) {
        self.clear_fn_bounds();
        let has_self = Class::has_self(f);
        let class_param_names: HashSet<&str> = owner.map_or_else(HashSet::new, |(_, tps)| tps.iter().map(|p| p.name.as_str()).collect());
        if let Some((owner_name, class_params)) = owner {
            for (i, cp) in class_params.iter().enumerate() {
                if !f.type_params.iter().any(|fp| fp.name == cp.name)
                    && let Some(binding) = self.class_bound.get(&(owner_name.to_string(), i))
                {
                    self.bound.borrow_mut().insert(cp.name.clone(), binding.clone());
                }
            }
        }
        for p in &f.type_params {
            if has_self && class_param_names.contains(p.name.as_str()) {
                continue;
            }
            match &p.constraint {
                Some(Ty::Obj(c)) if self.classes.contains_key(c) => {
                    self.bound.borrow_mut().insert(p.name.clone(), (c.clone(), Vec::new()));
                }
                Some(Ty::Generic(c, args)) if self.classes.contains_key(c) => {
                    self.bound.borrow_mut().insert(p.name.clone(), (c.clone(), args.clone()));
                }
                Some(Ty::Str) => {
                    self.fixed_params.borrow_mut().insert(p.name.clone(), Ty::Str);
                }
                Some(Ty::Num) => {
                    self.fixed_params.borrow_mut().insert(p.name.clone(), Ty::Num);
                }
                Some(Ty::Bool) => {
                    self.fixed_params.borrow_mut().insert(p.name.clone(), Ty::Bool);
                }
                Some(Ty::Obj(o)) if self.is_fixed_opaque(o) => {
                    self.fixed_params.borrow_mut().insert(p.name.clone(), Ty::Obj(o.clone()));
                }
                Some(Ty::ArrayOf(inner)) => {
                    let elem = match &**inner {
                        Ty::Obj(c) if self.classes.contains_key(c) => Some((c.clone(), Vec::new())),
                        Ty::Generic(c, args) if self.classes.contains_key(c) => Some((c.clone(), args.clone())),
                        Ty::Param(cp_name) => owner.and_then(|(_, tps)| {
                            tps.iter().find(|tp| tp.name == *cp_name).and_then(|tp| match &tp.constraint {
                                Some(Ty::Obj(c)) if self.classes.contains_key(c) => Some((c.clone(), Vec::new())),
                                Some(Ty::Generic(c, args)) if self.classes.contains_key(c) => Some((c.clone(), args.clone())),
                                _ => None,
                            })
                        }),
                        _ => None,
                    };
                    if let Some(binding) = elem {
                        self.bound_lists.borrow_mut().insert(p.name.clone(), binding);
                    }
                }
                _ => {}
            }
        }
    }

    fn crossing_params(&self, f: &Func, owner: Option<(&str, &[TParam])>) -> Vec<TParam> {
        let class_params: &[TParam] = owner.map_or(&[], |o| o.1);
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        for p in class_params.iter().chain(&f.type_params).filter(|p| seen.insert(p.name.clone())) {
            if f.params.iter().map(|x| &x.1).chain([&f.ret]).any(|t| self.crosses(t, &p.name)) {
                out.push(p.clone());
            }
        }
        out
    }

    // One external function, `name`, and its JavaScript.
    fn function(&mut self, name: &str, f: &Func, owner: Option<(&str, &[TParam])>) {
        self.setup_fn_bounds(f, owner);
        let mut params = Vec::new();
        let mut js_params = Vec::new();
        // A class's type is open in the parameters: any subtype.
        *self.open.borrow_mut() = Some(0);
        if let Some((class, tps)) = owner.filter(|_| Class::has_self(f)) {
            let own: Vec<Ty> = tps.iter().map(|p| Ty::Param(p.name.clone())).collect();
            params.push(format!("self: {}", self.class_type(class, &own)));
            js_params.push("self".to_string());
        }
        let mut taken: HashSet<String> = HashSet::from(["self".to_string()]);
        let mut args = Vec::new();
        for (i, (p, t)) in f.params.iter().enumerate() {
            // A parameter of the FFI's JavaScript too: `default_` there.
            let mut id = lower(p);
            while JS_RESERVED.contains(&id.as_str()) || !taken.insert(id.clone()) {
                id.push('_');
            }
            let last_rest = f.rest && i == f.params.len() - 1;
            let param_ty = self.ty(t);
            params.push(format!("{id}: {param_ty}"));
            js_params.push(id.clone());
            let value = self.from_gleam(t, &id);
            args.push(if last_rest { format!("...({value} ?? [])") } else { value });
        }
        // A codec for each type parameter whose values cross: the class's, then the function's own.
        *self.open.borrow_mut() = None;
        for p in self.crossing_params(f, owner) {
            self.note("Codec");
            params.push(format!("{}_codec: Codec({})", lower(&p.name), Self::tvar(&p.name)));
            js_params.push(codec_var(&p.name));
        }
        for (position, literal) in &f.fixed {
            args.insert((*position).min(args.len()), literal.clone());
        }
        let call = call_expr(&f.call, "self", &args);
        let body = match &f.ret {
            Ty::Void => format!("{call};\n  return undefined;"),
            t => format!("return {};", self.to_gleam(t, &call)),
        };
        let ret = self.ty(&f.ret);
        // A name JavaScript reserves (`new`, `delete`) is `new_` there.
        let js = if JS_RESERVED.contains(&name) { format!("{name}_") } else { name.to_string() };
        self.out += &format!("@external(javascript, \"{}\", \"{js}\")\npub fn {name}({}) -> {ret}\n\n", self.own_ffi(), params.join(", "));
        self.ffi += &format!("export function {js}({}) {{\n  {body}\n}}\n", js_params.join(", "));
        self.clear_fn_bounds();
    }

    // A union as a custom type, and its conversions: told apart by its tests.
    fn union(&mut self, u: &NamedUnion) {
        let name = self.tname(&u.name);
        let params = Self::type_params(&u.type_params);
        let variants: Vec<String> = (0..u.parts.len()).map(|i| self.variants[&(u.name.clone(), i)].clone()).collect();
        let cases: Vec<String> = u.parts.iter().zip(&variants).map(|(p, v)| format!("  {v}({})", self.ty(p))).collect();
        self.out += &format!("pub type {name}{params} {{\n{}\n}}\n\n", cases.join("\n"));
        let mut to = String::new();
        let mut fallback = None;
        for ((p, test), v) in u.parts.iter().zip(&u.tests).zip(&variants) {
            match test {
                Some(t) => to += &format!("  if ({}) return new B.{v}({});\n", js_test(t, u.discriminant.as_deref(), &self.m.js_refs), self.to_gleam(p, "v")),
                None => fallback = Some(format!("  return new B.{v}({});\n", self.to_gleam(p, "v"))),
            }
        }
        to += &fallback.unwrap_or_else(|| format!("  throw new TypeError(\"not a {name}: \" + v);\n"));
        let from: String = u.parts.iter().zip(&variants).map(|(p, v)| format!("  if (u instanceof B.{v}) return {};\n", self.from_gleam(p, "u[0]"))).collect();
        let codecs: String = self.union_codecs[&u.name].iter().map(|&i| format!(", {}", codec_var(&u.type_params[i].name))).collect();
        self.ffi += &format!("export function {name}_to(v{codecs}) {{\n{to}}}\nexport function {name}_from(u{codecs}) {{\n{from}}}\n");
    }

    fn emit_specializations(&mut self, c: &'m Class) {
        let mut chain = Vec::new();
        let mut at = c.name.clone();
        while let Some((p, _)) = self.parents.get(&at) {
            chain.push(p.clone());
            at = p.clone();
        }
        let mut planned = HashSet::new();
        let mut specs: Vec<(String, Func)> = Vec::new();
        for anc_name in chain {
            let Some(anc) = self.m.classes.iter().find(|k| k.name == anc_name) else { continue };
            if anc.type_params.is_empty() {
                continue;
            }
            let Some(sub_args) = base_args(self.m, c, &anc.name) else { continue };
            for f in anc.funcs.iter().filter(|f| Class::has_self(f)) {
                let f_anc = with_self_args(f, anc);
                let id = lower(&function_name(&f_anc, &f_anc.name));
                if self.reserved_values.contains(&id) || self.values.contains(&(self.unit.clone(), id.clone())) || planned.contains(&id) {
                    continue;
                }
                if f_anc.type_params.iter().any(|ftp| c.type_params.iter().any(|ctp| ctp.name == ftp.name)) {
                    continue;
                }
                self.setup_fn_bounds(&f_anc, Some((&anc.name, &anc.type_params)));
                let orig_codecs = self.crossing_params(&f_anc, Some((&anc.name, &anc.type_params))).len();
                self.clear_fn_bounds();
                let spec_f = map_types(&f_anc, &|t| subst(t, &anc.type_params, &sub_args));
                self.setup_fn_bounds(&spec_f, Some((&c.name, &c.type_params)));
                let spec_codecs = self.crossing_params(&spec_f, Some((&c.name, &c.type_params))).len();
                self.clear_fn_bounds();
                if spec_codecs < orig_codecs {
                    planned.insert(id.clone());
                    specs.push((id, spec_f));
                }
            }
        }
        for (id, spec_f) in specs {
            let id = self.value_name(&id);
            self.function(&id, &spec_f, Some((&c.name, &c.type_params)));
        }
    }

    fn emit_with_builders(&mut self, c: &'m Class) {
        if !c.funcs.iter().any(|f| matches!(f.call, Call::Create(_))) {
            return;
        }
        for orig_f in &c.funcs {
            if !Class::has_self(orig_f) {
                continue;
            }
            let f = with_self_args(orig_f, c);
            match &f.call {
                Call::Set(js) if f.params.len() == 1 => {
                    let fname = lower(&function_name(&f, &f.name));
                    let Some(rest) = fname.strip_prefix("set_") else { continue };
                    let with_id = lower(&format!("with_{rest}"));
                    if self.reserved_values.contains(&with_id) || self.values.contains(&(self.unit.clone(), with_id.clone())) {
                        continue;
                    }
                    let val_ty = match &f.params[0].1 {
                        Ty::Nullable(inner, _) => (**inner).clone(),
                        t => t.clone(),
                    };
                    let mut wf = f.clone();
                    wf.params = vec![(f.params[0].0.clone(), val_ty.clone())];
                    wf.ret = Ty::Void;
                    self.setup_fn_bounds(&wf, Some((&c.name, &c.type_params)));
                    *self.open.borrow_mut() = Some(0);
                    let own: Vec<Ty> = c.type_params.iter().map(|p| Ty::Param(p.name.clone())).collect();
                    let self_ty = self.class_type(&c.name, &own);
                    let mut val_param = lower(&wf.params[0].0);
                    while JS_RESERVED.contains(&val_param.as_str()) || val_param == "self" {
                        val_param.push('_');
                    }
                    let val_ty_str = self.ty(&val_ty);
                    let val_js = self.from_gleam(&val_ty, &val_param);
                    *self.open.borrow_mut() = None;
                    let mut params = vec![format!("self: {self_ty}"), format!("{val_param}: {val_ty_str}")];
                    let mut js_params = vec!["self".to_string(), val_param.clone()];
                    for p in self.crossing_params(&wf, Some((&c.name, &c.type_params))) {
                        self.note("Codec");
                        params.push(format!("{}_codec: Codec({})", lower(&p.name), Self::tvar(&p.name)));
                        js_params.push(codec_var(&p.name));
                    }
                    let js_fn = if JS_RESERVED.contains(&with_id.as_str()) { format!("{with_id}_") } else { with_id.clone() };
                    let with_id = self.value_name(&with_id);
                    self.out += &format!("@external(javascript, \"{}\", \"{js_fn}\")\npub fn {with_id}({}) -> {self_ty}\n\n", self.own_ffi(), params.join(", "));
                    self.ffi += &format!("export function {js_fn}({}) {{\n  self{} = {val_js};\n  return self;\n}}\n", js_params.join(", "), access(js));
                    self.clear_fn_bounds();
                }
                Call::IndexSet if f.params.len() == 2 => {
                    let with_id = "with_at".to_string();
                    if self.reserved_values.contains(&with_id) || self.values.contains(&(self.unit.clone(), with_id.clone())) {
                        continue;
                    }
                    let mut wf = f.clone();
                    wf.ret = Ty::Void;
                    self.setup_fn_bounds(&wf, Some((&c.name, &c.type_params)));
                    *self.open.borrow_mut() = Some(0);
                    let own: Vec<Ty> = c.type_params.iter().map(|p| Ty::Param(p.name.clone())).collect();
                    let self_ty = self.class_type(&c.name, &own);
                    let mut taken: HashSet<String> = HashSet::from(["self".to_string()]);
                    let mut params = vec![format!("self: {self_ty}")];
                    let mut js_params = vec!["self".to_string()];
                    let mut arg_js = Vec::new();
                    for (p, t) in &wf.params {
                        let mut pid = lower(p);
                        while JS_RESERVED.contains(&pid.as_str()) || !taken.insert(pid.clone()) {
                            pid.push('_');
                        }
                        params.push(format!("{pid}: {}", self.ty(t)));
                        js_params.push(pid.clone());
                        arg_js.push(self.from_gleam(t, &pid));
                    }
                    *self.open.borrow_mut() = None;
                    for p in self.crossing_params(&wf, Some((&c.name, &c.type_params))) {
                        self.note("Codec");
                        params.push(format!("{}_codec: Codec({})", lower(&p.name), Self::tvar(&p.name)));
                        js_params.push(codec_var(&p.name));
                    }
                    let with_id = self.value_name(&with_id);
                    self.out += &format!("@external(javascript, \"{}\", \"{with_id}\")\npub fn {with_id}({}) -> {self_ty}\n\n", self.own_ffi(), params.join(", "));
                    self.ffi += &format!("export function {with_id}({}) {{\n  self[{}] = {};\n  return self;\n}}\n", js_params.join(", "), arg_js[0], arg_js[1]);
                    self.clear_fn_bounds();
                }
                _ => {}
            }
        }
    }
}

// The bindings of the XML, or what Gleam cannot express of it.
pub fn module(xml: &str) -> Result<Output, Vec<String>> {
    module_named(xml, "bindings", &[])
}

pub fn module_named(xml: &str, package_name: &str, refs: &[ProjectRef]) -> Result<Output, Vec<String>> {
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
    super::names::settle(&mut m, &NAMING);
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
        units: BTreeMap::new(),
        home: HashMap::new(),
        classes: m.classes.iter().map(|c| (c.name.clone(), c.type_params.len())).collect(),
        parents: m.classes.iter().filter_map(|c| parent(&m, c).map(|p| (c.name.clone(), p))).collect(),
        class_bound: HashMap::new(),
        open: RefCell::new(None),
        bound: RefCell::default(),
        fixed_params: RefCell::default(),
        bound_lists: RefCell::default(),
        expanding_defaults: RefCell::default(),
        skipped: m.skipped.clone(),
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
    for o in m.opaque.iter().filter(|o| !OWN.contains(&o.as_str()) && *o != "PromiseLike") {
        name_type(o, &mut g);
    }
    g.types.insert("PromiseLike".into(), "Promise".into());
    // A variant by its own name where that is free, else its type's and its own.
    let mut variant_names: Vec<(String, usize, String)> = Vec::new();
    for u in m.unions.iter().filter(|u| u.decodable()) {
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
    ] {
        g.values.insert((runtime_mod.clone(), reserved.into()));
    }

    // Each area's modules: `areas/main` is `bindings/areas/main` (its
    // functions) and `bindings/types/areas_main` (its classes).
    let mut resolved_opaque: HashMap<String, usize> = HashMap::new();
    let mut ref_aliases: HashMap<String, (String, String)> = HashMap::new();
    for o in m.opaque.iter().filter(|o| !OWN.contains(&o.as_str()) && *o != "PromiseLike") {
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
        let segments: Vec<String> = area.split('/').filter(|s| !s.is_empty()).map(lower).collect();
        let mut path = format!("{pkg}/{}", segments.join("/"));
        let mut n = 2;
        while !taken.insert(path.clone()) {
            path = format!("{pkg}/{}_{n}", segments.join("/"));
            n += 1;
        }
        paths.insert(area.clone(), path);
    }
    // Each class's module, in its area's: `bindings/areas/main/shape`.
    let mut class_paths: HashMap<String, String> = HashMap::new();
    for c in &m.classes {
        // A namespace's class by its own name: `ns.Node`'s is `namespaces/ns/node`.
        let own = m.namespaced.get(&c.name).map(|p| {
            let depth = m.namespace_modules.get(&c.name).map_or(1, |(_, d)| *d).min(p.len() - 1);
            p[depth..].iter().map(|s| snake(&upper(s))).collect::<Vec<_>>().join("/")
        });
        let mut segment = own.unwrap_or_else(|| snake(&g.tname(&c.name)));
        if KEYWORDS.contains(&segment.as_str()) {
            segment.push('_');
        }
        // Its namespace's module, a package's in the package's (`areas/pb/util`).
        let module = m.namespace_modules.get(&c.name).map_or(&c.module, |(m, _)| m);
        let base = format!("{}/{segment}", module_path(&paths, pkg, module));
        let mut path = base.clone();
        let mut n = 2;
        while !taken.insert(path.clone()) {
            path = format!("{base}_{n}");
            n += 1;
        }
        class_paths.insert(c.name.clone(), path);
    }
    for o in OWN_RUNTIME {
        g.home.insert(o.to_string(), runtime_mod.clone());
    }
    for c in &m.classes {
        g.home.insert(g.tname(&c.name), types_mod.clone());
        g.home.insert(format!("{}Tag", g.tname(&c.name)), types_mod.clone());
    }
    let others = m.enums.iter().map(|e| &e.0).chain(m.num_enums.iter().map(|e| &e.0)).chain(m.callbacks.iter().map(|c| &c.name)).chain(m.unions.iter().map(|u| &u.name)).chain(&m.opaque);
    for n in others.filter(|n| !OWN.contains(&n.as_str()) && *n != "PromiseLike" && !resolved_opaque.contains_key(*n)) {
        g.home.insert(g.tname(n), types_mod.clone());
    }

    // The runtime: what every module may use.
    g.out += include_str!("runtime/gleam_runtime.gleam");
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
        include_str!("runtime/gleam_runtime_ffi.mjs")
    );

    // The types that may refer to each other: externals, enums, unions, callbacks.
    g.switch(&types_mod);
    for o in m.opaque.iter().filter(|o| !OWN.contains(&o.as_str()) && *o != "PromiseLike" && !resolved_opaque.contains_key(*o)) {
        let arity = m.opaque_arity.get(o).copied().unwrap_or(0);
        let params = if arity == 0 { String::new() } else { format!("({})", (1..=arity).map(|i| format!("t{i}")).collect::<Vec<_>>().join(", ")) };
        g.out += &format!("pub type {}{params}\n\n", g.tname(o));
    }
    // String and numeric enums: custom types without fields.
    for (e, values) in &m.enums {
        let name = g.tname(e);
        let variants: Vec<String> = (0..values.len()).map(|i| g.variants[&(e.clone(), i)].clone()).collect();
        g.out += &format!("pub type {name} {{\n{}\n}}\n\n", variants.iter().map(|v| format!("  {v}")).collect::<Vec<_>>().join("\n"));
        g.ffi += &format!(
            "const {name}_values = {:?};\nconst {name}_cases = () => [{}];\n\
             export function {name}_to(v) {{\n  const i = {name}_values.indexOf(v);\n  if (i < 0) throw new TypeError(\"not a {name}: \" + v);\n  return new ({name}_cases()[i])();\n}}\n\
             export function {name}_from(v) {{\n  return {name}_values[{name}_cases().findIndex((c) => v instanceof c)];\n}}\n",
            values,
            variants.iter().map(|v| format!("B.{v}")).collect::<Vec<_>>().join(", ")
        );
    }
    for (e, members) in &m.num_enums {
        let name = g.tname(e);
        let variants: Vec<String> = (0..members.len()).map(|i| g.variants[&(e.clone(), i)].clone()).collect();
        g.out += &format!("pub type {name} {{\n{}\n}}\n\n", variants.iter().map(|v| format!("  {v}")).collect::<Vec<_>>().join("\n"));
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
    for u in m.unions.iter().filter(|u| u.decodable()) {
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
    // A class's functions, in its own module.
    for c in &m.classes {
        g.switch(&class_paths[&c.name]);
        let own: Vec<Ty> = c.type_params.iter().map(|p| Ty::Param(p.name.clone())).collect();
        // The class's type, open (any subtype) or closed.
        let open = |g: &Gen| {
            *g.open.borrow_mut() = Some(0);
            let t = g.class_type(&c.name, &own);
            *g.open.borrow_mut() = None;
            t
        };
        for (constant, value) in &c.consts {
            let gleam = if value.starts_with('"') {
                value.clone()
            } else if value == "true" || value == "false" {
                upper(value)
            } else if let Ok(n) = value.parse::<f64>() {
                format!("{n:?}")
            } else if let Some(hex) = value.strip_prefix("0x").or(value.strip_prefix("0X")) {
                format!("{:?}", i64::from_str_radix(hex, 16).unwrap_or(0) as f64)
            } else {
                g.skip("constant of another kind");
                continue;
            };
            let id = g.value_name(constant);
            g.out += &format!("pub const {id} = {gleam}\n\n");
        }
        for f in &c.funcs {
            let f = with_self_args(f, c);
            let id = g.value_name(&function_name(&f, &f.name));
            g.function(&id, &f, Some((&c.name, &c.type_params)));
        }
        // Populate class_bound for synthesized to_list / get_at helpers.
        let populate_class_bound = |g: &Gen| {
            g.clear_fn_bounds();
            for (i, cp) in c.type_params.iter().enumerate() {
                if let Some(binding) = g.class_bound.get(&(c.name.clone(), i)) {
                    g.bound.borrow_mut().insert(cp.name.clone(), binding.clone());
                }
            }
        };
        // What it iterates, or holds as a subclass of Array: a list.
        if let Some(elem) = c.iterates.as_ref().or(c.elements.as_ref()) {
            let elem = with_self_args(&Func { type_params: vec![], module: String::new(), import: String::new(), name: String::new(), call: Call::Create(String::new()), params: vec![], rest: false, fixed: vec![], shared: None, ret: elem.clone() }, c).ret;
            populate_class_bound(&g);
            let id = g.value_name("to_list");
            let elem_ty = g.ty(&elem);
            let this = open(&g);
            let mut params = vec![format!("self: {this}")];
            let mut js_params = vec!["self".to_string()];
            for p in &c.type_params {
                if g.crosses(&elem, &p.name) {
                    g.note("Codec");
                    params.push(format!("{}_codec: Codec({})", lower(&p.name), Gen::tvar(&p.name)));
                    js_params.push(codec_var(&p.name));
                }
            }
            g.out += &format!("@external(javascript, \"{}\", \"{id}\")\npub fn {id}({}) -> List({elem_ty})\n\n", g.own_ffi(), params.join(", "));
            g.ffi += &format!("export function {id}({}) {{\n  return toList(Array.from(self, (v) => {}));\n}}\n", js_params.join(", "), g.to_gleam(&elem, "v"));
            g.clear_fn_bounds();
        }
        // A subclass of Array has no class to inherit Array's members from:
        // its elements by index, and how many.
        if let Some(elem) = c.elements.as_ref() {
            let elem = with_self_args(&Func { type_params: vec![], module: String::new(), import: String::new(), name: String::new(), call: Call::Create(String::new()), params: vec![], rest: false, fixed: vec![], shared: None, ret: elem.clone() }, c).ret;
            populate_class_bound(&g);
            let elem_ty = g.ty(&elem);
            g.note("Option");
            let (at, length) = (g.value_name("get_at"), g.value_name("length"));
            let this = open(&g);
            let mut at_params = vec![format!("self: {this}"), "index: Int".to_string()];
            let mut at_js_params = vec!["self".to_string(), "i".to_string()];
            for p in &c.type_params {
                if g.crosses(&elem, &p.name) {
                    g.note("Codec");
                    at_params.push(format!("{}_codec: Codec({})", lower(&p.name), Gen::tvar(&p.name)));
                    at_js_params.push(codec_var(&p.name));
                }
            }
            g.out += &format!("@external(javascript, \"{}\", \"{at}\")\npub fn {at}({}) -> Option({elem_ty})\n\n", g.own_ffi(), at_params.join(", "));
            let this = open(&g);
            g.out += &format!("@external(javascript, \"{}\", \"{length}\")\npub fn {length}(self: {this}) -> Int\n\n", g.own_ffi());
            g.ffi += &format!(
                "export function {at}({}) {{\n  const v = self[i];\n  return v === undefined ? new None() : new Some({});\n}}\nexport function {length}(self) {{\n  return self.length;\n}}\n",
                at_js_params.join(", "),
                g.to_gleam(&elem, "v")
            );
            g.clear_fn_bounds();
        }
        if c.iterates_async.is_some() {
            g.skip("async iteration");
        }
        // Upcasts. Along the chain a subtype is its base already; this closes
        // the type at this class, for a list or a branch of several subtypes:
        // `element.upcast(div)` is an `Element(Nil)`.
        let id = g.value_name("upcast");
        let (this, closed) = (open(&g), g.class_type(&c.name, &own));
        g.out += &format!("@external(javascript, \"{}\", \"dynamic\")\npub fn {id}(value: {this}) -> {closed}\n\n", g.runtime_ffi());
        // Covariant widening for generic classes with class-typed defaults:
        // `shape.widen(rect)` turns `Shape(RectConfig(Nil), RectTag(Nil))` into `Shape(ShapeConfig(Nil), Nil)`.
        if c.type_params.iter().any(|p| matches!(&p.default, Some(Ty::Obj(dn) | Ty::Generic(dn, _)) if g.classes.contains_key(dn))) {
            let wid = g.value_name("widen");
            let mut n = 0usize;
            let mut in_parts: Vec<String> = Vec::new();
            let mut out_parts: Vec<String> = Vec::new();
            for p in &c.type_params {
                match &p.default {
                    Some(Ty::Obj(dn)) if g.classes.contains_key(dn) => {
                        let count = g.classes[dn];
                        let mut sub_parts: Vec<String> = (0..count).map(|_| { n += 1; format!("w{n}_") }).collect();
                        n += 1;
                        sub_parts.push(format!("w{n}_"));
                        in_parts.push(format!("{}({})", g.gt(dn), sub_parts.join(", ")));
                        out_parts.push(g.class_type(dn, &[]));
                    }
                    Some(Ty::Generic(dn, dargs)) if g.classes.contains_key(dn) => {
                        let count = g.classes[dn];
                        let mut sub_parts: Vec<String> = (0..count).map(|i| match dargs.get(i) {
                            Some(Ty::Any) | None => { n += 1; format!("w{n}_") }
                            Some(a) => g.closed(|| g.ty(a)),
                        }).collect();
                        n += 1;
                        sub_parts.push(format!("w{n}_"));
                        in_parts.push(format!("{}({})", g.gt(dn), sub_parts.join(", ")));
                        out_parts.push(g.class_type(dn, dargs));
                    }
                    _ => {
                        let v = Gen::tvar(&p.name);
                        in_parts.push(v.clone());
                        out_parts.push(v);
                    }
                }
            }
            n += 1;
            in_parts.push(format!("w{n}_"));
            out_parts.push("Nil".into());
            let from = format!("{}({})", g.gt(&c.name), in_parts.join(", "));
            let to = format!("{}({})", g.gt(&c.name), out_parts.join(", "));
            g.out += &format!("@external(javascript, \"{}\", \"dynamic\")\npub fn {wid}(value: {from}) -> {to}\n\n", g.runtime_ffi());
        }
        // A base off the chain, whose members it has taken: `html_element.as_global_event_handlers(e)`.
        for a in secondary(&m, c) {
            let Some(args) = base_args(&m, c, &a) else { continue };
            let id = g.value_name(&format!("as_{}", snake(&g.tname(&a))));
            let (this, base) = (open(&g), g.class_type(&a, &args));
            g.out += &format!("@external(javascript, \"{}\", \"dynamic\")\npub fn {id}(value: {this}) -> {base}\n\n", g.runtime_ffi());
        }
        // Downcasts, checked: only a class has a constructor to test against.
        if let Some(js) = &c.js {
            let test = format!("v instanceof {}", global(js));
            let target = g.class_type(&c.name, &[]);
            let root = chain_root(&g, &c.name);
            if root != c.name {
                let id = if g.values.contains(&(g.unit.clone(), "from".into())) {
                    g.fresh_value("from_base")
                } else {
                    g.value_name("from")
                };
                *g.open.borrow_mut() = Some(0);
                let root_args: Vec<Ty> = (0..g.classes[&root]).map(|i| Ty::Param(format!("r_{i}"))).collect();
                let from = g.class_type(&root, &root_args);
                *g.open.borrow_mut() = None;
                g.out += &format!("@external(javascript, \"{}\", \"{id}\")\npub fn {id}(value: {from}) -> Result({target}, Nil)\n\n", g.own_ffi());
                g.ffi += &format!("export function {id}(v) {{\n  return {test} ? new Ok(v) : new GleamError(undefined);\n}}\n");
            }
            let id = if g.values.contains(&(g.unit.clone(), "cast".into())) {
                g.fresh_value("cast_dynamic")
            } else {
                g.value_name("cast")
            };
            g.note("Dynamic");
            g.out += &format!("@external(javascript, \"{}\", \"{id}\")\npub fn {id}(value: Dynamic) -> Result({target}, Nil)\n\n", g.own_ffi());
            g.ffi += &format!("export function {id}(v) {{\n  return {test} ? new Ok(v) : new GleamError(undefined);\n}}\n");
        }
        // Specialized primary-ancestor methods where substituting concrete generic args eliminates codecs.
        g.emit_specializations(c);
        // Fluent `with_<prop>` / `with_at` builders for option bags (`Call::Create`).
        g.emit_with_builders(c);
    }
    for f in &m.funcs {
        match m.func_namespaces.get(&f.import) {
            // A namespace's by its own name, in its module (a nested one's
            // in a module of it): `geometry.distance`, `deep.twice`.
            Some((path, own, module, depth)) => {
                let unit = std::iter::once(module_path(&paths, pkg, module)).chain(path[(*depth).min(path.len())..].iter().map(|s| snake(&upper(s)))).collect::<Vec<_>>().join("/");
                g.switch(&unit);
                let id = g.fresh_value(&function_name(f, own));
                g.function(&id, f, None);
            }
            None => {
                g.switch(&paths[&f.module]);
                let id = g.value_name(&function_name(f, &f.name));
                g.function(&id, f, None);
            }
        }
    }
    g.switch("");

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
                format!(
                    "import {module}.{{{}}}{suffix}\n",
                    ts.iter()
                        .map(|t| match ref_aliases.get(*t) {
                            Some((ref_mod, orig)) if ref_mod == module => format!("type {orig} as {t}"),
                            _ => format!("type {t}"),
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })
            .collect();
        files.push((format!("{path}.gleam"), format!("// Generated. Bindings for Gleam's JavaScript target.\n{imports}\n{}", unit.out)));
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
            header += &format!("import {{ $wrap, $same, PK, G }} from \"{}\";\nimport * as $T from \"{}\";\n", relative(&path, &format!("{pkg}/runtime_ffi.mjs")), relative(&path, &format!("{pkg}/types_ffi.mjs")));
        }
        if path == types_mod {
            header += "import * as B from \"./types.mjs\";\n";
        }
        files.push((format!("{path}_ffi.mjs"), format!("{header}{}", unit.ffi)));
    }
    let project_refs = used_refs.iter().map(|&i| (refs[i].package_name.clone(), refs[i].dep_path.clone())).collect();
    let meta_classes: Vec<String> = m.classes.iter().filter(|c| c.type_params.is_empty()).map(|c| g.tname(&c.name)).collect();
    let meta = super::pkg_meta::PkgMeta {
        target: "gleam".into(),
        package: pkg.to_string(),
        types: meta_classes,
        ..Default::default()
    };
    Ok(Output { name: pkg.to_string(), files, project_refs, skipped: g.skipped, manifest: super::model::manifest(&m), meta_toml: meta.to_toml() })
}

// What JavaScript reserves, as a function's name.
const JS_RESERVED: &[&str] = &[
    "await", "break", "case", "catch", "class", "const", "continue", "debugger", "default", "delete", "do", "else", "enum", "export", "extends", "false",
    "finally", "for", "function", "if", "implements", "import", "in", "instanceof", "interface", "let", "new", "null", "package", "private", "protected",
    "public", "return", "static", "super", "switch", "this", "throw", "true", "try", "typeof", "var", "void", "while", "with", "yield",
    // Not names in a module's strict code.
    "arguments", "eval",
];

// The modules every library's bindings have.
const OWN_RUNTIME: &[&str] = &["Promise", "JsError", "Codec", "Ref"];

// A class type's phantom parameter, where it is declared.
const PHANTOM: &str = "sub";

// The class a class's chain continues from, and the arguments it gives it:
// its first base that is a class of the model.
fn parent(m: &Model, c: &Class) -> Option<(String, Vec<Ty>)> {
    c.bases.iter().find_map(|b| match b {
        Ty::Obj(n) if m.classes.iter().any(|k| k.name == *n) => Some((n.clone(), Vec::new())),
        Ty::Generic(n, args) if m.classes.iter().any(|k| k.name == *n) => Some((n.clone(), args.clone())),
        _ => None,
    })
}

// The top of a class's chain.
fn chain_root(g: &Gen, class: &str) -> String {
    let mut at = class.to_string();
    while let Some((p, _)) = g.parents.get(&at) {
        at = p.clone();
    }
    at
}

// Every class a class extends, through any base.
fn ancestors(m: &Model, c: &Class) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut todo: Vec<&Class> = vec![c];
    while let Some(k) = todo.pop() {
        for b in &k.bases {
            let (Ty::Obj(n) | Ty::Generic(n, _)) = b else { continue };
            if let Some(base) = m.classes.iter().find(|x| x.name == *n)
                && !out.contains(n)
            {
                out.push(n.clone());
                todo.push(base);
            }
        }
    }
    out
}

// The classes a class extends off its chain: neither on the chain nor
// extended by a class on it. Their members it takes as its own.
fn secondary(m: &Model, c: &Class) -> Vec<String> {
    let mut chain: Vec<&Class> = Vec::new();
    let mut at = c;
    while let Some((p, _)) = parent(m, at) {
        let Some(k) = m.classes.iter().find(|k| k.name == p) else { break };
        chain.push(k);
        at = k;
    }
    let mut covered: HashSet<String> = chain.iter().map(|k| k.name.clone()).collect();
    for k in &chain {
        covered.extend(ancestors(m, k));
    }
    ancestors(m, c).into_iter().filter(|a| !covered.contains(a)).collect()
}

// Each class takes its secondary bases' members, their type arguments
// applied; its own of a name win. Those on its chain it has as a subtype.
fn take_secondary(m: &mut Model) {
    let mut taken: Vec<(usize, Vec<Func>)> = Vec::new();
    for (i, c) in m.classes.iter().enumerate() {
        let mut names: HashSet<String> = c.funcs.iter().map(|f| f.name.clone()).collect();
        let mut funcs = Vec::new();
        for s in secondary(m, c) {
            let (Some(k), Some(args)) = (m.classes.iter().find(|k| k.name == s), base_args(m, c, &s)) else { continue };
            for f in k.funcs.iter().filter(|f| Class::has_self(f)) {
                if names.insert(f.name.clone()) {
                    funcs.push(map_types(f, &|t| subst(t, &k.type_params, &args)));
                }
            }
        }
        taken.push((i, funcs));
    }
    for (i, funcs) in taken {
        m.classes[i].funcs.extend(funcs);
    }
}

// `to` (a path from the package's root), relative to module `from`.
fn relative(from: &str, to: &str) -> String {
    let from: Vec<&str> = from.split('/').collect();
    let from = &from[..from.len() - 1];
    let to: Vec<&str> = to.split('/').collect();
    let common = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
    let up = "../".repeat(from.len() - common);
    let rest = to[common..].join("/");
    if up.is_empty() { format!("./{rest}") } else { format!("{up}{rest}") }
}

// A function's name, as the model has it; a `$` method (`$add`, which
// returns a new value where `add` changes this one) is `add_new`, while a `$`
// property accessor (`$from`) is `get_dollar_from` / `set_dollar_from`.
fn function_name(f: &Func, name: &str) -> String {
    let dollar = super::case::has_dollar_js_name(&f.call);
    if !dollar {
        return name.to_string();
    }
    let is_prop = matches!(&f.call, Call::Get(_) | Call::GetBound(_) | Call::Set(_) | Call::StaticGet(_) | Call::StaticSet(_));
    let is_set = matches!(&f.call, Call::Set(_) | Call::StaticSet(_));
    if is_prop {
        let stripped = if is_set { name.strip_prefix("set_") } else { name.strip_prefix("get_") }.unwrap_or(name);
        let stem = stripped.trim_start_matches('_');
        let prefix = if is_set { "set_dollar" } else { "get_dollar" };
        return if stem.is_empty() { prefix.to_string() } else { format!("{prefix}_{stem}") };
    }
    let stem = name.trim_start_matches('_');
    match stem.strip_prefix("set_") {
        Some(rest) => format!("set_{}_new", rest.trim_start_matches('_')),
        None => format!("{}_new", if stem.is_empty() { "dollar" } else { stem }),
    }
}

// A callback holding itself, through its own types or another's: a Gleam
// alias may not, so within such a cycle a callback is `Dynamic`.
fn break_cycles(m: &mut Model) {
    fn refs(t: &Ty, names: &HashSet<String>, out: &mut HashSet<String>) {
        match t {
            Ty::Obj(n) if names.contains(n) => {
                out.insert(n.clone());
            }
            Ty::Generic(n, args) => {
                if names.contains(n) {
                    out.insert(n.clone());
                }
                args.iter().for_each(|a| refs(a, names, out));
            }
            Ty::Nullable(inner, _) | Ty::Listener(inner) | Ty::ArrayOf(inner) => refs(inner, names, out),
            Ty::Tuple(es) | Ty::Union(es) => es.iter().for_each(|e| refs(e, names, out)),
            _ => {}
        }
    }
    fn cut(t: &mut Ty, cyclic: &HashSet<String>) {
        match t {
            Ty::Obj(n) | Ty::Generic(n, _) if cyclic.contains(n) => *t = Ty::Any,
            Ty::Generic(_, args) => args.iter_mut().for_each(|a| cut(a, cyclic)),
            Ty::Nullable(inner, _) | Ty::Listener(inner) | Ty::ArrayOf(inner) => cut(inner, cyclic),
            Ty::Tuple(es) | Ty::Union(es) => es.iter_mut().for_each(|e| cut(e, cyclic)),
            _ => {}
        }
    }
    let names: HashSet<String> = m.callbacks.iter().map(|c| c.name.clone()).collect();
    let edges: HashMap<String, HashSet<String>> = m
        .callbacks
        .iter()
        .map(|c| {
            let mut out = HashSet::new();
            c.params.iter().map(|p| &p.1).chain([&c.ret]).for_each(|t| refs(t, &names, &mut out));
            (c.name.clone(), out)
        })
        .collect();
    let reach = |from: &str| {
        let mut seen: HashSet<&str> = HashSet::new();
        let mut todo: Vec<&str> = edges[from].iter().map(String::as_str).collect();
        while let Some(n) = todo.pop() {
            if seen.insert(n) {
                todo.extend(edges[n].iter().map(String::as_str));
            }
        }
        seen.into_iter().map(str::to_string).collect::<HashSet<String>>()
    };
    let reaches: HashMap<String, HashSet<String>> = names.iter().map(|n| (n.clone(), reach(n))).collect();
    for c in &mut m.callbacks {
        // Those it reaches that reach it back.
        let cycle: HashSet<String> = reaches[&c.name].iter().filter(|d| reaches[*d].contains(&c.name)).cloned().collect();
        if !cycle.is_empty() {
            c.params.iter_mut().for_each(|p| cut(&mut p.1, &cycle));
            cut(&mut c.ret, &cycle);
        }
    }
}

// The JavaScript name of the codec of type parameter `n`.
fn codec_var(n: &str) -> String {
    format!("$c_{}", n.replace(|c: char| !c.is_alphanumeric() && c != '_', "_"))
}

fn generic_args(t: &Ty) -> &[Ty] {
    match t {
        Ty::Generic(_, args) => args,
        _ => &[],
    }
}

// Whether `t` names any type parameter in `params`.
fn mentions_any(t: &Ty, params: &[TParam]) -> bool {
    params.iter().any(|p| mentions(t, &p.name))
}

fn is_chain_subtype(parents: &HashMap<String, (String, Vec<Ty>)>, sub: &str, base: &str) -> bool {
    let mut at = sub;
    loop {
        if at == base {
            return true;
        }
        let Some((p, _)) = parents.get(at) else { return false };
        at = p;
    }
}

fn effective_base_args(m: &Model, ty: &Ty, base: &str) -> Option<Vec<Ty>> {
    let (name, args) = match ty {
        Ty::Obj(n) => (n.as_str(), Vec::new()),
        Ty::Generic(n, args) => (n.as_str(), args.clone()),
        _ => return None,
    };
    if name == base {
        return Some(args);
    }
    let cls = m.classes.iter().find(|c| c.name == name)?;
    let bargs = base_args(m, cls, base)?;
    Some(bargs.into_iter().map(|a| subst(&a, &cls.type_params, &args)).collect())
}

fn arg_satisfies(
    m: &Model,
    classes: &HashMap<String, usize>,
    parents: &HashMap<String, (String, Vec<Ty>)>,
    arg: &Ty,
    base: &str,
    cargs: &[Ty],
    scope_tparams: &[TParam],
) -> bool {
    let concrete_check = |candidate: &Ty| -> bool {
        if cargs.is_empty() {
            return true;
        }
        let Some(eff) = effective_base_args(m, candidate, base) else { return false };
        cargs.iter().enumerate().all(|(k, ca)| matches!(ca, Ty::Any) || eff.get(k) == Some(ca))
    };
    match arg {
        Ty::Obj(x) | Ty::Generic(x, _) => classes.contains_key(x) && is_chain_subtype(parents, x, base) && concrete_check(arg),
        Ty::Param(tp_name) => {
            let mut cur = tp_name.as_str();
            let mut seen = HashSet::new();
            while seen.insert(cur) {
                let Some(tp) = scope_tparams.iter().find(|p| p.name == cur) else { return false };
                match &tp.constraint {
                    Some(c @ (Ty::Obj(x) | Ty::Generic(x, _))) => {
                        return classes.contains_key(x) && is_chain_subtype(parents, x, base) && concrete_check(c);
                    }
                    Some(Ty::Param(next)) => cur = next.as_str(),
                    _ => return false,
                }
            }
            false
        }
        _ => false,
    }
}

struct ClassBoundQuery<'a> {
    target_class: &'a str,
    param_idx: usize,
    base: &'a str,
    cargs: &'a [Ty],
    default_ok: bool,
    m: &'a Model,
    classes: &'a HashMap<String, usize>,
    parents: &'a HashMap<String, (String, Vec<Ty>)>,
}

fn check_class_bound_ty(t: &Ty, q: &ClassBoundQuery<'_>, scope_tparams: &[TParam]) -> bool {
    match t {
        Ty::Obj(cn) if cn == q.target_class => q.default_ok,
        Ty::Generic(cn, args) => {
            if cn == q.target_class {
                let ok = match args.get(q.param_idx) {
                    Some(a) => arg_satisfies(q.m, q.classes, q.parents, a, q.base, q.cargs, scope_tparams),
                    None => q.default_ok,
                };
                if !ok {
                    return false;
                }
            }
            args.iter().all(|a| check_class_bound_ty(a, q, scope_tparams))
        }
        Ty::Nullable(inner, _) | Ty::ArrayOf(inner) | Ty::Listener(inner) => check_class_bound_ty(inner, q, scope_tparams),
        Ty::Tuple(es) | Ty::Union(es) => es.iter().all(|e| check_class_bound_ty(e, q, scope_tparams)),
        _ => true,
    }
}

// Identifies class type parameters that are strictly constrained to a class
// across every instantiation in the model, so codecs can be omitted safely.
fn compute_class_bound(
    m: &Model,
    classes: &HashMap<String, usize>,
    parents: &HashMap<String, (String, Vec<Ty>)>,
) -> HashMap<(String, usize), (String, Vec<Ty>)> {
    let mut out = HashMap::new();
    for c in &m.classes {
        for (i, p) in c.type_params.iter().enumerate() {
            let Some((base, cargs)) = (match &p.constraint {
                Some(con @ Ty::Obj(base)) if classes.contains_key(base) && !mentions_any(con, &c.type_params) => Some((base.clone(), Vec::new())),
                Some(con @ Ty::Generic(base, args)) if classes.contains_key(base) && !mentions_any(con, &c.type_params) => Some((base.clone(), args.clone())),
                _ => None,
            }) else {
                continue;
            };
            let default_ok = match &p.default {
                Some(def) => arg_satisfies(m, classes, parents, def, &base, &cargs, &c.type_params),
                None => false,
            };
            // `from` and `cast` instantiate `class_type(&c.name, &[])`, which falls back
            // to `Dynamic` when a parameter lacks a valid default.
            if c.js.is_some() && !default_ok {
                continue;
            }
            let query = ClassBoundQuery { target_class: &c.name, param_idx: i, base: &base, cargs: &cargs, default_ok, m, classes, parents };
            let check = |t: &Ty, scope: &[TParam]| check_class_bound_ty(t, &query, scope);
            let mut ok = true;
            'classes: for c2 in &m.classes {
                for tp in &c2.type_params {
                    if tp.default.as_ref().is_some_and(|d| !check(d, &c2.type_params))
                        || tp.constraint.as_ref().is_some_and(|ct| !check(ct, &c2.type_params))
                    {
                        ok = false;
                        break 'classes;
                    }
                }
                for b in &c2.bases {
                    if !check(b, &c2.type_params) {
                        ok = false;
                        break 'classes;
                    }
                }
                for t in c2.iterates.iter().chain(&c2.iterates_async).chain(&c2.elements) {
                    if !check(t, &c2.type_params) {
                        ok = false;
                        break 'classes;
                    }
                }
                for f in &c2.funcs {
                    let mut scope: Vec<TParam> = f.type_params.clone();
                    scope.extend(c2.type_params.iter().cloned());
                    if f.type_params.iter().any(|tp| {
                        tp.default.as_ref().is_some_and(|d| !check(d, &scope))
                            || tp.constraint.as_ref().is_some_and(|ct| !check(ct, &scope))
                    }) || f.params.iter().any(|(_, pt)| !check(pt, &scope))
                        || !check(&f.ret, &scope)
                    {
                        ok = false;
                        break 'classes;
                    }
                }
            }
            if ok {
                for f in &m.funcs {
                    if f.type_params.iter().any(|tp| {
                        tp.default.as_ref().is_some_and(|d| !check(d, &f.type_params))
                            || tp.constraint.as_ref().is_some_and(|ct| !check(ct, &f.type_params))
                    }) || f.params.iter().any(|(_, pt)| !check(pt, &f.type_params))
                        || !check(&f.ret, &f.type_params)
                    {
                        ok = false;
                        break;
                    }
                }
            }
            if ok {
                for cb in &m.callbacks {
                    if cb.type_params.iter().any(|tp| {
                        tp.default.as_ref().is_some_and(|d| !check(d, &cb.type_params))
                            || tp.constraint.as_ref().is_some_and(|ct| !check(ct, &cb.type_params))
                    }) || cb.params.iter().any(|(_, pt)| !check(pt, &cb.type_params))
                        || !check(&cb.ret, &cb.type_params)
                    {
                        ok = false;
                        break;
                    }
                }
            }
            if ok {
                for u in &m.unions {
                    if u.type_params.iter().any(|tp| {
                        tp.default.as_ref().is_some_and(|d| !check(d, &u.type_params))
                            || tp.constraint.as_ref().is_some_and(|ct| !check(ct, &u.type_params))
                    }) || u.parts.iter().any(|pt| !check(pt, &u.type_params))
                    {
                        ok = false;
                        break;
                    }
                }
            }
            if ok {
                out.insert((c.name.clone(), i), (base, cargs));
            }
        }
    }
    out
}

// A module's Gleam path: as given out, else as one would be (`areas/pb`
// where only a namespace of the package's has functions).
fn module_path(paths: &HashMap<String, String>, pkg: &str, module: &str) -> String {
    paths.get(module).cloned().unwrap_or_else(|| format!("{pkg}/{}", module.split('/').map(lower).collect::<Vec<_>>().join("/")))
}
