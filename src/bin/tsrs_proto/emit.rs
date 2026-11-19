// `--emit`: bindings from the XML, written to a directory.

use std::path::Path;

use ts_type_transformer::generators::{c, cpp, fsharp, gleam, model::Model, rust, wasm, zig};

pub fn bindings(target: &str, xml: &str, out: &Path, glue_for: Option<&Path>, project_name: Option<&str>, refs: &[String]) -> Result<(), String> {
    let fsharp_ns = matches!(target, "fsharp" | "f#").then(|| fsharp_namespace(xml, project_name));
    let dest = match &fsharp_ns {
        Some(ns) if out.file_name().and_then(|s| s.to_str()) != Some(ns.as_str()) => out.join(ns),
        _ => out.to_path_buf(),
    };
    let loaded_refs: Vec<fsharp::ProjectRef> = if fsharp_ns.is_some() {
        refs.iter().map(|r| fsharp::load_project_ref(Path::new(r), &dest)).collect::<Result<Vec<_>, _>>()?
    } else {
        Vec::new()
    };
    let files = generate(target, xml, glue_for, fsharp_ns.as_deref(), &loaded_refs).map_err(|problems| problems.join("\n"))?;
    for (path, content) in &files {
        let path = dest.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| format!("{}: {e}", path.display()))?;
        std::fs::write(&path, content).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    eprintln!("{} files in {}", files.len(), dest.display());
    Ok(())
}

fn fsharp_namespace(xml: &str, explicit: Option<&str>) -> String {
    if let Some(name) = explicit {
        return fsharp::package_namespace(name);
    }
    if let Ok(doc) = roxmltree::Document::parse(xml)
        && let Some(import) = doc.root_element().attribute("import")
    {
        return fsharp::package_namespace(import);
    }
    "Bindings".to_string()
}

type Files = Vec<(String, String)>;

// The files of a target's bindings, or everything it cannot express.
fn generate(target: &str, xml: &str, glue_for: Option<&Path>, fsharp_ns: Option<&str>, fsharp_refs: &[fsharp::ProjectRef]) -> Result<Files, Vec<String>> {
    let used = match glue_for {
        Some(p) => Some(wasm::imports(&std::fs::read(p).map_err(|e| vec![format!("{}: {e}", p.display())])?)),
        None => None,
    };
    let wasm_files = |name: &str, files: fn(&Model) -> Result<Files, Vec<String>>| -> Result<Files, Vec<String>> {
        let model = wasm::model(xml, name)?;
        let mut out = files(&model)?;
        // The JavaScript side: every import, or only those a compiled module uses.
        out.push(("glue.mjs".into(), wasm::glue(&model, used.as_ref())));
        for (why, n) in &model.skipped {
            eprintln!("not bound: {n} x {why}");
        }
        Ok(out)
    };
    match target {
        "c" => wasm_files("C", c::files),
        "cpp" | "c++" => wasm_files("C++", cpp::files),
        "zig" => wasm_files("Zig", zig::files),
        "rust" => wasm_files("Rust", rust::files),
        "fsharp" | "f#" => {
            let ns = fsharp_ns.unwrap_or("Bindings");
            let m = fsharp::module_with_refs(xml, ns, fsharp_refs)?;
            for (why, n) in &m.skipped {
                eprintln!("not bound: {n} x {why}");
            }
            let fsproj_name = m.fsproj_name();
            let fsproj = m.fsproj();
            let mut files = m.files;
            files.push((fsproj_name, fsproj));
            Ok(files)
        }
        "gleam" => {
            let m = gleam::module(xml)?;
            for (why, n) in &m.skipped {
                eprintln!("not bound: {n} x {why}");
            }
            Ok(m.files)
        }
        other => Err(vec![format!("unknown target `{other}`: c, cpp, zig, rust, fsharp or gleam")]),
    }
}
