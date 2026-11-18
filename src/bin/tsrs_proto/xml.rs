// IR -> XML, plus a census of the constructs that were emitted.

use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::hash::{Hash, Hasher};

use std::fmt::Write as _;

use crate::ir::*;

#[derive(Default)]
pub struct Xml {
    pub out: String,
    depth: usize,
    // Element and flag counts: what a consumer of this output must handle.
    pub census: BTreeMap<String, usize>,
    // Attributes for the next element, from `Type::Tested`.
    pending: Vec<(&'static str, String)>,
    // The same, written after the element's own (`origin`).
    trailing: Vec<(&'static str, String)>,
    // Each type declaration's id (its path: `Konva.Shape`), by path and by
    // symbol; what a reference's `to` names. See `index`.
    ids: HashMap<String, String>,
    by_sym: HashMap<Sym, String>,
    // Declarations without a symbol: what lowering made up.
    symless: HashSet<String>,
    // The path of the namespaces being written.
    prefix: String,
    // Hashing what would be written, in place of writing it: see `key`.
    hash: Option<[DefaultHasher; 2]>,
}

type Attrs<'a> = Vec<(&'a str, &'a str)>;

impl Xml {
    fn tally(&mut self, what: &str) {
        *self.census.entry(what.to_string()).or_default() += 1;
    }

    fn line(&mut self, tag: &str, attrs: &Attrs, close: bool) {
        if let Some(hs) = &mut self.hash {
            for h in hs.iter_mut() {
                (b'<', tag).hash(h);
                for (k, v) in &self.pending {
                    (k, v).hash(h);
                }
                for (k, v) in attrs {
                    (k, v).hash(h);
                }
                for (k, v) in &self.trailing {
                    (k, v).hash(h);
                }
                close.hash(h);
            }
            self.pending.clear();
            self.trailing.clear();
            return;
        }
        self.tally(tag);
        let _ = write!(self.out, "{:w$}<{tag}", "", w = self.depth * 2);
        for (k, v) in std::mem::take(&mut self.pending) {
            self.tally(&format!("{tag}@{k}"));
            let _ = write!(self.out, " {k}=\"{}\"", escape(&v));
        }
        for (k, v) in attrs {
            if matches!(*k, "optional" | "rest" | "lossy") {
                self.tally(&format!("{tag}@{k}"));
            }
            let _ = write!(self.out, " {k}=\"{}\"", escape(v));
        }
        for (k, v) in std::mem::take(&mut self.trailing) {
            let _ = write!(self.out, " {k}=\"{}\"", escape(&v));
        }
        self.out.push_str(if close { "/>\n" } else { ">\n" });
    }
    fn open(&mut self, tag: &str, attrs: &Attrs) {
        self.line(tag, attrs, false);
        self.depth += 1;
    }
    fn close(&mut self, tag: &str) {
        self.depth -= 1;
        if let Some(hs) = &mut self.hash {
            hs.iter_mut().for_each(|h| (b'>', tag).hash(h));
            return;
        }
        let _ = writeln!(self.out, "{:w$}</{tag}>", "", w = self.depth * 2);
    }
    fn leaf(&mut self, tag: &str, attrs: &Attrs) {
        self.line(tag, attrs, true);
    }
    fn wrap(&mut self, tag: &str, types: &[&Type]) {
        self.open(tag, &vec![]);
        for t in types {
            self.ty(t);
        }
        self.close(tag);
    }

    // Gives each type declaration an id, its path, unique among them.
    fn index(&mut self, decls: &[Decl], prefix: &str) {
        for d in decls {
            let (name, sym) = match d {
                Decl::Interface(i) => (&i.name, i.sym),
                Decl::Alias(a) => (&a.name, a.sym),
                Decl::Enum(e) => (&e.name, e.sym),
                Decl::Namespace(n) => {
                    self.index(&n.decls, &format!("{prefix}{}.", n.name));
                    continue;
                }
                _ => continue,
            };
            let path = format!("{prefix}{name}");
            let mut id = path.clone();
            let mut n = 2;
            while self.ids.values().any(|v| *v == id) {
                id = format!("{path}#{n}");
                n += 1;
            }
            match sym {
                Some(_) => {
                    self.by_sym.insert(sym, id.clone());
                }
                None => {
                    self.symless.insert(path.clone());
                }
            }
            self.ids.insert(path, id);
        }
    }

    // The declaration a reference means: by its symbol; else, for one that
    // has none or whose symbol is not declared here, a declaration without a
    // symbol of its path. Never another declaration of the same name.
    fn target(&self, r: &Ref) -> Option<&String> {
        if r.kind == "typeParameter" {
            return None;
        }
        if let Some(id) = r.sym.and(self.by_sym.get(&r.sym)) {
            return Some(id);
        }
        let path = r.qualified.as_ref().unwrap_or(&r.name);
        if r.sym.is_none() || self.symless.contains(path) {
            return self.ids.get(path).filter(|_| r.sym.is_none() || self.symless.contains(path));
        }
        None
    }

    fn id_of(&self, name: &str) -> Option<String> {
        self.ids.get(&format!("{}{name}", self.prefix)).cloned()
    }

    // Top-level declarations, each with the module it belongs to.
    pub fn top(&mut self, decls: &[Decl], modules: &crate::config::Modules) {
        self.index(decls, "");
        // Origins tell declarations apart only where there are several.
        let origins: std::collections::HashSet<&String> = decls.iter().filter_map(|d| modules.origins.get(decl_key(d))).collect();
        for d in decls {
            let module = match d {
                Decl::Namespace(n) => n.name.clone(),
                Decl::Interface(i) => modules.module_of(&i.name, i.js_name.as_deref()).to_string(),
                Decl::Alias(a) => modules.module_of(&a.name, None).to_string(),
                Decl::Function(f) => modules.module_of(&f.name, f.js_name.as_deref()).to_string(),
                Decl::Var(v) => modules.module_of(&v.name, v.js_name.as_deref()).to_string(),
                Decl::Enum(e) => modules.module_of(&e.name, None).to_string(),
            };
            self.pending.push(("module", module));
            // With several packages, the one it is imported from.
            if let Some(package) = modules.packages.get(decl_key(d)) {
                self.pending.push(("import", package.clone()));
            }
            if origins.len() > 1
                && !matches!(d, Decl::Namespace(_))
                && let Some(origin) = modules.origins.get(decl_key(d))
            {
                self.trailing.push(("origin", origin.clone()));
            }
            // A namespace comes from the library, an area from the config: kept apart in file layouts.
            if matches!(d, Decl::Namespace(_)) {
                self.pending.push(("moduleKind", "namespace".into()));
            }
            self.decls(std::slice::from_ref(d));
        }
    }

    pub fn decls(&mut self, decls: &[Decl]) {
        for d in decls {
            match d {
                Decl::Interface(i) => self.interface(i),
                Decl::Alias(a) => {
                    let id = self.id_of(&a.name);
                    let mut attrs = vec![("name", a.name.as_str())];
                    if let Some(id) = &id {
                        attrs.push(("id", id));
                    }
                    if a.synthetic {
                        attrs.push(("synthetic", "true"));
                    }
                    self.open("typeAlias", &attrs);
                    self.type_params(&a.type_params);
                    if matches!(a.ty, Type::Object(_)) {
                        for e in &a.extends {
                            self.wrap("extends", &[e]);
                        }
                    }
                    self.ty(&a.ty);
                    self.close("typeAlias");
                }
                Decl::Function(f) => {
                    let mut attrs = vec![("name", f.name.as_str())];
                    if let Some(js) = &f.js_name {
                        attrs.push(("jsName", js));
                    }
                    let marks = f.sig.marks.join(" ");
                    if !marks.is_empty() {
                        attrs.push(("marks", &marks));
                    }
                    self.open("function", &attrs);
                    self.signature(&f.sig);
                    self.close("function");
                }
                Decl::Var(v) => {
                    let mut attrs = vec![("name", v.name.as_str())];
                    if let Some(js) = &v.js_name {
                        attrs.push(("jsName", js));
                    }
                    if v.readonly {
                        attrs.push(("readonly", "true"));
                    }
                    self.open("var", &attrs);
                    self.ty(&v.ty);
                    self.close("var");
                }
                Decl::Enum(e) => {
                    let id = self.id_of(&e.name);
                    let mut attrs = vec![("name", e.name.as_str())];
                    if let Some(id) = &id {
                        attrs.push(("id", id));
                    }
                    if e.synthetic {
                        attrs.push(("synthetic", "true"));
                    }
                    self.open("enum", &attrs);
                    for m in &e.members {
                        self.leaf("member", &vec![("name", &m.name), ("type", m.kind), ("value", &m.value)]);
                    }
                    self.close("enum");
                }
                Decl::Namespace(n) => {
                    let mut attrs = vec![("name", n.name.as_str())];
                    if let Some(js) = &n.js_name {
                        attrs.push(("jsName", js));
                    }
                    self.open("namespace", &attrs);
                    let inner = format!("{}{}.", self.prefix, n.name);
                    let outer = std::mem::replace(&mut self.prefix, inner);
                    self.decls(&n.decls);
                    self.prefix = outer;
                    self.close("namespace");
                }
            }
        }
    }

    fn interface(&mut self, i: &Interface) {
        let tag = if i.class.is_some() { "class" } else { "interface" };
        let id = self.id_of(&i.name);
        let mut attrs = vec![("name", i.name.as_str())];
        if let Some(id) = &id {
            attrs.push(("id", id));
        }
        if let Some(js) = &i.js_name {
            attrs.push(("jsName", js));
        }
        if i.synthetic {
            attrs.push(("synthetic", "true"));
        }
        self.open(tag, &attrs);
        self.type_params(&i.type_params);
        for e in &i.extends {
            self.wrap("extends", &[e]);
        }
        for (n, a) in i.ancestors.iter().enumerate() {
            let sym = i.ancestor_syms.get(n).copied().flatten();
            let r = Ref { name: a.clone(), kind: "interface", sym, args: Vec::new(), tid: None, qualified: None, file: None, computed: false, node: None };
            let to = self.target(&r).cloned();
            let mut attrs = vec![("name", a.as_str())];
            if let Some(to) = &to {
                attrs.push(("to", to));
            }
            self.leaf("ancestor", &attrs);
        }
        if let Some(c) = &i.class {
            for m in &c.constructors {
                let mut attrs = vec![("name", m.name.as_str())];
                if let Some(js) = &m.js_name {
                    attrs.push(("jsName", js));
                }
                let marks = m.sig.as_ref().map(|s| s.marks.join(" ")).unwrap_or_default();
                if !marks.is_empty() {
                    attrs.push(("marks", &marks));
                }
                self.open("constructor", &attrs);
                self.signature(m.sig.as_ref().unwrap());
                self.close("constructor");
            }
            self.members(&c.statics, true);
        }
        self.members(&i.members, false);
        self.close(tag);
    }

    fn members(&mut self, members: &[Member], is_static: bool) {
        // Same-named methods are overloads the consumer must be able to express.
        let mut names: BTreeMap<&str, usize> = BTreeMap::new();
        for m in members.iter().filter(|m| m.kind == MemberKind::Method) {
            *names.entry(&m.name).or_default() += 1;
        }
        let overloads: usize = names.values().filter(|n| **n > 1).map(|n| n - 1).sum();
        *self.census.entry("(overload)".into()).or_default() += overloads;
        for m in members {
            self.member(m, is_static);
        }
    }

    fn member(&mut self, m: &Member, is_static: bool) {
        let tag = match m.kind {
            MemberKind::Property => "property",
            MemberKind::Method => "method",
            MemberKind::Call => "call",
            MemberKind::Construct => "construct",
            MemberKind::Index => "index",
            MemberKind::Get => "get",
            MemberKind::Set => "set",
        };
        let mut attrs: Attrs = Vec::new();
        // A property or method always has a name, if an empty one: `"": number`.
        if !m.name.is_empty() || matches!(m.kind, MemberKind::Property | MemberKind::Method | MemberKind::Get | MemberKind::Set) {
            attrs.push(("name", &m.name));
        }
        if let Some(js) = &m.js_name {
            attrs.push(("jsName", js));
        }
        if is_static {
            attrs.push(("static", "true"));
        }
        if m.optional {
            attrs.push(("optional", "true"));
        }
        if m.readonly {
            attrs.push(("readonly", "true"));
        }
        if let Some(from) = &m.from {
            attrs.push(("from", from));
        }
        if let Some(c) = &m.constant {
            attrs.push(("const", c));
        }
        let marks = m.sig.as_ref().map(|s| s.marks.join(" ")).unwrap_or_default();
        if !marks.is_empty() {
            attrs.push(("marks", &marks));
        }
        self.open(tag, &attrs);
        if let Some(t) = &m.ty {
            self.ty(t);
        }
        if let Some(s) = &m.sig {
            self.signature(s);
        }
        self.close(tag);
    }

    fn type_params(&mut self, params: &[TypeParam]) {
        for p in params {
            self.type_param(p);
        }
    }

    fn type_param(&mut self, p: &TypeParam) {
        let attrs = vec![("name", p.name.as_str())];
        if p.constraint.is_none() && p.default.is_none() {
            return self.leaf("typeParameter", &attrs);
        }
        self.open("typeParameter", &attrs);
        if let Some(k) = &p.constraint {
            self.wrap("constraint", &[k]);
        }
        if let Some(d) = &p.default {
            self.wrap("default", &[d]);
        }
        self.close("typeParameter");
    }

    fn signature(&mut self, s: &Signature) {
        self.type_params(&s.type_params);
        // The declared receiver: not the `this` type, <this/>.
        if let Some(t) = &s.this {
            self.wrap("thisParameter", &[t]);
        }
        for p in &s.params {
            let mut attrs = vec![("name", p.name.as_str())];
            if p.optional {
                attrs.push(("optional", "true"));
            }
            if p.rest {
                attrs.push(("rest", "true"));
            }
            if let Some(c) = &p.constant {
                attrs.push(("const", c));
            }
            self.open("parameter", &attrs);
            self.ty(&p.ty);
            self.close("parameter");
        }
        if let Some(r) = &s.ret {
            self.wrap("returns", &[r]);
        }
    }

    pub fn ty(&mut self, t: &Type) {
        match t {
            Type::Primitive(n) => self.leaf("primitive", &vec![("name", n.as_str())]),
            Type::Erased(why) => self.leaf("primitive", &vec![("name", "any"), ("lossy", why)]),
            Type::Literal { kind, value } => self.leaf("literal", &vec![("type", kind), ("value", value.as_str())]),
            Type::Ref(r) => {
                let to = self.target(r).cloned();
                let mut attrs = vec![("name", r.name.as_str()), ("kind", r.kind)];
                if let Some(q) = &r.qualified {
                    attrs.push(("qualified", q));
                }
                if let Some(to) = &to {
                    attrs.push(("to", to));
                }
                if let Some(f) = &r.file {
                    attrs.push(("file", f));
                }
                if r.args.is_empty() {
                    return self.leaf("ref", &attrs);
                }
                self.open("ref", &attrs);
                for a in &r.args {
                    self.ty(a);
                }
                self.close("ref");
            }
            Type::Union(ts) => self.wrap("union", &ts.iter().collect::<Vec<_>>()),
            Type::Intersection(ts, _) => self.wrap("intersection", &ts.iter().collect::<Vec<_>>()),
            Type::Array(el) => self.wrap("array", &[el]),
            Type::Readonly(inner) => self.wrap("readonly", &[inner]),
            Type::Nullable(inner, absent) => {
                let absent = match absent {
                    Absent::Null => "null",
                    Absent::Undefined => "undefined",
                    Absent::Either => "either",
                };
                self.open("nullable", &vec![("absent", absent)]);
                self.ty(inner);
                self.close("nullable");
            }
            Type::This => self.leaf("this", &vec![]),
            Type::Tuple(els) => {
                self.open("tuple", &vec![]);
                for el in els {
                    let mut attrs: Attrs = Vec::new();
                    if let Some(n) = &el.name {
                        attrs.push(("name", n));
                    }
                    if el.optional {
                        attrs.push(("optional", "true"));
                    }
                    if el.rest {
                        attrs.push(("rest", "true"));
                    }
                    self.open("element", &attrs);
                    self.ty(&el.ty);
                    self.close("element");
                }
                self.close("tuple");
            }
            Type::Function(sig) | Type::Constructor(sig) => {
                let tag = if matches!(t, Type::Function(_)) { "functionType" } else { "constructorType" };
                self.open(tag, &vec![]);
                self.signature(sig);
                self.close(tag);
            }
            Type::Object(members) => {
                self.open("object", &vec![]);
                self.members(members, false);
                self.close("object");
            }
            Type::Op(op, _) => self.op(op),
            Type::Tested(inner, test) => {
                self.pending.push(match test {
                    Test::Discriminant(p) => ("discriminant", p.clone()),
                    t => ("test", test_text(t)),
                });
                self.ty(inner);
            }
        }
    }

    fn op(&mut self, op: &Op) {
        match op {
            Op::KeyOf(t) => self.wrap("keyof", &[t]),
            Op::Unique(t) => self.wrap("unique", &[t]),
            Op::IndexedAccess(o, i) => self.wrap("indexedAccess", &[o, i]),
            Op::TypeOf(name) => self.leaf("typeof", &vec![("name", name.as_str())]),
            Op::Predicate { param, asserts, ty } => {
                let mut attrs = vec![("parameter", param.as_str())];
                if *asserts {
                    attrs.push(("asserts", "true"));
                }
                self.open("predicate", &attrs);
                if let Some(t) = ty {
                    self.ty(t);
                }
                self.close("predicate");
            }
            Op::Template(parts) => {
                self.open("template", &vec![]);
                for p in parts {
                    match p {
                        TemplatePart::Text(s) => self.leaf("text", &vec![("value", s.as_str())]),
                        TemplatePart::Type(t) => self.ty(t),
                    }
                }
                self.close("template");
            }
            Op::Conditional { check, extends, yes, no } => {
                self.open("conditional", &vec![]);
                self.wrap("check", &[check]);
                // Not a base type, <extends>.
                self.wrap("extendsType", &[extends]);
                self.wrap("true", &[yes]);
                self.wrap("false", &[no]);
                self.close("conditional");
            }
            Op::Infer(p) => {
                self.open("infer", &vec![]);
                self.type_param(p);
                self.close("infer");
            }
            Op::Mapped { readonly, optional, param, as_, value } => {
                // `readonly`, `+readonly` add the modifier, `-readonly` removes it;
                // so `?`, `+?` and `-?`. Not the flags `readonly="true"`, `optional="true"`.
                let change = |m: &str| if m == "-" { "remove" } else { "add" };
                let mut attrs: Attrs = Vec::new();
                if let Some(r) = readonly {
                    attrs.push(("readonlyModifier", change(r)));
                }
                if let Some(o) = optional {
                    attrs.push(("optionalModifier", change(o)));
                }
                self.open("mapped", &attrs);
                self.type_param(param);
                if let Some(n) = as_ {
                    self.wrap("as", &[n]);
                }
                if let Some(v) = value {
                    self.wrap("value", &[v]);
                }
                self.close("mapped");
            }
            Op::Raw { kind, text } => self.leaf("raw", &vec![("kind", kind.as_str()), ("text", text.as_str())]),
        }
    }
}

// Structural identity of a type, member or signature: what its XML would
// be, hashed (128 bits) as it is walked, not written out.
fn key(write: impl FnOnce(&mut Xml)) -> String {
    let mut second = DefaultHasher::new();
    0x5au8.hash(&mut second);
    let mut x = Xml { hash: Some([DefaultHasher::new(), second]), ..Xml::default() };
    write(&mut x);
    let [a, b] = x.hash.unwrap();
    format!("{:016x}{:016x}", a.finish(), b.finish())
}

pub fn type_key(t: &Type) -> String {
    key(|x| x.ty(t))
}

pub fn member_key(m: &Member) -> String {
    key(|x| x.member(m, false))
}

pub fn signature_key(s: &Signature) -> String {
    key(|x| x.signature(s))
}

// `typeof:string`, `array`, `instanceof:HTMLElement`, `has:once`, `tag:circle`.
pub fn test_text(t: &Test) -> String {
    match t {
        Test::TypeOf(k) => format!("typeof:{k}"),
        Test::Array => "array".into(),
        Test::InstanceOf(c) => format!("instanceof:{c}"),
        Test::Has(p) => format!("has:{p}"),
        Test::Tag(v) => format!("tag:{v}"),
        Test::Discriminant(p) => format!("discriminant:{p}"),
    }
}

pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out += "&amp;",
            '<' => out += "&lt;",
            '>' => out += "&gt;",
            '"' => out += "&quot;",
            // As references, or a parser would read them as spaces.
            '\n' => out += "&#10;",
            '\t' => out += "&#9;",
            '\r' => out += "&#13;",
            // XML 1.0 cannot hold other control characters: written as JavaScript would.
            c if (c as u32) < 0x20 || c == '\u{7f}' => out += &format!("\\u{:04x}", c as u32),
            c => out.push(c),
        }
    }
    out
}

fn decl_key(d: &Decl) -> &str {
    match d {
        Decl::Interface(i) => &i.name,
        Decl::Alias(a) => &a.name,
        Decl::Function(f) => &f.name,
        Decl::Var(v) => &v.name,
        Decl::Enum(e) => &e.name,
        Decl::Namespace(n) => &n.name,
    }
}
