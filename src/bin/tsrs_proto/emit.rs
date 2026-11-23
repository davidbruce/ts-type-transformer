// `--emit`: bindings from the XML, written to a directory.

use std::path::Path;

use ts_type_transformer::generators::backend::{self, EmitRequest};

pub fn bindings(
    target: &str,
    xml: &str,
    out: &Path,
    glue_for: Option<&Path>,
    glue_out: Option<&Path>,
    project_name: Option<&str>,
    refs: &[String],
) -> Result<(), String> {
    let backend = backend::lookup(target)?;
    let pkg_name = backend.package_name(xml, project_name);
    let dest = if out.file_name().and_then(|s| s.to_str()) == Some(pkg_name.as_str()) {
        out.to_path_buf()
    } else {
        out.join(&pkg_name)
    };
    let req = EmitRequest { xml, pkg_name: &pkg_name, dest: &dest, refs, glue_for };
    let out = backend.emit(&req).map_err(|problems| problems.join("\n"))?;
    // Only the glue a compiled module imports, for that module alone.
    if let Some(file) = glue_out {
        let glue = out.files.iter().find(|(p, _)| p == "glue.mjs").ok_or("no glue for this target")?;
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        return std::fs::write(file, &glue.1).map_err(|e| format!("{}: {e}", file.display()));
    }
    for (why, n) in &out.skipped {
        eprintln!("not bound: {n} x {why}");
    }
    for (path, content) in &out.files {
        let path = dest.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| format!("{}: {e}", path.display()))?;
        std::fs::write(&path, content).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    eprintln!("{} files in {}", out.files.len(), dest.display());
    Ok(())
}
