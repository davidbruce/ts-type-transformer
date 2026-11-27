use std::collections::{HashMap, HashSet};

use roxmltree::Node;

use super::super::{attr, elements, flag, marks, named, only_child};
use super::{doc_lines, member_doc_lines, Gen, T};
use super::member_ir::{FsMember, FsMemberKind, FsParam, has_member_conflict, push_member, render_members};
use super::plan::AliasForm;
use super::syntax::{
    all_properties, case_name, case_test, expand, fable_builtin, find_decl, ident, is_function_interface, js_path, only_child_type, subst_params, tag_case_name, type_ident,
    type_path, words,
};

impl Gen {
    pub(super) fn decls(&mut self, scope: Node, prefix: &str, indent: &str) {
        // Functions can be overloaded and take optional arguments, which only
        // members can do in F#: they become static members of one erased type.
        let mut functions: Vec<FsMember> = Vec::new();
        let mut func_extras: Vec<FsMember> = Vec::new();
        for d in elements(scope) {
            // Each top-level declaration is imported from its package; a namespace's members from its.
            if prefix.is_empty() {
                self.import = d.attribute("import").map(str::to_string).or_else(|| self.root_import.clone());
            }
            let raw = d.attribute("name").unwrap_or_default();
            let name = if matches!(d.tag_name().name(), "interface" | "class" | "typeAlias" | "enum" | "namespace") { type_ident(raw) } else { ident(raw) };
            let path = js_path(&self.js_prefix, d.attribute("jsName").unwrap_or(raw));
            // A top-level declaration's types go in its module's file; an alias where its parts are.
            if indent.is_empty()
                && let Some(module) = d.attribute("module")
            {
                let unit = self.units.get(raw).cloned().unwrap_or_else(|| module.to_string());
                self.current_file = if d.has_tag_name("typeAlias") { self.file_of.get(raw).copied().unwrap_or(0) } else { self.file_of.get(&format!("module:{unit}")).copied().unwrap_or(0) };
            }
            let start = self.out.len();
            let next_globals = if indent.is_empty() { !d.has_attribute("module") } else { self.in_globals };
            let prev_globals = std::mem::replace(&mut self.in_globals, next_globals);
            let prev_indent = std::mem::replace(&mut self.cur_indent, indent.to_string());
            let prev_prefix = std::mem::replace(&mut self.cur_prefix, prefix.to_string());
            self.decl(d, raw, &name, &path, prefix, indent, &mut functions, &mut func_extras);
            self.in_globals = prev_globals;
            self.cur_indent = prev_indent;
            self.cur_prefix = prev_prefix;
            if !self.pending_delegates.is_empty() {
                let dels = std::mem::take(&mut self.pending_delegates);
                self.out.insert_str(start, &dels);
            }
            if indent.is_empty()
                && let Some(module) = d.attribute("module")
            {
                let text = self.out.split_off(start);
                if d.has_tag_name("typeAlias") {
                    let file = self.file_of.get(raw).copied().unwrap_or(0);
                    *self.alias_types.entry(file).or_default() += &text;
                } else {
                    let unit = self.units.get(raw).cloned().unwrap_or_else(|| module.to_string());
                    *self.types.entry(unit).or_default() += &text;
                }
            }
        }
        for e in func_extras {
            let carrier_conflict = self.carrier.as_ref().is_some_and(|c| has_member_conflict(c, &e, &self.abbrevs));
            if !carrier_conflict && !has_member_conflict(&functions, &e, &self.abbrevs) {
                functions.push(e);
            }
        }
        if !functions.is_empty() {
            match self.carrier.as_mut() {
                Some(carrier) => carrier.extend(functions),
                None => self.out += &format!("{indent}[<Erase>]\n{indent}type Globals =\n{}\n", render_members(&functions)),
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn decl(&mut self, d: Node, raw: &str, name: &str, path: &str, prefix: &str, indent: &str, functions: &mut Vec<FsMember>, func_extras: &mut Vec<FsMember>) {
        let name = name.to_string();
        let path = path.to_string();
        match d.tag_name().name() {
            "lowering" => {}
            // A namespace of a package's name (node's `stream` `Stream`): its
            // members are the package's module's, reached through it.
            "namespace" if d.attribute("moduleKind") == Some("merged") => {
                let inner = js_path(&self.js_prefix, d.attribute("jsName").unwrap_or(raw));
                let outer = std::mem::replace(&mut self.js_prefix, inner);
                self.decls(d, prefix, indent);
                self.js_prefix = outer;
            }
            "namespace" => {
                self.out += &format!("{indent}module {name} =\n");
                let before = self.out.len();
                // A package's module adds nothing to what JavaScript reaches: it imports.
                let package = d.attribute("moduleKind") == Some("package");
                let inner = if package { self.js_prefix.clone() } else { js_path(&self.js_prefix, d.attribute("jsName").unwrap_or(raw)) };
                // A package's members are imported from it, a nested one's (`fs/promises`) too.
                let outer_import = if package { std::mem::replace(&mut self.import, d.attribute("import").map(str::to_string)) } else { self.import.clone() };
                let outer = std::mem::replace(&mut self.js_prefix, inner);
                // Its functions on a type of its name beside it (`Geometry.point`,
                // `Geometry.Deep.twice`), F# resolving `Geometry.Point` in the
                // module still; in it, as `Globals`, where its scope has
                // something else of that name (node's class `Stream`).
                let alone = d.parent().is_some_and(|p| !elements(p).any(|o| o != d && o.attribute("name") == Some(raw)));
                let outer_carrier = std::mem::replace(&mut self.carrier, alone.then(Vec::new));
                self.decls(d, &js_path(prefix, raw), &format!("{indent}    "));
                let functions = std::mem::replace(&mut self.carrier, outer_carrier);
                self.js_prefix = outer;
                self.import = outer_import;
                let has_functions = functions.as_ref().is_some_and(|f| !f.is_empty());
                if self.out.len() == before && has_functions {
                    // Only functions: the type of its name is all of it.
                    let line = format!("{indent}module {name} =\n");
                    self.out.truncate(before - line.len());
                } else if self.out.len() == before {
                    self.out += &format!("{indent}    let private empty = ()\n\n");
                }
                if let Some(functions) = functions.filter(|f| !f.is_empty()) {
                    self.out += &format!("{indent}[<Erase>]\n{indent}type {name} =\n{}\n", render_members(&functions));
                }
            }
            "enum" => {
                if named(d, "member").any(|v| attr(v, "type") != "string") {
                    self.numeric_enum(d, &name, indent);
                    return;
                }
                self.out += &format!("{}{indent}[<StringEnum; RequireQualifiedAccess>]\n{indent}type {name} =\n", doc_lines(d, indent));
                let mut seen = HashSet::new();
                let mut testers = HashSet::new();
                for v in named(d, "member") {
                    let value = attr(v, "value");
                    let mut case: String = value.chars().map(|c| if c.is_alphanumeric() { c } else { '_' }).collect();
                    if case.is_empty() || !case.as_bytes()[0].is_ascii_alphabetic() {
                        case.insert(0, 'V');
                    }
                    case[..1].make_ascii_uppercase();
                    // F# synthesizes `member Is<Case>` for every union case;
                    // avoid clashing with another case named `Is<Case>` (e.g. `IsNaN` and `NaN`).
                    while seen.contains(&case) || testers.contains(&case) || seen.contains(&format!("Is{case}")) {
                        case.push('_');
                    }
                    testers.insert(format!("Is{case}"));
                    seen.insert(case.clone());
                    self.out += &format!("{}{indent}    | [<CompiledName({value:?})>] {case}\n", member_doc_lines(v, &format!("{indent}    ")));
                }
                self.out += "\n";
            }
            // Only a call signature (`EventListener`): a function, written as a
            // lambda. Unless a type extends it (F# inherits only interfaces),
            // or it names a later file's type (only a member can go there).
            "interface" if is_function_interface(d) && !self.is_extended(d) => {
                let call = only_child(d);
                if named(call, "parameter").any(|p| flag(p, "rest")) {
                    let depth = self.fn_params.len();
                    self.fn_params.extend(named(call, "typeParameter").map(|p| attr(p, "name").to_string()));
                    let sig = self.callback_sig(call, "");
                    self.fn_params.truncate(depth);
                    if words(&sig.invoke).iter().filter_map(|w| self.file_of.get(*w)).any(|f| *f > self.current_file) {
                        self.interface(d, prefix, indent);
                        return;
                    }
                    self.out += &self.callback_def(&name, "", &name, &sig, indent);
                    return;
                }
                let t = if named(call, "parameter").any(|p| flag(p, "optional")) {
                    format!("delegate of {}", self.callback_sig(call, "").invoke)
                } else {
                    self.ty(call, "")
                };
                if words(&t).iter().filter_map(|w| self.file_of.get(*w)).any(|f| *f > self.current_file) {
                    self.interface(d, prefix, indent);
                    return;
                }
                if !t.starts_with("delegate") {
                    self.abbrevs.insert(name.to_string(), t.clone());
                }
                self.out += &format!("{indent}type {name} = {t}\n\n");
            }
            "interface" | "class" => self.interface(d, prefix, indent),
            "function" => {
                let saved_constrained = self.cur_constrained.clone();
                for p in named(d, "typeParameter") {
                    self.cur_constrained.remove(&format!("'{}", attr(p, "name")));
                }
                // A type parameter no required argument names, whose default is
                // one that does (date-fns' `ResultDate = DateType`), is that one:
                // the result follows the argument, with no annotation.
                let required: HashSet<&str> = named(d, "parameter")
                    .filter(|p| !flag(*p, "optional"))
                    .flat_map(|p| p.descendants().filter(|r| r.has_tag_name("ref") && attr(*r, "kind") == "typeParameter").map(|r| attr(r, "name")))
                    .collect();
                let follows: HashMap<String, String> = named(d, "typeParameter")
                    .filter_map(|p| {
                        let def = named(p, "default").next().map(only_child).filter(|d| d.has_tag_name("ref") && d.attribute("kind") == Some("typeParameter"))?;
                        let to = def.attribute("name")?;
                        (!required.contains(attr(p, "name")) && required.contains(to)).then(|| (attr(p, "name").to_string(), format!("'{to}")))
                    })
                    .collect();
                self.skip_tparams = follows.keys().cloned().collect();
                let (tps, fn_constrained) = self.constrained_type_params_with_names(d);
                self.skip_tparams.clear();
                self.cur_constrained.extend(fn_constrained);
                let params: Vec<FsParam> = self.params(d, "").iter().map(|p| p.subst(&follows)).collect();
                let ret = subst_params(&self.returns(d, ""), &follows);
                let (fname, args) = match self.fixed(d, 0) {
                    // Escaped where it is written, by `at`.
                    // Only an options object the glue passes: named as lowering named it (`marked`, `marked_async`).
                    Some((suffix, args)) => (ident(&if suffix.is_empty() && args.contains('{') { raw.to_string() } else { format!("{raw}_{suffix}") }), args),
                    None => (name, "$0...".to_string()),
                };
                let at_attr = self.at(&path, Some(&args));
                let fn_marks = marks(d);
                let fn_tparams: Vec<Node> = named(d, "typeParameter").collect();
                let (ret_only_map, all_ret_only, _) = if fn_tparams.is_empty() {
                    (HashMap::new(), false, None)
                } else {
                    self.defaulted_tp_maps(d, "", &fn_tparams, None, false)
                };
                if all_ret_only {
                    let def_params: Vec<FsParam> = params.iter().map(|p| p.subst(&ret_only_map)).collect();
                    let def_ret = subst_params(&ret, &ret_only_map);
                    push_member(
                        functions,
                        FsMember {
                            indent: indent.to_string(),
                            attrs: vec![at_attr.clone()],
                            name: fname.clone(),
                            kind: FsMemberKind::StaticMethod {
                                tparams: String::new(),
                                params: def_params.clone(),
                                ret: def_ret.clone(),
                            },
                            marks: fn_marks.clone(),
                            doc: member_doc_lines(d, &format!("{indent}    ")),
                        },
                        &self.abbrevs,
                    );
                    for ep in self.union_param_expansions(d, "", &def_params) {
                        func_extras.push(FsMember {
                            indent: indent.to_string(),
                            attrs: vec![at_attr.clone()],
                            name: fname.clone(),
                            kind: FsMemberKind::StaticMethod {
                                tparams: String::new(),
                                params: ep,
                                ret: def_ret.clone(),
                            },
                            marks: fn_marks.clone(),
                            doc: String::new(),
                        });
                    }
                }
                push_member(
                    functions,
                    FsMember {
                        indent: indent.to_string(),
                        attrs: vec![at_attr.clone()],
                        name: fname.clone(),
                        kind: FsMemberKind::StaticMethod {
                            tparams: tps.clone(),
                            params: params.clone(),
                            ret: ret.clone(),
                        },
                        marks: fn_marks.clone(),
                        doc: member_doc_lines(d, &format!("{indent}    ")),
                    },
                    &self.abbrevs,
                );
                for ep in self.union_param_expansions(d, "", &params) {
                    func_extras.push(FsMember {
                        indent: indent.to_string(),
                        attrs: vec![at_attr.clone()],
                        name: fname.clone(),
                        kind: FsMemberKind::StaticMethod {
                            tparams: tps.clone(),
                            params: ep,
                            ret: ret.clone(),
                        },
                        marks: fn_marks.clone(),
                        doc: String::new(),
                    });
                }
                self.cur_constrained = saved_constrained;
            }
            "typeAlias" if only_child_type(d).has_tag_name("union") && named(d, "typeParameter").next().is_none() => {
                let body = only_child_type(d);
                if !self.named_union(&name, body, indent) {
                    // Erased: its types, though a match cannot tell them apart.
                    let recursive = body.descendants().any(|r| r.has_tag_name("ref") && r.attribute("name") == Some(raw));
                    let t = if recursive {
                        self.skip("recursive union, bound as obj");
                        "obj".to_string()
                    } else {
                        self.skip("union whose members cannot be told apart, erased");
                        self.ty(body, "")
                    };
                    self.abbrevs.insert(name.to_string(), t.clone());
                    self.out += &format!("{indent}type {name} = {t}\n\n");
                }
            }
            "typeAlias" => {
                let body = only_child_type(d);
                let id = d.attribute("id").unwrap_or(raw);
                let params = match self.alias_kept.get(id) {
                    Some(kept) => {
                        let all: Vec<Node> = named(d, "typeParameter").collect();
                        let shown: Vec<String> = kept.iter().map(|&i| format!("'{}", attr(all[i], "name"))).collect();
                        if shown.is_empty() { String::new() } else { format!("<{}>", shown.join(", ")) }
                    }
                    None => self.type_params(d),
                };
                // The constraints its type needs of those it shows, written as F# wants them.
                let kept = self.alias_kept.get(id).cloned();
                let when: Vec<String> = match self.constrained.get(id) {
                    Some(cs) => named(d, "typeParameter")
                        .zip(cs)
                        .enumerate()
                        .filter(|(i, _)| kept.as_ref().is_none_or(|k| k.contains(i)))
                        .filter_map(|(_, (p, c))| c.as_ref().map(|c| format!("'{} :> {}", attr(p, "name"), fable_builtin(c).unwrap_or_else(|| type_path(c)))))
                        .collect(),
                    None => Vec::new(),
                };
                let header = if when.is_empty() || params.is_empty() { params.clone() } else { format!("{} when {} >", params.trim_end_matches('>'), when.join(" and ")) };
                let saved_constrained = self.cur_constrained.clone();
                if let Some(cs) = self.constrained.get(id) {
                    for (p, c) in named(d, "typeParameter").zip(cs) {
                        if c.is_some() {
                            self.cur_constrained.insert(format!("'{}", attr(p, "name")));
                        }
                    }
                }
                let form = self.alias_form(d);
                match form {
                    AliasForm::Interface => self.interface(d, prefix, indent),
                    AliasForm::Delegate => {
                        // A function value's own type parameters are `obj`, as in `ty`.
                        let depth = self.fn_params.len();
                        self.fn_params.extend(named(body, "typeParameter").map(|p| attr(p, "name").to_string()));
                        let sig = self.callback_sig(body, "");
                        self.fn_params.truncate(depth);
                        let ret_type = format!("{name}{params}");
                        self.out += &self.callback_def(&name, &header, &ret_type, &sig, indent);
                    }
                    AliasForm::Abbreviation(t) => {
                        if params.is_empty() {
                            self.abbrevs.insert(name.to_string(), t.clone());
                        }
                        self.out += &format!("{indent}type {name}{header} = {t}\n\n");
                    }
                }
                self.cur_constrained = saved_constrained;
            }
            "var" => {
                let body = only_child(d);
                if body.has_tag_name("object") {
                    let members: Vec<Node> = elements(body).collect();
                    self.static_side(&name, &path, &members, None, None, indent);
                } else {
                    let t = self.out_ty(body, "");
                    let value = format!("{indent}{}\n{indent}let {name}: {t} = jsNative\n\n", self.at(&path, None));
                    if indent.is_empty() {
                        self.values += &value;
                    } else {
                        self.out += &value;
                    }
                }
            }
            _ => self.problems.element(T, d),
        }
    }

    // A union declared as an F# type: told apart by a property when it has a
    // discriminant, else erased, each case with the runtime test the XML gives.
    // False if Fable could not tell the cases apart.
    pub(super) fn named_union(&mut self, name: &str, union: Node, indent: &str) -> bool {
        let members: Vec<Node> = elements(union).collect();
        let mut cases: Vec<(String, String, Option<String>)> = Vec::new();
        let mut seen_names = HashSet::new();
        let mut seen_testers = HashSet::new();
        // A test two members share tells neither apart: Fable guesses, as for `U2`.
        let mut test_count: HashMap<&str, usize> = HashMap::new();
        for m in &members {
            if let Some(t) = m.attribute("test") {
                *test_count.entry(t).or_default() += 1;
            }
        }
        for m in &members {
            let payload = self.ty(*m, "");
            let mut case = if m.has_tag_name("object") {
                tag_case_name(*m).unwrap_or_else(|| {
                    // By its type's own name, without its arguments: `JsRecord`, not `NodeViewConstructor>`.
                    let own = payload.split('<').next().unwrap_or(&payload).rsplit('.').next().unwrap_or(&payload);
                    if !payload.starts_with("{|") && payload != "obj" && own.starts_with(|c: char| c.is_alphabetic()) && own.chars().all(|c| c.is_alphanumeric() || c == '_') {
                        own.to_string()
                    } else {
                        case_name(*m)
                    }
                })
            } else {
                case_name(*m)
            };
            while seen_names.contains(&case) || seen_testers.contains(&case) || seen_names.contains(&format!("Is{case}")) {
                case.push('_');
            }
            seen_testers.insert(format!("Is{case}"));
            seen_names.insert(case.clone());
            let test = m.attribute("test").filter(|t| test_count[t] == 1).map(str::to_string);
            cases.push((case, payload, test));
        }
        // Named however many members no test tells apart: it is made of any
        // of them, and its cases match by their tests, else by Fable's guess.
        if cases.iter().any(|c| c.1 == "obj" || c.1 == "unit") {
            return false;
        }
        // Fable tags a case by its interface's member: each must be declared here and carry a `tag:` test.
        let declared = members.iter().zip(&cases).all(|(m, (_, payload, _))| {
            (m.has_tag_name("ref") && m.has_attribute("to"))
                || (m.has_tag_name("object") && !payload.starts_with("{|") && payload != "obj")
        });
        let all_tagged = cases.iter().all(|(_, _, t)| t.as_deref().is_some_and(|t| t.starts_with("tag:")));
        // Fable's [<TypeScriptTaggedUnion>] matches against string literals (`=== "tag"`).
        // Non-string discriminants (e.g. boolean `{ done: true } | { done: false }`) fall
        // back to [<Erase>] with [<CaseTest>].
        let is_string_discriminant = |d_name: &str| -> bool {
            let root = union.document().root_element();
            members.iter().all(|m| {
                let target = if m.has_tag_name("object") {
                    Some(*m)
                } else if let Some(to) = m.attribute("to") {
                    root.descendants().find(|n| n.attribute("id") == Some(to))
                } else {
                    None
                };
                target
                    .and_then(|node| elements(node).find(|p| p.has_tag_name("property") && p.attribute("name") == Some(d_name)))
                    .and_then(|prop| elements(prop).find(|c| !c.has_tag_name("doc")))
                    .is_some_and(|ty| (ty.has_tag_name("literal") && ty.attribute("type") == Some("string")) || (ty.has_tag_name("primitive") && ty.attribute("name") == Some("string")))
            })
        };
        if let Some(d) = union.attribute("discriminant").filter(|d| declared && all_tagged && is_string_discriminant(d)) {
            self.out += &format!("{indent}[<TypeScriptTaggedUnion({d:?})>]\n{indent}type {name} =\n");
            for (case, payload, test) in &cases {
                let value = test.as_deref().and_then(|t| t.strip_prefix("tag:")).unwrap_or_default();
                self.out += &format!("{indent}    | [<CompiledName({value:?})>] {case} of {payload}\n");
            }
        } else {
            self.out += &format!("{indent}[<Erase; RequireQualifiedAccess>]\n{indent}type {name} =\n");
            for (case, payload, test) in &cases {
                let native = matches!(payload.as_str(), "string" | "float" | "bool");
                let class = test.as_deref().and_then(|t| t.strip_prefix("instanceof:"));
                let imported = class.is_some_and(|c| self.tested_classes.contains_key(c));
                // Where another case's class extends this one's (`Pt extends
                // Float32Array`), this case is only an instance of exactly this class.
                let exact = class.filter(|c| !c.contains('.')).filter(|c| cases.iter().any(|(_, payload, _)| payload != c && self.reaches(payload, c)));
                let attr = match exact {
                    Some(c) if !native && !imported => format!("[<CaseTest(\"$0 != null && Object.getPrototypeOf($0) === globalThis.{c}.prototype\")>] "),
                    _ => match test.as_deref().and_then(|t| case_test(t, union.attribute("discriminant"))) {
                        // An F# string: its backslashes too (katex's `"\\href"`).
                        Some(js) if !native && !imported => format!("[<CaseTest(\"{}\")>] ", js.replace('\\', "\\\\").replace('"', "\\\"")),
                        _ => String::new(),
                    },
                };
                self.out += &format!("{indent}    | {attr}{case} of {payload}\n");
            }
        }
        self.shared_fields(&members, indent);
        // A value of a case's type converts to the union: implicitly where an
        // argument is one, with `!^` elsewhere and from subtypes. Once per
        // type F# sees (`DOMHighResTimeStamp` is a float); never from a type
        // parameter, `obj`, `unit` or the union itself.
        if !name.contains('<') {
            let mut seen = HashSet::new();
            for ((case, payload, _), m) in cases.iter().zip(&members) {
                // An anonymous record is one type whatever its fields' order.
                let expanded = expand(&self.through_aliases(*m), &self.abbrevs);
                let seen_as = record_fields_sorted(&expanded);
                if payload.contains('\'') || matches!(payload.as_str(), "obj" | "unit") || payload == name || !seen.insert(seen_as) {
                    continue;
                }
                // Its cases by its path from the root: a value of its name, or
                // of its module's, nearer (node's class `Stream.Transform`, and
                // `Stream` itself) would be taken.
                let path = if self.cur_prefix.is_empty() { name.to_string() } else { format!("global.{}.{}.{name}", self.namespace, self.cur_prefix.split('.').map(type_ident).collect::<Vec<_>>().join(".")) };
                for op in ["op_Implicit", "op_ErasedCast"] {
                    self.out += &format!("{indent}    static member inline {op} (x: {payload}) : {name} = {path}.{case} x\n");
                }
                if let Some(val_ty) = js_record_value_ty(&expanded) {
                    for (pair_ty, seen_key) in [
                        (format!("(string * {val_ty}) list"), format!("(string * {val_ty}) list")),
                        (format!("(string * {val_ty})[]"), format!("(string * {val_ty})[]")),
                    ] {
                        if seen.insert(seen_key) {
                            self.out += &format!("{indent}    [<Emit(\"Object.fromEntries($0)\")>] static member op_Implicit (x: {pair_ty}) : {name} = jsNative\n");
                        }
                    }
                }
            }
        }
        self.out += "\n";
        true
    }

    // A property every member has, read without a match (marked's
    // `token.``type```): its own or a base's, of one type in each, optional
    // where any member's is.
    fn shared_fields(&mut self, members: &[Node], indent: &str) {
        let Some(root) = members.first().map(|m| m.document().root_element()) else { return };
        let props: Option<Vec<Vec<Node>>> = members.iter().map(|m| m.attribute("to").filter(|_| m.has_tag_name("ref")).and_then(|t| find_decl(root, t)).map(|d| all_properties(root, d))).collect();
        let Some(props) = props.filter(|p| p.len() > 1) else { return };
        let mut seen = HashSet::new();
        for key in props[0].iter().map(|p| attr(*p, "name")) {
            if !seen.insert(key) {
                continue;
            }
            let Some(found) = props.iter().map(|ps| ps.iter().find(|q| attr(**q, "name") == key).copied()).collect::<Option<Vec<Node>>>() else { continue };
            // Of a type that needs its owner (`this`, an inline object, a function), none.
            let owned = |q: &Node| q.descendants().any(|d| matches!(d.tag_name().name(), "this" | "object" | "functionType" | "call"));
            if found.iter().any(owned) || !key.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_') || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                continue;
            }
            // A constant all share is the base's (`Node.ELEMENT_NODE`).
            if found[0].has_attribute("const") && found.iter().all(|q| q.attribute("const") == found[0].attribute("const")) {
                continue;
            }
            let tys: Vec<String> = found.iter().map(|q| self.ty(only_child(*q), "")).collect();
            let bare: Vec<&str> = tys.iter().map(|t| t.strip_suffix(" option").unwrap_or(t)).collect();
            // Nor of a type a later file declares (`HTMLSlotElement`, after `core`).
            let later = words(bare[0]).iter().filter_map(|w| self.file_of.get(*w)).any(|f| *f > self.current_file);
            if later || bare.iter().any(|t| *t != bare[0]) || bare[0].contains('\'') || matches!(bare[0], "obj" | "unit") {
                continue;
            }
            let optional = found.iter().any(|q| q.attribute("optional") == Some("true")) || tys.iter().any(|t| t.ends_with(" option"));
            let t = if optional { format!("{} option", bare[0]) } else { bare[0].to_string() };
            self.out += &format!("{indent}    [<Emit(\"$0.{key}\")>] member _.{}: {t} = jsNative\n", ident(key));
        }
    }

    fn numeric_enum(&mut self, d: Node, name: &str, indent: &str) {
        self.out += &format!("{}{indent}[<RequireQualifiedAccess>]\n{indent}type {name} =\n", doc_lines(d, indent));
        for (i, v) in named(d, "member").enumerate() {
            let value = if attr(v, "type") == "number" { attr(v, "value").to_string() } else { i.to_string() };
            self.out += &format!("{}{indent}    | {} = {value}\n", member_doc_lines(v, &format!("{indent}    ")), ident(attr(v, "name")));
        }
        self.out += "\n";
    }
}

fn js_record_value_ty(expanded: &str) -> Option<&str> {
    let rest = expanded.strip_prefix("JsRecord<")?.strip_suffix('>')?;
    let mut depth = 0usize;
    for (i, ch) in rest.char_indices() {
        match ch {
            '<' => depth += 1,
            '>' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                let key = rest[..i].trim();
                let val = rest[i + 1..].trim();
                return (key == "string" && !val.is_empty() && val != "unit" && !val.contains('\'')).then_some(val);
            }
            _ => {}
        }
    }
    None
}

// `{| b: B; a: A |}` as `{| a: A; b: B |}`: its fields at the top, sorted.
fn record_fields_sorted(t: &str) -> String {
    let Some(inner) = t.strip_prefix("{|").and_then(|r| r.strip_suffix("|}")) else { return t.to_string() };
    let (mut fields, mut depth, mut start) = (Vec::new(), 0i32, 0);
    for (i, c) in inner.char_indices() {
        match c {
            '<' | '(' | '[' | '{' => depth += 1,
            '>' | ')' | ']' | '}' => depth -= 1,
            ';' if depth == 0 => {
                fields.push(inner[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    fields.push(inner[start..].trim());
    fields.sort_unstable();
    format!("{{| {} |}}", fields.join("; "))
}
