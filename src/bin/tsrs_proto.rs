// tsrs feasibility prototype: dump declarations as structured XML.
// Types are emitted as written; the checker resolves references and merges declarations.
// Usage: cargo run --bin tsrs_proto -- <file.d.ts | lib.dom.d.ts>

use std::collections::{BTreeMap, HashSet};
use std::fmt::Write as _;
use std::sync::Arc;

use tsrs_ast::{Kind, ModifierFlags, Node, SymbolFlags, has_syntactic_modifier};
use tsrs_checker::checker::Checker;
use tsrs_checker::types::LiteralValue;
use tsrs_compiler::{Context, ProgramOptions, new_compiler_host, new_program};
use tsrs_core::P;
use tsrs_scanner::get_text_of_node;
use tsrs_tsoptions::{self as tsoptions, ParseConfigHost};
use tsrs_vfs::{FS, bundled, vfstest};

struct ConfigHost {
    fs: Arc<dyn FS>,
}

impl ParseConfigHost for ConfigHost {
    fn fs(&self) -> &dyn FS {
        &*self.fs
    }
    fn get_current_directory(&self) -> &str {
        "/"
    }
}

fn main() {
    let arg = std::env::args().nth(1).unwrap_or_else(|| "test.d.ts".into());
    // Bare `lib.*.d.ts` names select a bundled lib file.
    let is_lib = arg.starts_with("lib.") && !std::path::Path::new(&arg).exists();
    let src = if is_lib { String::new() } else { std::fs::read_to_string(&arg).expect("read input") };

    let tsconfig = r#"{ "compilerOptions": { "lib": ["es2020", "dom", "dom.iterable", "dom.asynciterable"], "noEmit": true }, "files": ["/input.d.ts"] }"#;
    let fs: Arc<dyn FS> = Arc::new(bundled::wrap_fs(vfstest::from_map(
        [("/tsconfig.json", tsconfig.to_string()), ("/input.d.ts", src)],
        true,
    )));
    let host: &'static ConfigHost = Box::leak(Box::new(ConfigHost { fs: Arc::clone(&fs) }));
    let (config, errs) =
        tsoptions::get_parsed_command_line_of_config_file("/tsconfig.json", None, None, host, None);
    assert!(errs.is_empty(), "tsconfig errors: {}", errs.len());

    let compiler_host = new_compiler_host("/", fs, &bundled::lib_path(), None, None, None);
    let program = new_program(ProgramOptions::new(P::new(config.unwrap()), compiler_host));
    let name = if is_lib { format!("{}/{}", bundled::lib_path(), arg) } else { "/input.d.ts".into() };
    let file = program.get_source_file(&name).unwrap_or_else(|| panic!("{name} not in program"));
    let mut checker = program.get_type_checker_for_file(&Context::default(), file);

    let mut e = Emitter { c: &mut checker, out: String::new(), depth: 0, seen: HashSet::new(), raw: BTreeMap::new() };
    e.open("declarations", &[]);
    e.statements(file.statements().nodes());
    e.close("declarations");
    print!("{}", e.out);
    for (kind, n) in &e.raw {
        eprintln!("raw {kind}: {n}");
    }
}

struct Emitter<'c> {
    c: &'c mut Checker,
    out: String,
    depth: usize,
    // Merged symbols already emitted, by fully qualified name.
    seen: HashSet<String>,
    // Type node kinds emitted as <raw>, for coverage.
    raw: BTreeMap<String, usize>,
}

impl Emitter<'_> {
    fn line(&mut self, tag: &str, attrs: &[(&str, &str)], close: bool) {
        let _ = write!(self.out, "{:w$}<{tag}", "", w = self.depth * 2);
        for (k, v) in attrs {
            let _ = write!(self.out, " {k}=\"{}\"", xml(v));
        }
        self.out.push_str(if close { "/>\n" } else { ">\n" });
    }
    fn open(&mut self, tag: &str, attrs: &[(&str, &str)]) {
        self.line(tag, attrs, false);
        self.depth += 1;
    }
    fn close(&mut self, tag: &str) {
        self.depth -= 1;
        let _ = writeln!(self.out, "{:w$}</{tag}>", "", w = self.depth * 2);
    }
    fn leaf(&mut self, tag: &str, attrs: &[(&str, &str)]) {
        self.line(tag, attrs, true);
    }

    // Emits a merged symbol once, from all its declarations.
    fn first_time(&mut self, node: P<Node>) -> Option<Vec<P<Node>>> {
        let sym = self.c.get_symbol_of_declaration_exported(node)?;
        let sym = self.c.get_merged_symbol_exported(sym);
        let fqn = self.c.get_fully_qualified_name_exported(sym);
        let kind = node.kind();
        let key = format!("{kind:?}:{fqn}");
        if !self.seen.insert(key) {
            return None;
        }
        Some(sym.declarations().iter().copied().filter(|d| d.kind() == kind).collect())
    }

    fn statements(&mut self, stmts: &[P<Node>]) {
        for &s in stmts {
            match s.kind() {
                Kind::InterfaceDeclaration => self.interface(s),
                Kind::TypeAliasDeclaration => self.type_alias(s),
                Kind::FunctionDeclaration => self.function(s),
                Kind::EnumDeclaration => self.enumeration(s),
                Kind::ModuleDeclaration => self.namespace(s),
                Kind::VariableStatement => {
                    for d in s.as_variable_statement().declaration_list().as_variable_declaration_list().declarations().nodes() {
                        let name = text(d.as_variable_declaration().name());
                        self.open("var", &[("name", &name)]);
                        self.opt_type(d.type_node());
                        self.close("var");
                    }
                }
                _ => {}
            }
        }
    }

    fn interface(&mut self, node: P<Node>) {
        let Some(decls) = self.first_time(node) else { return };
        let name = text(name_of(node));
        self.open("interface", &[("name", &name)]);
        self.type_params(node.type_parameters());
        for d in &decls {
            for clause in d.as_interface_declaration().heritage_clauses().iter().flat_map(|l| l.nodes()) {
                for t in clause.as_heritage_clause().types().nodes() {
                    self.open("extends", &[]);
                    if t.kind() == Kind::ExpressionWithTypeArguments {
                        self.reference(t.expression().unwrap(), t.type_argument_list());
                    } else {
                        self.ty(*t);
                    }
                    self.close("extends");
                }
            }
        }
        for d in &decls {
            self.members(d.members());
        }
        self.close("interface");
    }

    fn type_alias(&mut self, node: P<Node>) {
        let name = text(name_of(node));
        self.open("typeAlias", &[("name", &name)]);
        self.type_params(node.type_parameters());
        self.opt_type(node.type_node());
        self.close("typeAlias");
    }

    fn function(&mut self, node: P<Node>) {
        // Overloads are separate declarations; emit each.
        let name = text(name_of(node));
        self.open("function", &[("name", &name)]);
        self.signature(node);
        self.close("function");
    }

    fn enumeration(&mut self, node: P<Node>) {
        let Some(decls) = self.first_time(node) else { return };
        let name = text(name_of(node));
        // Resolving the enum type computes member values.
        if let Some(sym) = self.c.get_symbol_of_declaration_exported(node) {
            self.c.get_declared_type_of_symbol_exported(sym);
        }
        self.open("enum", &[("name", &name)]);
        for d in decls {
            for m in d.members() {
                let (ty, value) = match self.c.get_constant_value(*m) {
                    Some(LiteralValue::String(s)) => ("string", s.to_string()),
                    Some(LiteralValue::Number(n)) => ("number", n.to_string()),
                    // Computed, non-constant member.
                    _ => ("computed", m.as_enum_member().initializer().map(text).unwrap_or_default()),
                };
                self.leaf("member", &[("name", &member_name(name_of(*m))), ("type", ty), ("value", &value)]);
            }
        }
        self.close("enum");
    }

    fn namespace(&mut self, node: P<Node>) {
        let name = text(name_of(node));
        self.open("namespace", &[("name", &name)]);
        match node.body() {
            Some(b) if b.kind() == Kind::ModuleDeclaration => self.namespace(b),
            Some(b) => self.statements(b.statements()),
            None => {}
        }
        self.close("namespace");
    }

    fn members(&mut self, members: &[P<Node>]) {
        for &m in members {
            let name = m.name().map(member_name).unwrap_or_default();
            let mut attrs: Vec<(&str, &str)> = Vec::new();
            if !name.is_empty() {
                attrs.push(("name", &name));
            }
            if m.question_token().is_some() {
                attrs.push(("optional", "true"));
            }
            if has_syntactic_modifier(m, ModifierFlags::Readonly) {
                attrs.push(("readonly", "true"));
            }
            let tag = match m.kind() {
                Kind::PropertySignature => "property",
                Kind::MethodSignature => "method",
                Kind::CallSignature => "call",
                Kind::ConstructSignature => "construct",
                Kind::IndexSignature => "index",
                Kind::GetAccessor => "get",
                Kind::SetAccessor => "set",
                k => {
                    *self.raw.entry(format!("member {k:?}")).or_default() += 1;
                    continue;
                }
            };
            self.open(tag, &attrs);
            if tag == "property" {
                self.opt_type(m.type_node());
            } else {
                self.signature(m);
            }
            self.close(tag);
        }
    }

    fn type_params(&mut self, params: &[P<Node>]) {
        for &p in params {
            let tp = p.as_type_parameter_declaration();
            let name = text(tp.name());
            if tp.constraint().is_none() && tp.default_type().is_none() {
                self.leaf("typeParameter", &[("name", &name)]);
                continue;
            }
            self.open("typeParameter", &[("name", &name)]);
            if let Some(k) = tp.constraint() {
                self.children("constraint", &[k]);
            }
            if let Some(d) = tp.default_type() {
                self.children("default", &[d]);
            }
            self.close("typeParameter");
        }
    }

    fn signature(&mut self, node: P<Node>) {
        self.type_params(node.type_parameters());
        for &p in node.parameters() {
            let pd = p.as_parameter_declaration();
            let name = text(pd.name());
            let mut attrs = vec![("name", name.as_str())];
            if pd.question_token().is_some() {
                attrs.push(("optional", "true"));
            }
            if pd.dot_dot_dot_token().is_some() {
                attrs.push(("rest", "true"));
            }
            self.open("parameter", &attrs);
            self.opt_type(p.type_node());
            self.close("parameter");
        }
        if let Some(r) = node.type_node() {
            self.children("returns", &[r]);
        }
    }

    fn opt_type(&mut self, node: Option<P<Node>>) {
        match node {
            Some(t) => self.ty(t),
            None => self.leaf("primitive", &[("name", "any"), ("implicit", "true")]),
        }
    }

    fn children(&mut self, tag: &str, nodes: &[P<Node>]) {
        self.open(tag, &[]);
        for &n in nodes {
            self.ty(n);
        }
        self.close(tag);
    }

    fn element(&mut self, name: Option<String>, optional: bool, rest: bool, t: P<Node>) {
        let mut attrs = Vec::new();
        if let Some(n) = &name {
            attrs.push(("name", n.as_str()));
        }
        if optional {
            attrs.push(("optional", "true"));
        }
        if rest {
            attrs.push(("rest", "true"));
        }
        self.open("element", &attrs);
        self.ty(t);
        self.close("element");
    }

    fn ty(&mut self, t: P<Node>) {
        match t.kind() {
            Kind::AnyKeyword | Kind::UnknownKeyword | Kind::StringKeyword | Kind::NumberKeyword
            | Kind::BigIntKeyword | Kind::BooleanKeyword | Kind::SymbolKeyword | Kind::ObjectKeyword
            | Kind::VoidKeyword | Kind::UndefinedKeyword | Kind::NeverKeyword | Kind::NullKeyword => {
                self.leaf("primitive", &[("name", &text(t))])
            }
            Kind::ThisType => self.leaf("this", &[]),
            Kind::LiteralType => {
                let lit = t.as_literal_type_node().literal();
                match lit.kind() {
                    Kind::NullKeyword => self.leaf("primitive", &[("name", "null")]),
                    Kind::TrueKeyword | Kind::FalseKeyword => self.leaf("literal", &[("type", "boolean"), ("value", &text(lit))]),
                    Kind::StringLiteral | Kind::NoSubstitutionTemplateLiteral => self.leaf("literal", &[("type", "string"), ("value", lit.text())]),
                    Kind::BigIntLiteral => self.leaf("literal", &[("type", "bigint"), ("value", &text(lit))]),
                    _ => self.leaf("literal", &[("type", "number"), ("value", &text(lit))]),
                }
            }
            Kind::ParenthesizedType => self.ty(t.type_node().unwrap()),
            Kind::TypeReference => {
                let r = t.as_type_reference_node();
                self.reference(r.type_name(), r.type_arguments());
            }
            Kind::UnionType => self.children("union", t.as_union_type_node().types().nodes()),
            Kind::IntersectionType => self.children("intersection", t.as_intersection_type_node().types().nodes()),
            Kind::ArrayType => self.children("array", &[t.as_array_type_node().element_type()]),
            Kind::TupleType => {
                self.open("tuple", &[]);
                for &el in t.as_tuple_type_node().elements().nodes() {
                    match el.kind() {
                        Kind::NamedTupleMember => {
                            let m = el.as_named_tuple_member();
                            self.element(Some(text(m.name())), m.question_token().is_some(), m.dot_dot_dot_token().is_some(), m.type_());
                        }
                        Kind::OptionalType => self.element(None, true, false, el.type_node().unwrap()),
                        Kind::RestType => self.element(None, false, true, el.type_node().unwrap()),
                        _ => self.element(None, false, false, el),
                    }
                }
                self.close("tuple");
            }
            Kind::FunctionType | Kind::ConstructorType => {
                let tag = if t.kind() == Kind::FunctionType { "function" } else { "constructor" };
                self.open(tag, &[]);
                self.signature(t);
                self.close(tag);
            }
            Kind::TypeLiteral => {
                self.open("object", &[]);
                self.members(t.members());
                self.close("object");
            }
            Kind::TypeOperator => {
                let op = t.as_type_operator_node();
                let tag = match op.operator() {
                    Kind::KeyOfKeyword => "keyof",
                    Kind::ReadonlyKeyword => "readonly",
                    _ => "unique",
                };
                self.children(tag, &[op.type_()]);
            }
            Kind::IndexedAccessType => {
                let ia = t.as_indexed_access_type_node();
                self.children("indexedAccess", &[ia.object_type(), ia.index_type()]);
            }
            Kind::TypeQuery => self.leaf("typeof", &[("name", &text(t.as_type_query_node().expr_name()))]),
            Kind::TypePredicate => {
                let p = t.as_type_predicate_node();
                let name = text(p.parameter_name());
                let mut attrs = vec![("parameter", name.as_str())];
                if p.asserts_modifier().is_some() {
                    attrs.push(("asserts", "true"));
                }
                self.open("predicate", &attrs);
                if let Some(pt) = p.type_() {
                    self.ty(pt);
                }
                self.close("predicate");
            }
            Kind::TemplateLiteralType => {
                let tl = t.as_template_literal_type_node();
                self.open("template", &[]);
                self.leaf("text", &[("value", tl.head().text())]);
                for &s in tl.template_spans().nodes() {
                    let span = s.as_template_literal_type_span();
                    self.ty(span.type_());
                    self.leaf("text", &[("value", span.literal().text())]);
                }
                self.close("template");
            }
            Kind::ConditionalType => {
                let ct = t.as_conditional_type_node();
                self.open("conditional", &[]);
                self.children("check", &[ct.check_type()]);
                self.children("extends", &[ct.extends_type()]);
                self.children("true", &[ct.true_type()]);
                self.children("false", &[ct.false_type()]);
                self.close("conditional");
            }
            Kind::InferType => {
                self.open("infer", &[]);
                self.type_params(&[t.as_infer_type_node().type_parameter()]);
                self.close("infer");
            }
            Kind::MappedType => {
                let mt = t.as_mapped_type_node();
                let mut attrs = Vec::new();
                // `+`/`-` modifiers are kept as written.
                let ro = mt.readonly_token().map(text);
                let opt = mt.question_token().map(text);
                if let Some(r) = &ro {
                    attrs.push(("readonly", r.as_str()));
                }
                if let Some(o) = &opt {
                    attrs.push(("optional", o.as_str()));
                }
                self.open("mapped", &attrs);
                self.type_params(&[mt.type_parameter()]);
                if let Some(n) = mt.name_type() {
                    self.children("as", &[n]);
                }
                if let Some(v) = mt.type_() {
                    self.children("value", &[v]);
                }
                self.close("mapped");
            }
            k => {
                // Import types and anything else unhandled.
                *self.raw.entry(format!("{k:?}")).or_default() += 1;
                self.leaf("raw", &[("kind", &format!("{k:?}")), ("text", &text(t))]);
            }
        }
    }

    // A named type, resolved by the checker to what it refers to.
    fn reference(&mut self, name: P<Node>, args: Option<P<tsrs_ast::NodeList>>) {
        let target = text(name);
        let kind = match self.c.get_symbol_at_location_exported(name) {
            Some(s) => symbol_kind(s.flags()),
            None => "unresolved",
        };
        let args = args.map(|l| l.nodes()).unwrap_or(&[]);
        if args.is_empty() {
            return self.leaf("ref", &[("name", &target), ("kind", kind)]);
        }
        self.open("ref", &[("name", &target), ("kind", kind)]);
        for &a in args {
            self.ty(a);
        }
        self.close("ref");
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

fn text(node: P<Node>) -> String {
    get_text_of_node(node)
}

fn xml(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}
