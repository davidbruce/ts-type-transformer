use std::collections::HashSet;

use tsrs_ast::{self as ast, Kind, Node, Symbol, SymbolFlags};
use tsrs_checker::checker::Checker;
use tsrs_checker::types::{
    ElementFlags, LiteralValue, MappedType, ObjectFlags, Signature as CSignature, SignatureKind,
    Type as CType, TypeFlags, TypeId,
};
use tsrs_core::P;

use crate::ir::*;

use super::{
    destructured_name, strip_undefined, symbol_member_name, text, Front, BUDGET, DEFERRED,
};

impl<'c> Front<'c> {
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

    // A type the checker made, as the IR has it. Types that keep making new
    // ones (`DeepKeys<T>` through every property of every property) stop, at
    // a depth or once one conversion has made too many, as an `any` that
    // says so.
    #[allow(clippy::wrong_self_convention)]
    pub fn from_type(&mut self, t: P<CType>) -> Type {
        if self.depth == 0 {
            self.budget = BUDGET;
        }
        if self.depth > 48 {
            self.cut += 1;
            self.count("erased: type too deep");
            return Type::Erased("type too deep");
        }
        if self.budget == 0 {
            self.cut += 1;
            self.count("erased: type too large");
            return Type::Erased("type too large");
        }
        // A type converts the same wherever it is, but for a key being
        // expanded or an alias's own body: each once, and again from memory.
        let cacheable = self.key_binding.is_none() && self.skip_alias.is_none();
        if cacheable && let Some((ty, size)) = self.converted.get(&t.id()) {
            if *size > self.budget {
                self.cut += 1;
                self.count("erased: type too large");
                return Type::Erased("type too large");
            }
            self.budget -= size;
            return ty.clone();
        }
        let (budget, cut) = (self.budget, self.cut);
        self.budget -= 1;
        self.depth += 1;
        let out = self.from_type_at(t);
        self.depth -= 1;
        // Not what a limit cut short here: elsewhere it may fit.
        if cacheable && self.cut == cut {
            self.converted.insert(t.id(), (out.clone(), budget - self.budget));
        }
        out
    }

    #[allow(clippy::wrong_self_convention)]
    fn from_type_at(&mut self, t: P<CType>) -> Type {
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
            // The checker's error type is what it could not resolve: not a primitive.
            let name = t.as_intrinsic_type().intrinsic_name();
            return if name == "error" { Type::Erased("unresolved") } else { Type::prim(name) };
        }
        // A type parameter being expanded is its key, and `M[K]` the type of that property.
        if let Some((tp, key)) = self.key_binding.clone() {
            if t.id() == tp {
                return Type::Literal { kind: "string", value: key };
            }
            if f.intersects(TypeFlags::IndexedAccess) {
                let ia = t.as_indexed_access_type();
                if ia.index_type().is_some_and(|i| i.id() == tp)
                    && let Some(object) = ia.object_type()
                {
                    return match self.c.get_type_of_property_of_type_exported(object, &key) {
                        Some(p) => self.from_type(p),
                        None => self.erased("missing key"),
                    };
                }
            }
        }
        // `M[K]` for a kept keyed type parameter stays as written.
        if f.intersects(TypeFlags::IndexedAccess) {
            let ia = t.as_indexed_access_type();
            if let (Some(i), Some(object)) = (ia.index_type(), ia.object_type())
                && self.kept_keyed.contains(&i.id())
            {
                let map = self.from_type(object);
                let key = self.from_type(i);
                return Type::Op(Box::new(Op::IndexedAccess(map, key)), None);
            }
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
            // An array with nothing more of its own (`Array<T> & { [k: string]:
            // never }`, `T[] & [...U]`, an array and a class) is the array:
            // merged, Array.prototype's members would be an object's. One with
            // an object literal's properties (marked's `Token[] & { links }`) is
            // merged, for them; unless the checker made it, as the constraint
            // of eventemitter3's `ArgumentMap<T>[K]` (each event's arguments).
            let is_array = |c: &mut Checker, p: P<CType>| {
                c.is_array_type_exported(p)
                    || (p.object_flags().intersects(ObjectFlags::Reference) && p.target().is_some_and(|g| g.object_flags().intersects(ObjectFlags::Tuple)))
                    || ["length", "slice", "indexOf", "concat", "push"].iter().all(|n| c.get_properties_of_type_exported(p).iter().any(|s| s.name() == *n))
            };
            let parts: Vec<P<CType>> = t.types().to_vec();
            let array = parts.iter().copied().find(|p| is_array(self.c, *p));
            let literal = |c: &mut Checker, p: P<CType>| !p.object_flags().intersects(ObjectFlags::ClassOrInterface | ObjectFlags::Reference) && !c.get_properties_of_type_exported(p).is_empty();
            let nothing_more = self.widening > 0 || parts.iter().all(|p| is_array(self.c, *p) || !literal(self.c, *p));
            if let Some(p) = array.filter(|_| nothing_more) {
                self.count("widened: intersection with an array -> the array");
                return self.from_type(p);
            }
            let mergeable = t.types().iter().all(|p| p.flags().intersects(TypeFlags::Object));
            // An alias's own body is written once, under its name, whatever its size.
            if mergeable && (skip.is_some() || self.c.get_properties_of_type_exported(t).len() <= self.intersection_limit) {
                if nothing_more && let Some(a) = self.array_like(t) {
                    return a;
                }
                // One of itself (leaflet's `extend(): { new(): any } & typeof Class`) once.
                if self.in_progress.contains(&t.id()) {
                    return self.erased("recursive anonymous type");
                }
                self.in_progress.push(t.id());
                let members = self.members_of_type(t, None);
                self.in_progress.pop();
                return Type::Object(members);
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
                    self.widening += 1;
                    let w = self.from_type(k);
                    self.widening -= 1;
                    w
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

    #[allow(clippy::wrong_self_convention)]
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
        if let Some(a) = self.array_like(t) {
            return a;
        }
        // A mapped type over a primitive's keys is the primitive (echarts'
        // `MapToType<string, S>`), as TypeScript has it: an object of `String`'s
        // members is a string.
        let names: HashSet<String> = self.c.get_properties_of_type_exported(t).iter().map(|p| p.name().to_string()).collect();
        let has = |ns: &[&str]| ns.iter().all(|n| names.contains(*n));
        for (prim, own) in [("string", ["charAt", "charCodeAt", "toUpperCase"]), ("number", ["toFixed", "toExponential", "toPrecision"])] {
            if has(&own) {
                self.count("widened: object of a primitive's members -> the primitive");
                return Type::prim(prim);
            }
        }
        // One alias of the same arguments within itself, as a new type each
        // time (echarts' `MapToType<any, S>`, `{ [k]: S | MapToType<any, S> }`), once.
        let alias = t.alias().and_then(|a| Some((a.symbol()?, a.type_arguments().iter().map(|x| x.id()).collect::<Vec<TypeId>>())));
        if alias.as_ref().is_some_and(|a| self.aliases_in_progress.contains(a)) {
            return self.erased("recursive anonymous type");
        }
        self.in_progress.push(t.id());
        if let Some(a) = &alias {
            self.aliases_in_progress.push(a.clone());
        }
        let members = self.members_of_type(t, None);
        if alias.is_some() {
            self.aliases_in_progress.pop();
        }
        self.in_progress.pop();
        if members.is_empty() && of.intersects(ObjectFlags::Mapped) {
            return self.mapped_over_parameter(t).unwrap_or_else(|| self.erased("generic mapped type"));
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

    // A mapped type over a type parameter's keys, whatever they turn out to
    // be: a record of what it maps them to, keyed as the parameter is
    // constrained (prosemirror's `{ [name in Nodes]: NodeSpec }`, `Nodes extends string`).
    fn mapped_over_parameter(&mut self, t: P<CType>) -> Option<Type> {
        MappedType::resolve_components(self.c, t);
        let m = t.as_mapped_type();
        if m.name_type().is_some() {
            return None;
        }
        // Not `keyof X<T>` (`Partial<TableState<TFeatures>>`), whose keys are X's.
        let keys = m.constraint_type().filter(|k| k.flags().intersects(TypeFlags::TypeParameter))?;
        let template = m.template_type()?;
        let base = self.c.get_base_constraint_of_type_exported(keys).unwrap_or(keys);
        let string = TypeFlags::String | TypeFlags::StringLiteral | TypeFlags::TemplateLiteral | TypeFlags::Any;
        let key = if base.flags().intersects(string) {
            "string"
        } else if base.flags().intersects(TypeFlags::Number | TypeFlags::NumberLiteral) {
            "number"
        } else if base.flags().intersects(TypeFlags::Union) && base.types().iter().any(|k| k.flags().intersects(string)) {
            "string"
        } else {
            return None;
        };
        self.count("widened: mapped type over a type parameter -> record");
        let mut index = Member::new(MemberKind::Index, String::new());
        let key = Param { name: "key".into(), optional: false, rest: false, constant: None, ty: Type::prim(key) };
        index.sig = Some(Signature { params: vec![key], ret: Some(self.from_type(template)), ..Default::default() });
        Some(Type::Object(vec![index]))
    }

    // All members of a type, inherited ones included, as the checker resolves them.
    // An array by its members (a mapped type over one, as eventemitter3's
    // `ArgumentMap<T>[K]`, or an intersection with one): an array of what it is
    // indexed to, not Array.prototype's members as an object's.
    fn array_like(&mut self, t: P<CType>) -> Option<Type> {
        let names: HashSet<String> = self.c.get_properties_of_type_exported(t).iter().map(|p| p.name().to_string()).collect();
        if !["length", "slice", "indexOf", "concat", "push"].iter().all(|n| names.contains(*n)) {
            return None;
        }
        let info = self.c.get_index_infos_of_type_exported(t).iter().find(|i| i.key_type().flags().intersects(TypeFlags::Number))?;
        self.count("widened: array-like object -> array");
        Some(Type::Array(Box::new(self.from_type(info.value_type()))))
    }

    pub fn members_of_type(&mut self, t: P<CType>, owner: Option<P<Symbol>>) -> Vec<Member> {
        let mut out = Vec::new();
        for (kind, mk) in [(SignatureKind::Call, MemberKind::Call), (SignatureKind::Construct, MemberKind::Construct)] {
            for sig in self.c.get_signatures_of_type_exported(t, kind) {
                let mut m = Member::new(mk, String::new());
                m.sig = Some(self.from_signature(*sig));
                m.doc = sig.declaration().and_then(|d| self.doc(d));
                out.push(m);
            }
        }
        for info in self.c.get_index_infos_of_type_exported(t) {
            let mut m = Member::new(MemberKind::Index, String::new());
            m.readonly = info.is_readonly();
            let key = Param { name: "key".into(), optional: false, rest: false, constant: None, ty: self.from_type(info.key_type()) };
            m.sig = Some(Signature { params: vec![key], ret: Some(self.from_type(info.value_type())), ..Default::default() });
            out.push(m);
        }
        for prop in self.c.get_properties_of_type_exported(t) {
            let name = symbol_member_name(*prop);
            let pty = self.c.get_type_of_symbol_exported(*prop);
            let from = match (owner, self.c.get_parent_of_symbol_exported(*prop)) {
                (Some(o), Some(p)) if p != o => Some(self.declaring_name(p)),
                _ => None,
            };
            if prop.flags().intersects(SymbolFlags::Method) {
                for sig in self.c.get_signatures_of_type_exported(pty, SignatureKind::Call) {
                    for variant in self.keyed_variants(*sig) {
                        let mut m = Member::new(MemberKind::Method, name.clone());
                        m.optional = prop.flags().intersects(SymbolFlags::Optional);
                        m.from = from.clone();
                        m.sig = Some(variant);
                        m.doc = sig.declaration().and_then(|d| self.doc(d));
                        out.push(m);
                    }
                }
                continue;
            }
            let mut m = Member::new(MemberKind::Property, name);
            m.optional = prop.flags().intersects(SymbolFlags::Optional);
            m.readonly = self.c.is_readonly_symbol_exported(*prop);
            m.from = from;
            m.doc = prop.declarations().iter().find_map(|d| self.doc(*d));
            let ty = if m.optional { self.optional_type(pty, *prop) } else { self.from_type(pty) };
            let syntactic = prop.declarations().iter().find_map(|d| d.type_node()).is_some_and(|n| n.kind() == Kind::UnionType && self.written_elements(n).len() >= 2);
            if let (Type::Union(parts), Some(o), true) = (&ty, owner, syntactic) {
                let members = parts.iter().filter(|p| !p.is_prim("null") && !p.is_prim("undefined"));
                if members.clone().count() >= 2 && !members.clone().any(|p| matches!(p, Type::Ref(r) if r.kind == "typeAlias")) {
                    let at = match &m.from {
                        Some(f) if f.starts_with(|c: char| c.is_alphabetic() || c == '_') => f.clone(),
                        _ => self.output_name(o),
                    };
                    self.written_members.insert(format!("{at}\u{1}{}", m.name));
                }
            }
            m.ty = Some(ty);
            out.push(m);
        }
        out
    }

    // `on<K extends "a" | "b">(type: K, f: (ev: Map[K]) => void)`: one signature
    // per key, the key a fixed argument and `Map[K]` its property's type.
    // Where keyed signatures are kept with their map (`keyed_params = "map"`),
    // one over a map these bindings do not declare (the DOM's
    // `HTMLElementTagNameMap`, for a library) is expanded: nothing here says
    // its keys.
    fn keyed_variants(&mut self, sig: P<CSignature>) -> Vec<Signature> {
        let keyed = self.keyed_parameter(sig);
        let map_elsewhere = |this: &mut Self, tp: TypeId| {
            let Some(t) = sig.type_parameters().iter().copied().find(|t| t.id() == tp) else { return false };
            this.declared_key_map(t).is_some_and(|map| !this.declared_here(map))
        };
        if let Some((tp, keys)) = keyed
            && (self.expand_keyed || self.map_keyed && map_elsewhere(self, tp))
        {
            self.count("expanded: keyed signature");
            return keys
                .into_iter()
                .map(|key| {
                    self.key_binding = Some((tp, key));
                    let s = self.from_signature(sig);
                    self.key_binding = None;
                    s
                })
                .collect();
        }
        vec![self.from_signature(sig)]
    }

    // A type parameter constrained to a set of string keys and taken directly by a parameter.
    fn keyed_parameter(&mut self, sig: P<CSignature>) -> Option<(TypeId, Vec<String>)> {
        for tp in sig.type_parameters() {
            let Some(k) = self.c.get_constraint_of_type_parameter_exported(*tp) else { continue };
            let parts: Vec<P<CType>> = if k.flags().intersects(TypeFlags::Union) { k.types().to_vec() } else { vec![k] };
            let keys: Option<Vec<String>> = parts
                .iter()
                .map(|p| match p.flags().intersects(TypeFlags::StringLiteral).then(|| p.as_literal_type().value()) {
                    Some(Some(LiteralValue::String(s))) => Some(s.to_string()),
                    _ => None,
                })
                .collect();
            let Some(keys) = keys else { continue };
            let taken = sig.parameters().iter().any(|p| self.c.get_type_of_symbol_exported(*p).id() == tp.id());
            if taken && !keys.is_empty() && keys.len() <= self.keyed_limit {
                return Some((tp.id(), keys));
            }
        }
        None
    }

    // How to recognise a value of this type at runtime, when the type says so
    // by itself: a primitive, an array, a function, a class with a constructor.
    pub fn runtime_test(&mut self, t: &Type) -> Option<Test> {
        match t {
            Type::Primitive(p) => match p.as_str() {
                "string" => Some(Test::TypeOf("string")),
                "number" => Some(Test::TypeOf("number")),
                "boolean" => Some(Test::TypeOf("boolean")),
                "bigint" => Some(Test::TypeOf("bigint")),
                "symbol" => Some(Test::TypeOf("symbol")),
                "undefined" => Some(Test::TypeOf("undefined")),
                _ => None,
            },
            Type::Literal { kind, .. } if *kind != "bigint" => Some(Test::TypeOf(kind)),
            Type::Array(_) | Type::Tuple(_) => Some(Test::Array),
            Type::Function(_) | Type::Constructor(_) => Some(Test::TypeOf("function")),
            Type::Ref(r) => {
                match r.qualified.as_deref().unwrap_or(&r.name) {
                    "ArrayBufferView" => return Some(Test::ArrayBufferView),
                    "ArrayBufferLike" => return Some(Test::ArrayBufferLike),
                    "PromiseLike" => return Some(Test::Thenable),
                    _ => {}
                }
                let s = self.c.get_merged_symbol_exported(r.sym?);
                match r.kind {
                    "enum" => None,
                    // An alias of an array or a tuple (leaflet's `LatLngTuple`) is an array.
                    "typeAlias" => {
                        let t = self.c.get_declared_type_of_symbol_exported(s);
                        (self.c.is_array_type_exported(t) || tsrs_checker::is_tuple_type_exported(t)).then_some(Test::Array)
                    }
                    // An interface that is only a call signature is a function.
                    "interface" if {
                        let t = self.c.get_declared_type_of_symbol_exported(s);
                        self.c.get_properties_of_type_exported(t).is_empty() && !self.c.get_signatures_of_type_exported(t, SignatureKind::Call).is_empty()
                    } =>
                    {
                        Some(Test::TypeOf("function"))
                    }
                    // A class, or an interface with a global constructor of the same name.
                    _ if s.flags().intersects(SymbolFlags::Class) || s.flags().intersects(SymbolFlags::Interface) && s.flags().intersects(SymbolFlags::Variable) => {
                        let transitive_only = !s.declarations().is_empty() && s.declarations().iter().all(|d| ast::get_source_file_of_node(*d).is_some_and(|f| self.is_transitive_file(&f)));
                        if transitive_only {
                            return None;
                        }
                        // By its name in the output, as a reference has it: generators
                        // reach it as they reach the declaration (its package, its jsName).
                        let r = self.make_ref(Some(s), vec![]);
                        let name = r.qualified.unwrap_or(r.name);
                        let constructible = s.flags().intersects(SymbolFlags::Class) || {
                            let v = self.c.get_type_of_symbol_exported(s);
                            !self.c.get_signatures_of_type_exported(v, SignatureKind::Construct).is_empty()
                        };
                        constructible.then_some(Test::InstanceOf(name))
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    // The properties of an object type: name, optional, literal value if it has one.
    pub fn object_shape(&mut self, t: &Type) -> Option<Vec<(String, bool, Option<String>)>> {
        match t {
            Type::Object(members) => Some(
                members
                    .iter()
                    .filter(|m| m.kind == MemberKind::Property || m.kind == MemberKind::Method)
                    .map(|m| {
                        let literal = match (&m.ty, &m.constant) {
                            (_, Some(c)) => Some(c.clone()),
                            (Some(Type::Literal { value, .. }), _) => Some(value.clone()),
                            _ => None,
                        };
                        (m.name.clone(), m.optional, literal)
                    })
                    .collect(),
            ),
            Type::Ref(r) if matches!(r.kind, "interface" | "class" | "typeAlias") => {
                let s = r.sym?;
                if r.kind == "typeAlias" && !s.flags().intersects(SymbolFlags::TypeAlias) {
                    return None;
                }
                let t = self.c.get_declared_type_of_symbol_exported(s);
                if !t.flags().intersects(TypeFlags::Object) {
                    return None;
                }
                let mut out = Vec::new();
                for p in self.c.get_properties_of_type_exported(t) {
                    let pt = self.c.get_type_of_symbol_exported(*p);
                    let literal = match pt.flags().intersects(TypeFlags::StringLiteral | TypeFlags::NumberLiteral | TypeFlags::BooleanLiteral).then(|| pt.as_literal_type().value()) {
                        Some(Some(LiteralValue::String(s))) => Some(s.to_string()),
                        Some(Some(LiteralValue::Number(n))) => Some(n.to_string()),
                        Some(Some(LiteralValue::Boolean(b))) => Some(b.to_string()),
                        _ => None,
                    };
                    out.push((symbol_member_name(*p), p.flags().intersects(SymbolFlags::Optional), literal));
                }
                Some(out)
            }
            _ => None,
        }
    }

    // `K extends keyof M` as declared, for a map `M` that is not generic: `M`.
    // Whether these bindings declare a symbol: of the package, or a library
    // file they include.
    fn declared_here(&mut self, sym: P<Symbol>) -> bool {
        let fqn = self.c.get_fully_qualified_name_exported(sym);
        let included = self.is_included(&fqn);
        let trans_type = self.is_transitive_type_symbol(sym);
        sym.declarations().iter().any(|d| {
            ast::get_source_file_of_node(*d).is_some_and(|f| {
                self.is_package_file(&f)
                    || (trans_type && self.is_transitive_file(&f))
                    || f.file_name() == self.root_file
                    || included && f.file_name().starts_with(&self.lib_path)
            })
        })
    }

    fn declared_key_map(&mut self, tp: P<CType>) -> Option<P<Symbol>> {
        let decl = tp.symbol()?.declarations().iter().copied().find(|d| d.kind() == Kind::TypeParameter)?;
        let constraint = decl.as_type_parameter_declaration().constraint()?;
        if constraint.kind() != Kind::TypeOperator || constraint.as_type_operator_node().operator() != Kind::KeyOfKeyword {
            return None;
        }
        let target = constraint.as_type_operator_node().type_();
        if target.kind() != Kind::TypeReference || target.as_type_reference_node().type_arguments().is_some() {
            return None;
        }
        let sym = self.c.get_symbol_at_location_exported(target.as_type_reference_node().type_name())?;
        let sym = self.c.get_merged_symbol_exported(sym);
        sym.flags().intersects(SymbolFlags::Interface).then_some(sym)
    }

    // The string keys of a declared object type.
    pub fn keys_of(&mut self, sym: P<Symbol>) -> Vec<String> {
        let t = self.c.get_declared_type_of_symbol_exported(sym);
        self.c.get_properties_of_type_exported(t).iter().map(|p| p.name()).filter(|n| !n.starts_with('\u{7f}')).map(str::to_string).collect()
    }

    // The type of one property of a declared object type.
    pub fn property_type(&mut self, sym: P<Symbol>, key: &str) -> Type {
        let t = self.c.get_declared_type_of_symbol_exported(sym);
        match self.c.get_type_of_property_of_type_exported(t, key) {
            Some(p) => self.from_type(p),
            None => self.erased("missing key"),
        }
    }

    // The type a member is declared in: for an object type written as an alias's
    // body (`type Unit = { ... }`), the alias.
    fn declaring_name(&mut self, parent: P<Symbol>) -> String {
        let alias = parent.declarations().iter().find_map(|d| d.parent().filter(|n| n.kind() == Kind::TypeAliasDeclaration));
        match alias.and_then(|a| self.c.get_symbol_of_declaration_exported(a)) {
            Some(s) => s.name().to_string(),
            None => parent.name().to_string(),
        }
    }

    // What a type node writes besides `undefined` and `null`: the members of a union.
    fn written_elements(&mut self, node: P<Node>) -> Vec<P<Node>> {
        if node.kind() != Kind::UnionType {
            return vec![node];
        }
        let nodes = node.as_union_type_node().types().nodes();
        nodes.iter().copied().filter(|n| !self.c.get_type_from_type_node_exported(*n).flags().intersects(TypeFlags::Undefined | TypeFlags::Null)).collect()
    }

    // An optional member's type without `undefined`: the union of an alias
    // written there (`backgroundColor?: ZRColor`) stays that alias, which the
    // checker's `ZRColor | undefined` has lost.
    fn optional_type(&mut self, t: P<CType>, sym: P<Symbol>) -> Type {
        if t.flags().intersects(TypeFlags::Union)
            && let Some(node) = sym.declarations().iter().find_map(|d| d.type_node())
        {
            let node = match self.written_elements(node).as_slice() {
                [only] => *only,
                _ => node,
            };
            let written = self.c.get_type_from_type_node_exported(node);
            let rest: Vec<TypeId> = t.types().iter().filter(|p| !p.flags().intersects(TypeFlags::Undefined)).map(|p| p.id()).collect();
            if written.flags().intersects(TypeFlags::Union)
                && written.alias().is_some_and(|a| a.type_arguments().is_empty())
                && written.types().len() == rest.len()
                && written.types().iter().all(|p| rest.contains(&p.id()))
            {
                return self.from_type(written);
            }
        }
        strip_undefined(self.from_type(t))
    }

    #[allow(clippy::wrong_self_convention)]
    pub(super) fn from_signature(&mut self, sig: P<CSignature>) -> Signature {
        let mut out = Signature::default();
        let bound = self.key_binding.as_ref().map(|b| b.0);
        for tp in sig.type_parameters().iter().filter(|tp| Some(tp.id()) != bound) {
            // The checker has evaluated `keyof M`; the declaration still says it.
            let constraint = match self.map_keyed.then(|| self.declared_key_map(*tp)).flatten() {
                Some(map) => {
                    self.kept_keyed.insert(tp.id());
                    let map = Type::Ref(self.make_ref(Some(map), Vec::new()));
                    Some(Type::Op(Box::new(Op::KeyOf(map)), None))
                }
                None => self.c.get_constraint_of_type_parameter_exported(*tp).map(|k| self.from_type(k)),
            };
            out.type_params.push(TypeParam {
                name: tp.symbol().map(|s| s.name().to_string()).unwrap_or_default(),
                sym: tp.symbol(),
                tid: Some(tp.id().0),
                constraint,
                default: None,
                self_bounded: false,
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
            // The parameter that takes the key being expanded is fixed to it.
            if let Some((tp, key)) = &self.key_binding
                && t.id() == *tp
            {
                out.params.push(Param { name: p.name().to_string(), optional, rest, constant: Some(key.clone()), ty: Type::prim("string") });
                continue;
            }
            let ty = if optional { self.optional_type(t, *p) } else { self.from_type(t) };
            // `__0`: a parameter destructured where it was declared.
            let name = if p.name().starts_with("__") { destructured_name(&ty, i) } else { p.name().to_string() };
            out.params.push(Param { name, optional, rest, constant: None, ty });
        }
        // A return type written `this` stays so, though the checker has put the receiver's type in.
        let written_this = self.keep_this && sig.declaration().and_then(|d| d.type_node()).is_some_and(|n| n.kind() == Kind::ThisType);
        let ret = self.c.get_return_type_of_signature_exported(sig);
        out.ret = Some(if written_this { Type::This } else { self.from_type(ret) });
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
    // Ancestors by their names in the output, as references have them, with
    // their symbols.
    pub fn flattened(&mut self, sym: P<Symbol>) -> (Vec<Member>, Vec<(String, Sym)>) {
        let t = self.c.get_declared_type_of_symbol_exported(sym);
        self.keep_this = true;
        let members = self.members_of_type(t, Some(sym));
        self.keep_this = false;
        let mut ancestors: Vec<(String, Sym)> = Vec::new();
        let mut queue = vec![t];
        while let Some(t) = queue.pop() {
            for base in self.c.get_base_types_exported(t) {
                let Some(s) = base.symbol() else { continue };
                let r = self.make_ref(Some(s), Vec::new());
                let name = r.qualified.clone().unwrap_or(r.name);
                if !name.is_empty() && !ancestors.iter().any(|a| a.0 == name) {
                    ancestors.push((name, r.sym));
                    queue.push(*base);
                }
            }
        }
        (members, ancestors)
    }
}
