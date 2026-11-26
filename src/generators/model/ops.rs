use std::collections::{BTreeMap, BTreeSet};

use roxmltree::Node;

use super::{Call, Class, Func, Model, TParam, Ty};
use super::super::{attr, named, only_child};

impl Model {
    pub fn all_funcs(&self) -> impl Iterator<Item = &Func> {
        self.classes.iter().flat_map(|c| c.funcs.iter()).chain(self.funcs.iter())
    }

    // The packages the program's JavaScript is reached through: its calls,
    // its classes, and those unions test values to be instances of. A type
    // has none at runtime (node's `stream/iter`).
    pub fn runtime_packages(&self) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        let mut note = |path: &str| {
            if let Some((p, _)) = path.split_once("::") {
                out.insert(p.to_string());
            }
        };
        for f in self.funcs.iter().chain(self.classes.iter().flat_map(|c| &c.funcs)) {
            if let Call::New(p) | Call::Static(p) | Call::StaticGet(p) | Call::StaticSet(p) = &f.call {
                note(p);
            }
        }
        for c in &self.classes {
            if let Some(js) = &c.js {
                note(js);
            }
        }
        for u in &self.unions {
            for t in u.tests.iter().flatten() {
                if let Some(class) = t.strip_prefix("instanceof:")
                    && let Some(js) = self.js_refs.get(class)
                {
                    note(js);
                }
            }
        }
        out
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
        returns_this: f.returns_this,
        doc: f.doc.clone(),
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
        Ty::Record(inner) => Ty::Record(Box::new(walk(inner, f))),
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
            Ty::Nullable(inner, _) | Ty::Listener(inner) | Ty::ArrayOf(inner) | Ty::Record(inner) => visit(at, inner, f),
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
    for (a, t) in &m.record_aliases {
        visit(a, t, f);
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
        Ty::Nullable(inner, _) | Ty::ArrayOf(inner) | Ty::Listener(inner) | Ty::Record(inner) => mentions(inner, p),
        Ty::Tuple(es) | Ty::Union(es) | Ty::Generic(_, es) => es.iter().any(|e| mentions(e, p)),
        _ => false,
    }
}

// What each import reaches, for a page to check against the real API:
// `export const bindings = [[import, kind, owner, member, class], ...]`, the
// owner being where its class's constructor is (`HTMLElement`), or, for an
// interface no constructor is of (`GlobalEventHandlers`), `interface:` its
// name; and `implementers`, the constructors of each class's or interface's
// descendants (Chrome has `AbstractRange`'s members on `Range`'s prototype).
pub fn manifest(m: &Model) -> String {
    let kind = |c: &Call| match c {
        Call::Method(_) => "method",
        Call::Get(_) | Call::GetBound(_) => "get",
        Call::Set(_) => "set",
        Call::New(_) => "new",
        Call::Static(_) => "static",
        Call::StaticGet(_) => "staticGet",
        Call::StaticSet(_) => "staticSet",
        Call::Create(_) => "create",
        Call::Invoke => "invoke",
        Call::Construct => "construct",
        Call::IndexGet => "indexGet",
        Call::IndexSet => "indexSet",
        Call::Apply => "invoke",
    };
    let member = |c: &Call| match c {
        Call::Method(n) | Call::Get(n) | Call::GetBound(n) | Call::Set(n) | Call::New(n) | Call::Static(n) | Call::StaticGet(n) | Call::StaticSet(n) => n.clone(),
        _ => String::new(),
    };
    let json = |s: &str| format!("{s:?}");
    let mut out = String::from("// Generated. What each import reaches.\nexport const bindings = [\n");
    for c in &m.classes {
        let owner = c.js.clone().unwrap_or_else(|| format!("interface:{}", c.name));
        for f in &c.funcs {
            out += &format!("  [{}, {}, {}, {}, {}],\n", json(&f.import), json(kind(&f.call)), json(&owner), json(&member(&f.call)), json(&c.name));
        }
    }
    for f in &m.funcs {
        out += &format!("  [{}, {}, \"\", {}, \"\"],\n", json(&f.import), json(kind(&f.call)), json(&member(&f.call)));
    }
    out += "];\nexport const implementers = {\n";
    let mut by: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for c in m.classes.iter().filter(|c| c.js.is_some()) {
        for a in c.ancestors.iter().chain(std::iter::once(&c.name)) {
            by.entry(a.as_str()).or_default().push(c.js.as_deref().unwrap());
        }
    }
    for c in &m.classes {
        let js = by.get(c.name.as_str()).cloned().unwrap_or_default();
        out += &format!("  {}: [{}],\n", json(&c.name), js.iter().map(|j| json(j)).collect::<Vec<_>>().join(", "));
    }
    out += "};\n";
    out
}
