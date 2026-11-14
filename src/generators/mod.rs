// Generators: lowered XML to bindings. They are deliberately thin: each
// prints what its target config produced and rejects anything else, so a
// construct the config should have lowered is an error, not a guess.

pub mod c;
pub mod cpp;
pub mod fsharp;
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

pub fn unsupported(target: &str, n: Node) -> ! {
    panic!("{target} generator cannot express <{}> at {}: the target config must lower it", n.tag_name().name(), path(n))
}
