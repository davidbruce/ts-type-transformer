// IR: declarations as written, with every reference resolved to a checker symbol.
// Lowering passes rewrite this; the XML printer serializes it.

use tsrs_ast::{Node, Symbol};
use tsrs_core::P;

pub type Sym = Option<P<Symbol>>;

#[derive(Clone)]
pub enum Type {
    Primitive(String),
    // `any` produced by a lowering that lost information; the reason is kept.
    Erased(&'static str),
    Literal { kind: &'static str, value: String },
    Ref(Ref),
    Union(Vec<Type>),
    Intersection(Vec<Type>, Option<P<Node>>),
    Array(Box<Type>),
    Tuple(Vec<Element>),
    Function(Box<Signature>),
    Constructor(Box<Signature>),
    Object(Vec<Member>),
    Readonly(Box<Type>),
    This,
    // Lowered form of `T | null | undefined`, with which of the two it was.
    Nullable(Box<Type>, Absent),
    // A type with how to recognise its values at runtime, or for a union, the
    // property that tells its members apart. Added last, by `lower::annotate_unions`.
    Tested(Box<Type>, Test),
    // Type-level expression; only the checker can evaluate it.
    Op(Box<Op>, Option<P<Node>>),
}

// How a JavaScript value is recognised as one member of a union.
#[derive(Clone, PartialEq)]
pub enum Test {
    TypeOf(&'static str),
    Array,
    InstanceOf(String),
    // An object that has this property and no other member does.
    Has(String),
    // The union's discriminant property has this value.
    Tag(String),
    // On a union: the property that tells its members apart.
    Discriminant(String),
}

// The value that stands for "no value". APIs that check `=== null` reject
// `undefined` and the reverse, so a binding must send the right one.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Absent {
    Null,
    Undefined,
    Either,
}

#[derive(Clone)]
pub enum Op {
    KeyOf(Type),
    IndexedAccess(Type, Type),
    Conditional { check: Type, extends: Type, yes: Type, no: Type },
    Mapped { readonly: Option<String>, optional: Option<String>, param: TypeParam, as_: Option<Type>, value: Option<Type> },
    Template(Vec<TemplatePart>),
    Infer(TypeParam),
    TypeOf(String),
    Predicate { param: String, asserts: bool, ty: Option<Type> },
    Unique(Type),
    Raw { kind: String, text: String },
}

#[derive(Clone)]
pub enum TemplatePart {
    Text(String),
    Type(Type),
}

#[derive(Clone)]
pub struct Ref {
    pub name: String,
    pub kind: &'static str,
    pub sym: Sym,
    pub args: Vec<Type>,
    // Checker identity of a type parameter: one symbol is shared by every
    // instantiation of a generic signature.
    pub tid: Option<u32>,
    // Fully qualified name and declaring file, when they add information.
    pub qualified: Option<String>,
    pub file: Option<String>,
    // Refers to a type-level alias (`Partial<T>`, `ReturnType<F>`).
    pub computed: bool,
    pub node: Option<P<Node>>,
}

#[derive(Clone)]
pub struct Element {
    pub name: Option<String>,
    pub optional: bool,
    pub rest: bool,
    pub ty: Type,
}

#[derive(Clone)]
pub struct TypeParam {
    pub name: String,
    pub sym: Sym,
    pub tid: Option<u32>,
    pub constraint: Option<Type>,
    pub default: Option<Type>,
}

#[derive(Clone)]
pub struct Param {
    pub name: String,
    pub optional: bool,
    pub rest: bool,
    // The one value this parameter can take, once its literal type is widened.
    pub constant: Option<String>,
    pub ty: Type,
}

#[derive(Clone, Default)]
pub struct Signature {
    pub type_params: Vec<TypeParam>,
    pub this: Option<Type>,
    pub params: Vec<Param>,
    pub ret: Option<Type>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MemberKind {
    Property,
    Method,
    Call,
    Construct,
    Index,
    Get,
    Set,
}

#[derive(Clone)]
pub struct Member {
    pub kind: MemberKind,
    pub name: String,
    // Real JS name, when a lowering renamed the member.
    pub js_name: Option<String>,
    pub optional: bool,
    pub readonly: bool,
    // Declaring interface, for inherited members after flattening.
    pub from: Option<String>,
    // Value of a property whose type was a single literal, once widened.
    pub constant: Option<String>,
    pub ty: Option<Type>,
    pub sig: Option<Signature>,
}

pub struct Interface {
    pub name: String,
    pub sym: Sym,
    pub type_params: Vec<TypeParam>,
    pub extends: Vec<Type>,
    // All transitive bases, kept after flattening for casts.
    pub ancestors: Vec<String>,
    pub members: Vec<Member>,
    // Set by class pairing.
    pub class: Option<Class>,
    pub synthetic: bool,
    // What its constructor is exported as, where not its name: `default`.
    pub js_name: Option<String>,
}

pub struct Class {
    // Kind `Construct`, named `new`.
    pub constructors: Vec<Member>,
    pub statics: Vec<Member>,
}

pub struct Alias {
    pub name: String,
    pub sym: Sym,
    pub type_params: Vec<TypeParam>,
    pub ty: Type,
    pub computed: bool,
    pub synthetic: bool,
}

pub struct Function {
    pub name: String,
    pub js_name: Option<String>,
    pub sig: Signature,
}

pub struct Var {
    pub name: String,
    pub ty: Type,
    // What it is exported as, where not its name: `default`.
    pub js_name: Option<String>,
    // `const`, or exported from a module: it cannot be assigned.
    pub readonly: bool,
}

pub struct EnumMember {
    pub name: String,
    pub kind: &'static str,
    pub value: String,
}

pub struct Enum {
    pub name: String,
    pub members: Vec<EnumMember>,
    pub synthetic: bool,
}

pub struct Namespace {
    pub name: String,
    // Its JavaScript name, when the output renamed it apart from another's.
    pub js_name: Option<String>,
    pub decls: Vec<Decl>,
}

pub enum Decl {
    Interface(Interface),
    Alias(Alias),
    Function(Function),
    Var(Var),
    Enum(Enum),
    Namespace(Namespace),
}

impl Type {
    pub fn prim(name: &str) -> Type {
        Type::Primitive(name.to_string())
    }
    // A reference to a declaration a lowering created.
    pub fn synthetic_ref(name: String, kind: &'static str, args: Vec<Type>) -> Type {
        Type::Ref(Ref { name, kind, sym: None, args, tid: None, qualified: None, file: None, computed: false, node: None })
    }
    pub fn is_prim(&self, name: &str) -> bool {
        matches!(self, Type::Primitive(n) if n == name)
    }
}

impl Member {
    pub fn new(kind: MemberKind, name: String) -> Member {
        Member { kind, name, js_name: None, optional: false, readonly: false, from: None, constant: None, ty: None, sig: None }
    }
}
