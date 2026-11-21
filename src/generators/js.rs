// JavaScript expressions that reach the API: shared by the wasm glue and
// Gleam's FFI module.

use std::collections::BTreeMap;

use super::model::Call;

// A union member's runtime test as a JavaScript condition on `v`.
// `js_refs` says where a class the output declares is reached.
pub(crate) fn js_test(test: &str, discriminant: Option<&str>, js_refs: &BTreeMap<String, String>) -> String {
    let (kind, arg) = test.split_once(':').unwrap_or((test, ""));
    match kind {
        "typeof" => format!("typeof v === {arg:?}"),
        "array" => "Array.isArray(v)".into(),
        "instanceof" => format!("v instanceof {}", global(js_refs.get(arg).map_or(arg, String::as_str))),
        "has" => format!("typeof v === 'object' && v !== null && {arg:?} in v"),
        "tag" => format!("v != null && String(v[{:?}]) === {arg:?}", discriminant.unwrap_or_default()),
        _ => "false".into(),
    }
}

// A name the API declares at its top level. In a package, only in its module:
// `prosemirror-model::Node` is `PK["prosemirror-model"].Node`, and not a
// global `Node` should the module lack it. Else in `options.root` if there,
// else a global: `Pt.make` is `(G.Pt ?? globalThis.Pt).make`.
pub(crate) fn global(path: &str) -> String {
    if let Some((package, path)) = path.split_once("::") {
        return format!("PK[{package:?}]{}", access_path(path));
    }
    let (first, rest) = path.split_once('.').map_or((path, String::new()), |(f, r)| (f, access_path(r)));
    format!("(G{0} ?? globalThis{0}){rest}", access(first))
}

// `.name`, or `["some-name"]` where the name is not an identifier.
pub(crate) fn access(name: &str) -> String {
    // A symbol's key is written as it is: `[Symbol.toStringTag]`.
    if name.starts_with("[Symbol.") {
        return name.to_string();
    }
    let plain = !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '$') && !name.as_bytes()[0].is_ascii_digit();
    if plain { format!(".{name}") } else { format!("[{name:?}]") }
}

pub(crate) fn access_path(path: &str) -> String {
    path.split('.').map(access).collect()
}

/// JavaScript expression invoking `call` against `self_expr` with formatted `args`.
pub(crate) fn call_expr(call: &Call, self_expr: &str, args: &[String]) -> String {
    let list = args.join(", ");
    let first = args.first().map(String::as_str).unwrap_or("");
    let second = args.get(1).map(String::as_str).unwrap_or("");
    match call {
        Call::Method(js) => format!("{self_expr}{}({list})", access(js)),
        Call::Get(prop) => format!("{self_expr}{}", access(prop)),
        Call::GetBound(prop) => format!("((v) => typeof v === 'function' ? v.bind({self_expr}) : v)({self_expr}{})", access(prop)),
        Call::Set(prop) => format!("{self_expr}{} = {list}", access(prop)),
        Call::New(path) => format!("new ({})({list})", global(path)),
        Call::Static(path) => format!("{}({list})", global(path)),
        Call::StaticGet(path) => global(path),
        // A variable of the library's own module cannot be assigned from outside it.
        Call::StaticSet(path) if !path.contains('.') && !path.contains("::") => format!("globalThis{} = {list}", access(path)),
        Call::StaticSet(path) => format!("{} = {list}", global(path)),
        Call::Create(literal) => format!("({literal})"),
        Call::Invoke => format!("{self_expr}({list})"),
        Call::Construct => format!("new ({self_expr})({list})"),
        Call::IndexGet => format!("{self_expr}[{first}]"),
        Call::IndexSet => format!("{self_expr}[{first}] = {second}"),
        // The function is a handle: `args[0]`, with no comma in it.
        Call::Apply => format!("({first})({})", args.get(1..).unwrap_or_default().join(", ")),
    }
}
