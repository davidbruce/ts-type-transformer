use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use super::types::{
    cpp_name, decodable, head, is_union, named_types, param_types, raise, ty_in, ty_out,
};
use super::{ident, import_attr, is_builtin_cpp_type, raw_fn, Ctx, ProjectRef};
use super::super::model::{Model, NamedUnion, TParam, Ty};

// Whether `u` declares the decoder of its variant: unions with the same
// members are the same type, and the first one does.
pub(super) fn decoder_owner(u: &NamedUnion, unions: &[NamedUnion], cx: &Ctx) -> bool {
    // As C++ sees it, through other unions' aliases: `ts::Array<OptionId>`
    // and `ts::Array<OrdinalRawValue>` are one type where both are one variant.
    let aliases: HashMap<&str, String> = unions.iter().filter(|o| o.type_params.is_empty()).map(|o| (o.name.as_str(), format!("std::variant<{}>", o.parts.iter().map(|p| ty_out(p, unions, cx)).collect::<Vec<_>>().join(", ")))).collect();
    let expand = |text: String| {
        let mut out = text;
        for _ in 0..8 {
            let mut next = String::new();
            let mut word = String::new();
            for c in out.chars().chain(['\0']) {
                if c.is_alphanumeric() || c == '_' {
                    word.push(c);
                    continue;
                }
                next += aliases.get(word.as_str()).map_or(word.as_str(), String::as_str);
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
    };
    let key = |u: &NamedUnion| u.parts.iter().map(|p| expand(ty_out(p, unions, cx))).collect::<Vec<_>>();
    u.type_params.is_empty() && u.decodable() && unions.iter().find(|o| o.type_params.is_empty() && o.decodable() && key(o) == key(u)).is_some_and(|o| o.name == u.name)
}

pub(super) fn decoder_guard_id(u: &NamedUnion, m: &Model, pkg_name: &str, resolved_opaque: &HashMap<String, (usize, String)>, refs: &[ProjectRef], cx: &Ctx) -> String {
    let unions = &m.unions;
    let aliases: HashMap<&str, String> = unions
        .iter()
        .filter(|o| o.type_params.is_empty())
        .map(|o| (o.name.as_str(), format!("std::variant<{}>", o.parts.iter().map(|p| ty_out(p, unions, cx)).collect::<Vec<_>>().join(", "))))
        .collect();
    let mut expanded = format!("std::variant<{}>", u.parts.iter().map(|p| ty_out(p, unions, cx)).collect::<Vec<_>>().join(", "));
    for _ in 0..8 {
        let mut next = String::new();
        let mut word = String::new();
        for c in expanded.chars().chain(['\0']) {
            if c.is_alphanumeric() || c == '_' {
                word.push(c);
                continue;
            }
            next += aliases.get(word.as_str()).map_or(word.as_str(), String::as_str);
            word.clear();
            if c != '\0' {
                next.push(c);
            }
        }
        if next == expanded {
            break;
        }
        expanded = next;
    }
    let local_types: HashSet<&str> = m
        .classes
        .iter()
        .map(|c| c.name.as_str())
        .chain(m.callbacks.iter().map(|c| c.name.as_str()))
        .chain(m.enums.iter().map(|e| e.0.as_str()))
        .chain(m.num_enums.iter().map(|e| e.0.as_str()))
        .chain(m.opaque.iter().map(String::as_str).filter(|n| !matches!(*n, "Promise" | "PromiseLike")))
        .collect();
    let mut canonical = String::new();
    let mut word = String::new();
    let mut preceded_by_colon = false;
    for c in expanded.chars().chain(['\0']) {
        if c.is_alphanumeric() || c == '_' {
            word.push(c);
            continue;
        }
        if !word.is_empty() {
            let tok = word.as_str();
            let is_std = matches!(tok, "std" | "variant" | "string" | "optional" | "double" | "bool" | "ts" | "Array" | "Tuple" | "Value" | "Promise" | "PromiseLike");
            if preceded_by_colon || is_std {
                canonical.push_str(tok);
            } else if let Some((idx, orig)) = resolved_opaque.get(tok) {
                canonical += &format!("::{}::{orig}", refs[*idx].pkg_name);
            } else if pkg_name != "bindings" && local_types.contains(tok) {
                canonical += &format!("::{pkg_name}::{tok}");
            } else {
                canonical.push_str(tok);
            }
            word.clear();
        }
        if c != '\0' {
            preceded_by_colon = c == ':';
            canonical.push(c);
        }
    }
    format!("{:016x}", crate::fnv1a64(&canonical))
}

// A union as a type in its own definitions: `Either` or `Either<T>`.
pub(super) fn union_type(u: &NamedUnion) -> String {
    if u.type_params.is_empty() { u.name.clone() } else { format!("{}<{}>", u.name, u.type_params.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(", ")) }
}

// `ts_types.hpp`: every type declared, so a struct may name any of them
// before it is defined. A template comes after what its defaults and
// constraints name.
pub(super) fn declarations(m: &Model, pkg_name: &str, refs: &[ProjectRef], resolved_opaque: &HashMap<String, (usize, String)>, cx: &Ctx) -> String {
    let wasm_module = if pkg_name == "bindings" { "env" } else { pkg_name };
    let raw_prefix = if pkg_name == "bindings" { String::new() } else { format!("{pkg_name}_") };
    let ns: Option<&str> = (pkg_name != "bindings").then_some(pkg_name);
    let ns_prefix = &cx.ns_prefix;
    let inc_prefix = if pkg_name == "bindings" { String::new() } else { format!("{pkg_name}/") };
    let used_refs: BTreeSet<usize> = resolved_opaque.values().map(|(i, _)| *i).collect();

    let unions = &m.unions;
    let mut types = format!("// Generated.\n#pragma once\n#include \"{inc_prefix}ts_runtime.hpp\"\n");
    for &idx in &used_refs {
        types += &format!("#include \"{}\"\n", refs[idx].umbrella_include);
    }
    types.push('\n');
    let enum_val_raw = raw_fn(&raw_prefix, "js_enum_value");
    let enum_idx_raw = raw_fn(&raw_prefix, "js_enum_index");
    if !m.enums.is_empty() {
        types += &format!(
            "extern \"C\" {{\n{} uint32_t {enum_val_raw}(int32_t, int32_t);\n{} int32_t {enum_idx_raw}(int32_t, uint32_t);\n}}\n\n",
            import_attr(wasm_module, "js_enum_value"),
            import_attr(wasm_module, "js_enum_index"),
        );
    }
    if let Some(pkg) = ns {
        types += &format!("namespace {pkg} {{\n");
    }
    let mut enum_specs = String::new();
    for (id, (name, values)) in m.enums.iter().enumerate() {
        let mut seen = std::collections::HashSet::new();
        let members: Vec<String> = values
            .iter()
            .map(|v| ident(if v.is_empty() { "empty" } else { v }))
            .map(|mut v| {
                if !seen.insert(v.clone()) {
                    let base = v;
                    let mut i = 2;
                    while !seen.insert(format!("{base}_{i}")) {
                        i += 1;
                    }
                    v = format!("{base}_{i}");
                }
                v
            })
            .collect();
        types += &format!("enum class {name} : int32_t {{ {} }};\n", members.join(", "));
        enum_specs += &format!(
            "template <>\ninline constexpr int32_t ts::enum_id<::{ns_prefix}{name}> = {id};\n\
             template <>\ninline uint32_t ts::enum_to_handle<::{ns_prefix}{name}>(::{ns_prefix}{name} x) {{ return {enum_val_raw}({id}, int32_t(x)); }}\n\
             template <>\ninline ::{ns_prefix}{name} ts::enum_from_handle<::{ns_prefix}{name}>(uint32_t h) {{ return ::{ns_prefix}{name}({enum_idx_raw}({id}, h)); }}\n"
        );
    }
    if !enum_specs.is_empty() {
        if let Some(pkg) = ns {
            types += &format!("}}  // namespace {pkg}\n{enum_specs}namespace {pkg} {{\n");
        } else {
            types += &enum_specs;
        }
    }
    for (name, members) in &m.num_enums {
        let members: Vec<String> = members.iter().map(|(member, value)| format!("{} = {value}", ident(member))).collect();
        types += &format!("enum class {name} : int32_t {{ {} }};\n", members.join(", "));
    }
    let opaque = |name: &str, params: &[TParam]| {
        let mut body = String::from("    ");
        body += &format!("{name}() = default;\n    {name}(ts::handle_t, uint32_t h) : ts::Handle(ts::handle, h) {{}}\n");
        if name == "Error" {
            body += "    Error(const ts::Error& e) : ts::Handle(e) {}\n\
                     static Error create(std::string_view msg) { return Error(ts::handle, ts_raw_js_error_new(msg.data(), msg.size())); }\n\
                     std::string name() const { return ts::from_handle<std::string>(ts_raw_js_error_name(this->h)); }\n\
                     std::string message() const { return ts::from_handle<std::string>(ts_raw_js_error_message(this->h)); }\n\
                     std::string stack() const { return ts::from_handle<std::string>(ts_raw_js_error_stack(this->h)); }\n";
        } else if name == "Date" {
            body += "    static Date from_time(double ms) { return Date(ts::handle, ts_raw_js_date_new(ms)); }\n\
                     static double now() { return ts_raw_js_date_now(); }\n\
                     double time() const { return ts_raw_js_date_time(this->h); }\n\
                     std::string to_iso_string() const { return ts::from_handle<std::string>(ts_raw_js_date_to_iso_string(this->h)); }\n";
        } else if let Some(s) = super::super::model::buffer_spec(name) {
            body += "    uint32_t byte_length() const { return ts_raw_js_bytes_len(this->h); }\n\
                     uint32_t read_bytes(void* dst, uint32_t len, uint32_t offset = 0) const { return ts_raw_js_bytes_read(this->h, offset, dst, len); }\n\
                     uint32_t write_bytes(const void* src, uint32_t len, uint32_t offset = 0) const { return ts_raw_js_bytes_write(this->h, offset, src, len); }\n";
            if s.has_buffer() {
                body += "    uint32_t byte_offset() const { return ts_raw_js_bytes_offset(this->h); }\n\
                         ts::Value buffer() const { return ts::Value(ts::handle, ts_raw_js_bytes_buffer(this->h)); }\n";
            }
            if s.is_typed_array() {
                let kind = s.kind.unwrap();
                let elem = super::super::c::c_elem(s.elem);
                let eb = s.elem.bytes();
                body += &format!(
                    "    static {name} create(size_t count) {{ uint32_t n = count > (UINT32_MAX / {eb}u) ? UINT32_MAX : uint32_t(count * {eb}u); return {name}(ts::handle, ts_raw_js_bytes_new({kind}, nullptr, n)); }}\n\
                     static {name} from(const {elem}* data, size_t count) {{ uint32_t n = count > (UINT32_MAX / {eb}u) ? UINT32_MAX : uint32_t(count * {eb}u); return {name}(ts::handle, ts_raw_js_bytes_new({kind}, data, n)); }}\n\
                     static {name} from(const std::vector<{elem}>& data) {{ return from(data.data(), data.size()); }}\n\
                     size_t size() const {{ return size_t(byte_length()) / {eb}; }}\n\
                     size_t read({elem}* dst, size_t count, size_t elem_offset = 0) const {{ if (elem_offset > (UINT32_MAX / {eb}u)) return 0; uint32_t n = count > (UINT32_MAX / {eb}u) ? UINT32_MAX : uint32_t(count * {eb}u); return size_t(ts_raw_js_bytes_read(this->h, uint32_t(elem_offset * {eb}u), dst, n)) / {eb}; }}\n\
                     size_t write(const {elem}* src, size_t count, size_t elem_offset = 0) const {{ if (elem_offset > (UINT32_MAX / {eb}u)) return 0; uint32_t n = count > (UINT32_MAX / {eb}u) ? UINT32_MAX : uint32_t(count * {eb}u); return size_t(ts_raw_js_bytes_write(this->h, uint32_t(elem_offset * {eb}u), src, n)) / {eb}; }}\n\
                     std::vector<{elem}> to_vector() const {{ std::vector<{elem}> out(size()); if (!out.empty()) read(out.data(), out.size()); return out; }}\n"
                );
            } else {
                if let Some(kind) = s.kind {
                    body += &format!(
                        "    static {name} create(uint32_t byte_len) {{ return {name}(ts::handle, ts_raw_js_bytes_new({kind}, nullptr, byte_len)); }}\n\
                         static {name} from(const void* data, uint32_t byte_len) {{ return {name}(ts::handle, ts_raw_js_bytes_new({kind}, data, byte_len)); }}\n\
                         static {name} from(const std::vector<uint8_t>& data) {{ return from(data.data(), uint32_t(data.size())); }}\n"
                    );
                }
                body += "    uint32_t read(void* dst, uint32_t len, uint32_t offset = 0) const { return read_bytes(dst, len, offset); }\n\
                         uint32_t write(const void* src, uint32_t len, uint32_t offset = 0) const { return write_bytes(src, len, offset); }\n\
                         std::vector<uint8_t> to_vector() const { std::vector<uint8_t> out(byte_length()); if (!out.empty()) read_bytes(out.data(), uint32_t(out.size())); return out; }\n";
            }
        }
        format!("{}struct {name} : virtual ts::Handle {{\n{body}}};\n", head(params, true, cx))
    };
    // (name, declaration, what it needs declared first)
    let mut templates: Vec<(String, String, BTreeSet<String>)> = Vec::new();
    let mut template = |name: &str, params: &[TParam], text: String| {
        let mut needs = BTreeSet::new();
        param_types(params, &mut needs);
        needs.remove(name);
        templates.push((name.to_string(), text, needs));
    };
    for c in &m.classes {
        let text = format!("{}struct {};\n", head(&c.type_params, true, cx), c.name);
        if c.type_params.is_empty() { types += &text } else { template(&c.name, &c.type_params, text) }
    }
    let mut cb_specs = String::new();
    for c in &m.callbacks {
        let decl = format!("{}struct {};\n", head(&c.type_params, true, cx), c.name);
        if c.type_params.is_empty() {
            types += &decl;
            cb_specs += &format!("template <>\ninline constexpr bool ts::is_callback<::{ns_prefix}{}> = true;\n", c.name);
        } else {
            let ps: Vec<String> = (0..c.type_params.len()).map(|i| format!("B{i}__")).collect();
            let spec_head = format!("template <{}>\n", ps.iter().map(|p| format!("class {p}")).collect::<Vec<_>>().join(", "));
            let spec_args = ps.join(", ");
            let spec = format!(
                "{spec_head}inline constexpr bool ts::is_callback<::{ns_prefix}{}<{spec_args}>> = true;\n",
                c.name
            );
            let text = if let Some(pkg) = ns {
                format!("{decl}}}  // namespace {pkg}\n{spec}namespace {pkg} {{\n")
            } else {
                format!("{decl}{spec}")
            };
            template(&c.name, &c.type_params, text);
        }
    }
    if !cb_specs.is_empty() {
        if let Some(pkg) = ns {
            types += &format!("}}  // namespace {pkg}\n{cb_specs}namespace {pkg} {{\n");
        } else {
            types += &cb_specs;
        }
    }
    // Types declared elsewhere: handles, generic or not.
    // Promises are the runtime's.
    for name in m.opaque.iter().filter(|n| matches!(n.as_str(), "Promise" | "PromiseLike")) {
        types += &format!("template <class T = void>\nusing {name} = ts::Promise<T>;\n");
    }
    for name in m.opaque.iter().filter(|n| !matches!(n.as_str(), "Promise" | "PromiseLike")) {
        if let Some((idx, orig)) = resolved_opaque.get(name) {
            types += &format!("using {name} = ::{}::{orig};\n", refs[*idx].pkg_name);
            continue;
        }
        let arity = m.opaque_arity.get(name).copied().unwrap_or(0);
        let params: Vec<TParam> = (1..=arity).map(|i| TParam { name: format!("T{i}"), default: Some(Ty::Any), constraint: None }).collect();
        let mark = if is_builtin_cpp_type(name) { "" } else { "// external\n" };
        if params.is_empty() { types += &format!("{mark}{}", opaque(name, &params)) } else { template(name, &params, format!("{mark}{}", opaque(name, &params))) }
    }
    // Unions, with the templates: either may name the other, in a default
    // argument (`Interval<StartType = DateArg<Date>>`) or a member (`Box<T>`).
    for u in unions {
        let parts: Vec<String> = u.parts.iter().map(|p| ty_out(p, unions, cx)).collect();
        let mut text = format!("{}using {} = std::variant<{}>;\n", head(&u.type_params, true, cx), u.name, parts.join(", "));
        if u.type_params.is_empty() {
            text += &m.type_aliases.iter().filter(|(_, to)| *to == u.name).map(|(alias, _)| format!("using {alias} = {};\n", u.name)).collect::<String>();
        }
        // Defined with the union, used wherever one comes back.
        if u.decodable() {
            text += &format!("{}{} decode_{}(uint32_t h);\n", head(&u.type_params, false, cx), union_type(u), u.name);
            // Unions with the same members are the same variant: one decoder.
            if decoder_owner(u, unions, cx) {
                let guard = decoder_guard_id(u, m, pkg_name, resolved_opaque, refs, cx);
                let spec = format!(
                    "#ifndef TS_DECODER_{guard}\n#define TS_DECODER_{guard}\n\
                     template <>\nstruct ts::decoder<::{ns_prefix}{0}> {{\n    static ::{ns_prefix}{0} from(uint32_t h);\n}};\n\
                     #endif\n",
                    u.name
                );
                if let Some(pkg) = ns {
                    text += &format!("}}  // namespace {pkg}\n{spec}namespace {pkg} {{\n");
                } else {
                    text += &spec;
                }
            }
        }
        let mut needs = BTreeSet::new();
        u.parts.iter().for_each(|p| named_types(p, &mut needs));
        param_types(&u.type_params, &mut needs);
        needs.remove(&u.name);
        // Naming a constrained template (`Path<PathProps>`) checks its
        // constraint there: the bases of what it names come first.
        let bases: Vec<String> = needs.iter().filter(|n| m.classes.iter().any(|c| c.name == **n && !c.bases.is_empty())).map(|n| format!("bases:{n}")).collect();
        needs.extend(bases);
        templates.push((u.name.clone(), text, needs));
    }
    // What each type extends, for constraints: see `ts::bases`. After the
    // types its bases name, theirs first.
    for c in m.classes.iter().filter(|c| !c.bases.is_empty()) {
        let base_list = c.bases.iter().map(|b| ty_in(b, cx)).collect::<Vec<_>>().join(", ");
        let tpl_head = if c.type_params.is_empty() {
            String::new()
        } else {
            format!("template <{}>\n", c.type_params.iter().map(|p| format!("class {}", p.name)).collect::<Vec<_>>().join(", "))
        };
        let (spec_head, spec_args) = if c.type_params.is_empty() {
            ("template <>\n".to_string(), String::new())
        } else {
            let ps: Vec<String> = (0..c.type_params.len()).map(|i| format!("B{i}__")).collect();
            (
                format!("template <{}>\n", ps.iter().map(|p| format!("class {p}")).collect::<Vec<_>>().join(", ")),
                format!("<{}>", ps.join(", ")),
            )
        };
        let text = if let Some(pkg) = ns {
            format!(
                "{tpl_head}using ts_bases_of_{0}_ = ts::list<{base_list}>;\n\
                 }}  // namespace {pkg}\n\
                 {spec_head}struct ts::bases<::{pkg}::{0}{spec_args}> {{\n    using type = ::{pkg}::ts_bases_of_{0}_{spec_args};\n}};\n\
                 namespace {pkg} {{\n",
                c.name
            )
        } else {
            format!(
                "{tpl_head}using ts_bases_of_{0}_ = ts::list<{base_list}>;\n\
                 {spec_head}struct ts::bases<::{0}{spec_args}> {{\n    using type = ::ts_bases_of_{0}_{spec_args};\n}};\n",
                c.name
            )
        };
        let mut named = BTreeSet::new();
        c.bases.iter().for_each(|b| named_types(b, &mut named));
        let mut needs: BTreeSet<String> = named.iter().filter(|n| **n != c.name && m.classes.iter().any(|k| k.name == **n && !k.bases.is_empty())).map(|n| format!("bases:{n}")).collect();
        needs.extend(named.into_iter().filter(|n| *n != c.name));
        // A template is specialized once declared.
        if !c.type_params.is_empty() {
            needs.insert(c.name.clone());
        }
        templates.push((format!("bases:{}", c.name), text, needs));
    }
    let names: BTreeSet<String> = templates.iter().map(|t| t.0.clone()).collect();
    let mut done = BTreeSet::new();
    while !templates.is_empty() {
        let ready = templates.iter().position(|(_, _, needs)| needs.iter().all(|n| done.contains(n) || !names.contains(n))).unwrap_or(0);
        let (name, text, _) = templates.remove(ready);
        types += &text;
        done.insert(name);
    }
    // A map's keys, each typed by the value it selects.
    for k in &m.maps {
        let keys: Vec<String> = k.keys.iter().zip(&k.values).enumerate().map(|(i, (key, v))| format!("    inline constexpr ts::Key<{}, {}> {}{{{i}}};\n", k.name, ty_out(v, unions, cx), ident(key))).collect();
        types += &format!("namespace {}Keys {{\n{}}}\n", k.name, keys.concat());
    }
    if let Some(pkg) = ns {
        types += &format!("}}  // namespace {pkg}\n");
    }
    types
}

// A callback argument. Objects are made as a type that depends on the
// lambda's, so they need be complete only where a lambda is passed.
pub(super) fn raise_callback(t: &Ty, a: &str, unions: &[NamedUnion], cx: &Ctx) -> String {
    match t {
        Ty::Num | Ty::NumEnum(_) | Ty::Bool | Ty::Enum(_) | Ty::Str => raise(t, a, unions, cx),
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Bool | Ty::Enum(_)) => raise(t, a, unions, cx),
        // A union that can be decoded is the variant; any other a value.
        Ty::NamedUnion(n) | Ty::Generic(n, _) if decodable(n, unions) => format!("ts::from_handle<ts::dep<F, {}>>({a})", ty_out(t, unions, cx)),
        Ty::NamedUnion(_) | Ty::Union(_) | Ty::Generic(..) if !matches!(t, Ty::Generic(n, _) if !is_union(n, unions)) => format!("ts::Value(ts::handle, {a})"),
        t => format!("ts::from_handle<ts::dep<F, {}>>({a})", ty_out(t, unions, cx)),
    }
}

// What a callback returns: a number, a flag, or a handle.
pub(super) fn lower_callback(t: &Ty, x: &str, cx: &Ctx) -> String {
    match t {
        Ty::Num => x.into(),
        Ty::NumEnum(_) => format!("double({x})"),
        Ty::Bool => format!("({x} ? 1 : 0)"),
        Ty::Enum(_) => format!("int32_t({x})"),
        Ty::Nullable(inner, _) if **inner == Ty::Bool => format!("[](std::optional<bool> v) {{ return v ? int32_t(*v) : -1; }}({x})"),
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Enum(_)) => {
            let Ty::Enum(e) = &**inner else { unreachable!() };
            format!("[](std::optional<{}> v) {{ return v ? int32_t(*v) : -1; }}({x})", cx.tn(e))
        }
        _ => format!("ts::Arg({x}).give()"),
    }
}

// `text` with each whole word in `to` replaced.
pub(super) fn rename_words(text: &str, to: &HashMap<String, String>) -> String {
    let mut out = String::new();
    let mut word = String::new();
    for c in text.chars().chain(['\0']) {
        if c.is_alphanumeric() || c == '_' {
            word.push(c);
            continue;
        }
        out += to.get(&word).map_or(word.as_str(), String::as_str);
        word.clear();
        if c != '\0' {
            out.push(c);
        }
    }
    out
}

// A namespace's types by their own names in it, as aliases of theirs:
// `Intl.Locale` is `Intl::Locale` too, a namespace of aliases after every
// type is declared. A namespace a class shares its name with (node's
// `Stream`) is in that class: `Stream::PipeOptions`. One a function or a
// generic class shares its name with is `<name>_ns`, as C++ allows a name
// one kind of thing in a scope. Returns the namespaces, and each class's
// aliases.
pub(super) fn namespace_aliases(m: &Model, ns_prefix: &str, cx: &Ctx) -> (String, HashMap<String, String>) {
    let params_of = |n: &str| -> Option<Vec<TParam>> {
        m.classes.iter().find(|c| c.name == n).map(|c| c.type_params.clone())
            .or_else(|| m.callbacks.iter().find(|c| c.name == n).map(|c| c.type_params.clone()))
            .or_else(|| m.unions.iter().find(|u| u.name == n).map(|u| u.type_params.clone()))
            .or_else(|| (m.enums.iter().any(|e| e.0 == n) || m.num_enums.iter().any(|e| e.0 == n)).then(Vec::new))
    };
    // (namespaces, own name, the type): every namespaced type C++ has.
    let mut entries: Vec<(Vec<String>, String, String)> = Vec::new();
    for (name, path) in &m.namespaced {
        if params_of(name).is_some() {
            entries.push((path[..path.len() - 1].to_vec(), path.last().unwrap().clone(), name.clone()));
        }
    }
    entries.sort();
    let mut taken: HashSet<String> = m.classes.iter().map(|c| c.name.clone()).chain(m.callbacks.iter().map(|c| c.name.clone())).chain(m.unions.iter().map(|u| u.name.clone())).collect();
    taken.extend(m.enums.iter().map(|e| e.0.clone()).chain(m.num_enums.iter().map(|e| e.0.clone())).chain(m.opaque.iter().cloned()));
    taken.extend(m.funcs.iter().map(|f| cpp_name(f, "create")).chain(m.maps.iter().map(|k| format!("{}Keys", k.name))).chain(["ts".to_string()]));
    let alias = |leaf: &str, target: &str| -> String {
        let params = params_of(target).unwrap_or_default();
        let args = if params.is_empty() { String::new() } else { format!("<{}>", params.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(", ")) };
        format!("{}using {} = ::{ns_prefix}{target}{args};\n", head(&params, true, cx).replace('\n', " "), ident(leaf))
    };
    // One level: its own types' aliases, then each namespace in it.
    let levels = level_names(m);
    fn level(entries: &[(Vec<String>, String, String)], at: &[String], kind: &str, indent: &str, alias: &dyn Fn(&str, &str) -> String, levels: &HashMap<Vec<String>, HashSet<String>>) -> String {
        let mut out = String::new();
        let own: Vec<&String> = entries.iter().filter(|(p, _, _)| p == at).map(|(_, l, _)| l).collect();
        // Each name once (node's `cluster` has a class and an interface `Worker`).
        let mut seen: HashSet<String> = HashSet::new();
        for (_, leaf, target) in entries.iter().filter(|(p, _, _)| p == at) {
            let mut name = leaf.clone();
            let mut i = 2;
            while !seen.insert(name.clone()) {
                name = format!("{leaf}_{i}");
                i += 1;
            }
            out += &format!("{indent}{}", alias(&name, target));
        }
        let children: BTreeSet<&String> = entries.iter().filter(|(p, _, _)| p.len() > at.len() && p[..at.len()] == *at).map(|(p, _, _)| &p[at.len()]).collect();
        for child in children {
            let mut path = at.to_vec();
            path.push(child.clone());
            // Beside a type or function of its name it is apart.
            let name = if own.contains(&child) || levels.get(at).is_some_and(|n| n.contains(&ident(child))) { format!("{}_ns", ident(child)) } else { ident(child) };
            let inner = level(entries, &path, kind, &format!("{indent}    "), alias, levels);
            out += &if kind == "struct" { format!("{indent}struct {name} {{\n{inner}{indent}}};\n") } else { format!("{indent}namespace {name} {{\n{inner}{indent}}}\n") };
        }
        out
    }
    let mut namespaces = String::new();
    let mut in_structs: HashMap<String, String> = HashMap::new();
    let _ = taken;
    let tops: BTreeSet<&String> = entries.iter().map(|(p, _, _)| &p[0]).collect();
    let containers = namespace_containers(m);
    for top in tops {
        match &containers[top] {
            Container::Class(c) => {
                in_structs.insert(c.clone(), level(&entries, std::slice::from_ref(top), "struct", "    ", &alias, &levels));
            }
            Container::Namespace(name) => {
                namespaces += &format!("namespace {name} {{\n{}}}\n", level(&entries, std::slice::from_ref(top), "namespace", "    ", &alias, &levels));
            }
        }
    }
    (namespaces, in_structs)
}

// The names directly in each namespace: its types' and functions' own.
pub(super) fn level_names(m: &Model) -> HashMap<Vec<String>, HashSet<String>> {
    let mut out: HashMap<Vec<String>, HashSet<String>> = HashMap::new();
    for path in m.namespaced.values() {
        out.entry(path[..path.len() - 1].to_vec()).or_default().insert(ident(path.last().unwrap()));
    }
    for (path, own, _, _) in m.func_namespaces.values() {
        out.entry(path.clone()).or_default().insert(ident(own));
    }
    out
}

// A nested namespace's name: `<name>_ns` where its parent has something of
// its name (`events`' class `EventEmitter` beside its namespace of functions).
pub(super) fn nested_name(levels: &HashMap<Vec<String>, HashSet<String>>, parent: &[String], name: &str) -> String {
    if levels.get(parent).is_some_and(|n| n.contains(&ident(name))) { format!("{}_ns", ident(name)) } else { ident(name) }
}

// Where a top-level namespace's types and functions are: in the class it
// shares its name with (node's `Stream`: `Stream::Duplex`,
// `Stream::finished`), but for a generic one (`Control<O>::Zoom` would want
// its arguments) or one a member of which has a name of theirs; else in a
// namespace of its name, `<name>_ns` where something else has it (marked's
// function `marked`), as C++ allows a name one kind of thing in a scope.
#[derive(Clone, PartialEq)]
pub(super) enum Container {
    Class(String),
    Namespace(String),
}

pub(super) fn namespace_containers(m: &Model) -> HashMap<String, Container> {
    // Each top-level namespace's own names, directly in it.
    let mut own: BTreeMap<String, HashSet<String>> = BTreeMap::new();
    for path in m.namespaced.values() {
        let e = own.entry(path[0].clone()).or_default();
        if path.len() == 2 {
            e.insert(ident(&path[1]));
        }
    }
    for (path, leaf, _, _) in m.func_namespaces.values() {
        let e = own.entry(path[0].clone()).or_default();
        if path.len() == 1 {
            e.insert(ident(leaf));
        }
    }
    let mut taken: HashSet<String> = m.classes.iter().map(|c| c.name.clone()).chain(m.callbacks.iter().map(|c| c.name.clone())).chain(m.unions.iter().map(|u| u.name.clone())).collect();
    taken.extend(m.enums.iter().map(|e| e.0.clone()).chain(m.num_enums.iter().map(|e| e.0.clone())).chain(m.opaque.iter().cloned()));
    taken.extend(m.funcs.iter().filter(|f| !m.func_namespaces.contains_key(&f.import)).map(|f| cpp_name(f, "create")).chain(m.maps.iter().map(|k| format!("{}Keys", k.name))).chain(["ts".to_string()]));
    let mut out = HashMap::new();
    for (top, names) in own {
        let class = m.classes.iter().find(|c| c.name == top && !m.namespaced.contains_key(&c.name));
        let members: HashSet<String> = class.map(|c| c.funcs.iter().map(|f| cpp_name(f, &c.name)).collect()).unwrap_or_default();
        let clashes = names.iter().any(|n| members.contains(n) || *n == top);
        let container = match class {
            Some(c) if c.type_params.is_empty() && !clashes => Container::Class(c.name.clone()),
            _ => Container::Namespace(if taken.contains(&top) { format!("{}_ns", ident(&top)) } else { ident(&top) }),
        };
        out.insert(top, container);
    }
    out
}
