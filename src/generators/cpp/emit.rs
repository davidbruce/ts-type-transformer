use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use super::super::comments::{Style, comment};
use super::decl::{
    declarations, decoder_guard_id, decoder_owner, level_names, lower_callback, namespace_aliases,
    namespace_containers, nested_name, raise_callback, union_type, Container,
};
use super::types::{
    constructors, cpp_name, cpp_name_as, exact_params_for, func_subsumed_by, function_variants,
    head, is_adapted_param, named_types, param_types, raise, raw_ret, rest_pack_info, ty_arg,
    ty_in, ty_out, ty_param, Place,
};
use super::{ident, import_attr, is_builtin_cpp_type, raw_fn, wrap_ns, Ctx, ProjectRef, RUNTIME};
use super::super::model::{base_args, map_types, subst, with_self_args, Call, Class, Func, Model, Ty};

pub(super) fn files_named(m: &Model, pkg_name: &str, refs: &[ProjectRef]) -> Result<Vec<(String, String)>, Vec<String>> {
    let wasm_module = if pkg_name == "bindings" { "env" } else { pkg_name };
    let raw_prefix = if pkg_name == "bindings" { String::new() } else { format!("{pkg_name}_") };
    let ns: Option<&str> = (pkg_name != "bindings").then_some(pkg_name);
    let ns_prefix = if pkg_name == "bindings" { String::new() } else { format!("{pkg_name}::") };
    let inc_prefix = if pkg_name == "bindings" { String::new() } else { format!("{pkg_name}/") };

    let unions = &m.unions;
    let decodable: BTreeSet<String> = unions.iter().filter(|u| u.decodable() && u.type_params.is_empty()).map(|u| u.name.clone()).collect();
    let resolved_opaque: HashMap<String, (usize, String)> = m
        .opaque
        .iter()
        .filter(|n| !is_builtin_cpp_type(n))
        .filter(|n| m.opaque_arity.get(*n).copied().unwrap_or(0) == 0)
        .filter_map(|n| {
            let orig = m.opaque_names.get(n).map(String::as_str).unwrap_or(n.as_str());
            refs.iter().enumerate().find(|(_, r)| r.types.contains(orig)).map(|(i, _)| ((*n).clone(), (i, orig.to_string())))
        })
        .collect();
    let resolved_ref: BTreeSet<String> = resolved_opaque.keys().cloned().collect();
    // A constructor is named as its class, and hides nothing.
    let members: BTreeSet<String> = m.classes.iter().flat_map(|c| c.funcs.iter().filter(|f| !matches!(f.call, Call::New(_) | Call::Create(_))).map(|f| cpp_name(f, &c.name))).collect();
    let qualified: BTreeSet<String> = members.into_iter().filter(|n| m.classes.iter().any(|c| c.name == *n) || m.callbacks.iter().any(|c| c.name == *n) || m.unions.iter().any(|u| u.name == *n) || m.enums.iter().any(|e| e.0 == *n) || m.num_enums.iter().any(|e| e.0 == *n)).collect();
    let cx = Ctx { decodable, qualified, ns_prefix: ns_prefix.clone(), resolved_ref };

    let mut taken = HashMap::new();
    // Where each declared type is defined, without the extension.
    let mut home: HashMap<String, String> = HashMap::new();
    for c in &m.classes {
        home.insert(c.name.clone(), super::super::model::file_name(&format!("{}/{}", c.module, c.name), &mut taken));
    }
    for name in m.callbacks.iter().map(|c| &c.name).chain(m.unions.iter().map(|u| &u.name)) {
        home.insert(name.clone(), super::super::model::file_name(&format!("types/{name}"), &mut taken));
    }
    // `#include` from one generated file to another, by its path from the
    // bindings' root (compiled with `-I` on it): relative to the including
    // file, a long chain of includes makes paths too long for the system,
    // as the compiler joins them without resolving `..`.
    let include = |_from: &str, to: &str| format!("#include \"{inc_prefix}{to}\"\n");
    // The headers of the given types.
    // What a union holds, by name: a value of it needs each of them complete.
    let union_parts: HashMap<&str, BTreeSet<String>> = unions
        .iter()
        .map(|u| {
            let mut parts = BTreeSet::new();
            u.parts.iter().for_each(|p| named_types(p, &mut parts));
            (u.name.as_str(), parts)
        })
        .collect();
    let classes: BTreeSet<&str> = m.classes.iter().map(|c| c.name.as_str()).collect();
    // First the structs of the classes used, and of those the unions used
    // hold: declarations include only declarations, so each is complete
    // before any definition, however the full headers include each other.
    // Then the full headers.
    let includes = |from: &str, types: &BTreeSet<String>| -> String {
        let mut all = types.clone();
        let mut todo: Vec<String> = types.iter().cloned().collect();
        while let Some(t) = todo.pop() {
            for p in union_parts.get(t.as_str()).into_iter().flatten() {
                if all.insert(p.clone()) {
                    todo.push(p.clone());
                }
            }
        }
        let decls = all.iter().filter(|t| classes.contains(t.as_str())).filter_map(|t| home.get(t)).map(|h| format!("{h}.decl.hpp"));
        let full = types.iter().filter_map(|t| home.get(t)).map(|h| format!("{h}.hpp"));
        decls.chain(full).filter(|h| h != from).map(|h| include(from, &h)).collect()
    };

    // A namespace's types, also by their own names in it.
    let (namespaces, in_structs) = namespace_aliases(m, &ns_prefix, &cx);
    // A namespace's functions where it is a class's: its statics, by their own names.
    let containers = namespace_containers(m);
    let levels = level_names(m);
    let mut class_statics: HashMap<String, Vec<Func>> = HashMap::new();
    let mut class_statics_of: HashSet<String> = HashSet::new();
    for f in &m.funcs {
        if let Some((path, own, _, _)) = m.func_namespaces.get(&f.import)
            && path.len() == 1
            && let Some(Container::Class(c)) = containers.get(&path[0])
        {
            class_statics.entry(c.clone()).or_default().push(Func { name: own.clone(), ..(*f).clone() });
            class_statics_of.insert(f.import.clone());
        }
    }
    let types = declarations(m, pkg_name, refs, &resolved_opaque, &cx) + &wrap_ns(ns, &namespaces);
    let mut out = vec![("ts_runtime.hpp".to_string(), RUNTIME.to_string()), ("ts_types.hpp".to_string(), types)];

    // A union: its alternatives, and how to tell which one came back.
    for u in unions {
        let path = format!("{}.hpp", home[&u.name]);
        let mut parts = BTreeSet::new();
        u.parts.iter().for_each(|p| named_types(p, &mut parts));
        parts.remove(&u.name);
        let tag_raw = raw_fn(&raw_prefix, &format!("{}_tag", u.name));
        let tag_attr = import_attr(wasm_module, &format!("{}_tag", u.name));
        let mut text = format!("// Generated.\n#pragma once\n{}{}", include(&path, "ts_types.hpp"), includes(&path, &parts));
        text += &format!("extern \"C\" {tag_attr} int32_t {tag_raw}(uint32_t, int32_t);\n");
        let mut decode_ns = String::new();
        let mut decoder_ext = String::new();
        if u.decodable() {
            let name = union_type(u);
            let cases: Vec<String> = u
                .parts
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let value = match p {
                        Ty::Num => "ts::from_handle<double>(h)".to_string(),
                        Ty::Bool => "ts::from_handle<bool>(h)".to_string(),
                        Ty::Enum(e) => format!("ts::from_handle<{e}>(h)"),
                        p => raise(p, "h", unions, &cx),
                    };
                    format!("        case {i}: return {name}(std::in_place_index<{i}>, {value});\n")
                })
                .collect();
            // A bare type parameter is told apart by its argument; with another
            // member no test can recognise, only a number, string or flag can be.
            let param = u.parts.iter().find_map(|p| if let Ty::Param(t) = p { Some(t.clone()) } else { None });
            let kind = param.as_ref().map_or("0".to_string(), |t| format!("ts::kind_of<{t}>"));
            let check = match &param {
                Some(t) if u.tests.iter().zip(&u.parts).any(|(test, p)| test.is_none() && !matches!(p, Ty::Param(_))) => {
                    format!("    static_assert(ts::kind_of<{t}> != 0, \"{t} must be a number, string or flag to be told apart\");\n")
                }
                _ => String::new(),
            };
            decode_ns += &format!(
                "{}inline {name} decode_{}(uint32_t h) {{\n{check}    switch ({tag_raw}(h, {kind})) {{\n{2}    }}\n    __builtin_trap();\n}}\n",
                head(&u.type_params, false, &cx),
                u.name,
                cases.concat()
            );
            if decoder_owner(u, unions, &cx) {
                let guard = decoder_guard_id(u, m, pkg_name, &resolved_opaque, refs, &cx);
                decoder_ext += &format!(
                    "#ifndef TS_DECODER_DEF_{guard}\n#define TS_DECODER_DEF_{guard}\n\
                     inline ::{ns_prefix}{0} ts::decoder<::{ns_prefix}{0}>::from(uint32_t h) {{ return ::{ns_prefix}decode_{0}(h); }}\n\
                     #endif\n",
                    u.name
                );
            }
        }
        text += &wrap_ns(ns, &decode_ns);
        text += &decoder_ext;
        out.push((path, text));
    }
    // Per module: headers, and the definitions compiled once.
    let mut modules: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut library: BTreeMap<String, (BTreeSet<String>, String)> = BTreeMap::new();
    let mut extra_lib_headers: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let callbacks: HashSet<&str> = m.callbacks.iter().map(|c| c.name.as_str()).collect();
    let mut callback_apply_imports: HashSet<String> = HashSet::new();

    // A callback, made from a lambda. Its arguments are made only once the
    // lambda's type is known, so the types they name need not be complete yet.
    for c in &m.callbacks {
        let path = format!("{}.hpp", home[&c.name]);
        let mut used = BTreeSet::new();
        c.params.iter().for_each(|p| named_types(&p.1, &mut used));
        named_types(&c.ret, &mut used);
        used.remove(&c.name);
        let raw_args: String = c.params.iter().enumerate().map(|(i, (_, t))| format!(", {} a{i}", raw_ret(t))).collect();
        let args: Vec<String> = c.params.iter().enumerate().map(|(i, (_, t))| raise_callback(t, &format!("a{i}"), unions, &cx)).collect();
        let call = format!("ts::closure_fn<F>(ctx)({})", args.join(", "));
        let cb_args: Vec<String> = c.params.iter().map(|(_, t)| format!("ts::dep<F, {}>", ty_out(t, unions, &cx))).collect();
        let arg_list = if cb_args.is_empty() { String::new() } else { format!(", {}", cb_args.join(", ")) };
        let (req, result) = match &c.ret {
            Ty::Void => (
                format!("ts::callable_candidate<F> && std::invocable<F&{arg_list}>"),
                format!("{call};"),
            ),
            Ty::Nullable(inner, _) => {
                let absent = if matches!(**inner, Ty::Bool | Ty::Enum(_)) { "-1" } else { "0" };
                (
                    format!("ts::callable_candidate<F> && std::invocable<F&{arg_list}>"),
                    format!("if constexpr (std::is_void_v<std::invoke_result_t<F&{arg_list}>>) {{ {call}; return {absent}; }} else {{ return {}; }}", lower_callback(&c.ret, &call, &cx)),
                )
            }
            t => (
                format!("ts::callable_candidate<F> && std::invocable<F&{arg_list}> && !std::is_void_v<std::invoke_result_t<F&{arg_list}>>"),
                format!("return {};", lower_callback(t, &call, &cx)),
            ),
        };
        let (name, ret) = (&c.name, raw_ret(&c.ret));
        let tpl = head(&c.type_params, false, &cx);
        let owner = if c.type_params.is_empty() {
            c.name.clone()
        } else {
            format!("{}<{}>", c.name, c.type_params.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(", "))
        };
        let cb_from = format!("{name}_from");
        let cb_raw = raw_fn(&raw_prefix, &cb_from);
        let cb_attr = import_attr(wasm_module, &cb_from);
        let mut cb_imports = BTreeSet::new();
        cb_imports.insert(format!("extern \"C\" {cb_attr} uint32_t {cb_raw}(uintptr_t, void*);\n"));
        let mut op_decls = String::new();
        let mut op_header_defs = String::new();
        let mut op_seen = BTreeSet::new();
        for f in m.funcs.iter().filter(|f| {
            f.call == Call::Apply
                && f.params.first().is_some_and(|(_, t)| matches!(t, Ty::Obj(n) | Ty::Generic(n, _) if n == &c.name))
        }) {
            callback_apply_imports.insert(f.import.clone());
            let mut op_f = f.clone();
            op_f.call = Call::Invoke;
            op_f.params = f.params[1..].to_vec();
            op_f.type_params = f.type_params.get(c.type_params.len()..).unwrap_or(&[]).to_vec();
            for func in function_variants(&op_f, "operator()".into(), None, &owner, &tpl, Place::Member, unions, &callbacks, wasm_module, &raw_prefix, &HashSet::new(), &cx) {
                if !op_seen.insert(func.key) {
                    continue;
                }
                op_decls += &func.decl;
                used.extend(func.refs);
                for t in &op_f.type_params {
                    param_types(std::slice::from_ref(t), &mut used);
                }
                if func.templated {
                    op_header_defs += &func.def;
                    cb_imports.insert(func.import);
                } else {
                    let lib = library.entry(f.module.clone()).or_default();
                    lib.0.insert(func.import);
                    lib.1 += &func.def;
                    extra_lib_headers.entry(f.module.clone()).or_default().insert(path.clone());
                }
            }
        }
        used.remove(&c.name);
        let body = wrap_ns(
            ns,
            &format!(
                "{tpl}struct {name} : ts::Callback {{\n{}    \
                 template <class F>\n        requires ({req})\n    \
                 {name}(F f) : ts::Handle(ts::handle, {cb_raw}(reinterpret_cast<uintptr_t>(&call<F>), ts::closure(std::move(f)))) {{}}\n\
                 {op_decls}\n  \
                 private:\n    template <class F>\n    static {ret} call(void* ctx{raw_args}) {{ {result} }}\n}};\n",
                constructors(name),
            ),
        );
        let mut text = format!(
            "// Generated.\n#pragma once\n{}{}\n{body}\n{}",
            include(&path, "ts_types.hpp"),
            cb_imports.into_iter().collect::<String>(),
            includes(&path, &used)
        );
        if !op_header_defs.is_empty() {
            text += &format!("\n{}", wrap_ns(ns, &op_header_defs));
        }
        out.push((path, text));
    }

    // Each class: its struct, its header, and its definitions.
    let ancestors = |name: &str| -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut todo = vec![name.to_string()];
        while let Some(c) = todo.pop() {
            for a in m.classes.iter().filter(|k| k.name == c).flat_map(|k| &k.ancestors) {
                if !out.contains(a) {
                    out.push(a.clone());
                    todo.insert(0, a.clone());
                }
            }
        }
        out
    };
    let member_names = |c: &Class| -> BTreeSet<String> { c.funcs.iter().filter(|f| Class::has_self(f)).map(|f| cpp_name(f, "create")).collect() };
    for c in &m.classes {
        let base = &home[&c.name];
        let (decl_path, impl_path) = (format!("{base}.decl.hpp"), format!("{base}.hpp"));
        // A class template is written `Box<T>` where its members are defined.
        let class_head = head(&c.type_params, false, &cx);
        let owner = if c.type_params.is_empty() { c.name.clone() } else { format!("{}<{}>", c.name, c.type_params.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(", ")) };
        // A member named `create` leaves the constructors `make`.
        let ctor = if c.funcs.iter().any(|f| !matches!(f.call, Call::New(_) | Call::Create(_)) && cpp_name(f, "") == "create") { "make" } else { "create" };

        // A name in two unrelated bases is ambiguous in C++. A more derived
        // ancestor's member already hides its own ancestors' (`Element::textContent`
        // over `Node::textContent`), as in TypeScript; the others are declared
        // again here, with the base's type arguments, and hide both.
        let all = ancestors(&c.name);
        let own = member_names(c);
        let mut holders: BTreeMap<String, Vec<&String>> = BTreeMap::new();
        for a in &all {
            if let Some(k) = m.classes.iter().find(|k| k.name == *a) {
                for n in member_names(k) {
                    holders.entry(n).or_default().push(a);
                }
            }
        }
        let mut again: Vec<Func> = Vec::new();
        // A name the type declares itself hides its ancestors' overloads in C++,
        // where TypeScript merges them: `using Base::name;` brings them back, and
        // an own overload of the same signature still hides its base's.
        let clashes_adapted = |bf: &Func, cf: &Func| -> bool {
            if bf.rest && cf.rest && bf.params.len() == cf.params.len()
                && rest_pack_info(bf, &cx).map(|r| r.elem_ty) != rest_pack_info(cf, &cx).map(|r| r.elem_ty)
            {
                return true;
            }
            if bf.params.len() != cf.params.len() {
                return false;
            }
            let mut diff = false;
            for ((_, bt), (_, ct)) in bf.params.iter().zip(&cf.params) {
                let b_ty = ty_param(bt, unions, &callbacks, &cx);
                let c_ty = ty_param(ct, unions, &callbacks, &cx);
                if b_ty == c_ty {
                    continue;
                }
                let b_ad = is_adapted_param(bt, unions, &callbacks);
                let c_ad = is_adapted_param(ct, unions, &callbacks);
                let b_opt = matches!(bt, Ty::Nullable(..));
                let c_opt = matches!(ct, Ty::Nullable(..));
                if (b_ad && c_ad)
                    || ((b_ad || b_opt) && *ct == Ty::Any)
                    || ((c_ad || c_opt) && *bt == Ty::Any)
                    || (matches!(bf.call, Call::Set(_)) && bf.call == cf.call)
                {
                    diff = true;
                } else {
                    return false;
                }
            }
            diff
        };
        let mut usings = in_structs.get(&c.name).cloned().unwrap_or_default();
        for (name, hs) in &holders {
            let visible: Vec<&&String> = hs.iter().filter(|h| !hs.iter().any(|o| o != *h && ancestors(o).contains(**h))).collect();
            if own.contains(name) && visible.len() == 1 {
                let h = visible[0];
                let k = m.classes.iter().find(|k| k.name == **h).unwrap();
                let args = base_args(m, c, h).unwrap_or_default();
                let base_funcs: Vec<Func> = k
                    .funcs
                    .iter()
                    .filter(|f| Class::has_self(f) && cpp_name(f, "create") == *name)
                    .map(|f| map_types(&with_self_args(f, k), &|t| subst(t, &k.type_params, &args)))
                    .collect();
                let own_funcs: Vec<Func> = c
                    .funcs
                    .iter()
                    .filter(|f| Class::has_self(f) && cpp_name(f, "create") == *name)
                    .map(|f| with_self_args(f, c))
                    .collect();
                if base_funcs.iter().any(|bf| own_funcs.iter().any(|cf| clashes_adapted(bf, cf))) {
                    for bf in base_funcs {
                        if !own_funcs.iter().any(|cf| clashes_adapted(&bf, cf)) {
                            again.push(bf);
                        }
                    }
                    continue;
                }
                let base = match args.is_empty() {
                    false => ty_in(&Ty::Generic((*h).clone(), args), &cx),
                    true => (*h).clone(),
                };
                usings += &format!("    using {base}::{};\n", ident(name));
                if name == "invoke" {
                    usings += &format!("    using {base}::operator();\n");
                }
                continue;
            }
            if visible.len() < 2 {
                continue;
            }
            for h in visible {
                let k = m.classes.iter().find(|k| k.name == **h).unwrap();
                let args = base_args(m, c, h).unwrap_or_default();
                for f in k.funcs.iter().filter(|f| Class::has_self(f) && cpp_name(f, "create") == *name) {
                    let f = with_self_args(f, k);
                    again.push(map_types(&f, &|t| subst(t, &k.type_params, &args)));
                }
            }
        }

        let mut seen = BTreeSet::new();
        let mut members = String::new();
        let mut header_defs = String::new();
        let mut header_imports = BTreeSet::new();
        let mut used = BTreeSet::new();
        param_types(&c.type_params, &mut used);
        let lib = library.entry(c.module.clone()).or_default();
        // C++ cannot overload a static member with a non-static one of the
        // same parameters, as TypeScript can: the static one says it is.
        let sig_of = |name: &str, ps: &[(String, Ty)]| format!("{name}({})", ps.iter().map(|p| ty_param(&p.1, unions, &callbacks, &cx)).collect::<Vec<_>>().join(","));
        let signature = |f: &Func| sig_of(&cpp_name(f, ctor), &f.params);
        // Redeclared where the base documents them.
        again.iter_mut().for_each(|f| f.doc = None);
        let mut instance = BTreeSet::new();
        for f in c.funcs.iter().map(|f| with_self_args(f, c)).chain(again.iter().cloned()).filter(Class::has_self) {
            instance.insert(signature(&f));
            if rest_pack_info(&f, &cx).is_some() {
                instance.insert(sig_of(&cpp_name(&f, ctor), &f.params[..f.params.len() - 1]));
            }
        }
        // The functions of the namespace it shares its name with, as its statics.
        let namespaced: Vec<Func> = class_statics.get(&c.name).cloned().unwrap_or_default();
        let all_funcs: Vec<Func> = c.funcs.iter().map(|f| with_self_args(f, c)).chain(again).chain(namespaced).collect();
        let names: Vec<String> = all_funcs.iter().map(|f| cpp_name(f, ctor)).collect();
        let subsumed: Vec<bool> = all_funcs
            .iter()
            .enumerate()
            .map(|(idx, f)| {
                all_funcs.iter().enumerate().any(|(j, g)| {
                    j != idx && func_subsumed_by(f, g, &names[idx], &names[j], unions, &callbacks, &cx)
                })
            })
            .collect();
        for (idx, f) in all_funcs.iter().enumerate() {
            if subsumed[idx] {
                continue;
            }
            let raw_name = names[idx].clone();
            let exact_params = exact_params_for(
                f,
                &raw_name,
                all_funcs.iter().enumerate().filter(|(j, _)| *j != idx && !subsumed[*j]).map(|(j, g)| (g, names[j].as_str())),
                unions,
                &callbacks,
                &cx,
            );
            let place = if Class::has_self(f) { Place::Member } else { Place::Static };
            let name = if place == Place::Static && instance.contains(&signature(f)) { format!("{raw_name}_static") } else { raw_name };
            let zero_name = if place == Place::Static
                && rest_pack_info(f, &cx).is_some()
                && instance.contains(&sig_of(&name, &f.params[..f.params.len() - 1]))
            {
                Some(format!("{name}_static"))
            } else {
                None
            };
            let mut emit_names = vec![name];
            if place == Place::Member && f.call == Call::Invoke && emit_names[0] == "invoke" {
                emit_names.push("operator()".to_string());
            }
            for emit_name in emit_names {
                for func in function_variants(f, emit_name, zero_name.clone(), &owner, &class_head, place, unions, &callbacks, wasm_module, &raw_prefix, &exact_params, &cx) {
                    // Overloads TypeScript tells apart and C++ cannot are the same C++ function.
                    if !seen.insert(func.key) {
                        continue;
                    }
                    members += &func.decl;
                    used.extend(func.refs);
                    for t in &f.type_params {
                        param_types(std::slice::from_ref(t), &mut used);
                    }
                    if func.templated {
                        header_defs += &func.def;
                        header_imports.insert(func.import);
                    } else {
                        lib.1 += &func.def;
                        lib.0.insert(func.import);
                    }
                }
            }
        }
        for (name, value) in &c.consts {
            let ty = if value.starts_with('"') {
                "std::string_view"
            } else if matches!(value.as_str(), "true" | "false") {
                "bool"
            } else {
                "double"
            };
            members += &format!("    static constexpr {ty} {} = {value};\n", ident(name));
        }
        // Covariant conversion for generic classes: `Shape<RectConfig>` -> `Shape<ShapeConfig>`.
        if !c.type_params.is_empty() {
            let u_names: Vec<String> = (0..c.type_params.len()).map(|i| format!("U{i}_")).collect();
            let u_decls: Vec<String> = u_names.iter().map(|u| format!("class {u}")).collect();
            let same: Vec<String> = c.type_params.iter().zip(&u_names).map(|(p, u)| format!("std::same_as<{}, {u}>", ident(&p.name))).collect();
            let mut reqs: Vec<String> = vec![format!("!({})", same.join(" && "))];
            for (p, u) in c.type_params.iter().zip(&u_names) {
                reqs.push(format!("ts::extends<{}, {u}>", ident(&p.name)));
            }
            let u_args_ty: Vec<Ty> = u_names.iter().map(|u| Ty::Param(u.clone())).collect();
            for (p, u) in c.type_params.iter().zip(&u_names) {
                if let Some(con) = &p.constraint {
                    let sub_con = subst(con, &c.type_params, &u_args_ty);
                    reqs.push(format!("ts::extends<{u}, {}>", ty_arg(&sub_con, &cx)));
                }
            }
            let target = format!("{}<{}>", ident(&c.name), u_names.join(", "));
            members += &format!(
                "    template <{}>\n        requires ({})\n    operator {target}() const {{ if (this->h) ts_raw_js_retain(this->h); return {target}(ts::handle, this->h); }}\n",
                u_decls.join(", "),
                reqs.join(" && ")
            );
        }
        // Checked downcast: only a class has a constructor to test against.
        if c.js.is_some() {
            let cast_name = if own.contains("cast") || holders.contains_key("cast") || c.consts.iter().any(|(n, _)| n == "cast") || class_statics.get(&c.name).is_some_and(|fs| fs.iter().any(|f| cpp_name(f, ctor) == "cast")) {
                "try_cast"
            } else {
                "cast"
            };
            let is_name = format!("js_is__{}", c.name);
            let is_raw = raw_fn(&raw_prefix, &is_name);
            let is_attr = import_attr(wasm_module, &is_name);
            let is_import = format!("extern \"C\" {is_attr} int32_t {is_raw}(uint32_t);\n");
            members += &format!("    static std::optional<{owner}> {cast_name}(const ts::Handle& v);\n");
            let def = format!(
                "{class_head}{}std::optional<{owner}> {owner}::{cast_name}(const ts::Handle& v) {{ if (v.h != 0 && {is_raw}(v.h)) {{ ts_raw_js_retain(v.h); return {owner}(ts::handle, v.h); }} return std::nullopt; }}\n",
                if class_head.is_empty() { "" } else { "inline " },
            );
            if !class_head.is_empty() {
                header_defs += &def;
                header_imports.insert(is_import);
            } else {
                lib.1 += &def;
                lib.0.insert(is_import);
            }
        }
        // Iterable: `for (Node n : list)`.
        if let Some(elem) = &c.iterates
            && !own.contains("begin")
            && !own.contains("end")
        {
            let it = format!("ts::Iterator<{}>", ty_arg(&with_self_args(&Func { type_params: vec![], module: String::new(), import: String::new(), name: String::new(), call: Call::Create(String::new()), params: vec![], rest: false, fixed: vec![], shared: None, ret: elem.clone(), returns_this: false, doc: None }, c).ret, &cx));
            members += &format!("    {it} begin() const;\n    ts::IterationEnd end() const {{ return {{}}; }}\n");
            let templated = !class_head.is_empty();
            let def = format!("{class_head}{}{it} {owner}::begin() const {{ return {it}(ts_raw_js_iterate(this->h)); }}\n", if templated { "inline " } else { "" });
            named_types(elem, &mut used);
            if templated { header_defs += &def } else { lib.1 += &def }
        }
        if let Some(elem) = &c.iterates_async
            && !own.contains("iterate_async")
        {
            let it = format!("ts::AsyncIterator<{}>", ty_arg(&with_self_args(&Func { type_params: vec![], module: String::new(), import: String::new(), name: String::new(), call: Call::Create(String::new()), params: vec![], rest: false, fixed: vec![], shared: None, ret: elem.clone(), returns_this: false, doc: None }, c).ret, &cx));
            members += &format!("    {it} iterate_async() const;\n");
            let templated = !class_head.is_empty();
            let def = format!("{class_head}{}{it} {owner}::iterate_async() const {{ return {it}(ts::handle, ts_raw_js_async_iterate(this->h)); }}\n", if templated { "inline " } else { "" });
            named_types(elem, &mut used);
            if templated { header_defs += &def } else { lib.1 += &def }
        }
        let bases: Vec<String> = if c.bases.is_empty() { vec!["virtual ts::Handle".into()] } else { c.bases.iter().map(|b| format!("virtual {}", ty_in(b, &cx))).collect() };
        let mut decl = format!("// Generated.\n#pragma once\n{}", include(&decl_path, "ts_types.hpp"));
        for a in &c.ancestors {
            // A class's declaration; a callback's (a call signature) one file.
            let class = m.classes.iter().any(|k| k.name == *a);
            if let Some(h) = home.get(a) {
                decl += &include(&decl_path, &format!("{h}{}", if class { ".decl.hpp" } else { ".hpp" }));
            }
        }
        let class_doc = c.doc.as_deref().map(|d| comment(d, "///", "", Style::C)).unwrap_or_default();
        let decl_body = format!(
            "{class_doc}{class_head}struct {0} : {1} {{\n{2}{usings}{members}}};\n",
            c.name,
            bases.join(", "),
            constructors(&c.name)
        );
        decl += &format!("\n{}", wrap_ns(ns, &decl_body));
        used.remove(&c.name);
        used.extend(c.ancestors.iter().cloned());
        let mut imp = format!("// Generated.\n#pragma once\n{}{}", include(&impl_path, &decl_path), includes(&impl_path, &used));
        if !header_defs.is_empty() {
            imp += &format!("\n{}\n{}", header_imports.into_iter().collect::<String>(), wrap_ns(ns, &header_defs));
        }
        modules.entry(c.module.clone()).or_default().push(impl_path.clone());
        out.push((decl_path, decl));
        out.push((impl_path, imp));
    }
    let mut functions: BTreeMap<String, (BTreeSet<String>, BTreeSet<String>, String, String)> = BTreeMap::new();
    let mut seen = BTreeSet::new();
    let free_funcs: Vec<(Option<String>, String, &Func)> = m
        .funcs
        .iter()
        .filter(|f| !class_statics_of.contains(&f.import) && !callback_apply_imports.contains(&f.import))
        .map(|f| {
            // A namespace's, by its own name in a namespace of it: `marked_ns::parseInline`.
            let space = m.func_namespaces.get(&f.import).map(|(path, _, _, _)| {
                let top = match &containers.get(&path[0]) {
                    Some(Container::Namespace(n)) => n.clone(),
                    _ => format!("{}_ns", ident(&path[0])),
                };
                let mut parts = vec![top];
                for i in 1..path.len() {
                    parts.push(nested_name(&levels, &path[..i], &path[i]));
                }
                parts.join("::")
            });
            let name = m.func_namespaces.get(&f.import).map_or_else(|| cpp_name(f, "create"), |(_, own, _, _)| cpp_name_as(f, own, "create"));
            (space, name, f)
        })
        .collect();
    let free_subsumed: Vec<bool> = free_funcs
        .iter()
        .enumerate()
        .map(|(idx, (space, name, f))| {
            free_funcs.iter().enumerate().any(|(j, (os, on, g))| {
                j != idx && os == space && func_subsumed_by(f, g, name, on, unions, &callbacks, &cx)
            })
        })
        .collect();
    for (idx, (space, name, f)) in free_funcs.iter().enumerate() {
        if free_subsumed[idx] {
            continue;
        }
        let exact_params = exact_params_for(
            f,
            name,
            free_funcs
                .iter()
                .enumerate()
                .filter(|(j, (os, _, _))| *j != idx && !free_subsumed[*j] && os == space)
                .map(|(_, (_, on, g))| (*g, on.as_str())),
            unions,
            &callbacks,
            &cx,
        );
        for mut func in function_variants(f, name.clone(), None, "", "", Place::Free, unions, &callbacks, wasm_module, &raw_prefix, &exact_params, &cx) {
            if let Some(space) = &space {
                let wrap = |text: &str| if text.is_empty() { String::new() } else { format!("namespace {space} {{\n{text}}}\n") };
                func.key = format!("{space}::{}", func.key);
                func.decl = wrap(&func.decl);
                func.def = wrap(&func.def);
            }
            if !seen.insert(func.key) {
                continue;
            }
            let entry = functions.entry(f.module.clone()).or_default();
            entry.1.extend(func.refs);
            f.type_params.iter().for_each(|t| param_types(std::slice::from_ref(t), &mut entry.1));
            entry.2 += &func.decl;
            if func.templated {
                entry.0.insert(func.import);
                entry.3 += &func.def;
            } else {
                let lib = library.entry(f.module.clone()).or_default();
                lib.0.insert(func.import);
                lib.1 += &func.def;
            }
        }
    }
    for (module, (imports, used, decls, defs)) in functions {
        let path = format!("{}.hpp", super::super::model::file_name(&format!("{module}/functions"), &mut taken));
        let body = wrap_ns(ns, &format!("{decls}\n{defs}"));
        let text = format!("// Generated.\n#pragma once\n{}{}\n{}\n{body}", include(&path, "ts_types.hpp"), includes(&path, &used), imports.into_iter().collect::<String>());
        out.push((path.clone(), text));
        modules.entry(module).or_default().push(path);
    }
    // The definitions of each module: a translation unit to compile once.
    for (module, (imports, defs)) in library {
        let path = format!("{module}.cpp");
        let mut header_set: BTreeSet<String> = modules.get(&module).into_iter().flatten().cloned().collect();
        header_set.extend(extra_lib_headers.remove(&module).into_iter().flatten());
        let headers: String = header_set.into_iter().map(|h| include(&path, &h)).collect();
        let body = wrap_ns(ns, &defs);
        out.push((path, format!("// Generated. The module's definitions: compile once, and link with the program.\n{headers}\n{}\n{body}", imports.into_iter().collect::<String>())));
    }
    let mut all = format!("// Generated.\n#pragma once\n#include \"{inc_prefix}ts_types.hpp\"\n");
    for (module, headers) in &modules {
        let parent = module.rsplit_once('/').map_or("", |p| p.0);
        let includes: String = headers.iter().map(|h| format!("#include \"{}\"\n", h.strip_prefix(&format!("{parent}/")).unwrap_or(h))).collect();
        out.push((format!("{module}.hpp"), format!("// Generated.\n#pragma once\n{includes}")));
        all += &format!("#include \"{inc_prefix}{module}.hpp\"\n");
    }
    out.push(("bindings.hpp".into(), all.clone()));
    if pkg_name != "bindings" {
        out.push((format!("{pkg_name}.hpp"), all));
    }
    let meta_types: Vec<String> = m
        .classes
        .iter()
        .filter(|c| c.type_params.is_empty())
        .map(|c| c.name.as_str())
        .chain(m.callbacks.iter().filter(|c| c.type_params.is_empty()).map(|c| c.name.as_str()))
        .chain(m.enums.iter().map(|(n, _)| n.as_str()))
        .chain(m.num_enums.iter().map(|(n, _)| n.as_str()))
        .chain(m.unions.iter().filter(|u| u.type_params.is_empty()).map(|u| u.name.as_str()))
        .chain(resolved_opaque.keys().map(String::as_str))
        .filter(|n| !n.is_empty() && !n.contains(':') && !n.starts_with("ts_bases_of_") && !is_builtin_cpp_type(n))
        .map(str::to_string)
        .collect();
    let entry = if pkg_name != "bindings" { format!("{pkg_name}.hpp") } else { "bindings.hpp".into() };
    let meta = super::super::pkg_meta::PkgMeta {
        target: "cpp".into(),
        package: pkg_name.to_string(),
        entry: Some(entry),
        types: meta_types,
        ..Default::default()
    };
    out.push((super::super::pkg_meta::META_FILE.into(), meta.to_toml()));
    Ok(out)
}
