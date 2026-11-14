// Lowering configuration. A target is described by a TOML file with one
// strategy per TypeScript construct; nothing here knows about any language.

use std::collections::HashMap;

use toml::{Table, Value};

use crate::lower::Options;

// (key, lowering strategy, what "keep" leaves in, what lowering does).
// Every construct also accepts "keep", the default.
pub const CONSTRUCTS: &[(&str, &str, &str, &str)] = &[
    ("classes", "pair", "`var X: { prototype: X; new() }` beside `interface X`", "one <class> with constructors and statics"),
    ("type_ops", "evaluate", "keyof, T[K], conditional, mapped, template and typeof types", "the type they evaluate to"),
    ("intersections", "merge", "A & B", "one object type with the members of both"),
    ("inheritance", "flatten", "<extends>", "inherited members copied into each type"),
    ("generics", "erase", "type parameters", "their constraint, or any"),
    ("nullable", "wrap", "T | null | undefined", "<nullable> around T"),
    ("literals", "enum", "literal types", "enums for string unions, const= for constants"),
    ("anonymous", "hoist", "inline object and function types", "named declarations"),
    ("optional_params", "overloads", "optional parameters", "one overload per arity"),
    ("union_params", "overloads", "union-typed parameters", "one overload per union member"),
    ("unions", "erase|name", "other unions", "erase: their common supertype, or any. name: a named declaration"),
    ("overloads", "rename", "same-named members", "unique names, with jsName"),
    ("redeclarations", "drop", "a method a type declares again as its ancestor declares it, but for the type of `this`", "the ancestor's alone"),
    ("keyed_params", "expand|map", "f<K extends keyof M>(key: K, value: M[K])", "expand: one overload per key, the key a fixed argument. map: kept generic, with keyof M and M[K] as written"),
];

const OVERLOAD_NAMES: &[&str] = &["with_params", "numbered"];

// Which module each top-level declaration goes in: the first area whose
// pattern matches its name, else `default`. A namespace is a module of its own.
#[derive(Clone)]
pub struct Modules {
    pub default: String,
    pub areas: Vec<(String, Vec<String>)>,
}

impl Modules {
    pub fn module_of(&self, name: &str) -> &str {
        self.areas.iter().find(|(_, patterns)| patterns.iter().any(|p| glob(p, name))).map(|(n, _)| n.as_str()).unwrap_or(&self.default)
    }
}

// `SVG*`, `*Event`, `*Audio*`, `Node`.
fn glob(pattern: &str, name: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 1 {
        return pattern == name;
    }
    let (first, last) = (parts[0], parts[parts.len() - 1]);
    if !name.starts_with(first) || !name[first.len()..].ends_with(last) || name.len() < first.len() + last.len() {
        return false;
    }
    let mut rest = &name[first.len()..name.len() - last.len()];
    for middle in &parts[1..parts.len() - 1] {
        match rest.find(middle) {
            Some(i) => rest = &rest[i + middle.len()..],
            None => return false,
        }
    }
    true
}

// Module names become file and folder names: they must differ even where
// letter case does not count (macOS, Windows), and must not be names Windows
// reserves.
fn check_module_names(m: &Modules) -> Result<(), String> {
    let mut seen: HashMap<String, &str> = HashMap::new();
    for name in std::iter::once(m.default.as_str()).chain(m.areas.iter().map(|a| a.0.as_str())) {
        if reserved(name) {
            return Err(format!("module `{name}`: a name Windows reserves for devices; choose another"));
        }
        if let Some(other) = seen.insert(name.to_lowercase(), name)
            && other != name
        {
            return Err(format!("modules `{other}` and `{name}` differ only in letter case: their files would clash"));
        }
    }
    Ok(())
}

// `CON`, `NUL`, `COM1`, ...: not usable as file names on Windows, with any extension.
pub fn reserved(name: &str) -> bool {
    let n = name.to_ascii_uppercase();
    matches!(n.as_str(), "CON" | "PRN" | "AUX" | "NUL") || (n.len() == 4 && (n.starts_with("COM") || n.starts_with("LPT")) && n.as_bytes()[3].is_ascii_digit())
}

// Later tables override earlier ones, key by key.
fn merge(into: &mut Table, from: Table) {
    for (k, v) in from {
        match (into.get_mut(&k), v) {
            (Some(Value::Table(a)), Value::Table(b)) => merge(a, b),
            (_, v) => {
                into.insert(k, v);
            }
        }
    }
}

pub fn load(paths: &[String], overrides: &[String]) -> Result<(Options, Modules), String> {
    let mut table = Table::new();
    for p in paths {
        let t = std::fs::read_to_string(p).map_err(|e| format!("{p}: {e}"))?.parse::<Table>().map_err(|e| format!("{p}: {e}"))?;
        merge(&mut table, t);
    }
    let mut modules = Modules { default: "main".into(), areas: Vec::new() };
    // `--set generics=erase`, `--set limits.union_param_overloads=4`.
    for o in overrides {
        let (key, value) = o.split_once('=').ok_or_else(|| format!("--set {o}: expected key=value"))?;
        let (section, key) = key.split_once('.').unwrap_or(("constructs", key));
        let value = value.parse::<i64>().map(Value::Integer).unwrap_or_else(|_| Value::String(value.to_string()));
        let entry = table.entry(section).or_insert_with(|| Value::Table(Table::new()));
        entry.as_table_mut().ok_or_else(|| format!("[{section}] is not a table"))?.insert(key.to_string(), value);
    }

    let mut o = Options::default();
    for (section, value) in &table {
        let entries = value.as_table().ok_or_else(|| format!("[{section}] is not a table"))?;
        for (key, value) in entries {
            let at = format!("{section}.{key}");
            match (section.as_str(), key.as_str()) {
                ("constructs", _) => {
                    let known = || CONSTRUCTS.iter().map(|c| c.0).collect::<Vec<_>>().join(", ");
                    let (_, lowered, ..) = CONSTRUCTS.iter().find(|c| c.0 == key).ok_or_else(|| format!("unknown construct `{key}`; known: {}", known()))?;
                    let mut allowed = vec!["keep"];
                    allowed.extend(lowered.split('|'));
                    let chosen = one_of(&at, value, &allowed)?;
                    // The first strategy is the switch; others are alternatives to it.
                    *o.slot(key).unwrap() = chosen == allowed[1];
                    if key == "unions" {
                        o.name_unions = chosen == "name";
                    }
                    if key == "keyed_params" {
                        o.map_keyed = chosen == "map";
                    }
                }
                ("limits", "union_param_overloads") => o.union_param_limit = count(&at, value)?,
                ("limits", "intersection_members") => o.intersection_limit = count(&at, value)?,
                ("limits", "keyed_overloads") => o.keyed_limit = count(&at, value)?,
                ("naming", "separator") => o.separator = Box::leak(string(&at, value)?.to_string().into_boxed_str()),
                ("naming", "overloads") => o.numbered_overloads = one_of(&at, value, OVERLOAD_NAMES)? == "numbered",
                ("modules", "default") => modules.default = string(&at, value)?.to_string(),
                // [[modules.area]] name = "svg", match = ["SVG*"]
                ("modules", "area") => {
                    for area in value.as_array().ok_or_else(|| format!("{at} must be an array of tables"))? {
                        let area = area.as_table().ok_or_else(|| format!("{at} must be an array of tables"))?;
                        let name = string(&at, area.get("name").ok_or_else(|| format!("{at}: an area needs a name"))?)?.to_string();
                        let patterns = area
                            .get("match")
                            .and_then(|m| m.as_array())
                            .ok_or_else(|| format!("{at} {name}: `match` must be a list of patterns"))?
                            .iter()
                            .map(|p| string(&at, p).map(str::to_string))
                            .collect::<Result<Vec<_>, _>>()?;
                        if let Some(k) = area.keys().find(|k| !matches!(k.as_str(), "name" | "match")) {
                            return Err(format!("{at} {name}: unknown setting `{k}`"));
                        }
                        modules.areas.push((name, patterns));
                    }
                }
                _ => return Err(format!("unknown setting `{at}`")),
            }
        }
    }
    // Inherited members come from the checker, which has already evaluated them.
    if o.inheritance && !o.typeops {
        return Err("inheritance = \"flatten\" requires type_ops = \"evaluate\"".into());
    }
    check_module_names(&modules)?;
    Ok((o, modules))
}

fn string<'a>(at: &str, v: &'a Value) -> Result<&'a str, String> {
    v.as_str().ok_or_else(|| format!("{at} must be a string"))
}

fn one_of<'a>(at: &str, v: &'a Value, allowed: &[&str]) -> Result<&'a str, String> {
    let s = string(at, v)?;
    if allowed.contains(&s) { Ok(s) } else { Err(format!("{at} = \"{s}\": expected one of \"{}\"", allowed.join("\", \""))) }
}

fn count(at: &str, v: &Value) -> Result<usize, String> {
    v.as_integer().and_then(|n| usize::try_from(n).ok()).ok_or_else(|| format!("{at} must be a non-negative integer"))
}

// The strategy in effect for every construct.
pub fn describe(mut o: Options) -> Vec<(&'static str, &'static str)> {
    let name_unions = o.name_unions;
    CONSTRUCTS
        .iter()
        .map(|(key, lowered, ..)| {
            let first = lowered.split('|').next().unwrap();
            let chosen = if *key == "unions" && name_unions {
                "name"
            } else if *key == "keyed_params" && o.map_keyed {
                "map"
            } else if *o.slot(key).unwrap() {
                first
            } else {
                "keep"
            };
            (*key, chosen)
        })
        .collect()
}

// A config with every setting at its default, documented.
pub fn template() -> String {
    let d = Options::default();
    let mut out = String::from("# Strategy per TypeScript construct. \"keep\" leaves it in the output as written.\n[constructs]\n");
    for (key, lowered, kept, becomes) in CONSTRUCTS {
        out += &format!("# {kept} -> {becomes}\n{key} = \"keep\" # or \"{}\"\n", lowered.replace('|', "\" or \""));
    }
    out += &format!(
        "\n[limits]\n# Most overloads one signature may expand into through union parameters.\nunion_param_overloads = {}\n\
         # Largest merged intersection that is written out inline.\nintersection_members = {}\n\
         # Most keys a keyed signature is expanded over.\nkeyed_overloads = {}\n\
         \n[naming]\n# Joins the parts of generated names.\nseparator = \"{}\"\n\
         # \"with_params\": f_with_x. \"numbered\": f_2.\noverloads = \"with_params\"\n",
        d.union_param_limit, d.intersection_limit, d.keyed_limit, d.separator
    );
    out
}
