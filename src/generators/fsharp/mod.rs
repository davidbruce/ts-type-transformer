// XML -> F# bindings for Fable. Keeps inheritance, generics, overloads,
// optional parameters and unions; needs literals and type-level expressions gone.

mod classes;
mod decls;
mod hoist;
mod member_ir;
mod members;
mod plan;
mod syntax;
mod types;

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::Path;

use roxmltree::{Document, Node};

use super::{Problems, attr, elements, named, relative_path};
use member_ir::FsMember;
use plan::{constrained_interfaces, declared_files, file_plan, package_modules, reaches_in};
use syntax::{pascal, shift, type_ident};

const T: &str = "F#";

#[derive(Clone, Debug)]
pub struct ProjectRef {
    /// Path written into `<ProjectReference Include="..." />` (relative to the generated `.fsproj`).
    pub include_path: String,
    /// Root F# namespace of the referenced binding project (e.g. `Dom`, `Webgpu.Types`).
    pub namespace: String,
    /// Top-level types exported by the referenced project: F# name -> `(arity, is_interface, abbrev_target)`.
    pub types: HashMap<String, (usize, bool, Option<String>)>,
}

// A declaration's documentation as `///` lines at `indent`.
pub(super) fn doc_lines(n: roxmltree::Node, indent: &str) -> String {
    super::doc(n).map(|d| super::comments::fsharp(d, indent)).unwrap_or_default()
}

// A member's: see `member_doc`.
pub(super) fn member_doc_lines(n: roxmltree::Node, indent: &str) -> String {
    super::member_doc(n).map(|d| super::comments::fsharp(&d, indent)).unwrap_or_default()
}

/// Loads a previously generated F# binding project (via `tsrs-meta.toml`)
/// so another generated binding project can reference its types
/// directly instead of emitting opaque stubs.
pub fn load_project_ref(fsproj_or_dir: &Path, from_dir: &Path) -> Result<ProjectRef, String> {
    let loaded = super::pkg_meta::load(fsproj_or_dir, from_dir, "fsharp")?;
    let fsproj_name = loaded.meta.entry.unwrap_or_else(|| format!("{}.fsproj", loaded.meta.package));
    let include_path = relative_path(from_dir, &loaded.root_dir.join(fsproj_name));
    Ok(ProjectRef {
        include_path,
        namespace: loaded.meta.package,
        types: loaded.meta.fsharp_types.into_iter().collect(),
    })
}

pub struct Output {
    // F# namespace and project name (`Bindings`, `Pts`, `Babylonjs.Lite`, `Dom`).
    pub name: String,
    // File name and contents, in compilation order.
    pub files: Vec<(String, String)>,
    // Relative `.fsproj` paths of referenced binding projects whose types were used.
    pub project_refs: Vec<String>,
    // What the generator does not bind yet, by reason.
    pub skipped: BTreeMap<String, usize>,
    // Machine-readable package metadata (`tsrs-meta.toml`).
    pub meta_toml: String,
}

impl Output {
    /// The `.fsproj` filename for this binding project (`<name>.fsproj`).
    pub fn fsproj_name(&self) -> String {
        format!("{}.fsproj", self.name)
    }

    /// An F# project file compiling the generated binding files in order,
    /// ready to pull into another project via `<ProjectReference>`.
    pub fn fsproj(&self) -> String {
        let compile: String = self.files.iter().map(|(name, _)| format!("    <Compile Include=\"{name}\" />\n")).collect();
        let refs = if self.project_refs.is_empty() {
            String::new()
        } else {
            let items: String = self.project_refs.iter().map(|r| format!("    <ProjectReference Include=\"{r}\" />\n")).collect();
            format!("  <ItemGroup>\n{items}  </ItemGroup>\n")
        };
        format!(
            "<Project Sdk=\"Microsoft.NET.Sdk\">\n\
             \x20 <PropertyGroup>\n\
             \x20   <TargetFramework>netstandard2.0</TargetFramework>\n\
             \x20 </PropertyGroup>\n\
             {refs}\
             \x20 <ItemGroup>\n\
             {compile}\
             \x20 </ItemGroup>\n\
             \x20 <ItemGroup>\n\
             \x20   <PackageReference Include=\"Fable.Core\" Version=\"5.6.1\" />\n\
             \x20 </ItemGroup>\n\
             </Project>\n"
        )
    }
}

/// Converts a package or lib name into a valid F# namespace / project name:
/// `@babylonjs/lite` -> `Babylonjs.Lite`, `date-fns` -> `DateFns`, `dom` -> `Dom`.
pub fn package_namespace(raw: &str) -> String {
    let raw = raw.trim().trim_start_matches('@');
    let mut parts = Vec::new();
    for seg in raw.split(['/', '.']) {
        let mut p = pascal(seg);
        if p.is_empty() {
            continue;
        }
        if !p.as_bytes()[0].is_ascii_alphabetic() {
            p.insert(0, 'N');
        }
        parts.push(p);
    }
    if parts.is_empty() { "Bindings".to_string() } else { parts.join(".") }
}

pub(super) struct Gen {
    // Root F# namespace (`Bindings`, `Pts`, `Babylonjs.Lite`, ...).
    namespace: String,
    // The module the library is imported from, if it is not globals.
    import: Option<String>,
    // Classes a union tells apart by `instanceof`: imported types, which Fable tests itself.
    tested_classes: HashMap<String, String>,
    // The module of the whole library, when there is one.
    root_import: Option<String>,
    // For a method with optional parameters and then a rest parameter, which
    // of its two overloads is being written: true, the one with the rest.
    split_rest: Option<bool>,
    out: String,
    skipped: BTreeMap<String, usize>,
    // Qualified names declared in the XML.
    declared: HashSet<String>,
    type_names: HashSet<String>,
    // Types declared elsewhere, with the most type arguments seen.
    external: BTreeMap<String, usize>,
    // In the signature being written: type parameters over a map's keys, and their map.
    keyed: HashMap<String, String>,
    // A function value's own type parameters, which F# has not: `obj`.
    fn_params: Vec<String>,
    // Aliases a type extends, by id: an interface, as F# inherits only those.
    inherited: HashSet<String>,
    // Every declared interface/class/alias referenced as a base in any `<extends>`.
    extended_bases: HashSet<String>,
    // Depth-layered `[<ParamObject>]` `Create` factories: file -> depth -> owner -> lines.
    factories: BTreeMap<usize, BTreeMap<usize, BTreeMap<String, Vec<String>>>>,
    // The numbers of arguments constructor types take: a `JsConstructor<N>` each.
    ctor_arities: BTreeSet<usize>,
    // Abbreviations without type parameters, by name: what F# sees through
    // when it tells members apart (`type ArrayLike_or_x = obj`).
    abbrevs: HashMap<String, String>,
    // Maps whose keys are used: a `<Map>Keys` module is written for each.
    maps: BTreeSet<String>,
    // Top-level types by module. Top-level values go in a file of their own.
    types: BTreeMap<String, String>,
    values: String,
    // The file each declared name ends up in, and the file being written.
    file_of: HashMap<String, usize>,
    current_file: usize,
    // The part of its module each top-level declaration goes in.
    units: HashMap<String, String>,
    // Top-level aliases by the file they are placed in.
    alias_types: BTreeMap<usize, String>,
    // Members moved to a later file, as extensions: file -> type -> member lines.
    extensions: BTreeMap<usize, BTreeMap<String, Vec<String>>>,
    // Interfaces with `[Symbol.iterator]`, and what each interface extends.
    iterable: HashSet<String>,
    bases: HashMap<String, Vec<String>>,
    // Interfaces that keep their constraints: `<'TNode when 'TNode :> Node>`,
    // by name; and the same, by how a type extension names the interface.
    constrained: HashMap<String, Vec<Option<String>>>,
    // An abbreviation's type parameters that its F# type shows (not those
    // only `this` used): F# abbreviations must show all theirs.
    alias_kept: HashMap<String, Vec<usize>>,
    constrained_owner: HashMap<String, String>,
    // What F# cannot express at all.
    problems: Problems,
    // Synthetic delegates synthesized for function types with optional parameters.
    delegates: HashMap<(String, Vec<String>, String), String>,
    delegate_counter: usize,
    // Synthetic interfaces hoisted from inline option-bag objects.
    records: HashMap<(String, Vec<String>, String), String>,
    record_counter: usize,
    probing: bool,
    cur_constrained: HashSet<String>,
    // Type parameters a function writes as others (`ResultDate` as `DateType`).
    skip_tparams: HashSet<String>,
    pending_delegates: String,
    global_delegates: String,
    in_globals: bool,
    // A namespace's functions, for a type of its name beside its module.
    carrier: Option<Vec<FsMember>>,
    cur_indent: String,
    cur_prefix: String,
    // The JavaScript path of the namespaces being written: `util` for the
    // module `pb_util`.
    js_prefix: String,
    // Method names declared on any ancestor of the interface currently being written.
    inherited_methods: HashSet<String>,
    // True while emitting members inside `hoist_inline_interface` so forward-ref members
    // are kept inline and accounted for via the hoisted type's placement file.
    hoisting: bool,
    // External binding projects whose types may be referenced directly.
    refs: Vec<ProjectRef>,
    // Indices into `refs` that were actually referenced.
    used_refs: BTreeSet<usize>,
}

impl Gen {
    pub(super) fn skip(&mut self, why: &str) {
        *self.skipped.entry(why.to_string()).or_default() += 1;
    }

    fn index(&mut self, scope: Node, prefix: &str) {
        for d in elements(scope) {
            let Some(name) = d.attribute("name") else { continue };
            let key = if prefix.is_empty() { name.to_string() } else { format!("{prefix}.{name}") };
            match d.tag_name().name() {
                "namespace" => self.index(d, &key),
                "var" | "function" => {}
                _ => {
                    self.declared.insert(key);
                }
            }
        }
    }

    // Looks up an external type name in any referenced binding project.
    pub(super) fn resolve_ref_type(&self, raw_name: &str, arity: usize, need_interface: bool) -> Option<(usize, String)> {
        let bare = raw_name.strip_prefix("global.").unwrap_or(raw_name);
        if bare.contains('.') {
            return None;
        }
        let fsharp_name = type_ident(bare);
        for (i, r) in self.refs.iter().enumerate() {
            if let Some(&(ref_arity, is_interface, _)) = r.types.get(&fsharp_name)
                && ref_arity == arity
                && (!need_interface || is_interface)
            {
                let first = r.namespace.split('.').next().unwrap_or(&r.namespace);
                let prefix = if self.declared.contains(first) || self.type_names.contains(first) {
                    format!("global.{}", r.namespace)
                } else {
                    r.namespace.clone()
                };
                return Some((i, format!("{prefix}.{fsharp_name}")));
            }
        }
        None
    }

    // The F# name of a type declared elsewhere: its path flattened
    // (`NodeJS_Dict`), in module `External` where a type declared here has
    // the name (GeoJSON's `Point` beside leaflet's).
    pub(super) fn external_name(&self, name: &str) -> String {
        let flat = name.replace('.', "_");
        let ti = type_ident(&flat);
        let shadows_core = matches!(
            ti.as_str(),
            "Map" | "ReadonlyMap" | "Set" | "ReadonlySet" | "WeakMap" | "WeakSet" | "List" | "Seq" | "Option" | "Result" | "Choice" | "Async" | "Task"
        );
        if shadows_core || self.declared.contains(name) || self.declared.contains(&flat) {
            format!("External.{ti}")
        } else {
            ti
        }
    }

    // The attribute that reaches `path`, a global, or a value of the imported
    // module: `[<Emit("Shape")>]`, `[<Import("Shape", "lib")>]`; called with
    // `args`, `[<Emit("f($0...)")>]` or `[<Import("f", "lib"); Emit("$0($1...)")>]`.
    pub(super) fn at(&self, path: &str, args: Option<&str>) -> String {
        let emit = |text: String| format!("Emit(\"{}\")", text.replace('"', "\\\""));
        match &self.import {
            None => format!("[<{}>]", emit(match args {
                Some(args) => format!("{path}({args})"),
                None => path.to_string(),
            })),
            Some(module) => {
                let (first, rest) = path.split_once('.').map_or((path, String::new()), |(a, b)| (a, format!(".{b}")));
                let import = format!("Import({first:?}, {module:?})");
                match args {
                    // The import is $0: each argument moves up one.
                    Some(args) => format!("[<{import}; {}>]", emit(format!("$0{rest}({})", shift(args)))),
                    None if rest.is_empty() => format!("[<{import}>]"),
                    None => format!("[<{import}; {}>]", emit(format!("$0{rest}"))),
                }
            }
        }
    }

    // Whether a type extends this one.
    pub(super) fn is_extended(&self, d: Node) -> bool {
        let id = d.attribute("id").unwrap_or(attr(d, "name"));
        self.bases.values().any(|bs| bs.iter().any(|b| b == id))
    }

    // Whether `from` extends `to`, through its bases, by id.
    pub(super) fn reaches(&self, from: &str, to: &str) -> bool {
        reaches_in(&self.bases, from, to)
    }
}

// The bindings as files, in the order F# compiles them: a prelude; the types
// of each module, in dependency order, with modules that refer to each other
// merged into one file (F# allows no cycles between files); and last the
// values: globals, functions as `Globals`, and the `<Map>Keys` modules.
// The bindings, or what F# cannot express of the XML.
pub fn module(xml: &str) -> Result<Output, Vec<String>> {
    module_named(xml, "Bindings")
}

pub fn module_named(xml: &str, namespace: &str) -> Result<Output, Vec<String>> {
    module_with_refs(xml, namespace, &[])
}

pub fn module_with_refs(xml: &str, namespace: &str, refs: &[ProjectRef]) -> Result<Output, Vec<String>> {
    let ns = if namespace.is_empty() { "Bindings".to_string() } else { namespace.to_string() };
    // Parse once; only re-parse if multiple packages must be regrouped into separate modules.
    let initial_doc = Document::parse(xml).map_err(|e| vec![format!("not well-formed XML: {e}")])?;
    let regrouped = package_modules(initial_doc.root_element());
    let parsed;
    let doc = if let Some(ref regrouped_xml) = regrouped {
        parsed = Document::parse(regrouped_xml).map_err(|e| vec![format!("not well-formed XML: {e}")])?;
        &parsed
    } else {
        &initial_doc
    };
    let mut g = Gen {
        namespace: ns.clone(),
        problems: Problems::default(),
        out: String::new(),
        skipped: BTreeMap::new(),
        declared: HashSet::new(),
        type_names: HashSet::new(),
        external: BTreeMap::new(),
        keyed: HashMap::new(),
        fn_params: Vec::new(),
        abbrevs: HashMap::new(),
        inherited: HashSet::new(),
        extended_bases: HashSet::new(),
        factories: BTreeMap::new(),
        ctor_arities: BTreeSet::new(),
        maps: BTreeSet::new(),
        types: BTreeMap::new(),
        values: String::new(),
        file_of: HashMap::new(),
        alias_types: BTreeMap::new(),
        units: HashMap::new(),
        current_file: 0,
        extensions: BTreeMap::new(),
        iterable: HashSet::new(),
        bases: HashMap::new(),
        constrained: HashMap::new(),
        alias_kept: HashMap::new(),
        constrained_owner: HashMap::new(),
        import: doc.root_element().attribute("import").map(str::to_string),
        tested_classes: HashMap::new(),
        root_import: doc.root_element().attribute("import").map(str::to_string),
        split_rest: None,
        delegates: HashMap::new(),
        delegate_counter: 0,
        records: HashMap::new(),
        record_counter: 0,
        probing: false,
        cur_constrained: HashSet::new(),
        skip_tparams: HashSet::new(),
        pending_delegates: String::new(),
        global_delegates: String::new(),
        in_globals: false,
        carrier: None,
        cur_indent: String::new(),
        cur_prefix: String::new(),
        js_prefix: String::new(),
        inherited_methods: HashSet::new(),
        hoisting: false,
        refs: refs.to_vec(),
        used_refs: BTreeSet::new(),
    };
    for r in &g.refs {
        for (name, (arity, _, abbrev)) in &r.types {
            if *arity == 0
                && let Some(target) = abbrev
            {
                g.abbrevs.insert(format!("{}.{name}", r.namespace), target.clone());
            }
        }
    }
    // Every type the declarations name: a delegate named by its place
    // (`each`'s `cb`, `EachCb`) takes none of their names.
    g.type_names = doc.root_element().descendants().filter(|d| matches!(d.tag_name().name(), "interface" | "class" | "typeAlias" | "enum")).filter_map(|d| d.attribute("name")).map(type_ident).collect();
    // Classes unions tell apart by instanceof (named as the output names
    // them), with the module each is imported from.
    let tested: HashSet<&str> = doc.root_element().descendants().filter_map(|n| n.attribute("test")?.strip_prefix("instanceof:")).collect();
    for d in doc.root_element().descendants().filter(|d| d.is_element() && d.parent().is_some_and(|p| p == doc.root_element() || p.has_tag_name("namespace"))) {
        let Some(id) = d.attribute("id").or(d.attribute("name")) else { continue };
        // Imported: at the top level, or in a package's module (`Pb.Node`).
        let namespaces: Vec<Node> = d.ancestors().skip(1).filter(|a| a.has_tag_name("namespace")).collect();
        if !namespaces.iter().all(|n| matches!(n.attribute("moduleKind"), Some("package" | "merged"))) {
            continue;
        }
        let module = namespaces.iter().find(|n| n.attribute("moduleKind") == Some("package")).and_then(|n| n.attribute("import")).or(d.attribute("import")).or(g.root_import.as_deref());
        if let Some(module) = module
            && tested.contains(id)
        {
            g.tested_classes.insert(id.to_string(), module.to_string());
        }
    }
    // By id: a namespace's `PixiMixins.ViewContainer` is not the `ViewContainer`.
    for d in doc.root_element().descendants().filter(|d| d.has_tag_name("interface") || d.has_tag_name("class") || d.has_tag_name("typeAlias")) {
        let name = d.attribute("id").unwrap_or(attr(d, "name")).to_string();
        let iter_src = if d.has_tag_name("typeAlias") {
            elements(d).find(|c| !c.has_tag_name("typeParameter") && !c.has_tag_name("extends")).unwrap_or(d)
        } else {
            d
        };
        if elements(iter_src).any(|m| m.attribute("name") == Some("[Symbol.iterator]")) {
            g.iterable.insert(name.clone());
        }
        // A base declared elsewhere by its name (`Pt extends Float32Array`).
        let bases: Vec<String> = named(d, "extends").filter_map(|e| elements(e).next()).map(|r| r.attribute("to").unwrap_or(attr(r, "name")).to_string()).collect();
        g.bases.entry(name).or_default().extend(bases);
    }
    g.constrained = constrained_interfaces(doc.root_element(), &g.bases);
    let (plan, units) = file_plan(doc.root_element());
    for (i, group) in plan.iter().enumerate() {
        for m in group {
            g.file_of.insert(format!("module:{m}"), i);
        }
    }
    g.file_of.extend(declared_files(doc.root_element(), &plan, &units));
    g.units = units;
    g.index(doc.root_element(), "");
    g.inherited = doc.root_element().descendants().filter(|e| e.has_tag_name("extends")).filter_map(|e| elements(e).next()).filter(|r| r.attribute("kind") == Some("typeAlias")).filter_map(|r| r.attribute("to")).map(str::to_string).collect();
    g.extended_bases = doc.root_element().descendants().filter(|e| e.has_tag_name("extends")).filter_map(|e| elements(e).next()).filter_map(|r| r.attribute("to")).map(str::to_string).collect();
    g.union_abbrevs(doc.root_element());
    g.settle_alias_kept(doc.root_element());
    g.decls(doc.root_element(), "", "");
    g.key_modules(doc.root_element());

    let mut prelude = format!(
        "// Generated.\nnamespace {ns}\n\nopen Fable.Core\n\n\
         /// A key of the map 'Map whose value has type 'V: `el.addEventListener (HTMLElementEventMapKeys.click, fun e -> ...)`.\n\
         [<Erase>]\ntype TypedKey<'Map, 'V> = TypedKey of string\n\n\
         /// A JavaScript symbol value.\n\
         [<AllowNullLiteral>]\ntype JsSymbol = interface end\n\n\
         /// A JavaScript object used as a map: `{{ [key: string]: T }}` is `JsRecord<string, T>`.\n\
         [<AllowNullLiteral; Interface>]\ntype JsRecord<'Key, 'Value> =\n    [<EmitIndexer>] abstract Item: key: 'Key -> 'Value with get, set\n    [<Emit(\"{{}}\")>] static member Create () : JsRecord<'Key, 'Value> = jsNative\n    [<Emit(\"Object.fromEntries($0)\")>] static member Create (entries: seq<'Key * 'Value>) : JsRecord<'Key, 'Value> = jsNative\n    [<Emit(\"Object.fromEntries($0)\")>] static member op_Implicit (entries: ('Key * 'Value) list) : JsRecord<'Key, 'Value> = jsNative\n    [<Emit(\"Object.fromEntries($0)\")>] static member op_Implicit (entries: ('Key * 'Value)[]) : JsRecord<'Key, 'Value> = jsNative\n\n\
         /// A JavaScript Error, where an API takes one: an F# exception is not an Error in JavaScript.\n\
         [<AutoOpen>]\nmodule JsErrors =\n    [<Emit(\"new Error($0)\")>]\n    let jsError (message: string) : exn = jsNative\n\n\
         /// A JavaScript async iterator: `next ()` is a promise of the next value, or of None at the end.\n\
         [<AllowNullLiteral>]\ntype JsAsyncIterator<'T> =\n    \
         [<Emit(\"$0.next().then(r => r.done ? undefined : r.value)\")>] abstract next: unit -> JS.Promise<'T option>\n\n",
    );
    // A constructor (`new (a: A) => T`), by its number of arguments: called with `new`.
    for k in &g.ctor_arities {
        let args: Vec<String> = (1..=*k).map(|i| format!("'A{i}")).collect();
        let params = if *k == 0 { "unit".to_string() } else { (1..=*k).map(|i| format!("a{i}: 'A{i}")).collect::<Vec<_>>().join(" * ") };
        let vars = args.iter().cloned().chain(["'T".to_string()]).collect::<Vec<_>>().join(", ");
        prelude += &format!("[<AllowNullLiteral>]\ntype JsConstructor{k}<{vars}> =\n    [<EmitConstructor>] abstract Create: {params} -> 'T\n\n");
    }
    // Types from other files are opaque.
    let mut apart = String::new();
    for (name, arity) in &g.external {
        let params: Vec<String> = (1..=*arity).map(|i| format!("'T{i}")).collect();
        let params = if params.is_empty() { String::new() } else { format!("<{}>", params.join(", ")) };
        match name.strip_prefix("External.") {
            Some(own) => apart += &format!("    [<AllowNullLiteral>]\n    type {own}{params} = interface end\n\n"),
            None if name == "Function" && *arity == 0 => {
                prelude += "[<AllowNullLiteral; Interface>]\n\
                            type Function =\n\
                            \x20   [<Emit(\"$0($1...)\")>] abstract Invoke: [<System.ParamArray>] args: obj[] -> obj\n\
                            \x20   [<Emit(\"$0\")>] static member Create (f: unit -> unit) : Function = jsNative\n\
                            \x20   [<Emit(\"((...r) => ($0)(r))\")>] static member Create (f: ResizeArray<obj> -> obj) : Function = jsNative\n\n";
            }
            None => prelude += &format!("[<AllowNullLiteral>]\ntype {name}{params} = interface end\n\n"),
        }
    }
    if !apart.is_empty() {
        prelude += &format!("/// Types declared elsewhere that have the names of types declared here.\nmodule External =\n{apart}");
    }
    let mut files = vec![(format!("{ns}.Prelude.fs"), prelude)];
    for (i, group) in plan.iter().enumerate() {
        let name = if group.len() == 1 { group[0].clone() } else { format!("{}_and_{}_more", group[0], group.len() - 1) };
        // A module of a path (`fs/promises`) is one file name still.
        let name: String = name.chars().map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' }).collect();
        let mut body: String = group.iter().filter_map(|m| g.types.get(m)).map(String::as_str).collect();
        body += g.alias_types.get(&i).map_or("", String::as_str);
        // Members of earlier types that mention types of this file.
        if let Some(ext) = g.extensions.get(&i) {
            body += &format!("/// Members of earlier types that refer to types of this file.\n[<AutoOpen>]\nmodule Extensions{:02} =\n", i + 1);
            for (owner, lines) in ext {
                let owner = g.constrained_owner.get(owner).unwrap_or(owner);
                body += &format!("    type {owner} with\n{}\n", lines.concat());
            }
        }
        // Depth-ordered factory overloads so derived option-bag `Create` shadows base `Create`.
        if let Some(by_depth) = g.factories.get(&i) {
            for (depth, by_owner) in by_depth {
                body += &format!("[<AutoOpen>]\nmodule Factories{:02}_{depth} =\n", i + 1);
                for (owner, lines) in by_owner {
                    let owner = g.constrained_owner.get(owner).unwrap_or(owner);
                    body += &format!("    type {owner} with\n{}\n", lines.concat());
                }
            }
        }
        files.push((format!("{ns}.{:02}.{name}.fs", i + 1), format!("// Generated. Modules: {}.\nnamespace rec {ns}\n\nopen Fable.Core\n\n{body}", group.join(", "))));
    }
    let values: String = g.values.lines().map(|l| if l.is_empty() { "\n".to_string() } else { format!("    {l}\n") }).collect();
    let globals = std::mem::take(&mut g.out);
    let global_delegates = std::mem::take(&mut g.global_delegates);
    files.push((format!("{ns}.Values.fs"), format!("// Generated.\nnamespace {ns}\n\nopen Fable.Core\n\n{global_delegates}{globals}[<AutoOpen>]\nmodule Values =\n    let private _values = ()\n\n{values}")));
    let project_refs = g.used_refs.iter().map(|&i| g.refs[i].include_path.clone()).collect();
    let mut fsharp_types: BTreeMap<String, (usize, bool, Option<String>)> = BTreeMap::new();
    for (file, src) in &files {
        if file.ends_with(".Prelude.fs") || file == "Prelude.fs" {
            continue;
        }
        let lines: Vec<&str> = src.lines().collect();
        for (i, &line) in lines.iter().enumerate() {
            let Some(rest) = line.strip_prefix("type ") else { continue };
            let Some((lhs, rhs)) = rest.split_once('=') else { continue };
            let lhs = lhs.trim();
            let (raw_name, arity) = match lhs.find('<') {
                Some(lt) => {
                    let n = lhs[..lt].trim();
                    let inside = &lhs[lt + 1..];
                    let inside = inside.split(" when ").next().unwrap_or(inside).trim_end_matches('>').trim();
                    let count = if inside.is_empty() { 0 } else { inside.split(',').count() };
                    (n, count)
                }
                None => (lhs, 0),
            };
            if raw_name.is_empty() || raw_name == "Globals" {
                continue;
            }
            let rhs = rhs.trim();
            let (is_interface, abbrev) = if rhs == "interface end" {
                (true, None)
            } else if !rhs.is_empty() {
                (false, Some(rhs.to_string()))
            } else {
                let first_body = lines[i + 1..].iter().map(|l| l.trim()).find(|l| !l.is_empty());
                let is_iface = first_body.is_some_and(|b| !b.starts_with('|'));
                (is_iface, None)
            };
            fsharp_types.entry(raw_name.to_string()).or_insert((arity, is_interface, abbrev));
        }
    }
    let meta = super::pkg_meta::PkgMeta {
        target: "fsharp".into(),
        package: ns.clone(),
        entry: Some(format!("{ns}.fsproj")),
        fsharp_types,
        ..Default::default()
    };
    g.problems.result(Output { name: ns, files, project_refs, skipped: g.skipped, meta_toml: meta.to_toml() })
}
