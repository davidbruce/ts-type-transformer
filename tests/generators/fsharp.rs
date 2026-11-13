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
    // Top-level declarations' types: name, module and text. Top-level values go in a file of their own.
    types: Vec<(String, String, String)>,
    values: String,
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
        types: Vec::new(),
        values: String::new(),
    };
    g.index(doc.root_element(), "");
    g.decls(doc.root_element(), "", "");
    g.key_modules(doc.root_element());

    let mut prelude = String::from(
        "// Generated.\nnamespace Bindings\n\nopen Fable.Core\n\n\
         /// A key of the map 'Map whose value has type 'V: `el.addEventListener (HTMLElementEventMapKeys.click, fun e -> ...)`.\n\
         [<Erase>]\ntype TypedKey<'Map, 'V> = TypedKey of string\n\n",
    );
    // Types from other files are opaque.
    for (name, arity) in &g.external {
        let params: Vec<String> = (1..=*arity).map(|i| format!("'T{i}")).collect();
        let params = if params.is_empty() { String::new() } else { format!("<{}>", params.join(", ")) };
        prelude += &format!("[<AllowNullLiteral>]\ntype {}{params} = interface end\n\n", ident(name));
    }
    let mut files = vec![("Bindings.Prelude.fs".to_string(), prelude)];
    for (i, (keys, members)) in file_order(&g.types).into_iter().enumerate() {
        let name = if keys.len() == 1 { keys[0].clone() } else { format!("{}_and_{}_more", keys[0], keys.len() - 1) };
        let body: String = members.iter().map(|&j| g.types[j].2.as_str()).collect();
        files.push((format!("Bindings.{:02}.{name}.fs", i + 1), format!("// Generated. Parts: {}.\nnamespace rec Bindings\n\nopen Fable.Core\n\n{body}", keys.join(", "))));
    }
    let values: String = g.values.lines().map(|l| if l.is_empty() { "\n".to_string() } else { format!("    {l}\n") }).collect();
    let globals = std::mem::take(&mut g.out);
    files.push(("Bindings.Values.fs".into(), format!("// Generated.\nnamespace Bindings\n\nopen Fable.Core\n\n{globals}[<AutoOpen>]\nmodule Values =\n    let private _values = ()\n\n{values}")));
    Output { files, skipped: g.skipped }
}

// The files of types, in the order F# must see them, each with its parts and
// the declarations in it. F# allows no cycles between files, and the types of
// a library like the DOM form one large cycle (`Window`, `Document`, `Node`,
// ...). That cycle gets a file of its own; every other declaration stays in
// its module, split into what the cycle uses (`<module>.base`, before it) and
// what uses the cycle (after it). Parts that still refer to each other are
// merged into one file.
fn file_order(types: &[(String, String, String)]) -> Vec<(Vec<String>, Vec<usize>)> {
    // A name can have several declarations: `type X` and `var X`.
    let mut index: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, t) in types.iter().enumerate() {
        index.entry(t.0.as_str()).or_default().push(i);
    }
    // Declarations a declaration's text mentions.
    let edges: Vec<BTreeSet<usize>> = types
        .iter()
        .enumerate()
        .map(|(i, t)| words(&t.2).into_iter().filter_map(|w| index.get(w)).flatten().copied().filter(|&j| j != i).collect())
        .collect();
    let sccs = tarjan(types.len(), &edges);
    let big: HashSet<usize> = sccs.iter().filter(|c| c.len() > 1).max_by_key(|c| c.len()).map(|c| c.iter().copied().collect()).unwrap_or_default();
    let mut before: HashSet<usize> = HashSet::new();
    let mut todo: Vec<usize> = big.iter().copied().collect();
    while let Some(v) = todo.pop() {
        for &w in &edges[v] {
            if !big.contains(&w) && before.insert(w) {
                todo.push(w);
            }
        }
    }
    let part = |i: usize| -> String {
        if big.contains(&i) {
            "cycle".into()
        } else if before.contains(&i) {
            format!("{}.base", types[i].1)
        } else {
            types[i].1.clone()
        }
    };
    let parts: Vec<String> = {
        let mut p: Vec<String> = (0..types.len()).map(part).collect();
        p.sort();
        p.dedup();
        p
    };
    let pindex: HashMap<String, usize> = parts.iter().enumerate().map(|(i, p)| (p.clone(), i)).collect();
    let mut pedges = vec![BTreeSet::new(); parts.len()];
    for (i, es) in edges.iter().enumerate() {
        for &j in es {
            let (a, b) = (pindex[&part(i)], pindex[&part(j)]);
            if a != b {
                pedges[a].insert(b);
            }
        }
    }
    tarjan(parts.len(), &pedges)
        .into_iter()
        .map(|group| {
            let mut keys: Vec<String> = group.iter().map(|&p| parts[p].clone()).collect();
            keys.sort();
            let members = (0..types.len()).filter(|&i| keys.contains(&part(i))).collect();
            (keys, members)
        })
        .collect()
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
                        _ if matches!(attr(n, "kind"), "class" | "interface") => {
                            let arity = self.external.entry(name.to_string()).or_default();
                            *arity = (*arity).max(args.len());
                            ident(name)
                        }
                        // An alias or value from another file: nothing to refer to.
                        _ => {
                            self.skip("reference to an undeclared alias, bound as obj");
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

    // The members of one side of a type, as `abstract` lines.
    fn members(&mut self, members: &[Node], owner: &str, indent: &str) -> Vec<String> {
        let mut lines: Vec<String> = Vec::new();
        let setters: HashSet<&str> = members.iter().filter(|m| m.has_tag_name("set")).map(|m| attr(*m, "name")).collect();
        for &m in members {
            let raw = m.attribute("name").unwrap_or_default();
            if raw.starts_with('[') {
                self.skip("symbol-named member");
                continue;
            }
            let name = ident(raw);
            let line = match m.tag_name().name() {
                "property" => {
                    let mut t = self.ty(only_child(m), owner);
                    if flag(m, "optional") {
                        t += " option";
                    }
                    format!("abstract {name}: {t}{}", if flag(m, "readonly") { "" } else { " with get, set" })
                }
                "get" => format!("abstract {name}: {}{}", self.returns(m, owner), if setters.contains(raw) { " with get, set" } else { "" }),
                "set" => continue,
                "method" => match self.fixed(m, 1) {
                    Some((suffix, args)) => {
                        let call = format!("$0{}({args})", js_access(raw)).replace('"', "\\\"");
                        format!("[<Emit(\"{call}\")>] abstract {}{}: {}", ident(&format!("{raw}_{suffix}")), self.type_params(m), self.signature(m, owner, None))
                    }
                    None => format!("abstract {name}{}: {}", self.type_params(m), self.signature(m, owner, None)),
                },
                "call" => format!("[<Emit(\"$0($1...)\")>] abstract Invoke{}: {}", self.type_params(m), self.signature(m, owner, None)),
                "construct" | "constructor" => format!("[<EmitConstructor>] abstract Create{}: {}", self.type_params(m), self.signature(m, owner, None)),
                "index" => {
                    let key = named(m, "parameter").next().map(|p| self.ty(only_child(p), owner)).unwrap_or("string".into());
                    let value = self.returns(m, owner);
                    format!("[<EmitIndexer>] abstract Item: key: {key} -> {value}{}", if flag(m, "readonly") { "" } else { " with get, set" })
                }
                _ => unsupported(T, m),
            };
            // Distinct TypeScript overloads can map to the same F# signature.
            let line = format!("{indent}    {line}\n");
            if !lines.contains(&line) {
                lines.push(line);
            }
        }
        lines
    }

    fn interface(&mut self, d: Node, prefix: &str, indent: &str) {
        let name = ident(attr(d, "name"));
        let path = js_path(prefix, attr(d, "name"));
        let params = self.type_params(d);
        let owner = format!("{name}{params}");
        let is_member = |m: &Node| matches!(m.tag_name().name(), "property" | "method" | "call" | "construct" | "index" | "get" | "set");
        let instance: Vec<Node> = elements(d).filter(|m| is_member(m) && !flag(*m, "static")).collect();
        let statics: Vec<Node> = elements(d).filter(|m| m.has_tag_name("constructor") || is_member(m) && flag(*m, "static")).collect();
        self.out += &format!("{indent}[<AllowNullLiteral>]\n{indent}type {owner} =\n");
        let mut body = Vec::new();
        for e in named(d, "extends") {
            let base = self.ty(only_child(e), &owner);
            body.push(format!("{indent}    inherit {base}\n"));
        }
        body.extend(self.members(&instance, &owner, indent));
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
            if m.has_tag_name("constructor") || m.has_tag_name("construct") {
                let ret = instance.map(str::to_string).unwrap_or_else(|| self.returns(m, ""));
                let line = format!("{indent}    [<EmitConstructor>] abstract Create{}: {}\n", self.type_params(m), self.signature(m, "", Some(&ret)));
                if !lines.contains(&line) {
                    lines.push(line);
                }
            } else {
                lines.extend(self.members(&[m], "obj", indent));
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
            // A top-level declaration's types go in its module's file.
            let start = self.out.len();
            self.decl(d, raw, &name, &path, prefix, indent, &mut functions);
            if indent.is_empty()
                && let Some(module) = d.attribute("module")
            {
                let text = self.out.split_off(start);
                // A value (`declare var X`) has no type text of its own.
                if !text.trim().is_empty() {
                    self.types.push((raw.to_string(), module.to_string(), text));
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
                    let line = format!("{indent}    [<Emit(\"{path}({args})\")>] static member {name}{} ({}): {ret} = jsNative\n", self.type_params(d), params.join(", "));
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
