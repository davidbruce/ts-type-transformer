// The wasm boundary, and the JavaScript glue every wasm generator shares.
//
// The boundary: numbers are f64, booleans i32, strings a pointer and a length
// going in. Everything else (objects, strings coming back, arrays, callbacks,
// `any`) is an integer handle into a table the glue keeps; 0 is null.
// A callback is a handle made from a wasm function pointer and a context pointer.

use std::collections::{BTreeMap, HashSet};

use super::js::{access, access_path, global, js_test};
use super::model::{Call, Class, Func, Model, Shared, Ty, ident};

// The model of a wasm target: as `model::model`, with imports that differ only
// in their fixed arguments made one.
pub fn model(xml: &str, target: &str) -> Result<Model, Vec<String>> {
    let mut m = super::model::model(xml, target)?;
    let naming = match target {
        "C" => Some(&super::c::NAMING),
        "C++" => Some(&super::cpp::NAMING),
        "Zig" => Some(&super::zig::NAMING),
        "Rust" => Some(&super::rust::NAMING),
        _ => None,
    };
    if let Some(n) = naming {
        super::names::settle(&mut m, n);
    }
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
            continue;
        }
        if let Ty::Nullable(inner, undefined) = t
            && **inner == Ty::Str
        {
            wasm_params.push(format!("{p}_ptr"));
            wasm_params.push(format!("{p}_len"));
            args.push(format!("({p}_ptr === 0 ? {} : str({p}_ptr, {p}_len))", if *undefined { "undefined" } else { "null" }));
            continue;
        }
        if let Ty::Nullable(inner, undefined) = t
            && matches!(**inner, Ty::Num | Ty::NumEnum(_))
        {
            wasm_params.push(format!("{p}_has"));
            wasm_params.push(format!("{p}_value"));
            args.push(format!("({p}_has ? {p}_value : {})", if *undefined { "undefined" } else { "null" }));
            continue;
        }
        // A rest parameter's array is spread; a null one is empty.
        if f.rest && i == f.params.len() - 1 {
            args.push(format!("...({} ?? [])", from_wasm(&p, t)));
        } else {
            args.push(from_wasm(&p, t));
        }
        wasm_params.push(p);
    }
    for (j, (position, literal)) in f.fixed.iter().enumerate() {
        args.insert((*position).min(args.len()), fixed(j, literal));
    }
    let (first, second) = (args.first().cloned().unwrap_or_default(), args.get(1).cloned().unwrap_or_default());
    let args = args.join(", ");
    let call = match &f.call {
        Call::Method(js) => format!("get(self){}({args})", access(js)),
        Call::Get(prop) => format!("get(self){}", access(prop)),
        Call::Set(prop) => format!("get(self){} = {args}", access(prop)),
        Call::New(path) => format!("new ({})({args})", global(path)),
        Call::Static(path) => format!("{}({args})", global(path)),
        Call::StaticGet(path) => global(path),
        // A variable of the library's own module cannot be assigned from outside it.
        Call::StaticSet(path) if !path.contains('.') => format!("globalThis{} = {args}", access_path(path)),
        Call::StaticSet(path) => format!("{} = {args}", global(path)),
        Call::Create => "({})".to_string(),
        Call::Invoke => format!("get(self)({args})"),
        Call::Construct => format!("new (get(self))({args})"),
        Call::IndexGet => format!("get(self)[{args}]"),
        Call::IndexSet => format!("get(self)[{first}] = {second}"),
    };
    let body = match &f.ret {
        Ty::Void => format!("{{ {call}; }}"),
        t => to_wasm(&call, t),
    };
    (wasm_params, body)
}

// What each import reaches, for a page to check against the real API:
// `export const bindings = [[import, kind, owner, member, class], ...]`, the
// owner being where its class's constructor is (`HTMLElement`), or, for an
// interface no constructor is of (`GlobalEventHandlers`), `interface:` its
// name; and `implementers`, the constructors of each class's or interface's
// descendants (Chrome has `AbstractRange`'s members on `Range`'s prototype).
pub fn manifest(m: &Model) -> String {
    let kind = |c: &Call| match c {
        Call::Method(_) => "method",
        Call::Get(_) => "get",
        Call::Set(_) => "set",
        Call::New(_) => "new",
        Call::Static(_) => "static",
        Call::StaticGet(_) => "staticGet",
        Call::StaticSet(_) => "staticSet",
        Call::Create => "create",
        Call::Invoke => "invoke",
        Call::Construct => "construct",
        Call::IndexGet => "indexGet",
        Call::IndexSet => "indexSet",
    };
    let member = |c: &Call| match c {
        Call::Method(n) | Call::Get(n) | Call::Set(n) | Call::New(n) | Call::Static(n) | Call::StaticGet(n) | Call::StaticSet(n) => n.clone(),
        _ => String::new(),
    };
    let json = |s: &str| format!("{s:?}");
    let mut out = String::from("// Generated. What each import reaches.\nexport const bindings = [\n");
    for c in &m.classes {
        let owner = c.js.clone().unwrap_or_else(|| format!("interface:{}", c.name));
        for f in &c.funcs {
            out += &format!("  [{}, {}, {}, {}, {}],\n", json(&f.import), json(kind(&f.call)), json(&owner), json(&member(&f.call)), json(&c.name));
        }
    }
    for f in &m.funcs {
        out += &format!("  [{}, {}, \"\", {}, \"\"],\n", json(&f.import), json(kind(&f.call)), json(&member(&f.call)));
    }
    out += "];\nexport const implementers = {\n";
    let mut by: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for c in m.classes.iter().filter(|c| c.js.is_some()) {
        for a in c.ancestors.iter().chain(std::iter::once(&c.name)) {
            by.entry(a.as_str()).or_default().push(c.js.as_deref().unwrap());
        }
    }
    for c in &m.classes {
        let js = by.get(c.name.as_str()).cloned().unwrap_or_default();
        out += &format!("  {}: [{}],\n", json(&c.name), js.iter().map(|j| json(j)).collect::<Vec<_>>().join(", "));
    }
    out += "};\n";
    out
}

// JavaScript that implements every import by calling the real API.
// The functions a compiled module imports from `env`: what the glue must provide.
pub fn imports(wasm: &[u8]) -> HashSet<String> {
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
                if module == "env" {
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
    let mut out = String::from(
        "// Generated. Implements the wasm imports by calling the JavaScript API.
//
// makeEnv(getInstance, options): `options.root` is where the API's names
// are, when not all are globals: a library's module, say; `options.modules`,
// with several packages, each one's module by its name. A package bound
// must be given. `options.check`
// reports mistakes with handles (one used after its release, or released
// twice), at some cost in speed; `options.unchecked(e)` is called with an
// exception no one took, and by default writes it to the console.
export function makeEnv(getInstance, options = {}) {
  const G = options.root ?? globalThis;
  // With several packages bound together, each one's module by its name.
  const M = options.modules ?? {};
  const CHECK = !!options.check;
  const RELEASED = Symbol('released');
  // Values by handle. Each `ref` is a reference wasm owns, in a slot of its
  // own; the slot is freed when its count reaches zero. Zero is null.
  const objs = [null];
  const refs = [0];
  const free = [];
  let live = 0;
  const ref = v => {
    if (v === null || v === undefined) return 0;
    // When checking, a slot is never reused: a stale handle stays stale.
    const h = free.length ? free.pop() : objs.length;
    objs[h] = v;
    refs[h] = 1;
    live++;
    return h;
  };
  // Open scopes: each holds the handles given to wasm while it is innermost,
  // released when it closes.
  const scopes = [];
  const put = v => {
    const h = ref(v);
    if (h !== 0 && scopes.length !== 0) scopes[scopes.length - 1].push(h);
    return h;
  };
  const release = h => {
    if (h === 0) return;
    if (!(refs[h] > 0)) return fail(new Error(`handle ${h} released twice`));
    if (--refs[h] === 0) {
      live--;
      if (CHECK) objs[h] = RELEASED;
      else {
        objs[h] = undefined;
        free.push(h);
      }
    }
  };
  const get = CHECK
    ? h => {
        const v = objs[h];
        if (v === RELEASED) throw new Error(`handle ${h} used after its release`);
        return v;
      }
    : h => objs[h];
  // A handle wasm gave up: what a callback returns.
  const take = h => {
    const v = get(h);
    release(h);
    return v;
  };
  // The exception a call threw, until wasm takes it: a handle, or 0. One that
  // is not taken by the time control returns to JavaScript, or before the
  // next one, is reported.
  let error = 0;
  const unchecked = options.unchecked ?? (e => console.error('Uncaught in a wasm call:', e));
  const fail = e => {
    if (error !== 0) {
      const old = objs[error];
      release(error);
      unchecked(old);
    }
    const mine = (error = ref(e));
    queueMicrotask(() => {
      if (error === mine && objs[mine] === e) {
        error = 0;
        release(mine);
        unchecked(e);
      }
    });
    return 0;
  };
  // Runtime calls that can throw, with up to four arguments.
  const guard = f => (a, b, c, d) => {
    try {
      return f(a, b, c, d);
    } catch (e) {
      return fail(e);
    }
  };
  // The module's memory, as bytes: a new view when it has grown.
  let bytes = new Uint8Array(0);
  const mem = () => (bytes.byteLength !== 0 ? bytes : (bytes = new Uint8Array(getInstance().exports.memory.buffer)));
  const table = () => getInstance().exports.__indirect_function_table;
  const enc = new TextEncoder(), dec = new TextDecoder();
  // A string from memory. Short ones are usually ASCII, read faster by hand.
  const decode = (m, ptr, len) => {
    if (len <= 16) {
      let s = '';
      for (let i = 0; i < len; i++) {
        const c = m[ptr + i];
        if (c > 127) return dec.decode(m.subarray(ptr, ptr + len));
        s += String.fromCharCode(c);
      }
      return s;
    }
    return dec.decode(m.subarray(ptr, ptr + len));
  };
  // Strings seen at an address, reused while the bytes there are the same:
  // most strings a program passes are its literals, passed again and again.
  const seen = new Map();
  const str = (ptr, len) => {
    const m = mem();
    const key = ptr * 65536 + len;
    const e = seen.get(key);
    if (e !== undefined) {
      const b = e.bytes;
      let i = 0;
      while (i < len && m[ptr + i] === b[i]) i++;
      if (i === len) return e.s;
      // A buffer that keeps changing is not worth remembering.
      if (e.misses++ > 2) return decode(m, ptr, len);
    }
    const s = decode(m, ptr, len);
    if (len > 256) return s;
    if (seen.size > 1024) seen.clear();
    seen.set(key, { s, bytes: m.slice(ptr, ptr + len), misses: e === undefined ? 0 : e.misses });
    return s;
  };
  // The length of a string in UTF-8.
  const utf8Length = s => {
    const n = s.length;
    for (let i = 0; i < n; i++) if (s.charCodeAt(i) > 127) return enc.encode(s).length;
    return n;
  };
  // Writes at most `cap` bytes of a string to memory; returns its whole length in UTF-8.
  const write = (s, ptr, cap) => {
    const m = mem();
    const n = s.length;
    const fits = n <= cap ? n : cap;
    let i = 0;
    for (; i < fits; i++) {
      const c = s.charCodeAt(i);
      if (c > 127) break;
      m[ptr + i] = c;
    }
    if (i === n) return n;
    if (i === fits && n > cap) return utf8Length(s);
    const r = enc.encodeInto(s, m.subarray(ptr, ptr + cap));
    return r.read === n ? r.written : utf8Length(s);
  };
  const text = h => String(get(h) ?? '');
  const viewOf = (h, off, len) => {
    const v = get(h);
    if (v == null) return new Uint8Array(0);
    const buf = ArrayBuffer.isView(v) ? v.buffer : v;
    const base = ArrayBuffer.isView(v) ? v.byteOffset : 0;
    const total = v.byteLength >>> 0;
    const start = Math.min(off >>> 0, total);
    const count = len === undefined ? total - start : Math.min(len >>> 0, total - start);
    return new Uint8Array(buf, base + start, count);
  };
  const bufferCtors = [
    null,
    typeof SharedArrayBuffer !== 'undefined' ? SharedArrayBuffer : null,
    Int8Array,
    Uint8Array,
    Uint8ClampedArray,
    Int16Array,
    Uint16Array,
    Int32Array,
    Uint32Array,
    Float32Array,
    Float64Array,
    typeof BigInt64Array !== 'undefined' ? BigInt64Array : null,
    typeof BigUint64Array !== 'undefined' ? BigUint64Array : null,
    DataView,
  ];
  // One JavaScript function per wasm function and context, so that a
  // callback passed twice is the same function (removeEventListener).
  const callbacks = new Map();
  // Callbacks made here: their key and context, until released.
  const owned = new Map();
  const dead = () => {
    throw new Error('callback called after release');
  };
  const callback = (kind, fn, ctx, make) => {
    const key = `${kind}:${fn}:${ctx}`;
    let f = callbacks.get(key);
    if (f === undefined) {
      const state = { key, ctx, alive: true };
      f = make(table().get(fn), state);
      owned.set(f, state);
      callbacks.set(key, f);
    }
    return put(f);
  };
  const enums = {
",
    );
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
    out = out.replacen("  const CHECK = !!options.check;\n", &format!("{pk}  const CHECK = !!options.check;\n"), 1);
    for (name, values) in &m.enums {
        out += &format!("    {name}: {values:?},\n");
    }
    // The keys of each map, by index.
    out += "  };\n  const keys = {\n";
    for k in &m.maps {
        out += &format!("    {}: {:?},\n", k.name, k.keys);
    }
    let listener = lending(&|args| format!("f(ctx, {args});"), &["put(ev)".to_string()]);
    let settle = lending(&|args| format!("table().get(fn)(ctx, ok, {args});"), &["put(v)".to_string()]);
    out += &format!(
        "  }};
  const enumList = Object.values(enums);
  // Each enum's values by name, to their place: looked up, not searched for.
  const places = {{}};
  for (const name in enums) places[name] = new Map(enums[name].map((v, i) => [v, i]));
  const placeList = Object.values(places);
  const env = {{
    js_string_len: guard(h => utf8Length(text(h))),
    js_string_read: guard((h, ptr) => {{ write(text(h), ptr, 0x7fffffff); }}),
    // At most `cap` bytes of the string; returns its whole length.
    js_string_copy: guard((h, ptr, cap) => write(text(h), ptr, cap)),
    // As js_string_copy, and if it fit, the handle is released.
    js_string_take: guard((h, ptr, cap) => {{ const n = write(text(h), ptr, cap); if (n <= cap) release(h); return n; }}),
    js_string_equals: guard((h, ptr, len) => (text(h) === str(ptr, len) ? 1 : 0)),
    js_from_number: x => put(x),
    js_from_bool: x => put(!!x),
    js_from_string: (ptr, len) => put(str(ptr, len)),
    js_to_number: guard(h => Number(get(h))),
    js_to_bool: guard(h => (get(h) ? 1 : 0)),
    // Whether two handles are one JavaScript value.
    js_equals: guard((a, b) => (a === b || get(a) === get(b) ? 1 : 0)),
    js_array_new: () => put([]),
    js_array_length: guard(h => get(h).length),
    js_array_get: guard((h, i) => put(get(h)[i])),
    js_array_push: guard((h, v) => {{ get(h).push(get(v)); }}),
    js_listener_from: (fn, ctx) => callback('listener', fn, ctx, (f, s) => ev => {{ if (!s.alive) dead(); {listener} }}),
    // Forgets a callback made here; returns its context, for wasm to free, or 0.
    js_callback_release: guard(h => {{
      const o = owned.get(get(h));
      if (!o) return 0;
      owned.delete(get(h));
      callbacks.delete(o.key);
      o.alive = false;
      return o.ctx;
    }}),
    js_retain: guard(h => {{ if (h !== 0 && get(h) !== undefined) refs[h]++; }}),
    js_release: release,
    js_scope_open: () => {{ scopes.push([]); }},
    js_scope_close: () => {{ (scopes.pop() ?? []).forEach(release); }},
    // References wasm holds.
    js_live: () => live,
    // An enum's value as a handle, and back as its index: enums by the order they are declared in.
    js_enum_value: (id, i) => put(enumList[id][i]),
    js_enum_index: guard((id, h) => placeList[id].get(get(h)) ?? -1),
    // Iteration: an iterator over a value, and its next value, or -1 at the end.
    js_iterate: guard(h => put(get(h)[Symbol.iterator]())),
    js_iterator_next: guard(it => {{ const r = get(it).next(); return r.done ? -1 : put(r.value); }}),
    // Asynchronous iteration: a promise of the next value, or of nothing at the end.
    js_async_iterate: guard(h => put(get(h)[Symbol.asyncIterator]())),
    js_async_iterator_next: guard(it => put(Promise.resolve(get(it).next()).then(r => (r.done ? undefined : r.value)))),
    // Whether an exception waits to be taken; the exception, which wasm now holds, or 0.
    js_has_error: () => (error !== 0 ? 1 : 0),
    js_take_error: () => {{ const e = error; error = 0; if (e !== 0 && scopes.length !== 0) scopes[scopes.length - 1].push(e); return e; }},
    js_error_message: guard(h => {{ const e = get(h); return put(String(e instanceof Error ? e.message : (e && typeof e === 'object' && 'message' in e ? e.message : e))); }}),
    js_error_name: guard(h => {{ const e = get(h); return put(String(e instanceof Error ? e.name : (e && typeof e === 'object' && 'name' in e ? e.name : typeof e))); }}),
    js_error_stack: guard(h => {{ const e = get(h); return put(String(e && typeof e === 'object' && e.stack != null ? e.stack : '')); }}),
    js_error_new: guard((ptr, len) => put(new Error(str(ptr, len)))),
    js_date_new: ms => put(new Date(ms)),
    js_date_now: () => Date.now(),
    js_date_time: guard(h => {{ const d = get(h); return d instanceof Date ? d.getTime() : Number(d); }}),
    js_date_to_iso_string: guard(h => put(get(h).toISOString())),
    js_bytes_len: guard(h => {{ const v = get(h); return v == null ? 0 : (v.byteLength >>> 0); }}),
    js_bytes_read: guard((h, srcOff, dstPtr, len) => {{ const s = viewOf(h, srcOff, len); if (s.byteLength) mem().set(s, dstPtr >>> 0); return s.byteLength; }}),
    js_bytes_write: guard((h, dstOff, srcPtr, len) => {{ const d = viewOf(h, dstOff, len); const p = srcPtr >>> 0; if (d.byteLength) d.set(mem().subarray(p, p + d.byteLength)); return d.byteLength; }}),
    js_bytes_new: guard((kind, srcPtr, byteLen) => {{
      const n = byteLen >>> 0;
      const p = srcPtr >>> 0;
      if (kind === 0) {{
        const ab = new ArrayBuffer(n);
        if (p !== 0 && n > 0) new Uint8Array(ab).set(mem().subarray(p, p + n));
        return put(ab);
      }}
      if (kind === 1) {{
        const Ctor = bufferCtors[1];
        if (!Ctor) throw new TypeError('SharedArrayBuffer is not available');
        const sab = new Ctor(n);
        if (p !== 0 && n > 0) new Uint8Array(sab).set(mem().subarray(p, p + n));
        return put(sab);
      }}
      const Ctor = bufferCtors[kind];
      if (!Ctor) throw new TypeError(`unsupported buffer kind ${{kind}}`);
      if (kind === 13) {{
        const ab = new ArrayBuffer(n);
        if (p !== 0 && n > 0) new Uint8Array(ab).set(mem().subarray(p, p + n));
        return put(new DataView(ab));
      }}
      const count = (n / Ctor.BYTES_PER_ELEMENT) >>> 0;
      const ta = new Ctor(count);
      if (p !== 0 && count > 0) new Uint8Array(ta.buffer, ta.byteOffset, ta.byteLength).set(mem().subarray(p, p + ta.byteLength));
      return put(ta);
    }}),
    js_bytes_buffer: guard(h => {{ const v = get(h); return put(ArrayBuffer.isView(v) ? v.buffer : v); }}),
    js_bytes_offset: guard(h => {{ const v = get(h); return ArrayBuffer.isView(v) ? (v.byteOffset >>> 0) : 0; }}),
    // Calls fn(ctx, ok, value) once the promise settles: ok 1 with its value, 0 with its reason.
    js_promise_then: guard((p, fn, ctx) => {{
      const settle = ok => v => {{ {settle} }};
      Promise.resolve(get(p)).then(settle(1), settle(0));
    }}),
"
    );
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
    out + "  };\n  return env;\n}\n"
}
