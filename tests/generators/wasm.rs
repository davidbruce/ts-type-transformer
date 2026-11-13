// What a wasm target needs from the XML, and the JavaScript glue both wasm
// generators share. Objects cross the boundary as integer handles.

use roxmltree::{Document, Node};

use super::{attr, elements, flag, named, only_child, unsupported};

pub enum Ty {
    Void,
    Num,
    Bool,
    Str,
    Any,
    Obj(String),
    Enum(String),
    // The flag says the absent value is `undefined`, not `null`.
    Nullable(Box<Ty>, bool),
    Union(Vec<Ty>),
}

pub enum Call {
    Method(String),
    Get(String),
    Set(String),
    New,
    Static(String),
    Global(String),
}

pub struct Func {
    // Name of the wasm import, unique in the module.
    pub import: String,
    // Name in the target language.
    pub name: String,
    pub call: Call,
    pub params: Vec<(String, Ty)>,
    pub ret: Ty,
}

pub struct Class {
    pub name: String,
    pub ancestors: Vec<String>,
    pub funcs: Vec<Func>,
    pub consts: Vec<(String, String)>,
}

pub struct Model {
    pub enums: Vec<(String, Vec<String>)>,
    pub classes: Vec<Class>,
    pub funcs: Vec<Func>,
}

pub fn model(xml: &str, target: &str) -> Model {
    let doc = Document::parse(xml).expect("well-formed XML");
    let mut m = Model { enums: Vec::new(), classes: Vec::new(), funcs: Vec::new() };
    for d in elements(doc.root_element()) {
        match d.tag_name().name() {
            "lowering" => {}
            "enum" => m.enums.push((attr(d, "name").into(), named(d, "member").map(|v| attr(v, "value").to_string()).collect())),
            "interface" | "class" => m.classes.push(class(d, target)),
            "function" => {
                let name = attr(d, "name");
                let (params, ret) = signature(d, target);
                m.funcs.push(Func { import: name.into(), name: name.into(), call: Call::Global(js_name(d).into()), params, ret });
            }
            _ => unsupported(target, d),
        }
    }
    m
}

fn js_name<'a>(n: Node<'a, 'a>) -> &'a str {
    n.attribute("jsName").unwrap_or_else(|| attr(n, "name"))
}

fn class(d: Node, target: &str) -> Class {
    let owner = attr(d, "name");
    let mut c = Class { name: owner.into(), ancestors: Vec::new(), funcs: Vec::new(), consts: Vec::new() };
    for m in elements(d) {
        let name = m.attribute("name").unwrap_or_default();
        let is_static = flag(m, "static");
        match m.tag_name().name() {
            "ancestor" => c.ancestors.push(name.into()),
            "constructor" => {
                let (params, _) = signature(m, target);
                c.funcs.push(Func { import: format!("{owner}_{name}"), name: name.into(), call: Call::New, params, ret: Ty::Obj(owner.into()) });
            }
            "method" => {
                let (params, ret) = signature(m, target);
                let call = if is_static { Call::Static(js_name(m).into()) } else { Call::Method(js_name(m).into()) };
                c.funcs.push(Func { import: format!("{owner}_{name}"), name: name.into(), call, params, ret });
            }
            // A constant needs no call across the boundary.
            "property" if m.has_attribute("const") => c.consts.push((name.into(), attr(m, "const").into())),
            "property" if is_static => unsupported(target, m),
            "property" => {
                let t = || ty(only_child(m), target);
                c.funcs.push(Func { import: format!("{owner}_get_{name}"), name: format!("get_{name}"), call: Call::Get(name.into()), params: vec![], ret: t() });
                if !flag(m, "readonly") {
                    c.funcs.push(Func {
                        import: format!("{owner}_set_{name}"),
                        name: format!("set_{name}"),
                        call: Call::Set(name.into()),
                        params: vec![("value".into(), t())],
                        ret: Ty::Void,
                    });
                }
            }
            _ => unsupported(target, m),
        }
    }
    c
}

fn signature(n: Node, target: &str) -> (Vec<(String, Ty)>, Ty) {
    let mut params = Vec::new();
    let mut ret = Ty::Void;
    for p in elements(n) {
        match p.tag_name().name() {
            "parameter" if flag(p, "optional") || flag(p, "rest") => unsupported(target, p),
            "parameter" => params.push((attr(p, "name").to_string(), ty(only_child(p), target))),
            "returns" => ret = ty(only_child(p), target),
            _ => unsupported(target, p),
        }
    }
    (params, ret)
}

fn ty(n: Node, target: &str) -> Ty {
    match n.tag_name().name() {
        "primitive" => match attr(n, "name") {
            "number" => Ty::Num,
            "boolean" => Ty::Bool,
            "string" => Ty::Str,
            "void" | "undefined" => Ty::Void,
            "any" | "unknown" => Ty::Any,
            _ => unsupported(target, n),
        },
        "ref" if elements(n).next().is_some() => unsupported(target, n),
        "ref" => match attr(n, "kind") {
            "class" | "interface" => Ty::Obj(attr(n, "name").into()),
            "enum" => Ty::Enum(attr(n, "name").into()),
            _ => unsupported(target, n),
        },
        "nullable" => Ty::Nullable(Box::new(ty(only_child(n), target)), attr(n, "absent") == "undefined"),
        "union" => Ty::Union(elements(n).map(|p| ty(p, target)).collect()),
        _ => unsupported(target, n),
    }
}

impl Model {
    pub fn all_funcs(&self) -> impl Iterator<Item = (Option<&Class>, &Func)> {
        self.classes.iter().flat_map(|c| c.funcs.iter().map(move |f| (Some(c), f))).chain(self.funcs.iter().map(|f| (None, f)))
    }
}

// JavaScript that implements every import by calling the real API.
pub fn glue(m: &Model) -> String {
    let mut out = String::from(
        "// Generated. Implements the wasm imports by calling the JavaScript API.
export function makeEnv(getMemory) {
  const objs = [null];
  // One handle per object, so handles can be compared for identity.
  const ids = new Map();
  const put = v => {
    if (v === null || v === undefined) return 0;
    if (!ids.has(v)) ids.set(v, objs.push(v) - 1);
    return ids.get(v);
  };
  const get = h => objs[h];
  const enc = new TextEncoder(), dec = new TextDecoder();
  const str = (ptr, len) => dec.decode(new Uint8Array(getMemory().buffer, ptr, len));
  const enums = {
",
    );
    for (name, values) in &m.enums {
        out += &format!("    {name}: {values:?},\n");
    }
    out += "  };
  return {
    js_string_len: h => enc.encode(get(h)).length,
    js_string_read: (h, ptr) => { new Uint8Array(getMemory().buffer).set(enc.encode(get(h)), ptr); },
    js_from_number: x => put(x),
    js_from_string: (ptr, len) => put(str(ptr, len)),
";
    for (class, f) in m.all_funcs() {
        let mut wasm_params = Vec::new();
        let mut args = Vec::new();
        if matches!(f.call, Call::Method(_) | Call::Get(_) | Call::Set(_)) {
            wasm_params.push("self".to_string());
        }
        for (i, (_, t)) in f.params.iter().enumerate() {
            let p = format!("a{i}");
            // A string is a pointer and a length; everything else is one value.
            if matches!(t, Ty::Str) {
                wasm_params.push(format!("{p}_ptr"));
                wasm_params.push(format!("{p}_len"));
                args.push(format!("str({p}_ptr, {p}_len)"));
                continue;
            }
            args.push(match t {
                Ty::Num => p.clone(),
                Ty::Bool => format!("!!{p}"),
                Ty::Enum(e) => format!("enums.{e}[{p}]"),
                Ty::Nullable(_, true) => format!("({p} === 0 ? undefined : get({p}))"),
                Ty::Any | Ty::Obj(_) | Ty::Nullable(..) | Ty::Union(_) => format!("get({p})"),
                Ty::Str | Ty::Void => unreachable!(),
            });
            wasm_params.push(p);
        }
        let args = args.join(", ");
        let owner = class.map(|c| c.name.as_str()).unwrap_or_default();
        let call = match &f.call {
            Call::Method(js) => format!("get(self).{js}({args})"),
            Call::Get(prop) => format!("get(self).{prop}"),
            Call::Set(prop) => format!("get(self).{prop} = {args}"),
            Call::New => format!("new globalThis.{owner}({args})"),
            Call::Static(js) => format!("globalThis.{owner}.{js}({args})"),
            Call::Global(js) => format!("globalThis.{js}({args})"),
        };
        let body = match &f.ret {
            Ty::Void => format!("{{ {call}; }}"),
            Ty::Num => call,
            Ty::Bool => format!("({call}) ? 1 : 0"),
            Ty::Enum(e) => format!("enums.{e}.indexOf({call})"),
            Ty::Str | Ty::Any | Ty::Obj(_) | Ty::Nullable(..) | Ty::Union(_) => format!("put({call})"),
        };
        out += &format!("    {}: ({}) => {body},\n", f.import, wasm_params.join(", "));
    }
    out + "  };\n}\n"
}
