// End-to-end tests of the target configs. For each target: lower a fixture
// with its config, generate bindings with a thin reference generator, compile
// them with the real toolchain, and run a test program against the fixture's
// JavaScript implementation.
//
// A target whose toolchain is not installed is skipped with a message.

mod generators;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn config(target: &str) -> PathBuf {
    root().join("examples/targets").join(format!("{target}.toml"))
}

fn run(cmd: &mut Command) -> String {
    let out = cmd.output().unwrap_or_else(|e| panic!("cannot run {cmd:?}: {e}"));
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    assert!(out.status.success(), "{cmd:?} failed:\n{text}");
    text
}

// Lowers `input` with a target config, plus `key=value` overrides, and returns the XML.
fn lower(input: &Path, target: Option<&str>, overrides: &[&str]) -> String {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_tsrs_proto"));
    cmd.arg(input);
    if let Some(t) = target {
        cmd.arg("--config").arg(config(t));
    }
    for o in overrides {
        cmd.args(["--set", o]);
    }
    let out = cmd.output().expect("run tsrs_proto");
    assert!(out.status.success(), "{cmd:?} failed:\n{}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8(out.stdout).unwrap()
}

fn find(tool: &str) -> Option<PathBuf> {
    let mut dirs: Vec<PathBuf> = env::split_paths(&env::var_os("PATH").unwrap_or_default()).collect();
    dirs.extend(env::var_os("HOME").map(|h| PathBuf::from(h).join(".dotnet/tools")));
    dirs.into_iter().map(|d| d.join(tool)).find(|p| p.is_file())
}

fn tools<const N: usize>(names: [&str; N]) -> Option<[PathBuf; N]> {
    let mut found = Vec::new();
    for n in names {
        match find(n) {
            Some(p) => found.push(p),
            None => {
                eprintln!("SKIPPED: `{n}` is not installed");
                return None;
            }
        }
    }
    found.try_into().ok()
}

// A clean directory for one fixture and target, with the fixture's XML in it.
struct Case {
    fixture: PathBuf,
    out: PathBuf,
    xml: String,
}

fn case(fixture: &str, target: &str) -> Case {
    let dir = root().join("tests/fixtures").join(fixture);
    let out = root().join("target/e2e").join(format!("{fixture}-{target}"));
    let _ = fs::remove_dir_all(&out);
    fs::create_dir_all(&out).unwrap();
    let xml = lower(&dir.join("api.d.ts"), Some(target), &[]);
    fs::write(out.join("api.xml"), &xml).unwrap();
    Case { fixture: dir, out, xml }
}

fn run_wasm(c: &Case, node: &Path) {
    let report = run(Command::new(node).arg(root().join("tests/run_wasm.mjs")).arg(c.fixture.join("impl.mjs")).arg(c.out.join("glue.mjs")).arg(c.out.join("test.wasm")));
    print!("{report}");
}

fn fsharp(fixture: &str) {
    let Some([fable, node]) = tools(["fable", "node"]) else { return };
    let c = case(fixture, "fsharp");
    let bindings = generators::fsharp::module(&c.xml);
    fs::write(c.out.join("Bindings.fs"), &bindings.source).unwrap();
    fs::copy(c.fixture.join("test.fs"), c.out.join("Test.fs")).unwrap();
    fs::write(c.out.join("Test.fsproj"), FSPROJ).unwrap();
    run(Command::new(fable).current_dir(&c.out).args(["Test.fsproj", "-o", "js", "--extension", ".mjs", "--noCache"]));
    let report = run(Command::new(node).arg(root().join("tests/run_fable.mjs")).arg(c.fixture.join("impl.mjs")).arg(c.out.join("js/Test.mjs")));
    print!("{report}{}", skipped(&bindings.skipped));
}

const FSPROJ: &str = r#"<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup><TargetFramework>net8.0</TargetFramework></PropertyGroup>
  <ItemGroup><Compile Include="Bindings.fs" /><Compile Include="Test.fs" /></ItemGroup>
  <ItemGroup><PackageReference Include="Fable.Core" Version="4.*" /></ItemGroup>
</Project>
"#;

const CLANG_WASM: &[&str] = &["--target=wasm32", "-nostdlib", "-ffreestanding", "-O1", "-Wall", "-Werror", "-Wl,--no-entry", "-Wl,--allow-undefined", "-Wl,--export-table"];
const ZIG_WASM: &[&str] = &["build-exe", "test.zig", "-target", "wasm32-freestanding", "-fno-entry", "-rdynamic", "--export-table", "-O", "ReleaseSmall"];

fn c(fixture: &str) {
    let Some([clang, node]) = tools(["clang", "node"]) else { return };
    let c = case(fixture, "c");
    let model = generators::wasm::model(&c.xml, "C");
    fs::write(c.out.join("bindings.h"), generators::c::header(&model)).unwrap();
    fs::write(c.out.join("glue.mjs"), generators::wasm::glue(&model)).unwrap();
    run(Command::new(clang).args(CLANG_WASM).arg("-I").arg(&c.out).arg("-o").arg(c.out.join("test.wasm")).arg(c.fixture.join("test.c")));
    run_wasm(&c, &node);
    print!("{}", skipped(&model.skipped));
}

fn zig(fixture: &str) {
    let Some([zig, node]) = tools(["zig", "node"]) else { return };
    let c = case(fixture, "zig");
    let model = generators::wasm::model(&c.xml, "Zig");
    fs::write(c.out.join("bindings.zig"), generators::zig::module(&model)).unwrap();
    fs::write(c.out.join("glue.mjs"), generators::wasm::glue(&model)).unwrap();
    fs::copy(c.fixture.join("test.zig"), c.out.join("test.zig")).unwrap();
    run(Command::new(zig).current_dir(&c.out).args(ZIG_WASM));
    run_wasm(&c, &node);
    print!("{}", skipped(&model.skipped));
}

// What a generator left unbound, for the test log.
fn skipped(counts: &std::collections::BTreeMap<String, usize>) -> String {
    counts.iter().map(|(why, n)| format!("  not bound: {n} x {why}\n")).collect()
}

// One test per fixture and target.
macro_rules! fixtures {
    ($($fixture:ident),*) => { $(mod $fixture {
        #[test] fn fsharp() { super::fsharp(stringify!($fixture)); }
        #[test] fn c() { super::c(stringify!($fixture)); }
        #[test] fn zig() { super::zig(stringify!($fixture)); }
    })* };
}

fixtures!(shapes, callbacks, data, globals);

// The generators must refuse what their target cannot express; otherwise a
// config that lowers too little would go unnoticed.

fn shapes_xml(target: &str, overrides: &[&str]) -> String {
    lower(&root().join("tests/fixtures/shapes/api.d.ts"), Some(target), overrides)
}

#[test]
#[should_panic(expected = "cannot express a union")]
fn c_rejects_unions() {
    let xml = shapes_xml("c", &["union_params=keep", "unions=keep"]);
    generators::c::header(&generators::wasm::model(&xml, "C"));
}

#[test]
#[should_panic(expected = "cannot express <extends>")]
fn c_rejects_inheritance() {
    generators::wasm::model(&shapes_xml("c", &["inheritance=keep"]), "C");
}

#[test]
#[should_panic(expected = "cannot express <parameter>")]
fn zig_rejects_optional_parameters() {
    generators::wasm::model(&shapes_xml("zig", &["optional_params=keep"]), "Zig");
}

#[test]
#[should_panic(expected = "cannot express <literal>")]
fn fsharp_rejects_literal_types() {
    generators::fsharp::module(&shapes_xml("fsharp", &["literals=keep"]));
}

// Whatever the input, a construct the config lowers must not appear in the output.
#[test]
fn lowered_constructs_are_absent() {
    use generators::{attr, elements, flag, path};
    let inputs = ["tests/fixtures/shapes/api.d.ts", "examples/lowering.d.ts", "examples/features.d.ts", "lib.es5.d.ts"];
    for target in ["fsharp", "ocaml", "rust", "c", "zig"] {
        for input in inputs {
            // A bare `lib.*.d.ts` name selects a bundled library.
            let file = if input.starts_with("lib.") { PathBuf::from(input) } else { root().join(input) };
            let xml = lower(&file, Some(target), &[]);
            let doc = roxmltree::Document::parse(&xml).unwrap_or_else(|e| panic!("{target} {input}: {e}"));
            let lowering = doc.root_element().first_element_child().expect("<lowering>");
            let on = |key: &str| attr(lowering, key) != "keep";
            let mut problems = Vec::new();
            for n in doc.descendants().filter(|n| n.is_element()) {
                let tag = n.tag_name().name();
                let parent = n.parent_element().map(|p| p.tag_name().name()).unwrap_or_default();
                let in_decl = matches!(parent, "interface" | "class");
                // Inline types may only remain as the body of a named declaration.
                let inline = |parent: &str| on("anonymous") && !matches!(parent, "typeAlias" | "var");
                let bad = match tag {
                    "extends" if in_decl => on("inheritance"),
                    "typeParameter" => on("generics"),
                    "keyof" | "indexedAccess" | "conditional" | "mapped" | "template" | "infer" | "typeof" | "predicate" | "raw" => on("type_ops"),
                    "intersection" => on("intersections"),
                    "literal" => on("literals"),
                    "union" => on("unions") || on("nullable") && elements(n).any(|c| matches!(c.attribute("name"), Some("null" | "undefined")) && c.has_tag_name("primitive")),
                    "parameter" if flag(n, "optional") => on("optional_params") && !matches!(parent, "functionType" | "constructorType"),
                    "functionType" | "constructorType" => inline(parent),
                    _ => false,
                };
                // Members of one type must have distinct names once overloads are renamed.
                let duplicate_names = matches!(tag, "interface" | "class" | "object") && on("overloads") && {
                    let mut names: Vec<&str> = elements(n).filter(|m| m.has_tag_name("method") && !flag(*m, "static")).map(|m| attr(m, "name")).collect();
                    let count = names.len();
                    names.sort();
                    names.dedup();
                    names.len() != count
                };
                let bad = bad || duplicate_names || tag == "object" && inline(parent);
                if bad {
                    problems.push(format!("<{tag}> at {}", path(n)));
                }
            }
            assert!(problems.is_empty(), "{target} config, {input}: {} constructs should have been lowered, e.g.\n  {}", problems.len(), problems[..problems.len().min(5)].join("\n  "));
        }
    }
}

// ---- the whole DOM ----
//
// Bindings for all of lib.dom.d.ts, compiled, then run in a real browser.

fn dom_case(target: &str) -> Case {
    let out = root().join("target/e2e").join(format!("dom-{target}"));
    let _ = fs::remove_dir_all(&out);
    fs::create_dir_all(&out).unwrap();
    let xml = lower(Path::new("lib.dom.d.ts"), Some(target), &[]);
    fs::write(out.join("api.xml"), &xml).unwrap();
    Case { fixture: root().join("tests/dom"), out, xml }
}

mod dom {
    use super::*;

    #[test]
    fn c() {
        let Some([clang]) = tools(["clang"]) else { return };
        let c = dom_case("c");
        let model = generators::wasm::model(&c.xml, "C");
        fs::write(c.out.join("bindings.h"), generators::c::header(&model)).unwrap();
        fs::write(c.out.join("glue.mjs"), generators::wasm::glue(&model)).unwrap();
        print!("{}", skipped(&model.skipped));
        run(Command::new(clang).args(CLANG_WASM).arg("-I").arg(&c.out).arg("-o").arg(c.out.join("test.wasm")).arg(c.fixture.join("test.c")));
    }

    #[test]
    fn fsharp() {
        let Some([fable]) = tools(["fable"]) else { return };
        let c = dom_case("fsharp");
        let bindings = generators::fsharp::module(&c.xml);
        fs::write(c.out.join("Bindings.fs"), &bindings.source).unwrap();
        fs::copy(c.fixture.join("test.fs"), c.out.join("Test.fs")).unwrap();
        fs::write(c.out.join("Test.fsproj"), FSPROJ).unwrap();
        print!("{}", skipped(&bindings.skipped));
        run(Command::new(fable).current_dir(&c.out).args(["Test.fsproj", "-o", "js", "--extension", ".mjs", "--noCache"]));
    }

    #[test]
    fn zig() {
        let Some([zig]) = tools(["zig"]) else { return };
        let c = dom_case("zig");
        let model = generators::wasm::model(&c.xml, "Zig");
        fs::write(c.out.join("bindings.zig"), generators::zig::module(&model)).unwrap();
        fs::write(c.out.join("glue.mjs"), generators::wasm::glue(&model)).unwrap();
        fs::copy(c.fixture.join("test.zig"), c.out.join("test.zig")).unwrap();
        print!("{}", skipped(&model.skipped));
        run(Command::new(zig).current_dir(&c.out).args(ZIG_WASM));
    }
}
