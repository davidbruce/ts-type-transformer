// tsrs feasibility prototype: dump interfaces with checker-resolved members.
// Usage: cargo run --bin tsrs_proto -- test.d.ts

use std::sync::Arc;

use tsrs_ast::{Kind, SymbolFlags};
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
    let path = std::env::args().nth(1).unwrap_or_else(|| "test.d.ts".into());
    let src = std::fs::read_to_string(&path).expect("read input");

    let tsconfig = r#"{ "compilerOptions": { "lib": ["es2020", "dom"], "noEmit": true }, "files": ["/input.d.ts"] }"#;
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
    let file = program.get_source_file("/input.d.ts").expect("input in program");
    let mut checker = program.get_type_checker_for_file(&Context::default(), file);
    let c = &mut *checker;

    for stmt in file.statements().nodes() {
        if stmt.kind() != Kind::InterfaceDeclaration {
            continue;
        }
        let Some(sym) = c.get_symbol_of_declaration_exported(*stmt) else { continue };
        let ty = c.get_declared_type_of_symbol_exported(sym);

        println!("<interface name=\"{}\">", xml(&c.type_to_string_exported(ty)));
        for base in c.get_base_types_exported(ty) {
            println!("    <extends>{}</extends>", xml(&c.type_to_string_exported(*base)));
        }
        // Includes inherited and merged members, with base type args substituted.
        for prop in c.get_properties_of_type_exported(ty) {
            let pty = c.get_type_of_symbol_exported(*prop);
            let optional = prop.flags().intersects(SymbolFlags::Optional);
            println!(
                "    <property name=\"{}\" optional=\"{}\">{}</property>",
                prop.name(),
                optional,
                xml(&c.type_to_string_exported(pty))
            );
        }
        println!("</interface>");
    }
}

fn xml(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}
