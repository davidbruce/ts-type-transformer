// `--emit`: bindings from the XML, written to a directory.

use std::path::Path;

use ts_type_transformer::generators::{c, cpp, fsharp, gleam, rust, wasm, zig};

pub fn bindings(target: &str, xml: &str, out: &Path, glue_for: Option<&Path>) -> Result<(), String> {
    let files = generate(target, xml, glue_for).map_err(|problems| problems.join("\n"))?;
    for (path, content) in &files {
        let path = out.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| format!("{}: {e}", path.display()))?;
        std::fs::write(&path, content).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    eprintln!("{} files in {}", files.len(), out.display());
    Ok(())
}

type Files = Vec<(String, String)>;

// The files of a target's bindings, or everything it cannot express.
fn generate(target: &str, xml: &str, glue_for: Option<&Path>) -> Result<Files, Vec<String>> {
    let used = match glue_for {
        Some(p) => Some(wasm::imports(&std::fs::read(p).map_err(|e| vec![format!("{}: {e}", p.display())])?)),
        None => None,
    };
    let wasm_files = |name: &str, files: fn(&wasm::Model) -> Result<Files, Vec<String>>| -> Result<Files, Vec<String>> {
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
            let m = fsharp::module(xml)?;
            for (why, n) in &m.skipped {
                eprintln!("not bound: {n} x {why}");
            }
            // In the order F# compiles them.
            let order: String = m.files.iter().map(|(name, _)| format!("<Compile Include=\"{name}\" />\n")).collect();
            let mut files = m.files;
            files.push(("compile-order.txt".into(), order));
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
