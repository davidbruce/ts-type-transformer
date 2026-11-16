// XML -> F# bindings for Fable. Keeps inheritance, generics, overloads,
// optional parameters and unions; needs literals and type-level expressions gone.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use roxmltree::{Document, Node};

use super::{Problems, attr, elements, flag, named, only_child};

const T: &str = "F#";

pub struct Output {
    // File name and contents, in compilation order.
    pub files: Vec<(String, String)>,
    // What the generator does not bind yet, by reason.
    pub skipped: BTreeMap<String, usize>,
}

struct Gen {
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
    // Types declared elsewhere, with the most type arguments seen.
    external: BTreeMap<String, usize>,
    // In the signature being written: type parameters over a map's keys, and their map.
    keyed: HashMap<String, String>,
    // A function value's own type parameters, which F# has not: `obj`.
    fn_params: Vec<String>,
    // Aliases a type extends, by id: an interface, as F# inherits only those.
    inherited: HashSet<String>,
    // The numbers of arguments constructor types take: a `JsConstructor<N>` each.
    ctor_arities: BTreeSet<usize>,
    // Abbreviations without type parameters, by name: what F# sees through
    // when it tells members apart (`type ArrayLike_or_x = obj`).
    abbrevs: HashMap<String, String>,
    // What tells each member line's TypeScript overload apart, by the line:
    // its `marks`.
    marks: HashMap<String, Vec<String>>,
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
    pending_delegates: String,
    global_delegates: String,
    in_globals: bool,
    // A namespace's functions, for a type of its name beside its module.
    carrier: Option<Vec<String>>,
    cur_indent: String,
    cur_prefix: String,
    // The JavaScript path of the namespaces being written: `util` for the
    // module `pb_util`.
    js_prefix: String,
}

// The constraints each generic interface can keep, by position: those every
// reference to it satisfies, with a declared type that extends the
// constraint or a type parameter constrained to one. F# checks an
// interface's constraints wherever it is named.
fn constrained_interfaces(root: Node, bases: &HashMap<String, Vec<String>>) -> HashMap<String, Vec<Option<String>>> {
    let extends = |t: &str, c: &str| {
        let mut todo = vec![t.to_string()];
        let mut seen = HashSet::new();
        while let Some(x) = todo.pop() {
            if x == c {
                return true;
            }
            if seen.insert(x.clone()) {
                todo.extend(bases.get(&x).cloned().unwrap_or_default());
            }
        }
        false
    };
    let constraint_of = |p: Node| elements(p).find(|c| c.has_tag_name("constraint")).map(only_child).filter(|c| c.has_tag_name("ref") && matches!(attr(*c, "kind"), "interface" | "class") && elements(*c).next().is_none()).and_then(|c| c.attribute("to")).map(str::to_string);
    let mut out: HashMap<String, Vec<Option<String>>> = HashMap::new();
    for d in elements(root).filter(|d| d.has_tag_name("interface") || d.has_tag_name("class")) {
        let cs: Vec<Option<String>> = named(d, "typeParameter").map(constraint_of).collect();
        if cs.iter().any(Option::is_some) {
            out.insert(d.attribute("id").unwrap_or(attr(d, "name")).to_string(), cs);
        }
    }
    // A type parameter in scope at a reference: the constraint it keeps. An
    // interface's is in `out`, once dropped there gone; a function's is its own.
    let param_constraint = |out: &HashMap<String, Vec<Option<String>>>, r: Node, name: &str| -> Option<String> {
        for a in r.ancestors() {
            let ps: Vec<Node> = named(a, "typeParameter").collect();
            let Some(i) = ps.iter().position(|p| attr(*p, "name") == name) else { continue };
            // A function value's own are `obj`, which meets nothing.
            if a.has_tag_name("functionType") {
                return None;
            }
            if a.has_tag_name("interface") || a.has_tag_name("class") || a.has_tag_name("typeAlias") {
                return out.get(a.attribute("id").unwrap_or(attr(a, "name"))).and_then(|cs| cs.get(i).cloned().flatten());
            }
            return constraint_of(ps[i]);
        }
        None
    };
    // An abbreviation (`Cell<F, D, V> = Cell_Core<F, D, V>`) has the
    // constraints its type needs of its parameters: F# wants them written.
    let aliases: Vec<Node> = root.descendants().filter(|d| d.has_tag_name("typeAlias") && named(*d, "typeParameter").next().is_some()).collect();
    let mut dropped_ever: HashSet<(String, usize)> = HashSet::new();
    // A constraint dropped can break one a type parameter met through it:
    // until nothing more is dropped.
    loop {
        for d in &aliases {
            let body = only_child_type(*d);
            let Some(target) = body.attribute("to").filter(|_| body.has_tag_name("ref")).and_then(|t| out.get(t)).cloned() else { continue };
            let id = d.attribute("id").unwrap_or(attr(*d, "name")).to_string();
            let cs: Vec<Option<String>> = named(*d, "typeParameter")
                .enumerate()
                .map(|(j, p)| {
                    let at = elements(body).position(|a| a.has_tag_name("ref") && a.attribute("kind") == Some("typeParameter") && attr(a, "name") == attr(p, "name"));
                    at.and_then(|i| target.get(i).cloned().flatten()).filter(|_| !dropped_ever.contains(&(id.clone(), j)))
                })
                .collect();
            if cs.iter().any(Option::is_some) {
                out.insert(id, cs);
            } else {
                out.remove(&id);
            }
        }
        let mut broken: HashSet<(String, usize)> = HashSet::new();
        for r in root.descendants().filter(|r| r.has_tag_name("ref")) {
            let Some(to) = r.attribute("to") else { continue };
            let Some(cs) = out.get(to) else { continue };
            for (i, arg) in elements(r).enumerate() {
                let Some(Some(c)) = cs.get(i) else { continue };
                let is = |t: Node| t.has_tag_name("ref") && matches!(attr(t, "kind"), "interface" | "class") && t.attribute("to").is_some_and(|to| extends(to, c));
                let ok = match (arg.tag_name().name(), arg.attribute("kind")) {
                    ("ref", Some("interface" | "class")) => is(arg),
                    ("ref", Some("typeParameter")) => param_constraint(&out, r, attr(arg, "name")).is_some_and(|pc| extends(&pc, c)),
                    // A union of such types: the reference names the constraint instead.
                    ("ref", Some("typeAlias")) => union_members(root, arg).is_some_and(|ms| ms.into_iter().all(is)),
                    // `M[K]`: one of a map's values, each of which must be one. The
                    // signature's type parameter then has the constraint.
                    ("indexedAccess", _) => {
                        let map = elements(arg).next().filter(|m| m.has_tag_name("ref")).and_then(|m| find_decl(root, m.attribute("to")?));
                        map.is_some_and(|m| map_properties(root, m).into_iter().all(|(_, t)| is(t)))
                    }
                    _ => false,
                };
                if !ok {
                    broken.insert((to.to_string(), i));
                }
            }
        }
        let mut dropped = false;
        for (name, cs) in out.iter_mut() {
            for (i, c) in cs.iter_mut().enumerate() {
                if c.is_some() && broken.contains(&(name.clone(), i)) {
                    *c = None;
                    dropped = true;
                    dropped_ever.insert((name.clone(), i));
                }
            }
        }
        if !dropped {
            break;
        }
    }
    out.retain(|_, cs| cs.iter().any(Option::is_some));
    out
}

impl Gen {
    // The unions F# cannot tell apart, bound as `obj`: known before any
    // member that takes one is written, to tell such members apart.
    fn union_abbrevs(&mut self, root: Node) {
        for d in root.descendants().filter(|d| d.has_tag_name("typeAlias") && named(*d, "typeParameter").next().is_none()) {
            let body = only_child_type(d);
            if !body.has_tag_name("union") {
                continue;
            }
            let (out, skipped) = (std::mem::take(&mut self.out), self.skipped.clone());
            let told_apart = self.named_union("probe", body, "");
            let recursive = body.descendants().any(|r| r.has_tag_name("ref") && r.attribute("name") == Some(attr(d, "name")));
            let erased = if told_apart { None } else if recursive { Some("obj".to_string()) } else { Some(self.ty(body, "")) };
            (self.out, self.skipped) = (out, skipped);
            if let Some(t) = erased {
                self.abbrevs.insert(type_ident(attr(d, "name")), t);
            }
        }
    }

    // The F# name of a type declared elsewhere: its path flattened
    // (`NodeJS_Dict`), in module `External` where a type declared here has
    // the name (GeoJSON's `Point` beside leaflet's).
    fn external_name(&self, name: &str) -> String {
        let flat = name.replace('.', "_");
        if self.declared.contains(name) || self.declared.contains(&flat) { format!("External.{}", type_ident(&flat)) } else { type_ident(&flat) }
    }
}

// The members of a union an alias names, if `r` refers to one.
fn union_members<'a>(root: Node<'a, 'a>, r: Node<'a, 'a>) -> Option<Vec<Node<'a, 'a>>> {
    let d = find_decl(root, r.attribute("to")?)?;
    let body = elements(d).find(|c| !c.has_tag_name("typeParameter"))?;
    body.has_tag_name("union").then(|| elements(body).collect())
}

fn ident(name: &str) -> String {
    const KEYWORDS: &[&str] = &[
        "abstract", "and", "as", "assert", "base", "begin", "checked", "class", "component", "const", "constraint", "default", "delegate", "do", "done",
        "downcast", "downto", "elif", "else", "end", "event", "exception", "extern", "false", "finally", "fixed", "for", "fun", "function", "global",
        "if", "in", "include", "inherit", "inline", "interface", "internal", "lazy", "let", "match", "member", "mixin", "mod", "module", "mutable",
        "namespace", "new", "null", "of", "open", "or", "override", "params", "private", "process", "protected", "public", "pure", "rec", "return",
        "sealed", "select", "sig", "static", "struct", "tailcall", "then", "to", "trait", "true", "try", "type", "upcast", "use", "val", "virtual",
        "void", "when", "while", "with", "yield",
    ];
    let plain = !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_') && !name.as_bytes()[0].is_ascii_digit();
    if KEYWORDS.contains(&name) || !plain { format!("``{name}``") } else { name.to_string() }
}

// A type's or module's name: `$` is no part of one, even in backticks
// (`AnimationOption$1`, as a bundler names a second declaration). A member
// may have it: ``$add``.
fn type_ident(name: &str) -> String {
    ident(&name.replace('$', "__"))
}

// The bindings as files, in the order F# compiles them: a prelude; the types
// of each module, in dependency order, with modules that refer to each other
// merged into one file (F# allows no cycles between files); and last the
// values: globals, functions as `Globals`, and the `<Map>Keys` modules.
// The bindings, or what F# cannot express of the XML.
pub fn module(xml: &str) -> Result<Output, Vec<String>> {
    let doc = Document::parse(xml).map_err(|e| vec![format!("not well-formed XML: {e}")])?;
    let mut g = Gen {
        problems: Problems::default(),
        out: String::new(),
        skipped: BTreeMap::new(),
        declared: HashSet::new(),
        external: BTreeMap::new(),
        keyed: HashMap::new(),
        fn_params: Vec::new(),
        abbrevs: HashMap::new(),
        marks: HashMap::new(),
        inherited: HashSet::new(),
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
        pending_delegates: String::new(),
        global_delegates: String::new(),
        in_globals: false,
        carrier: None,
        cur_indent: String::new(),
        cur_prefix: String::new(),
        js_prefix: String::new(),
    };
    // Classes unions tell apart by instanceof (named as the output names
    // them), with the module each is imported from.
    let tested: HashSet<&str> = doc.root_element().descendants().filter_map(|n| n.attribute("test")?.strip_prefix("instanceof:").filter(|c| !c.contains('.'))).collect();
    for d in elements(doc.root_element()) {
        let name = d.attribute("name").unwrap_or_default();
        if let Some(module) = d.attribute("import").or(g.root_import.as_deref())
            && tested.contains(name)
        {
            g.tested_classes.insert(name.to_string(), module.to_string());
        }
    }
    // By id: a namespace's `PixiMixins.ViewContainer` is not the `ViewContainer`.
    for d in doc.root_element().descendants().filter(|d| d.has_tag_name("interface") || d.has_tag_name("class")) {
        let name = d.attribute("id").unwrap_or(attr(d, "name")).to_string();
        if elements(d).any(|m| m.attribute("name") == Some("[Symbol.iterator]")) {
            g.iterable.insert(name.clone());
        }
        // A base declared elsewhere by its name (`Pt extends Float32Array`).
        let bases = named(d, "extends").filter_map(|e| elements(e).next()).map(|r| r.attribute("to").unwrap_or(attr(r, "name")).to_string()).collect();
        g.bases.entry(name).or_insert_with(Vec::new).extend::<Vec<String>>(bases);
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
    g.union_abbrevs(doc.root_element());
    g.settle_alias_kept(doc.root_element());
    g.decls(doc.root_element(), "", "");
    g.key_modules(doc.root_element());

    let mut prelude = String::from(
        "// Generated.\nnamespace Bindings\n\nopen Fable.Core\n\n\
         /// A key of the map 'Map whose value has type 'V: `el.addEventListener (HTMLElementEventMapKeys.click, fun e -> ...)`.\n\
         [<Erase>]\ntype TypedKey<'Map, 'V> = TypedKey of string\n\n\
         /// A JavaScript async iterator: `next ()` is a promise of the next value, or of None at the end.\n\
         /// A JavaScript object used as a map: `{ [key: string]: T }` is `JsRecord<string, T>`.\n\
         /// A value whose type F# cannot write, of type arguments it still takes.\n\
         [<AllowNullLiteral>]\ntype JsRecord<'Key, 'Value> =\n    [<EmitIndexer>] abstract Item: key: 'Key -> 'Value with get, set\n\n\
         /// A JavaScript Error, where an API takes one: an F# exception is not an Error in JavaScript.\n\
         [<AutoOpen>]\nmodule JsErrors =\n    [<Emit(\"new Error($0)\")>]\n    let jsError (message: string) : exn = jsNative\n\n\
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
            None => prelude += &format!("[<AllowNullLiteral>]\ntype {name}{params} = interface end\n\n"),
        }
    }
    if !apart.is_empty() {
        prelude += &format!("/// Types declared elsewhere that have the names of types declared here.\nmodule External =\n{apart}");
    }
    let mut files = vec![("Bindings.Prelude.fs".to_string(), prelude)];
    for (i, group) in plan.iter().enumerate() {
        let name = if group.len() == 1 { group[0].clone() } else { format!("{}_and_{}_more", group[0], group.len() - 1) };
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
        files.push((format!("Bindings.{:02}.{name}.fs", i + 1), format!("// Generated. Modules: {}.\nnamespace rec Bindings\n\nopen Fable.Core\n\n{body}", group.join(", "))));
    }
    let values: String = g.values.lines().map(|l| if l.is_empty() { "\n".to_string() } else { format!("    {l}\n") }).collect();
    let globals = std::mem::take(&mut g.out);
    let global_delegates = std::mem::take(&mut g.global_delegates);
    files.push(("Bindings.Values.fs".into(), format!("// Generated.\nnamespace Bindings\n\nopen Fable.Core\n\n{global_delegates}{globals}[<AutoOpen>]\nmodule Values =\n    let private _values = ()\n\n{values}")));
    g.problems.result(Output { files, skipped: g.skipped })
}

// The files of types, in the order F# must see them: one per module, each
// after the modules it cannot do without. Those are the bases of its types,
// the bodies of its aliases and anything a namespace mentions: F# has no way
// to refer to them from an earlier file. Other references only decide the
// order where these allow a choice; a member that refers forward becomes an
// extension member (see `members`). Modules whose bases or aliases refer to
// each other share one file.
fn file_plan(root: Node) -> (Vec<Vec<String>>, HashMap<String, String>) {
    let home = homes(root);
    let units = units(root, &home);
    let unit = |d: Node| -> Option<String> { d.attribute("name").and_then(|n| units.get(n)).cloned() };
    let mut modules: BTreeSet<String> = BTreeSet::new();
    let mut hard: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut soft: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for d in elements(root) {
        if d.attribute("module").is_none() || d.has_tag_name("var") || d.has_tag_name("function") {
            continue;
        }
        let Some(m) = unit(d) else { continue };
        let m = m.as_str();
        modules.insert(m.to_string());
        let target = |r: Node| r.attribute("to").and_then(|n| units.get(n).or_else(|| home.get(n))).filter(|t| t.as_str() != m).cloned();
        // An alias has no fixed place: it goes where its parts are (`alias_files`).
        let hard_scope: Vec<Node> = match d.tag_name().name() {
            "namespace" => vec![d],
            "typeAlias" => vec![],
            _ => named(d, "extends").collect(),
        };
        for scope in &hard_scope {
            hard.entry(m.to_string()).or_default().extend(scope.descendants().filter(|r| r.has_tag_name("ref")).filter_map(target));
        }
        soft.entry(m.to_string()).or_default().extend(d.descendants().filter(|r| r.has_tag_name("ref")).filter_map(target));
    }
    let names: Vec<String> = modules.into_iter().collect();
    let index: HashMap<&str, usize> = names.iter().enumerate().map(|(i, n)| (n.as_str(), i)).collect();
    let edges = |deps: &BTreeMap<String, BTreeSet<String>>| -> Vec<BTreeSet<usize>> {
        names.iter().map(|n| deps.get(n).into_iter().flatten().filter_map(|t| index.get(t.as_str()).copied()).collect()).collect()
    };
    let (hard, all) = (edges(&hard), edges(&soft));
    // Preferred order: dependencies first, counting every reference.
    let mut rank = vec![0; names.len()];
    for (r, group) in tarjan(names.len(), &all).into_iter().enumerate() {
        for i in group {
            rank[i] = r;
        }
    }
    // Groups that must share a file, then placed as soon as their bases are, in preferred order.
    let groups = tarjan(names.len(), &hard);
    let group_of: HashMap<usize, usize> = groups.iter().enumerate().flat_map(|(g, ms)| ms.iter().map(move |&m| (m, g))).collect();
    let mut placed = vec![false; groups.len()];
    let mut out = Vec::new();
    while out.len() < groups.len() {
        let ready = (0..groups.len())
            .filter(|&g| !placed[g] && groups[g].iter().all(|&m| hard[m].iter().all(|t| group_of[t] == g || placed[group_of[t]])))
            .min_by_key(|&g| groups[g].iter().map(|&m| rank[m]).min())
            .expect("an order exists");
        placed[ready] = true;
        let mut ms: Vec<String> = groups[ready].iter().map(|&m| names[m].clone()).collect();
        ms.sort();
        out.push(ms);
    }
    (out, units)
}

// The part of its module each declaration goes in. What other modules inherit
// from (transitively) is `<module>.base`, placed before the rest: `Window`
// inherits from other modules' types, which inherit from `EventTarget`, so
// both cannot share a file with the others in between.
fn units(root: Node, home: &HashMap<String, String>) -> HashMap<String, String> {
    let mut bases: HashMap<&str, Vec<String>> = HashMap::new();
    for d in elements(root).filter(|d| d.has_tag_name("interface") || d.has_tag_name("class")) {
        let refs = named(d, "extends").flat_map(|e| e.descendants()).filter(|r| r.has_tag_name("ref")).filter_map(|r| r.attribute("to")).map(str::to_string);
        bases.insert(attr(d, "name"), refs.collect());
    }
    let mut base: HashSet<String> = HashSet::new();
    for (d, refs) in &bases {
        let module = home.get(*d);
        // Everything a declaration of another module inherits, directly or not.
        let mut todo: Vec<&String> = refs.iter().filter(|r| home.get(*r) != module).collect();
        while let Some(r) = todo.pop() {
            if base.insert(r.clone()) {
                todo.extend(bases.get(r.as_str()).into_iter().flatten());
            }
        }
    }
    home.iter().filter(|(n, _)| !n.contains('.')).map(|(n, m)| (n.clone(), if base.contains(n) { format!("{m}.base") } else { m.clone() })).collect()
}

// Every declared name, qualified, with its module.
fn homes(root: Node) -> HashMap<String, String> {
    fn walk(scope: Node, prefix: &str, module: &str, home: &mut HashMap<String, String>) {
        for d in elements(scope) {
            let Some(n) = d.attribute("name") else { continue };
            let module = d.attribute("module").unwrap_or(module);
            let key = if prefix.is_empty() { n.to_string() } else { format!("{prefix}.{n}") };
            if d.has_tag_name("namespace") {
                walk(d, &key, module, home);
            }
            home.insert(key, module.to_string());
        }
    }
    let mut home = HashMap::new();
    walk(root, "", "main", &mut home);
    home
}

// The file of each top-level type name as it appears in F# source, its
// static side (`XStatic`) included.
fn declared_files(root: Node, plan: &[Vec<String>], units: &HashMap<String, String>) -> HashMap<String, usize> {
    let file: HashMap<&str, usize> = plan.iter().enumerate().flat_map(|(i, g)| g.iter().map(move |m| (m.as_str(), i))).collect();
    let mut out = HashMap::new();
    for d in elements(root) {
        let (Some(n), Some(_)) = (d.attribute("name"), d.attribute("module")) else { continue };
        if d.has_tag_name("var") || d.has_tag_name("function") {
            continue;
        }
        if let Some(&f) = units.get(n).and_then(|u| file.get(u.as_str())) {
            out.insert(n.to_string(), f);
            out.insert(format!("{n}Static"), f);
        }
    }
    // An alias goes in the first file where everything it refers to exists.
    let aliases: Vec<(String, Vec<String>)> = elements(root)
        .filter(|d| d.has_tag_name("typeAlias"))
        .map(|d| (attr(d, "name").to_string(), d.descendants().filter(|r| r.has_tag_name("ref")).filter_map(|r| r.attribute("to")).map(str::to_string).collect()))
        .collect();
    for _ in 0..aliases.len().min(32) {
        let mut changed = false;
        for (name, refs) in &aliases {
            let own = out.get(name).copied().unwrap_or(0);
            // `Tokens.Generic`: where `Tokens` is.
            let needed = refs.iter().filter_map(|r| out.get(r).or_else(|| out.get(r.split('.').next().unwrap()))).copied().max().unwrap_or(0).max(own);
            if needed != own {
                out.insert(name.clone(), needed);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    out
}

// The identifiers in F# source.
fn words(src: &str) -> HashSet<&str> {
    src.split(|c: char| !(c.is_alphanumeric() || c == '_')).filter(|w| !w.is_empty()).collect()
}

// Strongly connected components, each after those it has edges to.
fn tarjan(n: usize, edges: &[BTreeSet<usize>]) -> Vec<Vec<usize>> {
    struct State<'a> {
        edges: &'a [BTreeSet<usize>],
        index: Vec<Option<usize>>,
        low: Vec<usize>,
        on: Vec<bool>,
        stack: Vec<usize>,
        next: usize,
        out: Vec<Vec<usize>>,
    }
    fn visit(v: usize, s: &mut State) {
        s.index[v] = Some(s.next);
        s.low[v] = s.next;
        s.next += 1;
        s.stack.push(v);
        s.on[v] = true;
        for &w in s.edges[v].iter() {
            match s.index[w] {
                None => {
                    visit(w, s);
                    s.low[v] = s.low[v].min(s.low[w]);
                }
                Some(iw) if s.on[w] => s.low[v] = s.low[v].min(iw),
                _ => {}
            }
        }
        if Some(s.low[v]) == s.index[v] {
            let mut group = Vec::new();
            loop {
                let w = s.stack.pop().unwrap();
                s.on[w] = false;
                group.push(w);
                if w == v {
                    break;
                }
            }
            s.out.push(group);
        }
    }
    let mut s = State { edges, index: vec![None; n], low: vec![0; n], on: vec![false; n], stack: Vec::new(), next: 0, out: Vec::new() };
    for v in 0..n {
        if s.index[v].is_none() {
            visit(v, &mut s);
        }
    }
    s.out
}

impl Gen {
    // The attribute that reaches `path`, a global, or a value of the imported
    // module: `[<Emit("Shape")>]`, `[<Import("Shape", "lib")>]`; called with
    // `args`, `[<Emit("f($0...)")>]` or `[<Import("f", "lib"); Emit("$0($1...)")>]`.
    fn at(&self, path: &str, args: Option<&str>) -> String {
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

    // The members of generic bases left to another path (which has them at
    // other arguments, `GPUDataOwner<obj>`), and of their bases, at the
    // arguments given here: F# lets an interface declare again a member it
    // inherits, at a type of its own. Those whose type is the same either way,
    // or that it declares already, it inherits.
    fn redeclared(&mut self, d: Node, left: &[Node], body: &[String], owner: &str, extend: &str, indent: &str) -> Vec<String> {
        let root = d.document().root_element();
        let mut own: HashSet<String> = body.iter().filter_map(|l| member_name(l)).map(|(_, n)| n.to_string()).collect();
        let mut out = Vec::new();
        let mut todo: Vec<(Node, HashMap<String, String>)> = Vec::new();
        for r in left {
            let args: HashMap<String, String> = HashMap::new();
            todo.push((*r, args));
        }
        let mut seen = HashSet::new();
        while let Some((r, outer)) = todo.pop() {
            let Some(base) = r.attribute("to").and_then(|to| find_decl(root, to)) else { continue };
            if !seen.insert(r.attribute("to").unwrap_or_default().to_string()) {
                continue;
            }
            // Its type parameters, as the arguments given here.
            let mut map = HashMap::new();
            for (p, a) in named(base, "typeParameter").zip(elements(r)) {
                let t = self.ty(a, owner);
                map.insert(attr(p, "name").to_string(), subst_params(&t, &outer));
            }
            for e in named(base, "extends") {
                todo.push((only_child(e), map.clone()));
            }
            let src = if base.has_tag_name("typeAlias") { only_child_type(base) } else { base };
            let members: Vec<Node> = elements(src).filter(|m| matches!(m.tag_name().name(), "property" | "method" | "get" | "set") && !flag(*m, "static")).collect();
            // Rendered apart: what goes to later files' extensions, and what is skipped, is the base's.
            let saved = (self.extensions.clone(), self.skipped.clone());
            let lines = self.members(&members, owner, extend, indent);
            (self.extensions, self.skipped) = saved;
            for line in lines {
                let again = subst_params(&line, &map);
                let Some((_, name)) = member_name(&again) else { continue };
                if again != line && own.insert(name.to_string()) {
                    out.push(again);
                }
            }
        }
        out
    }

    // Whether a type extends this one.
    fn is_extended(&self, d: Node) -> bool {
        let id = d.attribute("id").unwrap_or(attr(d, "name"));
        self.bases.values().any(|bs| bs.iter().any(|b| b == id))
    }

    // Whether `from` extends `to`, through its bases, by id.
    fn reaches(&self, from: &str, to: &str) -> bool {
        let mut todo = vec![from.to_string()];
        let mut seen = HashSet::new();
        while let Some(c) = todo.pop() {
            if c == to {
                return true;
            }
            if seen.insert(c.clone()) {
                todo.extend(self.bases.get(&c).cloned().unwrap_or_default());
            }
        }
        false
    }

    // Whether a class extends another, through its bases.
    fn extends_class(&self, class: &str, base: &str) -> bool {
        let mut todo = vec![class.to_string()];
        let mut seen = HashSet::new();
        while let Some(c) = todo.pop() {
            for b in self.bases.get(&c).into_iter().flatten() {
                if b == base {
                    return true;
                }
                if seen.insert(b.clone()) {
                    todo.push(b.clone());
                }
            }
        }
        false
    }

    // What an alias other than a non-generic union is in F#: an interface
    // where an abbreviation cannot be one, a delegate where a function has
    // optional parameters, else an abbreviation of this type.
    fn alias_form(&mut self, d: Node) -> AliasForm {
        let raw = attr(d, "name");
        let body = only_child_type(d);
        // F# abbreviations cannot refer to themselves; an interface can.
        let recursive = body.descendants().any(|r| r.has_tag_name("ref") && r.attribute("name") == Some(raw));
        // So can one a type extends: F# inherits only interfaces.
        let inherited = d.attribute("id").is_some_and(|id| self.inherited.contains(id));
        // And one with more than properties, or a lone index signature, has members.
        let members = body.has_tag_name("object")
            && elements(body).next().is_some()
            && !elements(body).all(|m| m.has_tag_name("property"))
            && !(elements(body).count() == 1 && elements(body).all(|m| m.has_tag_name("index")));
        if body.has_tag_name("object") && (recursive || inherited || members) {
            return AliasForm::Interface;
        }
        if recursive && !(body.has_tag_name("functionType") && d.parent().is_some_and(|p| !p.has_tag_name("namespace"))) {
            self.skip("recursive type alias, bound as obj");
            return AliasForm::Abbreviation("obj".into());
        }
        if body.has_tag_name("functionType") && named(body, "parameter").any(|p| flag(p, "optional")) {
            return AliasForm::Delegate;
        }
        AliasForm::Abbreviation(self.ty(body, ""))
    }

    // Which type parameters each generic abbreviation shows: F# wants all it
    // declares in its type. Those its type does not show (in `this`, in a
    // union bound as `obj`, in an argument another drops) are dropped from
    // it and from references to it, until settled.
    fn settle_alias_kept(&mut self, root: Node) {
        let aliases: Vec<Node> = root.descendants().filter(|d| d.has_tag_name("typeAlias") && named(*d, "typeParameter").next().is_some()).collect();
        loop {
            let mut changed = false;
            for d in &aliases {
                let saved = (std::mem::take(&mut self.skipped), std::mem::take(&mut self.problems));
                let form = self.alias_form(*d);
                (self.skipped, self.problems) = saved;
                let AliasForm::Abbreviation(t) = form else { continue };
                let words = words(&t);
                let params: Vec<&str> = named(*d, "typeParameter").map(|p| attr(p, "name")).collect();
                let k: Vec<usize> = (0..params.len()).filter(|&i| words.contains(params[i])).collect();
                let id = d.attribute("id").unwrap_or(attr(*d, "name")).to_string();
                if k.len() < params.len() && self.alias_kept.get(&id) != Some(&k) {
                    self.alias_kept.insert(id, k);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
    }

    fn skip(&mut self, why: &str) {
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

    fn ty(&mut self, n: Node, owner: &str) -> String {
        match n.tag_name().name() {
            "primitive" => match attr(n, "name") {
                "string" => "string",
                "number" => "float",
                "boolean" => "bool",
                "bigint" => "bigint",
                "void" | "undefined" => "unit",
                _ => "obj",
            }
            .into(),
            "this" => owner.to_string(),
            "ref" if attr(n, "kind") == "typeParameter" && self.fn_params.iter().any(|p| p == attr(n, "name")) => "obj".into(),
            "ref" if attr(n, "kind") == "typeParameter" => format!("'{}", attr(n, "name")),
            "ref" => {
                // Declared here (`to`), or elsewhere, though one declared here may have its name.
                let to = n.attribute("to");
                let name = to.unwrap_or_else(|| n.attribute("qualified").unwrap_or(attr(n, "name")));
                if let Some(kept) = to.and_then(|t| self.alias_kept.get(t)).cloned() {
                    let args: Vec<Node> = elements(n).collect();
                    let args: Vec<String> = kept.iter().filter_map(|&i| args.get(i)).map(|a| self.ty(*a, owner)).collect();
                    let name = name.split('.').map(type_ident).collect::<Vec<_>>().join(".");
                    return if args.is_empty() { name } else { format!("{name}<{}>", args.join(", ")) };
                }
                // Where the interface keeps a constraint, a union of subtypes is named as the constraint: F# has no union that is one.
                let kept = to.and_then(|t| self.constrained.get(t)).cloned().unwrap_or_default();
                let args: Vec<String> = elements(n)
                    .enumerate()
                    .map(|(i, a)| match kept.get(i) {
                        Some(Some(c)) if a.has_tag_name("ref") && a.attribute("kind") == Some("typeAlias") => type_path(c),
                        _ => self.ty(a, owner),
                    })
                    .collect();
                let name = if to.is_some() {
                    name.split('.').map(type_ident).collect::<Vec<_>>().join(".")
                } else {
                    match (name, args.len()) {
                        ("Array" | "ReadonlyArray", 1) => "ResizeArray".into(),
                        ("Promise" | "PromiseLike", 1) => "JS.Promise".into(),
                        // Built-ins Fable has: its types, which have their members.
                        // Their buffer argument (`Uint8Array<ArrayBufferLike>`) it does not take.
                        // Not as a base: an interface cannot inherit them (`Pt extends Float32Array`).
                        (_, n_args) if !is_base(n) && (n_args == 0 || name != "Error") && let Some(t) = fable_builtin(name) => return t,
                        // A type from another library, an alias too: an opaque type by its name.
                        _ if matches!(attr(n, "kind"), "class" | "interface" | "typeAlias") => {
                            let ext = self.external_name(name);
                            let arity = self.external.entry(ext.clone()).or_default();
                            *arity = (*arity).max(args.len());
                            ext
                        }
                        // A value from another file: nothing to refer to.
                        _ => {
                            self.skip("reference to an undeclared value, bound as obj");
                            return "obj".into();
                        }
                    }
                };
                if args.is_empty() { name } else { format!("{name}<{}>", args.join(", ")) }
            }
            "nullable" => {
                let inner = only_child(n);
                let t = self.ty(inner, owner);
                // Fable sends `None` as `undefined`. Where only `null` is accepted, use
                // a type that has a null of its own, if there is one.
                if attr(n, "absent") != "null" || inner.has_tag_name("functionType") || t.starts_with('\'') || t == "unit" {
                    return format!("{t} option");
                }
                match t.as_str() {
                    "float" | "bool" => format!("System.Nullable<{t}>"),
                    _ => t,
                }
            }
            // Fable's erased unions.
            "union" => {
                let mut parts: Vec<String> = Vec::new();
                for p in elements(n) {
                    let t = self.ty(p, owner);
                    if !parts.contains(&t) {
                        parts.push(t);
                    }
                }
                erased_union(parts)
            }
            "array" => format!("ResizeArray<{}>", self.ty(only_child(n), owner)),
            // `M[K]` for a key parameter: the value type the key selects.
            "indexedAccess" => {
                let index = elements(n).nth(1).unwrap();
                match index.attribute("name").filter(|k| index.attribute("kind") == Some("typeParameter") && self.keyed.contains_key(*k)) {
                    Some(k) => format!("'{k}"),
                    None => {
                        self.problems.element(T, n);
                        "obj".into()
                    }
                }
            }
            // Fable compiles tuples to arrays.
            "tuple" => {
                let parts: Vec<String> = named(n, "element").map(|e| self.ty(only_child(e), owner)).collect();
                if parts.len() < 2 || named(n, "element").any(|e| flag(e, "rest") || flag(e, "optional")) {
                    // An array of the one type its elements have (`[A]`, `[A, ...A[]]`).
                    let elems: Vec<String> = named(n, "element")
                        .map(|e| match only_child(e) {
                            t if flag(e, "rest") && t.has_tag_name("array") => self.ty(only_child(t), owner),
                            t => self.ty(t, owner),
                        })
                        .collect();
                    if let Some(first) = elems.first().filter(|f| elems.iter().all(|e| e == *f)) {
                        return format!("ResizeArray<{first}>");
                    }
                    self.skip("irregular tuple, bound as an array");
                    return "ResizeArray<obj>".into();
                }
                format!("({})", parts.join(" * "))
            }
            // A call signature is one: an interface of only that is a function.
            "functionType" | "call" => {
                let own: Vec<String> = named(n, "typeParameter").map(|p| attr(p, "name").to_string()).collect();
                let depth = self.fn_params.len();
                self.fn_params.extend(own);
                let t = if named(n, "parameter").any(|p| flag(p, "optional")) {
                    self.function_delegate(n, owner)
                } else {
                    let mut parts: Vec<String> = named(n, "parameter").map(|p| self.ty(only_child(p), owner)).collect();
                    if parts.is_empty() {
                        parts.push("unit".into());
                    }
                    let ret = named(n, "returns").next().map(only_child);
                    // A listener (one argument, any result): its result is
                    // nothing JavaScript reads, as the model has it.
                    let listener = named(n, "parameter").count() == 1 && super::model::listener_result(n);
                    let ret = match ret {
                        Some(_) if listener => "unit".to_string(),
                        Some(r) => self.ty(r, owner),
                        None => "unit".into(),
                    };
                    parts.push(ret);
                    format!("({})", parts.join(" -> "))
                };
                self.fn_params.truncate(depth);
                t
            }
            // An object with only properties is an anonymous record.
            "object" if elements(n).all(|m| m.has_tag_name("property")) && elements(n).next().is_some() => {
                let fields: Vec<String> = elements(n)
                    .map(|m| {
                        let t = self.ty(only_child(m), owner);
                        format!("{}: {t}{}", ident(attr(m, "name")), if flag(m, "optional") { " option" } else { "" })
                    })
                    .collect();
                format!("{{| {} |}}", fields.join("; "))
            }
            // `{}`: any object.
            "object" if elements(n).next().is_none() => "obj".into(),
            // Only an index signature: a map.
            "object" if elements(n).count() == 1 && elements(n).all(|m| m.has_tag_name("index")) => {
                let m = elements(n).next().unwrap();
                let key = named(m, "parameter").next().map(|p| self.ty(only_child(p), owner)).unwrap_or("string".into());
                format!("JsRecord<{key}, {}>", self.returns(m, owner))
            }
            // A constructor: `new` on it makes a T. Its own type parameters are `obj`.
            "constructorType" if !named(n, "parameter").any(|p| flag(p, "rest") || flag(p, "optional")) => {
                let depth = self.fn_params.len();
                self.fn_params.extend(named(n, "typeParameter").map(|p| attr(p, "name").to_string()));
                let mut parts: Vec<String> = named(n, "parameter").map(|p| self.ty(only_child(p), owner)).collect();
                let k = parts.len();
                parts.push(named(n, "returns").next().map(|r| self.ty(only_child(r), owner)).unwrap_or("obj".into()));
                self.fn_params.truncate(depth);
                self.ctor_arities.insert(k);
                format!("JsConstructor{k}<{}>", parts.join(", "))
            }
            "object" | "constructorType" => {
                self.skip("inline type with methods, bound as obj");
                "obj".into()
            }
            _ => {
                self.problems.element(T, n);
                "obj".into()
            }
        }
    }

    fn delegate_sig(&mut self, n: Node, owner: &str) -> String {
        let saved_keyed = self.keyed.clone();
        for p in named(n, "typeParameter") {
            let map = elements(p).find(|c| c.has_tag_name("constraint")).map(only_child).filter(|k| k.has_tag_name("keyof")).map(only_child).filter(|r| r.has_tag_name("ref"));
            if let Some(map) = map {
                let name = map.attribute("to").unwrap_or_default().to_string();
                if self.maps.contains(&name) {
                    self.keyed.insert(attr(p, "name").to_string(), ident(&name));
                }
            }
        }
        let params: Vec<Node> = named(n, "parameter").filter(|p| !p.has_attribute("const")).collect();
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        for (i, &p) in params.iter().enumerate() {
            let t_node = only_child(p);
            let raw = attr(p, "name");
            // `_` names no parameter in F#.
            let mut pname = if raw.is_empty() || raw == "_" { format!("arg{i}") } else { raw.to_string() };
            while !seen.insert(pname.clone()) {
                pname.push('_');
            }
            let pname = ident(&pname);
            if flag(p, "rest") {
                let elem = if t_node.has_tag_name("array") { self.ty(only_child(t_node), owner) } else { "obj".into() };
                out.push(format!("[<System.ParamArray>] {pname}: {elem}[]"));
                continue;
            }
            let t = if t_node.has_tag_name("ref") && attr(t_node, "kind") == "typeParameter" && self.keyed.contains_key(attr(t_node, "name")) {
                format!("TypedKey<{}, '{}>", self.keyed[attr(t_node, "name")], attr(t_node, "name"))
            } else {
                self.ty(t_node, owner)
            };
            if flag(p, "optional") {
                let trailing_opt = params[i + 1..].iter().all(|q| flag(*q, "optional") && !flag(*q, "rest"));
                if trailing_opt {
                    let base = t.strip_suffix(" option").unwrap_or(&t);
                    out.push(format!("?{pname}: {base}"));
                } else {
                    let opt = if t.ends_with(" option") { t } else { format!("{t} option") };
                    out.push(format!("{pname}: {opt}"));
                }
            } else {
                out.push(format!("{pname}: {t}"));
            }
        }
        let params_str = if out.is_empty() { "unit".to_string() } else { out.join(" * ") };
        let ret = named(n, "returns").next().map(|r| self.ty(only_child(r), owner)).unwrap_or_else(|| "unit".into());
        self.keyed = saved_keyed;
        format!("{params_str} -> {ret}")
    }

    fn function_delegate(&mut self, n: Node, owner: &str) -> String {
        let sig_str = self.delegate_sig(n, owner);
        // The type parameters the signature shows, in order: erased ones it does not.
        let mut tparams: Vec<String> = Vec::new();
        let b = sig_str.as_bytes();
        for (i, _) in sig_str.match_indices('\'') {
            let end = b[i + 1..].iter().position(|c| !(c.is_ascii_alphanumeric() || *c == b'_')).map_or(b.len(), |p| i + 1 + p);
            let tp = sig_str[i..end].to_string();
            if end > i + 1 && !tparams.contains(&tp) {
                tparams.push(tp);
            }
        }
        let scope = if !self.cur_indent.is_empty() {
            format!("ns:{}", self.cur_prefix)
        } else if self.in_globals {
            "globals".into()
        } else {
            "files".into()
        };
        let key = (scope, tparams.clone(), sig_str.clone());
        let dname = if let Some(existing) = self.delegates.get(&key) {
            existing.clone()
        } else {
            let mut dname = loop {
                self.delegate_counter += 1;
                let candidate = format!("JsFn{}", self.delegate_counter);
                if !self.file_of.contains_key(&candidate) && !self.declared.contains(&candidate) {
                    break candidate;
                }
            };
            let tp_suffix = if tparams.is_empty() { String::new() } else { format!("<{}>", tparams.join(", ")) };
            let def = format!("{}type {dname}{tp_suffix} = delegate of {sig_str}\n\n", self.cur_indent);
            if !self.cur_indent.is_empty() {
                self.pending_delegates += &def;
            } else if self.in_globals {
                self.global_delegates += &def;
            } else {
                let latest = words(&sig_str).iter().filter_map(|w| self.file_of.get(*w)).copied().max().unwrap_or(0);
                let target_file = latest.max(self.current_file);
                self.file_of.insert(dname.clone(), target_file);
                *self.alias_types.entry(target_file).or_default() += &def;
            }
            self.delegates.insert(key, dname.clone());
            if !tparams.is_empty() {
                dname = format!("{dname}{tp_suffix}");
            }
            dname
        };
        if !tparams.is_empty() && !dname.contains('<') {
            format!("{dname}<{}>", tparams.join(", "))
        } else {
            dname
        }
    }

    // Notes the type parameters of a signature that range over a map's keys.
    fn enter_keyed(&mut self, n: Node) {
        self.keyed.clear();
        for p in named(n, "typeParameter") {
            let map = elements(p).find(|c| c.has_tag_name("constraint")).map(only_child).filter(|k| k.has_tag_name("keyof")).map(only_child).filter(|r| r.has_tag_name("ref"));
            if let Some(map) = map {
                let Some(name) = map.attribute("to").map(str::to_string) else { continue };
                self.maps.insert(name.clone());
                self.keyed.insert(attr(p, "name").to_string(), name.split('.').map(type_ident).collect::<Vec<_>>().join("."));
            }
        }
    }

    // For each map whose keys are used: a module with one typed key per property.
    fn key_modules(&mut self, root: Node) {
        for map in std::mem::take(&mut self.maps) {
            let Some(decl) = find_decl(root, &map) else {
                self.skip("map declared elsewhere, no keys");
                continue;
            };
            let map_type = map.split('.').map(type_ident).collect::<Vec<_>>().join(".");
            let module = ident(&format!("{}Keys", map.rsplit('.').next().unwrap()));
            self.out += &format!("/// The keys of {map}.\nmodule {module} =\n");
            let mut seen = HashSet::new();
            for (key, ty) in map_properties(root, decl) {
                if seen.insert(key.clone()) {
                    let t = self.ty(ty, "");
                    self.out += &format!("    let {}: TypedKey<{map_type}, {t}> = TypedKey {key:?}\n", ident(&key));
                }
            }
            self.out += "\n";
        }
    }

    // A method's or function's type parameters, with the constraints F# can
    // state: `<'E when 'E :> Element>`, checked where it is called. An
    // interface's are left unconstrained: F# would check them wherever the
    // interface is named, erased unions and `obj` arguments included.
    fn constrained_type_params(&mut self, n: Node) -> String {
        let mut params = Vec::new();
        let mut constraints = Vec::new();
        for p in named(n, "typeParameter") {
            let name = format!("'{}", attr(p, "name"));
            let c = elements(p).find(|c| c.has_tag_name("constraint")).map(only_child);
            if let Some(c) = c.filter(|c| c.has_tag_name("ref") && matches!(attr(*c, "kind"), "interface" | "class")) {
                let t = self.ty(c, "");
                if t != "obj" {
                    constraints.push(format!("{name} :> {t}"));
                }
            }
            // `K extends keyof M`: 'K is the value the key selects. Where the
            // signature passes `M[K]` to a constrained interface, 'K has that constraint.
            for r in n.descendants().filter(|r| r.has_tag_name("ref")) {
                let Some(kept) = r.attribute("to").and_then(|t| self.constrained.get(t)) else { continue };
                for (i, arg) in elements(r).enumerate() {
                    let index = elements(arg).nth(1).filter(|_| arg.has_tag_name("indexedAccess"));
                    if let (Some(Some(c)), Some(k)) = (kept.get(i), index)
                        && k.attribute("name") == p.attribute("name")
                    {
                        let bound = format!("{name} :> {}", type_path(c));
                        if !constraints.contains(&bound) {
                            constraints.push(bound);
                        }
                    }
                }
            }
            params.push(name);
        }
        match (params.is_empty(), constraints.is_empty()) {
            (true, _) => String::new(),
            (false, true) => format!("<{}>", params.join(", ")),
            // `X<Y> >`: F# would read `>>` as an operator.
            (false, false) => format!("<{} when {}{}>", params.join(", "), constraints.join(" and "), if constraints.last().unwrap().ends_with('>') { " " } else { "" }),
        }
    }

    fn type_params(&self, n: Node) -> String {
        let params: Vec<String> = named(n, "typeParameter").map(|p| format!("'{}", attr(p, "name"))).collect();
        if params.is_empty() { String::new() } else { format!("<{}>", params.join(", ")) }
    }

    // `a: float * ?b: string -> Shape`, or the same as arguments of a static member.
    fn params(&mut self, n: Node, owner: &str) -> Vec<String> {
        self.enter_keyed(n);
        let mut out = Vec::new();
        let has_optional = named(n, "parameter").any(|p| flag(p, "optional"));
        for p in named(n, "parameter") {
            // An argument with one possible value is written into the call instead.
            if p.has_attribute("const") {
                continue;
            }
            let t = only_child(p);
            let name = match attr(p, "name") {
                "_" | "" => "arg".to_string(),
                n => ident(n),
            };
            // F# wants optional arguments last and a parameter array last: not
            // both. A method is two overloads (see `members`); elsewhere the rest is left out.
            if flag(p, "rest") && has_optional && self.split_rest != Some(true) {
                if self.split_rest.is_none() {
                    self.skip("rest parameter after optional ones");
                }
                continue;
            }
            let optional = flag(p, "optional") && self.split_rest != Some(true);
            // A key of a map: typed by the value it selects.
            if t.has_tag_name("ref") && attr(t, "kind") == "typeParameter" && let Some(map) = self.keyed.get(attr(t, "name")) {
                out.push(format!("{}{name}: TypedKey<{map}, '{}>", if optional { "?" } else { "" }, attr(t, "name")));
                continue;
            }
            if flag(p, "rest") {
                let element = if t.has_tag_name("array") { self.ty(only_child(t), owner) } else { "obj".into() };
                out.push(format!("[<System.ParamArray>] {name}: {element}[]"));
            } else {
                out.push(format!("{}{name}: {}", if optional { "?" } else { "" }, self.ty(t, owner)));
            }
        }
        out
    }

    // For a signature with fixed arguments: a name suffix made of their values,
    // and the argument list of the call with them written in. `first` is the
    // placeholder of the first real argument.
    fn fixed(&mut self, n: Node, first: usize) -> Option<(String, String)> {
        if !named(n, "parameter").any(|p| p.has_attribute("const")) {
            return None;
        }
        let mut suffix = Vec::new();
        let mut args = Vec::new();
        let mut next = first;
        for p in named(n, "parameter") {
            if let Some(value) = p.attribute("const") {
                suffix.push(super::value_name(value));
                args.push(if only_child(p).attribute("name") == Some("string") { format!("{value:?}") } else { value.to_string() });
            } else if flag(p, "optional") || flag(p, "rest") {
                // Whatever is left is passed through as given.
                args.push(format!("${next}..."));
                break;
            } else {
                args.push(format!("${next}"));
                next += 1;
            }
        }
        Some((suffix.join("_"), args.join(", ")))
    }

    fn returns(&mut self, n: Node, owner: &str) -> String {
        self.enter_keyed(n);
        named(n, "returns").next().map(|r| self.ty(only_child(r), owner)).unwrap_or("unit".into())
    }

    fn signature(&mut self, n: Node, owner: &str, ret: Option<&str>) -> String {
        let params = self.params(n, owner);
        let params = if params.is_empty() { "unit".to_string() } else { params.join(" * ") };
        let ret = ret.map(str::to_string).unwrap_or_else(|| self.returns(n, owner));
        format!("{params} -> {ret}")
    }

    // The members of one side of a type, as `abstract` lines. A member that
    // mentions a type from a later file cannot be abstract (F# files cannot
    // refer forward): it becomes an extension of `extend` in that file.
    fn members(&mut self, members: &[Node], owner: &str, extend: &str, indent: &str) -> Vec<String> {
        let mut lines: Vec<String> = Vec::new();
        let setters: HashSet<&str> = members.iter().filter(|m| m.has_tag_name("set")).map(|m| attr(*m, "name")).collect();
        for &m in members {
            let raw = m.attribute("name").unwrap_or_default();
            // `[Symbol.iterator]` makes the type a seq: see `interface`.
            if raw == "[Symbol.iterator]" || m.has_tag_name("set") {
                continue;
            }
            let fn_prop = m.has_tag_name("property") && !flag(m, "optional") && only_child(m).has_tag_name("functionType");
            let sig_node = if fn_prop { Some(only_child(m)) } else if m.has_tag_name("method") { Some(m) } else { None };
            let pairs: Vec<(String, String)> = if raw == "[Symbol.asyncIterator]" {
                // `x.asyncIterator ()`, whose `next ()` is a promise of the next value.
                let elem = named(m, "returns").next().and_then(|r| elements(r).next()).and_then(|it| elements(it).next());
                let t = elem.map_or("obj".to_string(), |e| self.ty(e, owner));
                let emit = "[<Emit(\"$0[Symbol.asyncIterator]()\")>]";
                vec![(format!("{emit} abstract asyncIterator: unit -> JsAsyncIterator<{t}>"), format!("{emit} member this.asyncIterator(): JsAsyncIterator<{t}> = jsNative"))]
            } else if raw.starts_with('[') {
                self.skip("symbol-named member");
                Vec::new()
            } else if sig_node.is_some_and(|s| named(s, "parameter").any(|p| flag(p, "rest")) && named(s, "parameter").any(|p| flag(p, "optional"))) {
                // `f(a, b?, ...rest)`: `f(a, ?b)` and `f(a, b, [<ParamArray>] rest)`.
                let mut both = Vec::new();
                for with_rest in [false, true] {
                    self.split_rest = Some(with_rest);
                    both.extend(self.member(m, owner, setters.contains(raw)));
                }
                self.split_rest = None;
                both
            } else {
                self.member(m, owner, setters.contains(raw)).into_iter().collect()
            };
            for (line, ext) in pairs {
                // The latest file a type in the member is declared in.
                let latest = words(&line).iter().filter_map(|w| self.file_of.get(*w)).copied().max().unwrap_or(0);
                if latest > self.current_file && indent.is_empty() {
                    let lines = self.extensions.entry(latest).or_default().entry(extend.to_string()).or_default();
                    let ext = format!("        {ext}\n");
                    if !lines.contains(&ext) {
                        lines.push(ext);
                    }
                    continue;
                }
                // Distinct TypeScript overloads can map to the same F# signature.
                self.marks.insert(line.trim().to_string(), super::marks(m));
                push_member(&mut lines, format!("{indent}    {line}\n"), &self.abbrevs, &self.marks);
            }
        }
        lines
    }

    // One member, as an `abstract` line and as an extension member.
    fn member(&mut self, m: Node, owner: &str, has_setter: bool) -> Option<(String, String)> {
        let raw = m.attribute("name").unwrap_or_default();
        let name = ident(raw);
        let js = js_access(m.attribute("jsName").unwrap_or(raw));
        let tps = self.constrained_type_params(m);
        let property = |name: &str, t: &str, settable: bool, js: &str| {
            // A `unit` cannot be set (`static protoInitialize: void`).
            let settable = settable && t != "unit";
            let get = format!("[<Emit(\"$0{}\")>] get (): {t} = jsNative", js.replace('"', "\\\""));
            let set = format!("[<Emit(\"$0{} = $1\")>] set (v: {t}) = jsNative", js.replace('"', "\\\""));
            (format!("abstract {name}: {t}{}", if settable { " with get, set" } else { "" }), if settable { format!("member this.{name} with {get} and {set}") } else { format!("member this.{name} with {get}") })
        };
        let method_from = |g: &mut Gen, sig_node: Node, tps: &str, name: &str, emit: String, attr: &str| {
            let params = g.params(sig_node, owner);
            let ret = g.returns(sig_node, owner);
            let sig = if params.is_empty() { "unit".to_string() } else { params.join(" * ") };
            let abstract_line = format!("{attr}abstract {name}{tps}: {sig} -> {ret}");
            let ext = format!("[<Emit(\"{}\")>] member this.{name}{tps}({}): {ret} = jsNative", emit.replace('"', "\\\""), params.join(", "));
            (abstract_line, ext)
        };
        Some(match m.tag_name().name() {
            "property" if !flag(m, "optional") && only_child(m).has_tag_name("functionType") => {
                let fn_node = only_child(m);
                let fn_tps = self.constrained_type_params(fn_node);
                match self.fixed(fn_node, 1) {
                    Some((suffix, args)) => {
                        let emit = format!("$0{js}({args})");
                        let fixed_name = ident(&format!("{raw}_{suffix}"));
                        let attr = format!("[<Emit(\"{}\")>] ", emit.replace('"', "\\\""));
                        method_from(self, fn_node, &fn_tps, &fixed_name, emit, &attr)
                    }
                    None => method_from(self, fn_node, &fn_tps, &name, format!("$0{js}($1...)"), ""),
                }
            }
            "property" => {
                let mut t = self.ty(only_child(m), owner);
                if flag(m, "optional") {
                    t += " option";
                }
                property(&name, &t, !flag(m, "readonly"), &js)
            }
            "get" => {
                let t = self.returns(m, owner);
                property(&name, &t, has_setter, &js)
            }
            "method" => match self.fixed(m, 1) {
                Some((suffix, args)) => {
                    let emit = format!("$0{js}({args})");
                    let fixed_name = ident(&format!("{raw}_{suffix}"));
                    let attr = format!("[<Emit(\"{}\")>] ", emit.replace('"', "\\\""));
                    method_from(self, m, &tps, &fixed_name, emit, &attr)
                }
                None => method_from(self, m, &tps, &name, format!("$0{js}($1...)"), ""),
            },
            "call" => method_from(self, m, &tps, "Invoke", "$0($1...)".into(), "[<Emit(\"$0($1...)\")>] "),
            "construct" | "constructor" => method_from(self, m, &tps, "Create", "new $0($1...)".into(), "[<EmitConstructor>] "),
            "index" => {
                let key = named(m, "parameter").next().map(|p| self.ty(only_child(p), owner)).unwrap_or("string".into());
                let value = self.returns(m, owner);
                let settable = !flag(m, "readonly");
                let abstract_line = format!("[<EmitIndexer>] abstract Item: key: {key} -> {value}{}", if settable { " with get, set" } else { "" });
                let get = format!("[<Emit(\"$0[$1]\")>] get (key: {key}): {value} = jsNative");
                let ext = if settable {
                    format!("member this.Item with {get} and [<Emit(\"$0[$1] = $2\")>] set (key: {key}) (v: {value}) = jsNative")
                } else {
                    format!("member this.Item with {get}")
                };
                (abstract_line, ext)
            }
            _ => {
                self.problems.element(T, m);
                return None;
            }
        })
    }

    fn interface(&mut self, d: Node, prefix: &str, indent: &str) {
        let name = type_ident(attr(d, "name"));
        // Reached in JavaScript by what it is exported as: `default`.
        let js_name = d.attribute("jsName").unwrap_or(attr(d, "name"));
        let path = js_path(&self.js_prefix, js_name);
        let params = self.type_params(d);
        let owner = format!("{name}{params}");
        // The declaration, and its extensions, state the constraints it keeps.
        let header = match d.attribute("id").and_then(|id| self.constrained.get(id)) {
            Some(cs) => {
                let ps: Vec<String> = named(d, "typeParameter").map(|p| format!("'{}", attr(p, "name"))).collect();
                // The constraint as F# names it: a built-in Fable has is its type (`JS.Date`).
                let when: Vec<String> = ps.iter().zip(cs).filter_map(|(p, c)| c.as_ref().map(|c| format!("{p} :> {}", fable_builtin(c).unwrap_or_else(|| type_path(c))))).collect();
                format!("{name}<{} when {} >", ps.join(", "), when.join(" and "))
            }
            None => owner.clone(),
        };
        let is_member = |m: &Node| matches!(m.tag_name().name(), "property" | "method" | "call" | "construct" | "index" | "get" | "set");
        // An alias of an object type has its members in the object.
        let src = if d.has_tag_name("typeAlias") { only_child_type(d) } else { d };
        let instance: Vec<Node> = elements(src).filter(|m| is_member(m) && !flag(*m, "static")).collect();
        let statics: Vec<Node> = elements(src).filter(|m| m.has_tag_name("constructor") || is_member(m) && flag(*m, "static")).collect();
        // A class of an imported library that unions tell apart: Fable tests it with instanceof.
        let origin = match self.tested_classes.get(attr(d, "name")) {
            Some(module) if prefix.is_empty() => format!("{indent}[<Import({js_name:?}, {module:?})>]\n"),
            _ => String::new(),
        };
        // A static member makes F# take it for a class unless told.
        let factory = if d.has_tag_name("class") { None } else { self.factory(d, &owner) };
        let kind = if factory.is_some() { "AllowNullLiteral; Interface" } else { "AllowNullLiteral" };
        self.out += &format!("{origin}{indent}[<{kind}>]\n{indent}type {header} =\n");
        let mut body = Vec::new();
        // A subclass of Array is a seq with an indexer and a length: an F#
        // interface cannot inherit ResizeArray, which is a class.
        let mut is_array = false;
        let direct: Vec<Option<&str>> = named(d, "extends").map(|e| only_child(e).attribute("to")).collect();
        let mut left: Vec<Node> = Vec::new();
        for (i, e) in named(d, "extends").enumerate() {
            // A generic base another base reaches already, maybe with other
            // arguments (`GPUDataOwner<'D>` beside a Container that is a
            // `GPUDataOwner<obj>`): F# allows one instantiation; the other path has it.
            let target = only_child(e);
            if elements(target).next().is_some()
                && let Some(to) = target.attribute("to")
                && direct.iter().enumerate().any(|(j, other)| j != i && other.is_some_and(|o| self.reaches(o, to)))
            {
                // Its members are declared again, with these arguments.
                left.push(target);
                continue;
            }
            let base = self.ty(target, &owner);
            match base.strip_prefix("ResizeArray<").and_then(|b| b.strip_suffix('>')) {
                Some(elem) => {
                    is_array = true;
                    body.push(format!("{indent}    inherit seq<{elem}>\n"));
                    body.push(format!("{indent}    [<EmitIndexer>] abstract Item: index: int -> {elem} with get, set\n"));
                    body.push(format!("{indent}    [<Emit(\"$0.length\")>] abstract length: int\n"));
                }
                None => body.push(format!("{indent}    inherit {base}\n")),
            }
        }
        // `[Symbol.iterator](): ArrayIterator<Node>`: a seq of Nodes, which Fable iterates by that method.
        let iterator = elements(d).find(|m| m.attribute("name") == Some("[Symbol.iterator]") && !flag(*m, "static"));
        // A type iterates as its ancestor does when that one is a seq already:
        // F# allows one seq per type (`HTMLCollectionOf<'T>` stays a seq of Elements).
        let mut todo: Vec<String> = self.bases.get(d.attribute("id").unwrap_or(attr(d, "name"))).cloned().unwrap_or_default();
        let mut seen = HashSet::new();
        let mut inherited = false;
        while let Some(b) = todo.pop() {
            if seen.insert(b.clone()) {
                inherited |= self.iterable.contains(&b);
                todo.extend(self.bases.get(&b).cloned().unwrap_or_default());
            }
        }
        if let Some(elem) = iterator.filter(|_| !inherited && !is_array).and_then(|m| named(m, "returns").next()).and_then(|r| elements(r).next()).and_then(|it| elements(it).next()) {
            let line = format!("inherit seq<{}>", self.ty(elem, &owner));
            // F# files cannot refer forward, and an inherited interface cannot be an extension.
            let latest = words(&line).iter().filter_map(|w| self.file_of.get(*w)).copied().max().unwrap_or(0);
            if latest > self.current_file && indent.is_empty() {
                self.skip("iterable over a type of a later file");
            } else {
                body.push(format!("{indent}    {line}\n"));
            }
        }
        let extend = if prefix.is_empty() { owner.clone() } else { format!("{}.{owner}", prefix.split('.').map(type_ident).collect::<Vec<_>>().join(".")) };
        if header != owner {
            let qualified = if prefix.is_empty() { header.clone() } else { format!("{}.{header}", prefix.split('.').map(type_ident).collect::<Vec<_>>().join(".")) };
            self.constrained_owner.insert(extend.clone(), qualified);
        }
        body.extend(self.members(&instance, &owner, &extend, indent));
        body.extend(self.redeclared(d, &left, &body, &owner, &extend, indent));
        if let Some(line) = factory {
            body.push(format!("{indent}    [<ParamObject; Emit(\"$0\")>]\n{indent}    static member Create ({line}) : {owner} = jsNative\n"));
        }
        if body.is_empty() {
            body.push(format!("{indent}    interface end\n"));
        }
        self.out += &body.concat();
        self.out += "\n";
        if d.has_tag_name("class") {
            self.static_side(&name, &path, &statics, Some(&owner), indent);
        }
    }

    // An options bag (only properties, its bases' too) is made with named
    // arguments: `AddEventListenerOptions.Create (once = true)`, the object
    // they make, by Fable's ParamObject. Not where a value of its name has
    // a `Create` (a class), nor where an argument would be of a later file.
    fn factory(&mut self, d: Node, owner: &str) -> Option<String> {
        let root = d.document().root_element();
        let raw = attr(d, "name");
        let constrained = d.attribute("id").is_some_and(|id| self.constrained.contains_key(id));
        if constrained || d.parent().is_some_and(|p| elements(p).any(|v| v.has_tag_name("var") && attr(v, "name") == raw)) {
            return None;
        }
        // Own properties first, then each base's not declared again.
        let mut props: Vec<Node> = Vec::new();
        let mut todo = vec![d];
        let mut seen = HashSet::new();
        while let Some(t) = todo.pop() {
            if !seen.insert(t.attribute("id").unwrap_or(attr(t, "name")).to_string()) {
                continue;
            }
            let src = if t.has_tag_name("typeAlias") { only_child_type(t) } else { t };
            for m in elements(src).filter(|m| !m.has_tag_name("typeParameter") && !m.has_tag_name("extends")) {
                if !m.has_tag_name("property") || flag(m, "static") {
                    return None;
                }
                if !props.iter().any(|p| attr(*p, "name") == attr(m, "name")) {
                    props.push(m);
                }
            }
            for e in named(t, "extends") {
                let r = only_child(e);
                match r.attribute("to").filter(|_| elements(r).next().is_none()).and_then(|to| find_decl(root, to)) {
                    Some(b) if b.has_tag_name("interface") => todo.push(b),
                    _ => return None,
                }
            }
        }
        let plain = |n: &str| !n.is_empty() && n.chars().all(|c| c.is_alphanumeric() || c == '_') && !n.as_bytes()[0].is_ascii_digit();
        if props.is_empty() || props.iter().any(|p| !plain(attr(*p, "name")) || attr(*p, "name") == "Create") {
            return None;
        }
        let (required, optional): (Vec<Node>, Vec<Node>) = props.into_iter().partition(|p| !flag(*p, "optional"));
        let mut args = Vec::new();
        for p in required.iter().chain(&optional) {
            let t = self.ty(only_child(*p), owner);
            let q = if flag(*p, "optional") { "?" } else { "" };
            args.push(format!("{q}{}: {t}", ident(attr(*p, "name"))));
        }
        let line = args.join(", ");
        if words(&line).iter().filter_map(|w| self.file_of.get(*w)).any(|f| *f > self.current_file) {
            return None;
        }
        Some(line)
    }

    // A value with constructors and static members: a type for its shape and a global of that type.
    fn static_side(&mut self, name: &str, path: &str, members: &[Node], instance: Option<&str>, indent: &str) {
        let static_name = type_ident(&format!("{}Static", name.trim_matches('`')));
        self.out += &format!("{indent}[<AllowNullLiteral>]\n{indent}type {static_name} =\n");
        let mut lines = Vec::new();
        for &m in members {
            if (m.has_tag_name("constructor") || m.has_tag_name("construct")) && instance.is_some() {
                let ret = instance.unwrap().to_string();
                let tps = self.type_params(m);
                let sig = self.signature(m, "", Some(&ret));
                let latest = words(&sig).iter().filter_map(|w| self.file_of.get(*w)).copied().max().unwrap_or(0);
                if latest > self.current_file && indent.is_empty() {
                    let params = self.params(m, "");
                    let ext = format!("        [<Emit(\"new $0($1...)\")>] member this.Create{tps}({}): {ret} = jsNative\n", params.join(", "));
                    let lines = self.extensions.entry(latest).or_default().entry(static_name.clone()).or_default();
                    if !lines.contains(&ext) {
                        lines.push(ext);
                    }
                    continue;
                }
                let line = format!("{indent}    [<EmitConstructor>] abstract Create{tps}: {sig}\n");
                self.marks.insert(line.trim().to_string(), super::marks(m));
                push_member(&mut lines, line, &self.abbrevs, &self.marks);
            } else {
                for line in self.members(&[m], "obj", &static_name, indent) {
                    push_member(&mut lines, line, &self.abbrevs, &self.marks);
                }
            }
        }
        if lines.is_empty() {
            lines.push(format!("{indent}    interface end\n"));
        }
        self.out += &lines.concat();
        self.out += "\n";
        let value = format!("{indent}{}\n{indent}let {name}: {static_name} = jsNative\n\n", self.at(&path, None));
        if indent.is_empty() {
            self.values += &value;
        } else {
            self.out += &value;
        }
    }

    fn decls(&mut self, scope: Node, prefix: &str, indent: &str) {
        // Functions can be overloaded and take optional arguments, which only
        // members can do in F#: they become static members of one erased type.
        let mut functions: Vec<String> = Vec::new();
        for d in elements(scope) {
            // Each top-level declaration is imported from its package; a namespace's members from its.
            if prefix.is_empty() {
                self.import = d.attribute("import").map(str::to_string).or_else(|| self.root_import.clone());
            }
            let raw = d.attribute("name").unwrap_or_default();
            let name = if matches!(d.tag_name().name(), "interface" | "class" | "typeAlias" | "enum" | "namespace") { type_ident(raw) } else { ident(raw) };
            let path = js_path(&self.js_prefix, d.attribute("jsName").unwrap_or(raw));
            // A top-level declaration's types go in its module's file; an alias where its parts are.
            if indent.is_empty()
                && let Some(module) = d.attribute("module")
            {
                let unit = self.units.get(raw).cloned().unwrap_or_else(|| module.to_string());
                self.current_file = if d.has_tag_name("typeAlias") { self.file_of.get(raw).copied().unwrap_or(0) } else { self.file_of.get(&format!("module:{unit}")).copied().unwrap_or(0) };
            }
            let start = self.out.len();
            let prev_globals = std::mem::replace(&mut self.in_globals, indent.is_empty() && !d.has_attribute("module"));
            let prev_indent = std::mem::replace(&mut self.cur_indent, indent.to_string());
            let prev_prefix = std::mem::replace(&mut self.cur_prefix, prefix.to_string());
            self.decl(d, raw, &name, &path, prefix, indent, &mut functions);
            self.in_globals = prev_globals;
            self.cur_indent = prev_indent;
            self.cur_prefix = prev_prefix;
            if !self.pending_delegates.is_empty() {
                let dels = std::mem::take(&mut self.pending_delegates);
                self.out.insert_str(start, &dels);
            }
            if indent.is_empty()
                && let Some(module) = d.attribute("module")
            {
                let text = self.out.split_off(start);
                if d.has_tag_name("typeAlias") {
                    let file = self.file_of.get(raw).copied().unwrap_or(0);
                    *self.alias_types.entry(file).or_default() += &text;
                } else {
                    let unit = self.units.get(raw).cloned().unwrap_or_else(|| module.to_string());
                    *self.types.entry(unit).or_default() += &text;
                }
            }
        }
        if !functions.is_empty() {
            match self.carrier.as_mut() {
                Some(carrier) => carrier.extend(functions),
                None => self.out += &format!("{indent}[<Erase>]\n{indent}type Globals =\n{}\n", functions.concat()),
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn decl(&mut self, d: Node, raw: &str, name: &str, path: &str, prefix: &str, indent: &str, functions: &mut Vec<String>) {
        let name = name.to_string();
        let path = path.to_string();
        {
            match d.tag_name().name() {
                "lowering" => {}
                "namespace" => {
                    self.out += &format!("{indent}module {name} =\n");
                    let before = self.out.len();
                    let inner = js_path(&self.js_prefix, d.attribute("jsName").unwrap_or(raw));
                    let outer = std::mem::replace(&mut self.js_prefix, inner);
                    // Its functions on a type of its name beside it (`Geometry.point`,
                    // `Geometry.Deep.twice`), F# resolving `Geometry.Point` in the
                    // module still; in it, as `Globals`, where its scope has
                    // something else of that name (node's class `Stream`).
                    let alone = d.parent().is_some_and(|p| !elements(p).any(|o| o != d && o.attribute("name") == Some(raw)));
                    let outer_carrier = std::mem::replace(&mut self.carrier, alone.then(Vec::new));
                    self.decls(d, &js_path(prefix, raw), &format!("{indent}    "));
                    let functions = std::mem::replace(&mut self.carrier, outer_carrier);
                    self.js_prefix = outer;
                    if self.out.len() == before {
                        self.out += &format!("{indent}    let private empty = ()\n\n");
                    }
                    if let Some(functions) = functions.filter(|f| !f.is_empty()) {
                        self.out += &format!("{indent}[<Erase>]\n{indent}type {name} =\n{}\n", functions.concat());
                    }
                }
                "enum" => {
                    if named(d, "member").any(|v| attr(v, "type") != "string") {
                        self.numeric_enum(d, &name, indent);
                        return;
                    }
                    self.out += &format!("{indent}[<StringEnum; RequireQualifiedAccess>]\n{indent}type {name} =\n");
                    let mut seen = HashSet::new();
                    for v in named(d, "member") {
                        let value = attr(v, "value");
                        let mut case: String = value.chars().map(|c| if c.is_alphanumeric() { c } else { '_' }).collect();
                        if case.is_empty() || !case.as_bytes()[0].is_ascii_alphabetic() {
                            case.insert(0, 'V');
                        }
                        case[..1].make_ascii_uppercase();
                        while !seen.insert(case.clone()) {
                            case.push('_');
                        }
                        self.out += &format!("{indent}    | [<CompiledName({value:?})>] {case}\n");
                    }
                    self.out += "\n";
                }
                // Only a call signature (`EventListener`): a function, written as a
                // lambda. Unless a type extends it (F# inherits only interfaces),
                // or it names a later file's type (only a member can go there).
                "interface" if is_function_interface(d) && !self.is_extended(d) => {
                    let call = only_child(d);
                    let t = if named(call, "parameter").any(|p| flag(p, "optional")) {
                        format!("delegate of {}", self.delegate_sig(call, ""))
                    } else {
                        self.ty(call, "")
                    };
                    if words(&t).iter().filter_map(|w| self.file_of.get(*w)).any(|f| *f > self.current_file) {
                        self.interface(d, prefix, indent);
                        return;
                    }
                    if !t.starts_with("delegate") {
                        self.abbrevs.insert(name.to_string(), t.clone());
                    }
                    self.out += &format!("{indent}type {name} = {t}\n\n");
                }
                "interface" | "class" => self.interface(d, prefix, indent),
                "function" => {
                    let params = self.params(d, "");
                    let ret = self.returns(d, "");
                    let (name, args) = match self.fixed(d, 0) {
                        // Escaped where it is written, by `at`.
                        Some((suffix, args)) => (ident(&format!("{raw}_{suffix}")), args),
                        None => (name, "$0...".to_string()),
                    };
                    let tps = self.constrained_type_params(d);
                    let line = format!("{indent}    {} static member {name}{tps} ({}): {ret} = jsNative\n", self.at(&path, Some(&args)), params.join(", "));
                    self.marks.insert(line.trim().to_string(), super::marks(d));
                    push_member(functions, line, &self.abbrevs, &self.marks);
                }
                "typeAlias" if only_child_type(d).has_tag_name("union") && named(d, "typeParameter").next().is_none() => {
                    let body = only_child_type(d);
                    if !self.named_union(&name, body, indent) {
                        // Erased: its types, though a match cannot tell them apart.
                        let recursive = body.descendants().any(|r| r.has_tag_name("ref") && r.attribute("name") == Some(raw));
                        let t = if recursive {
                            self.skip("recursive union, bound as obj");
                            "obj".to_string()
                        } else {
                            self.skip("union whose members cannot be told apart, erased");
                            self.ty(body, "")
                        };
                        self.abbrevs.insert(name.to_string(), t.clone());
                        self.out += &format!("{indent}type {name} = {t}\n\n");
                    }
                }
                "typeAlias" => {
                    let body = only_child_type(d);
                    let id = d.attribute("id").unwrap_or(raw);
                    let params = match self.alias_kept.get(id) {
                        Some(kept) => {
                            let all: Vec<Node> = named(d, "typeParameter").collect();
                            let shown: Vec<String> = kept.iter().map(|&i| format!("'{}", attr(all[i], "name"))).collect();
                            if shown.is_empty() { String::new() } else { format!("<{}>", shown.join(", ")) }
                        }
                        None => self.type_params(d),
                    };
                    // The constraints its type needs of those it shows, written as F# wants them.
                    let kept = self.alias_kept.get(id).cloned();
                    let when: Vec<String> = match self.constrained.get(id) {
                        Some(cs) => named(d, "typeParameter")
                            .zip(cs)
                            .enumerate()
                            .filter(|(i, _)| kept.as_ref().is_none_or(|k| k.contains(i)))
                            .filter_map(|(_, (p, c))| c.as_ref().map(|c| format!("'{} :> {}", attr(p, "name"), fable_builtin(c).unwrap_or_else(|| type_path(c)))))
                            .collect(),
                        None => Vec::new(),
                    };
                    let header = if when.is_empty() || params.is_empty() { params.clone() } else { format!("{} when {} >", params.trim_end_matches('>'), when.join(" and ")) };
                    match self.alias_form(d) {
                        AliasForm::Interface => self.interface(d, prefix, indent),
                        AliasForm::Delegate => {
                            // A function value's own type parameters are `obj`, as in `ty`.
                            let depth = self.fn_params.len();
                            self.fn_params.extend(named(body, "typeParameter").map(|p| attr(p, "name").to_string()));
                            let sig_str = self.delegate_sig(body, "");
                            self.fn_params.truncate(depth);
                            self.out += &format!("{indent}type {name}{header} = delegate of {sig_str}\n\n");
                        }
                        AliasForm::Abbreviation(t) => {
                            if params.is_empty() {
                                self.abbrevs.insert(name.to_string(), t.clone());
                            }
                            self.out += &format!("{indent}type {name}{header} = {t}\n\n");
                        }
                    }
                }
                "var" => {
                    let body = only_child(d);
                    if body.has_tag_name("object") {
                        let members: Vec<Node> = elements(body).collect();
                        self.static_side(&name, &path, &members, None, indent);
                    } else {
                        let t = self.ty(body, "");
                        let value = format!("{indent}{}\n{indent}let {name}: {t} = jsNative\n\n", self.at(&path, None));
                        if indent.is_empty() {
                            self.values += &value;
                        } else {
                            self.out += &value;
                        }
                    }
                }
                _ => self.problems.element(T, d),
            }
        }
    }

    // A union declared as an F# type: told apart by a property when it has a
    // discriminant, else erased, each case with the runtime test the XML gives.
    // False if Fable could not tell the cases apart.
    fn named_union(&mut self, name: &str, union: Node, indent: &str) -> bool {
        let members: Vec<Node> = elements(union).collect();
        let mut cases: Vec<(String, String, Option<String>)> = Vec::new();
        let mut seen_names = HashSet::new();
        let mut seen_tests = HashSet::new();
        let mut untested = 0;
        for m in &members {
            let payload = self.ty(*m, "");
            let mut case = case_name(*m);
            while !seen_names.insert(case.clone()) {
                case.push('_');
            }
            let test = m.attribute("test").map(str::to_string);
            // Fable tests these itself; anything else needs a test from the XML.
            let native = matches!(payload.as_str(), "string" | "float" | "bool");
            match &test {
                Some(t) if !seen_tests.insert(t.clone()) => return false,
                None if !native => untested += 1,
                _ => {}
            }
            cases.push((case, payload, test));
        }
        if untested > 1 || cases.iter().any(|c| c.1 == "obj" || c.1 == "unit") {
            return false;
        }
        // Fable tags a case by its interface's member: each must be declared here.
        let declared = members.iter().all(|m| m.has_tag_name("ref") && m.has_attribute("to"));
        if let Some(d) = union.attribute("discriminant").filter(|_| declared) {
            self.out += &format!("{indent}[<TypeScriptTaggedUnion({d:?})>]\n{indent}type {name} =\n");
            for (case, payload, test) in &cases {
                let value = test.as_deref().and_then(|t| t.strip_prefix("tag:")).unwrap_or_default();
                self.out += &format!("{indent}    | [<CompiledName({value:?})>] {case} of {payload}\n");
            }
        } else {
            self.out += &format!("{indent}[<Erase; RequireQualifiedAccess>]\n{indent}type {name} =\n");
            for (case, payload, test) in &cases {
                let native = matches!(payload.as_str(), "string" | "float" | "bool");
                let class = test.as_deref().and_then(|t| t.strip_prefix("instanceof:"));
                let imported = class.is_some_and(|c| self.tested_classes.contains_key(c));
                // Where another case's class extends this one's (`Pt extends
                // Float32Array`), this case is only an instance of exactly this class.
                let exact = class.filter(|c| !c.contains('.')).filter(|c| cases.iter().any(|(_, payload, _)| payload != c && self.extends_class(payload, c)));
                let attr = match exact {
                    Some(c) if !native && !imported => format!("[<CaseTest(\"$0 != null && Object.getPrototypeOf($0) === globalThis.{c}.prototype\")>] "),
                    _ => match test.as_deref().and_then(|t| case_test(t, union.attribute("discriminant"))) {
                    Some(js) if !native && !imported => format!("[<CaseTest(\"{}\")>] ", js.replace('"', "\\\"")),
                    _ => String::new(),
                    },
                };
                self.out += &format!("{indent}    | {attr}{case} of {payload}\n");
            }
        }
        // A value of a case's type converts to the union: implicitly where an
        // argument is one, with `!^` elsewhere and from subtypes. Once per
        // type F# sees (`DOMHighResTimeStamp` is a float); never from a type
        // parameter, `obj`, `unit` or the union itself.
        if !name.contains('<') {
            let mut seen = HashSet::new();
            for ((case, payload, _), m) in cases.iter().zip(&members) {
                let seen_as = expand(&self.through_aliases(*m), &self.abbrevs);
                if payload.contains('\'') || matches!(payload.as_str(), "obj" | "unit") || payload == name || !seen.insert(seen_as) {
                    continue;
                }
                for op in ["op_Implicit", "op_ErasedCast"] {
                    self.out += &format!("{indent}    static member inline {op} (x: {payload}) : {name} = {name}.{case} x\n");
                }
            }
        }
        self.out += "\n";
        true
    }

    // A type as F# sees it through aliases not declared yet (`OrdinalNumber`
    // is a float): what an abbreviation of it would stand for.
    fn through_aliases<'a>(&mut self, mut t: Node<'a, 'a>) -> String {
        let root = t.document().root_element();
        for _ in 0..16 {
            let alias = t.has_tag_name("ref").then(|| t.attribute("to")).flatten().filter(|_| attr(t, "kind") == "typeAlias" && elements(t).next().is_none()).and_then(|to| find_decl(root, to));
            match alias.filter(|d| named(*d, "typeParameter").next().is_none()) {
                Some(d) => t = only_child_type(d),
                None => break,
            }
        }
        let saved = (std::mem::take(&mut self.skipped), std::mem::take(&mut self.problems));
        let out = self.ty(t, "");
        (self.skipped, self.problems) = saved;
        out
    }

    fn numeric_enum(&mut self, d: Node, name: &str, indent: &str) {
        self.out += &format!("{indent}[<RequireQualifiedAccess>]\n{indent}type {name} =\n");
        for (i, v) in named(d, "member").enumerate() {
            let value = if attr(v, "type") == "number" { attr(v, "value").to_string() } else { i.to_string() };
            self.out += &format!("{indent}    | {} = {value}\n", ident(attr(v, "name")));
        }
        self.out += "\n";
    }
}

// `.name`, or `["some-name"]` where the name is not an identifier.
fn js_access(name: &str) -> String {
    let plain = !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '$') && !name.as_bytes()[0].is_ascii_digit();
    if plain { format!(".{name}") } else { format!("[{name:?}]") }
}

// A JavaScript built-in as Fable has it: `JS.Date`; an Error is an F#
// exception, as Fable catches it.
fn fable_builtin(name: &str) -> Option<String> {
    match name {
        "ArrayBuffer" | "ArrayBufferView" | "DataView" | "Date" | "Uint8Array" | "Uint8ClampedArray" | "Int16Array" | "Int32Array" | "Uint32Array" | "Float32Array" | "Float64Array" | "BigInt64Array" => Some(format!("JS.{name}")),
        "Error" => Some("exn".into()),
        _ => None,
    }
}

// Whether a reference is a base type: `<extends>`, or an argument of one.
fn is_base(n: Node) -> bool {
    n.ancestors().skip(1).any(|a| a.has_tag_name("extends"))
}

// `$0, $1...` as `$1, $2...`.
fn shift(args: &str) -> String {
    let mut out = String::new();
    let mut chars = args.chars().peekable();
    while let Some(c) = chars.next() {
        out.push(c);
        if c == '$' && chars.peek().is_some_and(char::is_ascii_digit) {
            let mut n = String::new();
            while let Some(d) = chars.peek().filter(|d| d.is_ascii_digit()) {
                n.push(*d);
                chars.next();
            }
            out += &(n.parse::<usize>().unwrap() + 1).to_string();
        }
    }
    out
}

// The JavaScript test of a union member, from its `test` attribute; `$0` is the value.
fn case_test(test: &str, discriminant: Option<&str>) -> Option<String> {
    let (kind, arg) = test.split_once(':').unwrap_or((test, ""));
    Some(match kind {
        "tag" => format!("$0 != null && String($0[{:?}]) === {arg:?}", discriminant?),
        "typeof" => format!("typeof $0 === '{arg}'"),
        "array" => "Array.isArray($0)".into(),
        "instanceof" => format!("$0 instanceof globalThis{}", arg.split('.').map(js_access).collect::<String>()),
        "has" => format!("typeof $0 === 'object' && $0 !== null && {arg:?} in $0"),
        _ => return None,
    })
}

// The name of a union case, from its member: `String`, `Widget`, `Array`.
fn case_name(m: Node) -> String {
    let raw = match m.tag_name().name() {
        "primitive" => match attr(m, "name") {
            "number" => "Number".to_string(),
            "boolean" => "Bool".to_string(),
            p => p.to_string(),
        },
        "ref" => attr(m, "name").rsplit('.').next().unwrap_or_default().to_string(),
        "array" => "Array".into(),
        "tuple" => "Tuple".into(),
        "functionType" => "Function".into(),
        "object" => "Object".into(),
        t => t.to_string(),
    };
    let mut case: String = raw.chars().map(|c| if c.is_alphanumeric() { c } else { '_' }).collect();
    if case.is_empty() || !case.as_bytes()[0].is_ascii_alphabetic() {
        case.insert(0, 'C');
    }
    case[..1].make_ascii_uppercase();
    case
}

// A declaration by qualified name: `WebAssembly.Module`.
fn find_decl<'a>(root: Node<'a, 'a>, path: &str) -> Option<Node<'a, 'a>> {
    let mut scope = root;
    let mut parts = path.split('.').peekable();
    while let Some(part) = parts.next() {
        let last = parts.peek().is_none();
        scope = elements(scope).find(|d| d.attribute("name") == Some(part) && (last || d.has_tag_name("namespace")))?;
    }
    Some(scope)
}

// The properties of a map, its own and those of the maps it extends.
fn map_properties<'a>(root: Node<'a, 'a>, decl: Node<'a, 'a>) -> Vec<(String, Node<'a, 'a>)> {
    let mut out: Vec<(String, Node)> = named(decl, "property").map(|p| (attr(p, "name").to_string(), only_child(p))).collect();
    for e in named(decl, "extends") {
        let base = only_child(e);
        if let Some(b) = base.attribute("to").and_then(|t| find_decl(root, t)) {
            out.extend(map_properties(root, b));
        }
    }
    out
}

// A declaration's path (`Konva.Shape`) as F# names it.
fn type_path(path: &str) -> String {
    path.split('.').map(type_ident).collect::<Vec<_>>().join(".")
}

fn js_path(prefix: &str, name: &str) -> String {
    if prefix.is_empty() { name.to_string() } else { format!("{prefix}.{name}") }
}

// The type of an alias, after any type parameters.
fn only_child_type<'a>(d: Node<'a, 'a>) -> Node<'a, 'a> {
    elements(d).find(|c| !c.has_tag_name("typeParameter")).unwrap_or_else(|| panic!("alias {} has no type", attr(d, "name")))
}

// Adds a member. One F# would call as another is there (TypeScript
// overloads apart only in what F# leaves out: a literal option, a
// constraint): the same line is left out. Else each is named for what it
// has that the other has not, `merge` and `mergeDictionary`, `marked` and
// `markedAsync`, the one with nothing of its own keeping the name; where
// neither has, the next free `name_2`. Both call the same JavaScript.
fn push_member(lines: &mut Vec<String>, line: String, abbrevs: &HashMap<String, String>, marks: &HashMap<String, Vec<String>>) {
    let (head, types) = member_key(&line);
    let types = types.map(|t| expand(&t, abbrevs));
    let same = |l: &String| {
        let (h, t) = member_key(l);
        h == head && t.map(|t| expand(&t, abbrevs)) == types
    };
    let Some(at) = lines.iter().position(|l| same(l)) else {
        lines.push(line);
        return;
    };
    let other = lines[at].clone();
    if types.is_none() || expand(other.trim(), abbrevs) == expand(line.trim(), abbrevs) {
        return;
    }
    let Some((_, name)) = member_name(&line) else { return };
    let bare = name.trim_matches('`').to_string();
    let mark = |l: &str| marks.get(l.trim()).cloned().unwrap_or_default();
    let (mine, theirs) = (mark(&line), mark(&other));
    let own = |a: &[String], b: &[String]| a.iter().filter(|m| !b.contains(m)).map(|m| pascal(m)).collect::<String>();
    let (new_own, old_own) = (own(&mine, &theirs), own(&theirs, &mine));
    let mut taken: HashSet<String> = lines.iter().filter_map(|l| member_name(l)).map(|(_, n)| n.to_string()).collect();
    let mut free = |suffix: &str| -> String {
        let base = format!("{bare}{suffix}");
        let n = std::iter::once(ident(&base)).chain((2..).map(|n| ident(&format!("{base}_{n}")))).find(|n| !taken.contains(n)).unwrap();
        taken.insert(n.clone());
        n
    };
    if !old_own.is_empty() && new_own.is_empty() {
        // The one there has what this has not: it is renamed, this takes the name.
        let renamed = free(&old_own);
        lines[at] = rename_member(&other, &renamed);
        lines.push(line);
    } else {
        let renamed = free(&new_own);
        lines.push(rename_member(&line, &renamed));
    }
}

// A member line under another F# name, calling the same JavaScript.
fn rename_member(line: &str, to: &str) -> String {
    let Some((at, name)) = member_name(line) else { return line.to_string() };
    let bare = name.trim_matches('`');
    let emit = if line.contains("Emit") {
        String::new()
    } else if ident(bare) == bare {
        format!("[<Emit(\"$0.{bare}($1...)\")>] ")
    } else {
        format!("[<Emit(\"$0[\\\"{bare}\\\"]($1...)\")>] ")
    };
    let start = line.len() - line.trim_start().len();
    format!("{}{emit}{}{to}{}", &line[..start], &line[start..at], &line[at + name.len()..])
}

// `async` as `Async`, `no-cache` as `NoCache`.
fn pascal(word: &str) -> String {
    let mut out = String::new();
    let mut up = true;
    for c in word.chars() {
        if c.is_alphanumeric() {
            if up { out.extend(c.to_uppercase()) } else { out.push(c) }
            up = false;
        } else {
            up = true;
        }
    }
    out
}

// Where in a member line its name is, and the name.
fn member_name(line: &str) -> Option<(usize, &str)> {
    let mut s = line.trim_start();
    while let Some(rest) = s.strip_prefix("[<") {
        s = rest[rest.find(">]")? + 2..].trim_start();
    }
    let rest = s.strip_prefix("abstract ").or_else(|| s.strip_prefix("static member "))?;
    let start = line.len() - rest.len();
    let len = if rest.starts_with("``") { rest[2..].find("``")? + 4 } else { rest.find(|c: char| !(c.is_alphanumeric() || c == '_')).unwrap_or(rest.len()) };
    Some((start, &line[start..start + len]))
}

// `text` with each abbreviation's name replaced by what it stands for.
fn expand(text: &str, abbrevs: &HashMap<String, String>) -> String {
    let mut out = text.to_string();
    for _ in 0..4 {
        let mut next = String::new();
        let mut word = String::new();
        for c in out.chars().chain(['\0']) {
            if c.is_alphanumeric() || c == '_' || c == '\'' {
                word.push(c);
                continue;
            }
            next += abbrevs.get(&word).map_or(word.as_str(), String::as_str);
            word.clear();
            if c != '\0' {
                next.push(c);
            }
        }
        if next == out {
            break;
        }
        out = next;
    }
    out
}

// What F# tells members apart by: name and parameter types, not attributes,
// parameter names or return type. Overloads that differ only in a literal
// (`async: true` or `false`, each a `bool`) are one F# member.
fn member_key(line: &str) -> (String, Option<String>) {
    let mut s = line.trim();
    while let Some(rest) = s.strip_prefix("[<") {
        match rest.find(">]") {
            Some(i) => s = rest[i + 2..].trim_start(),
            None => break,
        }
    }
    // Splits `text` at `sep` outside brackets.
    fn split_top<'a>(text: &'a str, sep: &str) -> Vec<&'a str> {
        let b = text.as_bytes();
        let (mut depth, mut start, mut out, mut i) = (0i32, 0, Vec::new(), 0);
        while i < b.len() {
            match b[i] {
                b'-' if b.get(i + 1) == Some(&b'>') => i += 1,
                b'(' | b'<' | b'[' | b'{' => depth += 1,
                b')' | b'>' | b']' | b'}' => depth -= 1,
                _ if depth == 0 && text[i..].starts_with(sep) => {
                    out.push(&text[start..i]);
                    i += sep.len();
                    start = i;
                    continue;
                }
                _ => {}
            }
            i += 1;
        }
        out.push(&text[start..]);
        out
    }
    // `name: T` or `?name: T` is its type, with the `?`.
    let type_of = |p: &str| -> String {
        let p = p.trim();
        let optional = p.starts_with('?');
        let parts = split_top(p, ": ");
        let t = if parts.len() > 1 { parts[1..].join(": ").trim().to_string() } else { p.to_string() };
        if optional { format!("?{t}") } else { t }
    };
    if let Some(rest) = s.strip_prefix("static member ") {
        // `static member f<'T> (a: T, b: U): R = jsNative`.
        let Some(open) = rest.find(" (") else { return (s.to_string(), None) };
        let name = &rest[..open];
        let inner = &rest[open + 2..];
        let close = {
            let (mut depth, mut at) = (1i32, inner.len());
            for (i, c) in inner.char_indices() {
                match c {
                    '(' | '<' | '[' | '{' => depth += 1,
                    ')' | ']' | '}' => {
                        depth -= 1;
                        if depth == 0 {
                            at = i;
                            break;
                        }
                    }
                    '>' if !inner[..i].ends_with('-') => depth -= 1,
                    _ => {}
                }
            }
            at
        };
        let params: Vec<String> = split_top(&inner[..close], ", ").into_iter().filter(|p| !p.trim().is_empty()).map(type_of).collect();
        // F# tells no overloads apart by their constraints: the name alone.
        return (format!("static {}", name.split('<').next().unwrap_or(name)), Some(params.join(", ")));
    }
    // `abstract f: a: A * b: B -> R`: the types before its last arrow outside brackets.
    let Some(rest) = s.strip_prefix("abstract ") else { return (s.to_string(), None) };
    let Some((name, sig)) = rest.split_once(": ") else { return (s.to_string(), None) };
    let parts = split_top(sig, " -> ");
    if parts.len() < 2 {
        return (s.to_string(), None);
    }
    let params = parts[..parts.len() - 1].join(" -> ");
    let types: Vec<String> = split_top(&params, " * ").into_iter().map(type_of).collect();
    (format!("abstract {}", name.split('<').next().unwrap_or(name)), Some(types.join(" * ")))
}

// Fable's erased union of these types: `U2<A, B>` up to `U9`; more nest in
// the last place, `U9<A, ..., H, U3<I, J, K>>`.
fn erased_union(mut parts: Vec<String>) -> String {
    match parts.len() {
        0 => "obj".into(),
        1 => parts.pop().unwrap(),
        2..=9 => format!("U{}<{}>", parts.len(), parts.join(", ")),
        _ => {
            let rest = parts.split_off(8);
            parts.push(erased_union(rest));
            format!("U9<{}>", parts.join(", "))
        }
    }
}

enum AliasForm {
    Interface,
    Delegate,
    Abbreviation(String),
}

// `text` with each type parameter `'T` in `map` replaced by its type.
fn subst_params(text: &str, map: &HashMap<String, String>) -> String {
    if map.is_empty() {
        return text.to_string();
    }
    let mut out = String::new();
    let mut chars = text.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        let after_word = text[..i].chars().last().is_some_and(|p| p.is_alphanumeric() || p == '_');
        if c == '\'' && !after_word {
            let start = i + 1;
            let mut end = start;
            while let Some(&(j, n)) = chars.peek() {
                if n.is_alphanumeric() || n == '_' {
                    end = j + n.len_utf8();
                    chars.next();
                } else {
                    break;
                }
            }
            match map.get(&text[start..end]) {
                Some(t) => out += t,
                None => out += &text[i..end.max(start)],
            }
            continue;
        }
        out.push(c);
    }
    out
}

// An interface of only a call signature: a function type.
fn is_function_interface(d: Node) -> bool {
    let mut members = elements(d);
    members.next().is_some_and(|m| m.has_tag_name("call")) && members.next().is_none()
}
