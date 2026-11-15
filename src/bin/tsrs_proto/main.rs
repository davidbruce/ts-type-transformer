// tsrs feasibility prototype: .d.ts -> IR -> lowering passes -> XML, and
// from the XML, bindings.
// Usage: tsrs_proto <file.d.ts | lib.dom.d.ts> [--config target.toml ...]
//                   [--set key=value] [--census]
//                   [--emit c|cpp|zig|fsharp --out <dir> [--glue-for <module.wasm>]]
//        tsrs_proto --default-config

mod config;
mod emit;
mod front;
mod ir;
mod lower;
mod xml;

use std::path::Path;
use std::sync::Arc;

use tsrs_compiler::{Context, ProgramOptions, new_compiler_host, new_program};
use tsrs_core::{P, tspath};
use tsrs_tsoptions::{self as tsoptions, ParseConfigHost};
use tsrs_vfs::{FS, bundled, osvfs, vfstest, wrapvfs};

struct ConfigHost {
    fs: Arc<dyn FS>,
    cwd: String,
}

impl ParseConfigHost for ConfigHost {
    fn fs(&self) -> &dyn FS {
        &*self.fs
    }
    fn get_current_directory(&self) -> &str {
        &self.cwd
    }
}

fn json_string(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

// A JSON string at `p`, unescaped, and where it ends.
fn json_str(bytes: &[u8], mut p: usize) -> Option<(String, usize)> {
    if bytes.get(p) != Some(&b'"') {
        return None;
    }
    p += 1;
    let mut out: Vec<u8> = Vec::new();
    loop {
        match *bytes.get(p)? {
            b'"' => return Some((String::from_utf8(out).ok()?, p + 1)),
            b'\\' => {
                let c = *bytes.get(p + 1)?;
                p += 2;
                match c {
                    b'n' => out.push(b'\n'),
                    b'r' => out.push(b'\r'),
                    b't' => out.push(b'\t'),
                    b'b' => out.push(8),
                    b'f' => out.push(12),
                    b'u' => {
                        let hex = std::str::from_utf8(bytes.get(p..p + 4)?).ok()?;
                        let c = char::from_u32(u32::from_str_radix(hex, 16).ok()?).unwrap_or('\u{fffd}');
                        out.extend(c.to_string().as_bytes());
                        p += 4;
                    }
                    other => out.push(other),
                }
            }
            b => {
                out.push(b);
                p += 1;
            }
        }
    }
}

// The top-level `"name"` of a package.json. No JSON crate: this is all it reads.
fn package_json_name(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let bytes = text.as_bytes();
    let (mut i, mut depth) = (0, 0usize);
    while i < bytes.len() {
        match bytes[i] {
            b'{' | b'[' => {
                depth += 1;
                i += 1;
            }
            b'}' | b']' => {
                depth = depth.saturating_sub(1);
                i += 1;
            }
            b'"' => {
                let (key, end) = json_str(bytes, i)?;
                i = end;
                while bytes.get(i).is_some_and(u8::is_ascii_whitespace) {
                    i += 1;
                }
                if depth == 1 && bytes.get(i) == Some(&b':') {
                    i += 1;
                    while bytes.get(i).is_some_and(u8::is_ascii_whitespace) {
                        i += 1;
                    }
                    if key == "name" {
                        return json_str(bytes, i).map(|(v, _)| v).filter(|v| !v.is_empty());
                    }
                }
            }
            _ => i += 1,
        }
    }
    None
}

// The package an entry file belongs to: the nearest package.json above it
// that has a name. Not a guess from file names: a wrong one would be imported.
// A types package describes another: `@types/three` is `three`, and
// `@types/babel__core` is `@babel/core`.
fn infer_module_name(entry: &Path) -> Option<String> {
    let name = entry.ancestors().skip(1).find_map(|dir| package_json_name(&dir.join("package.json")))?;
    Some(match name.strip_prefix("@types/") {
        Some(described) => match described.split_once("__") {
            Some((scope, name)) => format!("@{scope}/{name}"),
            None => described.to_string(),
        },
        None => name,
    })
}

fn main() {
    let mut input = "test.d.ts".to_string();
    let mut config_paths: Vec<String> = Vec::new();
    let mut overrides: Vec<String> = Vec::new();
    let mut census = false;
    let mut emit: Option<String> = None;
    let mut out_dir: Option<String> = None;
    let mut glue_for: Option<String> = None;
    let mut libs_arg: Option<String> = None;
    // Lib namespaces to bind as well: `--include Intl`.
    let mut include: Vec<String> = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--config" => config_paths.push(args.next().expect("config path")),
            "--set" => overrides.push(args.next().expect("key=value")),
            "--default-config" => return print!("{}", config::template()),
            "--census" => census = true,
            "--emit" => emit = Some(args.next().expect("c, cpp, zig or fsharp")),
            "--out" => out_dir = Some(args.next().expect("output directory")),
            "--glue-for" => glue_for = Some(args.next().expect("compiled module")),
            "--lib" => libs_arg = Some(args.next().expect("comma-separated lib list")),
            "--include" => include.push(args.next().expect("a namespace of the lib files, such as Intl")),
            _ => input = a,
        }
    }
    let (opts, mut modules, renames) = config::load(&config_paths, &overrides).unwrap_or_else(|e| {
        eprintln!("config error: {e}");
        std::process::exit(2);
    });

    // Bare `lib.*.d.ts` names select a bundled lib file.
    let is_lib = input.starts_with("lib.") && !Path::new(&input).exists();
    let default_libs = |for_webworker: bool| -> Vec<String> {
        if for_webworker {
            ["es2020", "webworker", "webworker.iterable", "webworker.asynciterable", "webworker.importscripts"]
                .into_iter()
                .map(String::from)
                .collect()
        } else {
            ["es2020", "dom", "dom.iterable", "dom.asynciterable"].into_iter().map(String::from).collect()
        }
    };
    let libs_json = |libs: &[String]| -> String {
        libs.iter().map(|l| json_string(l)).collect::<Vec<_>>().join(", ")
    };
    let explicit_libs = libs_arg.map(|s| s.split(',').map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).collect::<Vec<_>>());
    let (fs, cwd, tsconfig_path, name, entry_path): (Arc<dyn FS>, String, String, String, Option<std::path::PathBuf>) = if is_lib {
        let libs = explicit_libs.unwrap_or_else(|| {
            let mut list = default_libs(input.starts_with("lib.webworker"));
            if let Some(lib_name) = input.strip_prefix("lib.").and_then(|s| s.strip_suffix(".d.ts"))
                && !list.iter().any(|l| l == lib_name)
            {
                list.push(lib_name.to_string());
            }
            list
        });
        let tsconfig = format!(r#"{{ "compilerOptions": {{ "lib": [{}], "noEmit": true }}, "files": ["/input.d.ts"] }}"#, libs_json(&libs));
        let fs: Arc<dyn FS> = Arc::new(bundled::wrap_fs(vfstest::from_map(
            [("/tsconfig.json", tsconfig), ("/input.d.ts", String::new())],
            true,
        )));
        (fs, "/".into(), "/tsconfig.json".into(), format!("{}/{}", bundled::lib_path(), input), None)
    } else {
        let abs = std::fs::canonicalize(&input).unwrap_or_else(|_| std::path::absolute(&input).expect("resolve input"));
        let abs_str = tspath::normalize_path(&abs.to_string_lossy());
        let dir = tspath::get_directory_path(&abs_str);
        let tsconfig_path = if dir.ends_with('/') { format!("{dir}__tsrs_tsconfig.json") } else { format!("{dir}/__tsrs_tsconfig.json") };
        let libs = explicit_libs.unwrap_or_else(|| {
            let head = std::fs::read_to_string(&abs).unwrap_or_default();
            let is_worker = head.contains("lib=\"webworker") || head.contains("lib='webworker");
            default_libs(is_worker)
        });
        let tsconfig = format!(
            r#"{{ "compilerOptions": {{ "lib": [{}], "noEmit": true }}, "files": [{}] }}"#,
            libs_json(&libs),
            json_string(&abs_str)
        );
        let tc_exists = tsconfig_path.clone();
        let tc_read = tsconfig_path.clone();
        let fs: Arc<dyn FS> = Arc::new(bundled::wrap_fs(wrapvfs::wrap(
            osvfs::fs(),
            wrapvfs::Replacements {
                file_exists: Some(Box::new(move |p| p == tc_exists || osvfs::fs().file_exists(p))),
                read_file: Some(Box::new(move |p| if p == tc_read { Some(tsconfig.clone()) } else { osvfs::fs().read_file(p) })),
                ..Default::default()
            },
        )));
        (fs, dir, tsconfig_path, abs_str, Some(abs))
    };
    let host: &'static ConfigHost = Box::leak(Box::new(ConfigHost { fs: Arc::clone(&fs), cwd: cwd.clone() }));
    let (config, errs) =
        tsoptions::get_parsed_command_line_of_config_file(&tsconfig_path, None, None, host, None);
    assert!(errs.is_empty(), "tsconfig errors: {}", errs.len());

    let compiler_host = new_compiler_host(&cwd, fs, &bundled::lib_path(), None, None, None);
    let program = new_program(ProgramOptions::new(P::new(config.unwrap()), compiler_host));
    let file = program.get_source_file(&name).unwrap_or_else(|| panic!("{name} not in program"));
    if modules.import.is_none()
        && let Some(path) = &entry_path
        && file.external_module_indicator().is_some()
    {
        modules.import = infer_module_name(path);
        if modules.import.is_none() {
            eprintln!("warning: {} is a module, but no package.json above it names it: bindings will treat it as globals; set [library] module = \"...\"", path.display());
        }
    }
    let mut checker = program.get_type_checker_for_file(&Context::default(), file);

    let mut front = front::Front::new(&mut checker, name, bundled::lib_path());
    // A lib file's namespaces gather their declarations from the other lib files.
    if is_lib {
        front.included.extend(file.statements().nodes().iter().filter(|s| s.kind() == tsrs_ast::Kind::ModuleDeclaration).filter_map(|s| s.name()).map(|n| n.text().to_string()));
    }
    let mut decls = front.collect(file);
    if !include.is_empty() {
        let libs: Vec<_> = program.get_source_files().iter().copied().filter(|f| f.file_name().starts_with(&bundled::lib_path())).collect();
        front.collect_included(&libs, &include, &mut decls);
    }
    lower::lower(&mut front, &mut decls, opts);
    match lower::rename(&mut decls, &renames) {
        Ok(unused) if census => unused.iter().for_each(|n| eprintln!("# rename.{n}: not generated for this target")),
        Ok(_) => {}
        Err(e) => {
            eprintln!("config error: {e}");
            std::process::exit(2);
        }
    }

    let mut x = xml::Xml::default();
    x.top(&decls, &modules);
    // The strategies in effect, so a consumer can check what it is reading.
    let strategies: Vec<String> = config::describe(opts).iter().map(|(k, v)| format!("{k}=\"{v}\"")).collect();
    let import = modules.import.as_ref().map(|m| format!(" import=\"{}\"", xml::escape(m))).unwrap_or_default();
    let mut doc = format!("<declarations{import}>\n  <lowering {}/>\n", strategies.join(" "));
    for line in x.out.lines() {
        doc += &format!("  {line}\n");
    }
    doc += "</declarations>\n";
    match emit {
        None => print!("{doc}"),
        Some(target) => {
            let out = std::path::PathBuf::from(out_dir.unwrap_or_else(|| "bindings".into()));
            if let Err(e) = emit::bindings(&target, &doc, &out, glue_for.as_deref().map(std::path::Path::new)) {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        }
    }
    if census {
        for (what, n) in &x.census {
            eprintln!("{what}\t{n}");
        }
        for (what, n) in &front.stats {
            eprintln!("# {what}\t{n}");
        }
    }
}
