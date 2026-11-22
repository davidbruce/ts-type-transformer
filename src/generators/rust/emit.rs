use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use super::classes::{callback, emit_classes, union_enum};
use super::types::{function, names_of, ns_module, rust_elem};
use super::{camel, camel_variant, rust_function, rust_type, snake, Context, ProjectRef, PRELUDE, RUNTIME, RUNTIME_TYPES};
use super::super::comments::{Style, comment};
use super::super::model::{Call, Model, Ty};

// Enum variants on one line; where some have documentation, one to a line.
fn variant_list<'a>(variants: impl Iterator<Item = (String, Option<&'a str>)>) -> String {
    let variants: Vec<(String, Option<&str>)> = variants.collect();
    if variants.iter().all(|v| v.1.is_none()) {
        return variants.into_iter().map(|v| v.0).collect::<Vec<_>>().join(", ");
    }
    let mut out = String::from("\n");
    for (variant, doc) in variants {
        out += &doc.map(|d| comment(d, "///", "    ", Style::Rust)).unwrap_or_default();
        out += &format!("    {variant},\n");
    }
    out
}

// A module's file: `areas/core` is `src/areas_core.rs`.
pub(super) fn module_file(module: &str) -> String {
    let name: String = snake(&module.replace('/', "_")).chars().map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' }).collect();
    if name.is_empty() { "main".into() } else { name }
}

pub(super) fn files_named(m: &Model, crate_name: &str, refs: &[ProjectRef]) -> Result<Vec<(String, String)>, Vec<String>> {
    let crate_name = if crate_name.is_empty() { "bindings" } else { crate_name };
    let wasm_module = if crate_name == "bindings" { "env" } else { crate_name };
    // Each type's Rust name: `names::settle` made them apart.
    let declared: Vec<&String> = m.enums.iter().map(|e| &e.0).chain(m.num_enums.iter().map(|e| &e.0)).chain(m.classes.iter().map(|c| &c.name)).chain(m.callbacks.iter().map(|c| &c.name)).chain(m.unions.iter().map(|u| &u.name)).chain(&m.opaque).collect();
    let mut types: HashMap<String, String> = RUNTIME_TYPES.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect();
    // A namespace's type is in a module of it: `Stream.PipeOptions` is
    // `stream::PipeOptions`. Its own name is apart in that module.
    let mut own: HashMap<String, String> = HashMap::new();
    let mut module_of: HashMap<String, Vec<String>> = HashMap::new();
    let mut taken_in: HashMap<Vec<String>, HashSet<String>> = HashMap::new();
    for n in declared.iter().filter(|n| m.namespaced.contains_key(n.as_str())) {
        let path = &m.namespaced[n.as_str()];
        let segments: Vec<String> = path[..path.len() - 1].iter().map(|s| ns_module(s)).collect();
        let base = rust_type(path.last().unwrap());
        let taken = taken_in.entry(segments.clone()).or_default();
        let mut leaf = base.clone();
        let mut i = 2;
        while !taken.insert(leaf.clone()) {
            leaf = format!("{base}{i}");
            i += 1;
        }
        types.insert(n.to_string(), format!("{}::{leaf}", segments.join("::")));
        own.insert(n.to_string(), leaf);
        module_of.insert(n.to_string(), segments);
    }
    for n in &declared {
        types.entry(n.to_string()).or_insert_with(|| rust_type(n));
    }
    let mut resolved_opaque: HashMap<String, (usize, String, bool)> = HashMap::new();
    for name in m.opaque.iter().filter(|n| !RUNTIME_TYPES.iter().any(|r| r.0 == n.as_str()) && *n != "Error" && *n != "Date" && super::super::model::buffer_spec(n).is_none()) {
        let arity = m.opaque_arity.get(name).copied().unwrap_or(0);
        if arity == 0 && !m.namespaced.contains_key(name.as_str()) {
            let orig = m.opaque_names.get(name).map(String::as_str).unwrap_or(name.as_str());
            let rn_orig = rust_type(orig);
            for (idx, r) in refs.iter().enumerate() {
                if let Some(&(0, is_handle)) = r.types.get(&rn_orig) {
                    resolved_opaque.insert(name.clone(), (idx, format!("::{}::{rn_orig}", r.crate_name), is_handle));
                    break;
                }
            }
        }
    }
    let leaves: HashSet<String> = own.values().cloned().collect();
    let cx = Context {
        decodable: m.unions.iter().filter(|u| u.decodable()).map(|u| u.name.clone()).collect(),
        unions: m.unions.iter().map(|u| u.name.clone()).collect(),
        classes: m.classes.iter().filter(|c| c.type_params.is_empty()).map(|c| c.name.clone()).chain(m.opaque.iter().filter(|n| !m.opaque_arity.contains_key(*n)).filter(|n| resolved_opaque.get(*n).is_none_or(|x| x.2)).cloned()).collect(),
        types,
        own,
        leaves,
    };
    // What a namespace's module defines, by its path.
    let mut namespaces: BTreeMap<Vec<String>, String> = BTreeMap::new();
    let mut imports: BTreeMap<String, String> = BTreeMap::new();
    let mut types_file = String::from(PRELUDE);

    for (at, (model_name, values)) in m.enums.iter().enumerate() {
        let name = cx.def_name(model_name);
        let docs = m.enum_docs.get(at);
        let mut seen = HashSet::new();
        let variants: Vec<(String, String)> = values
            .iter()
            .map(|v| {
                let id = camel(&super::super::model::ident(if v.is_empty() { "empty" } else { v }));
                // Nothing of it is a name (`"\\"`, `"/"`).
                let mut id = if id.trim_matches('_').is_empty() { "Value".to_string() } else { id };
                if !seen.insert(id.clone()) {
                    let mut i = 2;
                    while !seen.insert(format!("{id}{i}")) {
                        i += 1;
                    }
                    id = format!("{id}{i}");
                }
                (id, v.clone())
            })
            .collect();
        let index: String = variants.iter().enumerate().map(|(i, (v, _))| format!("{i} => {name}::{v}, ")).collect();
        let from: String = variants.iter().map(|(v, s)| format!("{s:?} => {name}::{v}, ")).collect();
        let to: String = variants.iter().map(|(v, s)| format!("{name}::{v} => {s:?}, ")).collect();
        let out = match module_of.get(model_name) {
            Some(path) => namespaces.entry(path.clone()).or_default(),
            None => &mut types_file,
        };
        *out += &docs.and_then(|d| d.doc.as_deref()).map(|d| comment(d, "///", "", Style::Rust)).unwrap_or_default();
        *out += &format!(
            "#[derive(Clone, Copy, PartialEq, Eq, Debug)]\n#[repr(i32)]\npub enum {name} {{ {} }}\n\
             impl AsRef<{name}> for {name} {{ fn as_ref(&self) -> &Self {{ self }} }}\n\
             impl {name} {{\n    pub fn from_index(i: i32) -> Self {{ match i {{ {index}_ => panic!(\"{name}: no value {{i}}\") }} }}\n    pub fn as_str(self) -> &'static str {{ match self {{ {to}}} }}\n}}\n\
             impl ToJs for {name} {{ fn lend(&self) -> Lent {{ self.as_str().lend() }} }}\n\
             impl FromJs for {name} {{ fn from_handle(h: u32) -> Self {{ let s = String::from_handle(h); match s.as_str() {{ {from}_ => panic!(\"{name}: no value {{s}}\") }} }} }}\n",
            variant_list(variants.iter().map(|(v, _)| v.clone()).enumerate().map(|(i, v)| (v, docs.and_then(|d| d.members.get(i)?.as_deref()))))
        );
    }
    for (at, (model_name, members)) in m.num_enums.iter().enumerate() {
        let name = cx.def_name(model_name);
        let docs = m.num_enum_docs.get(at);
        let mut seen = HashSet::new();
        let all: Vec<(String, &String, usize)> = members
            .iter()
            .enumerate()
            .map(|(n, (member, value))| {
                let mut id = camel_variant(&super::super::model::ident(member));
                if !seen.insert(id.clone()) {
                    let mut i = 2;
                    while !seen.insert(format!("{id}{i}")) {
                        i += 1;
                    }
                    id = format!("{id}{i}");
                }
                (id, value, n)
            })
            .collect();
        // A value once a variant; a member of a value given already
        // (`ROTATE = 0` beside `LEFT = 0`) is a constant of that variant.
        let mut variants: Vec<(String, &String)> = Vec::new();
        let mut variant_docs: Vec<Option<&str>> = Vec::new();
        let mut aliases: Vec<String> = Vec::new();
        for (v, value, n) in all {
            match variants.iter().find(|(_, w)| *w == value) {
                Some((first, _)) => aliases.push(format!("pub const {v}: {name} = {name}::{first}; ")),
                None => {
                    variants.push((v, value));
                    variant_docs.push(docs.and_then(|d| d.members.get(n)?.as_deref()));
                }
            }
        }
        let from: String = variants.iter().map(|(v, value)| format!("{value} => {name}::{v}, ")).collect();
        let out = match module_of.get(model_name) {
            Some(path) => namespaces.entry(path.clone()).or_default(),
            None => &mut types_file,
        };
        *out += &docs.and_then(|d| d.doc.as_deref()).map(|d| comment(d, "///", "", Style::Rust)).unwrap_or_default();
        *out += &format!(
            "#[derive(Clone, Copy, PartialEq, Eq, Debug)]\n#[repr(i32)]\npub enum {name} {{ {} }}\n\
             impl AsRef<{name}> for {name} {{ fn as_ref(&self) -> &Self {{ self }} }}\n\
             #[allow(non_upper_case_globals)]\nimpl {name} {{ {}pub fn from_value(v: i32) -> Self {{ match v {{ {from}_ => panic!(\"{name}: no value {{v}}\") }} }} }}\n\
             impl ToJs for {name} {{ fn lend(&self) -> Lent {{ (*self as i32 as f64).lend() }} }}\n\
             impl FromJs for {name} {{ fn from_handle(h: u32) -> Self {{ Self::from_value(f64::from_handle(h) as i32) }} }}\n",
            variant_list(variants.iter().zip(&variant_docs).map(|((v, value), d)| (format!("{v} = {value}"), *d))),
            aliases.concat()
        );
    }
    let mut used_refs: BTreeSet<usize> = BTreeSet::new();
    let mut impl_refs: HashSet<String> = HashSet::new();
    // Types declared elsewhere: resolve against referenced crates where possible, else opaque handles.
    for name in m.opaque.iter().filter(|n| !RUNTIME_TYPES.iter().any(|r| r.0 == n.as_str())) {
        if let Some((ref_idx, qualified, is_handle)) = resolved_opaque.get(name) {
            used_refs.insert(*ref_idx);
            let ref_crate = &refs[*ref_idx].crate_name;
            let short = qualified.trim_start_matches("::");
            let local_name = cx.def_name(name);
            let reexport = if short == format!("{ref_crate}::{local_name}") {
                format!("pub use {qualified};\n")
            } else {
                format!("pub use {qualified} as {local_name};\n")
            };
            types_file += &reexport;
            if impl_refs.insert(qualified.clone()) {
                types_file += &format!(
                    "impl crate::runtime::ToJs for {qualified} {{\n\
                     \x20   fn lend(&self) -> crate::runtime::Lent {{ let (h, owned) = ::{ref_crate}::ToJs::lend(self).into_raw(); crate::runtime::Lent::from_raw(h, owned) }}\n\
                     }}\n\
                     impl crate::runtime::FromJs for {qualified} {{\n\
                     \x20   fn from_handle(h: u32) -> Self {{ <{qualified} as ::{ref_crate}::FromJs>::from_handle(h) }}\n\
                     }}\n\
                     impl From<{qualified}> for crate::runtime::JsValue {{ fn from(x: {qualified}) -> Self {{ Self(crate::runtime::JsHandle(crate::runtime::give(&x))) }} }}\n\
                     impl From<&{qualified}> for crate::runtime::JsValue {{ fn from(x: &{qualified}) -> Self {{ Self(crate::runtime::JsHandle(crate::runtime::give(x))) }} }}\n"
                );
                if *is_handle {
                    types_file += &format!("impl crate::runtime::Is<{qualified}> for {qualified} {{}}\n");
                }
            }
            continue;
        }
        let arity = m.opaque_arity.get(name).copied().unwrap_or(0);
        let params: Vec<String> = (1..=arity).map(|i| format!("T{i}")).collect();
        let raw_name = name.as_str();
        let name = cx.def_name(name);
        let targs = if params.is_empty() { String::new() } else { format!("<{}>", params.join(", ")) };
        let phantom = if params.is_empty() { "" } else { ", PhantomData" };
        if raw_name != "Error" && raw_name != "Date" && super::super::model::buffer_spec(raw_name).is_none() {
            types_file += "// external\n";
        }
        types_file += &format!("handle!({name}{targs});\n");
        if params.is_empty() {
            types_file += &format!("impl Is<{name}> for {name} {{}}\n");
        }
        if raw_name == "Error" && params.is_empty() {
            types_file += "impl Error {\n\
                           pub fn new(msg: &str) -> Self { Self(JsHandle(unsafe { crate::runtime::rt::js_error_new(msg.as_ptr(), msg.len()) })) }\n\
                           pub fn name(&self) -> ::std::string::String { <::std::string::String>::from_handle(unsafe { crate::runtime::rt::js_error_name(self.0 .0) }) }\n\
                           pub fn message(&self) -> ::std::string::String { <::std::string::String>::from_handle(unsafe { crate::runtime::rt::js_error_message(self.0 .0) }) }\n\
                           pub fn stack(&self) -> ::std::string::String { <::std::string::String>::from_handle(unsafe { crate::runtime::rt::js_error_stack(self.0 .0) }) }\n\
                           }\n\
                           impl From<JsError> for Error { fn from(e: JsError) -> Self { Self(e.0) } }\n\
                           impl From<Error> for JsError { fn from(e: Error) -> Self { Self(e.0) } }\n";
        } else if raw_name == "Date" && params.is_empty() {
            types_file += "impl Date {\n\
                           pub fn from_time(ms: f64) -> Self { Self(JsHandle(unsafe { crate::runtime::rt::js_date_new(ms) })) }\n\
                           pub fn now() -> f64 { unsafe { crate::runtime::rt::js_date_now() } }\n\
                           pub fn time(&self) -> f64 { unsafe { crate::runtime::rt::js_date_time(self.0 .0) } }\n\
                           pub fn to_iso_string(&self) -> ::std::string::String { <::std::string::String>::from_handle(unsafe { crate::runtime::rt::js_date_to_iso_string(self.0 .0) }) }\n\
                           }\n";
        } else if let Some(s) = super::super::model::buffer_spec(raw_name) {
            types_file += &format!(
                "impl{targs} {name}{targs} {{\n\
                 pub fn byte_len(&self) -> usize {{ unsafe {{ crate::runtime::rt::js_bytes_len(self.0 .0) as usize }} }}\n\
                 pub fn read_bytes(&self, offset: usize, dst: &mut [u8]) -> usize {{ unsafe {{ crate::runtime::rt::js_bytes_read(self.0 .0, offset, dst.as_mut_ptr(), dst.len()) as usize }} }}\n\
                 pub fn write_bytes(&self, offset: usize, src: &[u8]) -> usize {{ unsafe {{ crate::runtime::rt::js_bytes_write(self.0 .0, offset, src.as_ptr(), src.len()) as usize }} }}\n"
            );
            if s.has_buffer() {
                types_file += "pub fn byte_offset(&self) -> usize { unsafe { crate::runtime::rt::js_bytes_offset(self.0 .0) as usize } }\n\
                               pub fn buffer(&self) -> JsValue { JsValue(JsHandle(unsafe { crate::runtime::rt::js_bytes_buffer(self.0 .0) })) }\n";
            }
            if s.is_typed_array() {
                let kind = s.kind.unwrap();
                let elem = rust_elem(s.elem);
                let eb = s.elem.bytes();
                types_file += &format!(
                    "pub fn new(len: usize) -> Self {{ Self(JsHandle(unsafe {{ crate::runtime::rt::js_bytes_new({kind}, core::ptr::null(), len.saturating_mul({eb})) }}){phantom}) }}\n\
                     pub fn from_slice(data: &[{elem}]) -> Self {{ Self(JsHandle(unsafe {{ crate::runtime::rt::js_bytes_new({kind}, data.as_ptr() as *const u8, data.len().saturating_mul({eb})) }}){phantom}) }}\n\
                     pub fn len(&self) -> usize {{ self.byte_len() / {eb} }}\n\
                     pub fn is_empty(&self) -> bool {{ self.len() == 0 }}\n\
                     pub fn read(&self, elem_offset: usize, dst: &mut [{elem}]) -> usize {{ let Some(off) = elem_offset.checked_mul({eb}) else {{ return 0; }}; unsafe {{ crate::runtime::rt::js_bytes_read(self.0 .0, off, dst.as_mut_ptr() as *mut u8, dst.len().saturating_mul({eb})) as usize / {eb} }} }}\n\
                     pub fn write(&self, elem_offset: usize, src: &[{elem}]) -> usize {{ let Some(off) = elem_offset.checked_mul({eb}) else {{ return 0; }}; unsafe {{ crate::runtime::rt::js_bytes_write(self.0 .0, off, src.as_ptr() as *const u8, src.len().saturating_mul({eb})) as usize / {eb} }} }}\n\
                     pub fn to_vec(&self) -> ::std::vec::Vec<{elem}> {{ let mut out = vec![Default::default(); self.len()]; self.read(0, &mut out); out }}\n"
                );
            } else {
                if let Some(kind) = s.kind {
                    types_file += &format!(
                        "pub fn new(byte_len: usize) -> Self {{ Self(JsHandle(unsafe {{ crate::runtime::rt::js_bytes_new({kind}, core::ptr::null(), byte_len) }}){phantom}) }}\n\
                         pub fn from_bytes(data: &[u8]) -> Self {{ Self(JsHandle(unsafe {{ crate::runtime::rt::js_bytes_new({kind}, data.as_ptr(), data.len()) }}){phantom}) }}\n"
                    );
                }
                types_file += "pub fn read(&self, offset: usize, dst: &mut [u8]) -> usize { self.read_bytes(offset, dst) }\n\
                               pub fn write(&self, offset: usize, src: &[u8]) -> usize { self.write_bytes(offset, src) }\n\
                               pub fn to_vec(&self) -> ::std::vec::Vec<u8> { let mut out = vec![0u8; self.byte_len()]; self.read_bytes(0, &mut out); out }\n";
            }
            types_file += "}\n";
        }
    }
    // Unions that can be decoded: enums.
    let mut alias_names: HashSet<String> = HashSet::new();
    for u in m.unions.iter().filter(|u| u.decodable()) {
        let mut text = union_enum(u, m, &mut imports, &cx);
        if u.type_params.is_empty() {
            // A library's name for it, where that is free.
            for (alias, _) in m.type_aliases.iter().filter(|(_, to)| *to == u.name) {
                let a = rust_type(alias);
                if !declared.iter().any(|d| cx.def_name(d) == a) && alias_names.insert(a.clone()) {
                    text += &format!("pub type {a} = {};\n", cx.def_name(&u.name));
                }
            }
        }
        match module_of.get(&u.name) {
            Some(path) => *namespaces.entry(path.clone()).or_default() += &text,
            None => types_file += &text,
        }
    }
    for c in &m.callbacks {
        let apply = m.funcs.iter().find(|f| {
            f.call == Call::Apply
                && f.params.first().is_some_and(|(_, t)| matches!(t, Ty::Obj(n) | Ty::Generic(n, _) if n == &c.name))
        });
        let text = callback(&c.name, &c.type_params, &c.params, &c.ret, apply, &mut imports, &cx);
        match module_of.get(&c.name) {
            Some(path) => *namespaces.entry(path.clone()).or_default() += &text,
            None => types_file += &text,
        }
    }

    let mut modules: BTreeMap<String, String> = BTreeMap::new();
    emit_classes(m, &cx, &declared, &resolved_opaque, &module_of, &mut namespaces, &mut modules, &mut imports);

    let function_names = names_of(m.funcs.iter());
    // A namespace's function is in its module, by its own name:
    // `stream::finished`, `intl::date_time_format`.
    let mut taken_in: HashMap<Vec<String>, HashSet<String>> = HashMap::new();
    for f in &m.funcs {
        let (out, name) = match m.func_namespaces.get(&f.import) {
            Some((path, own, _, _)) => {
                let segments: Vec<String> = path.iter().map(|s| ns_module(s)).collect();
                let base = rust_function(f, own);
                let taken = taken_in.entry(segments.clone()).or_default();
                let mut name = base.clone();
                let mut i = 2;
                while !taken.insert(name.clone()) {
                    name = format!("{base}_{i}");
                    i += 1;
                }
                (namespaces.entry(segments).or_default(), name)
            }
            None => (modules.entry(module_file(&f.module)).or_insert_with(|| PRELUDE.to_string()), function_names[&f.name].clone()),
        };
        for item in function(f, &name, "pub ", &mut imports, None, &cx) {
            *out += &format!("{item}\n");
        }
    }

    let deps = if used_refs.is_empty() {
        String::new()
    } else {
        let lines: String = used_refs
            .iter()
            .map(|&idx| format!("{} = {{ path = {:?} }}\n", refs[idx].crate_name, refs[idx].dep_path))
            .collect();
        format!("\n[dependencies]\n{lines}")
    };
    let mut lib = format!(
        "// Generated. Bindings: `use {crate_name}::*;`.\n#![allow(non_camel_case_types, non_snake_case, clippy::all)]\n\nmod raw;\nmod runtime;\npub use runtime::*;\nmod types;\npub use types::*;\n"
    );
    let mut out = vec![
        ("Cargo.toml".to_string(), format!("# Generated.\n[package]\nname = \"{crate_name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n{deps}")),
        ("src/runtime.rs".to_string(), RUNTIME.to_string()),
        ("src/types.rs".to_string(), types_file),
    ];
    // A namespace's module, by its own name (`pub mod stream;`), not taken
    // into the crate's: its types are `stream::PipeOptions`. Its members'
    // traits are in scope with the crate's, under no name.
    let tops: BTreeSet<&String> = namespaces.keys().filter_map(|k| k.first()).collect();
    for top in tops {
        let mut text = PRELUDE.to_string();
        fn nest(at: &[String], namespaces: &BTreeMap<Vec<String>, String>, text: &mut String) {
            *text += namespaces.get(at).map(String::as_str).unwrap_or_default();
            let children: BTreeSet<&String> = namespaces.keys().filter(|k| k.len() > at.len() && k[..at.len()] == *at).map(|k| &k[at.len()]).collect();
            for child in children {
                let mut path = at.to_vec();
                path.push(child.clone());
                *text += &format!("\npub mod {child} {{\n{}", PRELUDE.replace("// Generated.\n#![allow(unused_imports)]\n", "#[allow(unused_imports)]\nuse crate::*;\n"));
                nest(&path, namespaces, text);
                *text += "}\n";
            }
        }
        nest(std::slice::from_ref(top), &namespaces, &mut text);
        lib += &format!("pub mod {top};\n");
        out.push((format!("src/{top}.rs"), text));
    }
    for c in m.classes.iter().filter(|c| module_of.contains_key(&c.name)) {
        lib += &format!("#[allow(unused_imports)]\npub use {}Methods as _;\n", cx.tn(&c.name));
    }
    for (module, text) in modules {
        // A module named as one of the crate's own files gets another name.
        let module = if matches!(module.as_str(), "raw" | "runtime" | "types" | "lib") { format!("{module}_") } else { module };
        lib += &format!("mod {module};\npub use {module}::*;\n");
        out.push((format!("src/{module}.rs"), text));
    }
    out.push(("src/raw.rs".into(), format!("// Generated. The imports.\n#![allow(dead_code, unused_imports)]\nuse core::ffi::c_void;\n\n#[link(wasm_import_module = \"{wasm_module}\")]\nunsafe extern \"C\" {{\n{}}}\n", imports.into_values().collect::<String>())));
    out.push(("src/lib.rs".into(), lib));
    let mut rust_types = BTreeMap::new();
    let is_top_exported = |rn: &str| !matches!(rn, "JsValue" | "Error" | "Date") && super::super::model::buffer_spec(rn).is_none();
    for c in &m.classes {
        if !module_of.contains_key(&c.name) {
            let rn = cx.def_name(&c.name);
            if is_top_exported(&rn) {
                rust_types.insert(rn, (c.type_params.len(), true));
            }
        }
    }
    for c in &m.callbacks {
        if !module_of.contains_key(&c.name) {
            let rn = cx.def_name(&c.name);
            if is_top_exported(&rn) {
                rust_types.insert(rn, (c.type_params.len(), true));
            }
        }
    }
    for (model_name, _) in &m.enums {
        if !module_of.contains_key(model_name) {
            rust_types.entry(cx.def_name(model_name)).or_insert((0, false));
        }
    }
    for (model_name, _) in &m.num_enums {
        if !module_of.contains_key(model_name) {
            rust_types.entry(cx.def_name(model_name)).or_insert((0, false));
        }
    }
    for u in m.unions.iter().filter(|u| u.decodable() && u.type_params.is_empty()) {
        if !module_of.contains_key(&u.name) {
            rust_types.entry(cx.def_name(&u.name)).or_insert((0, false));
        }
    }
    let meta = super::super::pkg_meta::PkgMeta {
        target: "rust".into(),
        package: crate_name.to_string(),
        entry: Some("Cargo.toml".into()),
        rust_types,
        ..Default::default()
    };
    out.push((super::super::pkg_meta::META_FILE.into(), meta.to_toml()));
    Ok(out)
}
