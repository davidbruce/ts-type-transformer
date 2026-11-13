// tsrs feasibility prototype: dump interfaces with checker-resolved members.
// Usage: cargo run --bin tsrs_proto -- <file.d.ts | lib.dom.d.ts>

use std::collections::HashSet;
use std::sync::Arc;

use tsrs_ast::{Kind, Symbol, SymbolFlags};
use tsrs_checker::checker::Checker;
use tsrs_checker::types::{Signature, SignatureKind, Type};
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
    let arg = std::env::args().nth(1).unwrap_or_else(|| "test.d.ts".into());
    // Bare `lib.*.d.ts` names select a bundled lib file.
    let is_lib = arg.starts_with("lib.") && !std::path::Path::new(&arg).exists();
    let src = if is_lib { String::new() } else { std::fs::read_to_string(&arg).expect("read input") };

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
    let name = if is_lib { format!("{}/{}", bundled::lib_path(), arg) } else { "/input.d.ts".into() };
    let file = program.get_source_file(&name).unwrap_or_else(|| panic!("{name} not in program"));
    let mut checker = program.get_type_checker_for_file(&Context::default(), file);
    let c = &mut *checker;

    let mut seen = HashSet::new();
    for stmt in file.statements().nodes() {
        if stmt.kind() != Kind::InterfaceDeclaration {
            continue;
        }
        let Some(sym) = c.get_symbol_of_declaration_exported(*stmt) else { continue };
        if !seen.insert(sym.name()) {
            continue;
        }
        let ty = c.get_declared_type_of_symbol_exported(sym);

        println!("<interface name=\"{}\">", sym.name());
        for tp in c.get_local_type_parameters_of_class_or_interface_or_type_alias_exported(sym) {
            print_type_parameter(c, "  ", tp);
        }
        for base in c.get_base_types_exported(ty) {
            println!("  <extends type=\"{}\"/>", xml(&c.type_to_string_exported(*base)));
        }
        for sig in c.get_signatures_of_type_exported(ty, SignatureKind::Call) {
            print_signature(c, "call", *sig);
        }
        for sig in c.get_signatures_of_type_exported(ty, SignatureKind::Construct) {
            print_signature(c, "construct", *sig);
        }
        for info in c.get_index_infos_of_type_exported(ty) {
            println!(
                "  <index key=\"{}\" type=\"{}\"/>",
                xml(&c.type_to_string_exported(info.key_type())),
                xml(&c.type_to_string_exported(info.value_type()))
            );
        }
        // Includes inherited and merged members, with base type args substituted.
        for prop in c.get_properties_of_type_exported(ty) {
            let pty = c.get_type_of_symbol_exported(*prop);
            if prop.flags().intersects(SymbolFlags::Method) {
                let name = member_name(*prop);
                for sig in c.get_signatures_of_type_exported(pty, SignatureKind::Call) {
                    print_method(c, name.clone(), *sig);
                }
                continue;
            }
            let readonly = c.is_readonly_symbol_exported(*prop);
            println!(
                "  <property name=\"{}\" type=\"{}\"{}{}/>",
                xml(&member_name(*prop)),
                xml(&c.type_to_string_exported(pty)),
                attr("optional", prop.flags().intersects(SymbolFlags::Optional)),
                attr("readonly", readonly),
            );
        }
        println!("</interface>");
    }
}

fn print_method(c: &mut Checker, name: String, sig: P<Signature>) {
    println!("  <method name=\"{}\">", xml(&name));
    print_signature_body(c, sig);
    println!("  </method>");
}

fn print_signature(c: &mut Checker, tag: &str, sig: P<Signature>) {
    println!("  <{tag}>");
    print_signature_body(c, sig);
    println!("  </{tag}>");
}

fn print_signature_body(c: &mut Checker, sig: P<Signature>) {
    for tp in sig.type_parameters() {
        print_type_parameter(c, "    ", *tp);
    }
    let params = sig.parameters();
    for (i, p) in params.iter().enumerate() {
        let rest = sig.has_rest_parameter() && i == params.len() - 1;
        let pty = c.get_type_of_symbol_exported(*p);
        println!(
            "    <parameter name=\"{}\" type=\"{}\"{}{}/>",
            xml(p.name()),
            xml(&c.type_to_string_exported(pty)),
            attr("optional", !rest && i >= sig.min_argument_count()),
            attr("rest", rest),
        );
    }
    let ret = c.get_return_type_of_signature_exported(sig);
    println!("    <returns type=\"{}\"/>", xml(&c.type_to_string_exported(ret)));
}

fn print_type_parameter(c: &mut Checker, indent: &str, tp: P<Type>) {
    let name = xml(&c.type_to_string_exported(tp));
    match c.get_constraint_of_type_parameter_exported(tp) {
        Some(k) => println!("{indent}<typeParameter name=\"{name}\" extends=\"{}\"/>", xml(&c.type_to_string_exported(k))),
        None => println!("{indent}<typeParameter name=\"{name}\"/>"),
    }
}

// Symbol-keyed members have internal names like `\x7f@iterator@123`; print as `[Symbol.iterator]`.
fn member_name(sym: P<Symbol>) -> String {
    match sym.name().strip_prefix("\u{7f}@").and_then(|s| s.split('@').next()) {
        Some(well_known) => format!("[Symbol.{well_known}]"),
        None => sym.name().to_string(),
    }
}

fn attr(name: &str, on: bool) -> String {
    if on { format!(" {name}=\"true\"") } else { String::new() }
}

fn xml(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}
