// The wasm boundary, and the JavaScript glue every wasm generator shares.
//
// The boundary: numbers are f64, booleans i32, strings a pointer and a length
// going in. Everything else (objects, strings coming back, arrays, callbacks,
// `any`) is an integer handle into a table the glue keeps; 0 is null.
// A callback is a handle made from a wasm function pointer and a context pointer.

use std::collections::{BTreeMap, HashSet};

use super::js::{call_expr, global, js_test, trim_trailing_undef};
use super::model::{Call, Class, Func, Model, Shared, Ty, ident};
pub use super::model::manifest;

/// Primitive storage class on the raw wasm import/export ABI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbiSlot {
    Void,
    F64,
    I32,
    U32,
}

/// Wasm parameter lowering classification across the C/C++/Zig/Rust ABI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WasmParamAbi {
    None,
    One(AbiSlot),
    Str,
    OptNum,
}

/// Classifies a model parameter type for raw wasm import signatures.
pub fn param_abi(t: &Ty) -> WasmParamAbi {
    match t {
        Ty::Void => WasmParamAbi::None,
        Ty::Num | Ty::NumEnum(_) => WasmParamAbi::One(AbiSlot::F64),
        Ty::Bool | Ty::Enum(_) | Ty::Key(_) => WasmParamAbi::One(AbiSlot::I32),
        Ty::Str => WasmParamAbi::Str,
        Ty::Nullable(inner, _) if **inner == Ty::Str => WasmParamAbi::Str,
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Num | Ty::NumEnum(_)) => WasmParamAbi::OptNum,
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Bool | Ty::Enum(_)) => WasmParamAbi::One(AbiSlot::I32),
        _ => WasmParamAbi::One(AbiSlot::U32),
    }
}

/// Classifies a model return / callback ABI type for raw wasm signatures.
pub fn ret_abi(t: &Ty) -> AbiSlot {
    match t {
        Ty::Void => AbiSlot::Void,
        Ty::Num | Ty::NumEnum(_) => AbiSlot::F64,
        Ty::Bool | Ty::Enum(_) => AbiSlot::I32,
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Bool | Ty::Enum(_)) => AbiSlot::I32,
        _ => AbiSlot::U32,
    }
}

// The model of a wasm target: as `model::model`, with imports that differ only
// in their fixed arguments made one.
pub fn model(xml: &str, target: &str) -> Result<Model, Vec<String>> {
    match target {
        "C" => model_with(xml, super::c::MODEL_OPTIONS, &super::c::NAMING, &[]),
        "C++" => model_with(xml, super::cpp::MODEL_OPTIONS, &super::cpp::NAMING, &[]),
        "Zig" => model_with(xml, super::zig::MODEL_OPTIONS, &super::zig::NAMING, &[]),
        "Rust" => model_with(xml, super::rust::MODEL_OPTIONS, &super::rust::NAMING, &[]),
        other => {
            let mut m = super::model::model(xml, other)?;
            share_imports(&mut m);
            Ok(m)
        }
    }
}

pub fn model_with(
    xml: &str,
    opts: super::model::ModelOptions<'_>,
    naming: &super::names::Naming,
    extra_reserved: &[String],
) -> Result<Model, Vec<String>> {
    let mut m = super::model::model_with_options(xml, opts)?;
    super::names::settle_with(&mut m, naming, extra_reserved);
    share_imports(&mut m);
    Ok(m)
}

// Functions whose glue is the same but for their fixed arguments share one
// import, which takes the index of the function's arguments.
pub fn share_imports(m: &mut Model) {
    let mut groups: BTreeMap<String, Vec<(usize, usize)>> = BTreeMap::new();
    for (ci, c) in m.classes.iter().enumerate() {
        for (fi, f) in c.funcs.iter().enumerate().filter(|(_, f)| !f.fixed.is_empty()) {
            let (params, body) = entry(f, &|j, _| format!("$F{j}"));
            groups.entry(format!("{}|{body}", params.join(","))).or_default().push((ci, fi));
        }
    }
    for (_, members) in groups.into_iter().filter(|g| g.1.len() > 1) {
        let (ci, fi) = members[0];
        let first = &m.classes[ci].funcs[fi];
        let (params, body) = entry(first, &|j, _| format!("$F{j}"));
        // Named after what it calls: `addEventListener_keyed`.
        let what = match &first.call {
            Call::Method(js) => js.clone(),
            Call::New(path) | Call::Static(path) => path.rsplit('.').next().unwrap_or_default().to_string(),
            _ => first.import.clone(),
        };
        let name = unique_import(m, format!("{}_keyed", ident(&what)));
        let mut values = Vec::new();
        for (k, (ci, fi)) in members.into_iter().enumerate() {
            let f = &mut m.classes[ci].funcs[fi];
            values.push(f.fixed.iter().map(|x| x.1.clone()).collect());
            f.shared = Some((name.clone(), k));
        }
        m.shared.push(Shared { name, params, body, values });
    }
}


// An import name no function has yet.
fn unique_import(m: &Model, base: String) -> String {
    let taken: HashSet<&str> = m.all_funcs().map(|f| f.import.as_str()).chain(m.shared.iter().map(|s| s.name.as_str())).collect();
    let mut name = base.clone();
    let mut n = 2;
    while taken.contains(name.as_str()) {
        name = format!("{base}_{n}");
        n += 1;
    }
    name
}

// JavaScript expression turning a value from wasm into a JavaScript value.
fn from_wasm(p: &str, t: &Ty) -> String {
    match t {
        Ty::Num | Ty::NumEnum(_) => p.to_string(),
        Ty::Bool => format!("!!{p}"),
        Ty::Enum(e) => format!("enums.{e}[{p}]"),
        // A flag or an enum that may be absent is -1 when it is.
        Ty::Nullable(inner, u) if matches!(**inner, Ty::Bool) => format!("({p} < 0 ? {} : !!{p})", if *u { "undefined" } else { "null" }),
        Ty::Nullable(inner, u) if matches!(**inner, Ty::Enum(_)) => {
            let Ty::Enum(e) = &**inner else { unreachable!() };
            format!("({p} < 0 ? {} : enums.{e}[{p}])", if *u { "undefined" } else { "null" })
        }
        Ty::Nullable(_, true) => format!("({p} === 0 ? undefined : get({p}))"),
        Ty::Key(m) => format!("keys.{m}[{p}]"),
        _ => format!("get({p})"),
    }
}

// JavaScript expression turning a JavaScript value into one wasm can take.
fn to_wasm(x: &str, t: &Ty) -> String {
    match t {
        Ty::Void => format!("void ({x})"),
        Ty::Num | Ty::NumEnum(_) => x.to_string(),
        Ty::Bool => format!("(({x}) ? 1 : 0)"),
        Ty::Enum(e) => format!("(places.{e}.get({x}) ?? -1)"),
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Bool) => format!("(v => (v == null ? -1 : v ? 1 : 0))({x})"),
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Enum(_)) => {
            let Ty::Enum(e) = &**inner else { unreachable!() };
            format!("(places.{e}.get({x}) ?? -1)")
        }
        _ => format!("put({x})"),
    }
}

// The glue of one import: its parameters and body. `fixed` writes the j-th
// fixed argument, given its literal.
fn entry(f: &Func, fixed: &dyn Fn(usize, &str) -> String) -> (Vec<String>, String) {
    let mut wasm_params = Vec::new();
    let mut args = Vec::new();
    let mut may_undef = Vec::new();
    if Class::has_self(f) {
        wasm_params.push("self".to_string());
    }
    for (i, (_, t)) in f.params.iter().enumerate() {
        let p = format!("a{i}");
        // A string is a pointer and a length; everything else is one value.
        if *t == Ty::Str {
            wasm_params.push(format!("{p}_ptr"));
            wasm_params.push(format!("{p}_len"));
            args.push(format!("str({p}_ptr, {p}_len)"));
            may_undef.push(false);
            continue;
        }
        if let Ty::Nullable(inner, undefined) = t
            && **inner == Ty::Str
        {
            wasm_params.push(format!("{p}_ptr"));
            wasm_params.push(format!("{p}_len"));
            args.push(format!("({p}_ptr === 0 ? {} : str({p}_ptr, {p}_len))", if *undefined { "undefined" } else { "null" }));
            may_undef.push(*undefined);
            continue;
        }
        if let Ty::Nullable(inner, undefined) = t
            && matches!(**inner, Ty::Num | Ty::NumEnum(_))
        {
            wasm_params.push(format!("{p}_has"));
            wasm_params.push(format!("{p}_value"));
            args.push(format!("({p}_has ? {p}_value : {})", if *undefined { "undefined" } else { "null" }));
            may_undef.push(*undefined);
            continue;
        }
        // A rest parameter's array is spread; a null one is empty.
        if f.rest && i == f.params.len() - 1 {
            args.push(format!("...({} ?? [])", from_wasm(&p, t)));
            may_undef.push(false);
        } else {
            args.push(from_wasm(&p, t));
            may_undef.push(matches!(t, Ty::Nullable(_, true)));
        }
        wasm_params.push(p);
    }
    for (j, (position, literal)) in f.fixed.iter().enumerate() {
        let pos = (*position).min(args.len());
        args.insert(pos, fixed(j, literal));
        may_undef.insert(pos, false);
    }
    trim_trailing_undef(&f.call, f.rest, &mut args, &may_undef, "trim");
    let call = call_expr(&f.call, "get(self)", &args);
    let body = match &f.ret {
        Ty::Void => format!("{{ {call}; }}"),
        t => to_wasm(&call, t),
    };
    (wasm_params, body)
}

// JavaScript that implements every import by calling the real API.
// The functions a compiled module imports from `env` (or `module_name`): what the glue must provide.
pub fn imports(wasm: &[u8]) -> HashSet<String> {
    imports_for(wasm, "env")
}

pub fn imports_for(wasm: &[u8], module_name: &str) -> HashSet<String> {
    fn leb(b: &[u8], i: &mut usize) -> usize {
        let (mut v, mut shift) = (0usize, 0);
        loop {
            let byte = b[*i];
            *i += 1;
            v |= ((byte & 0x7f) as usize) << shift;
            if byte & 0x80 == 0 {
                return v;
            }
            shift += 7;
        }
    }
    fn name(b: &[u8], i: &mut usize) -> String {
        let n = leb(b, i);
        let s = String::from_utf8_lossy(&b[*i..*i + n]).into_owned();
        *i += n;
        s
    }
    let mut out = HashSet::new();
    let mut i = 8;
    while i < wasm.len() {
        let id = wasm[i];
        i += 1;
        let size = leb(wasm, &mut i);
        let end = i + size;
        if id == 2 {
            for _ in 0..leb(wasm, &mut i) {
                let module = name(wasm, &mut i);
                let field = name(wasm, &mut i);
                let kind = wasm[i];
                i += 1;
                match kind {
                    0 => {
                        leb(wasm, &mut i);
                    }
                    1 => {
                        i += 1;
                        let flags = leb(wasm, &mut i);
                        leb(wasm, &mut i);
                        if flags & 1 != 0 {
                            leb(wasm, &mut i);
                        }
                    }
                    2 => {
                        let flags = leb(wasm, &mut i);
                        leb(wasm, &mut i);
                        if flags & 1 != 0 {
                            leb(wasm, &mut i);
                        }
                    }
                    3 => i += 2,
                    _ => {
                        i += 1;
                        leb(wasm, &mut i);
                    }
                }
                if module == "env" || module == module_name {
                    out.insert(field);
                }
            }
        }
        i = end;
    }
    out
}

// The glue for every import in the model, or with `used`, only those a compiled module needs.
pub fn glue(m: &Model, used: Option<&HashSet<String>>) -> String {
    glue_for_module(m, used, "env")
}

pub fn glue_for_module(m: &Model, used: Option<&HashSet<String>>, wasm_module: &str) -> String {
    let wanted = |name: &str| used.is_none_or(|u| u.contains(name));
    // A value handed to a callback: wasm's own where it counts handles, else
    // lent, and released when the callback returns.
    let lend = |args: &[String]| -> (String, String) {
        if m.counted {
            return (args.iter().map(|a| a.replace("put(", "ref(")).collect::<Vec<_>>().join(", "), String::new());
        }
        let mut pre = String::new();
        let mut post = String::new();
        let mut out = Vec::new();
        for (i, a) in args.iter().enumerate() {
            match a.strip_prefix("put(") {
                Some(_) => {
                    pre += &format!("const h{i} = {}; ", a.replace("put(", "ref("));
                    post += &format!("release(h{i}); ");
                    out.push(format!("h{i}"));
                }
                None => out.push(a.clone()),
            }
        }
        (format!("{pre}\u{1}{}", out.join(", ")), post)
    };
    // `call` with its lent arguments released afterwards.
    let lending = |call: &dyn Fn(&str) -> String, args: &[String]| -> String {
        let (args, post) = lend(args);
        match args.split_once('\u{1}') {
            Some((pre, args)) if !post.is_empty() => format!("{pre}try {{ {} }} finally {{ {post}}}", call(args)),
            Some((_, args)) => call(args),
            None => call(&args),
        }
    };
    let listener = lending(&|args| format!("f(ctx, {args});"), &["put(ev)".to_string()]);
    let settle = lending(&|args| format!("table().get(fn)(ctx, ok, {args});"), &["put(v)".to_string()]);
    let mut out = include_str!("runtime/wasm_runtime.mjs")
        .replace("/*__LISTENER__*/", &listener)
        .replace("/*__SETTLE__*/", &settle);
    out += "\nexport function makeEnv(getInstance, options = {}) {\n\
            \x20 const rt = options.runtime ?? createRuntime(getInstance, options);\n\
            \x20 const { ref, put, release, get, take, str, mem, table, fail, guard, callback, dead, trim, env: rtEnv } = rt;\n\
            \x20 const G = options.root ?? globalThis;\n\
            \x20 // With several packages bound together, each one's module by its name.\n\
            \x20 const M = options.modules ?? {};\n";
    // Each package's module the program reaches: one asked for is given,
    // or makeEnv fails here, not at a call; one a declaration only comes
    // from (node's `stream`) fails where it is used, if not given.
    let used = m.runtime_packages();
    let mut packages: Vec<&String> = m.packages.values().filter(|p| used.contains(*p)).collect();
    packages.sort();
    packages.dedup();
    let alone = packages.len() == 1 && m.requested_packages.len() <= 1;
    let mut pk = String::from("  const PK = {};\n");
    for p in packages {
        let given = if alone { format!("M[{p:?}] ?? options.root") } else { format!("M[{p:?}]") };
        let missing = format!("makeEnv: no module for the package {p:?}: give it in options.modules{}", if alone { " or options.root" } else { "" });
        if m.requested_packages.is_empty() || m.requested_packages.contains(p) {
            pk += &format!("  PK[{p:?}] = {given};\n  if (PK[{p:?}] === undefined) throw new Error({missing:?});\n");
        } else {
            pk += &format!("  PK[{p:?}] = {given} ?? new Proxy({{}}, {{ get() {{ throw new Error({missing:?}); }} }});\n");
        }
    }
    out += &pk;
    out += "  const enums = {\n";
    for (name, values) in &m.enums {
        out += &format!("    {name}: {values:?},\n");
    }
    // The keys of each map, by index.
    out += "  };\n  const keys = {\n";
    for k in &m.maps {
        out += &format!("    {}: {:?},\n", k.name, k.keys);
    }
    out += "  };\n  const enumList = Object.values(enums);\n  // Each enum's values by name, to their place: looked up, not searched for.\n  const places = {};\n  for (const name in enums) places[name] = new Map(enums[name].map((v, i) => [v, i]));\n  const placeList = Object.values(places);\n  const pkgEnv = {\n    // An enum's value as a handle, and back as its index: enums by the order they are declared in.\n    js_enum_value: (id, i) => put(enumList[id][i]),\n    js_enum_index: guard((id, h) => placeList[id].get(get(h)) ?? -1),\n";
    // Which member of a union a value is: its index, or -1.
    for u in m.unions.iter().filter(|u| wanted(&format!("{}_tag", u.name))) {
        let mut arms = Vec::new();
        let mut default = None;
        // A bare type parameter: `k` says what its argument is, 1 a number, 2 a string, 3 a flag, 0 anything else.
        let mut param = None;
        for (i, (test, part)) in u.tests.iter().zip(&u.parts).enumerate() {
            match test {
                _ if matches!(part, Ty::Param(_)) => {
                    arms.insert(0, format!("if (typeof v === [, 'number', 'string', 'boolean'][k]) return {i};"));
                    param = Some(i);
                }
                Some(t) => arms.push(format!("if ({}) return {i};", js_test(t, u.discriminant.as_deref(), &m.js_refs))),
                None => default = Some(i),
            }
        }
        let default = match (default, param) {
            (Some(d), _) => d.to_string(),
            (None, Some(p)) => format!("(k === 0 ? {p} : -1)"),
            (None, None) => "-1".into(),
        };
        out += &format!("    {}_tag: guard((h, k) => {{ const v = get(h); {} return {default}; }}),\n", u.name, arms.join(" "));
    }
    // Checked downcasts: whether a handle is an instance of class `c` (1 or 0).
    for c in m.classes.iter().filter(|c| c.js.is_some() && wanted(&format!("js_is__{}", c.name))) {
        let js = c.js.as_deref().unwrap();
        out += &format!("    js_is__{}: guard(h => {{ const c = {}; return h !== 0 && typeof c === 'function' && get(h) instanceof c ? 1 : 0; }}),\n", c.name, global(js));
    }
    for c in m.callbacks.iter().filter(|c| wanted(&format!("{}_from", c.name))) {
        let names: Vec<String> = (0..c.params.len()).map(|i| format!("a{i}")).collect();
        let args: Vec<String> = c.params.iter().zip(&names).map(|((_, t), a)| to_wasm(a, t)).collect();
        // What a callback returns, wasm gives up.
        let ret = from_wasm("r", &c.ret).replace("get(r)", "take(r)");
        let body = lending(
            &|args| {
                let call = if args.is_empty() { "f(ctx)".to_string() } else { format!("f(ctx, {args})") };
                if c.ret == Ty::Void { format!("{call};") } else { format!("const r = {call}; return {ret};") }
            },
            &args,
        );
        // The rest of the arguments, gathered.
        let mut names = names;
        if c.rest && let Some(last) = names.last_mut() {
            *last = format!("...{last}");
        }
        out += &format!("    {}_from: (fn, ctx) => callback({:?}, fn, ctx, (f, s) => ({}) => {{ if (!s.alive) dead(); {body} }}),\n", c.name, c.name, names.join(", "));
    }
    // A call that throws returns zero, and its exception is kept for wasm to take.
    let guarded = |body: &str| match body.strip_prefix('{') {
        Some(_) => format!("{{ try {body} catch (e) {{ fail(e); }} }}"),
        None => format!("{{ try {{ return {body}; }} catch (e) {{ return fail(e); }} }}"),
    };
    for f in m.all_funcs().filter(|f| f.shared.is_none() && wanted(&f.import)) {
        let (params, body) = entry(f, &|_, literal| literal.to_string());
        out += &format!("    {}: ({}) => {},\n", f.import, params.join(", "), guarded(&body));
    }
    // A shared import takes the index of the fixed arguments after `self`.
    for s in m.shared.iter().filter(|s| wanted(&s.name)) {
        let mut params = s.params.clone();
        let at = usize::from(params.first().is_some_and(|p| p == "self"));
        params.insert(at, "k".into());
        let mut body = s.body.clone();
        for j in 0..s.values.first().map_or(0, Vec::len) {
            body = body.replace(&format!("$F{j}"), &format!("F[k][{j}]"));
        }
        let table: Vec<String> = s.values.iter().map(|v| format!("[{}]", v.join(", "))).collect();
        out += &format!("    {}: (() => {{ const F = [{}]; return ({}) => {}; }})(),\n", s.name, table.join(", "), params.join(", "), guarded(&body));
    }
    out + &format!(
        "  }};\n\
         \x20 const env = {{ ...rtEnv, ...pkgEnv }};\n\
         \x20 Object.defineProperties(env, {{\n\
         \x20   $runtime: {{ value: rt }},\n\
         \x20   $module: {{ value: {wasm_module:?} }},\n\
         \x20   $pkg: {{ value: pkgEnv }},\n\
         \x20 }});\n\
         \x20 return env;\n\
         }}\n"
    )
}
