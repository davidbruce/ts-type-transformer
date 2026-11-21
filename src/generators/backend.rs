use std::collections::HashSet;
use std::path::Path;

use super::{c, cpp, fsharp, gleam, model::Model, pkg_meta, rust, snake_package_name, wasm, zig};

pub struct EmitRequest<'a> {
    pub xml: &'a str,
    pub pkg_name: &'a str,
    pub dest: &'a Path,
    pub refs: &'a [String],
    pub glue_for: Option<&'a Path>,
}

pub struct EmitOutput {
    pub files: Vec<(String, String)>,
    pub skipped: Vec<(String, usize)>,
}

pub trait TargetBackend {
    fn id(&self) -> &'static str;
    fn package_name(&self, xml: &str, explicit: Option<&str>) -> String;
    fn emit(&self, req: &EmitRequest<'_>) -> Result<EmitOutput, Vec<String>>;
}

fn xml_import_attr(xml: &str) -> Option<String> {
    roxmltree::Document::parse(xml)
        .ok()
        .and_then(|doc| doc.root_element().attribute("import").map(str::to_string))
}

fn snake_package(xml: &str, explicit: Option<&str>) -> String {
    if let Some(name) = explicit {
        return snake_package_name(name);
    }
    xml_import_attr(xml)
        .map(|import| snake_package_name(&import))
        .unwrap_or_else(|| "bindings".to_string())
}

fn wasm_module_name(pkg_name: &str) -> &str {
    if pkg_name == "bindings" { "env" } else { pkg_name }
}

fn resolve_used_imports(req: &EmitRequest<'_>, wasm_module: &str) -> Result<Option<HashSet<String>>, Vec<String>> {
    match req.glue_for {
        Some(p) => {
            let bytes = std::fs::read(p).map_err(|e| vec![format!("{}: {e}", p.display())])?;
            Ok(Some(wasm::imports_for(&bytes, wasm_module)))
        }
        None => Ok(None),
    }
}

fn load_refs<R>(refs: &[String], dest: &Path, load: fn(&Path, &Path) -> Result<R, String>) -> Result<Vec<R>, Vec<String>> {
    refs.iter()
        .map(|r| load(Path::new(r), dest))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| vec![e])
}

fn push_wasm_glue(
    model: &Model,
    used: Option<&HashSet<String>>,
    wasm_module: &str,
    mut files: Vec<(String, String)>,
) -> EmitOutput {
    files.push(("glue.mjs".into(), wasm::glue_for_module(model, used, wasm_module)));
    let skipped = model.skipped.iter().map(|(why, n)| (why.clone(), *n)).collect();
    EmitOutput { files, skipped }
}

struct CBackend;
struct CppBackend;
struct ZigBackend;
struct RustBackend;
struct FSharpBackend;
struct GleamBackend;

static C_BACKEND: CBackend = CBackend;
static CPP_BACKEND: CppBackend = CppBackend;
static ZIG_BACKEND: ZigBackend = ZigBackend;
static RUST_BACKEND: RustBackend = RustBackend;
static FSHARP_BACKEND: FSharpBackend = FSharpBackend;
static GLEAM_BACKEND: GleamBackend = GleamBackend;

impl TargetBackend for CBackend {
    fn id(&self) -> &'static str {
        "c"
    }

    fn package_name(&self, xml: &str, explicit: Option<&str>) -> String {
        snake_package(xml, explicit)
    }

    fn emit(&self, req: &EmitRequest<'_>) -> Result<EmitOutput, Vec<String>> {
        let wasm_module = wasm_module_name(req.pkg_name);
        let used = resolve_used_imports(req, wasm_module)?;
        let loaded = load_refs(req.refs, req.dest, c::load_project_ref)?;
        let model = c::model_with_refs(req.xml, &loaded)?;
        let files = c::files_named(&model, req.pkg_name, &loaded)?;
        Ok(push_wasm_glue(&model, used.as_ref(), wasm_module, files))
    }
}

impl TargetBackend for CppBackend {
    fn id(&self) -> &'static str {
        "cpp"
    }

    fn package_name(&self, xml: &str, explicit: Option<&str>) -> String {
        snake_package(xml, explicit)
    }

    fn emit(&self, req: &EmitRequest<'_>) -> Result<EmitOutput, Vec<String>> {
        let wasm_module = wasm_module_name(req.pkg_name);
        let used = resolve_used_imports(req, wasm_module)?;
        let loaded = load_refs(req.refs, req.dest, cpp::load_project_ref)?;
        let model = wasm::model_with(req.xml, cpp::MODEL_OPTIONS, &cpp::NAMING, &[])?;
        let files = cpp::files_named(&model, req.pkg_name, &loaded)?;
        Ok(push_wasm_glue(&model, used.as_ref(), wasm_module, files))
    }
}

impl TargetBackend for ZigBackend {
    fn id(&self) -> &'static str {
        "zig"
    }

    fn package_name(&self, xml: &str, explicit: Option<&str>) -> String {
        snake_package(xml, explicit)
    }

    fn emit(&self, req: &EmitRequest<'_>) -> Result<EmitOutput, Vec<String>> {
        let wasm_module = wasm_module_name(req.pkg_name);
        let used = resolve_used_imports(req, wasm_module)?;
        let loaded = load_refs(req.refs, req.dest, zig::load_project_ref)?;
        let model = wasm::model_with(req.xml, zig::MODEL_OPTIONS, &zig::NAMING, &[])?;
        let (files, zig_imports) = zig::files_with_imports(&model, req.pkg_name, &loaded)?;
        Ok(push_wasm_glue(&model, used.as_ref().or(Some(&zig_imports)), wasm_module, files))
    }
}

impl TargetBackend for RustBackend {
    fn id(&self) -> &'static str {
        "rust"
    }

    fn package_name(&self, xml: &str, explicit: Option<&str>) -> String {
        snake_package(xml, explicit)
    }

    fn emit(&self, req: &EmitRequest<'_>) -> Result<EmitOutput, Vec<String>> {
        let wasm_module = wasm_module_name(req.pkg_name);
        let used = resolve_used_imports(req, wasm_module)?;
        let loaded = load_refs(req.refs, req.dest, rust::load_project_ref)?;
        let model = wasm::model_with(req.xml, rust::MODEL_OPTIONS, &rust::NAMING, &[])?;
        let files = rust::files_named(&model, req.pkg_name, &loaded)?;
        Ok(push_wasm_glue(&model, used.as_ref(), wasm_module, files))
    }
}

impl TargetBackend for FSharpBackend {
    fn id(&self) -> &'static str {
        "fsharp"
    }

    fn package_name(&self, xml: &str, explicit: Option<&str>) -> String {
        if let Some(name) = explicit {
            return fsharp::package_namespace(name);
        }
        xml_import_attr(xml)
            .map(|import| fsharp::package_namespace(&import))
            .unwrap_or_else(|| "Bindings".to_string())
    }

    fn emit(&self, req: &EmitRequest<'_>) -> Result<EmitOutput, Vec<String>> {
        let loaded = load_refs(req.refs, req.dest, fsharp::load_project_ref)?;
        let m = fsharp::module_with_refs(req.xml, req.pkg_name, &loaded)?;
        let skipped = m.skipped.iter().map(|(why, n)| (why.clone(), *n)).collect();
        let fsproj_name = m.fsproj_name();
        let fsproj = m.fsproj();
        let meta_toml = m.meta_toml;
        let mut files = m.files;
        files.push((fsproj_name, fsproj));
        files.push((pkg_meta::META_FILE.into(), meta_toml));
        Ok(EmitOutput { files, skipped })
    }
}

impl TargetBackend for GleamBackend {
    fn id(&self) -> &'static str {
        "gleam"
    }

    fn package_name(&self, xml: &str, explicit: Option<&str>) -> String {
        snake_package(xml, explicit)
    }

    fn emit(&self, req: &EmitRequest<'_>) -> Result<EmitOutput, Vec<String>> {
        let loaded = load_refs(req.refs, req.dest, gleam::load_project_ref)?;
        let m = gleam::module_named(req.xml, req.pkg_name, &loaded)?;
        let skipped = m.skipped.iter().map(|(why, n)| (why.clone(), *n)).collect();
        let toml = m.gleam_toml();
        let meta_toml = m.meta_toml;
        let mut files: Vec<(String, String)> = m.files.into_iter().map(|(p, c)| (format!("src/{p}"), c)).collect();
        files.push(("gleam.toml".into(), toml));
        files.push((pkg_meta::META_FILE.into(), meta_toml));
        Ok(EmitOutput { files, skipped })
    }
}

pub fn lookup(target: &str) -> Result<&'static dyn TargetBackend, String> {
    match target {
        "c" => Ok(&C_BACKEND),
        "cpp" | "c++" => Ok(&CPP_BACKEND),
        "zig" => Ok(&ZIG_BACKEND),
        "rust" => Ok(&RUST_BACKEND),
        "fsharp" | "f#" => Ok(&FSHARP_BACKEND),
        "gleam" => Ok(&GLEAM_BACKEND),
        other => Err(format!("unknown target `{other}`: c, cpp, zig, rust, fsharp or gleam")),
    }
}
