// `--emit`: bindings from the XML, written to a directory.

use std::collections::HashSet;
use std::path::Path;

use ts_type_transformer::generators::{self, c, cpp, fsharp, gleam, rust, wasm, zig};

pub fn bindings(target: &str, xml: &str, out: &Path, glue_for: Option<&Path>, project_name: Option<&str>, refs: &[String]) -> Result<(), String> {
    let pkg_name = match target {
        "fsharp" | "f#" => fsharp_namespace(xml, project_name),
        "c" | "cpp" | "c++" | "zig" | "rust" | "gleam" => snake_package(xml, project_name),
        other => return Err(format!("unknown target `{other}`: c, cpp, zig, rust, fsharp or gleam")),
    };
    let dest = if out.file_name().and_then(|s| s.to_str()) == Some(pkg_name.as_str()) {
        out.to_path_buf()
    } else {
        out.join(&pkg_name)
    };
    let files = generate(target, xml, glue_for, &pkg_name, &dest, refs).map_err(|problems| problems.join("\n"))?;
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

fn snake_package(xml: &str, explicit: Option<&str>) -> String {
    if let Some(name) = explicit {
        return generators::snake_package_name(name);
    }
    if let Ok(doc) = roxmltree::Document::parse(xml)
        && let Some(import) = doc.root_element().attribute("import")
    {
        return generators::snake_package_name(import);
    }
    "bindings".to_string()
}

type Files = Vec<(String, String)>;

// The files of a target's bindings, or everything it cannot express.
fn generate(target: &str, xml: &str, glue_for: Option<&Path>, pkg_name: &str, dest: &Path, refs: &[String]) -> Result<Files, Vec<String>> {
    let wasm_module = if pkg_name == "bindings" { "env" } else { pkg_name };
    let used = match glue_for {
        Some(p) => Some(wasm::imports_for(&std::fs::read(p).map_err(|e| vec![format!("{}: {e}", p.display())])?, wasm_module)),
        None => None,
    };
    let push_glue = |model: &generators::model::Model, out: &mut Files| {
        out.push(("glue.mjs".into(), wasm::glue_for_module(model, used.as_ref(), wasm_module)));
        for (why, n) in &model.skipped {
            eprintln!("not bound: {n} x {why}");
        }
    };
    match target {
        "c" => {
            let loaded: Vec<c::ProjectRef> = refs
                .iter()
                .map(|r| c::load_project_ref(Path::new(r), dest))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| vec![e])?;
            let extra_reserved: Vec<String> = loaded.iter().flat_map(|r| r.all_names.iter().cloned()).collect();
            let ref_structs: HashSet<String> = loaded
                .iter()
                .flat_map(|r| r.structs.iter().chain(r.external_structs.iter()).cloned())
                .collect();
            let model = wasm::model_with_reserved(xml, "C", &extra_reserved, &ref_structs)?;
            let mut out = c::files_named(&model, pkg_name, &loaded)?;
            push_glue(&model, &mut out);
            Ok(out)
        }
        "cpp" | "c++" => {
            let loaded: Vec<cpp::ProjectRef> = refs
                .iter()
                .map(|r| cpp::load_project_ref(Path::new(r), dest))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| vec![e])?;
            let model = wasm::model(xml, "C++")?;
            let mut out = cpp::files_named(&model, pkg_name, &loaded)?;
            push_glue(&model, &mut out);
            Ok(out)
        }
        "zig" => {
            let loaded: Vec<zig::ProjectRef> = refs
                .iter()
                .map(|r| zig::load_project_ref(Path::new(r), dest))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| vec![e])?;
            let model = wasm::model(xml, "Zig")?;
            let mut out = zig::files_named(&model, pkg_name, &loaded)?;
            push_glue(&model, &mut out);
            Ok(out)
        }
        "rust" => {
            let loaded: Vec<rust::ProjectRef> = refs
                .iter()
                .map(|r| rust::load_project_ref(Path::new(r), dest))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| vec![e])?;
            let model = wasm::model(xml, "Rust")?;
            let mut out = rust::files_named(&model, pkg_name, &loaded)?;
            push_glue(&model, &mut out);
            Ok(out)
        }
        "fsharp" | "f#" => {
            let loaded: Vec<fsharp::ProjectRef> = refs
                .iter()
                .map(|r| fsharp::load_project_ref(Path::new(r), dest))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| vec![e])?;
            let m = fsharp::module_with_refs(xml, pkg_name, &loaded)?;
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
            let loaded: Vec<gleam::ProjectRef> = refs
                .iter()
                .map(|r| gleam::load_project_ref(Path::new(r), dest))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| vec![e])?;
            let m = gleam::module_named(xml, pkg_name, &loaded)?;
            for (why, n) in &m.skipped {
                eprintln!("not bound: {n} x {why}");
            }
            let toml = m.gleam_toml();
            let mut files: Files = m.files.into_iter().map(|(p, c)| (format!("src/{p}"), c)).collect();
            files.push(("gleam.toml".into(), toml));
            Ok(files)
        }
        other => Err(vec![format!("unknown target `{other}`: c, cpp, zig, rust, fsharp or gleam")]),
    }
}
