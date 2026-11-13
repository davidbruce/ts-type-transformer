// XML -> F# bindings for Fable. Keeps inheritance, generics, overloads,
// optional parameters and unions; needs literals and type-level expressions gone.

use roxmltree::{Document, Node};

use super::{attr, elements, flag, named, only_child, unsupported};

const T: &str = "F#";

fn ident(name: &str) -> String {
    const KEYWORDS: &[&str] = &["type", "namespace", "module", "end", "done", "to", "of", "in", "val", "member", "default", "base", "begin", "class", "function", "fun", "match", "open", "rec", "then", "with", "yield"];
    if KEYWORDS.contains(&name) || !name.chars().all(|c| c.is_alphanumeric() || c == '_') { format!("``{name}``") } else { name.to_string() }
}

fn ty(n: Node) -> String {
    match n.tag_name().name() {
        "primitive" => match attr(n, "name") {
            "string" => "string",
            "number" => "float",
            "boolean" => "bool",
            "void" | "undefined" => "unit",
            _ => "obj",
        }
        .into(),
        "ref" if attr(n, "kind") == "typeParameter" => format!("'{}", attr(n, "name")),
        "ref" => {
            let args: Vec<String> = elements(n).map(ty).collect();
            if args.is_empty() { attr(n, "name").into() } else { format!("{}<{}>", attr(n, "name"), args.join(", ")) }
        }
        // Fable sends `None` as `undefined`. Where only `null` is accepted, use
        // a type that has a null of its own.
        "nullable" if attr(n, "absent") == "null" => match ty(only_child(n)).as_str() {
            t @ ("float" | "bool") => format!("System.Nullable<{t}>"),
            t => t.to_string(),
        },
        "nullable" => format!("{} option", ty(only_child(n))),
        // Fable's erased unions.
        "union" => {
            let parts: Vec<String> = elements(n).map(ty).collect();
            format!("U{}<{}>", parts.len(), parts.join(", "))
        }
        "array" => format!("ResizeArray<{}>", ty(only_child(n))),
        "functionType" => {
            let mut parts: Vec<String> = named(n, "parameter").map(|p| ty(only_child(p))).collect();
            if parts.is_empty() {
                parts.push("unit".into());
            }
            parts.push(named(n, "returns").next().map(|r| ty(only_child(r))).unwrap_or("unit".into()));
            format!("({})", parts.join(" -> "))
        }
        _ => unsupported(T, n),
    }
}

fn type_params(n: Node) -> String {
    let params: Vec<String> = named(n, "typeParameter").map(|p| format!("'{}", attr(p, "name"))).collect();
    if params.is_empty() { String::new() } else { format!("<{}>", params.join(", ")) }
}

// `a: float * ?b: string -> Shape`
fn signature(n: Node, ret: Option<&str>) -> String {
    let params: Vec<String> = named(n, "parameter")
        .map(|p| {
            if flag(p, "rest") {
                unsupported(T, p);
            }
            format!("{}{}: {}", if flag(p, "optional") { "?" } else { "" }, ident(attr(p, "name")), ty(only_child(p)))
        })
        .collect();
    let params = if params.is_empty() { "unit".to_string() } else { params.join(" * ") };
    let ret = ret.map(str::to_string).or_else(|| named(n, "returns").next().map(|r| ty(only_child(r)))).unwrap_or("unit".into());
    format!("{params} -> {ret}")
}

fn member(out: &mut String, m: Node) {
    let name = ident(m.attribute("name").unwrap_or_default());
    match m.tag_name().name() {
        "property" => {
            let mut t = ty(only_child(m));
            if flag(m, "optional") {
                t += " option";
            }
            *out += &format!("    abstract {name}: {t}{}\n", if flag(m, "readonly") { "" } else { " with get, set" });
        }
        "method" => *out += &format!("    abstract {name}{}: {}\n", type_params(m), signature(m, None)),
        _ => unsupported(T, m),
    }
}

fn interface(out: &mut String, d: Node) {
    let name = attr(d, "name");
    let instance: Vec<Node> = elements(d).filter(|m| matches!(m.tag_name().name(), "property" | "method" | "call" | "construct" | "index" | "get" | "set") && !flag(*m, "static")).collect();
    *out += &format!("[<AllowNullLiteral>]\ntype {name}{} =\n", type_params(d));
    for e in named(d, "extends") {
        *out += &format!("    inherit {}\n", ty(only_child(e)));
    }
    for m in &instance {
        member(out, *m);
    }
    if instance.is_empty() && named(d, "extends").next().is_none() {
        *out += "    interface end\n";
    }
    *out += "\n";
    if d.tag_name().name() != "class" {
        return;
    }
    // A class also has a static side: a global value with constructors and statics.
    *out += &format!("[<AllowNullLiteral>]\ntype {name}Static =\n");
    for c in named(d, "constructor") {
        *out += &format!("    [<EmitConstructor>] abstract Create: {}\n", signature(c, Some(name)));
    }
    for m in elements(d).filter(|m| flag(*m, "static")) {
        member(out, m);
    }
    *out += &format!("\n[<Global>]\nlet {name}: {name}Static = jsNative\n\n");
}

pub fn module(xml: &str) -> String {
    let doc = Document::parse(xml).expect("well-formed XML");
    let mut out = String::from("// Generated.\nmodule rec Bindings\n\nopen Fable.Core\n\n");
    for d in elements(doc.root_element()) {
        let name = d.attribute("name").unwrap_or_default();
        match d.tag_name().name() {
            "lowering" => {}
            "enum" => {
                out += &format!("[<StringEnum; RequireQualifiedAccess>]\ntype {name} =\n");
                for v in named(d, "member") {
                    if attr(v, "type") != "string" {
                        unsupported(T, v);
                    }
                    let value = attr(v, "value");
                    let mut case: String = value.chars().map(|c| if c.is_alphanumeric() { c } else { '_' }).collect();
                    case[..1].make_ascii_uppercase();
                    out += &format!("    | [<CompiledName({value:?})>] {case}\n");
                }
                out += "\n";
            }
            "interface" | "class" => interface(&mut out, d),
            "function" => {
                let params: Vec<String> = named(d, "parameter")
                    .map(|p| {
                        // Only members can have optional arguments in F#.
                        if flag(p, "optional") || flag(p, "rest") {
                            unsupported(T, p);
                        }
                        format!("{}: {}", ident(attr(p, "name")), ty(only_child(p)))
                    })
                    .collect();
                let ret = named(d, "returns").next().map(|r| ty(only_child(r))).unwrap_or("unit".into());
                let js = d.attribute("jsName").map(|j| format!("({j:?})")).unwrap_or_default();
                out += &format!("[<Global{js}>]\nlet {}{} ({}): {ret} = jsNative\n\n", ident(name), type_params(d), params.join(", "));
            }
            "typeAlias" => out += &format!("type {name}{} = {}\n\n", type_params(d), ty(only_child(d))),
            "var" => out += &format!("[<Global>]\nlet {}: {} = jsNative\n\n", ident(name), ty(only_child(d))),
            _ => unsupported(T, d),
        }
    }
    out
}
