// Front end: tsrs AST -> IR (as written), and checker Type -> IR (resolved).

use std::collections::{BTreeMap, HashMap, HashSet};

use tsrs_ast::{self as ast, Kind, ModifierFlags, Node, NodeList, Symbol, SymbolFlags, has_syntactic_modifier};
use tsrs_checker::checker::Checker;
use tsrs_checker::types::{
    ElementFlags, LiteralValue, ObjectFlags, Signature as CSignature, SignatureKind, Type as CType, TypeFlags, TypeId,
};
use tsrs_core::P;
use tsrs_scanner::get_text_of_node;

use crate::ir::*;

pub struct Front<'c> {
    pub c: &'c mut Checker,
    pub root_file: String,
    // Merged symbols already converted, by kind and fully qualified name.
    seen: HashSet<String>,
    // Counts of lossy conversions and unhandled syntax, by reason.
    pub stats: BTreeMap<String, usize>,
    computed_alias: HashMap<P<Symbol>, bool>,
    // Anonymous types being converted; guards recursive types.
    in_progress: Vec<TypeId>,
    // Lowering: merge `A & B` into one object type.
    pub merge_intersections: bool,
    pub intersection_limit: usize,
    skip_alias: Option<P<Symbol>>,
}

const DEFERRED: TypeFlags = TypeFlags::Index
    .union(TypeFlags::IndexedAccess)
    .union(TypeFlags::Conditional)
    .union(TypeFlags::Substitution)
    .union(TypeFlags::TemplateLiteral)
    .union(TypeFlags::StringMapping);

impl<'c> Front<'c> {
    pub fn new(c: &'c mut Checker, root_file: String) -> Self {
        Front { c, root_file, seen: HashSet::new(), stats: BTreeMap::new(), computed_alias: HashMap::new(), in_progress: Vec::new(), merge_intersections: false, intersection_limit: 64, skip_alias: None }
    }

    pub fn count(&mut self, what: &str) {
        *self.stats.entry(what.to_string()).or_default() += 1;
    }

    fn erased(&mut self, why: &'static str) -> Type {
        self.count(&format!("erased: {why}"));
        Type::Erased(why)
    }

    // ---- AST -> IR ----

    // A merged symbol is converted once, from all its declarations.
    fn first_time(&mut self, node: P<Node>) -> Option<(P<Symbol>, Vec<P<Node>>)> {
        let sym = self.c.get_symbol_of_declaration_exported(node)?;
        let sym = self.c.get_merged_symbol_exported(sym);
        let fqn = self.c.get_fully_qualified_name_exported(sym);
        let kind = node.kind();
        if !self.seen.insert(format!("{kind:?}:{fqn}")) {
            return None;
        }
        Some((sym, sym.declarations().iter().copied().filter(|d| d.kind() == kind).collect()))
    }

    pub fn statements(&mut self, stmts: &[P<Node>]) -> Vec<Decl> {
        let mut out = Vec::new();
        for &s in stmts {
            match s.kind() {
                Kind::InterfaceDeclaration => out.extend(self.interface(s).map(Decl::Interface)),
                Kind::TypeAliasDeclaration => {
                    let sym = self.c.get_symbol_of_declaration_exported(s);
                    out.push(Decl::Alias(Alias {
                        name: text(name_of(s)),
                        sym,
                        type_params: self.type_params(s.type_parameters()),
                        ty: self.opt_type(s.type_node()),
                        computed: sym.is_some_and(|sym| self.is_computed_alias(sym)),
                        synthetic: false,
                    }));
                }
                Kind::FunctionDeclaration => {
                    out.push(Decl::Function(Function { name: text(name_of(s)), js_name: None, sig: self.signature(s) }))
                }
                Kind::EnumDeclaration => out.extend(self.enumeration(s).map(Decl::Enum)),
                Kind::ModuleDeclaration => out.push(Decl::Namespace(self.namespace(s))),
                Kind::VariableStatement => {
                    for d in s.as_variable_statement().declaration_list().as_variable_declaration_list().declarations().nodes() {
                        out.push(Decl::Var(Var { name: text(d.as_variable_declaration().name()), ty: self.opt_type(d.type_node()) }));
                    }
                }
                _ => {}
            }
        }
        out
    }

    fn interface(&mut self, node: P<Node>) -> Option<Interface> {
        let (sym, decls) = self.first_time(node)?;
        let mut extends = Vec::new();
        let mut members: Vec<Member> = Vec::new();
        let mut keys = HashSet::new();
        for d in &decls {
            for clause in d.as_interface_declaration().heritage_clauses().iter().flat_map(|l| l.nodes()) {
                for t in clause.as_heritage_clause().types().nodes() {
                    let base = if t.kind() == Kind::ExpressionWithTypeArguments {
                        self.reference(t.expression().unwrap(), t.type_argument_list(), None)
                    } else {
                        self.ty(*t)
                    };
                    if keys.insert(format!("extends {}", crate::xml::type_key(&base))) {
                        extends.push(base);
                    }
                }
            }
            // Merged declarations may repeat a member.
            for m in self.members(d.members()) {
                if keys.insert(crate::xml::member_key(&m)) {
                    members.push(m);
                }
            }
        }
        Some(Interface {
            name: text(name_of(node)),
            sym: Some(sym),
            type_params: self.type_params(node.type_parameters()),
            extends,
            ancestors: Vec::new(),
            members,
            class: None,
            synthetic: false,
        })
    }

    fn enumeration(&mut self, node: P<Node>) -> Option<Enum> {
        let (sym, decls) = self.first_time(node)?;
        // Resolving the enum type computes member values.
        self.c.get_declared_type_of_symbol_exported(sym);
        let mut members = Vec::new();
        for d in decls {
            for m in d.members() {
                let (kind, value) = match self.c.get_constant_value(*m) {
                    Some(LiteralValue::String(s)) => ("string", s.to_string()),
                    Some(LiteralValue::Number(n)) => ("number", n.to_string()),
                    // Computed, non-constant member.
                    _ => ("computed", m.as_enum_member().initializer().map(text).unwrap_or_default()),
                };
                members.push(EnumMember { name: member_name(name_of(*m)), kind, value });
            }
        }
        Some(Enum { name: text(name_of(node)), members, synthetic: false })
    }

    fn namespace(&mut self, node: P<Node>) -> Namespace {
        let decls = match node.body() {
            Some(b) if b.kind() == Kind::ModuleDeclaration => vec![Decl::Namespace(self.namespace(b))],
            Some(b) => self.statements(b.statements()),
            None => Vec::new(),
        };
        Namespace { name: text(name_of(node)), decls }
    }

    fn members(&mut self, nodes: &[P<Node>]) -> Vec<Member> {
        let mut out = Vec::new();
        for &m in nodes {
            let kind = match m.kind() {
                Kind::PropertySignature => MemberKind::Property,
                Kind::MethodSignature => MemberKind::Method,
                Kind::CallSignature => MemberKind::Call,
                Kind::ConstructSignature => MemberKind::Construct,
                Kind::IndexSignature => MemberKind::Index,
                Kind::GetAccessor => MemberKind::Get,
                Kind::SetAccessor => MemberKind::Set,
                k => {
                    self.count(&format!("unhandled member {k:?}"));
                    continue;
                }
            };
            let mut member = Member::new(kind, m.name().map(member_name).unwrap_or_default());
            member.optional = m.question_token().is_some();
            member.readonly = has_syntactic_modifier(m, ModifierFlags::Readonly);
            if kind == MemberKind::Property {
                member.ty = Some(self.opt_type(m.type_node()));
            } else {
                member.sig = Some(self.signature(m));
            }
            out.push(member);
        }
        out
    }

    fn type_params(&mut self, nodes: &[P<Node>]) -> Vec<TypeParam> {
        nodes.iter().map(|&p| self.type_param(p)).collect()
    }

    fn type_param(&mut self, p: P<Node>) -> TypeParam {
        let tp = p.as_type_parameter_declaration();
        TypeParam {
            name: text(tp.name()),
            sym: self.c.get_symbol_of_declaration_exported(p),
            tid: None,
            constraint: tp.constraint().map(|k| self.ty(k)),
            default: tp.default_type().map(|d| self.ty(d)),
        }
    }

    fn signature(&mut self, node: P<Node>) -> Signature {
        let mut sig = Signature { type_params: self.type_params(node.type_parameters()), ..Default::default() };
        for &p in node.parameters() {
            let pd = p.as_parameter_declaration();
            let name = text(pd.name());
            let ty = self.opt_type(p.type_node());
            // `this:` declares the receiver; it is not an argument.
            if name == "this" {
                sig.this = Some(ty);
                continue;
            }
            sig.params.push(Param { name, optional: pd.question_token().is_some(), rest: pd.dot_dot_dot_token().is_some(), ty });
        }
        sig.ret = node.type_node().map(|r| self.ty(r));
        sig
    }

    fn opt_type(&mut self, node: Option<P<Node>>) -> Type {
        match node {
            Some(t) => self.ty(t),
            None => Type::prim("any"),
        }
    }

    fn types(&mut self, nodes: &[P<Node>]) -> Vec<Type> {
        nodes.iter().map(|&n| self.ty(n)).collect()
    }

    fn op(&mut self, op: Op, node: P<Node>) -> Type {
        Type::Op(Box::new(op), Some(node))
    }

    pub fn ty(&mut self, t: P<Node>) -> Type {
        match t.kind() {
            Kind::AnyKeyword | Kind::UnknownKeyword | Kind::StringKeyword | Kind::NumberKeyword
            | Kind::BigIntKeyword | Kind::BooleanKeyword | Kind::SymbolKeyword | Kind::ObjectKeyword
            | Kind::VoidKeyword | Kind::UndefinedKeyword | Kind::NeverKeyword | Kind::NullKeyword => Type::Primitive(text(t)),
            Kind::ThisType => Type::This,
            Kind::LiteralType => {
                let lit = t.as_literal_type_node().literal();
                match lit.kind() {
                    Kind::NullKeyword => Type::prim("null"),
                    Kind::TrueKeyword | Kind::FalseKeyword => Type::Literal { kind: "boolean", value: text(lit) },
                    Kind::StringLiteral | Kind::NoSubstitutionTemplateLiteral => Type::Literal { kind: "string", value: lit.text().to_string() },
                    Kind::BigIntLiteral => Type::Literal { kind: "bigint", value: text(lit) },
                    _ => Type::Literal { kind: "number", value: text(lit) },
                }
            }
            Kind::ParenthesizedType => self.ty(t.type_node().unwrap()),
            Kind::TypeReference => {
                let r = t.as_type_reference_node();
                self.reference(r.type_name(), r.type_arguments(), Some(t))
            }
            Kind::UnionType => Type::Union(self.types(t.as_union_type_node().types().nodes())),
            Kind::IntersectionType => Type::Intersection(self.types(t.as_intersection_type_node().types().nodes()), Some(t)),
            Kind::ArrayType => Type::Array(Box::new(self.ty(t.as_array_type_node().element_type()))),
            Kind::TupleType => {
                let mut els = Vec::new();
                for &el in t.as_tuple_type_node().elements().nodes() {
                    els.push(match el.kind() {
                        Kind::NamedTupleMember => {
                            let m = el.as_named_tuple_member();
                            Element { name: Some(text(m.name())), optional: m.question_token().is_some(), rest: m.dot_dot_dot_token().is_some(), ty: self.ty(m.type_()) }
                        }
                        Kind::OptionalType => Element { name: None, optional: true, rest: false, ty: self.ty(el.type_node().unwrap()) },
                        Kind::RestType => Element { name: None, optional: false, rest: true, ty: self.ty(el.type_node().unwrap()) },
                        _ => Element { name: None, optional: false, rest: false, ty: self.ty(el) },
                    });
                }
                Type::Tuple(els)
            }
            Kind::FunctionType => Type::Function(Box::new(self.signature(t))),
            Kind::ConstructorType => Type::Constructor(Box::new(self.signature(t))),
            Kind::TypeLiteral => Type::Object(self.members(t.members())),
            Kind::TypeOperator => {
                let o = t.as_type_operator_node();
                let inner = self.ty(o.type_());
                match o.operator() {
                    Kind::KeyOfKeyword => self.op(Op::KeyOf(inner), t),
                    Kind::ReadonlyKeyword => Type::Readonly(Box::new(inner)),
                    _ => self.op(Op::Unique(inner), t),
                }
            }
            Kind::IndexedAccessType => {
                let ia = t.as_indexed_access_type_node();
                let op = Op::IndexedAccess(self.ty(ia.object_type()), self.ty(ia.index_type()));
                self.op(op, t)
            }
            Kind::TypeQuery => self.op(Op::TypeOf(text(t.as_type_query_node().expr_name())), t),
            Kind::TypePredicate => {
                let p = t.as_type_predicate_node();
                let op = Op::Predicate { param: text(p.parameter_name()), asserts: p.asserts_modifier().is_some(), ty: p.type_().map(|pt| self.ty(pt)) };
                self.op(op, t)
            }
            Kind::TemplateLiteralType => {
                let tl = t.as_template_literal_type_node();
                let mut parts = vec![TemplatePart::Text(tl.head().text().to_string())];
                for &s in tl.template_spans().nodes() {
                    let span = s.as_template_literal_type_span();
                    parts.push(TemplatePart::Type(self.ty(span.type_())));
                    parts.push(TemplatePart::Text(span.literal().text().to_string()));
                }
                self.op(Op::Template(parts), t)
            }
            Kind::ConditionalType => {
                let ct = t.as_conditional_type_node();
                let op = Op::Conditional {
                    check: self.ty(ct.check_type()),
                    extends: self.ty(ct.extends_type()),
                    yes: self.ty(ct.true_type()),
                    no: self.ty(ct.false_type()),
                };
                self.op(op, t)
            }
            Kind::InferType => {
                let op = Op::Infer(self.type_param(t.as_infer_type_node().type_parameter()));
                self.op(op, t)
            }
            Kind::MappedType => {
                let mt = t.as_mapped_type_node();
                // `+`/`-` modifiers are kept as written.
                let op = Op::Mapped {
                    readonly: mt.readonly_token().map(text),
                    optional: mt.question_token().map(text),
                    param: self.type_param(mt.type_parameter()),
                    as_: mt.name_type().map(|n| self.ty(n)),
                    value: mt.type_().map(|v| self.ty(v)),
                };
                self.op(op, t)
            }
            k => {
                // `intrinsic`, import types and anything else unhandled.
                self.count(&format!("raw {k:?}"));
                self.op(Op::Raw { kind: format!("{k:?}"), text: text(t) }, t)
            }
        }
    }

    // A named type, resolved by the checker to what it refers to.
    fn reference(&mut self, name: P<Node>, args: Option<P<NodeList>>, node: Option<P<Node>>) -> Type {
        let written = text(name);
        let sym = self.c.get_symbol_at_location_exported(name);
        let args = args.map(|l| l.nodes()).unwrap_or(&[]);
        let args = self.types(args);
        let mut r = self.make_ref(sym, args);
        r.name = written;
        r.qualified = r.qualified.filter(|q| *q != r.name);
        r.node = node;
        Type::Ref(r)
    }

    fn make_ref(&mut self, sym: Option<P<Symbol>>, args: Vec<Type>) -> Ref {
        let Some(s) = sym else {
            return Ref { name: String::new(), kind: "unresolved", sym: None, args, tid: None, qualified: None, file: None, computed: false, node: None };
        };
        let name = s.name().to_string();
        let fqn = self.c.get_fully_qualified_name_exported(s);
        let file = s
            .declarations()
            .first()
            .and_then(|d| ast::get_source_file_of_node(*d))
            .map(|f| f.file_name().to_string())
            .filter(|f| *f != self.root_file)
            .map(|f| f.rsplit('/').next().unwrap_or(&f).to_string());
        Ref {
            kind: symbol_kind(s.flags()),
            computed: s.flags().intersects(SymbolFlags::TypeAlias) && self.is_computed_alias(s),
            qualified: Some(fqn).filter(|q| *q != name),
            name,
            sym,
            args,
            tid: None,
            file,
            node: None,
        }
    }

    // A generic alias whose body contains a type-level expression is a type
    // function: its uses are evaluated, and it has no declaration of its own.
    fn is_computed_alias(&mut self, sym: P<Symbol>) -> bool {
        if let Some(&known) = self.computed_alias.get(&sym) {
            return known;
        }
        self.computed_alias.insert(sym, false);
        let decl = sym.declarations().iter().copied().find(|d| d.kind() == Kind::TypeAliasDeclaration);
        let computed = decl.is_some_and(|d| {
            let generic = !d.type_parameters().is_empty();
            let body = d.type_node();
            let intrinsic = body.is_some_and(|b| b.kind() == Kind::IntrinsicKeyword);
            intrinsic || generic && body.is_some_and(|b| self.contains_op(b))
        });
        self.computed_alias.insert(sym, computed);
        computed
    }

    fn contains_op(&mut self, node: P<Node>) -> bool {
        match node.kind() {
            Kind::ConditionalType | Kind::MappedType | Kind::IndexedAccessType | Kind::TemplateLiteralType | Kind::InferType => return true,
            Kind::TypeOperator if node.as_type_operator_node().operator() == Kind::KeyOfKeyword => return true,
            Kind::TypeReference => {
                let name = node.as_type_reference_node().type_name();
                if let Some(s) = self.c.get_symbol_at_location_exported(name)
                    && s.flags().intersects(SymbolFlags::TypeAlias)
                    && self.is_computed_alias(s)
                {
                    return true;
                }
            }
            _ => {}
        }
        let mut children = Vec::new();
        node.for_each_child(&mut |c| {
            children.push(c);
            false
        });
        children.into_iter().any(|c| self.contains_op(c))
    }

    // ---- checker Type -> IR ----

    // Evaluates a type-level expression at its source location.
    pub fn eval(&mut self, node: P<Node>) -> Type {
        let t = self.c.get_type_from_type_node_exported(node);
        self.from_type(t)
    }

    // Evaluates an alias body without folding the result back into the alias.
    pub fn eval_alias_body(&mut self, node: P<Node>, alias: P<Symbol>) -> Type {
        let t = self.c.get_type_from_type_node_exported(node);
        self.skip_alias = Some(alias);
        self.from_type(t)
    }

    pub fn from_type(&mut self, t: P<CType>) -> Type {
        let f = t.flags();
        let skip = self.skip_alias.take();
        // Named aliases are kept as references unless they are type functions.
        if let Some(alias) = t.alias()
            && let Some(s) = alias.symbol()
            && Some(s) != skip
            && !self.is_computed_alias(s)
        {
            let args = alias.type_arguments().iter().map(|a| self.from_type(*a)).collect();
            return Type::Ref(self.make_ref(Some(s), args));
        }
        if f.intersects(TypeFlags::Boolean) {
            return Type::prim("boolean");
        }
        if f.intersects(TypeFlags::EnumLiteral | TypeFlags::Enum)
            && let Some(s) = t.symbol()
        {
            // Enum members refer to their enum.
            let s = if s.flags().intersects(SymbolFlags::EnumMember) { self.c.get_parent_of_symbol_exported(s).unwrap_or(s) } else { s };
            return Type::Ref(self.make_ref(Some(s), Vec::new()));
        }
        if f.intersects(TypeFlags::StringLiteral | TypeFlags::NumberLiteral | TypeFlags::BigIntLiteral | TypeFlags::BooleanLiteral) {
            return match t.as_literal_type().value() {
                Some(LiteralValue::String(s)) => Type::Literal { kind: "string", value: s.to_string() },
                Some(LiteralValue::Number(n)) => Type::Literal { kind: "number", value: n.to_string() },
                Some(LiteralValue::Boolean(b)) => Type::Literal { kind: "boolean", value: b.to_string() },
                _ => Type::prim("bigint"),
            };
        }
        if f.intersects(TypeFlags::UniqueESSymbol) {
            return Type::prim("symbol");
        }
        if f.intersects(
            TypeFlags::Any | TypeFlags::Unknown | TypeFlags::String | TypeFlags::Number | TypeFlags::BigInt | TypeFlags::ESSymbol
                | TypeFlags::Void | TypeFlags::Undefined | TypeFlags::Null | TypeFlags::Never | TypeFlags::NonPrimitive,
        ) {
            return Type::prim(t.as_intrinsic_type().intrinsic_name());
        }
        if f.intersects(TypeFlags::TypeParameter) {
            if t.as_type_parameter().is_this_type() {
                return Type::This;
            }
            return match t.symbol() {
                Some(s) => {
                    let mut r = self.make_ref(Some(s), Vec::new());
                    r.tid = Some(t.id().0);
                    Type::Ref(r)
                }
                None => self.erased("anonymous type parameter"),
            };
        }
        if f.intersects(TypeFlags::Union) {
            // Widening can make members equal.
            let mut seen = HashSet::new();
            let mut parts: Vec<Type> = t.types().iter().map(|u| self.from_type(*u)).collect();
            parts.retain(|p| seen.insert(crate::xml::type_key(p)));
            return if parts.len() == 1 { parts.pop().unwrap() } else { Type::Union(parts) };
        }
        if f.intersects(TypeFlags::Intersection) {
            if !self.merge_intersections {
                return Type::Intersection(t.types().iter().map(|u| self.from_type(*u)).collect(), None);
            }
            // Branded primitives (`string & { __brand }`) are the primitive.
            let primitive = TypeFlags::String | TypeFlags::Number | TypeFlags::BigInt | TypeFlags::Boolean | TypeFlags::StringLiteral | TypeFlags::NumberLiteral;
            if let Some(p) = t.types().iter().find(|p| p.flags().intersects(primitive)) {
                self.count("widened: branded primitive");
                return self.from_type(*p);
            }
            let mergeable = t.types().iter().all(|p| p.flags().intersects(TypeFlags::Object));
            if mergeable && self.c.get_properties_of_type_exported(t).len() <= self.intersection_limit {
                return Type::Object(self.members_of_type(t, None));
            }
            // Too large to inline (`Window & typeof globalThis`): keep the named part.
            let named = t.types().iter().find(|p| p.flags().intersects(TypeFlags::Object) && p.object_flags().intersects(ObjectFlags::ClassOrInterface | ObjectFlags::Reference));
            if let Some(p) = named {
                self.count("widened: intersection -> named member");
                return self.from_type(*p);
            }
            return self.erased("intersection");
        }
        if f.intersects(DEFERRED) {
            // Depends on a type parameter: widen to what it is constrained to.
            return match self.c.get_base_constraint_of_type_exported(t) {
                Some(k) if k != t && !k.flags().intersects(DEFERRED) => {
                    self.count("widened: generic type-level expression -> constraint");
                    self.from_type(k)
                }
                _ if f.intersects(TypeFlags::TemplateLiteral | TypeFlags::StringMapping) => {
                    self.count("widened: template literal -> string");
                    Type::prim("string")
                }
                _ => self.erased("generic type-level expression"),
            };
        }
        if f.intersects(TypeFlags::Object) {
            return self.from_object(t);
        }
        self.erased("unknown type kind")
    }

    fn from_object(&mut self, t: P<CType>) -> Type {
        let of = t.object_flags();
        if of.intersects(ObjectFlags::Reference)
            && let Some(target) = t.target()
        {
            let args = self.c.get_type_arguments_exported(t);
            if self.c.is_array_type_exported(t) && !args.is_empty() {
                return Type::Array(Box::new(self.from_type(args[0])));
            }
            if target.object_flags().intersects(ObjectFlags::Tuple) {
                let infos = target.as_tuple_type().element_infos();
                let mut els = Vec::new();
                for (info, arg) in infos.iter().zip(args) {
                    let ef = info.tuple_element_flags();
                    els.push(Element {
                        name: info.labeled_declaration().and_then(|d| d.name()).map(text),
                        optional: ef.intersects(ElementFlags::Optional),
                        rest: ef.intersects(ElementFlags::Variable),
                        // As written: `...rest: T[]`.
                        ty: if ef.intersects(ElementFlags::Rest) { Type::Array(Box::new(self.from_type(*arg))) } else { self.from_type(*arg) },
                    });
                }
                return Type::Tuple(els);
            }
            let n = target.try_as_interface_type().map(|i| i.local_type_parameters().len()).unwrap_or(args.len());
            let args = args.iter().take(n).map(|a| self.from_type(*a)).collect();
            return Type::Ref(self.make_ref(target.symbol(), args));
        }
        if of.intersects(ObjectFlags::ClassOrInterface) {
            // Uninstantiated generic: its own type parameters are the arguments.
            let args = t.try_as_interface_type().map(|i| i.local_type_parameters()).unwrap_or(&[]);
            let args = args.iter().map(|a| self.from_type(*a)).collect();
            return Type::Ref(self.make_ref(t.symbol(), args));
        }
        // Anonymous or mapped: structural.
        if let Some(s) = t.symbol()
            && s.flags().intersects(SymbolFlags::Module | SymbolFlags::Class | SymbolFlags::Enum)
        {
            return self.erased("typeof namespace/class/enum");
        }
        if self.in_progress.contains(&t.id()) || self.in_progress.len() > 12 {
            return self.erased("recursive anonymous type");
        }
        self.in_progress.push(t.id());
        let members = self.members_of_type(t, None);
        self.in_progress.pop();
        if members.is_empty() && of.intersects(ObjectFlags::Mapped) {
            return self.erased("generic mapped type");
        }
        // `(a) => b` and `new (a) => b` are objects with a single signature.
        if let [m] = members.as_slice()
            && matches!(m.kind, MemberKind::Call | MemberKind::Construct)
        {
            let sig = Box::new(m.sig.clone().unwrap());
            return if m.kind == MemberKind::Call { Type::Function(sig) } else { Type::Constructor(sig) };
        }
        Type::Object(members)
    }

    // All members of a type, inherited ones included, as the checker resolves them.
    pub fn members_of_type(&mut self, t: P<CType>, owner: Option<P<Symbol>>) -> Vec<Member> {
        let mut out = Vec::new();
        for (kind, mk) in [(SignatureKind::Call, MemberKind::Call), (SignatureKind::Construct, MemberKind::Construct)] {
            for sig in self.c.get_signatures_of_type_exported(t, kind) {
                let mut m = Member::new(mk, String::new());
                m.sig = Some(self.from_signature(*sig));
                out.push(m);
            }
        }
        for info in self.c.get_index_infos_of_type_exported(t) {
            let mut m = Member::new(MemberKind::Index, String::new());
            m.readonly = info.is_readonly();
            let key = Param { name: "key".into(), optional: false, rest: false, ty: self.from_type(info.key_type()) };
            m.sig = Some(Signature { params: vec![key], ret: Some(self.from_type(info.value_type())), ..Default::default() });
            out.push(m);
        }
        for prop in self.c.get_properties_of_type_exported(t) {
            let name = symbol_member_name(*prop);
            let pty = self.c.get_type_of_symbol_exported(*prop);
            let from = match (owner, self.c.get_parent_of_symbol_exported(*prop)) {
                (Some(o), Some(p)) if p != o => Some(p.name().to_string()),
                _ => None,
            };
            if prop.flags().intersects(SymbolFlags::Method) {
                for sig in self.c.get_signatures_of_type_exported(pty, SignatureKind::Call) {
                    let mut m = Member::new(MemberKind::Method, name.clone());
                    m.optional = prop.flags().intersects(SymbolFlags::Optional);
                    m.from = from.clone();
                    m.sig = Some(self.from_signature(*sig));
                    out.push(m);
                }
                continue;
            }
            let mut m = Member::new(MemberKind::Property, name);
            m.optional = prop.flags().intersects(SymbolFlags::Optional);
            m.readonly = self.c.is_readonly_symbol_exported(*prop);
            m.from = from;
            let mut ty = self.from_type(pty);
            if m.optional {
                ty = strip_undefined(ty);
            }
            m.ty = Some(ty);
            out.push(m);
        }
        out
    }

    fn from_signature(&mut self, sig: P<CSignature>) -> Signature {
        let mut out = Signature::default();
        for tp in sig.type_parameters() {
            out.type_params.push(TypeParam {
                name: tp.symbol().map(|s| s.name().to_string()).unwrap_or_default(),
                sym: tp.symbol(),
                tid: Some(tp.id().0),
                constraint: self.c.get_constraint_of_type_parameter_exported(*tp).map(|k| self.from_type(k)),
                default: None,
            });
        }
        if let Some(this) = sig.this_parameter() {
            let t = self.c.get_type_of_symbol_exported(this);
            out.this = Some(self.from_type(t));
        }
        let params = sig.parameters();
        for (i, p) in params.iter().enumerate() {
            let rest = sig.has_rest_parameter() && i == params.len() - 1;
            let optional = !rest && i >= sig.min_argument_count();
            let t = self.c.get_type_of_symbol_exported(*p);
            let mut ty = self.from_type(t);
            if optional {
                ty = strip_undefined(ty);
            }
            out.params.push(Param { name: p.name().to_string(), optional, rest, ty });
        }
        let ret = self.c.get_return_type_of_signature_exported(sig);
        out.ret = Some(self.from_type(ret));
        out
    }

    // The member of a union of named types that every other member is
    // assignable to, if there is one.
    pub fn common_supertype(&mut self, parts: &[Type]) -> Option<Type> {
        let mut types = Vec::new();
        for p in parts {
            let Type::Ref(r) = p else { return None };
            if !matches!(r.kind, "interface" | "class") || !r.args.is_empty() {
                return None;
            }
            types.push(self.c.get_declared_type_of_symbol_exported(r.sym?));
        }
        let top = (0..types.len()).find(|&i| (0..types.len()).all(|j| i == j || self.c.is_type_assignable_to_exported(types[j], types[i])))?;
        Some(parts[top].clone())
    }

    // Own and inherited members of an interface, with base type arguments applied.
    pub fn flattened(&mut self, sym: P<Symbol>) -> (Vec<Member>, Vec<String>) {
        let t = self.c.get_declared_type_of_symbol_exported(sym);
        let members = self.members_of_type(t, Some(sym));
        let mut ancestors = Vec::new();
        let mut queue = vec![t];
        while let Some(t) = queue.pop() {
            for base in self.c.get_base_types_exported(t) {
                let name = base.symbol().map(|s| s.name().to_string()).unwrap_or_default();
                if !name.is_empty() && !ancestors.contains(&name) {
                    ancestors.push(name);
                    queue.push(*base);
                }
            }
        }
        (members, ancestors)
    }
}

fn strip_undefined(ty: Type) -> Type {
    match ty {
        Type::Union(parts) => {
            let mut parts: Vec<Type> = parts.into_iter().filter(|p| !p.is_prim("undefined")).collect();
            if parts.len() == 1 { parts.pop().unwrap() } else { Type::Union(parts) }
        }
        t => t,
    }
}

fn symbol_kind(f: SymbolFlags) -> &'static str {
    if f.intersects(SymbolFlags::TypeParameter) {
        "typeParameter"
    } else if f.intersects(SymbolFlags::Class) {
        "class"
    } else if f.intersects(SymbolFlags::Interface) {
        "interface"
    } else if f.intersects(SymbolFlags::TypeAlias) {
        "typeAlias"
    } else if f.intersects(SymbolFlags::Enum) {
        "enum"
    } else if f.intersects(SymbolFlags::Alias) {
        "import"
    } else {
        "value"
    }
}

fn name_of(node: P<Node>) -> P<Node> {
    node.name().expect("named declaration")
}

// `[Symbol.iterator]` stays as written; quoted names are unquoted.
fn member_name(name: P<Node>) -> String {
    match name.kind() {
        Kind::StringLiteral | Kind::NumericLiteral => name.text().to_string(),
        _ => text(name),
    }
}

// Symbol-keyed members have internal names like `\x7f@iterator@123`.
fn symbol_member_name(sym: P<Symbol>) -> String {
    match sym.name().strip_prefix("\u{7f}@").and_then(|s| s.split('@').next()) {
        Some(well_known) => format!("[Symbol.{well_known}]"),
        None => sym.name().to_string(),
    }
}

fn text(node: P<Node>) -> String {
    get_text_of_node(node)
}
