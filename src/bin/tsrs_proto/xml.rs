// IR -> XML, plus a census of the constructs that were emitted.

use std::collections::BTreeMap;
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
}

type Attrs<'a> = Vec<(&'a str, &'a str)>;

impl Xml {
    fn tally(&mut self, what: &str) {
        *self.census.entry(what.to_string()).or_default() += 1;
    }

    fn line(&mut self, tag: &str, attrs: &Attrs, close: bool) {
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
        self.out.push_str(if close { "/>\n" } else { ">\n" });
    }
    fn open(&mut self, tag: &str, attrs: &Attrs) {
        self.line(tag, attrs, false);
        self.depth += 1;
    }
    fn close(&mut self, tag: &str) {
        self.depth -= 1;
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

    pub fn decls(&mut self, decls: &[Decl]) {
        for d in decls {
            match d {
                Decl::Interface(i) => self.interface(i),
                Decl::Alias(a) => {
                    let mut attrs = vec![("name", a.name.as_str())];
                    if a.synthetic {
                        attrs.push(("synthetic", "true"));
                    }
                    self.open("typeAlias", &attrs);
                    self.type_params(&a.type_params);
                    self.ty(&a.ty);
                    self.close("typeAlias");
                }
                Decl::Function(f) => {
                    let mut attrs = vec![("name", f.name.as_str())];
                    if let Some(js) = &f.js_name {
                        attrs.push(("jsName", js));
                    }
                    self.open("function", &attrs);
                    self.signature(&f.sig);
                    self.close("function");
                }
                Decl::Var(v) => {
                    self.open("var", &vec![("name", v.name.as_str())]);
                    self.ty(&v.ty);
                    self.close("var");
                }
                Decl::Enum(e) => {
                    let mut attrs = vec![("name", e.name.as_str())];
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
                    self.open("namespace", &vec![("name", n.name.as_str())]);
                    self.decls(&n.decls);
                    self.close("namespace");
                }
            }
        }
    }

    fn interface(&mut self, i: &Interface) {
        let tag = if i.class.is_some() { "class" } else { "interface" };
        let mut attrs = vec![("name", i.name.as_str())];
        if i.synthetic {
            attrs.push(("synthetic", "true"));
        }
        self.open(tag, &attrs);
        self.type_params(&i.type_params);
        for e in &i.extends {
            self.wrap("extends", &[e]);
        }
        for a in &i.ancestors {
            self.leaf("ancestor", &vec![("name", a.as_str())]);
        }
        if let Some(c) = &i.class {
            for m in &c.constructors {
                let mut attrs = vec![("name", m.name.as_str())];
                if let Some(js) = &m.js_name {
                    attrs.push(("jsName", js));
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
        if !m.name.is_empty() {
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
        if let Some(t) = &s.this {
            self.wrap("this", &[t]);
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
                let mut attrs = vec![("name", r.name.as_str()), ("kind", r.kind)];
                if let Some(q) = &r.qualified {
                    attrs.push(("qualified", q));
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
                self.wrap("extends", &[extends]);
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
                let mut attrs: Attrs = Vec::new();
                if let Some(r) = readonly {
                    attrs.push(("readonly", r));
                }
                if let Some(o) = optional {
                    attrs.push(("optional", o));
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

// Structural identity of a type or member: its XML rendering.
pub fn type_key(t: &Type) -> String {
    let mut x = Xml::default();
    x.ty(t);
    x.out
}

pub fn member_key(m: &Member) -> String {
    let mut x = Xml::default();
    x.member(m, false);
    x.out
}

pub fn signature_key(s: &Signature) -> String {
    let mut x = Xml::default();
    x.signature(s);
    x.out
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

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}
