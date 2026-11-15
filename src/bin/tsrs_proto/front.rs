// Front end: tsrs AST -> IR (as written), and checker Type -> IR (resolved).

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use tsrs_ast::{self as ast, Kind, ModifierFlags, Node, NodeList, SourceFile, Symbol, SymbolFlags, has_syntactic_modifier};
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
    // Packages being bound, by the directory they are installed in and the
    // name they are imported by. None: the input file and what it imports
    // outside node_modules.
    packages: Vec<(String, String)>,
    // The package each top-level declaration comes from, by its output name.
    pub package_of_name: HashMap<String, String>,
    lib_path: String,
    // Merged symbols already converted, by kind and fully qualified name.
    seen: HashSet<String>,
    // Declarations exported under another name than their own, by their own.
    // What a symbol is exported as, when not its own name.
    exported_as: HashMap<P<Symbol>, String>,
    // A package's top-level symbols by the name they have in the output, and
    // the reverse: two files may each declare their own `Options`.
    output_names: HashMap<P<Symbol>, String>,
    named: HashMap<String, P<Symbol>>,
    emitted_nodes: HashSet<P<Node>>,
    queued: HashSet<P<Symbol>>,
    pending: VecDeque<P<Symbol>>,
    // Counts of lossy conversions and unhandled syntax, by reason.
    pub stats: BTreeMap<String, usize>,
    computed_alias: HashMap<P<Symbol>, bool>,
    // Anonymous types being converted; guards recursive types.
    in_progress: Vec<TypeId>,
    // How deep `from_type` is, and how many types the declaration being
    // converted may still convert.
    depth: usize,
    budget: usize,
    // Conversions cut short by a limit, and checker types converted, with
    // how much of a budget each took.
    cut: usize,
    converted: HashMap<TypeId, (Type, usize)>,
    // Lowering: merge `A & B` into one object type.
    pub merge_intersections: bool,
    pub intersection_limit: usize,
    // Lowering: expand a signature over the keys its type parameter may take.
    pub expand_keyed: bool,
    // Keep keyed signatures generic: `K extends keyof M` and `M[K]` stay.
    pub map_keyed: bool,
    kept_keyed: HashSet<TypeId>,
    pub keyed_limit: usize,
    // During expansion: the type parameter being replaced, and its key.
    key_binding: Option<(TypeId, String)>,
    skip_alias: Option<P<Symbol>>,
    // Namespaces of the bundled lib files that are part of the output, with
    // their declarations from every lib file: those `--include` names, and
    // when the input is a lib file, the ones it declares (`Intl` in
    // lib.es2020.intl.d.ts gathers lib.es5.d.ts's `Intl` too).
    pub included: HashSet<String>,
}

// Checker types one conversion may make: what an ordinary type needs many
// times over, and less than a type that expands without end.
const BUDGET: usize = 2_000;

const DEFERRED: TypeFlags = TypeFlags::Index
    .union(TypeFlags::IndexedAccess)
    .union(TypeFlags::Conditional)
    .union(TypeFlags::Substitution)
    .union(TypeFlags::TemplateLiteral)
    .union(TypeFlags::StringMapping);

impl<'c> Front<'c> {
    pub fn new(c: &'c mut Checker, root_file: String, lib_path: String) -> Self {
        Front {
            c,
            root_file,
            packages: Vec::new(),
            package_of_name: HashMap::new(),
            lib_path,
            seen: HashSet::new(),
            exported_as: HashMap::new(),
            output_names: HashMap::new(),
            named: HashMap::new(),
            emitted_nodes: HashSet::new(),
            queued: HashSet::new(),
            pending: VecDeque::new(),
            stats: BTreeMap::new(),
            computed_alias: HashMap::new(),
            in_progress: Vec::new(),
            depth: 0,
            budget: BUDGET,
            cut: 0,
            converted: HashMap::new(),
            merge_intersections: false,
            intersection_limit: 64,
            expand_keyed: false,
            map_keyed: false,
            kept_keyed: HashSet::new(),
            keyed_limit: 0,
            key_binding: None,
            skip_alias: None,
            included: HashSet::new(),
        }
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

    pub fn collect(&mut self, file: P<SourceFile>) -> Vec<Decl> {
        // The entry file's `export default function f`, `export default class C`.
        for &s in file.statements().nodes() {
            if matches!(s.kind(), Kind::FunctionDeclaration | Kind::ClassDeclaration)
                && has_syntactic_modifier(s, ModifierFlags::Default)
                && let Some(sym) = self.c.get_symbol_of_declaration_exported(s)
            {
                let sym = self.c.get_merged_symbol_exported(sym);
                self.exported_as.insert(sym, "default".into());
            }
        }
        if let Some(module) = file.symbol() {
            let module = self.c.get_merged_symbol_exported(module);
            for sym in self.c.get_exports_of_module_exported(module) {
                self.reserve(sym);
            }
        }
        let mut out = Vec::new();
        for &s in file.statements().nodes() {
            self.emit_top(s, &mut out);
        }
        while let Some(sym) = self.pending.pop_front() {
            self.emit_symbol(sym, &mut out);
        }
        self.exports(&mut out);
        out
    }

    // A declaration exported under another name is known by that name; one
    // exported as `default` keeps its own and says so: `jsName="default"`.
    fn exports(&mut self, out: &mut [Decl]) {
        let name_of = |d: &Decl| -> Option<String> {
            match d {
                Decl::Interface(i) => Some(i.name.clone()),
                Decl::Alias(a) => Some(a.name.clone()),
                Decl::Function(f) => Some(f.name.clone()),
                Decl::Var(v) => Some(v.name.clone()),
                Decl::Enum(e) => Some(e.name.clone()),
                Decl::Namespace(n) => Some(n.name.clone()),
            }
        };
        let declared: HashSet<String> = out.iter().filter_map(name_of).collect();
        // By the name each exported symbol has in the output.
        let by_name: HashMap<String, String> = self
            .exported_as
            .iter()
            .map(|(s, e)| (self.output_names.get(s).cloned().unwrap_or_else(|| self.own_name(*s)), e.clone()))
            .collect();
        let mut renames = Vec::new();
        for d in out.iter_mut() {
            let Some(name) = name_of(d) else { continue };
            let Some(exported) = by_name.get(&name).cloned() else { continue };
            if exported == "default" {
                match d {
                    Decl::Interface(i) => i.js_name = Some(exported.clone()),
                    Decl::Function(f) => f.js_name = Some(exported.clone()),
                    Decl::Var(v) => v.js_name = Some(exported.clone()),
                    Decl::Namespace(n) => n.js_name = Some(exported.clone()),
                    _ => {}
                }
            // A namespace is reached by what contains it: `path.posix` is `path` by
            // another name, not a rename of it.
            } else if exported != name && !declared.contains(&exported) && !matches!(d, Decl::Namespace(_)) {
                renames.push((name, exported));
            }
        }
        crate::lower::rename_everywhere(out, &renames);
        // References made after this (union tests, in lowering) have the new names.
        for name in self.output_names.values_mut() {
            if let Some((_, new)) = renames.iter().find(|(old, _)| old == name) {
                *name = new.clone();
            }
        }
        for (old, new) in &renames {
            if let Some(package) = self.package_of_name.remove(old) {
                self.package_of_name.insert(new.clone(), package);
            }
        }
    }

    // A package that is an ambient module, `declare module "fs" { }`: what it exports.
    pub fn collect_module(&mut self, module: P<Symbol>) -> Vec<Decl> {
        let module = self.c.get_merged_symbol_exported(module);
        let exports = self.c.get_exports_of_module_exported(module);
        for &sym in &exports {
            self.reserve(sym);
        }
        for sym in exports {
            self.enqueue_symbol(sym);
        }
        let mut out = Vec::new();
        while let Some(sym) = self.pending.pop_front() {
            self.emit_symbol(sym, &mut out);
        }
        self.exports(&mut out);
        out
    }

    // The lib namespaces `names` (`--include Intl`), from whichever lib files declare them.
    pub fn collect_included(&mut self, libs: &[P<SourceFile>], names: &[String], out: &mut Vec<Decl>) {
        self.included.extend(names.iter().cloned());
        for lib in libs {
            for &s in lib.statements().nodes() {
                if s.kind() == Kind::ModuleDeclaration && s.name().is_some_and(|n| names.contains(&text(n))) {
                    self.emit_decl_node(s, out);
                }
            }
        }
        while let Some(sym) = self.pending.pop_front() {
            self.emit_symbol(sym, out);
        }
    }

    // Whether a qualified name is in a namespace of the lib files that is part of the output.
    fn is_included(&self, fqn: &str) -> bool {
        self.included.contains(fqn.split('.').next().unwrap_or(fqn))
    }

    // The directory a package is installed in: `.../node_modules/@scope/name`.
    fn package_dir(file: &str) -> Option<String> {
        let (before, after) = file.rsplit_once("/node_modules/")?;
        let mut segs = after.split('/');
        let len = match segs.next()? {
            s if s.starts_with('@') => s.len() + 1 + segs.next()?.len(),
            s => s.len(),
        };
        Some(file[..before.len() + "/node_modules/".len() + len].to_string())
    }

    // Binds `name`, whose entry file is `entry`, with the packages before it.
    pub fn add_package(&mut self, name: &str, entry: &str) {
        let dir = Self::package_dir(entry).unwrap_or_else(|| entry.rsplit_once('/').map_or(entry, |p| p.0).to_string());
        self.packages.push((dir, name.to_string()));
    }

    // The package a file belongs to, of those being bound.
    fn package_of(&self, file: &str) -> Option<&str> {
        self.packages
            .iter()
            .find(|(dir, _)| file.starts_with(dir.as_str()) && matches!(file.as_bytes().get(dir.len()), None | Some(b'/')) && !file[dir.len()..].contains("/node_modules/"))
            .map(|p| p.1.as_str())
    }

    fn is_package_file(&self, sf: &SourceFile) -> bool {
        let name = sf.file_name();
        if self.root_file.starts_with(&self.lib_path) || name.starts_with(&self.lib_path) {
            return false;
        }
        if !self.packages.is_empty() {
            return self.package_of(name).is_some();
        }
        if let Some((before, after)) = self.root_file.rsplit_once("/node_modules/") {
            let mut segs = after.split('/');
            let pkg_len = match segs.next() {
                Some(s) if s.starts_with('@') => s.len() + 1 + segs.next().map_or(0, str::len),
                Some(s) => s.len(),
                None => 0,
            };
            let pkg_root = &self.root_file[..before.len() + "/node_modules/".len() + pkg_len];
            return name.starts_with(pkg_root)
                && matches!(name.as_bytes().get(pkg_root.len()), None | Some(b'/'))
                && !name[pkg_root.len()..].contains("/node_modules/");
        }
        !name.contains("/node_modules/")
    }

    fn resolve_alias(&mut self, sym: P<Symbol>) -> P<Symbol> {
        if sym.flags().intersects(SymbolFlags::Alias) {
            let (resolved, ok) = self.c.resolve_alias_exported(Some(sym));
            if ok && let Some(target) = resolved {
                return target;
            }
        }
        sym
    }

    // A symbol's own name: a default export's symbol is named `default` (or
    // `export=`), its declaration by what it is called, if anything.
    fn own_name(&self, s: P<Symbol>) -> String {
        match s.name() {
            "default" | "export=" => match s.declarations().first() {
                Some(d) => d.name().map(text).unwrap_or_else(|| anonymous_name(*d)),
                None => "default".into(),
            },
            name => name.to_string(),
        }
    }

    // A package's top-level symbol: one of a package file, not in a namespace.
    // A module's own block (`declare module "tty" { ... }`) counts as its file.
    fn package_top(&self, sym: P<Symbol>) -> bool {
        let top = |n: P<Node>| {
            n.kind() == Kind::SourceFile
                || n.kind() == Kind::ModuleBlock && n.parent().is_some_and(|m| m.name().is_some_and(|x| x.kind() == Kind::StringLiteral))
        };
        sym.declarations().iter().any(|d| {
            ast::get_source_file_of_node(*d).is_some_and(|f| self.is_package_file(&f))
                && d.parent().is_some_and(|p| top(p) || p.kind() == Kind::VariableDeclarationList && p.parent().and_then(|st| st.parent()).is_some_and(top))
        })
    }

    // The name a symbol has in the output: its own, unless another of the
    // package's symbols has it, then its file's before it (`RenderList_RenderItem`).
    fn output_name(&mut self, sym: P<Symbol>) -> String {
        if let Some(name) = self.output_names.get(&sym) {
            return name.clone();
        }
        let own = self.own_name(sym);
        if !self.package_top(sym) {
            return own;
        }
        let mut name = own.clone();
        if self.named.get(&name).is_some_and(|s| *s != sym) {
            let file = sym.declarations().first().map(|d| anonymous_name(*d)).unwrap_or_default();
            name = format!("{file}_{own}");
            let mut n = 2;
            while self.named.contains_key(&name) {
                name = format!("{file}_{own}_{n}");
                n += 1;
            }
        }
        self.named.insert(name.clone(), sym);
        self.output_names.insert(sym, name.clone());
        name
    }

    // Its JavaScript name, where the output renamed it apart from another's
    // (`pb_Node` is pb's `Node`).
    fn renamed_from(&self, sym: P<Symbol>) -> Option<String> {
        let own = self.own_name(sym);
        self.output_names.get(&sym).filter(|n| **n != own).map(|_| own)
    }

    // What the package exports keeps its own name: named before anything
    // else, from the entry module's exports, where its own declarations hide
    // what `export *` brings of the same name.
    fn reserve(&mut self, sym: P<Symbol>) {
        let sym = self.resolve_alias(sym);
        self.output_name(sym);
    }

    fn enqueue_symbol(&mut self, sym: P<Symbol>) {
        // `export { local as exported }`: the alias has the name it is exported as.
        let exported = sym.name();
        // A module's export, not a namespace's (`namespace util { export { util_merge as merge } }`).
        let in_namespace = sym.declarations().first().is_some_and(|d| {
            let mut n = d.parent();
            while let Some(p) = n.filter(|p| !matches!(p.kind(), Kind::SourceFile | Kind::ModuleBlock)) {
                n = p.parent();
            }
            n.is_some_and(|b| b.kind() == Kind::ModuleBlock && b.parent().and_then(|m| m.name()).is_some_and(|x| x.kind() != Kind::StringLiteral))
        });
        let sym = self.resolve_alias(sym);
        let own = self.own_name(sym);
        if exported != own && !in_namespace && !matches!(exported, "export=" | "default") && !exported.is_empty() {
            self.exported_as.entry(sym).or_insert_with(|| exported.to_string());
        }
        if sym.flags().intersects(SymbolFlags::TypeParameter) {
            return;
        }
        // The outermost namespace it is in; a module block (`declare module "fs"`) is not one.
        let mut top = sym;
        while let Some(parent) = self.c.get_parent_of_symbol_exported(top) {
            if parent.declarations().iter().any(|d| d.kind() == Kind::ModuleDeclaration && d.name().is_some_and(|n| n.kind() != Kind::StringLiteral)) {
                top = parent;
            } else {
                break;
            }
        }
        if top.declarations().iter().any(|d| ast::get_source_file_of_node(*d).is_some_and(|f| self.is_package_file(&f)))
            && self.queued.insert(top)
        {
            self.pending.push_back(top);
        }
    }

    fn emit_symbol(&mut self, sym: P<Symbol>, out: &mut Vec<Decl>) {
        let decls: Vec<P<Node>> = sym.declarations().to_vec();
        for d in decls {
            if ast::get_source_file_of_node(d).is_some_and(|f| self.is_package_file(&f)) {
                self.emit_top(d, out);
            }
        }
    }

    // A top-level declaration, noting the package it comes from: the ambient
    // module it is declared in (`declare module "fs"`), else its file's.
    fn emit_top(&mut self, node: P<Node>, out: &mut Vec<Decl>) {
        let before = out.len();
        self.emit_decl_node(node, out);
        let ambient = std::iter::successors(node.parent(), |n| n.parent())
            .find(|n| n.kind() == Kind::ModuleDeclaration && n.name().is_some_and(|m| m.kind() == Kind::StringLiteral))
            .and_then(|m| m.name().map(|n| text(n).trim_matches(|c| c == '"' || c == '\'').to_string()))
            // `node:events` is `events`: one module by two names.
            .and_then(|m| self.packages.iter().find(|p| p.1.trim_start_matches("node:") == m.trim_start_matches("node:")).map(|p| p.1.clone()));
        let package = ambient.or_else(|| ast::get_source_file_of_node(node).and_then(|f| self.package_of(f.file_name()).map(str::to_string)));
        if let Some(package) = package {
            for d in &out[before..] {
                self.package_of_name.insert(decl_name(d).to_string(), package.clone());
            }
        }
    }

    fn emit_decl_node(&mut self, s: P<Node>, out: &mut Vec<Decl>) {
        match s.kind() {
            Kind::InterfaceDeclaration => out.extend(self.interface(s).map(Decl::Interface)),
            Kind::ClassDeclaration => out.extend(self.class(s).map(Decl::Interface)),
            Kind::TypeAliasDeclaration => {
                if let Some((sym, _)) = self.first_time(s) {
                    out.push(Decl::Alias(Alias {
                        name: self.output_name(sym),
                        sym: Some(sym),
                        type_params: self.type_params(s.type_parameters()),
                        ty: self.opt_type(s.type_node()),
                        computed: self.is_computed_alias(sym),
                        synthetic: false,
                    }));
                }
            }
            Kind::FunctionDeclaration => {
                if self.emitted_nodes.insert(s) {
                    let name = match self.c.get_symbol_of_declaration_exported(s) {
                        Some(sym) => self.output_name(sym),
                        None => s.name().map(text).unwrap_or_else(|| anonymous_name(s)),
                    };
                    let js_name = self.c.get_symbol_of_declaration_exported(s).and_then(|sym| self.renamed_from(sym));
                    out.push(Decl::Function(Function { name, js_name, sig: self.signature(s) }));
                }
            }
            Kind::EnumDeclaration => out.extend(self.enumeration(s).map(Decl::Enum)),
            // `declare module "fs" { }` or an augmentation, `declare module "./Node.js"
            // { }`: not a namespace. A module is bound as a package; what an
            // augmentation declares the checker has merged into what it augments.
            Kind::ModuleDeclaration if s.name().is_some_and(|n| n.kind() == Kind::StringLiteral) => {}
            // `declare global { }` adds to the global scope's declarations: the library's, not the package's.
            Kind::ModuleDeclaration if ast::is_global_scope_augmentation(s) => {}
            Kind::ModuleDeclaration => {
                if let Some(ns) = self.namespace(s) {
                    out.push(Decl::Namespace(ns));
                }
            }
            Kind::VariableDeclaration => {
                if let Some((sym, _)) = self.first_time(s) {
                    // A `const`, or any variable at the top of a module: a module's
                    // exports cannot be assigned from outside it.
                    let constant = ast::get_combined_node_flags(s).intersects(ast::NodeFlags::Constant);
                    let statement = s.parent().and_then(|list| list.parent());
                    let module_top = statement.and_then(|st| st.parent()).is_some_and(|f| f.kind() == Kind::SourceFile)
                        && ast::get_source_file_of_node(s).is_some_and(|f| f.external_module_indicator().is_some());
                    let name = self.output_name(sym);
                    let js_name = self.renamed_from(sym);
                    out.push(Decl::Var(Var { name, ty: self.opt_type(s.type_node()), js_name, readonly: constant || module_top }));
                }
            }
            Kind::VariableStatement => {
                for d in s.as_variable_statement().declaration_list().as_variable_declaration_list().declarations().nodes() {
                    self.emit_decl_node(*d, out);
                }
            }
            Kind::ExportDeclaration => {
                let ed = s.as_export_declaration();
                match ed.export_clause() {
                    None => {
                        if let Some(spec) = ed.module_specifier()
                            && let Some(mod_sym) = self.c.resolve_external_module_name_exported(spec, None)
                        {
                            for sym in self.c.get_exports_of_module_exported(mod_sym) {
                                self.enqueue_symbol(sym);
                            }
                        }
                    }
                    Some(clause) if clause.kind() == Kind::NamedExports => {
                        for &spec in clause.as_named_exports().elements().nodes() {
                            if let Some(sym) = self.c.get_symbol_of_declaration_exported(spec) {
                                self.enqueue_symbol(sym);
                            }
                        }
                    }
                    Some(clause) if clause.kind() == Kind::NamespaceExport => {
                        if let Some(spec) = ed.module_specifier()
                            && let Some(mod_sym) = self.c.resolve_external_module_name_exported(spec, None)
                        {
                            let mut decls = Vec::new();
                            for sym in self.c.get_exports_of_module_exported(mod_sym) {
                                let target = self.resolve_alias(sym);
                                if target.declarations().iter().any(|d| ast::get_source_file_of_node(*d).is_some_and(|f| self.is_package_file(&f))) {
                                    self.emit_symbol(target, &mut decls);
                                }
                            }
                            out.push(Decl::Namespace(Namespace { name: text(clause.as_namespace_export().name()), js_name: None, decls }));
                        }
                    }
                    _ => {}
                }
            }
            // `export default X` and `export = X`: imported as `default`.
            Kind::ExportAssignment => {
                if let Some(sym) = self.c.get_symbol_at_location_exported(s.as_export_assignment().expression()) {
                    let target = self.resolve_alias(sym);
                    self.exported_as.insert(target, "default".into());
                    self.enqueue_symbol(sym);
                }
            }
            _ => {}
        }
    }

    pub fn statements(&mut self, stmts: &[P<Node>]) -> Vec<Decl> {
        let mut out = Vec::new();
        for &s in stmts {
            self.emit_decl_node(s, &mut out);
        }
        out
    }

    fn interface(&mut self, node: P<Node>) -> Option<Interface> {
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
            type_params: self.type_params(node.type_parameters()),
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
    fn class(&mut self, node: P<Node>) -> Option<Interface> {
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
            type_params: self.type_params(node.type_parameters()),
            extends,
            ancestors: Vec::new(),
            ancestor_syms: Vec::new(),
            members,
            class: Some(class),
            synthetic: false,
            js_name: self.renamed_from(sym),
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
        Some(Enum { name: self.output_name(sym), sym: Some(sym), members, synthetic: false })
    }

    fn namespace(&mut self, node: P<Node>) -> Option<Namespace> {
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
        let mut args = self.types(args);
        // `Float32Array` means `Float32Array<ArrayBufferLike>`: spell out the
        // defaults, so every reference has all its type arguments.
        if let Some(s) = sym {
            let params = s
                .declarations()
                .iter()
                .filter(|d| matches!(d.kind(), Kind::InterfaceDeclaration | Kind::ClassDeclaration | Kind::TypeAliasDeclaration))
                .map(|d| d.type_parameters())
                .find(|p| !p.is_empty())
                .unwrap_or(&[]);
            for p in params.iter().skip(args.len()) {
                let default = p.as_type_parameter_declaration().default_type().map(|d| self.ty(d));
                // A default may name an earlier parameter, which means nothing here.
                let usable = default.filter(|d| !crate::xml::type_key(d).contains("kind=\"typeParameter\""));
                args.push(usable.unwrap_or_else(|| Type::prim("any")));
            }
        }
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

    fn make_ref(&mut self, sym: Option<P<Symbol>>, args: Vec<Type>) -> Ref {
        let Some(s) = sym else {
            return Ref { name: String::new(), kind: "unresolved", sym: None, args, tid: None, qualified: None, file: None, computed: false, node: None };
        };
        let s = self.resolve_alias(s);
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
            while let Some(parent) = self.c.get_parent_of_symbol_exported(top) {
                if parent.declarations().iter().any(|d| d.kind() == Kind::ModuleDeclaration && d.name().is_some_and(|n| n.kind() != Kind::StringLiteral)) {
                    top = parent;
                    hops += 1;
                } else {
                    break;
                }
            }
            let segments: Vec<&str> = fqn.split('.').collect();
            if hops == 0 || segments.len() <= hops { fqn } else { format!("{}.{}", self.output_name(top), segments[segments.len() - hops..].join(".")) }
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

    // A type the checker made, as the IR has it. Types that keep making new
    // ones (`DeepKeys<T>` through every property of every property) stop, at
    // a depth or once one conversion has made too many, as an `any` that
    // says so.
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
            let mergeable = t.types().iter().all(|p| p.flags().intersects(TypeFlags::Object));
            // An alias's own body is written once, under its name, whatever its size.
            if mergeable && (skip.is_some() || self.c.get_properties_of_type_exported(t).len() <= self.intersection_limit) {
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
            let key = Param { name: "key".into(), optional: false, rest: false, constant: None, ty: self.from_type(info.key_type()) };
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
                    for variant in self.keyed_variants(*sig) {
                        let mut m = Member::new(MemberKind::Method, name.clone());
                        m.optional = prop.flags().intersects(SymbolFlags::Optional);
                        m.from = from.clone();
                        m.sig = Some(variant);
                        out.push(m);
                    }
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

    // `on<K extends "a" | "b">(type: K, f: (ev: Map[K]) => void)`: one signature
    // per key, the key a fixed argument and `Map[K]` its property's type.
    fn keyed_variants(&mut self, sig: P<CSignature>) -> Vec<Signature> {
        if self.expand_keyed
            && let Some((tp, keys)) = self.keyed_parameter(sig)
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
                let s = self.c.get_merged_symbol_exported(r.sym?);
                match r.kind {
                    "enum" => None,
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
            Type::Ref(r) if matches!(r.kind, "interface" | "class" | "typeAlias") && r.args.is_empty() => {
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
                    let literal = match pt.flags().intersects(TypeFlags::StringLiteral | TypeFlags::NumberLiteral).then(|| pt.as_literal_type().value()) {
                        Some(Some(LiteralValue::String(s))) => Some(s.to_string()),
                        Some(Some(LiteralValue::Number(n))) => Some(n.to_string()),
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

    fn from_signature(&mut self, sig: P<CSignature>) -> Signature {
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
            let mut ty = self.from_type(t);
            if optional {
                ty = strip_undefined(ty);
            }
            // `__0`: a parameter destructured where it was declared.
            let name = if p.name().starts_with("__") { destructured_name(&ty, i) } else { p.name().to_string() };
            out.params.push(Param { name, optional, rest, constant: None, ty });
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
    // Ancestors by their names in the output, as references have them, with
    // their symbols.
    pub fn flattened(&mut self, sym: P<Symbol>) -> (Vec<Member>, Vec<(String, Sym)>) {
        let t = self.c.get_declared_type_of_symbol_exported(sym);
        let members = self.members_of_type(t, Some(sym));
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

// The name of `export default class { }`: its file's, or for an `index`
// file its directory's, as a JavaScript name: `anon_default`, `widget`.
fn anonymous_name(node: P<Node>) -> String {
    let file = ast::get_source_file_of_node(node).map(|f| f.file_name().to_string()).unwrap_or_default();
    let mut parts = file.rsplit('/');
    let base = parts.next().unwrap_or_default();
    let stem = [".d.ts", ".d.mts", ".d.cts", ".mts", ".cts", ".ts"].iter().find_map(|e| base.strip_suffix(e)).unwrap_or(base);
    let stem = if stem == "index" { parts.next().unwrap_or(stem) } else { stem };
    let name: String = stem.chars().map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' }).collect();
    if name.is_empty() || name.starts_with(|c: char| c.is_ascii_digit()) { format!("_{name}") } else { name }
}

fn decl_name(d: &Decl) -> &str {
    match d {
        Decl::Interface(i) => &i.name,
        Decl::Alias(a) => &a.name,
        Decl::Function(f) => &f.name,
        Decl::Var(v) => &v.name,
        Decl::Enum(e) => &e.name,
        Decl::Namespace(n) => &n.name,
    }
}

// A destructured parameter's name, `{ years, months }: Duration`: its type's,
// `duration`, else its place, `arg0`.
fn destructured_name(ty: &Type, index: usize) -> String {
    match ty {
        Type::Ref(r) if !r.name.is_empty() && r.name.chars().all(|c| c.is_alphanumeric() || c == '_') => {
            let mut cs = r.name.chars();
            cs.next().map(|c| c.to_lowercase().collect::<String>() + cs.as_str()).unwrap_or_default()
        }
        _ => format!("arg{index}"),
    }
}

