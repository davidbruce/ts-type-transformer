// Generators: lowered XML to bindings. They are deliberately thin: each
// prints what its target config produced and rejects anything else, so a
// construct the config should have lowered is an error, not a guess.

pub mod c;
pub mod cpp;
pub mod fsharp;
pub mod rust;
pub mod wasm;
pub mod zig;

use roxmltree::Node;

pub fn elements<'a>(n: Node<'a, 'a>) -> impl Iterator<Item = Node<'a, 'a>> {
    n.children().filter(|c| c.is_element())
}

pub fn named<'a>(n: Node<'a, 'a>, tag: &'static str) -> impl Iterator<Item = Node<'a, 'a>> {
    elements(n).filter(move |c| c.has_tag_name(tag))
}

pub fn attr<'a>(n: Node<'a, 'a>, name: &str) -> &'a str {
    n.attribute(name).unwrap_or_else(|| panic!("<{}> has no `{name}`", n.tag_name().name()))
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
        let tag = n.tag_name().name();
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
