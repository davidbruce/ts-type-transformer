// XML -> F# bindings for Fable. Keeps inheritance, generics, overloads,
// optional parameters and unions; needs literals and type-level expressions gone.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use roxmltree::{Document, Node};

use super::{attr, elements, flag, named, only_child, unsupported};

const T: &str = "F#";

pub struct Output {
    // File name and contents, in compilation order.
    pub files: Vec<(String, String)>,
    // What the generator does not bind yet, by reason.
    pub skipped: BTreeMap<String, usize>,
}

struct Gen {
    out: String,
    skipped: BTreeMap<String, usize>,
    // Qualified names declared in the XML.
    declared: HashSet<String>,
    // Types declared elsewhere, with the most type arguments seen.
    external: BTreeMap<String, usize>,
    // In the signature being written: type parameters over a map's keys, and their map.
    keyed: HashMap<String, String>,
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
    constrained_owner: HashMap<String, String>,
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
    let constraint_of = |p: Node| elements(p).find(|c| c.has_tag_name("constraint")).map(only_child).filter(|c| c.has_tag_name("ref") && matches!(attr(*c, "kind"), "interface" | "class") && elements(*c).next().is_none()).map(|c| attr(c, "name").to_string());
    let mut out: HashMap<String, Vec<Option<String>>> = HashMap::new();
    for d in elements(root).filter(|d| d.has_tag_name("interface") || d.has_tag_name("class")) {
        let cs: Vec<Option<String>> = named(d, "typeParameter").map(constraint_of).collect();
        if cs.iter().any(Option::is_some) {
            out.insert(attr(d, "name").to_string(), cs);
        }
    }
    // A type parameter in scope at a reference: its constraint.
    let param_constraint = |r: Node, name: &str| r.ancestors().find_map(|a| named(a, "typeParameter").find(|p| attr(*p, "name") == name)).and_then(constraint_of);
    let mut broken: HashSet<(String, usize)> = HashSet::new();
    for r in root.descendants().filter(|r| r.has_tag_name("ref")) {
        let Some(cs) = out.get(attr(r, "name")) else { continue };
        for (i, arg) in elements(r).enumerate() {
            let Some(Some(c)) = cs.get(i) else { continue };
            let ok = arg.has_tag_name("ref")
                && match attr(arg, "kind") {
                    "interface" | "class" => extends(attr(arg, "name"), c),
                    "typeParameter" => param_constraint(r, attr(arg, "name")).is_some_and(|pc| extends(&pc, c)),
                    _ => false,
                };
            if !ok {
                broken.insert((attr(r, "name").to_string(), i));
            }
        }
    }
    for (name, cs) in out.iter_mut() {
        for (i, c) in cs.iter_mut().enumerate() {
            if broken.contains(&(name.clone(), i)) {
                *c = None;
            }
        }
    }
    out.retain(|_, cs| cs.iter().any(Option::is_some));
    out
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

// The bindings as files, in the order F# compiles them: a prelude; the types
// of each module, in dependency order, with modules that refer to each other
// merged into one file (F# allows no cycles between files); and last the
// values: globals, functions as `Globals`, and the `<Map>Keys` modules.
pub fn module(xml: &str) -> Output {
    let doc = Document::parse(xml).expect("well-formed XML");
    let mut g = Gen {
        out: String::new(),
        skipped: BTreeMap::new(),
        declared: HashSet::new(),
        external: BTreeMap::new(),
        keyed: HashMap::new(),
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
        constrained_owner: HashMap::new(),
    };
    for d in elements(doc.root_element()).filter(|d| d.has_tag_name("interface") || d.has_tag_name("class")) {
        let name = attr(d, "name").to_string();
        if elements(d).any(|m| m.attribute("name") == Some("[Symbol.iterator]")) {
            g.iterable.insert(name.clone());
        }
        let bases = named(d, "extends").filter_map(|e| elements(e).next()).map(|r| attr(r, "name").to_string()).collect();
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
    g.decls(doc.root_element(), "", "");
    g.key_modules(doc.root_element());

    let mut prelude = String::from(
        "// Generated.\nnamespace Bindings\n\nopen Fable.Core\n\n\
         /// A key of the map 'Map whose value has type 'V: `el.addEventListener (HTMLElementEventMapKeys.click, fun e -> ...)`.\n\
         [<Erase>]\ntype TypedKey<'Map, 'V> = TypedKey of string\n\n\
         /// A JavaScript async iterator: `next ()` is a promise of the next value, or of None at the end.\n\
         [<AllowNullLiteral>]\ntype JsAsyncIterator<'T> =\n    \
         [<Emit(\"$0.next().then(r => r.done ? undefined : r.value)\")>] abstract next: unit -> JS.Promise<'T option>\n\n",
    );
    // Types from other files are opaque.
    for (name, arity) in &g.external {
        let params: Vec<String> = (1..=*arity).map(|i| format!("'T{i}")).collect();
        let params = if params.is_empty() { String::new() } else { format!("<{}>", params.join(", ")) };
        prelude += &format!("[<AllowNullLiteral>]\ntype {}{params} = interface end\n\n", ident(name));
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
    files.push(("Bindings.Values.fs".into(), format!("// Generated.\nnamespace Bindings\n\nopen Fable.Core\n\n{globals}[<AutoOpen>]\nmodule Values =\n    let private _values = ()\n\n{values}")));
    Output { files, skipped: g.skipped }
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
        let target = |r: Node| r.attribute("qualified").or(r.attribute("name")).and_then(|n| units.get(n).or_else(|| home.get(n))).filter(|t| t.as_str() != m).cloned();
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
        let refs = named(d, "extends").flat_map(|e| e.descendants()).filter(|r| r.has_tag_name("ref")).filter_map(|r| r.attribute("qualified").or(r.attribute("name"))).map(str::to_string);
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
        .map(|d| (attr(d, "name").to_string(), d.descendants().filter(|r| r.has_tag_name("ref")).filter_map(|r| r.attribute("qualified").or(r.attribute("name"))).map(str::to_string).collect()))
        .collect();
    for _ in 0..aliases.len().min(32) {
        let mut changed = false;
        for (name, refs) in &aliases {
            let own = out.get(name).copied().unwrap_or(0);
            let needed = refs.iter().filter_map(|r| out.get(r)).copied().max().unwrap_or(0).max(own);
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
            "ref" if attr(n, "kind") == "typeParameter" => format!("'{}", attr(n, "name")),
            "ref" => {
                let name = n.attribute("qualified").unwrap_or(attr(n, "name"));
                let args: Vec<String> = elements(n).map(|a| self.ty(a, owner)).collect();
                let name = if self.declared.contains(name) {
                    name.split('.').map(ident).collect::<Vec<_>>().join(".")
                } else {
                    match (name, args.len()) {
                        ("Array" | "ReadonlyArray", 1) => "ResizeArray".into(),
                        ("Promise" | "PromiseLike", 1) => "JS.Promise".into(),
                        // A type from another library, an alias too: an opaque type by its name.
                        _ if matches!(attr(n, "kind"), "class" | "interface" | "typeAlias") => {
                            let arity = self.external.entry(name.to_string()).or_default();
                            *arity = (*arity).max(args.len());
                            ident(name)
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
                match parts.len() {
                    1 => parts.pop().unwrap(),
                    2..=9 => format!("U{}<{}>", parts.len(), parts.join(", ")),
                    _ => {
                        self.skip("union of more than 9 types, bound as obj");
                        "obj".into()
                    }
                }
            }
            "array" => format!("ResizeArray<{}>", self.ty(only_child(n), owner)),
            // `M[K]` for a key parameter: the value type the key selects.
            "indexedAccess" => {
                let index = elements(n).nth(1).unwrap();
                match index.attribute("name").filter(|k| index.attribute("kind") == Some("typeParameter") && self.keyed.contains_key(*k)) {
                    Some(k) => format!("'{k}"),
                    None => unsupported(T, n),
                }
            }
            // Fable compiles tuples to arrays.
            "tuple" => {
                let parts: Vec<String> = named(n, "element").map(|e| self.ty(only_child(e), owner)).collect();
                if parts.len() < 2 || named(n, "element").any(|e| flag(e, "rest") || flag(e, "optional")) {
                    self.skip("irregular tuple, bound as an array");
                    return "ResizeArray<obj>".into();
                }
                format!("({})", parts.join(" * "))
            }
            "functionType" => {
                let mut parts: Vec<String> = named(n, "parameter").map(|p| self.ty(only_child(p), owner)).collect();
                if parts.is_empty() {
                    parts.push("unit".into());
                }
                let ret = named(n, "returns").next().map(|r| self.ty(only_child(r), owner)).unwrap_or("unit".into());
                parts.push(ret);
                format!("({})", parts.join(" -> "))
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
            "object" | "constructorType" => {
                self.skip("inline type with methods, bound as obj");
                "obj".into()
            }
            _ => unsupported(T, n),
        }
    }

    // Notes the type parameters of a signature that range over a map's keys.
    fn enter_keyed(&mut self, n: Node) {
        self.keyed.clear();
        for p in named(n, "typeParameter") {
            let map = elements(p).find(|c| c.has_tag_name("constraint")).map(only_child).filter(|k| k.has_tag_name("keyof")).map(only_child).filter(|r| r.has_tag_name("ref"));
            if let Some(map) = map {
                let name = map.attribute("qualified").unwrap_or(attr(map, "name")).to_string();
                self.maps.insert(name.clone());
                self.keyed.insert(attr(p, "name").to_string(), name.split('.').map(ident).collect::<Vec<_>>().join("."));
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
            let map_type = map.split('.').map(ident).collect::<Vec<_>>().join(".");
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
            let name = ident(attr(p, "name"));
            // F# wants optional arguments last and a parameter array last: not both.
            if flag(p, "rest") && has_optional {
                self.skip("rest parameter after optional ones");
                continue;
            }
            // A key of a map: typed by the value it selects.
            if t.has_tag_name("ref") && attr(t, "kind") == "typeParameter" && let Some(map) = self.keyed.get(attr(t, "name")) {
                out.push(format!("{}{name}: TypedKey<{map}, '{}>", if flag(p, "optional") { "?" } else { "" }, attr(t, "name")));
                continue;
            }
            if flag(p, "rest") {
                let element = if t.has_tag_name("array") { self.ty(only_child(t), owner) } else { "obj".into() };
                out.push(format!("[<System.ParamArray>] {name}: {element}[]"));
            } else {
                out.push(format!("{}{name}: {}", if flag(p, "optional") { "?" } else { "" }, self.ty(t, owner)));
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
                suffix.push(value.chars().map(|c| if c.is_alphanumeric() { c } else { '_' }).collect::<String>());
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
            let pair = if raw == "[Symbol.asyncIterator]" {
                // `x.asyncIterator ()`, whose `next ()` is a promise of the next value.
                let elem = named(m, "returns").next().and_then(|r| elements(r).next()).and_then(|it| elements(it).next());
                let t = elem.map_or("obj".to_string(), |e| self.ty(e, owner));
                let emit = "[<Emit(\"$0[Symbol.asyncIterator]()\")>]";
                Some((format!("{emit} abstract asyncIterator: unit -> JsAsyncIterator<{t}>"), format!("{emit} member this.asyncIterator(): JsAsyncIterator<{t}> = jsNative")))
            } else if raw.starts_with('[') {
                self.skip("symbol-named member");
                None
            } else {
                self.member(m, owner, setters.contains(raw))
            };
            let Some((line, ext)) = pair else { continue };
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
            let line = format!("{indent}    {line}\n");
            if !lines.contains(&line) {
                lines.push(line);
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
            let get = format!("[<Emit(\"$0{}\")>] get (): {t} = jsNative", js.replace('"', "\\\""));
            let set = format!("[<Emit(\"$0{} = $1\")>] set (v: {t}) = jsNative", js.replace('"', "\\\""));
            (format!("abstract {name}: {t}{}", if settable { " with get, set" } else { "" }), if settable { format!("member this.{name} with {get} and {set}") } else { format!("member this.{name} with {get}") })
        };
        let method = |g: &mut Gen, name: &str, emit: String, attr: &str| {
            let params = g.params(m, owner);
            let ret = g.returns(m, owner);
            let sig = if params.is_empty() { "unit".to_string() } else { params.join(" * ") };
            let abstract_line = format!("{attr}abstract {name}{tps}: {sig} -> {ret}");
            let ext = format!("[<Emit(\"{}\")>] member this.{name}{tps}({}): {ret} = jsNative", emit.replace('"', "\\\""), params.join(", "));
            (abstract_line, ext)
        };
        Some(match m.tag_name().name() {
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
                    method(self, &fixed_name, emit, &attr)
                }
                None => method(self, &name, format!("$0{js}($1...)"), ""),
            },
            "call" => method(self, "Invoke", "$0($1...)".into(), "[<Emit(\"$0($1...)\")>] "),
            "construct" | "constructor" => method(self, "Create", "new $0($1...)".into(), "[<EmitConstructor>] "),
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
            _ => unsupported(T, m),
        })
    }

    fn interface(&mut self, d: Node, prefix: &str, indent: &str) {
        let name = ident(attr(d, "name"));
        let path = js_path(prefix, attr(d, "name"));
        let params = self.type_params(d);
        let owner = format!("{name}{params}");
        // The declaration, and its extensions, state the constraints it keeps.
        let header = match self.constrained.get(attr(d, "name")) {
            Some(cs) => {
                let ps: Vec<String> = named(d, "typeParameter").map(|p| format!("'{}", attr(p, "name"))).collect();
                let when: Vec<String> = ps.iter().zip(cs).filter_map(|(p, c)| c.as_ref().map(|c| format!("{p} :> {c}"))).collect();
                format!("{name}<{} when {} >", ps.join(", "), when.join(" and "))
            }
            None => owner.clone(),
        };
        let is_member = |m: &Node| matches!(m.tag_name().name(), "property" | "method" | "call" | "construct" | "index" | "get" | "set");
        let instance: Vec<Node> = elements(d).filter(|m| is_member(m) && !flag(*m, "static")).collect();
        let statics: Vec<Node> = elements(d).filter(|m| m.has_tag_name("constructor") || is_member(m) && flag(*m, "static")).collect();
        self.out += &format!("{indent}[<AllowNullLiteral>]\n{indent}type {header} =\n");
        let mut body = Vec::new();
        // A subclass of Array is a seq with an indexer and a length: an F#
        // interface cannot inherit ResizeArray, which is a class.
        let mut is_array = false;
        for e in named(d, "extends") {
            let base = self.ty(only_child(e), &owner);
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
        let mut todo: Vec<String> = self.bases.get(attr(d, "name")).cloned().unwrap_or_default();
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
        let extend = if prefix.is_empty() { owner.clone() } else { format!("{}.{owner}", prefix.split('.').map(ident).collect::<Vec<_>>().join(".")) };
        if header != owner {
            let qualified = if prefix.is_empty() { header.clone() } else { format!("{}.{header}", prefix.split('.').map(ident).collect::<Vec<_>>().join(".")) };
            self.constrained_owner.insert(extend.clone(), qualified);
        }
        body.extend(self.members(&instance, &owner, &extend, indent));
        if body.is_empty() {
            body.push(format!("{indent}    interface end\n"));
        }
        self.out += &body.concat();
        self.out += "\n";
        if d.has_tag_name("class") {
            self.static_side(&name, &path, &statics, Some(&owner), indent);
        }
    }

    // A value with constructors and static members: a type for its shape and a global of that type.
    fn static_side(&mut self, name: &str, path: &str, members: &[Node], instance: Option<&str>, indent: &str) {
        let static_name = ident(&format!("{}Static", name.trim_matches('`')));
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
                if !lines.contains(&line) {
                    lines.push(line);
                }
            } else {
                lines.extend(self.members(&[m], "obj", &static_name, indent));
            }
        }
        if lines.is_empty() {
            lines.push(format!("{indent}    interface end\n"));
        }
        self.out += &lines.concat();
        self.out += "\n";
        let value = format!("{indent}[<Emit({path:?})>]\n{indent}let {name}: {static_name} = jsNative\n\n");
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
            let raw = d.attribute("name").unwrap_or_default();
            let name = ident(raw);
            let path = js_path(prefix, d.attribute("jsName").unwrap_or(raw));
            // A top-level declaration's types go in its module's file; an alias where its parts are.
            if indent.is_empty()
                && let Some(module) = d.attribute("module")
            {
                let unit = self.units.get(raw).cloned().unwrap_or_else(|| module.to_string());
                self.current_file = if d.has_tag_name("typeAlias") { self.file_of[raw] } else { self.file_of[&format!("module:{unit}")] };
            }
            let start = self.out.len();
            self.decl(d, raw, &name, &path, prefix, indent, &mut functions);
            if indent.is_empty()
                && let Some(module) = d.attribute("module")
            {
                let text = self.out.split_off(start);
                if d.has_tag_name("typeAlias") {
                    let file = self.file_of[raw];
                    *self.alias_types.entry(file).or_default() += &text;
                } else {
                    let unit = self.units.get(raw).cloned().unwrap_or_else(|| module.to_string());
                    *self.types.entry(unit).or_default() += &text;
                }
            }
        }
        if !functions.is_empty() {
            self.out += &format!("{indent}[<Erase>]\n{indent}type Globals =\n{}\n", functions.concat());
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
                    self.decls(d, &js_path(prefix, raw), &format!("{indent}    "));
                    if self.out.len() == before {
                        self.out += &format!("{indent}    let private empty = ()\n\n");
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
                "interface" | "class" => self.interface(d, prefix, indent),
                "function" => {
                    let params = self.params(d, "");
                    let ret = self.returns(d, "");
                    let (name, args) = match self.fixed(d, 0) {
                        Some((suffix, args)) => (ident(&format!("{raw}_{suffix}")), args.replace('"', "\\\"")),
                        None => (name, "$0...".to_string()),
                    };
                    let tps = self.constrained_type_params(d);
                    let line = format!("{indent}    [<Emit(\"{path}({args})\")>] static member {name}{tps} ({}): {ret} = jsNative\n", params.join(", "));
                    if !functions.contains(&line) {
                        functions.push(line);
                    }
                }
                "typeAlias" if only_child_type(d).has_tag_name("union") && named(d, "typeParameter").next().is_none() => {
                    let body = only_child_type(d);
                    if !self.named_union(&name, body, indent) {
                        self.skip("union whose members cannot be told apart, bound as obj");
                        self.out += &format!("{indent}type {name} = obj\n\n");
                    }
                }
                "typeAlias" => {
                    let body = only_child_type(d);
                    // F# abbreviations cannot refer to themselves.
                    if body.descendants().any(|r| r.has_tag_name("ref") && r.attribute("name") == Some(raw)) {
                        self.skip("recursive type alias, bound as obj");
                        self.out += &format!("{indent}type {name} = obj\n\n");
                        return;
                    }
                    let t = self.ty(body, "");
                    self.out += &format!("{indent}type {name}{} = {t}\n\n", self.type_params(d));
                }
                "var" => {
                    let body = only_child(d);
                    if body.has_tag_name("object") {
                        let members: Vec<Node> = elements(body).collect();
                        self.static_side(&name, &path, &members, None, indent);
                    } else {
                        let t = self.ty(body, "");
                        let value = format!("{indent}[<Emit({path:?})>]\n{indent}let {name}: {t} = jsNative\n\n");
                        if indent.is_empty() {
                            self.values += &value;
                        } else {
                            self.out += &value;
                        }
                    }
                }
                _ => unsupported(T, d),
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
        if let Some(d) = union.attribute("discriminant") {
            self.out += &format!("{indent}[<TypeScriptTaggedUnion({d:?})>]\n{indent}type {name} =\n");
            for (case, payload, test) in &cases {
                let value = test.as_deref().and_then(|t| t.strip_prefix("tag:")).unwrap_or_default();
                self.out += &format!("{indent}    | [<CompiledName({value:?})>] {case} of {payload}\n");
            }
        } else {
            self.out += &format!("{indent}[<Erase; RequireQualifiedAccess>]\n{indent}type {name} =\n");
            for (case, payload, test) in &cases {
                let native = matches!(payload.as_str(), "string" | "float" | "bool");
                let attr = match test.as_deref().and_then(case_test) {
                    Some(js) if !native => format!("[<CaseTest(\"{}\")>] ", js.replace('"', "\\\"")),
                    _ => String::new(),
                };
                self.out += &format!("{indent}    | {attr}{case} of {payload}\n");
            }
        }
        self.out += "\n";
        true
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

// The JavaScript test of a union member, from its `test` attribute; `$0` is the value.
fn case_test(test: &str) -> Option<String> {
    let (kind, arg) = test.split_once(':').unwrap_or((test, ""));
    Some(match kind {
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
        if let Some(b) = find_decl(root, base.attribute("qualified").unwrap_or(attr(base, "name"))) {
            out.extend(map_properties(root, b));
        }
    }
    out
}

fn js_path(prefix: &str, name: &str) -> String {
    if prefix.is_empty() { name.to_string() } else { format!("{prefix}.{name}") }
}

// The type of an alias, after any type parameters.
fn only_child_type<'a>(d: Node<'a, 'a>) -> Node<'a, 'a> {
    elements(d).find(|c| !c.has_tag_name("typeParameter")).unwrap_or_else(|| panic!("alias {} has no type", attr(d, "name")))
}
