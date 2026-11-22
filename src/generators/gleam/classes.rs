use std::collections::{HashMap, HashSet};

use super::bounds::{chain_root, mentions_any, secondary};
use super::types::Gen;
use super::{
    codec_var, func_unit, function_name, gleam_const_literal, lower, relative, snake,
    JS_RESERVED, MAX_INHERITED_ALIASES,
};
use super::super::js::{access, global};
use super::super::model::{base_args, map_types, subst, with_self_args, Call, Class, Func, Model, TParam, Ty};

#[derive(Clone)]
pub(super) struct EmittedMethod {
    gleam_name: String,
    js_name: String,
    ffi_path: String,
    func: Func,
}

#[derive(Clone)]
pub(super) struct EmittedWith {
    gleam_name: String,
    js_name: String,
    ffi_path: String,
    func: Func,
}

impl<'m> Gen<'m> {
    fn emit_specializations(&mut self, c: &'m Class) {
        let mut chain = Vec::new();
        let mut at = c.name.clone();
        while let Some((p, _)) = self.parents.get(&at) {
            chain.push(p.clone());
            at = p.clone();
        }
        let mut planned = HashSet::new();
        let mut specs: Vec<(String, Func)> = Vec::new();
        for anc_name in chain {
            let Some(anc) = self.m.classes.iter().find(|k| k.name == anc_name) else { continue };
            if anc.type_params.is_empty() {
                continue;
            }
            let Some(sub_args) = base_args(self.m, c, &anc.name) else { continue };
            for f in anc.funcs.iter().filter(|f| Class::has_self(f)) {
                let f_anc = with_self_args(f, anc);
                let id = lower(&function_name(&f_anc, &f_anc.name));
                if self.reserved_values.contains(&id) || self.values.contains(&(self.unit.clone(), id.clone())) || planned.contains(&id) {
                    continue;
                }
                if f_anc.type_params.iter().any(|ftp| c.type_params.iter().any(|ctp| ctp.name == ftp.name)) {
                    continue;
                }
                self.setup_fn_bounds(&f_anc, Some((&anc.name, &anc.type_params)));
                let orig_codecs = self.crossing_params(&f_anc, Some((&anc.name, &anc.type_params))).len();
                self.clear_fn_bounds();
                let spec_f = map_types(&f_anc, &|t| subst(t, &anc.type_params, &sub_args));
                self.setup_fn_bounds(&spec_f, Some((&c.name, &c.type_params)));
                let spec_codecs = self.crossing_params(&spec_f, Some((&c.name, &c.type_params))).len();
                self.clear_fn_bounds();
                if spec_codecs < orig_codecs {
                    planned.insert(id.clone());
                    specs.push((id, spec_f));
                }
            }
        }
        for (id, spec_f) in specs {
            let id = self.value_name(&id);
            self.function(&id, &spec_f, Some((&c.name, &c.type_params)));
        }
    }

    fn inherited_spec_crossing(
        &self,
        f_anc: &Func,
        anc: &Class,
        spec_f: &Func,
        c: &Class,
        sub_args: &[Ty],
    ) -> Option<Vec<TParam>> {
        if f_anc.type_params.iter().any(|ftp| {
            c.type_params.iter().any(|ctp| ctp.name == ftp.name)
                || anc.type_params.iter().any(|atp| atp.name == ftp.name)
                || ftp.constraint.as_ref().is_some_and(|t| mentions_any(t, &anc.type_params))
                || ftp.default.as_ref().is_some_and(|t| mentions_any(t, &anc.type_params))
        }) {
            return None;
        }
        self.setup_fn_bounds(f_anc, Some((&anc.name, &anc.type_params)));
        let orig_crossing = self.crossing_params(f_anc, Some((&anc.name, &anc.type_params)));
        self.clear_fn_bounds();

        self.setup_fn_bounds(spec_f, Some((&c.name, &c.type_params)));
        let spec_crossing = self.crossing_params(spec_f, Some((&c.name, &c.type_params)));
        if spec_crossing.len() != orig_crossing.len() {
            self.clear_fn_bounds();
            return None;
        }
        let mut expected_names = Vec::with_capacity(orig_crossing.len());
        for p in &orig_crossing {
            if f_anc.type_params.iter().any(|fp| fp.name == p.name) {
                expected_names.push(p.name.clone());
            } else if let Some(idx) = anc.type_params.iter().position(|ap| ap.name == p.name) {
                let Some(Ty::Param(cp_name)) = sub_args.get(idx) else {
                    self.clear_fn_bounds();
                    return None;
                };
                if !c.type_params.iter().any(|cp| cp.name == *cp_name)
                    || self.bound.borrow().contains_key(cp_name)
                    || self.fixed_params.borrow().contains_key(cp_name)
                    || self.bound_lists.borrow().contains_key(cp_name)
                {
                    self.clear_fn_bounds();
                    return None;
                }
                expected_names.push(cp_name.clone());
            } else {
                self.clear_fn_bounds();
                return None;
            }
        }
        let spec_names: Vec<String> = spec_crossing.iter().map(|p| p.name.clone()).collect();
        self.clear_fn_bounds();
        (spec_names == expected_names).then_some(spec_crossing)
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_alias_decl(
        &mut self,
        c: &'m Class,
        own: &[Ty],
        spec_f: &Func,
        spec_crossing: &[TParam],
        gleam_name: &str,
        js_name: &str,
        ffi_path: &str,
        return_self: bool,
    ) {
        self.setup_fn_bounds(spec_f, Some((&c.name, &c.type_params)));
        *self.open.borrow_mut() = Some(0);
        let self_ty = self.class_type(&c.name, own);
        let mut params = vec![format!("self: {self_ty}")];
        let mut taken: HashSet<String> = HashSet::from(["self".to_string()]);
        for (p, t) in &spec_f.params {
            let mut id = lower(p);
            while JS_RESERVED.contains(&id.as_str()) || !taken.insert(id.clone()) {
                id.push('_');
            }
            params.push(format!("{id}: {}", self.ty(t)));
        }
        *self.open.borrow_mut() = None;
        for p in spec_crossing {
            self.note("Codec");
            params.push(format!("{}_codec: Codec({})", lower(&p.name), Self::tvar(&p.name)));
        }
        let ret = if return_self { self_ty } else { self.ty(&spec_f.ret) };
        self.clear_fn_bounds();
        self.values.insert((self.unit.clone(), gleam_name.to_string()));
        let rel_ffi = relative(&self.unit, ffi_path);
        self.out += &format!("@external(javascript, \"{rel_ffi}\", \"{js_name}\")\npub fn {gleam_name}({}) -> {ret}\n\n", params.join(", "));
    }

    fn emit_inherited_aliases(
        &mut self,
        c: &'m Class,
        emitted_methods: &HashMap<String, Vec<EmittedMethod>>,
        emitted_withs: &HashMap<String, Vec<EmittedWith>>,
        emit_all_inherited: bool,
    ) {
        let mut chain = Vec::new();
        let mut at = c.name.clone();
        while let Some((p, _)) = self.parents.get(&at) {
            chain.push(p.clone());
            at = p.clone();
        }
        if chain.is_empty() {
            return;
        }
        let c_is_bag = c.funcs.iter().any(|f| matches!(f.call, Call::Create(_)));
        let own: Vec<Ty> = c.type_params.iter().map(|p| Ty::Param(p.name.clone())).collect();
        for anc_name in chain {
            let Some(anc) = self.m.classes.iter().find(|k| k.name == anc_name) else { continue };
            let Some(sub_args) = (if anc.type_params.is_empty() { Some(Vec::new()) } else { base_args(self.m, c, &anc.name) }) else { continue };
            if c_is_bag && let Some(withs) = emitted_withs.get(&anc.name) {
                for ew in withs {
                    if self.reserved_values.contains(&ew.gleam_name) || self.values.contains(&(self.unit.clone(), ew.gleam_name.clone())) {
                        continue;
                    }
                    let spec_wf = map_types(&ew.func, &|t| subst(t, &anc.type_params, &sub_args));
                    let Some(spec_crossing) = self.inherited_spec_crossing(&ew.func, anc, &spec_wf, c, &sub_args) else { continue };
                    self.emit_alias_decl(c, &own, &spec_wf, &spec_crossing, &ew.gleam_name, &ew.js_name, &ew.ffi_path, true);
                }
            }
            if emit_all_inherited && let Some(methods) = emitted_methods.get(&anc.name) {
                for em in methods {
                    if self.reserved_values.contains(&em.gleam_name) || self.values.contains(&(self.unit.clone(), em.gleam_name.clone())) {
                        continue;
                    }
                    let spec_f = map_types(&em.func, &|t| subst(t, &anc.type_params, &sub_args));
                    let Some(spec_crossing) = self.inherited_spec_crossing(&em.func, anc, &spec_f, c, &sub_args) else { continue };
                    self.emit_alias_decl(c, &own, &spec_f, &spec_crossing, &em.gleam_name, &em.js_name, &em.ffi_path, spec_f.returns_this);
                }
            }
        }
    }

    fn emit_with_builders(&mut self, c: &'m Class, prefix: Option<&str>) -> Vec<EmittedWith> {
        let mut emitted = Vec::new();
        if !c.funcs.iter().any(|f| matches!(f.call, Call::Create(_))) {
            return emitted;
        }
        for orig_f in &c.funcs {
            if !Class::has_self(orig_f) {
                continue;
            }
            let f = with_self_args(orig_f, c);
            match &f.call {
                Call::Set(js) if f.params.len() == 1 => {
                    let fname = lower(&function_name(&f, &f.name));
                    let rest_owned;
                    let rest = if !super::super::case::has_dollar_js_name(&f.call)
                        && super::super::case::starts_with_word(js, "set")
                    {
                        rest_owned = lower(js);
                        rest_owned.as_str()
                    } else {
                        let Some(r) = fname.strip_prefix("set_").or_else(|| fname.strip_prefix("assign_")) else { continue };
                        r
                    };
                    let with_base = match prefix {
                        Some(p) => lower(&format!("{p}_with_{rest}")),
                        None => lower(&format!("with_{rest}")),
                    };
                    let with_id = match prefix {
                        Some(_) => {
                            if self.reserved_values.contains(&with_base) {
                                continue;
                            }
                            self.fresh_value(&with_base)
                        }
                        None => {
                            if self.reserved_values.contains(&with_base) || self.values.contains(&(self.unit.clone(), with_base.clone())) {
                                continue;
                            }
                            self.value_name(&with_base)
                        }
                    };
                    let val_ty = match &f.params[0].1 {
                        Ty::Nullable(inner, _) => (**inner).clone(),
                        t => t.clone(),
                    };
                    let mut wf = f.clone();
                    wf.params = vec![(f.params[0].0.clone(), val_ty.clone())];
                    wf.ret = Ty::Void;
                    self.setup_fn_bounds(&wf, Some((&c.name, &c.type_params)));
                    *self.open.borrow_mut() = Some(0);
                    let own: Vec<Ty> = c.type_params.iter().map(|p| Ty::Param(p.name.clone())).collect();
                    let self_ty = self.class_type(&c.name, &own);
                    let mut val_param = lower(&wf.params[0].0);
                    while JS_RESERVED.contains(&val_param.as_str()) || val_param == "self" {
                        val_param.push('_');
                    }
                    let val_ty_str = self.ty(&val_ty);
                    let val_js = self.from_gleam(&val_ty, &val_param);
                    *self.open.borrow_mut() = None;
                    let mut params = vec![format!("self: {self_ty}"), format!("{val_param}: {val_ty_str}")];
                    let mut js_params = vec!["self".to_string(), val_param.clone()];
                    for p in self.crossing_params(&wf, Some((&c.name, &c.type_params))) {
                        self.note("Codec");
                        params.push(format!("{}_codec: Codec({})", lower(&p.name), Self::tvar(&p.name)));
                        js_params.push(codec_var(&p.name));
                    }
                    let js_fn = if JS_RESERVED.contains(&with_id.as_str()) { format!("{with_id}_") } else { with_id.clone() };
                    self.out += &format!("@external(javascript, \"{}\", \"{js_fn}\")\npub fn {with_id}({}) -> {self_ty}\n\n", self.own_ffi(), params.join(", "));
                    self.ffi += &format!("export function {js_fn}({}) {{\n  self{} = {val_js};\n  return self;\n}}\n", js_params.join(", "), access(js));
                    self.clear_fn_bounds();
                    if prefix.is_none() {
                        emitted.push(EmittedWith {
                            gleam_name: with_id,
                            js_name: js_fn,
                            ffi_path: format!("{}_ffi.mjs", self.unit),
                            func: wf,
                        });
                    }
                }
                Call::IndexSet if f.params.len() == 2 => {
                    let with_base = match prefix {
                        Some(p) => lower(&format!("{p}_with_at")),
                        None => "with_at".to_string(),
                    };
                    let with_id = match prefix {
                        Some(_) => {
                            if self.reserved_values.contains(&with_base) {
                                continue;
                            }
                            self.fresh_value(&with_base)
                        }
                        None => {
                            if self.reserved_values.contains(&with_base) || self.values.contains(&(self.unit.clone(), with_base.clone())) {
                                continue;
                            }
                            self.value_name(&with_base)
                        }
                    };
                    let mut wf = f.clone();
                    wf.ret = Ty::Void;
                    self.setup_fn_bounds(&wf, Some((&c.name, &c.type_params)));
                    *self.open.borrow_mut() = Some(0);
                    let own: Vec<Ty> = c.type_params.iter().map(|p| Ty::Param(p.name.clone())).collect();
                    let self_ty = self.class_type(&c.name, &own);
                    let mut taken: HashSet<String> = HashSet::from(["self".to_string()]);
                    let mut params = vec![format!("self: {self_ty}")];
                    let mut js_params = vec!["self".to_string()];
                    let mut arg_js = Vec::new();
                    for (p, t) in &wf.params {
                        let mut pid = lower(p);
                        while JS_RESERVED.contains(&pid.as_str()) || !taken.insert(pid.clone()) {
                            pid.push('_');
                        }
                        params.push(format!("{pid}: {}", self.ty(t)));
                        js_params.push(pid.clone());
                        arg_js.push(self.from_gleam(t, &pid));
                    }
                    *self.open.borrow_mut() = None;
                    for p in self.crossing_params(&wf, Some((&c.name, &c.type_params))) {
                        self.note("Codec");
                        params.push(format!("{}_codec: Codec({})", lower(&p.name), Self::tvar(&p.name)));
                        js_params.push(codec_var(&p.name));
                    }
                    let js_fn = if JS_RESERVED.contains(&with_id.as_str()) { format!("{with_id}_") } else { with_id.clone() };
                    self.out += &format!("@external(javascript, \"{}\", \"{js_fn}\")\npub fn {with_id}({}) -> {self_ty}\n\n", self.own_ffi(), params.join(", "));
                    self.ffi += &format!("export function {js_fn}({}) {{\n  self[{}] = {};\n  return self;\n}}\n", js_params.join(", "), arg_js[0], arg_js[1]);
                    self.clear_fn_bounds();
                    if prefix.is_none() {
                        emitted.push(EmittedWith {
                            gleam_name: with_id,
                            js_name: js_fn,
                            ffi_path: format!("{}_ffi.mjs", self.unit),
                            func: wf,
                        });
                    }
                }
                _ => {}
            }
        }
        emitted
    }
}

pub(super) fn emit_class_and_func_modules<'m>(
    g: &mut Gen<'m>,
    m: &'m Model,
    class_paths: &HashMap<String, String>,
    folded: &HashMap<String, (String, String)>,
    paths: &HashMap<String, String>,
    pkg: &str,
) {
    // Pass 1: non-folded classes, each in its own module.
    let mut emitted_methods: HashMap<String, Vec<EmittedMethod>> = HashMap::new();
    let mut emitted_withs: HashMap<String, Vec<EmittedWith>> = HashMap::new();
    for c in &m.classes {
        if folded.contains_key(&c.name) {
            continue;
        }
        g.switch(&class_paths[&c.name]);
        let own: Vec<Ty> = c.type_params.iter().map(|p| Ty::Param(p.name.clone())).collect();
        // The class's type, open (any subtype) or closed.
        let open = |g: &Gen| {
            *g.open.borrow_mut() = Some(0);
            let t = g.class_type(&c.name, &own);
            *g.open.borrow_mut() = None;
            t
        };
        for (constant, value) in &c.consts {
            let Some(gleam) = gleam_const_literal(value) else {
                g.skip("constant of another kind");
                continue;
            };
            let id = g.value_name(constant);
            g.out += &format!("pub const {id} = {gleam}\n\n");
        }
        let mut cls_methods = Vec::new();
        for f in &c.funcs {
            let f = with_self_args(f, c);
            let id = g.value_name(&function_name(&f, &f.name));
            let js_name = g.function(&id, &f, Some((&c.name, &c.type_params)));
            if Class::has_self(&f) {
                cls_methods.push(EmittedMethod {
                    gleam_name: id,
                    js_name,
                    ffi_path: format!("{}_ffi.mjs", class_paths[&c.name]),
                    func: f,
                });
            }
        }
        emitted_methods.insert(c.name.clone(), cls_methods);
        // Populate class_bound for synthesized to_list / get_at helpers.
        let populate_class_bound = |g: &Gen| {
            g.clear_fn_bounds();
            for (i, cp) in c.type_params.iter().enumerate() {
                if let Some(binding) = g.class_bound.get(&(c.name.clone(), i)) {
                    g.bound.borrow_mut().insert(cp.name.clone(), binding.clone());
                }
            }
        };
        // What it iterates, or holds as a subclass of Array: a list.
        if let Some(elem) = c.iterates.as_ref().or(c.elements.as_ref()) {
            let elem = with_self_args(&Func { type_params: vec![], module: String::new(), import: String::new(), name: String::new(), call: Call::Create(String::new()), params: vec![], rest: false, fixed: vec![], shared: None, ret: elem.clone(), returns_this: false }, c).ret;
            populate_class_bound(g);
            let id = g.value_name("to_list");
            let elem_ty = g.ty(&elem);
            let this = open(g);
            let mut params = vec![format!("self: {this}")];
            let mut js_params = vec!["self".to_string()];
            for p in &c.type_params {
                if g.crosses(&elem, &p.name) {
                    g.note("Codec");
                    params.push(format!("{}_codec: Codec({})", lower(&p.name), Gen::tvar(&p.name)));
                    js_params.push(codec_var(&p.name));
                }
            }
            g.out += &format!("@external(javascript, \"{}\", \"{id}\")\npub fn {id}({}) -> List({elem_ty})\n\n", g.own_ffi(), params.join(", "));
            g.ffi += &format!("export function {id}({}) {{\n  return toList(Array.from(self, (v) => {}));\n}}\n", js_params.join(", "), g.to_gleam(&elem, "v"));
            g.clear_fn_bounds();
        }
        // A subclass of Array has no class to inherit Array's members from:
        // its elements by index, and how many.
        if let Some(elem) = c.elements.as_ref() {
            let elem = with_self_args(&Func { type_params: vec![], module: String::new(), import: String::new(), name: String::new(), call: Call::Create(String::new()), params: vec![], rest: false, fixed: vec![], shared: None, ret: elem.clone(), returns_this: false }, c).ret;
            populate_class_bound(g);
            let elem_ty = g.ty(&elem);
            g.note("Option");
            let (at, length) = (g.value_name("get_at"), g.value_name("length"));
            let this = open(g);
            let mut at_params = vec![format!("self: {this}"), "index: Int".to_string()];
            let mut at_js_params = vec!["self".to_string(), "i".to_string()];
            for p in &c.type_params {
                if g.crosses(&elem, &p.name) {
                    g.note("Codec");
                    at_params.push(format!("{}_codec: Codec({})", lower(&p.name), Gen::tvar(&p.name)));
                    at_js_params.push(codec_var(&p.name));
                }
            }
            g.out += &format!("@external(javascript, \"{}\", \"{at}\")\npub fn {at}({}) -> Option({elem_ty})\n\n", g.own_ffi(), at_params.join(", "));
            let this = open(g);
            g.out += &format!("@external(javascript, \"{}\", \"{length}\")\npub fn {length}(self: {this}) -> Int\n\n", g.own_ffi());
            g.ffi += &format!(
                "export function {at}({}) {{\n  const v = self[i];\n  return v === undefined ? new None() : new Some({});\n}}\nexport function {length}(self) {{\n  return self.length;\n}}\n",
                at_js_params.join(", "),
                g.to_gleam(&elem, "v")
            );
            g.clear_fn_bounds();
        }
        if c.iterates_async.is_some() {
            g.skip("async iteration");
        }
        // Upcasts. Along the chain a subtype is its base already; this closes
        // the type at this class, for a list or a branch of several subtypes:
        // `element.upcast(div)` is an `Element(Nil)`.
        let id = g.value_name("upcast");
        let (this, closed) = (open(g), g.class_type(&c.name, &own));
        g.out += &format!("@external(javascript, \"{}\", \"dynamic\")\npub fn {id}(value: {this}) -> {closed}\n\n", g.runtime_ffi());
        // Covariant widening for generic classes with class-typed defaults:
        // `shape.widen(rect)` turns `Shape(RectConfig(Nil), RectTag(Nil))` into `Shape(ShapeConfig(Nil), Nil)`.
        if c.type_params.iter().any(|p| matches!(&p.default, Some(Ty::Obj(dn) | Ty::Generic(dn, _)) if g.classes.contains_key(dn))) {
            let wid = g.value_name("widen");
            let mut n = 0usize;
            let mut in_parts: Vec<String> = Vec::new();
            let mut out_parts: Vec<String> = Vec::new();
            for p in &c.type_params {
                match &p.default {
                    Some(Ty::Obj(dn)) if g.classes.contains_key(dn) => {
                        let count = g.classes[dn];
                        let mut sub_parts: Vec<String> = (0..count).map(|_| { n += 1; format!("w{n}_") }).collect();
                        n += 1;
                        sub_parts.push(format!("w{n}_"));
                        in_parts.push(format!("{}({})", g.gt(dn), sub_parts.join(", ")));
                        out_parts.push(g.class_type(dn, &[]));
                    }
                    Some(Ty::Generic(dn, dargs)) if g.classes.contains_key(dn) => {
                        let count = g.classes[dn];
                        let mut sub_parts: Vec<String> = (0..count).map(|i| match dargs.get(i) {
                            Some(Ty::Any) | None => { n += 1; format!("w{n}_") }
                            Some(a) => g.closed(|| g.ty(a)),
                        }).collect();
                        n += 1;
                        sub_parts.push(format!("w{n}_"));
                        in_parts.push(format!("{}({})", g.gt(dn), sub_parts.join(", ")));
                        out_parts.push(g.class_type(dn, dargs));
                    }
                    _ => {
                        let v = Gen::tvar(&p.name);
                        in_parts.push(v.clone());
                        out_parts.push(v);
                    }
                }
            }
            n += 1;
            in_parts.push(format!("w{n}_"));
            out_parts.push("Nil".into());
            let from = format!("{}({})", g.gt(&c.name), in_parts.join(", "));
            let to = format!("{}({})", g.gt(&c.name), out_parts.join(", "));
            g.out += &format!("@external(javascript, \"{}\", \"dynamic\")\npub fn {wid}(value: {from}) -> {to}\n\n", g.runtime_ffi());
        }
        // A base off the chain, whose members it has taken: `html_element.as_global_event_handlers(e)`.
        for a in secondary(m, c) {
            let Some(args) = base_args(m, c, &a) else { continue };
            let id = g.value_name(&format!("as_{}", snake(&g.tname(&a))));
            let (this, base) = (open(g), g.class_type(&a, &args));
            g.out += &format!("@external(javascript, \"{}\", \"dynamic\")\npub fn {id}(value: {this}) -> {base}\n\n", g.runtime_ffi());
        }
        // Downcasts, checked: only a class has a constructor to test against.
        if let Some(js) = &c.js {
            let test = format!("v instanceof {}", global(js));
            let target = g.class_type(&c.name, &[]);
            let root = chain_root(g, &c.name);
            if root != c.name {
                let id = if g.values.contains(&(g.unit.clone(), "from".into())) {
                    g.fresh_value("from_base")
                } else {
                    g.value_name("from")
                };
                *g.open.borrow_mut() = Some(0);
                let root_args: Vec<Ty> = (0..g.classes[&root]).map(|i| Ty::Param(format!("r_{i}"))).collect();
                let from = g.class_type(&root, &root_args);
                *g.open.borrow_mut() = None;
                g.out += &format!("@external(javascript, \"{}\", \"{id}\")\npub fn {id}(value: {from}) -> Result({target}, Nil)\n\n", g.own_ffi());
                g.ffi += &format!("export function {id}(v) {{\n  return {test} ? new Ok(v) : new GleamError(undefined);\n}}\n");
            }
            let id = if g.values.contains(&(g.unit.clone(), "cast".into())) {
                g.fresh_value("cast_dynamic")
            } else {
                g.value_name("cast")
            };
            g.note("Dynamic");
            g.out += &format!("@external(javascript, \"{}\", \"{id}\")\npub fn {id}(value: Dynamic) -> Result({target}, Nil)\n\n", g.own_ffi());
            g.ffi += &format!("export function {id}(v) {{\n  return {test} ? new Ok(v) : new GleamError(undefined);\n}}\n");
        }
        // Specialized primary-ancestor methods where substituting concrete generic args eliminates codecs.
        g.emit_specializations(c);
        // Fluent `with_<prop>` / `with_at` builders for option bags (`Call::Create`).
        let cls_withs = g.emit_with_builders(c, None);
        emitted_withs.insert(c.name.clone(), cls_withs);
        // Pure Gleam Key constants for KeyMap interfaces (`keyed_params = "map"`).
        if let Some(map) = m.maps.iter().find(|k| k.name == c.name) {
            let map_ty = g.closed(|| g.bare(&map.name));
            for (raw, val) in map.keys.iter().zip(&map.values) {
                let id = g.fresh_value(raw);
                let val_ty = g.closed(|| g.ty(val));
                g.note("Key");
                g.used_key_ctor = true;
                g.out += &format!("pub const {id}: Key({map_ty}, {val_ty}) = Key({raw:?})\n\n");
            }
        }
    }
    // Top-level and namespaced functions.
    for f in &m.funcs {
        let unit = func_unit(m, paths, pkg, f);
        g.switch(&unit);
        let id = match m.func_namespaces.get(&f.import) {
            Some((_, own, _, _)) => g.fresh_value(&function_name(f, own)),
            None => g.value_name(&function_name(f, &f.name)),
        };
        g.function(&id, f, None);
    }
    // Pass 3: folded single-owner synthetic option bags / result structs.
    for c in &m.classes {
        let Some((target_unit, prefix)) = folded.get(&c.name) else { continue };
        g.switch(target_unit);
        let own: Vec<Ty> = c.type_params.iter().map(|p| Ty::Param(p.name.clone())).collect();
        let open = |g: &Gen| {
            *g.open.borrow_mut() = Some(0);
            let t = g.class_type(&c.name, &own);
            *g.open.borrow_mut() = None;
            t
        };
        for (constant, value) in &c.consts {
            let Some(gleam) = gleam_const_literal(value) else {
                g.skip("constant of another kind");
                continue;
            };
            let id = g.fresh_value(&format!("{prefix}_{}", lower(constant)));
            g.out += &format!("pub const {id} = {gleam}\n\n");
        }
        for f in &c.funcs {
            let f = with_self_args(f, c);
            let id = g.fresh_value(&format!("{prefix}_{}", lower(&function_name(&f, &f.name))));
            g.function(&id, &f, Some((&c.name, &c.type_params)));
        }
        let id = g.fresh_value(&format!("{prefix}_upcast"));
        let (this, closed) = (open(g), g.class_type(&c.name, &own));
        g.out += &format!("@external(javascript, \"{}\", \"dynamic\")\npub fn {id}(value: {this}) -> {closed}\n\n", g.runtime_ffi());
        g.emit_with_builders(c, Some(prefix));
    }
    // Pass 2: zero-JS primary-chain inherited method & `with_*` builder aliases.
    // Emitted after own functions and folded synthetics so inherited aliases never shadow them.
    let total_inst_aliases: usize = m
        .classes
        .iter()
        .filter(|c| !folded.contains_key(&c.name))
        .map(|c| {
            let mut sum = 0;
            let mut at = c.name.clone();
            while let Some((p, _)) = g.parents.get(&at) {
                sum += emitted_methods.get(p).map_or(0, Vec::len);
                at = p.clone();
            }
            sum
        })
        .sum();
    let emit_all_inherited = total_inst_aliases <= MAX_INHERITED_ALIASES;
    for c in &m.classes {
        if folded.contains_key(&c.name) {
            continue;
        }
        g.switch(&class_paths[&c.name]);
        g.emit_inherited_aliases(c, &emitted_methods, &emitted_withs, emit_all_inherited);
    }
    g.switch("");
}
