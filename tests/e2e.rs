// End-to-end tests of the target configs. For each target: lower a fixture
// with its config, generate bindings with a thin reference generator, compile
// them with the real toolchain, and run a test program against the fixture's
// JavaScript implementation.
//
// A target whose toolchain is not installed is skipped with a message.

use ts_type_transformer::generators;

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
    lower_with(input, target, &[], overrides)
}

// As `lower`, with more config files after the target's: module areas, say.
fn lower_with(input: &Path, target: Option<&str>, configs: &[PathBuf], overrides: &[&str]) -> String {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_tsrs_proto"));
    cmd.arg(input);
    if let Some(t) = target {
        cmd.arg("--config").arg(config(t));
    }
    for c in configs {
        cmd.arg("--config").arg(c);
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
    case_in(fixture, target, &format!("{fixture}-{target}"))
}

// As `case`, in `target/e2e/<name>`.
fn case_in(fixture: &str, target: &str, name: &str) -> Case {
    let dir = root().join("tests/fixtures").join(fixture);
    let out = root().join("target/e2e").join(name);
    let _ = fs::remove_dir_all(&out);
    fs::create_dir_all(&out).unwrap();
    let xml = lower(&dir.join("api.d.ts"), Some(target), &[]);
    fs::write(out.join("api.xml"), &xml).unwrap();
    Case { fixture: dir, out, xml }
}

// Writes generated files under `dir`, creating directories as needed.
fn write_files(dir: &Path, files: Vec<(String, String)>) {
    for (path, content) in files {
        let path = dir.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }
}

// The glue for what the compiled module imports, and nothing else.
fn write_glue(c: &Case, model: &generators::wasm::Model) {
    let used = generators::wasm::imports(&fs::read(c.out.join("test.wasm")).unwrap());
    fs::write(c.out.join("glue.mjs"), generators::wasm::glue(model, Some(&used))).unwrap();
}

fn run_wasm(c: &Case, node: &Path) {
    let report = run(Command::new(node).arg(root().join("tests/run_wasm.mjs")).arg(c.fixture.join("impl.mjs")).arg(c.out.join("glue.mjs")).arg(c.out.join("test.wasm")));
    print!("{report}");
}

fn fsharp(fixture: &str) {
    let Some([fable, node]) = tools(["fable", "node"]) else { return };
    let c = case(fixture, "fsharp");
    let bindings = generators::fsharp::module(&c.xml);
    write_fsharp(&c, &bindings);
    run(Command::new(fable).current_dir(&c.out).args(["Test.fsproj", "-o", "js", "--extension", ".mjs", "--noCache"]));
    let report = run(Command::new(node).arg(root().join("tests/run_fable.mjs")).arg(c.fixture.join("impl.mjs")).arg(c.out.join("js/Test.mjs")));
    print!("{report}{}", skipped(&bindings.skipped));
}

// The bindings, the test, and a project that compiles them in order.
fn write_fsharp(c: &Case, bindings: &generators::fsharp::Output) {
    let mut compile = String::new();
    for (name, content) in &bindings.files {
        fs::write(c.out.join(name), content).unwrap();
        compile += &format!("    <Compile Include=\"{name}\" />\n");
    }
    fs::copy(c.fixture.join("test.fs"), c.out.join("Test.fs")).unwrap();
    fs::write(
        c.out.join("Test.fsproj"),
        format!(
            r#"<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup><TargetFramework>net8.0</TargetFramework></PropertyGroup>
  <ItemGroup>
{compile}    <Compile Include="Test.fs" />
  </ItemGroup>
  <ItemGroup><PackageReference Include="Fable.Core" Version="5.6.1" /></ItemGroup>
</Project>
"#
        ),
    )
    .unwrap();
}

const CLANG_WASM: &[&str] = &["--target=wasm32", "-std=c11", "-pedantic", "-nostdlib", "-ffreestanding", "-O1", "-Wall", "-Werror", "-Wl,--no-entry", "-Wl,--allow-undefined", "-Wl,--export-table"];
const ZIG_WASM: &[&str] = &["build-exe", "test.zig", "-target", "wasm32-freestanding", "-fno-entry", "-rdynamic", "--export-table", "-O", "ReleaseSmall"];

fn c(fixture: &str) {
    let Some([clang, node]) = tools(["clang", "node"]) else { return };
    let c = case(fixture, "c");
    let model = generators::wasm::model(&c.xml, "C");
    write_files(&c.out.join("bindings"), generators::c::files(&model));
    run(Command::new(clang).args(CLANG_WASM).arg("-I").arg(c.out.join("bindings")).arg("-o").arg(c.out.join("test.wasm")).arg(c.fixture.join("test.c")));
    write_glue(&c, &model);
    run_wasm(&c, &node);
    print!("{}", skipped(&model.skipped));
}

fn zig(fixture: &str) {
    let Some([zig, node]) = tools(["zig", "node"]) else { return };
    let c = case(fixture, "zig");
    let model = generators::wasm::model(&c.xml, "Zig");
    write_files(&c.out, generators::zig::files(&model));
    fs::copy(c.fixture.join("test.zig"), c.out.join("test.zig")).unwrap();
    run(Command::new(zig).current_dir(&c.out).args(ZIG_WASM));
    write_glue(&c, &model);
    run_wasm(&c, &node);
    print!("{}", skipped(&model.skipped));
}

// The WASI SDK's clang++, which has a C++ standard library for wasm32.
fn wasi_clang() -> Option<PathBuf> {
    let from_env = env::var_os("WASI_SDK_PATH").map(|p| PathBuf::from(p).join("bin/clang++"));
    let from_home = env::var_os("HOME").and_then(|h| {
        let mut sdks: Vec<PathBuf> = fs::read_dir(PathBuf::from(h).join(".wasi-sdk")).ok()?.filter_map(|e| e.ok()).map(|e| e.path().join("bin/clang++")).collect();
        sdks.sort();
        sdks.pop()
    });
    let found = from_env.into_iter().chain(from_home).find(|p| p.is_file());
    if found.is_none() {
        eprintln!("SKIPPED: no WASI SDK (set WASI_SDK_PATH)");
    }
    found
}

// No `--allow-undefined`: the imports are declared as such, and anything else
// missing is a link error rather than an import that fails when the page loads.
const CLANGXX_WASM: &[&str] = &["--target=wasm32-wasip1", "-mexec-model=reactor", "-std=c++20", "-fno-exceptions", "-O1", "-Wall", "-Werror", "-Wl,--export-table"];

// Writes the C++ bindings, compiles their definitions once, each module in
// parallel, and links them with the program.
fn build_cpp(clang: &Path, c: &Case, model: &generators::wasm::Model) {
    let bindings = c.out.join("bindings");
    let files = generators::cpp::files(model);
    let sources: Vec<String> = files.iter().map(|f| f.0.clone()).filter(|p| p.ends_with(".cpp")).collect();
    write_files(&bindings, files);
    let flags: Vec<&str> = CLANGXX_WASM.iter().filter(|f| !f.starts_with("-Wl,") && !f.starts_with("-mexec-model")).copied().collect();
    let objects: Vec<PathBuf> = sources.iter().map(|s| c.out.join("obj").join(s.replace('/', "_")).with_extension("o")).collect();
    fs::create_dir_all(c.out.join("obj")).unwrap();
    let next = std::sync::atomic::AtomicUsize::new(0);
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| {
                while let Some(i) = Some(next.fetch_add(1, std::sync::atomic::Ordering::SeqCst)).filter(|i| *i < sources.len()) {
                    run(Command::new(clang).args(&flags).arg("-c").arg("-I").arg(&bindings).arg(bindings.join(&sources[i])).arg("-o").arg(&objects[i]));
                }
            });
        }
    });
    run(Command::new(clang).args(CLANGXX_WASM).arg("-I").arg(&bindings).arg("-o").arg(c.out.join("test.wasm")).arg(c.fixture.join("test.cpp")).args(&objects));
}

fn cpp(fixture: &str) {
    let Some(clang) = wasi_clang() else { return };
    let Some([node]) = tools(["node"]) else { return };
    let c = case(fixture, "cpp");
    let model = generators::wasm::model(&c.xml, "C++");
    build_cpp(&clang, &c, &model);
    write_glue(&c, &model);
    run_wasm(&c, &node);
    print!("{}", skipped(&model.skipped));
}

// rustc, where the wasm32 target is installed.
fn rustc_wasm() -> Option<PathBuf> {
    let rustc = find("rustc")?;
    let out = Command::new(&rustc).args(["--print", "target-libdir", "--target", "wasm32-unknown-unknown"]).output().ok()?;
    if !Path::new(String::from_utf8_lossy(&out.stdout).trim()).is_dir() {
        eprintln!("SKIPPED: no wasm32-unknown-unknown target (rustup target add wasm32-unknown-unknown)");
        return None;
    }
    Some(rustc)
}

const RUSTC_WASM: &[&str] = &["--edition", "2024", "--target", "wasm32-unknown-unknown", "--crate-type", "cdylib", "-O", "-C", "link-arg=--export-table", "-D", "warnings"];

// Writes the Rust bindings next to the program and compiles them together.
fn build_rust(rustc: &Path, c: &Case, model: &generators::wasm::Model) {
    write_files(&c.out, generators::rust::files(model));
    fs::copy(c.fixture.join("test.rs"), c.out.join("test.rs")).unwrap();
    run(Command::new(rustc).current_dir(&c.out).args(RUSTC_WASM).args(["-o", "test.wasm", "test.rs"]));
}

fn rust(fixture: &str) {
    let Some(rustc) = rustc_wasm() else { return };
    let Some([node]) = tools(["node"]) else { return };
    let c = case(fixture, "rust");
    let model = generators::wasm::model(&c.xml, "Rust");
    build_rust(&rustc, &c, &model);
    write_glue(&c, &model);
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
        #[test] fn cpp() { super::cpp(stringify!($fixture)); }
        #[test] fn rust() { super::rust(stringify!($fixture)); }
    })* };
}

fixtures!(shapes, callbacks, data, globals, unions);

// The generators must refuse what their target cannot express; otherwise a
// config that lowers too little would go unnoticed.

fn shapes_xml(target: &str, overrides: &[&str]) -> String {
    lower(&root().join("tests/fixtures/shapes/api.d.ts"), Some(target), overrides)
}

#[test]
#[should_panic(expected = "cannot express a union")]
fn c_rejects_unions() {
    let xml = shapes_xml("c", &["union_params=keep", "unions=keep"]);
    generators::c::files(&generators::wasm::model(&xml, "C"));
}

// Mistakes the glue reports: a handle used after its release, a handle
// released twice, and an exception no one took.
#[test]
fn mistakes_are_reported() {
    let Some([clang, node]) = tools(["clang", "node"]) else { return };
    let programs = [
        ("shapes", "used-after-release", "Shape s = Shape_unit(); js_release(s); Shape_area(s);", "used after its release"),
        ("shapes", "released-twice", "Shape s = Shape_unit(); js_release(s); js_release(s);", "released twice"),
        ("callbacks", "not-taken", "explode(\"boom\");", "unchecked exception: boom"),
    ];
    for (fixture, name, body, expected) in programs {
        let c = case_in(fixture, "c", &format!("mistake-{name}"));
        let model = generators::wasm::model(&c.xml, "C");
        write_files(&c.out.join("bindings"), generators::c::files(&model));
        fs::write(c.out.join("test.c"), format!("#include \"bindings.h\"\nTS_IMPORT(check) void check(int ok, int line);\n__attribute__((export_name(\"run\"))) void run(void) {{ check(1, 1); {body} }}\n")).unwrap();
        run(Command::new(&clang).args(CLANG_WASM).arg("-I").arg(c.out.join("bindings")).arg("-o").arg(c.out.join("test.wasm")).arg(c.out.join("test.c")));
        write_glue(&c, &model);
        let out = Command::new(&node).arg(root().join("tests/run_wasm.mjs")).arg(c.fixture.join("impl.mjs")).arg(c.out.join("glue.mjs")).arg(c.out.join("test.wasm")).output().unwrap();
        let report = String::from_utf8_lossy(&out.stderr);
        assert!(!out.status.success() && report.contains(expected), "{name} should be reported as `{expected}`:\n{report}");
    }
}

// Benchmarks: the same workloads in TypeScript and through each target's
// bindings, timed from JavaScript. Not part of the suite:
// `cargo test --test e2e bench -- --ignored --nocapture`.
#[test]
#[ignore]
fn bench() {
    let Some([node]) = tools(["node"]) else { return };
    let dir = root().join("tests/bench");
    let out = root().join("target/e2e/bench");
    let _ = fs::remove_dir_all(&out);
    let xml = |target: &str| lower(&dir.join("api.d.ts"), Some(target), &[]);
    let glue = |o: &Path, model: &generators::wasm::Model| {
        let used = generators::wasm::imports(&fs::read(o.join("bench.wasm")).unwrap());
        fs::write(o.join("glue.mjs"), generators::wasm::glue(model, Some(&used))).unwrap();
    };
    let optimized = |flags: &[&'static str]| -> Vec<&'static str> { flags.iter().map(|f| if *f == "-O1" { "-O2" } else { *f }).collect() };
    let mut names: Vec<&str> = Vec::new();
    if let Some(fable) = find("fable") {
        let o = out.join("fsharp");
        fs::create_dir_all(&o).unwrap();
        let bindings = generators::fsharp::module(&xml("fsharp"));
        let mut compile = String::new();
        for (name, content) in &bindings.files {
            fs::write(o.join(name), content).unwrap();
            compile += &format!("    <Compile Include=\"{name}\" />\n");
        }
        fs::copy(dir.join("bench.fs"), o.join("Bench.fs")).unwrap();
        fs::write(o.join("Bench.fsproj"), format!("<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup><TargetFramework>net8.0</TargetFramework></PropertyGroup>\n  <ItemGroup>\n{compile}    <Compile Include=\"Bench.fs\" />\n  </ItemGroup>\n  <ItemGroup><PackageReference Include=\"Fable.Core\" Version=\"5.6.1\" /></ItemGroup>\n</Project>\n")).unwrap();
        run(Command::new(fable).current_dir(&o).args(["Bench.fsproj", "-o", "js", "--extension", ".mjs", "--noCache"]));
        names.push("fsharp");
    }
    if let Some(clang) = find("clang") {
        let o = out.join("c");
        let model = generators::wasm::model(&xml("c"), "C");
        write_files(&o.join("bindings"), generators::c::files(&model));
        run(Command::new(clang).args(optimized(CLANG_WASM)).arg("-I").arg(o.join("bindings")).arg("-o").arg(o.join("bench.wasm")).arg(dir.join("bench.c")));
        glue(&o, &model);
        names.push("c");
    }
    if let Some(clang) = wasi_clang() {
        let o = out.join("cpp");
        let model = generators::wasm::model(&xml("cpp"), "C++");
        let files = generators::cpp::files(&model);
        let sources: Vec<PathBuf> = files.iter().filter(|f| f.0.ends_with(".cpp")).map(|f| o.join("bindings").join(&f.0)).collect();
        write_files(&o.join("bindings"), files);
        run(Command::new(clang).args(optimized(CLANGXX_WASM)).arg("-I").arg(o.join("bindings")).arg("-o").arg(o.join("bench.wasm")).arg(dir.join("bench.cpp")).args(&sources));
        glue(&o, &model);
        names.push("cpp");
    }
    if let Some(zig) = find("zig") {
        let o = out.join("zig");
        let model = generators::wasm::model(&xml("zig"), "Zig");
        write_files(&o, generators::zig::files(&model));
        fs::copy(dir.join("bench.zig"), o.join("bench.zig")).unwrap();
        run(Command::new(zig).current_dir(&o).args(["build-exe", "bench.zig", "-target", "wasm32-freestanding", "-fno-entry", "-rdynamic", "--export-table", "-O", "ReleaseFast"]));
        glue(&o, &model);
        names.push("zig");
    }
    if let Some(rustc) = rustc_wasm() {
        let o = out.join("rust");
        let model = generators::wasm::model(&xml("rust"), "Rust");
        write_files(&o, generators::rust::files(&model));
        fs::copy(dir.join("bench.rs"), o.join("bench.rs")).unwrap();
        run(Command::new(rustc).current_dir(&o).args(RUSTC_WASM).args(["-C", "opt-level=3", "-o", "bench.wasm", "bench.rs"]));
        glue(&o, &model);
        names.push("rust");
    }
    let report = run(Command::new(node).arg(root().join("tests/run_bench.mjs")).arg(dir.join("impl.mjs")).arg(dir.join("bench.ts")).arg(&out).args(&names));
    print!("{report}");
}

// `--emit` writes bindings with their glue; what a generator cannot express is an error.
#[test]
fn emit_writes_bindings_or_says_why_not() {
    let out = root().join("target/e2e/emit");
    let _ = fs::remove_dir_all(&out);
    let shapes = root().join("tests/fixtures/shapes/api.d.ts");
    let run = |extra: &[&str]| Command::new(env!("CARGO_BIN_EXE_tsrs_proto")).arg(&shapes).arg("--config").arg(config("c")).args(extra).arg("--emit").arg("c").arg("--out").arg(&out).output().unwrap();
    let ok = run(&[]);
    assert!(ok.status.success() && out.join("bindings.h").exists() && out.join("glue.mjs").exists(), "{}", String::from_utf8_lossy(&ok.stderr));
    let bad = run(&["--set", "unions=keep", "--set", "union_params=keep"]);
    let errors = String::from_utf8_lossy(&bad.stderr);
    assert!(!bad.status.success() && errors.contains("cannot express a union"), "{errors}");
}

// Generated names do not depend on the order of declarations.
#[test]
fn generated_names_do_not_depend_on_order() {
    let decls = [
        "declare function zoom(point: { x: number; y: number }): number;",
        "declare function at(point: { x: number; y: number }): number;",
        "declare function pick(mode: \"a\" | \"b\"): number;",
        "interface Area { mode: \"a\" | \"b\"; corner: { x: number; y: number } }",
    ];
    let dir = root().join("target/e2e/names-order");
    fs::create_dir_all(&dir).unwrap();
    let names = |order: &[usize], file: &str| {
        let path = dir.join(file);
        fs::write(&path, order.iter().map(|&i| decls[i]).collect::<Vec<_>>().join("\n")).unwrap();
        let xml = lower(&path, Some("c"), &[]);
        let doc = roxmltree::Document::parse(&xml).unwrap();
        let mut out: Vec<String> = doc.root_element().children().filter(|n| n.attribute("synthetic") == Some("true")).filter_map(|n| n.attribute("name").map(str::to_string)).collect();
        out.sort();
        out
    };
    let forward = names(&[0, 1, 2, 3], "forward.d.ts");
    assert!(!forward.is_empty());
    assert_eq!(forward, names(&[3, 2, 1, 0], "backward.d.ts"));
}

// Kept optional parameters are optional values, not overloads.
#[test]
fn optional_parameters_are_optional_values() {
    let m = generators::wasm::model(&shapes_xml("zig", &[]), "Zig");
    assert!(m.all_funcs().any(|f| f.params.iter().any(|p| matches!(p.1, generators::wasm::Ty::Nullable(..)))));
}

#[test]
#[should_panic(expected = "cannot express <literal>")]
fn fsharp_rejects_literal_types() {
    generators::fsharp::module(&shapes_xml("fsharp", &["literals=keep"]));
}

// A type argument outside its parameter's constraint is an F# compile error:
// `largest<T extends Item>` takes no array of numbers.
#[test]
fn fsharp_checks_constraints() {
    let Some([fable]) = tools(["fable"]) else { return };
    let c = case_in("data", "fsharp", "data-fsharp-wrong");
    write_fsharp(&c, &generators::fsharp::module(&c.xml));
    fs::write(c.out.join("Test.fs"), "module Test\nopen Bindings\nopen type Bindings.Globals\nlet wrong = largest (ResizeArray<float>())\nlet held: Holder<float> = null\n").unwrap();
    let out = Command::new(fable).current_dir(&c.out).args(["Test.fsproj", "-o", "js", "--extension", ".mjs", "--noCache"]).output().unwrap();
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    assert!(!out.status.success() && text.contains("Test.fs") && text.contains("Item"), "a float array should not satisfy T :> Item:\n{text}");
}

// Whatever the input, a construct the config lowers must not appear in the output.
#[test]
fn lowered_constructs_are_absent() {
    use generators::{attr, elements, flag, path};
    let inputs = ["tests/fixtures/shapes/api.d.ts", "examples/lowering.d.ts", "examples/features.d.ts", "lib.es5.d.ts"];
    for target in ["fsharp", "ocaml", "rust", "c", "zig", "cpp"] {
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
                    // Named unions remain, as the body of their declaration.
                    "union" => (match attr(lowering, "unions") {
                        "erase" => true,
                        "name" => parent != "typeAlias",
                        _ => false,
                    }) || on("nullable") && elements(n).any(|c| matches!(c.attribute("name"), Some("null" | "undefined")) && c.has_tag_name("primitive")),
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
    // Split by the DOM's areas, as a real build would.
    let xml = lower_with(Path::new("lib.dom.d.ts"), Some(target), &[root().join("examples/modules/dom.toml")], &[]);
    fs::write(out.join("api.xml"), &xml).unwrap();
    Case { fixture: root().join("tests/dom"), out, xml }
}

// The page a wasm target runs in: loads the module with the generated glue,
// runs it, then inspects the page.
fn wasm_page(lang: &str) -> String {
    format!(
        r#"<!doctype html>
<title>untouched</title>
<div id="seed" data-x="42">seed text</div>
<script type="module">
import {{ makeEnv }} from './glue.mjs';
import {{ verify }} from './verify.mjs';
let instance;
let passed = 0;
const failed = [];
const options = {{ check: true, unchecked: e => failed.push('unchecked exception: ' + (e?.message ?? e)) }};
const env = {{ ...makeEnv(() => instance, options), check: (ok, line) => (ok ? passed++ : failed.push('line ' + line)) }};
const wasi = new Proxy({{}}, {{ get: () => () => 0 }});
({{ instance }} = await WebAssembly.instantiate(await (await fetch('./test.wasm')).arrayBuffer(), {{ env, wasi_snapshot_preview1: wasi }}));
instance.exports._initialize?.();
instance.exports.run();
await new Promise(r => setTimeout(r, 500));
instance.exports.finish?.();
const page = verify('{lang}');
window.__result = {{ passed, failed, dom: page.problems, domChecks: page.checks }};
</script>
"#
    )
}

// The page the F# target runs in: the compiled program runs when imported.
const FSHARP_PAGE: &str = r#"<!doctype html>
<title>untouched</title>
<div id="seed" data-x="42">seed text</div>
<script type="module">
import { verify } from './verify.mjs';
try {
  await import('./js/Test.mjs');
} catch (e) {
  window.__fsharp = { passed: 0, failed: ['threw: ' + e] };
}
const page = verify('fsharp');
const { passed, failed } = window.__fsharp ?? { passed: 0, failed: ['the program did not report'] };
window.__result = { passed, failed, dom: page.problems, domChecks: page.checks };
</script>
"#;

// Runs `index.html` of the case in a real browser.
fn run_browser(c: &Case) {
    let Some([node]) = tools(["node"]) else { return };
    let out = Command::new(node).arg(root().join("tests/run_browser.mjs")).arg(&c.out).arg(&c.fixture).output().expect("run node");
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    // 77: no browser installed.
    if out.status.code() == Some(77) {
        return eprint!("{text}");
    }
    assert!(out.status.success(), "browser test failed:\n{text}");
    print!("{text}");
}

mod dom {
    use super::*;

    #[test]
    fn c() {
        let Some([clang]) = tools(["clang"]) else { return };
        let c = dom_case("c");
        let model = generators::wasm::model(&c.xml, "C");
        write_files(&c.out.join("bindings"), generators::c::files(&model));
        fs::write(c.out.join("index.html"), wasm_page("c")).unwrap();
        print!("{}", skipped(&model.skipped));
        run(Command::new(clang).args(CLANG_WASM).arg("-I").arg(c.out.join("bindings")).arg("-o").arg(c.out.join("test.wasm")).arg(c.fixture.join("test.c")));
        write_glue(&c, &model);
        run_browser(&c);
        // Every header compiles, not only those the program includes.
        let flags: Vec<&str> = CLANG_WASM.iter().filter(|f| !f.starts_with("-Wl,")).copied().collect();
        fs::write(c.out.join("all.c"), "#include \"bindings.h\"\n").unwrap();
        run(Command::new(find("clang").unwrap()).args(&flags).arg("-fsyntax-only").arg("-I").arg(c.out.join("bindings")).arg(c.out.join("all.c")));
        // A call with a type that is not a subtype is a compile error.
        let out = Command::new(find("clang").unwrap()).args(&flags).arg("-fsyntax-only").arg("-I").arg(c.out.join("bindings")).arg(c.fixture.join("wrong.c")).output().unwrap();
        let errors = String::from_utf8_lossy(&out.stderr);
        assert!(!out.status.success() && errors.contains("not compatible with any generic association"), "wrong.c should not compile:\n{errors}");
    }
    #[test]
    fn cpp() {
        let Some(clang) = wasi_clang() else { return };
        let c = dom_case("cpp");
        let model = generators::wasm::model(&c.xml, "C++");
        fs::write(c.out.join("index.html"), wasm_page("cpp")).unwrap();
        print!("{}", skipped(&model.skipped));
        build_cpp(&clang, &c, &model);
        write_glue(&c, &model);
        run_browser(&c);
        // A listener of the wrong event type is a compile error.
        let flags = CLANGXX_WASM.iter().filter(|f| !f.starts_with("-Wl,") && !f.starts_with("-mexec-model"));
        let out = Command::new(&clang).args(flags).arg("-fsyntax-only").arg("-I").arg(c.out.join("bindings")).arg(c.fixture.join("wrong.cpp")).output().unwrap();
        let errors = String::from_utf8_lossy(&out.stderr);
        let expected = ["no matching member function for call to 'addEventListener'", "constraints not satisfied"];
        assert!(!out.status.success() && expected.iter().all(|e| errors.contains(e)), "wrong.cpp should not compile, for both reasons:\n{errors}");
    }

    #[test]
    fn rust() {
        let Some(rustc) = super::rustc_wasm() else { return };
        let c = dom_case("rust");
        let model = generators::wasm::model(&c.xml, "Rust");
        fs::write(c.out.join("index.html"), wasm_page("rust")).unwrap();
        print!("{}", skipped(&model.skipped));
        super::build_rust(&rustc, &c, &model);
        write_glue(&c, &model);
        run_browser(&c);
    }

    #[test]
    fn fsharp() {
        let Some([fable]) = tools(["fable"]) else { return };
        let c = dom_case("fsharp");
        let bindings = generators::fsharp::module(&c.xml);
        write_fsharp(&c, &bindings);
        fs::write(c.out.join("index.html"), FSHARP_PAGE).unwrap();
        print!("{}", skipped(&bindings.skipped));
        run(Command::new(fable).current_dir(&c.out).args(["Test.fsproj", "-o", "js", "--extension", ".mjs", "--noCache"]));
        run_browser(&c);
    }

    #[test]
    fn zig() {
        let Some([zig]) = tools(["zig"]) else { return };
        let c = dom_case("zig");
        let model = generators::wasm::model(&c.xml, "Zig");
        write_files(&c.out, generators::zig::files(&model));
            fs::copy(c.fixture.join("test.zig"), c.out.join("test.zig")).unwrap();
        fs::write(c.out.join("index.html"), wasm_page("zig")).unwrap();
        print!("{}", skipped(&model.skipped));
        run(Command::new(zig).current_dir(&c.out).args(ZIG_WASM));
        write_glue(&c, &model);
        run_browser(&c);
    }
}

// File names must differ where letter case does not count, and avoid names
// Windows reserves.
#[test]
fn file_names_never_clash() {
    let xml = lower(&root().join("tests/fixtures/names/api.d.ts"), Some("c"), &[]);
    let files: Vec<String> = generators::c::files(&generators::wasm::model(&xml, "C")).into_iter().map(|f| f.0).collect();
    let mut lower: Vec<String> = files.iter().map(|f| f.to_lowercase()).collect();
    lower.sort();
    let count = lower.len();
    lower.dedup();
    assert_eq!(lower.len(), count, "clashing files: {files:?}");
    assert!(files.contains(&"areas/main/Point.h".to_string()) && files.contains(&"areas/main/point_2.h".to_string()), "{files:?}");
    assert!(files.contains(&"areas/main/Con_.h".to_string()), "{files:?}");
}

// Module names come from the config: clashes there are errors to fix.
#[test]
fn module_names_are_checked() {
    let dir = root().join("target/e2e/config-checks");
    fs::create_dir_all(&dir).unwrap();
    for (config, error) in [
        ("[modules]\n[[modules.area]]\nname = \"Svg\"\nmatch = [\"A*\"]\n[[modules.area]]\nname = \"svg\"\nmatch = [\"B*\"]\n", "differ only in letter case"),
        ("[modules]\ndefault = \"nul\"\n", "Windows reserves"),
    ] {
        let path = dir.join("modules.toml");
        fs::write(&path, config).unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_tsrs_proto")).arg(root().join("tests/fixtures/names/api.d.ts")).arg("--config").arg(&path).output().unwrap();
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(!out.status.success() && stderr.contains(error), "expected `{error}`, got: {stderr}");
    }
}
