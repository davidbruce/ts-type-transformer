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
    let mut project_name: Option<String> = None;
    let mut glue_for: Option<String> = None;
    let mut libs_arg: Option<String> = None;
    // Lib namespaces to bind as well: `--include Intl`.
    let mut include: Vec<String> = Vec::new();
    // Packages, by the name they are imported by (`--package date-fns`),
    // resolved as TypeScript would from `--from` (default: the current directory).
    let mut packages: Vec<String> = Vec::new();
    let mut from: Option<String> = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--config" => config_paths.push(args.next().expect("config path")),
            "--set" => overrides.push(args.next().expect("key=value")),
            "--default-config" => return print!("{}", config::template()),
            "--census" => census = true,
            "--emit" => emit = Some(args.next().expect("c, cpp, zig or fsharp")),
            "--out" => out_dir = Some(args.next().expect("output directory")),
            "--name" => project_name = Some(args.next().expect("project / namespace name")),
            "--glue-for" => glue_for = Some(args.next().expect("compiled module")),
            "--lib" => libs_arg = Some(args.next().expect("comma-separated lib list")),
            "--include" => include.push(args.next().expect("a namespace of the lib files, such as Intl")),
            "--package" => packages.push(args.next().expect("a package name, such as date-fns")),
            "--from" => from = Some(args.next().expect("the directory packages are resolved from")),
            _ => input = a,
        }
    }
    let (opts, mut modules, renames) = config::load(&config_paths, &overrides).unwrap_or_else(|e| {
        eprintln!("config error: {e}");
        std::process::exit(2);
    });

    // Bare `lib.*.d.ts` names select a bundled lib file.
    let is_lib = packages.is_empty() && input.starts_with("lib.") && !Path::new(&input).exists();
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
    let (fs, cwd, tsconfig_path, name, entry_path): (Arc<dyn FS>, String, String, String, Option<std::path::PathBuf>) = if !packages.is_empty() {
        // An entry that imports each package: TypeScript resolves them, by
        // `exports`, `types`, `typesVersions` or `@types`, as for a user of them.
        let base = std::fs::canonicalize(from.as_deref().unwrap_or(".")).expect("the directory to resolve packages from");
        let dir = tspath::normalize_path(&base.to_string_lossy());
        let at = |file: &str| if dir.ends_with('/') { format!("{dir}{file}") } else { format!("{dir}/{file}") };
        let (entry, tsconfig_path) = (at("__tsrs_entry__.ts"), at("__tsrs_tsconfig.json"));
        let source: String = packages.iter().enumerate().map(|(i, p)| format!("import * as __package{i} from {};\n", json_string(p))).collect();
        let libs = explicit_libs.unwrap_or_else(|| default_libs(false));
        // Every @types package installed, as TypeScript loads them by default:
        // `@types/node` declares `fs`, which is then a module to bind.
        let mut types: Vec<String> = std::fs::read_dir(base.join("node_modules/@types"))
            .map(|dir| dir.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().to_string()).collect())
            .unwrap_or_default();
        types.sort();
        let tsconfig = format!(
            r#"{{ "compilerOptions": {{ "lib": [{}], "module": "esnext", "moduleResolution": "bundler", "allowImportingTsExtensions": true, "types": [{}], "noEmit": true }}, "files": [{}] }}"#,
            libs_json(&libs),
            libs_json(&types),
            json_string(&entry)
        );
        let virtual_files: Vec<(String, String)> = vec![(tsconfig_path.clone(), tsconfig), (entry.clone(), source)];
        let (exists, read) = (virtual_files.clone(), virtual_files);
        let fs: Arc<dyn FS> = Arc::new(bundled::wrap_fs(wrapvfs::wrap(
            osvfs::fs(),
            wrapvfs::Replacements {
                file_exists: Some(Box::new(move |p| exists.iter().any(|f| f.0 == p) || osvfs::fs().file_exists(p))),
                read_file: Some(Box::new(move |p| read.iter().find(|f| f.0 == p).map(|f| f.1.clone()).or_else(|| osvfs::fs().read_file(p)))),
                ..Default::default()
            },
        )));
        (fs, dir, tsconfig_path, entry, None)
    } else if is_lib {
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
        let Ok(abs) = std::fs::canonicalize(&input) else {
            eprintln!("error: no such file: {input} (a package is given with --package)");
            std::process::exit(2);
        };
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
    // Each package's entry file, as the virtual entry's imports resolved.
    let mut entries = Vec::new();
    for (statement, package) in file.statements().nodes().iter().zip(&packages) {
        let resolved = checker.resolve_external_module_name_exported(statement.as_import_declaration().module_specifier(), None);
        let found = resolved.and_then(|m| m.declarations().first().and_then(|d| tsrs_ast::get_source_file_of_node(*d)).map(|f| (m, f)));
        let Some((module, source)) = found else {
            eprintln!("error: cannot find the types of {package} from {cwd}: is it installed?");
            std::process::exit(2);
        };
        // A file, or an ambient module declared in one (`declare module "fs"`).
        let ambient = module.declarations().first().is_some_and(|d| d.kind() == tsrs_ast::Kind::ModuleDeclaration);
        entries.push((package.clone(), source, ambient.then_some(module)));
    }

    let mut front = front::Front::new(&mut checker, name, bundled::lib_path());
    // Types the checker makes while collecting merge as lowering's would.
    front.merge_intersections = opts.intersections;
    for (package, source, ambient) in &entries {
        front.add_package(package, source.file_name());
        if ambient.is_some() {
            front.ambient_packages.insert(package.clone());
        }
    }
    // A lib file's namespaces gather their declarations from the other lib files.
    if is_lib {
        front.included.extend(file.statements().nodes().iter().filter(|s| s.kind() == tsrs_ast::Kind::ModuleDeclaration).filter_map(|s| s.name()).map(|n| n.text().to_string()));
    }
    let mut decls = if entries.is_empty() {
        front.collect(file)
    } else {
        // One front end for all: references between packages are to one declaration.
        let mut all = Vec::new();
        for (_, source, ambient) in &entries {
            front.root_file = source.file_name().to_string();
            all.extend(match ambient {
                Some(module) => front.collect_module(*module),
                None => front.collect(*source),
            });
        }
        all
    };
    if !include.is_empty() {
        let libs: Vec<_> = program.get_source_files().iter().copied().filter(|f| f.file_name().starts_with(&bundled::lib_path())).collect();
        front.collect_included(&libs, &include, &mut decls);
    }
    lower::lower(&mut front, &mut decls, opts);
    // After lowering: an overload it renamed is of its package too.
    modules.origins = front.origin_of_name.clone();
    match entries.as_slice() {
        [] => {}
        [(package, _, _)] => modules.import = modules.import.take().or_else(|| Some(package.clone())),
        _ => modules.packages = front.package_of_name.clone(),
    }
    match lower::rename(&mut decls, &renames) {
        Ok(unused) if census => unused.iter().for_each(|n| eprintln!("# rename.{n}: not generated for this target")),
        Ok(_) => {}
        Err(e) => {
            eprintln!("config error: {e}");
            std::process::exit(2);
        }
    }
    if opts.pascal_types {
        lower::pascal_types(&mut decls);
    }

    let mut x = xml::Xml::default();
    x.top(&decls, &modules);
    // The strategies in effect, so a consumer can check what it is reading.
    let strategies: Vec<String> = config::describe(opts).iter().map(|(k, v)| format!("{k}=\"{v}\"")).collect();
    let import = modules.import.as_ref().map(|m| format!(" import=\"{}\"", xml::escape(m))).unwrap_or_default();
    // The packages asked for: a program gives each one's module; others
    // declarations come from only where they are used.
    let requested = if entries.len() > 1 { format!(" packages=\"{}\"", xml::escape(&entries.iter().map(|e| e.0.as_str()).collect::<Vec<_>>().join(" "))) } else { String::new() };
    let mut doc = format!("<declarations{import}{requested}>\n  <lowering {}/>\n", strategies.join(" "));
    for line in x.out.lines() {
        doc += &format!("  {line}\n");
    }
    doc += "</declarations>\n";
    match emit {
        None => print!("{doc}"),
        Some(target) => {
            let out = std::path::PathBuf::from(out_dir.unwrap_or_else(|| "bindings".into()));
            let derived_name = project_name.or_else(|| modules.import.clone()).or_else(|| {
                is_lib.then(|| input.strip_prefix("lib.").and_then(|s| s.strip_suffix(".d.ts")).unwrap_or(&input).to_string())
            });
            if let Err(e) = emit::bindings(&target, &doc, &out, glue_for.as_deref().map(std::path::Path::new), derived_name.as_deref()) {
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
