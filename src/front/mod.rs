// Front end: tsrs AST -> IR (as written), and checker Type -> IR (resolved).

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use tsrs_ast::{self as ts_ast, Kind, ModifierFlags, Node, SourceFile, Symbol, SymbolFlags, has_syntactic_modifier};
use tsrs_checker::checker::Checker;
use tsrs_checker::types::{TypeFlags, TypeId};
use tsrs_core::P;
use tsrs_scanner::get_text_of_node;

use crate::ir::*;

mod ast;
mod doc;
mod types;

struct PackageEntry {
    dir: String,
    entry: String,
    name: String,
}

pub struct Front<'c> {
    pub c: &'c mut Checker,
    pub root_file: String,
    // Packages being bound, by installation dir, resolved entry file, and
    // import specifier. Empty: the input file and what it imports outside
    // node_modules.
    packages: Vec<PackageEntry>,
    // Package specifier currently being collected, when binding multiple entries.
    pub current_package: Option<String>,
    // Top-level symbols exported by a requested package/subpath -> its specifier.
    exported_by_package: HashMap<P<Symbol>, String>,
    // The package each top-level declaration comes from, by its output name.
    pub package_of_name: HashMap<String, String>,
    // Packages that are ambient modules (`@types/node`'s `fs`): what their
    // files declare outside a `declare module` is global.
    pub ambient_packages: HashSet<String>,
    // Where each top-level declaration comes from, as a word to name it by
    // where its name must give way: `readline` for node's `Key`.
    pub origin_of_name: HashMap<String, String>,
    lib_path: String,
    // Merged symbols already converted, by declaration kind and symbol identity.
    seen: HashSet<(Kind, P<Symbol>)>,
    // Declarations exported under another name than their own, by their own.
    // What a symbol is exported as, when not its own name.
    exported_as: HashMap<P<Symbol>, String>,
    // Symbols a module exports by their own names.
    exported_plain: HashSet<P<Symbol>>,
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
    aliases_in_progress: Vec<(P<Symbol>, Vec<TypeId>)>,
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
    // Within a generic expression's constraint, which the checker makes.
    widening: usize,
    pub intersection_limit: usize,
    // Members written with a union type, where the checker rebuilt them (flattened,
    // evaluated): `owner\u{1}member`, as lowering paths them.
    pub written_members: HashSet<String>,
    // Lowering: expand a signature over the keys its type parameter may take.
    pub expand_keyed: bool,
    // Keep keyed signatures generic: `K extends keyof M` and `M[K]` stay.
    pub map_keyed: bool,
    // Members copied into each subtype keep a return type written `this`.
    pub(crate) keep_this: bool,
    kept_keyed: HashSet<TypeId>,
    pub keyed_limit: usize,
    // During expansion: the type parameter being replaced, and its key.
    key_binding: Option<(TypeId, String)>,
    skip_alias: Option<P<Symbol>>,
    // Types owned by an explicit `--ref` binding directory: skip transitive re-emission.
    pub external_ref_types: HashSet<String>,
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
            current_package: None,
            exported_by_package: HashMap::new(),
            package_of_name: HashMap::new(),
            ambient_packages: HashSet::new(),
            origin_of_name: HashMap::new(),
            lib_path,
            seen: HashSet::new(),
            exported_as: HashMap::new(),
            exported_plain: HashSet::new(),
            output_names: HashMap::new(),
            named: HashMap::new(),
            emitted_nodes: HashSet::new(),
            queued: HashSet::new(),
            pending: VecDeque::new(),
            stats: BTreeMap::new(),
            computed_alias: HashMap::new(),
            in_progress: Vec::new(),
            aliases_in_progress: Vec::new(),
            depth: 0,
            budget: BUDGET,
            cut: 0,
            converted: HashMap::new(),
            merge_intersections: false,
            widening: 0,
            intersection_limit: 64,
            expand_keyed: false,
            map_keyed: false,
            keep_this: false,
            kept_keyed: HashSet::new(),
            keyed_limit: 0,
            key_binding: None,
            skip_alias: None,
            written_members: HashSet::new(),
            external_ref_types: HashSet::new(),
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
        let kind = node.kind();
        if !self.seen.insert((kind, sym)) {
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
        // A module's values it does not export are not there to call (marked's
        // `declare const markedInstance`); its types may be named by others.
        let exported: HashSet<P<Symbol>> = match file.symbol().filter(|_| file.external_module_indicator().is_some()) {
            Some(module) => {
                let module = self.c.get_merged_symbol_exported(module);
                self.c.get_exports_of_module_exported(module).iter().map(|e| self.resolve_alias(*e)).collect()
            }
            None => HashSet::new(),
        };
        let module_file = file.external_module_indicator().is_some();
        let mut out = Vec::new();
        for &s in file.statements().nodes() {
            if module_file && matches!(s.kind(), Kind::VariableStatement | Kind::FunctionDeclaration) && !has_syntactic_modifier(s, ModifierFlags::Export) {
                let symbols: Vec<P<Symbol>> = match s.kind() {
                    Kind::VariableStatement => s.as_variable_statement().declaration_list().as_variable_declaration_list().declarations().nodes().iter().filter_map(|d| self.c.get_symbol_of_declaration_exported(*d)).collect(),
                    _ => self.c.get_symbol_of_declaration_exported(s).into_iter().collect(),
                };
                if symbols.iter().all(|sym| !exported.contains(&self.c.get_merged_symbol_exported(*sym))) {
                    continue;
                }
            }
            self.emit_top(s, &mut out);
        }
        while let Some(sym) = self.pending.pop_front() {
            self.emit_symbol(sym, &mut out);
        }
        if let Some(module) = file.symbol() {
            let module = self.c.get_merged_symbol_exported(module);
            self.held_classes(module, &mut out);
        }
        self.exports(&mut out);
        out
    }

    // A class the module exports only as a property of an object it exports
    // is reached there: Konva's `Stage` is `default.Stage`, the module
    // exporting `Konva`, of `Stage: typeof Stage`, as its default.
    fn held_classes(&mut self, module: P<Symbol>, out: &mut [Decl]) {
        let exports = self.c.get_exports_of_module_exported(module);
        let direct: HashSet<P<Symbol>> = exports.iter().map(|e| self.resolve_alias(*e)).collect();
        let mut held: HashMap<String, String> = HashMap::new();
        for e in &exports {
            let v = self.resolve_alias(*e);
            if !v.flags().intersects(SymbolFlags::Variable) {
                continue;
            }
            let as_ = if e.name() == "export=" { "default".to_string() } else { e.name().to_string() };
            let t = self.c.get_type_of_symbol_exported(v);
            for p in self.c.get_properties_of_type_exported(t) {
                let pt = self.c.get_type_of_symbol_exported(*p);
                let Some(class) = pt.symbol().filter(|c| c.flags().intersects(SymbolFlags::Class)) else { continue };
                let class = self.c.get_merged_symbol_exported(class);
                if direct.contains(&class) {
                    continue;
                }
                if let Some(name) = self.output_names.get(&class) {
                    held.entry(name.clone()).or_insert_with(|| format!("{as_}.{}", p.name()));
                }
            }
        }
        for d in out.iter_mut() {
            if let Decl::Interface(i) = d
                && i.class.is_some()
                && i.js_name.is_none()
                && let Some(path) = held.get(&i.name)
            {
                i.js_name = Some(path.clone());
            }
        }
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
        let mut declared: HashSet<String> = out.iter().filter_map(name_of).collect();
        // A name held by what is not exported by it gives way: rollup's
        // `BarSeriesOption$1`, exported as `BarSeriesOption`, takes the name
        // from the local interface it extends, which takes `BarSeriesOption$1`.
        let plain: HashSet<String> = self.exported_plain.iter().filter_map(|s| self.output_names.get(s).cloned()).collect();
        // By the name each exported symbol has in the output.
        let by_name: HashMap<String, String> = self
            .exported_as
            .iter()
            .map(|(s, e)| (self.output_names.get(s).cloned().unwrap_or_else(|| self.own_name(*s)), e.clone()))
            .collect();
        let mut renames = Vec::new();
        let mut swaps = Vec::new();
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
            // another name, not a rename of it; one only exported by another
            // name (rollup's `color_d`, exported as `color`) is known by it.
            } else if let Decl::Namespace(n) = d {
                if exported != name && !declared.contains(&exported) {
                    n.name = exported.clone();
                    declared.insert(exported.clone());
                    renames.push((name, exported));
                }
            } else if exported != name {
                if !declared.contains(&exported) {
                    renames.push((name, exported));
                } else if !plain.contains(&exported) && !by_name.contains_key(&exported) && !renames.iter().any(|(_, n)| *n == name || *n == exported) {
                    swaps.push((name, exported));
                }
            }
        }
        for (name, exported) in swaps {
            declared.insert(name.clone());
            renames.push((exported.clone(), name.clone()));
            renames.push((name, exported));
        }
        crate::lower::rename_everywhere(out, &renames);
        // References made after this (union tests, in lowering) have the new names.
        for name in self.output_names.values_mut() {
            if let Some((_, new)) = renames.iter().find(|(old, _)| old == name) {
                *name = new.clone();
            }
        }
        // All at once: a swap renames each to the other.
        let origins: Vec<_> = renames.iter().map(|(old, new)| (new.clone(), self.origin_of_name.remove(old), self.package_of_name.remove(old))).collect();
        for (new, origin, package) in origins {
            if let Some(o) = origin {
                self.origin_of_name.insert(new.clone(), o);
            }
            if let Some(p) = package {
                self.package_of_name.insert(new, p);
            }
        }
    }

    // A package that is an ambient module, `declare module "fs" { }`: what it exports.
    pub fn collect_module(&mut self, module: P<Symbol>) -> Vec<Decl> {
        let module = self.c.get_merged_symbol_exported(module);
        // `export = path`: the module is `path`, imported as `default`.
        if let Some(target) = module.exports().and_then(|t| t.lookup("export=")) {
            let target = self.resolve_alias(target);
            self.exported_as.insert(target, "default".into());
        }
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
        self.packages.push(PackageEntry { dir, entry: entry.to_string(), name: name.to_string() });
    }

    // Records resolved symbols exported by package/subpath `name` so sibling
    // subpaths sharing a `node_modules/<pkg>` directory keep distinct origins.
    pub fn register_package_exports(&mut self, name: &str, module: P<Symbol>) {
        for sym in self.c.get_exports_of_module_exported(module) {
            let target = self.resolve_alias(sym);
            self.exported_by_package.entry(target).or_insert_with(|| name.to_string());
        }
    }

    // The package a file belongs to, of those being bound.
    fn package_of(&self, file: &str) -> Option<&str> {
        self.packages
            .iter()
            .find(|p| p.entry == file)
            .or_else(|| {
                self.packages.iter().find(|p| {
                    file.starts_with(p.dir.as_str()) && matches!(file.as_bytes().get(p.dir.len()), None | Some(b'/')) && !file[p.dir.len()..].contains("/node_modules/")
                })
            })
            .map(|p| p.name.as_str())
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

    // A transitive npm module dependency outside the requested package(s),
    // excluding bundled TS libs and ambient script files (`@types/node`,
    // `@webgpu/types`), which define host globals rather than imported APIs.
    pub(super) fn is_transitive_file(&self, sf: &SourceFile) -> bool {
        let name = sf.file_name();
        if self.root_file.starts_with(&self.lib_path) || name.starts_with(&self.lib_path) {
            return false;
        }
        if self.is_package_file(sf) {
            return false;
        }
        name.contains("/node_modules/") && sf.external_module_indicator().is_some()
    }

    pub(super) fn is_emittable_file(&self, sf: &SourceFile) -> bool {
        self.is_package_file(sf) || self.is_transitive_file(sf)
    }

    pub(super) fn is_transitive_type_symbol(&self, sym: P<Symbol>) -> bool {
        sym.flags().intersects(
            SymbolFlags::Type
                | SymbolFlags::Class
                | SymbolFlags::Interface
                | SymbolFlags::TypeAlias
                | SymbolFlags::RegularEnum
                | SymbolFlags::ConstEnum
                | SymbolFlags::Namespace,
        ) && !self.external_ref_types.contains(&self.own_name(sym))
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
            ts_ast::get_source_file_of_node(*d).is_some_and(|f| self.is_emittable_file(&f))
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
        let exported = sym.name().to_string();
        let sym = self.resolve_alias(sym);
        if exported == self.own_name(sym) {
            self.exported_plain.insert(sym);
        }
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
            if parent.declarations().iter().any(|d| d.kind() == Kind::ModuleDeclaration && d.name().is_some_and(|n| n.kind() != Kind::StringLiteral) && !ts_ast::is_global_scope_augmentation(*d)) {
                top = parent;
            } else {
                break;
            }
        }
        if top.declarations().iter().any(|d| {
            ts_ast::get_source_file_of_node(*d)
                .is_some_and(|f| self.is_package_file(&f) || (self.is_transitive_file(&f) && self.is_transitive_type_symbol(top)))
        }) && self.queued.insert(top)
        {
            self.pending.push_back(top);
        }
    }

    // A variable or function `export { local as exported }` brings into a
    // namespace, by its exported name: false when it is not one, is out
    // already, or is exported by its own module too (node's `finished`, also
    // `finished.__promisify__`), where it stays.
    fn emit_as(&mut self, alias: P<Symbol>, out: &mut Vec<Decl>) -> bool {
        let target = self.resolve_alias(alias);
        let value = SymbolFlags::Variable | SymbolFlags::Function;
        if !target.flags().intersects(value) || target.flags().intersects(!value & SymbolFlags::Value) || self.c.get_parent_of_symbol_exported(target).is_some() {
            return false;
        }
        let before = out.len();
        // The value only: `graphic_d_Arc` is a type too, which stays where it is.
        for d in target.declarations().to_vec() {
            if matches!(d.kind(), Kind::VariableDeclaration | Kind::FunctionDeclaration) && ts_ast::get_source_file_of_node(d).is_some_and(|f| self.is_package_file(&f)) {
                self.emit_decl_node(d, out);
            }
        }
        for d in &mut out[before..] {
            match d {
                Decl::Var(v) => (v.name, v.js_name) = (alias.name().to_string(), None),
                Decl::Function(f) => (f.name, f.js_name) = (alias.name().to_string(), None),
                _ => {}
            }
        }
        out.len() > before
    }

    fn emit_symbol(&mut self, sym: P<Symbol>, out: &mut Vec<Decl>) {
        let decls: Vec<P<Node>> = sym.declarations().to_vec();
        for d in decls {
            let Some(f) = ts_ast::get_source_file_of_node(d) else { continue };
            if self.is_package_file(&f) {
                self.emit_top(d, out);
            } else if self.is_transitive_file(&f)
                && matches!(
                    d.kind(),
                    Kind::InterfaceDeclaration
                        | Kind::ClassDeclaration
                        | Kind::TypeAliasDeclaration
                        | Kind::EnumDeclaration
                        | Kind::ModuleDeclaration
                )
                && self
                    .c
                    .get_symbol_of_declaration_exported(d)
                    .map(|s| self.c.get_merged_symbol_exported(s))
                    .is_none_or(|s| !self.external_ref_types.contains(&self.own_name(s)))
            {
                self.emit_top(d, out);
            }
        }
    }

    // A top-level declaration, noting the package it comes from: the ambient
    // module it is declared in (`declare module "fs"`), else its file's.
    fn emit_top(&mut self, node: P<Node>, out: &mut Vec<Decl>) {
        let before = out.len();
        self.emit_decl_node(node, out);
        // In `declare global`, or in an ambient-module package's file outside
        // any `declare module` (`@types/node`'s `console`, `NodeJS`), or in a
        // transitive type-only dependency: no runtime package import.
        let global = std::iter::successors(node.parent(), |n| n.parent()).any(|n| n.kind() == Kind::ModuleDeclaration && ts_ast::is_global_scope_augmentation(n));
        let transitive = ts_ast::get_source_file_of_node(node).is_some_and(|f| self.is_transitive_file(&f));
        let ambient = std::iter::successors(node.parent(), |n| n.parent())
            .find(|n| n.kind() == Kind::ModuleDeclaration && n.name().is_some_and(|m| m.kind() == Kind::StringLiteral))
            .and_then(|m| m.name().map(|n| text(n).trim_matches(|c| c == '"' || c == '\'').to_string()))
            // `node:events` is `events`: one module by two names; one not bound
            // is its own module still (`stream`, `readline`), where JavaScript
            // imports it from.
            .map(|m| self.packages.iter().find(|p| p.name.trim_start_matches("node:") == m.trim_start_matches("node:")).map_or_else(|| m.trim_start_matches("node:").to_string(), |p| p.name.clone()));
        // A wildcard module (pixi's `declare module '*.frag'`) types files a
        // bundler imports, not a module JavaScript can: nothing to bind.
        if ambient.as_deref().is_some_and(|m| m.contains('*')) {
            out.truncate(before);
            return;
        }
        let symbol_package = |this: &mut Self, n: P<Node>| -> Option<String> {
            let target = match n.kind() {
                Kind::VariableStatement => n
                    .as_variable_statement()
                    .declaration_list()
                    .as_variable_declaration_list()
                    .declarations()
                    .nodes()
                    .first()
                    .copied(),
                _ => Some(n),
            }?;
            let sym = this.c.get_symbol_of_declaration_exported(target)?;
            let sym = this.c.get_merged_symbol_exported(sym);
            this.exported_by_package.get(&sym).cloned()
        };
        let package = if global || transitive {
            None
        } else {
            ambient.or_else(|| {
                let file_name = ts_ast::get_source_file_of_node(node).map(|f| f.file_name().to_string());
                let by_export = symbol_package(self, node);
                let by_entry = file_name.as_deref().and_then(|fnm| self.packages.iter().find(|p| p.entry == fnm).map(|p| p.name.clone()));
                let by_current = self.current_package.as_ref().filter(|cur| {
                    file_name.as_deref().is_some_and(|fnm| {
                        self.packages.iter().find(|p| &p.name == *cur).is_some_and(|p| {
                            fnm.starts_with(p.dir.as_str())
                                && matches!(fnm.as_bytes().get(p.dir.len()), None | Some(b'/'))
                                && !fnm[p.dir.len()..].contains("/node_modules/")
                        })
                    })
                }).cloned();
                let file_package = by_export
                    .or(by_entry)
                    .or(by_current)
                    .or_else(|| file_name.as_deref().and_then(|f| self.package_of(f).map(str::to_string)));
                file_package.filter(|p| !self.ambient_packages.contains(p))
            })
        };
        // Its ambient module (`node:fs/promises` is `fs/promises`), else its file
        // (`readline.d.ts`), else its directory where the file is an index.
        let module = std::iter::successors(node.parent(), |n| n.parent())
            .find(|n| n.kind() == Kind::ModuleDeclaration && n.name().is_some_and(|m| m.kind() == Kind::StringLiteral))
            .and_then(|m| m.name().map(|n| text(n).trim_matches(|c| c == '"' || c == '\'').to_string()));
        let file = ts_ast::get_source_file_of_node(node).map(|f| {
            let path = f.file_name().to_string();
            let mut parts = path.rsplit('/');
            let stem = parts.next().unwrap_or_default().split('.').next().unwrap_or_default().to_string();
            if stem == "index" { parts.next().unwrap_or_default().to_string() } else { stem }
        });
        let origin = module.map(|m| m.trim_start_matches("node:").to_string()).or(file).filter(|o| !o.is_empty());
        if let Some(origin) = origin {
            for d in &out[before..] {
                self.origin_of_name.insert(d.name().to_string(), origin.clone());
            }
        }
        if let Some(package) = package {
            for d in &out[before..] {
                self.package_of_name.insert(d.name().to_string(), package.clone());
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
                        extends: Vec::new(),
                        ty: self.opt_type(s.type_node()),
                        computed: self.is_computed_alias(sym),
                        synthetic: false,
                        doc: self.doc(s),
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
                    out.push(Decl::Function(Function { name, js_name, sig: self.signature(s), doc: self.doc(s) }));
                }
            }
            Kind::EnumDeclaration => out.extend(self.enumeration(s).map(Decl::Enum)),
            // `declare module "fs" { }` or an augmentation, `declare module "./Node.js"
            // { }`: not a namespace. A module is bound as a package; what an
            // augmentation declares the checker has merged into what it augments.
            Kind::ModuleDeclaration if s.name().is_some_and(|n| n.kind() == Kind::StringLiteral) => {}
            // `declare global { }` adds to the global scope's declarations: the library's, not the package's.
            Kind::ModuleDeclaration if ts_ast::is_global_scope_augmentation(s) => {}
            Kind::ModuleDeclaration => {
                if let Some(ns) = self.namespace(s) {
                    out.push(Decl::Namespace(ns));
                }
            }
            Kind::VariableDeclaration => {
                if let Some((sym, _)) = self.first_time(s) {
                    // A `const`, or any variable at the top of a module: a module's
                    // exports cannot be assigned from outside it.
                    let constant = ts_ast::get_combined_node_flags(s).intersects(ts_ast::NodeFlags::Constant);
                    let statement = s.parent().and_then(|list| list.parent());
                    let module_top = statement.and_then(|st| st.parent()).is_some_and(|f| f.kind() == Kind::SourceFile)
                        && ts_ast::get_source_file_of_node(s).is_some_and(|f| f.external_module_indicator().is_some());
                    let name = self.output_name(sym);
                    let js_name = self.renamed_from(sym);
                    out.push(Decl::Var(Var { name, ty: self.opt_type(s.type_node()), js_name, readonly: constant || module_top, doc: self.doc(s) }));
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
                        // A namespace's (`namespace color_d { export { color_d_parse as parse } }`):
                        // a value is reached through it, by the name it exports.
                        let in_namespace = ed.module_specifier().is_none()
                            && s.parent().is_some_and(|b| b.kind() == Kind::ModuleBlock && b.parent().and_then(|m| m.name()).is_some_and(|n| n.kind() != Kind::StringLiteral));
                        for &spec in clause.as_named_exports().elements().nodes() {
                            let Some(sym) = self.c.get_symbol_of_declaration_exported(spec) else { continue };
                            if in_namespace && self.emit_as(sym, out) {
                                continue;
                            }
                            self.enqueue_symbol(sym);
                        }
                    }
                    Some(clause) if clause.kind() == Kind::NamespaceExport => {
                        if let Some(spec) = ed.module_specifier()
                            && let Some(mod_sym) = self.c.resolve_external_module_name_exported(spec, None)
                        {
                            let mut decls = Vec::new();
                            for sym in self.c.get_exports_of_module_exported(mod_sym) {
                                let target = self.resolve_alias(sym);
                                if target.declarations().iter().any(|d| ts_ast::get_source_file_of_node(*d).is_some_and(|f| self.is_package_file(&f))) {
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
    let file = ts_ast::get_source_file_of_node(node).map(|f| f.file_name().to_string()).unwrap_or_default();
    let mut parts = file.rsplit('/');
    let base = parts.next().unwrap_or_default();
    let stem = [".d.ts", ".d.mts", ".d.cts", ".mts", ".cts", ".ts"].iter().find_map(|e| base.strip_suffix(e)).unwrap_or(base);
    let stem = if stem == "index" { parts.next().unwrap_or(stem) } else { stem };
    let name: String = stem.chars().map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' }).collect();
    if name.is_empty() || name.starts_with(|c: char| c.is_ascii_digit()) { format!("_{name}") } else { name }
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

// Whether a type names a type parameter anywhere in it.
fn mentions_type_param(t: &Type) -> bool {
    let mut found = false;
    let mut t = t.clone();
    crate::lower::visit(&mut t, &mut |t| {
        found |= matches!(t, Type::Ref(r) if r.kind == "typeParameter");
        !found
    });
    found
}

// A merged declaration's type parameters: the first of its declarations
// that has any (pixi's `interface Container` five times, `<C>` once).
fn declared_type_params(sym: P<Symbol>, node: P<Node>) -> &'static [P<Node>] {
    sym.declarations()
        .iter()
        .filter(|d| matches!(d.kind(), Kind::InterfaceDeclaration | Kind::ClassDeclaration))
        .map(|d| d.type_parameters())
        .find(|p| !p.is_empty())
        .unwrap_or(node.type_parameters())
}
