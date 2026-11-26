use std::collections::{HashMap, HashSet};

use roxmltree::Node;

use super::super::{attr, elements, named, only_child};
use super::super::case::pascal_alnum;
use super::super::js::access as js_access;

/// Whether `s` is a valid bare F# / JS identifier chunk (alphanumeric or `_`, not starting with a digit).
pub(super) fn is_plain_ident(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_alphanumeric() || c == '_') && !s.as_bytes()[0].is_ascii_digit()
}

pub(super) fn ident(name: &str) -> String {
    const KEYWORDS: &[&str] = &[
        "abstract", "and", "as", "asr", "assert", "atomic", "base", "begin", "break", "checked", "class", "component", "const", "constraint", "constructor",
        "continue", "default", "delegate", "do", "done", "downcast", "downto", "eager", "elif", "else", "end", "event", "exception", "extern", "external",
        "false", "finally", "fixed", "for", "fun", "function", "functor", "global", "if", "in", "include", "inherit", "inline", "interface", "internal",
        "land", "lazy", "let", "lor", "lsl", "lsr", "lxor", "match", "measure", "member", "method", "mixin", "mod", "module", "mutable", "namespace", "new",
        "null", "object", "of", "open", "or", "override", "parallel", "params", "private", "process", "protected", "public", "pure", "rec", "recursive",
        "return", "sealed", "select", "sig", "static", "struct", "tailcall", "then", "to", "trait", "true", "try", "type", "upcast", "use", "val",
        "virtual", "void", "volatile", "when", "while", "with", "yield",
    ];
    if KEYWORDS.contains(&name) || !is_plain_ident(name) {
        format!("``{name}``")
    } else {
        name.to_string()
    }
}

// A type's or module's name: `$` is no part of one, even in backticks
// (`AnimationOption$1`, as a bundler names a second declaration). A member
// may have it: ``$add``.
pub(super) fn type_ident(name: &str) -> String {
    ident(&name.replace('$', "__"))
}

// A declaration's path (`Konva.Shape`) as F# names it.
pub(super) fn type_path(path: &str) -> String {
    path.split('.').map(type_ident).collect::<Vec<_>>().join(".")
}

pub(super) fn js_path(prefix: &str, name: &str) -> String {
    if prefix.is_empty() { name.to_string() } else { format!("{prefix}.{name}") }
}

// `async` as `Async`, `no-cache` as `NoCache`.
pub(super) fn pascal(word: &str) -> String {
    pascal_alnum(word)
}

pub(super) fn strip_opt(t: &str) -> &str {
    t.strip_suffix(" option").unwrap_or(t)
}

pub(super) fn add_opt(t: String) -> String {
    if t.ends_with(" option") { t } else { format!("{t} option") }
}

// `$0, $1...` as `$1, $2...`.
pub(super) fn shift(args: &str) -> String {
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

// The identifiers in F# source.
pub(super) fn words(src: &str) -> HashSet<&str> {
    // Not in a string or a comment: an `Emit`'s `"time"` names no type.
    let b = src.as_bytes();
    let mut out = HashSet::new();
    let (mut i, mut start) = (0, None);
    while i < b.len() {
        let c = b[i];
        if c.is_ascii_alphanumeric() || c == b'_' || c >= 0x80 {
            start.get_or_insert(i);
            i += 1;
            continue;
        }
        if let Some(s) = start.take() {
            out.insert(&src[s..i]);
        }
        if c == b'"' {
            i += 1;
            while i < b.len() && b[i] != b'"' {
                i += if b[i] == b'\\' { 2 } else { 1 };
            }
        } else if c == b'/' && b.get(i + 1) == Some(&b'/') {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
        }
        i += 1;
    }
    if let Some(s) = start {
        out.insert(&src[s..]);
    }
    out
}

// A JavaScript built-in as Fable has it: `JS.Date`; an Error is an F#
// exception, as Fable catches it.
pub(super) fn fable_builtin(name: &str) -> Option<String> {
    match name {
        "ArrayBuffer"
        | "ArrayBufferView"
        | "DataView"
        | "Date"
        | "Int8Array"
        | "Uint8Array"
        | "Uint8ClampedArray"
        | "Int16Array"
        | "Uint16Array"
        | "Int32Array"
        | "Uint32Array"
        | "Float32Array"
        | "Float64Array"
        | "BigInt64Array" => Some(format!("JS.{name}")),
        "Error" => Some("exn".into()),
        _ => None,
    }
}

// Fable's erased union of these types: `U2<A, B>` up to `U9`; more nest in
// the last place, `U9<A, ..., H, U3<I, J, K>>`.
pub(super) fn erased_union(mut parts: Vec<String>) -> String {
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

// The JavaScript test of a union member, from its `test` attribute; `$0` is the value.
pub(super) fn case_test(test: &str, discriminant: Option<&str>) -> Option<String> {
    let (kind, arg) = test.split_once(':').unwrap_or((test, ""));
    Some(match kind {
        "tag" => format!("$0 != null && String($0[{:?}]) === {arg:?}", discriminant?),
        "tag_has" => {
            let (v, p) = arg.rsplit_once(':')?;
            format!("$0 != null && String($0[{:?}]) === {v:?} && {p:?} in $0", discriminant?)
        }
        "typeof" => format!("typeof $0 === '{arg}'"),
        "array" => "Array.isArray($0)".into(),
        "instanceof" => format!("$0 instanceof globalThis{}", arg.split('.').map(js_access).collect::<String>()),
        "has" => format!("typeof $0 === 'object' && $0 !== null && {arg:?} in $0"),
        _ => return None,
    })
}

pub(super) fn tag_case_name(m: Node) -> Option<String> {
    let test = m.attribute("test")?;
    let raw = if let Some(v) = test.strip_prefix("tag:") {
        v
    } else if let Some(rest) = test.strip_prefix("tag_has:") {
        rest.rsplit_once(':')?.0
    } else if let Some(p) = test.strip_prefix("has:") {
        p
    } else {
        return None;
    };
    let mut case = pascal(raw);
    if case.is_empty() {
        return None;
    }
    if !case.as_bytes()[0].is_ascii_alphabetic() {
        case.insert(0, 'C');
    }
    Some(case)
}

// The name of a union case, from its member: `String`, `Widget`, `Array`.
pub(super) fn case_name(m: Node) -> String {
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
        "object" => tag_case_name(m).unwrap_or_else(|| "Object".into()),
        t => t.to_string(),
    };
    let mut case: String = raw.chars().map(|c| if c.is_alphanumeric() { c } else { '_' }).collect();
    if case.is_empty() || !case.as_bytes()[0].is_ascii_alphabetic() {
        case.insert(0, 'C');
    }
    case[..1].make_ascii_uppercase();
    case
}

// A type in a word, if it is one: `AsyncIterable`, `StringArray`.
pub(super) fn type_word(t: Node) -> Option<String> {
    match t.tag_name().name() {
        "ref" => t.attribute("name").and_then(|n| n.rsplit('.').next()).map(pascal),
        "primitive" => t.attribute("name").filter(|n| !matches!(*n, "any" | "void" | "undefined" | "unknown")).map(pascal),
        "array" => type_word(only_child(t)).map(|w| format!("{w}Array")),
        _ => None,
    }
}

/// Like `type_word`, also mapping `any`/`unknown` -> `Obj` and `void`/`undefined` -> `Unit`
/// for return-type suffixes on callback delegates.
pub(super) fn ret_type_word(t: Node) -> Option<String> {
    if t.has_tag_name("primitive") {
        match t.attribute("name")? {
            "any" | "unknown" => return Some("Obj".into()),
            "void" | "undefined" => return Some("Unit".into()),
            _ => {}
        }
    }
    type_word(t)
}

// Whether a reference is a base type: `<extends>`, or an argument of one.
pub(super) fn is_base(n: Node) -> bool {
    n.ancestors().skip(1).any(|a| a.has_tag_name("extends"))
}

pub(super) fn is_absent_null(n: Node) -> bool {
    n.has_tag_name("nullable") && attr(n, "absent") == "null"
}

// An interface of only a call signature: a function type.
pub(super) fn is_function_interface(d: Node) -> bool {
    let mut members = elements(d);
    members.next().is_some_and(|m| m.has_tag_name("call")) && members.next().is_none()
}

// A property's fixed (`const`) value as a JavaScript literal.
pub(super) fn const_js(m: Node) -> Option<String> {
    let val = m.attribute("const")?;
    Some(if only_child(m).attribute("name") == Some("string") { format!("{val:?}") } else { val.to_string() })
}

// A declaration by qualified name: `WebAssembly.Module`.
pub(super) fn find_decl<'a>(root: Node<'a, 'a>, path: &str) -> Option<Node<'a, 'a>> {
    let mut scope = root;
    let mut parts = path.split('.').peekable();
    while let Some(part) = parts.next() {
        let last = parts.peek().is_none();
        scope = elements(scope).find(|d| d.attribute("name") == Some(part) && (last || d.has_tag_name("namespace")))?;
    }
    Some(scope)
}

// The type of an alias, after any type parameters and intersection bases.
pub(super) fn only_child_type<'a>(d: Node<'a, 'a>) -> Node<'a, 'a> {
    elements(d).find(|c| !c.has_tag_name("typeParameter") && !c.has_tag_name("extends")).unwrap_or_else(|| panic!("alias {} has no type", attr(d, "name")))
}

// The properties of a map, its own and those of the maps it extends.
pub(super) fn map_properties<'a>(root: Node<'a, 'a>, decl: Node<'a, 'a>) -> Vec<(String, Node<'a, 'a>)> {
    all_properties(root, decl).into_iter().map(|p| (attr(p, "name").to_string(), only_child(p))).collect()
}

// A declaration's properties, its own first, then its bases'.
pub(super) fn all_properties<'a>(root: Node<'a, 'a>, decl: Node<'a, 'a>) -> Vec<Node<'a, 'a>> {
    let mut out: Vec<Node> = named(decl, "property").collect();
    for e in named(decl, "extends") {
        let base = only_child(e);
        if let Some(b) = base.attribute("to").and_then(|t| find_decl(root, t)) {
            out.extend(all_properties(root, b));
        }
    }
    out
}

// The members of a union an alias names, if `r` refers to one (following up to 4 alias hops).
pub(super) fn union_members<'a>(root: Node<'a, 'a>, r: Node<'a, 'a>) -> Option<Vec<Node<'a, 'a>>> {
    let mut cur = find_decl(root, r.attribute("to")?)?;
    for _ in 0..4 {
        let body = elements(cur).find(|c| !c.has_tag_name("typeParameter"))?;
        if body.has_tag_name("union") {
            return Some(elements(body).collect());
        }
        if body.has_tag_name("ref") && body.attribute("kind") == Some("typeAlias") {
            cur = find_decl(root, body.attribute("to")?)?;
        } else {
            return None;
        }
    }
    None
}

// Byte end offset of a type variable `'T` starting at byte index `i`, if `text[i..]` starts one.
pub(super) fn type_var_end(text: &str, i: usize) -> Option<usize> {
    let after_word = text[..i].chars().last().is_some_and(|p| p.is_alphanumeric() || p == '_');
    if after_word || !text[i..].starts_with('\'') {
        return None;
    }
    let start = i + 1;
    let end = text[start..]
        .char_indices()
        .take_while(|(_, c)| c.is_alphanumeric() || *c == '_')
        .last()
        .map_or(start, |(j, c)| start + j + c.len_utf8());
    Some(end)
}

// `text` with each type parameter `'T` in `map` replaced by its type.
pub(super) fn subst_params(text: &str, map: &HashMap<String, String>) -> String {
    if map.is_empty() {
        return text.to_string();
    }
    let mut out = String::new();
    let mut i = 0;
    while i < text.len() {
        if let Some(end) = type_var_end(text, i) {
            out += map.get(&text[i + 1..end]).map_or(&text[i..end], String::as_str);
            i = end;
        } else {
            let c = text[i..].chars().next().unwrap();
            out.push(c);
            i += c.len_utf8();
        }
    }
    out
}

pub(super) fn collect_type_vars(text: &str) -> HashSet<String> {
    let mut vars = HashSet::new();
    let mut i = 0;
    while i < text.len() {
        if let Some(end) = type_var_end(text, i) {
            if end > i + 1 {
                vars.insert(text[i + 1..end].to_string());
            }
            i = end;
        } else {
            i += text[i..].chars().next().unwrap().len_utf8();
        }
    }
    vars
}

pub(super) fn collect_ordered_tparams(text: &str, excluded: &HashSet<String>) -> Vec<String> {
    let mut tparams = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    let mut in_str = false;
    while i < text.len() {
        if in_str {
            if bytes[i] == b'\\' && i + 1 < text.len() {
                i += 2;
                continue;
            }
            if bytes[i] == b'"' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        if bytes[i] == b'"' {
            in_str = true;
            i += 1;
            continue;
        }
        if let Some(end) = type_var_end(text, i)
            && end > i + 1
        {
            let tp = text[i..end].to_string();
            if !excluded.contains(&tp) && !tparams.contains(&tp) {
                tparams.push(tp);
            }
            i = end;
        } else {
            i += text[i..].chars().next().unwrap().len_utf8();
        }
    }
    tparams
}

// `text` with each abbreviation's name replaced by what it stands for.
pub(super) fn expand(text: &str, abbrevs: &HashMap<String, String>) -> String {
    let mut out = text.to_string();
    for _ in 0..4 {
        let mut next = String::new();
        let mut word = String::new();
        for c in out.chars().chain(['\0']) {
            if c.is_alphanumeric() || c == '_' || c == '\'' || c == '.' {
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

// Whether an F# type string (or what it abbreviates) is an arrow/delegate type `(a -> b)`.
pub(super) fn is_fn_type<'a>(mut t: &'a str, abbrevs: &'a HashMap<String, String>) -> bool {
    for _ in 0..8 {
        let key = t.split('<').next().unwrap_or(t);
        let Some(next) = abbrevs.get(key) else { break };
        t = next.as_str();
    }
    if t.starts_with("System.Func<") || t == "System.Action" || t.starts_with("System.Action<") {
        return true;
    }
    let Some(inner) = t.strip_prefix('(').and_then(|s| s.strip_suffix(')')) else {
        return false;
    };
    let mut depth = 0usize;
    let b = inner.as_bytes();
    for i in 0..b.len() {
        match b[i] {
            b'(' | b'<' => depth += 1,
            b')' | b'>' => depth = depth.saturating_sub(1),
            b'-' if depth == 0 && inner[i..].starts_with("->") => return true,
            _ => {}
        }
    }
    false
}
