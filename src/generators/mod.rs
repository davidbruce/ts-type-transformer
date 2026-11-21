// Generators: lowered XML to bindings. They are deliberately thin: each
// prints what its target config produced and rejects anything else, so a
// construct the config should have lowered is an error, not a guess.

pub mod backend;
pub mod c;
pub mod case;
pub mod cpp;
pub mod fsharp;
pub mod gleam;
pub mod js;
pub mod model;
pub mod names;
pub mod pkg_meta;
pub mod rust;
pub mod wasm;
pub mod zig;

use std::path::{Component, Path, PathBuf};

use roxmltree::Node;

/// Converts a package or lib name to a snake_case identifier usable as a crate,
/// Gleam package, Zig module, C/C++ umbrella header/namespace, or WASM import module.
pub fn snake_package_name(raw: &str) -> String {
    let raw = raw.trim().trim_start_matches('@');
    let mut out = String::new();
    for c in raw.chars() {
        let c = if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '_' };
        if c != '_' || !out.ends_with('_') {
            out.push(c);
        }
    }
    let mut out = out.trim_matches('_').to_string();
    if out.is_empty() {
        return "bindings".to_string();
    }
    if !out.as_bytes()[0].is_ascii_alphabetic() {
        out.insert_str(0, "pkg_");
    }
    out
}

fn abs_normalized(p: &Path) -> PathBuf {
    let full = if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::env::current_dir().map(|cwd| cwd.join(p)).unwrap_or_else(|_| p.to_path_buf())
    };
    let mut tail: Vec<std::ffi::OsString> = Vec::new();
    let mut cur = full.as_path();
    loop {
        if let Ok(canon) = std::fs::canonicalize(cur) {
            let mut out = canon;
            for part in tail.into_iter().rev() {
                out.push(part);
            }
            return out;
        }
        let Some(name) = cur.file_name() else { break };
        tail.push(name.to_os_string());
        let Some(parent) = cur.parent() else { break };
        cur = parent;
    }
    let mut norm = PathBuf::new();
    for c in full.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                norm.pop();
            }
            other => norm.push(other.as_os_str()),
        }
    }
    norm
}

/// Relative path with forward slashes from `from_dir` to `target`.
pub fn relative_path(from_dir: &Path, target: &Path) -> String {
    let abs_from = abs_normalized(from_dir);
    let abs_target = abs_normalized(target);
    let a: Vec<Component> = abs_from.components().collect();
    let b: Vec<Component> = abs_target.components().collect();
    let mut common = 0;
    while common < a.len() && common < b.len() && a[common] == b[common] {
        common += 1;
    }
    let mut parts: Vec<String> = Vec::new();
    for _ in common..a.len() {
        parts.push("..".to_string());
    }
    for comp in &b[common..] {
        parts.push(comp.as_os_str().to_string_lossy().into_owned());
    }
    if parts.is_empty() { ".".to_string() } else { parts.join("/") }
}

pub fn elements<'a>(n: Node<'a, 'a>) -> impl Iterator<Item = Node<'a, 'a>> {
    n.children().filter(|c| c.is_element())
}

pub fn named<'a>(n: Node<'a, 'a>, tag: &'static str) -> impl Iterator<Item = Node<'a, 'a>> {
    elements(n).filter(move |c| c.has_tag_name(tag))
}

pub fn attr<'a>(n: Node<'a, 'a>, name: &str) -> &'a str {
    n.attribute(name).unwrap_or_else(|| panic!("<{}> has no `{name}`", n.tag_name().name()))
}

// A fixed argument's value as part of a name, as the lowering names it:
// `2d`, `image_png`, and a URL by its last part, `svg`. No runs of `_`.
pub fn value_name(value: &str) -> String {
    let value = value.trim_matches('"');
    let value = if value.contains("://") { value.trim_end_matches('/').rsplit('/').next().unwrap_or(value) } else { value };
    let mut out = String::new();
    for c in value.chars() {
        let c = if c.is_ascii_alphanumeric() { c } else { '_' };
        if c != '_' || !out.ends_with('_') {
            out.push(c);
        }
    }
    let out = out.trim_matches('_').to_string();
    if out.is_empty() { "empty".into() } else { out }
}

pub fn flag(n: Node, name: &str) -> bool {
    n.attribute(name) == Some("true")
}

// The single type element inside a property, parameter, returns, ...
pub fn only_child<'a>(n: Node<'a, 'a>) -> Node<'a, 'a> {
    elements(n).next().unwrap_or_else(|| panic!("<{}> is empty", n.tag_name().name()))
}

// Where an element sits, for error messages.
pub fn path(n: Node) -> String {
    let mut parts: Vec<String> = n
        .ancestors()
        .filter(|a| a.is_element())
        .map(|a| match a.attribute("name") {
            Some(name) => format!("{}[{name}]", a.tag_name().name()),
            None => a.tag_name().name().to_string(),
        })
        .collect();
    parts.reverse();
    parts.join("/")
}

// What a generator cannot express: each kind once, with how often it occurs
// and where first, and the setting that lowers it.
#[derive(Default)]
pub struct Problems(std::collections::BTreeMap<String, (usize, String)>);

impl Problems {
    pub fn add(&mut self, what: String, at: String) {
        self.0.entry(what).or_insert((0, at)).0 += 1;
    }

    // An element of the XML the generator has no form for.
    pub fn element(&mut self, target: &str, n: Node) {
        // A reference to a type parameter is the type parameter's problem.
        let tag = if n.has_tag_name("ref") && n.attribute("kind") == Some("typeParameter") { "typeParameter" } else { n.tag_name().name() };
        self.add(format!("{target} generator cannot express <{tag}>: {}", hint(tag)), path(n));
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn into_messages(self) -> Vec<String> {
        self.0.into_iter().map(|(what, (n, at))| if n == 1 { format!("{what} (at {at})") } else { format!("{what} ({n} places, first at {at})") }).collect()
    }

    pub fn result<T>(self, value: T) -> Result<T, Vec<String>> {
        if self.is_empty() { Ok(value) } else { Err(self.into_messages()) }
    }
}

// The setting that lowers an element.
fn hint(tag: &str) -> &'static str {
    match tag {
        "typeParameter" => "set generics = \"erase\"",
        "union" => "set unions = \"name\" or \"erase\"",
        "literal" => "set literals = \"enum\"",
        "object" | "functionType" | "constructorType" => "set anonymous = \"hoist\"",
        "intersection" => "set intersections = \"merge\"",
        "keyof" | "indexedAccess" | "conditional" | "mapped" | "template" | "typeQuery" | "typeOperator" | "predicate" | "infer" => "set type_ops = \"evaluate\"",
        "extends" => "set inheritance = \"flatten\"",
        "var" => "set classes = \"pair\", or anonymous = \"hoist\"",
        _ => "the target config must lower it",
    }
}

// An overload's marks, as the lowering wrote them: what tells it apart
// where a target loses it (`async`, `Dictionary`).
pub fn marks(n: roxmltree::Node) -> Vec<String> {
    n.attribute("marks").map(|m| m.split(' ').map(str::to_string).collect()).unwrap_or_default()
}

// Each overload's marks the others do not all have: what to name it by.
pub fn own_marks(marks: &[Vec<String>]) -> Vec<Vec<String>> {
    marks.iter().map(|m| m.iter().filter(|w| marks.iter().any(|o| !o.contains(w))).cloned().collect()).collect()
}
