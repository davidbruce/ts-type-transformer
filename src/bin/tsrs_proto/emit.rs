// `--emit`: bindings from the XML, written to a directory.

use std::path::Path;

use ts_type_transformer::generators::{c, cpp, fsharp, wasm, zig};

// What each target needs lowered, for an error that says what to change.
const HINT: &str = "the generator cannot express a construct the config left in; lower it in the target's config (see `--default-config`)";

pub fn bindings(target: &str, xml: &str, out: &Path, glue_for: Option<&Path>) -> Result<(), String> {
    let files = generate(target, xml, glue_for)?;
    for (path, content) in &files {
        let path = out.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| format!("{}: {e}", path.display()))?;
        std::fs::write(&path, content).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    eprintln!("{} files in {}", files.len(), out.display());
    Ok(())
}

fn generate(target: &str, xml: &str, glue_for: Option<&Path>) -> Result<Vec<(String, String)>, String> {
    let used = match glue_for {
        Some(p) => Some(wasm::imports(&std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()))?)),
        None => None,
    };
    // A generator rejects what it cannot express by panicking: that is an error here.
    std::panic::set_hook(Box::new(|_| {}));
    let result = std::panic::catch_unwind(|| -> Result<Vec<(String, String)>, String> {
        let wasm_files = |name: &str, files: fn(&wasm::Model) -> Vec<(String, String)>| {
            let model = wasm::model(xml, name);
            let mut out = files(&model);
            // The JavaScript side: every import, or only those a compiled module uses.
            out.push(("glue.mjs".into(), wasm::glue(&model, used.as_ref())));
            for (why, n) in &model.skipped {
                eprintln!("not bound: {n} x {why}");
            }
            out
        };
        Ok(match target {
            "c" => wasm_files("C", c::files),
            "cpp" | "c++" => wasm_files("C++", cpp::files),
            "zig" => wasm_files("Zig", zig::files),
            "fsharp" | "f#" => {
                let m = fsharp::module(xml);
                for (why, n) in &m.skipped {
                    eprintln!("not bound: {n} x {why}");
                }
                // In the order F# compiles them.
                let order: String = m.files.iter().map(|(name, _)| format!("<Compile Include=\"{name}\" />\n")).collect();
                let mut files = m.files;
                files.push(("compile-order.txt".into(), order));
                files
            }
            other => return Err(format!("unknown target `{other}`: c, cpp, zig or fsharp")),
        })
    });
    let _ = std::panic::take_hook();
    match result {
        Ok(r) => r,
        Err(payload) => {
            let msg = payload.downcast_ref::<String>().cloned().or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default();
            Err(format!("{msg}\n{HINT}"))
        }
    }
}
