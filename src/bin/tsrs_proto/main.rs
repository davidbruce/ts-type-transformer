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

use std::sync::Arc;

use tsrs_compiler::{Context, ProgramOptions, new_compiler_host, new_program};
use tsrs_core::P;
use tsrs_tsoptions::{self as tsoptions, ParseConfigHost};
use tsrs_vfs::{FS, bundled, vfstest};

struct ConfigHost {
    fs: Arc<dyn FS>,
}

impl ParseConfigHost for ConfigHost {
    fn fs(&self) -> &dyn FS {
        &*self.fs
    }
    fn get_current_directory(&self) -> &str {
        "/"
    }
}

fn main() {
    let mut input = "test.d.ts".to_string();
    let mut config_paths: Vec<String> = Vec::new();
    let mut overrides: Vec<String> = Vec::new();
    let mut census = false;
    let mut emit: Option<String> = None;
    let mut out_dir: Option<String> = None;
    let mut glue_for: Option<String> = None;
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
            _ => input = a,
        }
    }
    let (opts, modules) = config::load(&config_paths, &overrides).unwrap_or_else(|e| {
        eprintln!("config error: {e}");
        std::process::exit(2);
    });

    // Bare `lib.*.d.ts` names select a bundled lib file.
    let is_lib = input.starts_with("lib.") && !std::path::Path::new(&input).exists();
    let src = if is_lib { String::new() } else { std::fs::read_to_string(&input).expect("read input") };
    let tsconfig = r#"{ "compilerOptions": { "lib": ["es2020", "dom", "dom.iterable", "dom.asynciterable"], "noEmit": true }, "files": ["/input.d.ts"] }"#;
    let fs: Arc<dyn FS> = Arc::new(bundled::wrap_fs(vfstest::from_map(
        [("/tsconfig.json", tsconfig.to_string()), ("/input.d.ts", src)],
        true,
    )));
    let host: &'static ConfigHost = Box::leak(Box::new(ConfigHost { fs: Arc::clone(&fs) }));
    let (config, errs) =
        tsoptions::get_parsed_command_line_of_config_file("/tsconfig.json", None, None, host, None);
    assert!(errs.is_empty(), "tsconfig errors: {}", errs.len());

    let compiler_host = new_compiler_host("/", fs, &bundled::lib_path(), None, None, None);
    let program = new_program(ProgramOptions::new(P::new(config.unwrap()), compiler_host));
    let name = if is_lib { format!("{}/{}", bundled::lib_path(), input) } else { "/input.d.ts".into() };
    let file = program.get_source_file(&name).unwrap_or_else(|| panic!("{name} not in program"));
    let mut checker = program.get_type_checker_for_file(&Context::default(), file);

    let mut front = front::Front::new(&mut checker, name);
    let mut decls = front.statements(file.statements().nodes());
    lower::lower(&mut front, &mut decls, opts);

    let mut x = xml::Xml::default();
    x.top(&decls, &modules);
    // The strategies in effect, so a consumer can check what it is reading.
    let strategies: Vec<String> = config::describe(opts).iter().map(|(k, v)| format!("{k}=\"{v}\"")).collect();
    let mut doc = format!("<declarations>\n  <lowering {}/>\n", strategies.join(" "));
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
