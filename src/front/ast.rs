use std::collections::HashSet;

use tsrs_ast::{self as ast, Kind, ModifierFlags, Node, NodeList, Symbol, SymbolFlags, has_syntactic_modifier};
use tsrs_checker::types::SignatureKind;
use tsrs_core::P;

use crate::ir::*;

use super::{
    declared_type_params, destructured_name, member_name, mentions_type_param, name_of,
    symbol_kind, text, Front,
};

impl<'c> Front<'c> {
    pub(super) fn interface(&mut self, node: P<Node>) -> Option<Interface> {
        let (sym, decls) = self.first_time(node)?;
        // `class X` and `interface X` merge: the class has the interface's members.
        if sym.declarations().iter().any(|d| d.kind() == Kind::ClassDeclaration) {
            return None;
        }
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
            name: self.output_name(sym),
            sym: Some(sym),
            type_params: self.type_params(declared_type_params(sym, node)),
            extends,
            ancestors: Vec::new(),
            ancestor_syms: Vec::new(),
            members,
            class: None,
            synthetic: false,
            js_name: self.renamed_from(sym),
        })
    }

    // `declare class X extends B implements I { ... }`: the interface of its
    // instances, with its constructors and static members. Private and
    // protected members are not what a caller sees.
    pub(super) fn class(&mut self, node: P<Node>) -> Option<Interface> {
        let (sym, _) = self.first_time(node)?;
        let mut extends = Vec::new();
        let mut keys = HashSet::new();
        for clause in node.as_class_declaration().heritage_clauses().iter().flat_map(|l| l.nodes()) {
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
        let mut members = Vec::new();
        let mut class = Class { constructors: Vec::new(), statics: Vec::new() };
        let mut declares_constructor = false;
        for &m in node.members() {
            let hidden = has_syntactic_modifier(m, ModifierFlags::Private) || has_syntactic_modifier(m, ModifierFlags::Protected);
            if m.kind() == Kind::Constructor {
                declares_constructor = true;
                if !hidden {
                    let mut c = Member::new(MemberKind::Construct, "new".into());
                    // The constructor's result and type parameters are the class's own.
                    c.sig = Some(Signature { type_params: Vec::new(), ret: None, ..self.signature(m) });
                    class.constructors.push(c);
                }
                continue;
            }
            if hidden || m.name().is_some_and(|n| n.kind() == Kind::PrivateIdentifier) {
                continue;
            }
            let kind = match m.kind() {
                Kind::PropertyDeclaration => MemberKind::Property,
                Kind::MethodDeclaration => MemberKind::Method,
                Kind::IndexSignature => MemberKind::Index,
                Kind::GetAccessor => MemberKind::Get,
                Kind::SetAccessor => MemberKind::Set,
                k => {
                    self.count(&format!("unhandled class member {k:?}"));
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
            let is_static = has_syntactic_modifier(m, ModifierFlags::Static);
            if keys.insert(format!("{is_static}{}", crate::xml::member_key(&member))) {
                if is_static { class.statics.push(member) } else { members.push(member) }
            }
        }
        // An interface of the same name adds to the class: its bases and members.
        let merged: Vec<P<Node>> = sym.declarations().iter().copied().filter(|d| d.kind() == Kind::InterfaceDeclaration).collect();
        for d in merged {
            for clause in d.as_interface_declaration().heritage_clauses().iter().flat_map(|l| l.nodes()) {
                for t in clause.as_heritage_clause().types().nodes() {
                    let base = if t.kind() == Kind::ExpressionWithTypeArguments { self.reference(t.expression().unwrap(), t.type_argument_list(), None) } else { self.ty(*t) };
                    if keys.insert(format!("extends {}", crate::xml::type_key(&base))) {
                        extends.push(base);
                    }
                }
            }
            for m in self.members(d.members()) {
                if keys.insert(format!("false{}", crate::xml::member_key(&m))) {
                    members.push(m);
                }
            }
        }
        // A class that declares no constructor has its base's, or one without
        // arguments: the checker knows which. An abstract class has none to call.
        if !declares_constructor && !has_syntactic_modifier(node, ModifierFlags::Abstract) {
            let t = self.c.get_type_of_symbol_exported(sym);
            for sig in self.c.get_signatures_of_type_exported(t, SignatureKind::Construct) {
                let mut c = Member::new(MemberKind::Construct, "new".into());
                c.sig = Some(Signature { type_params: Vec::new(), ret: None, ..self.from_signature(*sig) });
                class.constructors.push(c);
            }
        }
        if has_syntactic_modifier(node, ModifierFlags::Abstract) {
            class.constructors.clear();
        }
        Some(Interface {
            name: self.output_name(sym),
            sym: Some(sym),
            type_params: self.type_params(declared_type_params(sym, node)),
            extends,
            ancestors: Vec::new(),
            ancestor_syms: Vec::new(),
            members,
            class: Some(class),
            synthetic: false,
            js_name: self.renamed_from(sym),
        })
    }

    pub(super) fn enumeration(&mut self, node: P<Node>) -> Option<Enum> {
        let (sym, decls) = self.first_time(node)?;
        // Resolving the enum type computes member values.
        self.c.get_declared_type_of_symbol_exported(sym);
        let mut members = Vec::new();
        for d in decls {
            for m in d.members() {
                let (kind, value) = match self.c.get_constant_value(*m) {
                    Some(tsrs_checker::types::LiteralValue::String(s)) => ("string", s.to_string()),
                    Some(tsrs_checker::types::LiteralValue::Number(n)) => ("number", n.to_string()),
                    // Computed, non-constant member.
                    _ => ("computed", m.as_enum_member().initializer().map(text).unwrap_or_default()),
                };
                members.push(EnumMember { name: member_name(name_of(*m)), kind, value });
            }
        }
        Some(Enum { name: self.output_name(sym), sym: Some(sym), members, synthetic: false })
    }

    pub(super) fn namespace(&mut self, node: P<Node>) -> Option<Namespace> {
        let (sym, all_decls) = self.first_time(node)?;
        let fqn = self.c.get_fully_qualified_name_exported(sym);
        let included = self.is_included(&fqn);
        let mut decls = Vec::new();
        for d in all_decls {
            let keep = ast::get_source_file_of_node(d).is_some_and(|sf| {
                let name = sf.file_name();
                name == self.root_file || self.is_package_file(&sf) || (included && name.starts_with(&self.lib_path))
            });
            if !keep {
                continue;
            }
            match d.body() {
                Some(b) if b.kind() == Kind::ModuleDeclaration => {
                    if let Some(ns) = self.namespace(b) {
                        decls.push(Decl::Namespace(ns));
                    }
                }
                Some(b) => decls.extend(self.statements(b.statements())),
                None => {}
            }
        }
        Some(Namespace { name: self.output_name(sym), js_name: self.renamed_from(sym), decls })
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

    pub(super) fn type_params(&mut self, nodes: &[P<Node>]) -> Vec<TypeParam> {
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

    pub(super) fn signature(&mut self, node: P<Node>) -> Signature {
        let mut sig = Signature { type_params: self.type_params(node.type_parameters()), ..Default::default() };
        for &p in node.parameters() {
            let pd = p.as_parameter_declaration();
            let ty = self.opt_type(p.type_node());
            let name = match pd.name().kind() {
                Kind::ObjectBindingPattern | Kind::ArrayBindingPattern => destructured_name(&ty, sig.params.len()),
                _ => text(pd.name()),
            };
            // `this:` declares the receiver; it is not an argument.
            if name == "this" {
                sig.this = Some(ty);
                continue;
            }
            sig.params.push(Param { name, optional: pd.question_token().is_some(), rest: pd.dot_dot_dot_token().is_some(), constant: None, ty });
        }
        sig.ret = node.type_node().map(|r| self.ty(r));
        sig
    }

    pub(super) fn opt_type(&mut self, node: Option<P<Node>>) -> Type {
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
            // `import("./shapes").Circle<T>`: what it refers to, resolved by the checker.
            // Named (`import("./Shape").ShapeConfig`), as a reference written there:
            // the checker's type of it may carry the alias being declared.
            Kind::ImportType if !t.as_import_type_node().is_type_of() && t.as_import_type_node().qualifier().is_some() => {
                self.reference(t.as_import_type_node().qualifier().unwrap(), t.type_argument_list(), Some(t))
            }
            Kind::ImportType => {
                let resolved = self.c.get_type_from_type_node_exported(t);
                self.from_type(resolved)
            }
            k => {
                // `intrinsic` and anything else unhandled.
                self.count(&format!("raw {k:?}"));
                self.op(Op::Raw { kind: format!("{k:?}"), text: text(t) }, t)
            }
        }
    }

    // A named type, resolved by the checker to what it refers to.
    fn reference(&mut self, name: P<Node>, args: Option<P<NodeList>>, node: Option<P<Node>>) -> Type {
        let written = text(name);
        let raw_sym = self.c.get_symbol_at_location_exported(name);
        let was_alias = raw_sym.is_some_and(|s| s.flags().intersects(SymbolFlags::Alias));
        let sym = raw_sym.map(|s| self.resolve_alias(s));
        let args = args.map(|l| l.nodes()).unwrap_or(&[]);
        let args = self.types(args);
        let mut r = self.make_ref(sym, args);
        // Written qualified (`Geometry.Point`): its name as written. Not one the
        // output renamed so that it does not collide with another's.
        let renamed = r.sym.is_some_and(|s| self.output_names.get(&s).is_some_and(|n| *n != self.own_name(s)));
        if !was_alias && !renamed && !r.name.is_empty() && r.qualified.as_deref() == Some(written.as_str()) {
            r.name = written;
        }
        r.qualified = r.qualified.filter(|q| *q != r.name);
        r.node = node;
        Type::Ref(r)
    }

    pub(super) fn make_ref(&mut self, sym: Option<P<Symbol>>, mut args: Vec<Type>) -> Ref {
        let Some(s) = sym else {
            return Ref { name: String::new(), kind: "unresolved", sym: None, args, tid: None, qualified: None, file: None, computed: false, node: None };
        };
        let s = self.resolve_alias(s);
        // `Float32Array` means `Float32Array<ArrayBufferLike>`: spell out the
        // defaults, so every reference has all its type arguments, written or
        // made by the checker.
        let params = s
            .declarations()
            .iter()
            .filter(|d| matches!(d.kind(), Kind::InterfaceDeclaration | Kind::ClassDeclaration | Kind::TypeAliasDeclaration))
            .map(|d| d.type_parameters())
            .find(|p| !p.is_empty())
            .unwrap_or(&[]);
        if !s.flags().intersects(SymbolFlags::TypeParameter) {
            for p in params.iter().skip(args.len()) {
                let default = p.as_type_parameter_declaration().default_type().map(|d| self.ty(d));
                // A default may name an earlier parameter, which means nothing here.
                let usable = default.filter(|d| !mentions_type_param(d));
                args.push(usable.unwrap_or_else(|| Type::prim("any")));
            }
        }
        let name = self.output_name(s);
        let raw_fqn = self.c.get_fully_qualified_name_exported(s);
        let fqn = if raw_fqn.starts_with('"') {
            raw_fqn.find("\".").map(|i| raw_fqn[i + 2..].to_string()).unwrap_or(raw_fqn)
        } else {
            raw_fqn
        };
        // A path from the outermost namespace, by that namespace's name in the
        // output: two files may each declare a `Konva`.
        // (`global.NodeJS.Dict` in a `declare global`: `NodeJS.Dict`.)
        let fqn = {
            let mut top = s;
            let mut hops = 0;
            let mut in_global = false;
            while let Some(parent) = self.c.get_parent_of_symbol_exported(top) {
                if parent.declarations().iter().any(|d| d.kind() == Kind::ModuleDeclaration && d.name().is_some_and(|n| n.kind() != Kind::StringLiteral) && !ast::is_global_scope_augmentation(*d)) {
                    top = parent;
                    hops += 1;
                } else {
                    in_global = parent.declarations().iter().any(|d| ast::is_global_scope_augmentation(*d));
                    break;
                }
            }
            let segments: Vec<&str> = fqn.split('.').collect();
            if hops == 0 {
                if in_global { fqn.strip_prefix("global.").map(str::to_string).unwrap_or(fqn) } else { fqn }
            } else if segments.len() <= hops {
                fqn
            } else {
                format!("{}.{}", self.output_name(top), segments[segments.len() - hops..].join("."))
            }
        };
        let decl_file = s.declarations().first().and_then(|d| ast::get_source_file_of_node(*d));
        let in_pkg = decl_file.as_ref().is_some_and(|f| self.is_package_file(f));
        let included = self.is_included(&fqn) && decl_file.as_ref().is_some_and(|f| f.file_name().starts_with(&self.lib_path));
        if in_pkg {
            self.enqueue_symbol(s);
        }
        let file = decl_file
            .filter(|_| !in_pkg && !included)
            .map(|f| f.file_name().to_string())
            .filter(|f| *f != self.root_file)
            .map(|f| f.rsplit('/').next().unwrap_or(&f).to_string());
        Ref {
            kind: symbol_kind(s.flags()),
            computed: s.flags().intersects(SymbolFlags::TypeAlias) && self.is_computed_alias(s),
            // A path in a namespace (`Geometry.Point`); a top-level name has none.
            qualified: Some(fqn).filter(|q| *q != name && q.contains('.')),
            name,
            sym: Some(s),
            args,
            tid: None,
            file,
            node: None,
        }
    }

    // A generic alias whose body contains a type-level expression is a type
    // function: its uses are evaluated, and it has no declaration of its own.
    pub(super) fn is_computed_alias(&mut self, sym: P<Symbol>) -> bool {
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
}
