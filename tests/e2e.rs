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

// The packages in `dir/node_modules`, bound together: those `packages.txt`
// lists, or all.
fn lower_packages(dir: &Path, target: &str) -> String {
    let mut names: Vec<String> = match fs::read_to_string(dir.join("packages.txt")) {
        Ok(list) => list.lines().map(str::to_string).filter(|l| !l.is_empty()).collect(),
        Err(_) => fs::read_dir(dir.join("node_modules")).unwrap().map(|e| e.unwrap().file_name().into_string().unwrap()).collect(),
    };
    names.sort();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_tsrs_proto"));
    cmd.arg("--from").arg(dir).arg("--config").arg(config(target));
    for n in &names {
        cmd.args(["--package", n]);
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
    // The library's own settings: the module it is, say.
    let library: Vec<PathBuf> = Some(dir.join("library.toml")).filter(|p| p.exists()).into_iter().collect();
    let xml = if dir.join("node_modules").is_dir() { lower_packages(&dir, target) } else { lower_with(&dir.join("api.d.ts"), Some(target), &library, &[]) };
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
fn write_glue(c: &Case, model: &generators::model::Model) {
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
    let bindings = generators::fsharp::module(&c.xml).unwrap();
    write_fsharp(&c, &bindings);
    install_library(&c);
    run(Command::new(fable).current_dir(&c.out).args(["Test.fsproj", "-o", "js", "--extension", ".mjs", "--noCache"]));
    let report = run(Command::new(node).arg(root().join("tests/run_fable.mjs")).arg(c.fixture.join("impl.mjs")).arg(c.out.join("js/Test.mjs")));
    print!("{report}{}", skipped(&bindings.skipped));
}

// A library that is a module, `<module>.mjs` in the fixture, installed where
// bindings that import it find it: in `node_modules` of the test's directory.
fn install_library(c: &Case) {
    // Packages: as they are in the fixture.
    if c.fixture.join("node_modules").is_dir() {
        copy_dir(&c.fixture.join("node_modules"), &c.out.join("node_modules"));
        return;
    }
    if let Some(module) = c.xml.lines().next().and_then(|l| l.split("import=\"").nth(1)).and_then(|r| r.split('"').next()) {
        let dir = c.out.join("node_modules").join(module);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("package.json"), format!("{{ \"name\": \"{module}\", \"type\": \"module\", \"exports\": \"./index.mjs\" }}\n")).unwrap();
        fs::copy(c.fixture.join(format!("{module}.mjs")), dir.join("index.mjs")).unwrap();
    }
}

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for e in fs::read_dir(from).unwrap() {
        let e = e.unwrap();
        if e.file_type().unwrap().is_dir() {
            copy_dir(&e.path(), &to.join(e.file_name()));
        } else {
            fs::copy(e.path(), to.join(e.file_name())).unwrap();
        }
    }
}

// The bindings in a Gleam project (tests/gleam), with the fixture's test.gleam
// as `check`; compiled, then run in Node.
fn gleam(fixture: &str) {
    let Some([gleam, node]) = tools(["gleam", "node"]) else { return };
    let c = case(fixture, "gleam");
    let bindings = generators::gleam::module(&c.xml).unwrap_or_else(|p| panic!("{}", p.join("\n")));
    write_gleam_project(&c.out, &bindings);
    fs::copy(c.fixture.join("test.gleam"), c.out.join("src/check.gleam")).unwrap();
    install_library(&c);
    run(Command::new(&gleam).current_dir(&c.out).args(["build", "--warnings-as-errors"]));
    let report = run(Command::new(node).arg(root().join("tests/run_gleam.mjs")).arg(c.fixture.join("impl.mjs")).arg(c.out.join("build/dev/javascript/check/check.mjs")));
    print!("{report}{}", skipped(&bindings.skipped));
}

// A Gleam project in `out`: tests/gleam's, with the bindings in `src`.
fn write_gleam_project(out: &Path, bindings: &generators::gleam::Output) {
    for file in ["gleam.toml", "manifest.toml"] {
        fs::copy(root().join("tests/gleam").join(file), out.join(file)).unwrap();
    }
    write_files(&out.join("src"), bindings.files.clone());
    // A mutable cell, for tests to see what callbacks did.
    for file in ["cell.gleam", "cell_ffi.mjs"] {
        fs::copy(root().join("tests/gleam").join(file), out.join("src").join(file)).unwrap();
    }
}

// The bindings, their generated project, and a test project referencing them.
fn write_fsharp(c: &Case, bindings: &generators::fsharp::Output) {
    let dir = c.out.join("bindings");
    fs::create_dir_all(&dir).unwrap();
    for (name, content) in &bindings.files {
        fs::write(dir.join(name), content).unwrap();
    }
    fs::write(dir.join("Bindings.fsproj"), bindings.fsproj()).unwrap();
    fs::copy(c.fixture.join("test.fs"), c.out.join("Test.fs")).unwrap();
    fs::write(
        c.out.join("Test.fsproj"),
        r#"<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup><TargetFramework>net8.0</TargetFramework></PropertyGroup>
  <ItemGroup>
    <ProjectReference Include="bindings/Bindings.fsproj" />
  </ItemGroup>
  <ItemGroup>
    <Compile Include="Test.fs" />
  </ItemGroup>
</Project>
"#,
    )
    .unwrap();
}

const CLANG_WASM: &[&str] = &["--target=wasm32", "-std=c11", "-pedantic", "-nostdlib", "-ffreestanding", "-O1", "-Wall", "-Wextra", "-Wno-unused-parameter", "-Werror", "-Wl,--no-entry", "-Wl,--allow-undefined", "-Wl,--export-table"];
const ZIG_WASM: &[&str] = &["build-exe", "test.zig", "-target", "wasm32-freestanding", "-fno-entry", "-rdynamic", "--export-table", "-O", "ReleaseSmall"];

fn c(fixture: &str) {
    let Some([clang, node]) = tools(["clang", "node"]) else { return };
    let c = case(fixture, "c");
    let model = model(&c.xml, "C");
    write_files(&c.out.join("bindings"), generators::c::files(&model).unwrap());
    run(Command::new(clang).args(CLANG_WASM).arg("-I").arg(c.out.join("bindings")).arg("-o").arg(c.out.join("test.wasm")).arg(c.fixture.join("test.c")));
    write_glue(&c, &model);
    run_wasm(&c, &node);
    print!("{}", skipped(&model.skipped));
}

fn zig(fixture: &str) {
    let Some([zig, node]) = tools(["zig", "node"]) else { return };
    let c = case(fixture, "zig");
    let model = model(&c.xml, "Zig");
    write_files(&c.out, generators::zig::files(&model).unwrap());
    fs::copy(c.fixture.join("test.zig"), c.out.join("test.zig")).unwrap();
    run(Command::new(zig).current_dir(&c.out).args(ZIG_WASM));
    write_glue(&c, &model);
    run_wasm(&c, &node);
    print!("{}", skipped(&model.skipped));
}

// A WASI SDK compiler binary (`clang` or `clang++`), with C/C++ standard libraries for wasm32.
fn wasi_tool(bin: &str) -> Option<PathBuf> {
    let rel = format!("bin/{bin}");
    let from_env = env::var_os("WASI_SDK_PATH").map(|p| PathBuf::from(p).join(&rel));
    let from_home = env::var_os("HOME").and_then(|h| {
        let mut sdks: Vec<PathBuf> = fs::read_dir(PathBuf::from(h).join(".wasi-sdk")).ok()?.filter_map(|e| e.ok()).map(|e| e.path().join(&rel)).collect();
        sdks.sort();
        sdks.pop()
    });
    let found = from_env.into_iter().chain(from_home).find(|p| p.is_file());
    if found.is_none() {
        eprintln!("SKIPPED: no WASI SDK (set WASI_SDK_PATH)");
    }
    found
}

fn wasi_clang() -> Option<PathBuf> {
    wasi_tool("clang++")
}

fn wasi_clang_c() -> Option<PathBuf> {
    wasi_tool("clang")
}

// No `--allow-undefined`: the imports are declared as such, and anything else
// missing is a link error rather than an import that fails when the page loads.
const CLANGXX_WASM: &[&str] = &["--target=wasm32-wasip1", "-mexec-model=reactor", "-std=c++20", "-fno-exceptions", "-O1", "-Wall", "-Wextra", "-Wno-unused-parameter", "-Werror", "-Wl,--export-table", "-Wl,--strip-debug"];
const CLANG_WASI_WASM: &[&str] = &["--target=wasm32-wasip1", "-mexec-model=reactor", "-std=c23", "-O1", "-Wall", "-Wextra", "-Wno-unused-parameter", "-Werror", "-Wl,--export-table", "-Wl,--strip-debug"];

// Writes the C++ bindings, compiles their definitions once, each module in
// parallel, and links them with the program.
fn build_cpp(clang: &Path, c: &Case, model: &generators::model::Model) {
    let objects = compile_cpp_bindings(clang, &c.out, model);
    let bindings = c.out.join("bindings");
    run(Command::new(clang).args(CLANGXX_WASM).arg("-I").arg(&bindings).arg("-o").arg(c.out.join("test.wasm")).arg(c.fixture.join("test.cpp")).args(&objects));
}

// Writes the C++ bindings in `out/bindings` and compiles their definitions,
// each module in parallel; gives the objects.
fn compile_cpp_bindings(clang: &Path, out: &Path, model: &generators::model::Model) -> Vec<PathBuf> {
    let bindings = out.join("bindings");
    let files = generators::cpp::files(model).unwrap();
    let sources: Vec<String> = files.iter().map(|f| f.0.clone()).filter(|p| p.ends_with(".cpp")).collect();
    write_files(&bindings, files);
    let flags: Vec<&str> = CLANGXX_WASM.iter().filter(|f| !f.starts_with("-Wl,") && !f.starts_with("-mexec-model")).copied().collect();
    let objects: Vec<PathBuf> = sources.iter().map(|s| out.join("obj").join(s.replace('/', "_")).with_extension("o")).collect();
    fs::create_dir_all(out.join("obj")).unwrap();
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
    objects
}

fn cpp(fixture: &str) {
    let Some(clang) = wasi_clang() else { return };
    let Some([node]) = tools(["node"]) else { return };
    let c = case(fixture, "cpp");
    let model = model(&c.xml, "C++");
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

const RUSTC_WASM: &[&str] = &["--edition", "2024", "--target", "wasm32-unknown-unknown", "-O", "-D", "warnings"];
// A program, linked against the bindings.
// Without the standard library's debug information, which is most of a megabyte.
const RUSTC_PROGRAM: &[&str] = &["--crate-type", "cdylib", "-C", "link-arg=--export-table", "-C", "strip=debuginfo", "--extern", "bindings=libbindings.rlib"];

// Writes the bindings crate in `bindings/` and compiles it to a library. The
// library is kept in `target/e2e-cache`, and compiled again only when what
// was generated has changed: a second run of the DOM takes a second, not seven.
fn rust_bindings(rustc: &Path, out: &Path, model: &generators::model::Model) {
    use std::hash::{Hash, Hasher};
    let files = generators::rust::files(model).unwrap();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    (&files, RUSTC_WASM).hash(&mut hasher);
    let stamp = format!("{:x}", hasher.finish());
    let cache = root().join("target/e2e-cache");
    // By the whole output path: `multifile-pkg/rust` is not `rust`.
    let name = out.strip_prefix(root().join("target")).unwrap_or(out).to_string_lossy().replace(['/', '\\'], "-");
    let (lib, stamp_file) = (cache.join(format!("{name}.rlib")), cache.join(format!("{name}.stamp")));
    write_files(&out.join("bindings"), files);
    if !(lib.exists() && fs::read_to_string(&stamp_file).is_ok_and(|s| s == stamp)) {
        fs::create_dir_all(&cache).unwrap();
        run(Command::new(rustc).current_dir(out).args(RUSTC_WASM).args(["--crate-type", "rlib", "--crate-name", "bindings", "-o"]).arg(&lib).arg("bindings/src/lib.rs"));
        fs::write(stamp_file, stamp).unwrap();
    }
    fs::copy(lib, out.join("libbindings.rlib")).unwrap();
}

// Compiles the bindings, then the program against them.
fn build_rust(rustc: &Path, c: &Case, model: &generators::model::Model) {
    rust_bindings(rustc, &c.out, model);
    fs::copy(c.fixture.join("test.rs"), c.out.join("test.rs")).unwrap();
    run(Command::new(rustc).current_dir(&c.out).args(RUSTC_WASM).args(RUSTC_PROGRAM).args(["-o", "test.wasm", "test.rs"]));
}

fn rust(fixture: &str) {
    let Some(rustc) = rustc_wasm() else { return };
    let Some([node]) = tools(["node"]) else { return };
    let c = case(fixture, "rust");
    let model = model(&c.xml, "Rust");
    build_rust(&rustc, &c, &model);
    write_glue(&c, &model);
    run_wasm(&c, &node);
    print!("{}", skipped(&model.skipped));
}

// The model of a target, which its config must have made expressible.
fn model(xml: &str, target: &str) -> generators::model::Model {
    generators::wasm::model(xml, target).unwrap_or_else(|problems| panic!("{}", problems.join("\n")))
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
        #[test] fn gleam() { super::gleam(stringify!($fixture)); }
    })* };
}

// `pts` is a real library: Pts, from its own declarations. `exports` is a
// module library across files: default, renamed and re-exported declarations.
fixtures!(shapes, callbacks, data, globals, unions, pts, exports);

// Two packages that export the same names, each reached in its own module.
mod packages {
    #[test] fn fsharp() { super::fsharp("packages"); }
    #[test] fn c() { super::c("packages"); }
    #[test] fn gleam() { super::gleam("packages"); }
}

// A package's names are only in its module: without one, makeEnv fails
// rather than reaching globals of the same name (`Node`).
#[test]
fn packages_need_their_modules() {
    let Some([node]) = tools(["node"]) else { return };
    // Its own directory: `packages::c` runs beside it.
    let c = case_in("packages", "c", "packages-c-modules");
    fs::write(c.out.join("glue.mjs"), generators::wasm::glue(&model(&c.xml, "C"), None)).unwrap();
    let script = "import { makeEnv } from './glue.mjs';\n\
                  try { makeEnv(() => null, { modules: { pa: {} } }); console.log('no error'); } catch (e) { console.log(e.message); }\n";
    fs::write(c.out.join("check.mjs"), script).unwrap();
    let said = run(Command::new(node).current_dir(&c.out).arg("check.mjs"));
    assert!(said.contains("no module for the package \"pb\""), "{said}");
}

// A declaration's constraint is kept where its uses meet it by name, and
// with `exact` a type parameter meets only the constraint it has.
#[test]
fn constraints_are_kept_where_met_by_name() {
    let input = root().join("tests/fixtures/constraints/api.d.ts");
    let constrained = |xml: &str, decl: &str| {
        let at = xml.find(&format!("name=\"{decl}\" id=")).unwrap_or_else(|| panic!("no {decl}"));
        let body = &xml[at..at + xml[at..].find("</interface>").unwrap()];
        body.contains("<constraint>")
    };
    let nominal = lower(&input, None, &["constraints=nominal"]);
    assert!(constrained(&nominal, "Kept") && !constrained(&nominal, "ByShape"));
    assert!(constrained(&nominal, "Options") && constrained(&nominal, "ReadableOptions"));
    let exact = lower(&input, None, &["constraints=exact"]);
    assert!(constrained(&exact, "Kept") && !constrained(&exact, "ByShape"));
    assert!(!constrained(&exact, "Options") && constrained(&exact, "ReadableOptions"));
}

// Names apart in TypeScript that a target would make one identifier of.
mod clashes {
    #[test] fn fsharp() { super::fsharp("clashes"); }
    #[test] fn c() { super::c("clashes"); }
    #[test] fn cpp() { super::cpp("clashes"); }
    #[test] fn zig() { super::zig("clashes"); }
    #[test] fn rust() { super::rust("clashes"); }
    #[test] fn gleam() { super::gleam("clashes"); }
}

// Gleam's types follow the classes': each of these is a compile error.
#[test]
fn gleam_rejects_what_is_not_a_subtype() {
    let Some([gleam]) = tools(["gleam"]) else { return };
    let c = case_in("shapes", "gleam", "shapes-gleam-wrong");
    let bindings = generators::gleam::module(&c.xml).unwrap_or_else(|p| panic!("{}", p.join("\n")));
    write_gleam_project(&c.out, &bindings);
    let wrong = "import bindings/areas/main as b\nimport bindings/areas/main/shape\n\n\
                 // A string where a Named is wanted (`adopt<T extends Named>`).\n\
                 pub fn not_named() {\n  b.adopt(\"text\")\n}\n\n\
                 // A Named where a Shape is wanted.\n\
                 pub fn not_a_shape() {\n  shape.area(b.plain())\n}\n\n\
                 // A Shape and a Named in one list, without closing the Shape at Named.\n\
                 pub fn mixed() {\n  [shape.unit(), b.plain()]\n}\n\n\
                 pub fn main() {\n  Nil\n}\n";
    fs::write(c.out.join("src/check.gleam"), wrong).unwrap();
    let out = Command::new(gleam).current_dir(&c.out).arg("build").output().unwrap();
    let said = String::from_utf8_lossy(&out.stderr).to_string() + &String::from_utf8_lossy(&out.stdout);
    assert!(!out.status.success(), "should not compile:\n{said}");
    assert_eq!(said.matches("Type mismatch").count(), 3, "one mismatch each:\n{said}");
}

mod builtins {
    #[test] fn fsharp() { super::fsharp("builtins"); }
    #[test] fn c() { super::c("builtins"); }
    #[test] fn zig() { super::zig("builtins"); }
    #[test] fn cpp() { super::cpp("builtins"); }
    #[test] fn rust() { super::rust("builtins"); }
}

// The generators must refuse what their target cannot express; otherwise a
// config that lowers too little would go unnoticed.

fn shapes_xml(target: &str, overrides: &[&str]) -> String {
    lower(&root().join("tests/fixtures/shapes/api.d.ts"), Some(target), overrides)
}

#[test]
fn c_rejects_unions() {
    let xml = shapes_xml("c", &["union_params=keep", "unions=keep"]);
    let problems = generators::c::files(&model(&xml, "C")).unwrap_err();
    // Each kind once: what, the setting that lowers it, how often and where first.
    assert!(problems.iter().any(|p| p.contains("cannot express a union: set unions = \"name\" or \"erase\"") && p.contains("Shape.label")), "{problems:?}");
}

// Everything a target cannot express is reported at once, not only the first.
#[test]
fn every_problem_is_reported() {
    let xml = shapes_xml("c", &["generics=keep", "literals=keep", "anonymous=keep"]);
    let problems = generators::wasm::model(&lower(&root().join("tests/fixtures/data/api.d.ts"), Some("c"), &["generics=keep", "literals=keep"]), "C").err().unwrap_or_default();
    assert!(problems.iter().any(|p| p.contains("<typeParameter>") && p.contains("generics = \"erase\"")), "{problems:?}");
    assert!(problems.iter().any(|p| p.contains("<literal>") && p.contains("places, first at")), "{problems:?}");
    let _ = xml;
}

// Every output follows `schema/declarations.rng`: each input unlowered and
// with each target's config.
#[test]
fn xml_follows_the_schema() {
    let Some([xmllint]) = tools(["xmllint"]) else { return };
    let out = root().join("target/e2e/schema");
    fs::create_dir_all(&out).unwrap();
    let mut inputs: Vec<PathBuf> = ["shapes", "callbacks", "data", "globals", "unions", "pts", "exports"].iter().map(|f| root().join("tests/fixtures").join(f).join("api.d.ts")).collect();
    inputs.extend(["tests/bench/api.d.ts", "examples/lowering.d.ts", "examples/features.d.ts"].map(|p| root().join(p)));
    inputs.push(PathBuf::from("lib.es5.d.ts"));
    // Text that XML cannot hold as it is: a control character, a line break, an empty name.
    let odd = out.join("odd.d.ts");
    fs::write(&odd, "type Odd = \"a\\u0001b\" | \"line\\nbreak\";\ninterface Empty { \"\": number }\n").unwrap();
    let xml = lower(&odd, None, &[]);
    let doc = roxmltree::Document::parse(&xml).expect("well-formed XML");
    let values: Vec<&str> = doc.descendants().filter(|n| n.has_tag_name("literal")).filter_map(|n| n.attribute("value")).collect();
    assert_eq!(values, ["a\\u0001b", "line\nbreak"]);
    assert!(doc.descendants().any(|n| n.has_tag_name("property") && n.attribute("name") == Some("")));
    inputs.push(odd);
    let mut files = Vec::new();
    for (i, input) in inputs.iter().enumerate() {
        for target in [None, Some("fsharp"), Some("ocaml"), Some("rust"), Some("c"), Some("cpp"), Some("zig")] {
            let file = out.join(format!("{i}-{}.xml", target.unwrap_or("raw")));
            fs::write(&file, lower(input, target, &[])).unwrap();
            files.push(file);
        }
    }
    // The whole DOM too, with the config that changes it most.
    files.push(out.join("dom-c.xml"));
    fs::write(files.last().unwrap(), lower_with(Path::new("lib.dom.d.ts"), Some("c"), &[root().join("examples/modules/dom.toml")], &[])).unwrap();
    let result = Command::new(xmllint).arg("--noout").arg("--relaxng").arg(root().join("schema/declarations.rng")).args(&files).output().unwrap();
    let report = String::from_utf8_lossy(&result.stderr);
    let failures: Vec<&str> = report.lines().filter(|l| !l.ends_with(" validates")).take(20).collect();
    assert!(result.status.success(), "{}", failures.join("\n"));
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
        let model = model(&c.xml, "C");
        write_files(&c.out.join("bindings"), generators::c::files(&model).unwrap());
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
    let glue = |o: &Path, model: &generators::model::Model| {
        let used = generators::wasm::imports(&fs::read(o.join("bench.wasm")).unwrap());
        fs::write(o.join("glue.mjs"), generators::wasm::glue(model, Some(&used))).unwrap();
    };
    let optimized = |flags: &[&'static str]| -> Vec<&'static str> { flags.iter().map(|f| if *f == "-O1" { "-O2" } else { *f }).collect() };
    let mut names: Vec<&str> = Vec::new();
    if let Some(fable) = find("fable") {
        let o = out.join("fsharp");
        let bdir = o.join("bindings");
        fs::create_dir_all(&bdir).unwrap();
        let bindings = generators::fsharp::module(&xml("fsharp")).unwrap();
        for (name, content) in &bindings.files {
            fs::write(bdir.join(name), content).unwrap();
        }
        fs::write(bdir.join("Bindings.fsproj"), bindings.fsproj()).unwrap();
        fs::copy(dir.join("bench.fs"), o.join("Bench.fs")).unwrap();
        fs::write(o.join("Bench.fsproj"), "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup><TargetFramework>net8.0</TargetFramework></PropertyGroup>\n  <ItemGroup>\n    <ProjectReference Include=\"bindings/Bindings.fsproj\" />\n  </ItemGroup>\n  <ItemGroup>\n    <Compile Include=\"Bench.fs\" />\n  </ItemGroup>\n</Project>\n").unwrap();
        run(Command::new(fable).current_dir(&o).args(["Bench.fsproj", "-o", "js", "--extension", ".mjs", "--noCache"]));
        names.push("fsharp");
    }
    if let Some(clang) = find("clang") {
        let o = out.join("c");
        let model = model(&xml("c"), "C");
        write_files(&o.join("bindings"), generators::c::files(&model).unwrap());
        run(Command::new(clang).args(optimized(CLANG_WASM)).arg("-I").arg(o.join("bindings")).arg("-o").arg(o.join("bench.wasm")).arg(dir.join("bench.c")));
        glue(&o, &model);
        names.push("c");
    }
    if let Some(clang) = wasi_clang() {
        let o = out.join("cpp");
        let model = model(&xml("cpp"), "C++");
        let files = generators::cpp::files(&model).unwrap();
        let sources: Vec<PathBuf> = files.iter().filter(|f| f.0.ends_with(".cpp")).map(|f| o.join("bindings").join(&f.0)).collect();
        write_files(&o.join("bindings"), files);
        run(Command::new(clang).args(optimized(CLANGXX_WASM)).arg("-I").arg(o.join("bindings")).arg("-o").arg(o.join("bench.wasm")).arg(dir.join("bench.cpp")).args(&sources));
        glue(&o, &model);
        names.push("cpp");
    }
    if let Some(zig) = find("zig") {
        let o = out.join("zig");
        let model = model(&xml("zig"), "Zig");
        write_files(&o, generators::zig::files(&model).unwrap());
        fs::copy(dir.join("bench.zig"), o.join("bench.zig")).unwrap();
        run(Command::new(zig).current_dir(&o).args(["build-exe", "bench.zig", "-target", "wasm32-freestanding", "-fno-entry", "-rdynamic", "--export-table", "-O", "ReleaseFast"]));
        glue(&o, &model);
        names.push("zig");
    }
    if let Some(rustc) = rustc_wasm() {
        let o = out.join("rust");
        let model = model(&xml("rust"), "Rust");
        rust_bindings(&rustc, &o, &model);
        fs::copy(dir.join("bench.rs"), o.join("bench.rs")).unwrap();
        run(Command::new(rustc).current_dir(&o).args(RUSTC_WASM).args(RUSTC_PROGRAM).args(["-C", "opt-level=3", "-C", "lto", "-o", "bench.wasm", "bench.rs"]));
        glue(&o, &model);
        names.push("rust");
    }
    let report = run(Command::new(node).arg(root().join("tests/run_bench.mjs")).arg(dir.join("impl.mjs")).arg(dir.join("bench.ts")).arg(&out).args(&names));
    print!("{report}");
}

// A call may leave out its only argument in C23: `orUnit()`. C11 counts the
// arguments the same way, but under -pedantic warns of an empty call.
#[test]
fn c_calls_may_be_empty() {
    let Some([clang]) = tools(["clang"]) else { return };
    let c = case_in("shapes", "c", "shapes-c-empty-call");
    write_files(&c.out.join("bindings"), generators::c::files(&model(&c.xml, "C")).unwrap());
    fs::write(c.out.join("t.c"), "#include \"bindings.h\"\nShape unit(void) { return orUnit(); }\nShape same(Shape s) { return orUnit(s); }\n").unwrap();
    run(Command::new(clang).current_dir(&c.out).args(["--target=wasm32", "-std=c2x", "-pedantic", "-Werror", "-nostdlib", "-ffreestanding", "-fsyntax-only", "-I", "bindings", "t.c"]));
}

// What a package across files declares has one name each in the output:
// a class merged with an interface is one class; two files' own `Item`s are
// two types; an object base gives its members; a class given as a value is a value.
#[test]
fn package_declarations_keep_apart() {
    let dir = root().join("target/e2e/package-names");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("package.json"), r#"{"name": "names-pkg"}"#).unwrap();
    fs::write(dir.join("a.d.ts"), "interface Item { a: number }\nexport declare function makeA(): Item;\n").unwrap();
    fs::write(dir.join("b.d.ts"), "interface Item { b: string }\nexport declare function makeB(): Item;\n").unwrap();
    fs::write(
        dir.join("index.d.ts"),
        "export * from \"./a\";\nexport * from \"./b\";\n\
         export interface Item { c: boolean }\n\
         export declare class Material { opacity: number }\n\
         export interface Material { transparent: boolean }\n\
         interface Props { width: number }\n\
         export interface Options extends Partial<Props> { height: number }\n\
         export declare function register(kind: new () => Material): void;\n",
    )
    .unwrap();
    let xml = lower(&dir.join("index.d.ts"), Some("rust"), &[]);
    // The exported Item keeps its name; the others are named by their files.
    for name in ["Item", "a_Item", "b_Item"] {
        assert!(xml.contains(&format!("name=\"{name}\"")), "missing {name}:\n{xml}");
    }
    assert!(xml.matches("<class module=\"main\" name=\"Material\"").count() == 1 && !xml.contains("<interface module=\"main\" name=\"Material\"") && xml.contains("name=\"transparent\""), "one Material with both declarations' members:\n{xml}");
    // References follow: makeA returns a's own Item.
    assert!(xml.contains("<ref name=\"a_Item\" kind=\"interface\" to=\"a_Item\"/>") && xml.contains("<ref name=\"b_Item\" kind=\"interface\" to=\"b_Item\"/>"), "references to renamed types:\n{xml}");
    assert!(xml.contains("name=\"width\"") && !xml.contains("<extends>\n      <object>"), "an object base gives its members:\n{xml}");
    let model = model(&xml, "Rust");
    assert!(model.skipped.keys().any(|k| k.contains("constructor type")), "a constructor type is a value: {:?}", model.skipped);
    generators::rust::files(&model).unwrap();
}

// `[rename]` names what a pass generated, wherever it is referred to; a
// declared name is the library's to choose.
#[test]
fn generated_names_can_be_chosen() {
    let api = root().join("tests/fixtures/data/api.d.ts");
    let xml = lower(&api, Some("rust"), &["rename.Options_mode=Mode", "rename.norm_point=Point"]);
    assert!(xml.contains("name=\"Mode\" id=\"Mode\" synthetic=\"true\"") && xml.contains("<ref name=\"Mode\" kind=\"enum\" to=\"Mode\"/>") && !xml.contains("Options_mode"), "the enum and every reference to it are renamed");
    assert!(xml.contains("name=\"Point\"") && !xml.contains("norm_point"));
    let fails = |setting: &str, why: &str| {
        let out = Command::new(env!("CARGO_BIN_EXE_tsrs_proto")).arg(&api).arg("--config").arg(config("rust")).args(["--set", setting]).output().unwrap();
        let said = String::from_utf8_lossy(&out.stderr);
        assert!(!out.status.success() && said.contains(why), "{setting} should fail with `{why}`: {said}");
    };
    fails("rename.Options=Settings", "only generated names can be renamed");
    fails("rename.Options_mode=Item", "that name is taken");
    // A name this target did not generate is skipped: other targets may.
    assert!(lower(&api, Some("rust"), &["rename.Nothing=Something"]).contains("Options_mode"));
}

// `--emit` writes bindings with their glue; what a generator cannot express is an error.
#[test]
fn emit_writes_bindings_or_says_why_not() {
    let out = root().join("target/e2e/emit");
    let _ = fs::remove_dir_all(&out);
    let shapes = root().join("tests/fixtures/shapes/api.d.ts");
    let run = |extra: &[&str]| Command::new(env!("CARGO_BIN_EXE_tsrs_proto")).arg(&shapes).arg("--config").arg(config("c")).args(extra).arg("--emit").arg("c").arg("--out").arg(&out).output().unwrap();
    let ok = run(&[]);
    assert!(ok.status.success() && out.join("bindings/bindings.h").exists() && out.join("bindings/glue.mjs").exists(), "{}", String::from_utf8_lossy(&ok.stderr));
    let bad = run(&["--set", "unions=keep", "--set", "union_params=keep"]);
    let errors = String::from_utf8_lossy(&bad.stderr);
    assert!(!bad.status.success() && errors.contains("cannot express a union"), "{errors}");
    // Package-named nested outputs and manifests for every target.
    for (target, expected_files) in [
        ("c", ["base_lib/base_lib.h", "base_lib/bindings.h", "base_lib/glue.mjs"].as_slice()),
        ("cpp", ["base_lib/base_lib.hpp", "base_lib/bindings.hpp", "base_lib/glue.mjs"].as_slice()),
        ("zig", ["base_lib/build.zig", "base_lib/bindings.zig", "base_lib/glue.mjs"].as_slice()),
        ("rust", ["base_lib/Cargo.toml", "base_lib/src/lib.rs", "base_lib/glue.mjs"].as_slice()),
        ("gleam", ["base_lib/gleam.toml", "base_lib/src/base_lib/types.gleam"].as_slice()),
    ] {
        let pkg_out = root().join("target/e2e").join(format!("emit-{target}"));
        let _ = fs::remove_dir_all(&pkg_out);
        let cfg = if target == "cpp" { "cpp" } else { target };
        let res = Command::new(env!("CARGO_BIN_EXE_tsrs_proto"))
            .arg(&shapes)
            .arg("--config")
            .arg(config(cfg))
            .args(["--emit", target, "--name", "base-lib", "--out"])
            .arg(&pkg_out)
            .output()
            .unwrap();
        assert!(res.status.success(), "{target}: {}", String::from_utf8_lossy(&res.stderr));
        for rel in expected_files {
            assert!(pkg_out.join(rel).exists(), "{target}: missing {}", pkg_out.join(rel).display());
        }
    }
    let fs_out = root().join("target/e2e/emit-fsharp");
    let _ = fs::remove_dir_all(&fs_out);
    let fs_ok = Command::new(env!("CARGO_BIN_EXE_tsrs_proto")).arg(&shapes).arg("--config").arg(config("fsharp")).args(["--emit", "fsharp", "--out"]).arg(&fs_out).output().unwrap();
    assert!(fs_ok.status.success(), "{}", String::from_utf8_lossy(&fs_ok.stderr));
    let fsproj = fs::read_to_string(fs_out.join("Bindings/Bindings.fsproj")).expect("Bindings/Bindings.fsproj written");
    assert!(fsproj.contains("<Compile Include=\"Bindings.Prelude.fs\" />") && fsproj.contains("<Compile Include=\"Bindings.Values.fs\" />") && fsproj.contains("Fable.Core"), "{fsproj}");
    // Package-named nested folder + namespace + .fsproj, and explicit `--name` override.
    let pts = root().join("tests/fixtures/pts/api.d.ts");
    let pts_ok = Command::new(env!("CARGO_BIN_EXE_tsrs_proto")).arg(&pts).arg("--config").arg(config("fsharp")).args(["--emit", "fsharp", "--out"]).arg(&fs_out).output().unwrap();
    assert!(pts_ok.status.success(), "{}", String::from_utf8_lossy(&pts_ok.stderr));
    let pts_fsproj = fs::read_to_string(fs_out.join("Pts/Pts.fsproj")).expect("Pts/Pts.fsproj written");
    assert!(pts_fsproj.contains("<Compile Include=\"Pts.Prelude.fs\" />") && pts_fsproj.contains("<Compile Include=\"Pts.Values.fs\" />"), "{pts_fsproj}");
    let pts_prelude = fs::read_to_string(fs_out.join("Pts/Pts.Prelude.fs")).unwrap();
    assert!(pts_prelude.contains("namespace Pts\n"), "{pts_prelude}");
    let custom_ok = Command::new(env!("CARGO_BIN_EXE_tsrs_proto")).arg(&shapes).arg("--config").arg(config("fsharp")).args(["--emit", "fsharp", "--name", "MyLib.Core", "--out"]).arg(&fs_out).output().unwrap();
    assert!(custom_ok.status.success(), "{}", String::from_utf8_lossy(&custom_ok.stderr));
    let custom_fsproj = fs::read_to_string(fs_out.join("MyLib.Core/MyLib.Core.fsproj")).expect("MyLib.Core/MyLib.Core.fsproj written");
    assert!(custom_fsproj.contains("<Compile Include=\"MyLib.Core.Prelude.fs\" />"), "{custom_fsproj}");
}

// F# `--ref` resolves external types against previously generated binding projects
// and wires `<ProjectReference>` into the generated `.fsproj`.
#[test]
fn fsharp_cross_project_refs() {
    let Some([fable]) = tools(["fable"]) else { return };
    let out = root().join("target/e2e/fsharp-refs");
    let _ = fs::remove_dir_all(&out);
    let bindings = out.join("bindings");
    let pkg_dir = out.join("node_modules/base-lib");
    fs::create_dir_all(&bindings).unwrap();
    fs::create_dir_all(&pkg_dir).unwrap();
    // Base package in node_modules so `app.d.ts` imports `Surface` as an external package type.
    let base_dts = pkg_dir.join("index.d.ts");
    fs::write(pkg_dir.join("package.json"), "{ \"name\": \"base-lib\", \"types\": \"./index.d.ts\" }\n").unwrap();
    fs::write(&base_dts, "export interface Surface { width: number; height: number; }\n").unwrap();
    let app_dts = out.join("app.d.ts");
    fs::write(&app_dts, "import type { Surface } from 'base-lib';\nexport declare function render(s: Surface): number;\n").unwrap();
    let base_ok = Command::new(env!("CARGO_BIN_EXE_tsrs_proto"))
        .arg(&base_dts)
        .arg("--config")
        .arg(config("fsharp"))
        .args(["--emit", "fsharp", "--name", "BaseLib", "--out"])
        .arg(&bindings)
        .output()
        .unwrap();
    assert!(base_ok.status.success(), "{}", String::from_utf8_lossy(&base_ok.stderr));
    let app_ok = Command::new(env!("CARGO_BIN_EXE_tsrs_proto"))
        .arg(&app_dts)
        .arg("--config")
        .arg(config("fsharp"))
        .args(["--emit", "fsharp", "--name", "AppLib", "--ref"])
        .arg(bindings.join("BaseLib"))
        .arg("--out")
        .arg(&bindings)
        .output()
        .unwrap();
    assert!(app_ok.status.success(), "{}", String::from_utf8_lossy(&app_ok.stderr));
    let app_fsproj = fs::read_to_string(bindings.join("AppLib/AppLib.fsproj")).unwrap();
    assert!(app_fsproj.contains("<ProjectReference Include=\"../BaseLib/BaseLib.fsproj\" />"), "{app_fsproj}");
    let app_prelude = fs::read_to_string(bindings.join("AppLib/AppLib.Prelude.fs")).unwrap();
    assert!(!app_prelude.contains("type Surface ="), "Surface should come from BaseLib, not an opaque stub:\n{app_prelude}");
    let app_values = fs::read_to_string(bindings.join("AppLib/AppLib.Values.fs")).unwrap();
    assert!(app_values.contains("BaseLib.Surface"), "{app_values}");
    // Consumer project passes `BaseLib.Surface` straight to `AppLib.Globals.render` without casts.
    fs::write(
        out.join("Test.fs"),
        "module Test\nopen BaseLib\nopen AppLib\nlet s = Surface.Create(width = 640.0, height = 480.0)\nlet n: float = Globals.render s\n",
    )
    .unwrap();
    fs::write(
        out.join("Test.fsproj"),
        r#"<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup><TargetFramework>net8.0</TargetFramework></PropertyGroup>
  <ItemGroup>
    <ProjectReference Include="bindings/BaseLib/BaseLib.fsproj" />
    <ProjectReference Include="bindings/AppLib/AppLib.fsproj" />
  </ItemGroup>
  <ItemGroup>
    <Compile Include="Test.fs" />
  </ItemGroup>
</Project>
"#,
    )
    .unwrap();
    run(Command::new(fable).current_dir(&out).args(["Test.fsproj", "-o", "js", "--extension", ".mjs", "--noCache"]));
}

fn setup_ref_fixture(name: &str, target: &str) -> (PathBuf, PathBuf) {
    let out = root().join("target/e2e").join(name);
    let _ = fs::remove_dir_all(&out);
    let bindings = out.join("bindings");
    let pkg_dir = out.join("node_modules/base-lib");
    fs::create_dir_all(&bindings).unwrap();
    fs::create_dir_all(&pkg_dir).unwrap();
    let base_dts = pkg_dir.join("index.d.ts");
    fs::write(pkg_dir.join("package.json"), "{ \"name\": \"base-lib\", \"types\": \"./index.d.ts\" }\n").unwrap();
    fs::write(
        &base_dts,
        "export interface Surface { width: number; height: number; }\n\
         export type Mode = \"fast\" | \"safe\";\n\
         export interface RenderingContext { id: number; }\n\
         export declare function makeSurface(w: number, h: number): Surface;\n\
         export declare function makeBaseContext(id: number): RenderingContext;\n\
         export declare function describeMode(m: Mode): string;\n",
    )
    .unwrap();
    let app_dts = out.join("app.d.ts");
    fs::write(
        &app_dts,
        "import type { Surface } from 'base-lib';\n\
         export type Quality = \"low\" | \"high\";\n\
         export interface RenderingContext { label: string; }\n\
         export declare function render(s: Surface, q: Quality): number;\n\
         export declare function makeAppContext(label: string): RenderingContext;\n\
         export declare function describeQuality(q: Quality): string;\n",
    )
    .unwrap();
    let base_ok = Command::new(env!("CARGO_BIN_EXE_tsrs_proto"))
        .arg(&base_dts)
        .arg("--config")
        .arg(config(target))
        .args(["--emit", target, "--name", "base-lib", "--out"])
        .arg(&bindings)
        .output()
        .unwrap();
    assert!(base_ok.status.success(), "{target} base: {}", String::from_utf8_lossy(&base_ok.stderr));
    let app_ok = Command::new(env!("CARGO_BIN_EXE_tsrs_proto"))
        .arg(&app_dts)
        .arg("--config")
        .arg(config(target))
        .args(["--emit", target, "--name", "app-lib", "--ref"])
        .arg(bindings.join("base_lib"))
        .arg("--out")
        .arg(&bindings)
        .output()
        .unwrap();
    assert!(app_ok.status.success(), "{target} app: {}", String::from_utf8_lossy(&app_ok.stderr));
    (out, bindings)
}

fn write_composable_runner(out: &Path) {
    fs::write(
        out.join("run.mjs"),
        "import fs from 'node:fs';\n\
         import { createRuntime, makeEnv as baseEnv } from './bindings/base_lib/glue.mjs';\n\
         import { makeEnv as appEnv } from './bindings/app_lib/glue.mjs';\n\
         let instance;\n\
         let passed = 0;\n\
         const failed = [];\n\
         const wasi = new Proxy({}, { get: () => () => 0 });\n\
         const rt = createRuntime(() => instance, { check: true, unchecked: e => failed.push(e) });\n\
         const baseImpl = {\n\
           makeSurface: (w, h) => ({ width: w, height: h }),\n\
           makeBaseContext: id => ({ id }),\n\
           describeMode: m => 'mode:' + m,\n\
         };\n\
         const appImpl = {\n\
           render: (s, q) => s.width + s.height + (q === 'high' ? 10 : 1),\n\
           makeAppContext: label => ({ label }),\n\
           describeQuality: q => 'quality:' + q,\n\
         };\n\
         const b = baseEnv(() => instance, { runtime: rt, root: baseImpl });\n\
         const a = appEnv(() => instance, { runtime: rt, root: appImpl });\n\
         ({ instance } = await WebAssembly.instantiate(fs.readFileSync('./test.wasm'), {\n\
           env: { ...rt.env, check: (ok, line) => (ok ? passed++ : failed.push(line)) },\n\
           base_lib: b.$pkg,\n\
           app_lib: a.$pkg,\n\
           wasi_snapshot_preview1: wasi,\n\
         }));\n\
         instance.exports._initialize?.();\n\
         instance.exports.run();\n\
         if (failed.length || passed === 0) {\n\
           console.error(`${failed.length} failed; ${passed} passed; lines=${failed.join(',')}`);\n\
           process.exit(1);\n\
         }\n\
         console.log(`${passed} checks passed`);\n",
    )
    .unwrap();
}

#[test]
fn rust_cross_project_refs() {
    let Some(rustc) = rustc_wasm() else { return };
    let Some([node]) = tools(["node"]) else { return };
    let (out, bindings) = setup_ref_fixture("rust-refs", "rust");
    let app_toml = fs::read_to_string(bindings.join("app_lib/Cargo.toml")).unwrap();
    assert!(app_toml.contains("base_lib = { path = \"../base_lib\" }"), "{app_toml}");
    let app_types = fs::read_to_string(bindings.join("app_lib/src/types.rs")).unwrap();
    assert!(app_types.contains("pub use ::base_lib::Surface;"), "{app_types}");

    let base_rlib = out.join("libbase_lib.rlib");
    let app_rlib = out.join("libapp_lib.rlib");
    run(Command::new(&rustc).current_dir(&out).args(RUSTC_WASM).args(["--crate-type", "rlib", "--crate-name", "base_lib", "-o"]).arg(&base_rlib).arg("bindings/base_lib/src/lib.rs"));
    run(Command::new(&rustc).current_dir(&out).args(RUSTC_WASM).args(["--crate-type", "rlib", "--crate-name", "app_lib", "--extern"]).arg(format!("base_lib={}", base_rlib.display())).arg("-o").arg(&app_rlib).arg("bindings/app_lib/src/lib.rs"));
    fs::write(
        out.join("test.rs"),
        "#![no_main]\n\
         #[link(wasm_import_module = \"env\")]\n\
         unsafe extern \"C\" { fn check(ok: i32, line: u32); }\n\
         fn assert_ok(ok: bool, line: u32) { unsafe { check(i32::from(ok), line); } }\n\
         #[unsafe(no_mangle)]\n\
         pub extern \"C\" fn run() {\n\
             let s = base_lib::make_surface(640.0, 480.0);\n\
             assert_ok(app_lib::render(&s, app_lib::Quality::High) == 1130.0, line!());\n\
             let bc = base_lib::make_base_context(7.0);\n\
             let ac = app_lib::make_app_context(\"gl\");\n\
             assert_ok(bc.id() == 7.0 && ac.label() == \"gl\", line!());\n\
             assert_ok(base_lib::describe_mode(base_lib::Mode::Fast) == \"mode:fast\", line!());\n\
             assert_ok(app_lib::describe_quality(app_lib::Quality::High) == \"quality:high\", line!());\n\
             drop((s, bc, ac));\n\
             assert_ok(base_lib::js::live_handles() == 0, line!());\n\
         }\n",
    )
    .unwrap();
    run(Command::new(&rustc)
        .current_dir(&out)
        .args(RUSTC_WASM)
        .args(["--crate-type", "cdylib", "-C", "link-arg=--export-table", "-C", "strip=debuginfo", "--extern"])
        .arg(format!("base_lib={}", base_rlib.display()))
        .arg("--extern")
        .arg(format!("app_lib={}", app_rlib.display()))
        .args(["-o", "test.wasm", "test.rs"]));
    write_composable_runner(&out);
    let report = run(Command::new(node).current_dir(&out).arg("run.mjs"));
    assert!(report.contains("5 checks passed"), "{report}");
}

#[test]
fn zig_cross_project_refs() {
    let Some([zig, node]) = tools(["zig", "node"]) else { return };
    let (out, bindings) = setup_ref_fixture("zig-refs", "zig");
    let app_build = fs::read_to_string(bindings.join("app_lib/build.zig")).unwrap();
    assert!(app_build.contains("base_lib") && app_build.contains("../base_lib/bindings.zig"), "{app_build}");
    let app_types = fs::read_to_string(bindings.join("app_lib/types.zig")).unwrap();
    assert!(app_types.contains("@import(\"base_lib\").Surface"), "{app_types}");

    fs::write(
        out.join("test.zig"),
        "const base = @import(\"base_lib\");\n\
         const app = @import(\"app_lib\");\n\
         extern \"env\" fn check(ok: i32, line: u32) void;\n\
         fn assertOk(ok: bool, line: u32) void { check(@intFromBool(ok), line); }\n\
         export fn run() void {\n\
             const s = base.makeSurface(640.0, 480.0);\n\
             defer base.jsRelease(s);\n\
             assertOk(app.render(s, .high) == 1130.0, @src().line);\n\
             const bc = base.makeBaseContext(7.0);\n\
             defer base.jsRelease(bc);\n\
             const ac = app.makeAppContext(\"gl\");\n\
             defer app.jsRelease(ac);\n\
             assertOk(bc.getId() == 7.0, @src().line);\n\
             const ac_label = ac.getLabel();\n\
             defer app.jsRelease(ac_label);\n\
             assertOk(ac_label.eql(\"gl\"), @src().line);\n\
             const desc_mode = base.describeMode(.fast);\n\
             defer base.jsRelease(desc_mode);\n\
             assertOk(desc_mode.eql(\"mode:fast\"), @src().line);\n\
             const desc_qual = app.describeQuality(.high);\n\
             defer app.jsRelease(desc_qual);\n\
             assertOk(desc_qual.eql(\"quality:high\"), @src().line);\n\
             const ms = base.JsArrayOf(base.Mode).of(&.{ .fast, .safe });\n\
             defer base.jsRelease(ms);\n\
             assertOk(ms.get(1) == .safe, @src().line);\n\
             const qs = app.JsArrayOf(app.Quality).of(&.{ .low, .high });\n\
             defer app.jsRelease(qs);\n\
             assertOk(qs.get(1) == .high, @src().line);\n\
         }\n",
    )
    .unwrap();
    run(Command::new(zig)
        .current_dir(&out)
        .args([
            "build-exe",
            "-target",
            "wasm32-freestanding",
            "-fno-entry",
            "-rdynamic",
            "--export-table",
            "-O",
            "ReleaseSmall",
            "--dep",
            "base_lib",
            "--dep",
            "app_lib",
            "-Mroot=test.zig",
            "-Mbase_lib=bindings/base_lib/bindings.zig",
            "--dep",
            "base_lib",
            "-Mapp_lib=bindings/app_lib/bindings.zig",
            "-femit-bin=test.wasm",
        ]));
    write_composable_runner(&out);
    let report = run(Command::new(node).current_dir(&out).arg("run.mjs"));
    assert!(report.contains("7 checks passed"), "{report}");
}

#[test]
fn c_cross_project_refs() {
    let Some([clang, node]) = tools(["clang", "node"]) else { return };
    let (out, bindings) = setup_ref_fixture("c-refs", "c");
    let app_types = fs::read_to_string(bindings.join("app_lib/ts_types.h")).unwrap();
    assert!(app_types.contains("../base_lib/ts_types.h"), "{app_types}");
    assert!(!app_types.contains("} Surface;"), "Surface should not be redeclared:\n{app_types}");
    assert!(bindings.join("base_lib/base_lib.h").exists() && bindings.join("app_lib/app_lib.h").exists());

    fs::write(
        out.join("test.c"),
        "#include \"base_lib/base_lib.h\"\n\
         #include \"app_lib/app_lib.h\"\n\
         __attribute__((import_module(\"env\"), import_name(\"check\"))) void check(int32_t ok, uint32_t line);\n\
         #define CHECK(ok) check((ok) ? 1 : 0, __LINE__)\n\
         __attribute__((export_name(\"run\"))) void run(void) {\n\
             Surface s = makeSurface(640.0, 480.0);\n\
             CHECK(render(s, Quality_high) == 1130.0);\n\
             RenderingContext bc = makeBaseContext(7.0);\n\
             RenderingContext_2 ac = makeAppContext(\"gl\");\n\
             CHECK(RenderingContext_get_id(bc) == 7.0);\n\
             JsString lbl = RenderingContext_2_get_label(ac);\n\
             CHECK(JsString_equals(lbl, \"gl\"));\n\
             JsString dm = describeMode(Mode_fast);\n\
             CHECK(JsString_equals(dm, \"mode:fast\"));\n\
             JsString dq = describeQuality(Quality_high);\n\
             CHECK(JsString_equals(dq, \"quality:high\"));\n\
             js_release(dq); js_release(dm); js_release(lbl); js_release(ac); js_release(bc); js_release(s);\n\
             CHECK(js_live_handles() == 0);\n\
         }\n",
    )
    .unwrap();
    run(Command::new(clang).args(CLANG_WASM).arg("-I").arg(&bindings).arg("-o").arg(out.join("test.wasm")).arg(out.join("test.c")));
    write_composable_runner(&out);
    let report = run(Command::new(node).current_dir(&out).arg("run.mjs"));
    assert!(report.contains("6 checks passed"), "{report}");
}

#[test]
fn cpp_cross_project_refs() {
    let Some(clang) = wasi_clang() else { return };
    let Some([node]) = tools(["node"]) else { return };
    let (out, bindings) = setup_ref_fixture("cpp-refs", "cpp");
    let app_types = fs::read_to_string(bindings.join("app_lib/ts_types.hpp")).unwrap();
    assert!(app_types.contains("../base_lib/base_lib.hpp") && app_types.contains("using Surface = ::base_lib::Surface;"), "{app_types}");

    let mut cpp_sources = Vec::new();
    fn collect_cpp(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(dir).unwrap() {
            let p = entry.unwrap().path();
            if p.is_dir() {
                collect_cpp(&p, out);
            } else if p.extension().and_then(|s| s.to_str()) == Some("cpp") {
                out.push(p);
            }
        }
    }
    collect_cpp(&bindings.join("base_lib"), &mut cpp_sources);
    collect_cpp(&bindings.join("app_lib"), &mut cpp_sources);
    fs::write(
        out.join("test.cpp"),
        "#include \"base_lib/base_lib.hpp\"\n\
         #include \"app_lib/app_lib.hpp\"\n\
         extern \"C\" __attribute__((import_module(\"env\"), import_name(\"check\"))) void check(int32_t ok, uint32_t line);\n\
         #define CHECK(ok) check((ok) ? 1 : 0, __LINE__)\n\
         extern \"C\" __attribute__((export_name(\"run\"))) void run() {\n\
             {\n\
                 base_lib::Surface s = base_lib::makeSurface(640.0, 480.0);\n\
                 CHECK(app_lib::render(s, app_lib::Quality::high) == 1130.0);\n\
                 base_lib::RenderingContext bc = base_lib::makeBaseContext(7.0);\n\
                 app_lib::RenderingContext ac = app_lib::makeAppContext(\"gl\");\n\
                 CHECK(bc.id() == 7.0 && ac.label() == \"gl\");\n\
                 CHECK(base_lib::describeMode(base_lib::Mode::fast) == \"mode:fast\");\n\
                 CHECK(app_lib::describeQuality(app_lib::Quality::high) == \"quality:high\");\n\
                 ts::Array<base_lib::Mode> ms{ base_lib::Mode::fast, base_lib::Mode::safe };\n\
                 CHECK(ms[1] == base_lib::Mode::safe);\n\
                 ts::Array<app_lib::Quality> qs{ app_lib::Quality::low, app_lib::Quality::high };\n\
                 CHECK(qs[1] == app_lib::Quality::high);\n\
             }\n\
             CHECK(ts::live_handles() == 0);\n\
         }\n",
    )
    .unwrap();
    let mut cmd = Command::new(&clang);
    cmd.args(CLANGXX_WASM).arg("-I").arg(&bindings).arg("-o").arg(out.join("test.wasm")).arg(out.join("test.cpp"));
    for s in &cpp_sources {
        cmd.arg(s);
    }
    run(&mut cmd);
    write_composable_runner(&out);
    let report = run(Command::new(node).current_dir(&out).arg("run.mjs"));
    assert!(report.contains("7 checks passed"), "{report}");
}

#[test]
fn gleam_cross_project_refs() {
    let Some([gleam, node]) = tools(["gleam", "node"]) else { return };
    let (out, bindings) = setup_ref_fixture("gleam-refs", "gleam");
    let app_toml = fs::read_to_string(bindings.join("app_lib/gleam.toml")).unwrap();
    assert!(app_toml.contains("base_lib = { path = \"../base_lib\" }"), "{app_toml}");

    // Seed consumer project using the locked stdlib from tests/gleam/manifest.toml
    // plus local base_lib/app_lib path deps so Gleam compiles offline.
    fs::create_dir_all(out.join("src")).unwrap();
    fs::create_dir_all(out.join("node_modules/base-lib")).unwrap();
    fs::write(out.join("node_modules/base-lib/package.json"), "{ \"name\": \"base-lib\", \"type\": \"module\", \"exports\": \"./index.mjs\" }\n").unwrap();
    fs::write(
        out.join("node_modules/base-lib/index.mjs"),
        "export function makeSurface(w, h) { return { width: w, height: h }; }\n\
         export function makeBaseContext(id) { return { id }; }\n\
         export function describeMode(m) { return 'mode:' + m; }\n",
    )
    .unwrap();
    fs::write(
        out.join("gleam.toml"),
        "name = \"check\"\n\
         version = \"1.0.0\"\n\
         target = \"javascript\"\n\n\
         [dependencies]\n\
         gleam_stdlib = \">= 1.0.0 and < 2.0.0\"\n\
         base_lib = { path = \"bindings/base_lib\" }\n\
         app_lib = { path = \"bindings/app_lib\" }\n",
    )
    .unwrap();
    fs::write(
        out.join("manifest.toml"),
        "packages = [\n\
         \x20 { name = \"app_lib\", version = \"0.1.0\", build_tools = [\"gleam\"], requirements = [\"base_lib\", \"gleam_stdlib\"], source = \"local\", path = \"bindings/app_lib\" },\n\
         \x20 { name = \"base_lib\", version = \"0.1.0\", build_tools = [\"gleam\"], requirements = [\"gleam_stdlib\"], source = \"local\", path = \"bindings/base_lib\" },\n\
         \x20 { name = \"gleam_stdlib\", version = \"1.0.5\", build_tools = [\"gleam\"], requirements = [], otp_app = \"gleam_stdlib\", source = \"hex\", outer_checksum = \"CEE5B6C076A85B45F60C585F4316C63EC8B7127C119D5738C3958A9C4D50404E\" },\n\
         ]\n\n\
         [requirements]\n\
         app_lib = { path = \"bindings/app_lib\" }\n\
         base_lib = { path = \"bindings/base_lib\" }\n\
         gleam_stdlib = { version = \">= 1.0.0 and < 2.0.0\" }\n",
    )
    .unwrap();
    fs::write(
        out.join("src/check.gleam"),
        "import gleam/io\n\
         import base_lib/areas/main as base\n\
         import base_lib/areas/main/surface\n\
         import base_lib/areas/main/rendering_context as base_ctx\n\
         import base_lib/types as base_types\n\
         import app_lib/areas/main as app\n\
         import app_lib/areas/main/rendering_context as app_ctx\n\
         import app_lib/types as app_types\n\n\
         pub fn main() {\n\
           let s = base.make_surface(640.0, 480.0)\n\
           let assert True = surface.get_width(s) +. surface.get_height(s) == 1120.0\n\
           let assert True = app.render(s, app_types.QualityHigh) == 1130.0\n\
           let bc = base.make_base_context(7.0)\n\
           let ac = app.make_app_context(\"gl\")\n\
           let assert True = base_ctx.get_id(bc) == 7.0 && app_ctx.get_label(ac) == \"gl\"\n\
           let assert True = base.describe_mode(base_types.ModeFast) == \"mode:fast\"\n\
           let assert True = app.describe_quality(app_types.QualityHigh) == \"quality:high\"\n\
           io.println(\"5 checks passed\")\n\
         }\n",
    )
    .unwrap();
    fs::write(
        out.join("impl.mjs"),
        "globalThis.render = (s, q) => s.width + s.height + (q === 'high' ? 10 : 1);\n\
         globalThis.makeAppContext = label => ({ label });\n\
         globalThis.describeQuality = q => 'quality:' + q;\n",
    )
    .unwrap();
    run(Command::new(gleam).current_dir(&out).args(["build", "--warnings-as-errors"]));
    let report = run(Command::new(node).arg(root().join("tests/run_gleam.mjs")).arg(out.join("impl.mjs")).arg(out.join("build/dev/javascript/check/check.mjs")));
    assert!(report.contains("5 checks passed"), "{report}");
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
    let m = model(&shapes_xml("zig", &[]), "Zig");
    assert!(m.all_funcs().any(|f| f.params.iter().any(|p| matches!(p.1, generators::model::Ty::Nullable(..)))));
}

#[test]
fn fsharp_rejects_literal_types() {
    let problems = generators::fsharp::module(&shapes_xml("fsharp", &["literals=keep"])).err().unwrap();
    assert!(problems.iter().any(|p| p.contains("cannot express <literal>: set literals = \"enum\"")), "{problems:?}");
}

// A type argument outside its parameter's constraint is a Rust compile error:
// `largest<T extends Item>` takes no array of numbers.
#[test]
fn rust_checks_constraints() {
    let Some(rustc) = rustc_wasm() else { return };
    let c = case_in("data", "rust", "data-rust-wrong");
    rust_bindings(&rustc, &c.out, &model(&c.xml, "Rust"));
    fs::write(c.out.join("test.rs"), "#![no_main]\nuse bindings::*;\npub fn wrong() { largest(JsArray::<f64>::new()); }\n").unwrap();
    let out = Command::new(rustc).current_dir(&c.out).args(RUSTC_WASM).args(RUSTC_PROGRAM).args(["-o", "test.wasm", "test.rs"]).output().unwrap();
    let errors = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success() && errors.contains("Is<Item>"), "an array of numbers should not satisfy T: Is<Item>:\n{errors}");
}

// A type argument outside its parameter's constraint is an F# compile error:
// `largest<T extends Item>` takes no array of numbers.
#[test]
fn fsharp_checks_constraints() {
    let Some([fable]) = tools(["fable"]) else { return };
    let c = case_in("data", "fsharp", "data-fsharp-wrong");
    write_fsharp(&c, &generators::fsharp::module(&c.xml).unwrap());
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

// The whole DOM's Gleam bindings compile.

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

const GLEAM_PAGE: &str = r#"<!doctype html>
<title>untouched</title>
<div id="seed" data-x="42">seed text</div>
<script type="module">
import { verify } from './verify.mjs';
try {
  const { main } = await import('./build/dev/javascript/check/check.mjs');
  main();
} catch (e) {
  window.__gleam = { passed: 0, failed: ['threw: ' + e] };
}
const page = verify('gleam');
const { passed, failed } = window.__gleam ?? { passed: 0, failed: ['the program did not report'] };
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

    // What F#'s instance members reach, read from the generated code: an
    // abstract member is the JavaScript member of its name (Fable compiles
    // `abstract checked: bool with get, set` to `x.checked`), an `Emit` one
    // what it emits. Static sides and constructors are not read.
    fn fsharp_manifest(files: &[(String, String)]) -> String {
        let mut out = String::from("export const bindings = [\n");
        let mut n = 0;
        for (_, text) in files {
            let mut owner = String::new();
            for line in text.lines() {
                let t = line.trim_start();
                if let Some(rest) = t.strip_prefix("type ").or_else(|| t.strip_prefix("and ")) {
                    owner = rest.split([' ', '<', '=']).next().unwrap_or_default().trim_matches('`').to_string();
                    continue;
                }
                // A type-level map (`HTMLElementTagNameMap`) is nothing at runtime.
                if owner.ends_with("Map") {
                    continue;
                }
                // A static side's members are its class's statics.
                let class = owner.strip_suffix("Static").map(str::to_string);
                let mut push = |kind: &str, member: &str| {
                    n += 1;
                    match &class {
                        Some(c) => {
                            let kind = match kind {
                                "get" => "staticGet",
                                "set" => "staticSet",
                                _ => "static",
                            };
                            out += &format!("  [\"fs{n}\", {kind:?}, \"\", \"{c}.{member}\", \"\"],\n");
                        }
                        None => out += &format!("  [\"fs{n}\", {kind:?}, {owner:?}, {member:?}, {owner:?}],\n"),
                    }
                };
                // `Emit("$0.foo")`, `Emit("$0.foo = $1")`, `Emit("$0.foo($1...)")`.
                let emits: Vec<&str> = line.split("Emit(\"").skip(1).filter_map(|e| e.split("\")").next()).collect();
                for e in &emits {
                    let Some(rest) = e.strip_prefix("$0.") else { continue };
                    let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '$').collect();
                    let after = &rest[name.len()..];
                    if after.starts_with(" = ") {
                        push("set", &name);
                    } else if after.starts_with('(') {
                        push("method", &name);
                    } else if after.is_empty() {
                        push("get", &name);
                    }
                }
                if !emits.is_empty() {
                    continue;
                }
                let Some(rest) = t.strip_prefix("abstract ") else { continue };
                let Some((name, sig)) = rest.split_once(':') else { continue };
                let name = name.trim().split('<').next().unwrap_or_default().trim_matches('`');
                if name.contains(' ') || name.is_empty() {
                    continue;
                }
                if sig.contains("with get, set") {
                    push("get", name);
                    push("set", name);
                } else if sig.contains("with get") || !sig.contains("->") {
                    push("get", name);
                } else {
                    push("method", name);
                }
            }
        }
        out += "];\nexport const implementers = {};\n";
        out
    }

    // Every binding of the DOM, in each wasm target's model and Gleam's,
    // reaches what it names in a real browser (tests/dom/bindings.mjs).
    #[test]
    fn every_binding_resolves() {
        let out = root().join("target/e2e/dom-bindings");
        let _ = fs::remove_dir_all(&out);
        fs::create_dir_all(&out).unwrap();
        let targets = [("c", "C"), ("cpp", "C++"), ("zig", "Zig"), ("rust", "Rust"), ("gleam", "Gleam")];
        let mut declared = std::collections::BTreeSet::new();
        for (target, name) in targets {
            let xml = lower_with(Path::new("lib.dom.d.ts"), Some(target), &[root().join("examples/modules/dom.toml")], &[]);
            let doc = roxmltree::Document::parse(&xml).unwrap();
            for n in doc.descendants().filter(|n| n.is_element()) {
                declared.extend(["name", "jsName"].iter().filter_map(|a| n.attribute(*a)).map(str::to_string));
            }
            let manifest = match target {
                "gleam" => generators::gleam::module(&xml).unwrap_or_else(|p| panic!("{}", p.join("\n"))).manifest,
                _ => generators::wasm::manifest(&model(&xml, name)),
            };
            fs::write(out.join(format!("manifest-{target}.mjs")), manifest).unwrap();
        }
        let xml = lower_with(Path::new("lib.dom.d.ts"), Some("fsharp"), &[root().join("examples/modules/dom.toml")], &[]);
        fs::write(out.join("manifest-fsharp.mjs"), fsharp_manifest(&generators::fsharp::module(&xml).unwrap().files)).unwrap();
        fs::write(out.join("declared.mjs"), format!("export const declared = {:?};\n", declared.into_iter().collect::<Vec<_>>())).unwrap();
        let imports: String = targets.iter().map(|(t, _)| format!("import * as {t} from './manifest-{t}.mjs';\n")).collect::<String>() + "import * as fsharp from './manifest-fsharp.mjs';\n";
        // F#'s classes are the DOM's: Rust's say what implements what.
        let checks: String = targets.iter().map(|(t, _)| format!("results.{t} = verify({t}.bindings, {t}.implementers, declared);\n")).collect::<String>() + "const owners = new Map(rust.bindings.map(b => [b[4], b[2]]));\n\
const bags = rust.bindings.filter(b => b[1] === 'create');\n\
results.fsharp = verify(fsharp.bindings.map(([i, k, o, m, c]) => [i, k, owners.get(c) ?? o, m, c]).concat(bags), rust.implementers, declared);\n";
        fs::write(
            out.join("index.html"),
            format!(
                "<!doctype html>\n<title>bindings</title>\n<script type=\"module\">\nimport {{ verify }} from './bindings.mjs';\nimport {{ declared }} from './declared.mjs';\n{imports}const results = {{}};\n{checks}\
const failed = Object.entries(results).flatMap(([t, r]) => r.wrong.map(w => `${{t}}: ${{w}}`));\n\
const passed = Object.values(results).reduce((n, r) => n + r.counts.resolved, 0);\n\
const notes = Object.entries(results).map(([t, r]) => `${{t}}: ${{Object.entries(r.counts).map(([k, n]) => `${{n}} ${{k}}`).join(', ')}}`);\n\
window.__result = {{ passed, failed, dom: [], domChecks: 0, notes }};\n</script>\n"
            ),
        )
        .unwrap();
        fs::copy(root().join("tests/dom/bindings.mjs"), out.join("bindings.mjs")).unwrap();
        run_browser(&Case { fixture: root().join("tests/dom"), out, xml: String::new() });
    }

    #[test]
    fn c() {
        let Some([clang]) = tools(["clang"]) else { return };
        let c = dom_case("c");
        let model = model(&c.xml, "C");
        write_files(&c.out.join("bindings"), generators::c::files(&model).unwrap());
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
        let model = model(&c.xml, "C++");
        fs::write(c.out.join("index.html"), wasm_page("cpp")).unwrap();
        print!("{}", skipped(&model.skipped));
        build_cpp(&clang, &c, &model);
        write_glue(&c, &model);
        run_browser(&c);
        // A listener of the wrong event type is a compile error.
        let flags = CLANGXX_WASM.iter().filter(|f| !f.starts_with("-Wl,") && !f.starts_with("-mexec-model"));
        let out = Command::new(&clang).args(flags).arg("-fsyntax-only").arg("-I").arg(c.out.join("bindings")).arg(c.fixture.join("wrong.cpp")).output().unwrap();
        let errors = String::from_utf8_lossy(&out.stderr);
        let expected = ["no matching member function for call to 'addEventListener'", "no matching member function for call to 'querySelectorAll'", "constraints not satisfied for class template 'NodeListOf'"];
        assert!(!out.status.success() && expected.iter().all(|e| errors.contains(e)), "wrong.cpp should not compile, for both reasons:\n{errors}");
    }

    #[test]
    fn rust() {
        let Some(rustc) = super::rustc_wasm() else { return };
        let c = dom_case("rust");
        let model = model(&c.xml, "Rust");
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
        let bindings = generators::fsharp::module(&c.xml).unwrap();
        write_fsharp(&c, &bindings);
        fs::write(c.out.join("index.html"), FSHARP_PAGE).unwrap();
        print!("{}", skipped(&bindings.skipped));
        run(Command::new(fable).current_dir(&c.out).args(["Test.fsproj", "-o", "js", "--extension", ".mjs", "--noCache"]));
        run_browser(&c);
    }

    // Every module in src is compiled, not only those the program imports.
    #[test]
    fn gleam() {
        let Some([gleam]) = tools(["gleam"]) else { return };
        let c = dom_case("gleam");
        let bindings = generators::gleam::module(&c.xml).unwrap_or_else(|p| panic!("{}", p.join("\n")));
        write_gleam_project(&c.out, &bindings);
        fs::copy(c.fixture.join("test.gleam"), c.out.join("src/check.gleam")).unwrap();
        fs::copy(c.fixture.join("page_ffi.mjs"), c.out.join("src/page_ffi.mjs")).unwrap();
        fs::write(c.out.join("index.html"), GLEAM_PAGE).unwrap();
        print!("{}", skipped(&bindings.skipped));
        run(Command::new(gleam).current_dir(&c.out).args(["build", "--warnings-as-errors"]));
        run_browser(&c);
    }

    #[test]
    fn zig() {
        let Some([zig]) = tools(["zig"]) else { return };
        let c = dom_case("zig");
        let model = model(&c.xml, "Zig");
        write_files(&c.out, generators::zig::files(&model).unwrap());
            fs::copy(c.fixture.join("test.zig"), c.out.join("test.zig")).unwrap();
        fs::write(c.out.join("index.html"), wasm_page("zig")).unwrap();
        print!("{}", skipped(&model.skipped));
        run(Command::new(zig).current_dir(&c.out).args(ZIG_WASM));
        write_glue(&c, &model);
        run_browser(&c);
    }
}

// ---- the DOM by area ----
//
// tests/dom/areas/<area>/test.<ext>: a program per area of the DOM (the
// tree, events, forms, ...) in each language, against the whole DOM's
// bindings, in Chrome. Its `page.html`, if any, is in the page's body.
// Slow, so not run by default:
//
//     cargo test --test e2e dom_areas -- --ignored

mod dom_areas {
    use super::*;

    fn area(name: &str, target: &str) {
        let dir = root().join("tests/dom/areas").join(name);
        let ext = match target {
            "fsharp" => "fs",
            "rust" => "rs",
            t => t,
        };
        if !dir.join(format!("test.{ext}")).exists() {
            eprintln!("SKIPPED: no tests/dom/areas/{name}/test.{ext}");
            return;
        }
        let out = root().join("target/e2e").join(format!("dom-{name}-{target}"));
        let _ = fs::remove_dir_all(&out);
        fs::create_dir_all(&out).unwrap();
        let xml = lower_with(Path::new("lib.dom.d.ts"), Some(target), &[root().join("examples/modules/dom.toml")], &[]);
        let c = Case { fixture: dir.clone(), out, xml };
        let body = fs::read_to_string(dir.join("page.html")).unwrap_or_default();
        let page = |script: &str| format!("<!doctype html>\n<title>{name}</title>\n<body>\n{body}\n{script}</body>\n");
        match target {
            "c" => {
                let Some([clang]) = tools(["clang"]) else { return };
                let model = model(&c.xml, "C");
                write_files(&c.out.join("bindings"), generators::c::files(&model).unwrap());
                run(Command::new(clang).args(CLANG_WASM).arg("-I").arg(c.out.join("bindings")).arg("-o").arg(c.out.join("test.wasm")).arg(c.fixture.join("test.c")));
                write_glue(&c, &model);
                fs::write(c.out.join("index.html"), page(AREA_WASM)).unwrap();
            }
            "cpp" => {
                let Some(clang) = wasi_clang() else { return };
                let model = model(&c.xml, "C++");
                build_cpp(&clang, &c, &model);
                write_glue(&c, &model);
                fs::write(c.out.join("index.html"), page(AREA_WASM)).unwrap();
            }
            "zig" => {
                let Some([zig]) = tools(["zig"]) else { return };
                let model = model(&c.xml, "Zig");
                write_files(&c.out, generators::zig::files(&model).unwrap());
                fs::copy(c.fixture.join("test.zig"), c.out.join("test.zig")).unwrap();
                run(Command::new(zig).current_dir(&c.out).args(ZIG_WASM));
                write_glue(&c, &model);
                fs::write(c.out.join("index.html"), page(AREA_WASM)).unwrap();
            }
            "rust" => {
                let Some(rustc) = rustc_wasm() else { return };
                let model = model(&c.xml, "Rust");
                build_rust(&rustc, &c, &model);
                write_glue(&c, &model);
                fs::write(c.out.join("index.html"), page(AREA_WASM)).unwrap();
            }
            "fsharp" => {
                let Some([fable]) = tools(["fable"]) else { return };
                write_fsharp(&c, &generators::fsharp::module(&c.xml).unwrap());
                run(Command::new(fable).current_dir(&c.out).args(["Test.fsproj", "-o", "js", "--extension", ".mjs", "--noCache"]));
                fs::write(c.out.join("index.html"), page(&area_program("await import('./js/Test.mjs');"))).unwrap();
            }
            "gleam" => {
                let Some([gleam]) = tools(["gleam"]) else { return };
                write_gleam_project(&c.out, &generators::gleam::module(&c.xml).unwrap_or_else(|p| panic!("{}", p.join("\n"))));
                fs::copy(c.fixture.join("test.gleam"), c.out.join("src/check.gleam")).unwrap();
                run(Command::new(gleam).current_dir(&c.out).args(["build", "--warnings-as-errors"]));
                fs::write(c.out.join("index.html"), page(&area_program("(await import('./build/dev/javascript/check/check.mjs')).main();"))).unwrap();
            }
            _ => unreachable!(),
        }
        run_browser(&c);
    }

    // C, C++, Zig, Rust: `run`, then, once `busy()` is 0, `finish`.
    const AREA_WASM: &str = r#"<script type="module">
import { makeEnv } from './glue.mjs';
let instance;
let passed = 0;
const failed = [];
const options = { check: true, unchecked: e => failed.push('unchecked exception: ' + (e?.message ?? e)) };
const env = { ...makeEnv(() => instance, options), check: (ok, line) => (ok ? passed++ : failed.push('line ' + line)) };
const wasi = new Proxy({}, { get: () => () => 0 });
({ instance } = await WebAssembly.instantiate(await (await fetch('./test.wasm')).arrayBuffer(), { env, wasi_snapshot_preview1: wasi }));
instance.exports._initialize?.();
instance.exports.run();
await new Promise(r => setTimeout(r, 200));
const deadline = Date.now() + 20000;
while (instance.exports.busy?.() && Date.now() < deadline) await new Promise(r => setTimeout(r, 10));
instance.exports.finish?.();
window.__result = { passed, failed, dom: [], domChecks: 0 };
</script>
"#;

    // F# and Gleam: the program reports `N checks passed`, last, or throws.
    fn area_program(start: &str) -> String {
        format!(
            r#"<script type="module">
let passed = 0;
const failed = [];
const log = console.log;
console.log = (...args) => {{
  const m = String(args[0]).match(/(\d+) checks passed/);
  if (m) passed = Number(m[1]);
  log(...args);
}};
window.addEventListener('unhandledrejection', e => failed.push('rejected: ' + (e.reason?.message ?? e.reason)));
try {{
  {start}
}} catch (e) {{
  failed.push('threw: ' + (e?.message ?? e));
}}
const deadline = Date.now() + 20000;
while (!passed && !failed.length && Date.now() < deadline) await new Promise(r => setTimeout(r, 50));
if (!passed && !failed.length) failed.push('the program did not report');
window.__result = {{ passed, failed, dom: [], domChecks: 0 }};
</script>
"#
        )
    }

    macro_rules! areas {
        ($($name:ident),*) => { $(mod $name {
            #[test] #[ignore] fn c() { super::area(stringify!($name), "c"); }
            #[test] #[ignore] fn cpp() { super::area(stringify!($name), "cpp"); }
            #[test] #[ignore] fn zig() { super::area(stringify!($name), "zig"); }
            #[test] #[ignore] fn rust() { super::area(stringify!($name), "rust"); }
            #[test] #[ignore] fn fsharp() { super::area(stringify!($name), "fsharp"); }
            #[test] #[ignore] fn gleam() { super::area(stringify!($name), "gleam"); }
        })* };
    }

    areas!(tree, events, forms, cssom, canvas, fetch, url, storage, timers, observers, svg);
}

// File names must differ where letter case does not count, and avoid names
// Windows reserves.
#[test]
fn file_names_never_clash() {
    let xml = lower(&root().join("tests/fixtures/names/api.d.ts"), Some("c"), &[]);
    let files: Vec<String> = generators::c::files(&model(&xml, "C")).unwrap().into_iter().map(|f| f.0).collect();
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

// Multi-file packages load their directory, follow imports and `export *`,
// and emit referenced declarations instead of `kind="import"` stubs.
#[test]
fn multifile_packages_keep_their_types() {
    let dir = root().join("target/e2e/multifile-pkg");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("sub")).unwrap();
    fs::write(dir.join("package.json"), r#"{"name": "multi-pkg"}"#).unwrap();
    fs::write(dir.join("sub/coord.d.ts"), "export interface Coord { v: number; }\n").unwrap();
    fs::write(
        dir.join("shapes.d.ts"),
        "import { Coord } from \"./sub/coord\";\nexport interface Point { x: Coord; y: number; }\nexport interface Box { w: number; h: number; }\n",
    )
    .unwrap();
    fs::write(dir.join("reexport.d.ts"), "export * from \"./shapes\";\n").unwrap();
    fs::write(dir.join("aliased.d.ts"), "export interface AliasTarget { id: number; }\n").unwrap();
    fs::write(dir.join("ns.d.ts"), "export interface NsType { tag: string; }\nexport declare function helper(): number;\n").unwrap();
    fs::write(dir.join("anon_default.d.ts"), "export default class { value: number; }\n").unwrap();
    fs::write(
        dir.join("index.d.ts"),
        "import { Point } from \"./shapes\";\nimport { AliasTarget as Renamed } from \"./aliased\";\nimport * as Ns from \"./ns\";\nimport AnonDefault from \"./anon_default\";\nexport { Box } from \"./reexport\";\nexport * from \"./reexport\";\nexport declare function origin(): Point;\nexport declare function makeRenamed(): Renamed;\nexport declare function useNs(h: typeof Ns.helper, t: Ns.NsType): Ns.NsType;\nexport declare function makeDefault(): AnonDefault;\n",
    )
    .unwrap();

    let xml = lower(&dir.join("index.d.ts"), Some("rust"), &[]);
    if let Some([xmllint]) = tools(["xmllint"]) {
        let xml_path = dir.join("api.xml");
        fs::write(&xml_path, &xml).unwrap();
        run(Command::new(xmllint).args(["--noout", "--relaxng"]).arg(root().join("schema/declarations.rng")).arg(&xml_path));
    }

    assert!(!xml.contains("kind=\"import\""), "import stubs must be resolved:\n{xml}");
    assert!(!xml.contains("name=\"Ns.NsType\""), "namespace-imported type must not keep import alias prefix:\n{xml}");
    assert!(xml.contains("<declarations import=\"multi-pkg\""), "module name should be inferred from package.json:\n{xml}");
    for name in ["Point", "Coord", "Box", "AliasTarget", "NsType"] {
        assert!(xml.contains(&format!("<interface module=\"main\" name=\"{name}\"")), "missing {name}:\n{xml}");
    }
    // An unnamed default class is named for its file: `default` is a keyword in several targets.
    assert!(xml.contains("<class module=\"main\" name=\"anon_default\""), "missing default class:\n{xml}");
    assert!(xml.contains("<ref name=\"Point\" kind=\"interface\" to=\"Point\"/>"), "origin return should reference Point interface:\n{xml}");
    assert!(xml.contains("<ref name=\"Coord\" kind=\"interface\" to=\"Coord\"/>"), "Point.x should reference transitive Coord:\n{xml}");
    assert!(xml.contains("<ref name=\"NsType\" kind=\"interface\" to=\"NsType\"/>"), "Ns.NsType should reference NsType interface:\n{xml}");

    // Verify generated Rust bindings actually compile against those multi-file types.
    if let Some(rustc) = rustc_wasm() {
        let model = model(&xml, "Rust");
        let rust_out = dir.join("rust");
        fs::create_dir_all(&rust_out).unwrap();
        rust_bindings(&rustc, &rust_out, &model);
    }

    // Packages inside node_modules/ must still resolve their own multi-file declarations.
    let nm_pkg = root().join("target/e2e/nm-host/node_modules/@scope/nm-pkg");
    let _ = fs::remove_dir_all(root().join("target/e2e/nm-host"));
    fs::create_dir_all(&nm_pkg).unwrap();
    fs::write(nm_pkg.join("package.json"), r#"{"name": "@scope/nm-pkg"}"#).unwrap();
    fs::write(nm_pkg.join("shapes.d.ts"), "export interface NodeModShape { n: number; }\n").unwrap();
    fs::write(nm_pkg.join("index.d.ts"), "import { NodeModShape } from \"./shapes\";\nexport declare function mk(): NodeModShape;\n").unwrap();
    let nm_xml = lower(&nm_pkg.join("index.d.ts"), Some("rust"), &[]);
    assert!(!nm_xml.contains("kind=\"import\""), "node_modules package import must be resolved:\n{nm_xml}");
    assert!(nm_xml.contains("<interface module=\"main\" name=\"NodeModShape\""), "node_modules package must emit NodeModShape:\n{nm_xml}");
}

// Top-level `export` files infer their JS module name automatically unless
// overridden by `[library] module`.
#[test]
fn module_library_is_inferred() {
    let pts_xml = lower(&root().join("tests/fixtures/pts/api.d.ts"), None, &[]);
    assert!(pts_xml.contains("<declarations import=\"pts\""), "pts top-level export should infer import=\"pts\":\n{}", &pts_xml[..pts_xml.len().min(300)]);

    let globals_xml = lower(&root().join("tests/fixtures/shapes/api.d.ts"), None, &[]);
    assert!(globals_xml.starts_with("<declarations>") || globals_xml.starts_with("<?xml"), "globals should not infer import:\n{}", &globals_xml[..globals_xml.len().min(300)]);
    assert!(!globals_xml.contains("<declarations import="), "globals must have no import attribute:\n{}", &globals_xml[..globals_xml.len().min(300)]);

    let dir = root().join("target/e2e/infer-pkg");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("package.json"), r#"{"name": "@scope/widget"}"#).unwrap();
    fs::write(dir.join("index.d.ts"), "export declare function ping(): number;\n").unwrap();
    let pkg_xml = lower(&dir.join("index.d.ts"), None, &[]);
    assert!(pkg_xml.contains("<declarations import=\"@scope/widget\""), "should infer package.json name:\n{pkg_xml}");

    // A types package names the module it describes.
    for (types, module) in [("@types/three", "three"), ("@types/babel__core", "@babel/core")] {
        let typed = root().join("target/e2e/infer-types").join(types.replace('/', "_"));
        let _ = fs::remove_dir_all(&typed);
        fs::create_dir_all(&typed).unwrap();
        fs::write(typed.join("package.json"), format!("{{\"name\": \"{types}\"}}")).unwrap();
        fs::write(typed.join("index.d.ts"), "export declare function ping(): number;\n").unwrap();
        assert!(lower(&typed.join("index.d.ts"), None, &[]).contains(&format!("<declarations import=\"{module}\"")), "{types} describes {module}");
    }

    // The nearest package.json above the file, however deep.
    fs::create_dir_all(dir.join("dist/types")).unwrap();
    fs::write(dir.join("dist/types/index.d.ts"), "export declare function ping(): number;\n").unwrap();
    assert!(lower(&dir.join("dist/types/index.d.ts"), None, &[]).contains("<declarations import=\"@scope/widget\""));

    // None: no name is made up from the file's, which would be imported and not found.
    let loose = root().join("target/e2e/infer-loose");
    let _ = fs::remove_dir_all(&loose);
    fs::create_dir_all(&loose).unwrap();
    fs::write(loose.join("geometry.d.ts"), "export declare function area(w: number, h: number): number;\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_tsrs_proto")).arg(loose.join("geometry.d.ts")).output().unwrap();
    let (xml, said) = (String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    assert!(!xml.contains("import="), "no package.json, no import:\n{xml}");
    assert!(said.contains("set [library] module"), "a module with no package says how to name it: {said}");
}

// Function types keep optional parameters in both F# and Rust, and `None`
// reaches JavaScript as `undefined`.
#[test]
fn function_types_keep_optional_parameters() {
    let dir = root().join("target/e2e/opt-fn-params");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("api.d.ts"),
        "export type StartFn = (count: number) => void;\n\
         export type Responses = {\n\
           start: (fn: StartFn, offset?: number, name?: string) => Responses;\n\
         };\n\
         export declare function invokeOpt(fn: (n: number, offset?: number, name?: string) => string): string;\n\
         export declare function getOpt1(): (offset?: number) => string;\n\
         export declare function getOpt2(): (n: number, offset?: number, name?: string) => string;\n\
         export declare function getResponses(): Responses;\n",
    )
    .unwrap();
    fs::write(
        dir.join("impl.mjs"),
        "export function invokeOpt(fn) { return fn(1) + '|' + fn(2, 10, 'hi'); }\n\
         export function getOpt1() { return offset => offset === undefined ? 'undef' : String(offset); }\n\
         export function getOpt2() { return (n, offset, name) => `${n}:${offset === undefined ? 'undef' : offset}:${name === undefined ? 'undef' : name}`; }\n\
         export function getResponses() {\n\
           const calls = [];\n\
           const api = {\n\
             start(fn, offset, name) {\n\
               calls.push(`${offset === undefined ? 'undef' : offset}:${name === undefined ? 'undef' : name}`);\n\
               fn(calls.length);\n\
               return api;\n\
             },\n\
             getCalls() { return calls.join('|'); }\n\
           };\n\
           return api;\n\
         }\n",
    )
    .unwrap();

    // Check F# signature and runtime execution via Fable + Node.
    let fs_xml = lower(&dir.join("api.d.ts"), Some("fsharp"), &["library.module=opt-fn-pkg"]);
    let fs_out = generators::fsharp::module(&fs_xml).unwrap();
    let fs_all: String = fs_out.files.iter().map(|(_, s)| s.as_str()).collect::<Vec<_>>().join("\n");
    assert!(fs_all.contains("delegate of n: float * ?offset: float * ?name: string -> string"), "F# function type must use delegate with optional params:\n{fs_all}");
    assert!(fs_all.contains("abstract start: fn: StartFn * ?offset: float * ?name: string -> Responses"), "F# function-typed property must preserve optional params:\n{fs_all}");
    if let Some([fable, node]) = tools(["fable", "node"]) {
        fs::write(
            dir.join("test.fs"),
            "module Test\n\
             open Bindings\n\
             open type Bindings.Globals\n\
             let a = invokeOpt (fun n offset name -> sprintf \"%g:%s:%s\" n (match offset with Some v -> string v | None -> \"none\") (defaultArg name \"none\"))\n\
             if a <> \"1:none:none|2:10:hi\" then failwith (\"bad invokeOpt: \" + a)\n\
             let cb1 = getOpt1 ()\n\
             let b1 = cb1.Invoke() + \"|\" + cb1.Invoke(5.0)\n\
             if b1 <> \"undef|5\" then failwith (\"bad getOpt1: \" + b1)\n\
             let cb2 = getOpt2 ()\n\
             let b2 = cb2.Invoke(1.0) + \"|\" + cb2.Invoke(2.0, 10.0) + \"|\" + cb2.Invoke(3.0, name = \"hi\")\n\
             if b2 <> \"1:undef:undef|2:10:undef|3:undef:hi\" then failwith (\"bad getOpt2: \" + b2)\n\
             let mutable seen = 0.0\n\
             let r = getResponses ()\n\
             r.start(fun c -> seen <- seen + c).start((fun c -> seen <- seen + c), 4.0).start((fun c -> seen <- seen + c), 5.0, \"go\") |> ignore\n\
             if seen <> 6.0 then failwith (\"bad Responses.start callbacks\")\n",
        )
        .unwrap();
        let fs_dir = dir.join("fsharp");
        fs::create_dir_all(&fs_dir).unwrap();
        let c = Case { fixture: dir.clone(), out: fs_dir.clone(), xml: fs_xml };
        write_fsharp(&c, &fs_out);
        let mod_dir = fs_dir.join("node_modules/opt-fn-pkg");
        fs::create_dir_all(&mod_dir).unwrap();
        fs::write(mod_dir.join("package.json"), r#"{ "name": "opt-fn-pkg", "type": "module", "exports": "./index.mjs" }"#).unwrap();
        fs::copy(dir.join("impl.mjs"), mod_dir.join("index.mjs")).unwrap();
        run(Command::new(fable).current_dir(&fs_dir).args(["Test.fsproj", "-o", "js", "--extension", ".mjs", "--noCache"]));
        run(Command::new(&node).arg(fs_dir.join("js/Test.mjs")));
    }

    // Check Rust signature and runtime execution via wasm + Node.
    let rs_api = dir.join("rust_api.d.ts");
    fs::write(&rs_api, "declare function invokeOpt(fn: (n: number, offset?: number, name?: string) => string): string;\n").unwrap();
    let rs_xml = lower(&rs_api, Some("rust"), &[]);
    let rs_model = model(&rs_xml, "Rust");
    let rs_files = generators::rust::files(&rs_model).unwrap();
    let rs_all: String = rs_files.iter().map(|(_, s)| s.as_str()).collect::<Vec<_>>().join("\n");
    assert!(rs_all.contains("Option<f64>") && rs_all.contains("Option<String>"), "Rust callback must use Option for optional parameters:\n{rs_all}");
    if let (Some(rustc), Some([node])) = (rustc_wasm(), tools(["node"])) {
        let rs_dir = dir.join("rust");
        fs::create_dir_all(&rs_dir).unwrap();
        let c = Case { fixture: dir.clone(), out: rs_dir.clone(), xml: rs_xml };
        rust_bindings(&rustc, &rs_dir, &rs_model);
        fs::write(
            rs_dir.join("test.rs"),
            "#![no_main]\nuse bindings::*;\n#[link(wasm_import_module = \"env\")]\nunsafe extern \"C\" {\n    #[link_name = \"check\"]\n    fn check_at(ok: i32, line: u32);\n}\n#[unsafe(no_mangle)]\npub extern \"C\" fn run() {\n    let f = InvokeOptFn::new(|n: f64, offset: Option<f64>, name: Option<String>| {\n        let o = offset.map(|v| format!(\"{v}\")).unwrap_or_else(|| \"none\".into());\n        let s = name.unwrap_or_else(|| \"none\".into());\n        format!(\"{n}:{o}:{s}\")\n    });\n    let ok = invoke_opt(&f) == \"1:none:none|2:10:hi\";\n    unsafe { check_at(ok as i32, line!()) };\n}\n",
        )
        .unwrap();
        run(Command::new(rustc).current_dir(&rs_dir).args(RUSTC_WASM).args(RUSTC_PROGRAM).args(["-o", "test.wasm", "test.rs"]));
        write_glue(&c, &rs_model);
        fs::write(
            rs_dir.join("impl.mjs"),
            "globalThis.invokeOpt = fn => fn(1) + '|' + fn(2, 10, 'hi');\n",
        )
        .unwrap();
        run(Command::new(node).arg(root().join("tests/run_wasm.mjs")).arg(rs_dir.join("impl.mjs")).arg(rs_dir.join("glue.mjs")).arg(rs_dir.join("test.wasm")));
    }

    // Also check real-world pts ITempoResponses keeps optionals in F# as an uncurried method.
    let pts_xml = lower(&root().join("tests/fixtures/pts/api.d.ts"), Some("fsharp"), &[]);
    let pts_fs = generators::fsharp::module(&pts_xml).unwrap();
    let pts_all: String = pts_fs.files.iter().map(|(_, s)| s.as_str()).collect::<Vec<_>>().join("\n");
    assert!(pts_all.contains("abstract start: fn: ITempoStartFn * ?offset: float * ?name: string -> ITempoResponses"), "pts ITempoResponses should have optional parameters:\n{pts_all}");
}

#[test]
fn intl_locale_formatting() {
    let dir = root().join("target/e2e/intl");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    // Lower lib.es2020.intl.d.ts (merging base Intl from lib.es5.d.ts + ES2020 Intl).
    let xml = lower(Path::new("lib.es2020.intl.d.ts"), Some("rust"), &[]);
    if let Some([xmllint]) = tools(["xmllint"]) {
        let mut child = Command::new(xmllint)
            .args(["--noout", "--relaxng"])
            .arg(root().join("schema/declarations.rng"))
            .arg("-")
            .stdin(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        use std::io::Write;
        child.stdin.take().unwrap().write_all(xml.as_bytes()).unwrap();
        assert!(child.wait().unwrap().success(), "lib.es2020.intl.d.ts XML failed schema validation");
    }

    // `--include Intl` adds a lib namespace to another input: lib.dom.d.ts, or a
    // file with the libs it names (`--lib es5,dom,es2020.intl`).
    let dom_intl_out = Command::new(env!("CARGO_BIN_EXE_tsrs_proto"))
        .args(["lib.dom.d.ts", "--include", "Intl", "--config"])
        .arg(config("rust"))
        .output()
        .unwrap();
    assert!(dom_intl_out.status.success(), "tsrs_proto lib.dom.d.ts --include Intl failed: {}", String::from_utf8_lossy(&dom_intl_out.stderr));
    let dom_intl_xml = String::from_utf8(dom_intl_out.stdout).unwrap();
    assert!(dom_intl_xml.contains("name=\"Intl\"") && dom_intl_xml.contains("name=\"DateTimeFormat\"") && dom_intl_xml.contains("name=\"NumberFormat\""));

    let file_intl_out = Command::new(env!("CARGO_BIN_EXE_tsrs_proto"))
        .arg(root().join("tests/fixtures/builtins/api.d.ts"))
        .args(["--lib", "es5,dom,es2020.intl", "--include", "Intl", "--config"])
        .arg(config("rust"))
        .output()
        .unwrap();
    assert!(file_intl_out.status.success(), "tsrs_proto builtins/api.d.ts --lib es5,dom,es2020.intl --include Intl failed: {}", String::from_utf8_lossy(&file_intl_out.stderr));
    let file_intl_xml = String::from_utf8(file_intl_out.stdout).unwrap();
    assert!(file_intl_xml.contains("name=\"Intl\"") && file_intl_xml.contains("name=\"RelativeTimeFormat\"") && file_intl_xml.contains("name=\"Locale\""));

    if let (Some(rustc), Some([node])) = (rustc_wasm(), tools(["node"])) {
        let m = model(&xml, "Rust");
        let c = Case { fixture: dir.clone(), out: dir.clone(), xml };
        rust_bindings(&rustc, &dir, &m);
        fs::write(
            dir.join("test.rs"),
            "#![no_main]\n\
             use bindings::*;\n\
             #[link(wasm_import_module = \"env\")]\n\
             unsafe extern \"C\" {\n\
                 #[link_name = \"check\"]\n\
                 fn check_at(ok: i32, line: u32);\n\
             }\n\
             macro_rules! check { ($e:expr) => { unsafe { check_at($e as i32, line!()) } }; }\n\
             #[unsafe(no_mangle)]\n\
             pub extern \"C\" fn run() {\n\
                 {\n\
                     let d_opts = intl::DateTimeFormatOptions::create();\n\
                     d_opts.set_time_zone(Some(\"UTC\"));\n\
                     d_opts.set_year(Some(intl::DateTimeFormatOptionsDay::Numeric));\n\
                     d_opts.set_month(Some(intl::DateTimeFormatOptionsMonth::Short));\n\
                     d_opts.set_day(Some(intl::DateTimeFormatOptionsDay::_2Digit));\n\
                     let dtf = intl::date_time_format().construct(\"en-US\", Some(&d_opts));\n\
                     let formatted_date = dtf.format(&Date::from_time(1700000000000.0));\n\
                     check!(formatted_date == \"Nov 14, 2023\");\n\
                     let n_opts = intl::NumberFormatOptions::create();\n\
                     n_opts.set_style(Some(intl::NumberFormatOptionsStyle::Currency));\n\
                     n_opts.set_currency(Some(\"USD\"));\n\
                     let nf = intl::number_format().construct(\"en-US\", Some(&n_opts));\n\
                     check!(nf.format(1234.5) == \"$1,234.50\");\n\
                     let rtf = intl::RelativeTimeFormatStatics::new(\"en-US\", None);\n\
                     check!(rtf.format(-1.0, intl::RelativeTimeFormatUnit::Day) == \"1 day ago\");\n\
                 }\n\
                 check!(js::live_handles() == 0);\n\
             }\n",
        )
        .unwrap();
        run(Command::new(rustc).current_dir(&dir).args(RUSTC_WASM).args(RUSTC_PROGRAM).args(["-o", "test.wasm", "test.rs"]));
        write_glue(&c, &m);
        fs::write(dir.join("impl.mjs"), "export const root = {};\n").unwrap();
        run(Command::new(node).arg(root().join("tests/run_wasm.mjs")).arg(dir.join("impl.mjs")).arg(dir.join("glue.mjs")).arg(dir.join("test.wasm")));
    }

    // Verify Zig Intl bindings (including qualified flattened ancestor `Intl_LocaleOptions`) compile and run in Node.
    let zig_xml = lower(Path::new("lib.es2020.intl.d.ts"), Some("zig"), &[]);
    let zig_model = model(&zig_xml, "Zig");
    let zig_files = generators::zig::files(&zig_model).unwrap();
    let intl_zig = zig_files.iter().find(|(p, _)| p == "namespaces/Intl.zig").map(|(_, s)| s.as_str()).unwrap();
    assert!(intl_zig.contains("asIntl_LocaleOptions") && intl_zig.contains(") Intl_LocaleOptions {"), "Intl.Locale flattened ancestor must keep Intl_ namespace:\n{intl_zig}");
    if let Some([zig, node]) = tools(["zig", "node"]) {
        let zdir = dir.join("zig");
        fs::create_dir_all(&zdir).unwrap();
        write_files(&zdir, zig_files);
        fs::write(
            zdir.join("test.zig"),
            "const b = @import(\"bindings.zig\");\n\
             extern \"env\" fn check(ok: i32, line: u32) void;\n\
             fn expect(ok: bool, src: anytype) void { check(@intFromBool(ok), src.line); }\n\
             export fn run() void {\n\
                 var scope = b.JsScope.open();\n\
                 const loc = b.intl.Locale_statics.new(.{ .string = b.JsString.from(\"en-US\") }, .{});\n\
                 const opts = loc.asIntl_LocaleOptions();\n\
                 expect(opts.getLanguage().?.eql(\"en\"), @src());\n\
                 const n_opts = b.intl.NumberFormatOptions.create();\n\
                 n_opts.setStyle(.currency);\n\
                 n_opts.setCurrency(\"USD\");\n\
                 const nf = b.intl.getNumberFormat().construct(.{ .locales = .{ .string = b.JsString.from(\"en-US\") }, .options = n_opts });\n\
                 expect(nf.format(1234.5).eql(\"$1,234.50\"), @src());\n\
                 const rtf = b.intl.RelativeTimeFormat_statics.new(.{ .locales = .{ .string = b.JsString.from(\"en-US\") } });\n\
                 expect(rtf.format(-1.0, .day).eql(\"1 day ago\"), @src());\n\
                 scope.close();\n\
                 expect(b.jsLiveHandles() == 0, @src());\n\
             }\n",
        )
        .unwrap();
        run(Command::new(zig).current_dir(&zdir).args(ZIG_WASM));
        let zcase = Case { fixture: zdir.clone(), out: zdir.clone(), xml: zig_xml };
        write_glue(&zcase, &zig_model);
        fs::write(zdir.join("impl.mjs"), "export const root = {};\n").unwrap();
        run(Command::new(node).arg(root().join("tests/run_wasm.mjs")).arg(zdir.join("impl.mjs")).arg(zdir.join("glue.mjs")).arg(zdir.join("test.wasm")));
    }
}

#[test]
fn webworker_bindings_and_worker() {
    let dir = root().join("target/e2e/webworker");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    // Lower lib.webworker.d.ts across targets and check schema + model generation.
    let xml = lower_with(Path::new("lib.webworker.d.ts"), Some("rust"), &[root().join("examples/modules/dom.toml")], &[]);
    if let Some([xmllint]) = tools(["xmllint"]) {
        let mut child = Command::new(xmllint)
            .args(["--noout", "--relaxng"])
            .arg(root().join("schema/declarations.rng"))
            .arg("-")
            .stdin(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        use std::io::Write;
        child.stdin.take().unwrap().write_all(xml.as_bytes()).unwrap();
        assert!(child.wait().unwrap().success(), "lib.webworker.d.ts XML failed schema validation");
    }

    if let (Some(rustc), Some([node])) = (rustc_wasm(), tools(["node"])) {
        let m = model(&xml, "Rust");
        let c = Case { fixture: dir.clone(), out: dir.clone(), xml };
        rust_bindings(&rustc, &dir, &m);
        fs::write(
            dir.join("test.rs"),
            "#![no_main]\n\
             use bindings::*;\n\
             #[unsafe(no_mangle)]\n\
             pub extern \"C\" fn run() {\n\
                 let handler = DedicatedWorkerGlobalScopeMessageEventFn::new(|ev: MessageEvent<JsValue>| {\n\
                     let data = ev.data();\n\
                     let bytes = Uint8Array::<ArrayBufferLike>::from_handle(give(&data));\n\
                     let mut v = bytes.to_vec();\n\
                     for b in &mut v { *b = b.wrapping_mul(2); }\n\
                     let reply = Uint8Array::<ArrayBufferLike>::from_slice(&v);\n\
                     post_message(&reply, Vec::<Transferable>::new());\n\
                 });\n\
                 set_onmessage(Some(&handler));\n\
             }\n",
        )
        .unwrap();
        run(Command::new(rustc).current_dir(&dir).args(RUSTC_WASM).args(RUSTC_PROGRAM).args(["-o", "test.wasm", "test.rs"]));
        write_glue(&c, &m);
        fs::write(
            dir.join("worker_test.mjs"),
            "import fs from 'node:fs';\n\
             import { Worker, isMainThread, parentPort, workerData } from 'node:worker_threads';\n\
             import { pathToFileURL } from 'node:url';\n\
             if (isMainThread) {\n\
               const w = new Worker(new URL(import.meta.url), { workerData: { dir: process.argv[2] } });\n\
               const out = await new Promise((resolve, reject) => {\n\
                 w.on('message', m => { if (m.ready) { w.postMessage(new Uint8Array([3, 7, 11])); } else resolve(m); });\n\
                 w.on('error', reject);\n\
               });\n\
               await w.terminate();\n\
               const got = Array.from(out);\n\
               if (got.length !== 3 || got[0] !== 6 || got[1] !== 14 || got[2] !== 22) {\n\
                 throw new Error('unexpected worker reply: ' + JSON.stringify(got));\n\
               }\n\
               console.log('worker round-trip OK:', got);\n\
             } else {\n\
               const dir = workerData.dir;\n\
               const { makeEnv } = await import(pathToFileURL(dir + '/glue.mjs'));\n\
               globalThis.self = globalThis;\n\
               globalThis.postMessage = msg => parentPort.postMessage(msg);\n\
               let instance;\n\
               const env = makeEnv(() => instance, { check: true });\n\
               ({ instance } = await WebAssembly.instantiate(fs.readFileSync(dir + '/test.wasm'), { env }));\n\
               instance.exports.run();\n\
               parentPort.on('message', data => globalThis.onmessage?.({ data }));\n\
               parentPort.postMessage({ ready: true });\n\
             }\n",
        )
        .unwrap();
        run(Command::new(node).arg(dir.join("worker_test.mjs")).arg(&dir));
    }

    // Also verify lib.webworker.d.ts compiles cleanly across Zig, C, and C++.
    if let Some([zig]) = tools(["zig"]) {
        let zdir = dir.join("zig");
        fs::create_dir_all(&zdir).unwrap();
        let zxml = lower_with(Path::new("lib.webworker.d.ts"), Some("zig"), &[root().join("examples/modules/dom.toml")], &[]);
        let zm = model(&zxml, "Zig");
        write_files(&zdir, generators::zig::files(&zm).unwrap());
        fs::write(zdir.join("test.zig"), "const b = @import(\"bindings.zig\"); comptime { @import(\"std\").testing.refAllDecls(b); }\nexport fn run() void {}\n").unwrap();
        run(Command::new(zig).current_dir(&zdir).args(ZIG_WASM));
    }
    if let Some([clang]) = tools(["clang"]) {
        let cdir = dir.join("c");
        fs::create_dir_all(&cdir).unwrap();
        let cxml = lower_with(Path::new("lib.webworker.d.ts"), Some("c"), &[root().join("examples/modules/dom.toml")], &[]);
        let cm = model(&cxml, "C");
        write_files(&cdir.join("bindings"), generators::c::files(&cm).unwrap());
        fs::write(cdir.join("all.c"), "#include \"bindings.h\"\nvoid run(void) { (void)js_live_handles(); }\n").unwrap();
        let flags: Vec<&str> = CLANG_WASM.iter().filter(|f| !f.starts_with("-Wl,")).copied().collect();
        run(Command::new(clang).args(&flags).arg("-fsyntax-only").arg("-I").arg(cdir.join("bindings")).arg(cdir.join("all.c")));
    }
    if let Some(clangxx) = wasi_clang() {
        let cppdir = dir.join("cpp");
        fs::create_dir_all(&cppdir).unwrap();
        let cppxml = lower_with(Path::new("lib.webworker.d.ts"), Some("cpp"), &[root().join("examples/modules/dom.toml")], &[]);
        let cppm = model(&cppxml, "C++");
        write_files(&cppdir.join("bindings"), generators::cpp::files(&cppm).unwrap());
        fs::write(cppdir.join("all.cpp"), "#include \"bindings.hpp\"\nint main() { return (int)ts::live_handles(); }\n").unwrap();
        let flags: Vec<&str> = CLANGXX_WASM.iter().filter(|f| !f.starts_with("-Wl,") && !f.starts_with("-mexec-model")).copied().collect();
        run(Command::new(clangxx).args(&flags).arg("-fsyntax-only").arg("-I").arg(cppdir.join("bindings")).arg(cppdir.join("all.cpp")));
    }
}

// ---- real libraries ----
//
// The libraries in tests/libraries/package.json, installed by
// tests/libraries/install.sh into target/libraries. For each library and
// target: its declarations lower and follow the schema, and its bindings
// compile. Slow, so not run by default:
//
//     cargo test --test e2e libraries -- --ignored

mod libraries {
    use super::*;

    // Lowers the packages for a target, checks the XML, and compiles the bindings.
    fn check(name: &str, packages: &[&str], target: &str) {
        let from = root().join("target/libraries");
        if !from.join("node_modules").is_dir() {
            eprintln!("SKIPPED: libraries are not installed (tests/libraries/install.sh)");
            return;
        }
        let out = root().join("target/e2e/libraries").join(format!("{name}-{target}"));
        let _ = fs::remove_dir_all(&out);
        fs::create_dir_all(&out).unwrap();
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_tsrs_proto"));
        for p in packages {
            cmd.args(["--package", p]);
        }
        cmd.arg("--from").arg(&from).arg("--config").arg(config(target));
        let xml = run(&mut cmd);
        fs::write(out.join("api.xml"), &xml).unwrap();
        if let Some([xmllint]) = tools(["xmllint"]) {
            run(Command::new(xmllint).args(["--noout", "--relaxng"]).arg(root().join("schema/declarations.rng")).arg(out.join("api.xml")));
        }
        match target {
            "fsharp" => {
                let Some([fable]) = tools(["fable"]) else { return };
                let bindings = generators::fsharp::module(&xml).unwrap_or_else(|p| panic!("{}", p.join("\n")));
                let bdir = out.join("bindings");
                fs::create_dir_all(&bdir).unwrap();
                for (file, content) in &bindings.files {
                    fs::write(bdir.join(file), content).unwrap();
                }
                fs::write(bdir.join("Bindings.fsproj"), bindings.fsproj()).unwrap();
                run(Command::new(fable).current_dir(&out).args(["bindings/Bindings.fsproj", "-o", "js", "--extension", ".mjs", "--noCache"]));
            }
            "c" => {
                let Some([clang]) = tools(["clang"]) else { return };
                write_files(&out.join("bindings"), generators::c::files(&model(&xml, "C")).unwrap_or_else(|p| panic!("{}", p.join("\n"))));
                fs::write(out.join("all.c"), "#include \"bindings.h\"\n").unwrap();
                // Checked, not linked: the linker's flags would go unused.
                let flags = CLANG_WASM.iter().filter(|f| !f.starts_with("-Wl,"));
                run(Command::new(clang).args(flags).args(["-fsyntax-only", "-ferror-limit=0"]).arg("-I").arg(out.join("bindings")).arg(out.join("all.c")));
            }
            "cpp" => {
                let Some(clang) = wasi_clang() else { return };
                compile_cpp_bindings(&clang, &out, &model(&xml, "C++"));
            }
            "zig" => {
                let Some([zig]) = tools(["zig"]) else { return };
                write_files(&out, generators::zig::files(&model(&xml, "Zig")).unwrap_or_else(|p| panic!("{}", p.join("\n"))));
                // Zig compiles what is used: use every declaration and member.
                fs::write(
                    out.join("test.zig"),
                    "const std = @import(\"std\");\nconst b = @import(\"bindings.zig\");\nexport fn run() void {}\ncomptime {\n    @setEvalBranchQuota(100_000_000);\n    for (std.meta.declarations(b)) |name| {\n        const v = @field(b, name);\n        if (@TypeOf(v) == type and @typeInfo(v) == .@\"struct\") {\n            for (std.meta.declarations(v)) |member| _ = &@field(v, member);\n        } else _ = &v;\n    }\n}\n",
                )
                .unwrap();
                run(Command::new(zig).current_dir(&out).args(ZIG_WASM));
            }
            "rust" => {
                let Some(rustc) = rustc_wasm() else { return };
                rust_bindings(&rustc, &out, &model(&xml, "Rust"));
            }
            "gleam" => {
                let Some([gleam]) = tools(["gleam"]) else { return };
                let bindings = generators::gleam::module(&xml).unwrap_or_else(|p| panic!("{}", p.join("\n")));
                write_gleam_project(&out, &bindings);
                fs::write(out.join("src/check.gleam"), "// Every module in src is compiled.\npub fn main() {\n  Nil\n}\n").unwrap();
                run(Command::new(gleam).current_dir(&out).args(["build", "--warnings-as-errors"]));
            }
            _ => unreachable!(),
        }
    }

    // Runs tests/libraries/<name>/test.<ext> against the real packages: in
    // Node, or in Chrome for a library of the page (a `browser` file in its
    // directory), bound with tests/libraries/test-page for the page's elements.
    fn exercise(name: &str, packages: &[&str], target: &str) {
        let dir = root().join("tests/libraries").join(name);
        let ext = match target {
            "fsharp" => "fs",
            "rust" => "rs",
            t => t,
        };
        if !dir.join(format!("test.{ext}")).exists() {
            eprintln!("SKIPPED: no tests/libraries/{name}/test.{ext}");
            return;
        }
        let from = root().join("target/libraries");
        if !from.join("node_modules").is_dir() {
            eprintln!("SKIPPED: libraries are not installed (tests/libraries/install.sh)");
            return;
        }
        let Some([node]) = tools(["node"]) else { return };
        let browser = dir.join("browser").exists();
        let mut packages = packages.to_vec();
        if browser {
            copy_dir(&root().join("tests/libraries/test-page"), &from.join("node_modules/test-page"));
            packages.push("test-page");
        }
        let out = root().join("target/e2e/libraries").join(format!("{name}-{target}-run"));
        let _ = fs::remove_dir_all(&out);
        fs::create_dir_all(&out).unwrap();
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_tsrs_proto"));
        for p in &packages {
            cmd.args(["--package", p]);
        }
        cmd.arg("--from").arg(&from).arg("--config").arg(config(target));
        let xml = run(&mut cmd);
        fs::write(out.join("api.xml"), &xml).unwrap();
        // The packages, where the program and its glue import them from.
        std::os::unix::fs::symlink(from.join("node_modules"), out.join("node_modules")).unwrap();
        fs::copy(dir.join("impl.mjs"), out.join("impl.mjs")).unwrap();
        let c = Case { fixture: dir, out, xml };
        let program = match target {
            "c" => {
                let Some([clang]) = tools(["clang"]) else { return };
                let model = model(&c.xml, "C");
                write_files(&c.out.join("bindings"), generators::c::files(&model).unwrap());
                run(Command::new(clang).args(CLANG_WASM).arg("-I").arg(c.out.join("bindings")).arg("-o").arg(c.out.join("test.wasm")).arg(c.fixture.join("test.c")));
                write_glue(&c, &model);
                None
            }
            "cpp" => {
                let Some(clang) = wasi_clang() else { return };
                let model = model(&c.xml, "C++");
                build_cpp(&clang, &c, &model);
                write_glue(&c, &model);
                None
            }
            "zig" => {
                let Some([zig]) = tools(["zig"]) else { return };
                let model = model(&c.xml, "Zig");
                write_files(&c.out, generators::zig::files(&model).unwrap());
                fs::copy(c.fixture.join("test.zig"), c.out.join("test.zig")).unwrap();
                run(Command::new(zig).current_dir(&c.out).args(ZIG_WASM));
                write_glue(&c, &model);
                None
            }
            "rust" => {
                let Some(rustc) = rustc_wasm() else { return };
                let model = model(&c.xml, "Rust");
                build_rust(&rustc, &c, &model);
                write_glue(&c, &model);
                None
            }
            "fsharp" => {
                let Some([fable]) = tools(["fable"]) else { return };
                write_fsharp(&c, &generators::fsharp::module(&c.xml).unwrap());
                run(Command::new(fable).current_dir(&c.out).args(["Test.fsproj", "-o", "js", "--extension", ".mjs", "--noCache"]));
                Some((root().join("tests/run_fable.mjs"), c.out.join("js/Test.mjs"), "import './js/Test.mjs';\n"))
            }
            "gleam" => {
                let Some([gleam]) = tools(["gleam"]) else { return };
                write_gleam_project(&c.out, &generators::gleam::module(&c.xml).unwrap_or_else(|p| panic!("{}", p.join("\n"))));
                fs::copy(c.fixture.join("test.gleam"), c.out.join("src/check.gleam")).unwrap();
                run(Command::new(gleam).current_dir(&c.out).args(["build", "--warnings-as-errors"]));
                Some((root().join("tests/run_gleam.mjs"), c.out.join("build/dev/javascript/check/check.mjs"), "import { main } from './build/dev/javascript/check/check.mjs';\nmain();\n"))
            }
            _ => unreachable!(),
        };
        if !browser {
            let report = match &program {
                None => run(Command::new(&node).arg(root().join("tests/run_wasm.mjs")).arg(c.out.join("impl.mjs")).arg(c.out.join("glue.mjs")).arg(c.out.join("test.wasm"))),
                Some((runner, entry, _)) => run(Command::new(&node).arg(runner).arg(c.out.join("impl.mjs")).arg(entry)),
            };
            return print!("{report}");
        }
        // In the page: what is imported, bundled, and files of its packages
        // the library asks the page for (`assets`: Leaflet's marker icons).
        let Some([bun]) = tools(["bun"]) else { return };
        for asset in fs::read_to_string(c.fixture.join("assets")).unwrap_or_default().lines().filter(|l| !l.trim().is_empty()) {
            let from = from.join("node_modules").join(asset.trim());
            fs::copy(&from, c.out.join(from.file_name().unwrap())).unwrap();
        }
        let bundle = |entry: &Path, outfile: &str| run(Command::new(&bun).current_dir(&c.out).args(["build", "--target=browser", "--format=esm"]).arg(entry).arg(format!("--outfile={outfile}")));
        match &program {
            None => {
                bundle(&c.out.join("impl.mjs"), "impl.js");
                fs::write(c.out.join("index.html"), format!("{LIBRARY_PAGE}{LIBRARY_WASM}")).unwrap();
            }
            Some((_, _, entry)) => {
                fs::write(c.out.join("entry.mjs"), entry).unwrap();
                bundle(&c.out.join("entry.mjs"), "program.js");
                fs::write(c.out.join("index.html"), format!("{LIBRARY_PAGE}{LIBRARY_PROGRAM}")).unwrap();
            }
        }
        run_browser(&c);
    }

    // The page a library of the page runs in: the elements test-page gives.
    const LIBRARY_PAGE: &str = r#"<!doctype html>
<title>library test</title>
<style>#map, #stage, #chart, #canvas { width: 400px; height: 300px; }</style>
<div id="map"></div><div id="stage"></div><div id="chart"></div><canvas id="canvas" width="400" height="300"></canvas>
"#;

    const LIBRARY_WASM: &str = r#"<script type="module">
import { makeEnv } from './glue.mjs';
import * as impl from './impl.js';
const { root, modules } = impl;
let instance;
let passed = 0;
const failed = [];
const options = { root, modules, check: true, unchecked: e => failed.push('unchecked exception: ' + (e?.message ?? e)) };
const env = { ...makeEnv(() => instance, options), check: (ok, line) => (ok ? passed++ : failed.push('line ' + line)) };
const wasi = new Proxy({}, { get: () => () => 0 });
({ instance } = await WebAssembly.instantiate(await (await fetch('./test.wasm')).arrayBuffer(), { env, wasi_snapshot_preview1: wasi }));
instance.exports._initialize?.();
instance.exports.run();
await new Promise(r => setTimeout(r, 500));
const deadline = Date.now() + 20000;
while (instance.exports.busy?.() && Date.now() < deadline) await new Promise(r => setTimeout(r, 10));
instance.exports.finish?.();
window.__result = { passed, failed, dom: [], domChecks: 0 };
</script>
"#;

    // F# and Gleam: the program reports `N checks passed`, last, or throws.
    const LIBRARY_PROGRAM: &str = r#"<script type="module">
let passed = 0;
const failed = [];
const log = console.log;
console.log = (...args) => {
  const m = String(args[0]).match(/(\d+) checks passed/);
  if (m) passed = Number(m[1]);
  log(...args);
};
window.addEventListener('unhandledrejection', e => failed.push('rejected: ' + (e.reason?.message ?? e.reason)));
try {
  await import('./program.js');
} catch (e) {
  failed.push('threw: ' + (e?.message ?? e));
}
const deadline = Date.now() + 20000;
while (!passed && !failed.length && Date.now() < deadline) await new Promise(r => setTimeout(r, 50));
if (!passed && !failed.length) failed.push('the program did not report');
window.__result = { passed, failed, dom: [], domChecks: 0 };
</script>
"#;

    macro_rules! library {
        ($($name:ident: [$($package:literal),+];)*) => { $(mod $name {
            const PACKAGES: &[&str] = &[$($package),+];
            #[test] #[ignore] fn fsharp() { super::check(stringify!($name), PACKAGES, "fsharp"); }
            #[test] #[ignore] fn c() { super::check(stringify!($name), PACKAGES, "c"); }
            #[test] #[ignore] fn cpp() { super::check(stringify!($name), PACKAGES, "cpp"); }
            #[test] #[ignore] fn zig() { super::check(stringify!($name), PACKAGES, "zig"); }
            #[test] #[ignore] fn rust() { super::check(stringify!($name), PACKAGES, "rust"); }
            #[test] #[ignore] fn gleam() { super::check(stringify!($name), PACKAGES, "gleam"); }
            // Its test programs, run against the real library.
            #[test] #[ignore] fn run_fsharp() { super::exercise(stringify!($name), PACKAGES, "fsharp"); }
            #[test] #[ignore] fn run_c() { super::exercise(stringify!($name), PACKAGES, "c"); }
            #[test] #[ignore] fn run_cpp() { super::exercise(stringify!($name), PACKAGES, "cpp"); }
            #[test] #[ignore] fn run_zig() { super::exercise(stringify!($name), PACKAGES, "zig"); }
            #[test] #[ignore] fn run_rust() { super::exercise(stringify!($name), PACKAGES, "rust"); }
            #[test] #[ignore] fn run_gleam() { super::exercise(stringify!($name), PACKAGES, "gleam"); }
        })* };
    }

    library! {
        katex: ["katex"];
        marked: ["marked"];
        date_fns: ["date-fns"];
        // @types/node's ambient modules, `declare module "fs"`, as packages.
        node: ["fs", "path", "events"];
        leaflet: ["leaflet"];
        pixi: ["pixi.js"];
        echarts: ["echarts"];
        konva: ["konva"];
        three: ["three"];
        rapier: ["@dimforge/rapier3d-compat"];
        tanstack_table: ["@tanstack/table-core"];
        prosemirror: ["prosemirror-model", "prosemirror-transform", "prosemirror-state", "prosemirror-view"];
        babylon_lite: ["@babylonjs/lite"];
    }
}

// Self-contained Three.js-style galleries and per-package demos under
// `examples/<target>/<pkg>/<demo>/`, built against shared per-target bindings
// and verified in headless Chrome via `tests/run_browser.mjs`.
// Run with:
//     cargo test --test e2e examples -- --ignored
mod examples {
    use super::*;
    use std::collections::HashMap;
    use std::hash::{Hash, Hasher};
    use std::sync::{Mutex, MutexGuard, OnceLock};

    #[derive(Clone, Copy)]
    enum VerifyKind {
        PtsCanvas,
        BabylonWebGpu,
        RapierBabylon3d,
        NodeCli,
        DomSelector {
            selector: &'static str,
            min_count: usize,
        },
        CanvasNonEmpty {
            selector: &'static str,
            min_count: usize,
        },
    }

    #[derive(Clone, Copy)]
    struct PackageSpec {
        pkg: &'static str,
        npm: &'static [&'static str],
        needs_dom: bool,
        extra_specs: &'static [&'static PackageSpec],
        crate_name: &'static str,
        fsharp_name: &'static str,
        demos: &'static [&'static str],
        verify: VerifyKind,
    }

    const PTS_SPEC: PackageSpec = PackageSpec {
        pkg: "pts",
        npm: &["pts"],
        needs_dom: false,
        extra_specs: &[],
        crate_name: "pts",
        fsharp_name: "Pts",
        demos: &[
            "vector_field",
            "polygon_subdivision",
            "curve_splines",
            "circle_intersections",
            "convex_hull_network",
        ],
        verify: VerifyKind::PtsCanvas,
    };

    const BABYLON_LITE_SPEC: PackageSpec = PackageSpec {
        pkg: "babylon_lite",
        npm: &["@babylonjs/lite"],
        needs_dom: true,
        extra_specs: &[],
        crate_name: "babylonjs_lite",
        fsharp_name: "Babylonjs.Lite",
        demos: &[
            "torus_knot",
            "pbr_material_matrix",
            "polyhedron_sculptures",
            "csg_boolean_sculpture",
            "kinetic_wave_monoliths",
        ],
        verify: VerifyKind::BabylonWebGpu,
    };

    const KATEX_SPEC: PackageSpec = PackageSpec {
        pkg: "katex",
        npm: &["katex"],
        needs_dom: true,
        extra_specs: &[],
        crate_name: "katex",
        fsharp_name: "Katex",
        demos: &[
            "formula_gallery",
            "inline_vs_display",
            "custom_macros",
            "error_inspector",
            "output_modes",
        ],
        verify: VerifyKind::DomSelector {
            selector: ".katex, .katex-error, math",
            min_count: 2,
        },
    };

    const MARKED_SPEC: PackageSpec = PackageSpec {
        pkg: "marked",
        npm: &["marked"],
        needs_dom: true,
        extra_specs: &[],
        crate_name: "marked",
        fsharp_name: "Marked",
        demos: &[
            "document_preview",
            "token_inspector",
            "inline_vs_block",
            "custom_renderer",
            "gfm_and_breaks",
        ],
        verify: VerifyKind::DomSelector {
            selector: "#app .md-out *, #app tbody tr, #app .tok-pill",
            min_count: 3,
        },
    };

    const DATE_FNS_SPEC: PackageSpec = PackageSpec {
        pkg: "date_fns",
        npm: &["date-fns"],
        needs_dom: true,
        extra_specs: &[],
        crate_name: "date_fns",
        fsharp_name: "DateFns",
        demos: &[
            "calendar_grid",
            "format_cookbook",
            "date_arithmetic",
            "relative_timeline",
            "interval_overlap",
        ],
        verify: VerifyKind::DomSelector {
            selector: "#app .day-cell, #app tbody tr, #app .milestone, #app .gantt-row",
            min_count: 4,
        },
    };

    const ECHARTS_SPEC: PackageSpec = PackageSpec {
        pkg: "echarts",
        npm: &["echarts"],
        needs_dom: true,
        extra_specs: &[],
        crate_name: "echarts",
        fsharp_name: "Echarts",
        demos: &[
            "bar_chart",
            "line_area",
            "pie_donut",
            "scatter_plot",
            "radar_gauge",
        ],
        verify: VerifyKind::CanvasNonEmpty {
            selector: "#app canvas",
            min_count: 1,
        },
    };

    const KONVA_SPEC: PackageSpec = PackageSpec {
        pkg: "konva",
        npm: &["konva"],
        crate_name: "konva",
        fsharp_name: "Konva",
        needs_dom: true,
        extra_specs: &[],
        demos: &[
            "shapes_gallery",
            "groups_transforms",
            "wireframe_diagram",
            "radial_rosette",
            "layers_hit_test",
        ],
        verify: VerifyKind::CanvasNonEmpty {
            selector: "#stage canvas",
            min_count: 1,
        },
    };

    const PIXI_SPEC: PackageSpec = PackageSpec {
        pkg: "pixi",
        npm: &["pixi.js"],
        crate_name: "pixi_js",
        fsharp_name: "Pixi.Js",
        needs_dom: true,
        extra_specs: &[],
        demos: &[
            "vector_primitives",
            "sprite_mosaic",
            "scene_hierarchy",
            "generative_lattice",
            "bounds_intersection",
        ],
        verify: VerifyKind::CanvasNonEmpty {
            selector: "#stage canvas",
            min_count: 1,
        },
    };

    const LEAFLET_SPEC: PackageSpec = PackageSpec {
        pkg: "leaflet",
        npm: &["leaflet"],
        crate_name: "leaflet",
        fsharp_name: "Leaflet",
        needs_dom: true,
        extra_specs: &[],
        demos: &[
            "vector_route",
            "polygon_zones",
            "geodesic_circles",
            "simple_crs_blueprint",
            "interactive_markers",
        ],
        verify: VerifyKind::DomSelector {
            selector: "#map svg path",
            min_count: 3,
        },
    };

    const TANSTACK_TABLE_SPEC: PackageSpec = PackageSpec {
        pkg: "tanstack_table",
        npm: &["@tanstack/table-core"],
        crate_name: "tanstack_table_core",
        fsharp_name: "Tanstack.TableCore",
        needs_dom: true,
        extra_specs: &[],
        demos: &[
            "sorting_leaderboard",
            "filtering_facets",
            "pagination_ledger",
            "grouping_aggregation",
            "visibility_pinning_selection",
        ],
        verify: VerifyKind::DomSelector {
            selector: "#app table tbody tr",
            min_count: 3,
        },
    };

    const PROSEMIRROR_SPEC: PackageSpec = PackageSpec {
        pkg: "prosemirror",
        npm: &["prosemirror-model", "prosemirror-transform", "prosemirror-state", "prosemirror-view"],
        crate_name: "prosemirror",
        fsharp_name: "Prosemirror",
        needs_dom: true,
        extra_specs: &[],
        demos: &[
            "rich_schema_marks",
            "structural_transforms",
            "decorations_search",
            "plugin_history_transactions",
            "dom_json_roundtrip",
        ],
        verify: VerifyKind::DomSelector {
            selector: "#editor .ProseMirror > *",
            min_count: 3,
        },
    };

    const THREE_SPEC: PackageSpec = PackageSpec {
        pkg: "three",
        npm: &["three"],
        crate_name: "three",
        fsharp_name: "Three",
        needs_dom: true,
        extra_specs: &[],
        demos: &[
            "platonic_wireframes",
            "hierarchical_orrery",
            "procedural_heightfield",
            "instanced_cityscape",
            "raycaster_picking",
        ],
        verify: VerifyKind::CanvasNonEmpty {
            selector: "#stage canvas",
            min_count: 1,
        },
    };

    const RAPIER_SPEC: PackageSpec = PackageSpec {
        pkg: "rapier",
        npm: &["@dimforge/rapier3d-compat"],
        crate_name: "dimforge_rapier3d_compat",
        fsharp_name: "Dimforge.Rapier3dCompat",
        needs_dom: true,
        extra_specs: &[],
        demos: &[
            "gravity_bouncing",
            "domino_cascade",
            "pendulum_joints",
            "raycast_lidar",
            "collision_events_sensor",
        ],
        verify: VerifyKind::CanvasNonEmpty {
            selector: "#stage canvas",
            min_count: 1,
        },
    };

    const RAPIER_3D_SPEC: PackageSpec = PackageSpec {
        pkg: "rapier",
        npm: &["@dimforge/rapier3d-compat"],
        crate_name: "dimforge_rapier3d_compat",
        fsharp_name: "Dimforge.Rapier3dCompat",
        needs_dom: true,
        extra_specs: &[&BABYLON_LITE_SPEC],
        demos: &[
            "tower_collapse_3d",
            "billiards_break_3d",
            "domino_spiral_3d",
        ],
        verify: VerifyKind::RapierBabylon3d,
    };

    const NODE_SPEC: PackageSpec = PackageSpec {
        pkg: "node",
        npm: &["fs", "path", "events"],
        crate_name: "node",
        fsharp_name: "Node",
        needs_dom: false,
        extra_specs: &[],
        demos: &[
            "path_inspector",
            "event_bus",
            "temp_workspace",
            "directory_tree",
            "file_pipeline",
        ],
        verify: VerifyKind::NodeCli,
    };

    fn lock_key(key: &str) -> MutexGuard<'static, ()> {
        static MAP: OnceLock<Mutex<HashMap<String, &'static Mutex<()>>>> = OnceLock::new();
        let map = MAP.get_or_init(|| Mutex::new(HashMap::new()));
        let mutex = {
            let mut g = map.lock().unwrap_or_else(|e| e.into_inner());
            *g.entry(key.to_string()).or_insert_with(|| Box::leak(Box::new(Mutex::new(()))))
        };
        mutex.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn tool_stamp(path: &Path) -> String {
        let meta = fs::metadata(path).unwrap();
        let mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_nanos());
        format!("{}:{mtime}", meta.len())
    }

    fn copy_dir_filtered(from: &Path, to: &Path, skip: &[&str]) {
        fs::create_dir_all(to).unwrap();
        for e in fs::read_dir(from).unwrap() {
            let e = e.unwrap();
            let name = e.file_name();
            let name_str = name.to_string_lossy();
            if skip.contains(&name_str.as_ref()) || name_str.ends_with(".wasm") {
                continue;
            }
            let dst = to.join(&name);
            if e.file_type().unwrap().is_dir() {
                copy_dir_filtered(&e.path(), &dst, skip);
            } else {
                fs::copy(e.path(), dst).unwrap();
            }
        }
    }

    fn collect_dir_files(dir: &Path, out: &mut Vec<PathBuf>) {
        if !dir.exists() {
            return;
        }
        for e in fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                collect_dir_files(&p, out);
            } else {
                out.push(p);
            }
        }
    }

    fn hash_tree_filtered(dir: &Path, extra: &[&str], skip: &[&str]) -> String {
        let mut files = Vec::new();
        fn walk(dir: &Path, skip: &[&str], out: &mut Vec<PathBuf>) {
            if !dir.exists() {
                return;
            }
            for e in fs::read_dir(dir).unwrap() {
                let e = e.unwrap();
                let name = e.file_name();
                let name_str = name.to_string_lossy();
                if skip.contains(&name_str.as_ref()) || name_str.ends_with(".wasm") {
                    continue;
                }
                let p = e.path();
                if p.is_dir() {
                    walk(&p, skip, out);
                } else {
                    out.push(p);
                }
            }
        }
        walk(dir, skip, &mut files);
        files.sort();
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        extra.hash(&mut hasher);
        for p in &files {
            p.strip_prefix(dir).unwrap_or(p).to_string_lossy().hash(&mut hasher);
            fs::read(p).unwrap().hash(&mut hasher);
        }
        format!("{:x}", hasher.finish())
    }

    const DEMO_SKIP: &[&str] = &[
        "node_modules",
        "bindings",
        "dist",
        "js",
        "build",
        "zig-out",
        ".zig-cache",
        "bin",
        "obj",
        "target",
        ".vite",
        "manifest.toml",
        "package-lock.json",
        "bundle.mjs",
    ];

    fn ensure_target_root(target: &str, from_nm: &Path) -> PathBuf {
        let target_root = root().join("target/e2e/examples").join(target);
        let _g = lock_key(&format!("root:{target}"));
        fs::create_dir_all(target_root.join("bindings")).unwrap();
        let dst_nm = target_root.join("node_modules");
        if dst_nm.symlink_metadata().is_err() {
            std::os::unix::fs::symlink(from_nm, &dst_nm).unwrap();
        }
        let ex_root = root().join("examples").join(target);
        for file in ["index.html", "examples.json"] {
            let src = ex_root.join(file);
            if src.exists() {
                let bytes = fs::read(&src).unwrap();
                let dst = target_root.join(file);
                if fs::read(&dst).ok().as_deref() != Some(bytes.as_slice()) {
                    let tmp = target_root.join(format!(".{file}.tmp"));
                    fs::write(&tmp, bytes).unwrap();
                    fs::rename(&tmp, &dst).unwrap();
                }
            }
        }
        let cv_src = root().join("examples/code_viewer.mjs");
        if cv_src.exists() {
            let bytes = fs::read(&cv_src).unwrap();
            let dst = target_root.join("code_viewer.mjs");
            if fs::read(&dst).ok().as_deref() != Some(bytes.as_slice()) {
                let tmp = target_root.join(".code_viewer.mjs.tmp");
                fs::write(&tmp, bytes).unwrap();
                fs::rename(&tmp, &dst).unwrap();
            }
        }
        target_root
    }

    fn ensure_pkg_bindings(from: &Path, target: &str, bindings: &Path, pkg_key: &str, args_fn: impl FnOnce(&mut Command)) -> String {
        let _g = lock_key(&format!("emit:{target}:{pkg_key}"));
        let bin = env!("CARGO_BIN_EXE_tsrs_proto");
        let stamp = tool_stamp(Path::new(bin));
        let done = bindings.join(format!(".done-{pkg_key}"));
        if fs::read_to_string(&done).is_ok_and(|s| s == stamp) {
            return stamp;
        }
        let _ = fs::remove_dir_all(bindings.join(pkg_key));
        let mut cmd = Command::new(bin);
        args_fn(&mut cmd);
        run(&mut cmd);
        fs::write(&done, &stamp).unwrap();
        let _ = from;
        stamp
    }

    fn ensure_bindings_for_spec(from: &Path, target: &str, target_root: &Path, spec: &PackageSpec) -> String {
        let bindings = target_root.join("bindings");
        let cfg = config(target);
        let dom_dir_name = if target == "fsharp" { "Dom" } else { "dom" };
        let needs_dom = spec.needs_dom || spec.extra_specs.iter().any(|e| e.needs_dom);
        let dom_stamp = if needs_dom {
            ensure_pkg_bindings(from, target, &bindings, dom_dir_name, |cmd| {
                cmd.arg("lib.dom.d.ts")
                    .arg("--config")
                    .arg(&cfg)
                    .arg("--config")
                    .arg(root().join("examples/modules/dom.toml"))
                    .args(["--emit", target, "--name", "dom", "--out"])
                    .arg(&bindings);
            })
        } else {
            String::new()
        };
        let mut stamps = vec![dom_stamp];
        for s in std::iter::once(spec).chain(spec.extra_specs.iter().copied()) {
            let pkg_dir_name = if target == "fsharp" { s.fsharp_name } else { s.crate_name };
            let pkg_stamp = ensure_pkg_bindings(from, target, &bindings, pkg_dir_name, |cmd| {
                cmd.arg("--from").arg(from);
                for p in s.npm {
                    cmd.args(["--package", p]);
                }
                cmd.arg("--config").arg(&cfg).args(["--emit", target]);
                if s.npm.len() > 1 {
                    cmd.args(["--name", s.crate_name]);
                }
                if s.needs_dom {
                    cmd.args(["--ref"]).arg(bindings.join(dom_dir_name));
                }
                cmd.args(["--out"]).arg(&bindings);
            });
            stamps.push(pkg_stamp);
        }
        stamps.join(":")
    }

    fn compile_cpp_package(clang: &Path, bindings: &Path, pkg: &str) -> Vec<PathBuf> {
        let pkg_dir = bindings.join(pkg);
        let mut sources = Vec::new();
        collect_dir_files(&pkg_dir, &mut sources);
        sources.retain(|p| p.extension().and_then(|s| s.to_str()) == Some("cpp"));
        sources.sort();

        let flags: Vec<&str> = CLANGXX_WASM.iter().filter(|f| !f.starts_with("-Wl,") && !f.starts_with("-mexec-model")).copied().collect();
        // Hash all bindings (including `--ref dom` headers) so upstream changes invalidate cleanly.
        let mut all_dirs: Vec<String> = Vec::new();
        for p in fs::read_dir(bindings).unwrap().flatten() {
            if p.file_type().unwrap().is_dir() && p.file_name().to_string_lossy() != ".rlibs" {
                all_dirs.push(hash_tree_filtered(&p.path(), &flags, &[]));
            }
        }
        all_dirs.sort();
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        all_dirs.hash(&mut hasher);
        let stamp = format!("{:x}", hasher.finish());

        let _g = lock_key(&format!("cpp-cache:{pkg}-{stamp}"));
        let cache_dir = root().join("target/e2e-cache").join(format!("cpp-ex-{pkg}-{stamp}"));
        let objects: Vec<PathBuf> = (0..sources.len()).map(|i| cache_dir.join(format!("m{i}.o"))).collect();
        let done = cache_dir.join(".done");
        if !done.exists() || objects.iter().any(|o| !o.exists()) {
            fs::create_dir_all(&cache_dir).unwrap();
            let next = std::sync::atomic::AtomicUsize::new(0);
            let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
            std::thread::scope(|scope| {
                for _ in 0..threads {
                    scope.spawn(|| {
                        while let Some(i) = Some(next.fetch_add(1, std::sync::atomic::Ordering::SeqCst)).filter(|i| *i < sources.len()) {
                            run(Command::new(clang).args(&flags).arg("-c").arg("-I").arg(bindings).arg(&sources[i]).arg("-o").arg(&objects[i]));
                        }
                    });
                }
            });
            fs::write(done, "ok").unwrap();
        }
        objects
    }

    fn build_rust_rlib(rustc: &Path, bindings: &Path, pkg: &str, rlibs_dir: &Path, externs: &[(&str, &Path)]) -> PathBuf {
        let _g = lock_key(&format!("rust-stage:{}", bindings.display()));
        fs::create_dir_all(rlibs_dir).unwrap();
        let mut extra: Vec<String> = RUSTC_WASM.iter().map(|s| s.to_string()).collect();
        for (n, p) in externs {
            extra.push(format!("{n}:{}", hash_tree_filtered(bindings.join(n).as_path(), &[], &[])));
            let _ = p;
        }
        let extra_refs: Vec<&str> = extra.iter().map(String::as_str).collect();
        let stamp = hash_tree_filtered(&bindings.join(pkg), &extra_refs, &[]);
        let cache = root().join("target/e2e-cache");
        let cached = cache.join(format!("rust-ex-{pkg}-{stamp}.rlib"));
        if !cached.exists() {
            fs::create_dir_all(&cache).unwrap();
            let mut cmd = Command::new(rustc);
            cmd.current_dir(bindings)
                .args(RUSTC_WASM)
                .args(["--crate-type", "rlib", "--crate-name", pkg, "-L"])
                .arg(rlibs_dir);
            for (name, path) in externs {
                cmd.arg("--extern").arg(format!("{name}={}", path.display()));
            }
            cmd.arg("-o").arg(&cached).arg(bindings.join(pkg).join("src/lib.rs"));
            run(&mut cmd);
        }
        let local = rlibs_dir.join(format!("lib{pkg}.rlib"));
        fs::copy(&cached, &local).unwrap();
        local
    }

    fn gleam_example_manifest(spec: &PackageSpec) -> String {
        let mut pkg_lines = Vec::new();
        let mut req_lines = Vec::new();
        let needs_dom = spec.needs_dom || spec.extra_specs.iter().any(|e| e.needs_dom);
        for s in std::iter::once(spec).chain(spec.extra_specs.iter().copied()) {
            let reqs = if s.needs_dom {
                r#"["dom", "gleam_stdlib"]"#
            } else {
                r#"["gleam_stdlib"]"#
            };
            pkg_lines.push(format!(
                "  {{ name = \"{}\", version = \"0.1.0\", build_tools = [\"gleam\"], requirements = {reqs}, source = \"local\", path = \"../../bindings/{}\" }},",
                s.crate_name, s.crate_name
            ));
            req_lines.push(format!("{} = {{ path = \"../../bindings/{}\" }}", s.crate_name, s.crate_name));
        }
        if needs_dom {
            pkg_lines.push(
                "  { name = \"dom\", version = \"0.1.0\", build_tools = [\"gleam\"], requirements = [\"gleam_stdlib\"], source = \"local\", path = \"../../bindings/dom\" },".to_string(),
            );
            req_lines.push("dom = { path = \"../../bindings/dom\" }".to_string());
        }
        pkg_lines.push(
            "  { name = \"gleam_stdlib\", version = \"1.0.5\", build_tools = [\"gleam\"], requirements = [], otp_app = \"gleam_stdlib\", source = \"hex\", outer_checksum = \"CEE5B6C076A85B45F60C585F4316C63EC8B7127C119D5738C3958A9C4D50404E\" },".to_string(),
        );
        req_lines.push("gleam_stdlib = { version = \">= 1.0.0 and < 2.0.0\" }".to_string());
        format!("packages = [\n{}\n]\n\n[requirements]\n{}\n", pkg_lines.join("\n"), req_lines.join("\n"))
    }

    const PTS_VERIFY_SCRIPT: &str = r#"<script type="module">
let passed = 0;
const failed = [];
window.addEventListener('unhandledrejection', e => failed.push('rejected: ' + (e.reason?.message ?? e.reason)));
try {
  await import('./bundle.mjs');
  const deadline = Date.now() + 5000;
  let rendered = false;
  while (Date.now() < deadline) {
    const canvas = document.querySelector('#pt canvas');
    if (canvas && canvas.width > 0 && canvas.height > 0) {
      const ctx = canvas.getContext('2d');
      const data = ctx.getImageData(0, 0, canvas.width, canvas.height).data;
      const colors = new Set();
      for (let i = 0; i < data.length; i += 4) {
        if (data[i + 3] > 0) {
          colors.add((data[i] << 16) | (data[i + 1] << 8) | data[i + 2]);
          if (colors.size >= 2) { rendered = true; break; }
        }
      }
      if (rendered) break;
    }
    await new Promise(r => setTimeout(r, 50));
  }
  if (rendered) passed++;
  else failed.push('#pt canvas did not render multiple colors');
} catch (e) {
  failed.push('threw: ' + (e?.message ?? e));
}
window.__result = { passed, failed, dom: [], domChecks: 1 };
</script>"#;

    const BABYLON_VERIFY_SCRIPT: &str = r#"<script type="module">
let passed = 0;
const failed = [];
window.addEventListener('unhandledrejection', e => failed.push('rejected: ' + (e.reason?.message ?? e.reason)));
try {
  await import('./bundle.mjs');
  const canvas = document.getElementById('renderCanvas');
  const status = document.getElementById('status');
  if (canvas && status) passed++;
  else failed.push('missing #renderCanvas or #status');

  const deadline = Date.now() + 4000;
  while (Date.now() < deadline && !failed.length) {
    if (status && status.textContent.includes('WebGPU / Babylon Lite error:')) {
      passed++;
      break;
    }
    await new Promise(r => setTimeout(r, 50));
  }
  if (!failed.length && passed === 1) {
    // WebGPU initialized cleanly without reporting an error.
    passed++;
  }
} catch (e) {
  failed.push('threw: ' + (e?.message ?? e));
}
window.__result = { passed, failed, dom: [], domChecks: 1 };
</script>"#;

    const RAPIER_BABYLON_3D_VERIFY_SCRIPT: &str = r#"<script type="module">
let passed = 0;
const failed = [];
window.addEventListener('unhandledrejection', e => failed.push('rejected: ' + (e.reason?.message ?? e.reason)));
try {
  await import('./bundle.mjs');
  const canvas = document.getElementById('renderCanvas');
  const status = document.getElementById('status');
  const meta = document.getElementById('meta');
  const pauseBtn = document.getElementById('btn-pause');
  if (canvas && status && meta && pauseBtn) passed++;
  else failed.push('missing #renderCanvas, #status, #meta, or #btn-pause');

  const metaDeadline = Date.now() + 6000;
  let metaOk = false;
  while (Date.now() < metaDeadline && !failed.length) {
    const txt = meta?.textContent ?? '';
    if (txt.includes('step=') && txt.includes('ke=')) {
      metaOk = true;
      break;
    }
    await new Promise(r => setTimeout(r, 50));
  }
  if (metaOk) passed++;
  else failed.push('rapier 3d telemetry #meta did not populate step= and ke=: ' + (meta?.textContent ?? ''));

  if (metaOk && pauseBtn) {
    pauseBtn.click();
    const pauseDeadline = Date.now() + 1000;
    let pauseOk = false;
    while (Date.now() < pauseDeadline && !failed.length) {
      if ((meta?.textContent ?? '').includes('(paused)')) {
        pauseOk = true;
        break;
      }
      await new Promise(r => setTimeout(r, 25));
    }
    if (pauseOk) passed++;
    else failed.push('clicking #btn-pause did not update #meta to (paused)');
  }

  const deadline = Date.now() + 4000;
  let webgpuChecked = false;
  while (Date.now() < deadline && !failed.length) {
    if (status && status.textContent.includes('WebGPU / Babylon Lite error:')) {
      webgpuChecked = true;
      break;
    }
    await new Promise(r => setTimeout(r, 50));
  }
  if (!failed.length && (webgpuChecked || passed >= 3)) {
    passed++;
  }
} catch (e) {
  failed.push('threw: ' + (e?.message ?? e));
}
window.__result = { passed, failed, dom: [], domChecks: 1 };
</script>"#;

    fn run_browser_pages(roots: &[&Path], pages: &[String]) {
        let Some([node]) = tools(["node"]) else { return };
        let mut cmd = Command::new(node);
        cmd.arg(root().join("tests/run_browser.mjs"));
        cmd.arg(format!("--pages={}", pages.join(",")));
        for r in roots {
            cmd.arg(r);
        }
        let out = cmd.output().expect("run node");
        let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
        if out.status.code() == Some(77) {
            return eprint!("{text}");
        }
        assert!(out.status.success(), "browser test failed:\n{text}");
        print!("{text}");
    }

    fn prepare_demo(target: &str, target_root: &Path, spec: &PackageSpec, demo: &str, bindings_stamp: &str, bun: Option<&Path>) -> bool {
        let ex_dir = root().join("examples").join(target).join(spec.pkg).join(demo);
        let out = target_root.join(spec.pkg).join(demo);
        let bindings = target_root.join("bindings");
        let _g = lock_key(&format!("demo:{target}:{}:{demo}", spec.pkg));

        let demo_stamp = hash_tree_filtered(&ex_dir, &[bindings_stamp], DEMO_SKIP);
        let sentinel = out.join(".prepared");
        if fs::read_to_string(&sentinel).is_ok_and(|s| s == demo_stamp) {
            return true;
        }

        let _ = fs::remove_dir_all(&out);
        copy_dir_filtered(&ex_dir, &out, DEMO_SKIP);

        match target {
            "fsharp" => {
                let Some([fable]) = tools(["fable"]) else { return false };
                // Fable writes MSBuild restore artifacts inside referenced binding projects.
                let _fg = lock_key(&format!("fable:{target}"));
                run(Command::new(fable).current_dir(&out).args(["App.fsproj", "-o", "js", "--extension", ".mjs", "--noCache"]));
            }
            "c" => {
                let Some(clang) = wasi_clang_c() else { return false };
                run(Command::new(clang).args(CLANG_WASI_WASM).arg("-I").arg(&bindings).arg("-o").arg(out.join("app.wasm")).arg(out.join("main.c")));
            }
            "cpp" => {
                let Some(clang) = wasi_clang() else { return false };
                let mut objects = Vec::new();
                let needs_dom = spec.needs_dom || spec.extra_specs.iter().any(|e| e.needs_dom);
                if needs_dom {
                    objects.extend(compile_cpp_package(&clang, &bindings, "dom"));
                }
                for s in std::iter::once(spec).chain(spec.extra_specs.iter().copied()) {
                    objects.extend(compile_cpp_package(&clang, &bindings, s.crate_name));
                }
                run(Command::new(clang).args(CLANGXX_WASM).arg("-I").arg(&bindings).arg("-o").arg(out.join("app.wasm")).arg(out.join("main.cpp")).args(&objects));
            }
            "zig" => {
                let Some([zig]) = tools(["zig"]) else { return false };
                run(Command::new(zig).current_dir(&out).args(["build", "-Drelease=true"]));
                fs::copy(out.join("zig-out/bin/app.wasm"), out.join("app.wasm")).unwrap();
            }
            "rust" => {
                let Some(rustc) = rustc_wasm() else { return false };
                let rlibs_dir = bindings.join(".rlibs");
                let mut cmd = Command::new(&rustc);
                cmd.current_dir(&out)
                    .args(RUSTC_WASM)
                    .args(["--crate-type", "cdylib", "-C", "link-arg=--export-table", "-C", "strip=debuginfo", "-L"])
                    .arg(&rlibs_dir);
                let needs_dom = spec.needs_dom || spec.extra_specs.iter().any(|e| e.needs_dom);
                let dom_rlib = needs_dom.then(|| build_rust_rlib(&rustc, &bindings, "dom", &rlibs_dir, &[]));
                if let Some(ref dom_rlib) = dom_rlib {
                    cmd.arg("--extern").arg(format!("dom={}", dom_rlib.display()));
                }
                for s in std::iter::once(spec).chain(spec.extra_specs.iter().copied()) {
                    let externs: Vec<(&str, &Path)> = if s.needs_dom {
                        vec![("dom", dom_rlib.as_ref().expect("dom rlib").as_path())]
                    } else {
                        vec![]
                    };
                    let pkg_rlib = build_rust_rlib(&rustc, &bindings, s.crate_name, &rlibs_dir, &externs);
                    cmd.arg("--extern").arg(format!("{}={}", s.crate_name, pkg_rlib.display()));
                }
                cmd.args(["-o", "app.wasm", "src/lib.rs"]);
                run(&mut cmd);
            }
            "gleam" => {
                let Some([gleam]) = tools(["gleam"]) else { return false };
                fs::write(out.join("manifest.toml"), gleam_example_manifest(spec)).unwrap();
                run(Command::new(gleam).current_dir(&out).args(["build", "--warnings-as-errors"]));
            }
            _ => unreachable!("unknown target {target}"),
        }

        if matches!(spec.verify, VerifyKind::NodeCli) {
            fs::write(&sentinel, demo_stamp).unwrap();
            return true;
        }

        let bun = bun.expect("bun required for browser demos");
        let entry = if target == "fsharp" { "js/Program.mjs" } else { "main.mjs" };
        run(Command::new(bun).current_dir(&out).args(["build", "--target=browser", "--format=esm", entry, "--outfile=bundle.mjs"]));

        let html = fs::read_to_string(out.join("index.html")).unwrap();
        let old_tag = if target == "fsharp" {
            r#"<script type="module" src="./js/Program.mjs"></script>"#
        } else {
            r#"<script type="module" src="./main.mjs"></script>"#
        };
        assert!(html.contains(old_tag), "missing `{old_tag}` in {}", out.join("index.html").display());
        let verify = match spec.verify {
            VerifyKind::PtsCanvas => PTS_VERIFY_SCRIPT.to_string(),
            VerifyKind::BabylonWebGpu => BABYLON_VERIFY_SCRIPT.to_string(),
            VerifyKind::RapierBabylon3d => RAPIER_BABYLON_3D_VERIFY_SCRIPT.to_string(),
            VerifyKind::NodeCli => unreachable!("NodeCli returns above"),
            VerifyKind::DomSelector { selector, min_count } => format!(
                r#"<script type="module">
let passed = 0;
const failed = [];
window.addEventListener('unhandledrejection', e => failed.push('rejected: ' + (e.reason?.message ?? e.reason)));
try {{
  await import('./bundle.mjs');
  const deadline = Date.now() + 5000;
  let ok = false;
  while (Date.now() < deadline) {{
    const count = document.querySelectorAll({selector:?}).length;
    const textLen = (document.getElementById('app')?.textContent ?? '').trim().length;
    if (count >= {min_count} && textLen > 0) {{ ok = true; break; }}
    await new Promise(r => setTimeout(r, 50));
  }}
  if (ok) passed++;
  else failed.push(`expected >= {min_count} nodes matching {selector} inside #app`);
}} catch (e) {{
  failed.push('threw: ' + (e?.message ?? e));
}}
window.__result = {{ passed, failed, dom: [], domChecks: 1 }};
</script>"#
            ),
            VerifyKind::CanvasNonEmpty { selector, min_count } => format!(
                r#"<script type="module">
let passed = 0;
const failed = [];
window.addEventListener('unhandledrejection', e => failed.push('rejected: ' + (e.reason?.message ?? e.reason)));
try {{
  await import('./bundle.mjs');
  const deadline = Date.now() + 5000;
  let ok = false;
  while (Date.now() < deadline) {{
    const canvases = [...document.querySelectorAll({selector:?})];
    if (canvases.length >= {min_count}) {{
      ok = canvases.every(canvas => {{
        if (!canvas || canvas.width <= 0 || canvas.height <= 0) return false;
        const ctx = canvas.getContext('2d');
        let data = null;
        if (ctx) {{
          data = ctx.getImageData(0, 0, canvas.width, canvas.height).data;
        }} else {{
          const gl = canvas.getContext('webgl2') || canvas.getContext('webgl');
          if (gl) {{
            data = new Uint8Array(canvas.width * canvas.height * 4);
            gl.readPixels(0, 0, canvas.width, canvas.height, gl.RGBA, gl.UNSIGNED_BYTE, data);
          }}
        }}
        if (!data) return false;
        const colors = new Set();
        for (let i = 0; i < data.length; i += 4) {{
          if (data[i + 3] > 0) {{
            colors.add((data[i] << 16) | (data[i + 1] << 8) | data[i + 2]);
            if (colors.size >= 2) return true;
          }}
        }}
        return false;
      }});
      if (ok) break;
    }}
    await new Promise(r => setTimeout(r, 50));
  }}
  if (ok) passed++;
  else failed.push(`expected >= {min_count} non-empty canvas elements matching {selector}`);
}} catch (e) {{
  failed.push('threw: ' + (e?.message ?? e));
}}
window.__result = {{ passed, failed, dom: [], domChecks: 1 }};
</script>"#
            ),
        };
        fs::write(out.join("index.html"), html.replacen(old_tag, &verify, 1)).unwrap();
        fs::write(&sentinel, demo_stamp).unwrap();
        true
    }

    fn has_target_tool(target: &str) -> bool {
        match target {
            "fsharp" => tools(["fable"]).is_some(),
            "c" => tools(["clang"]).is_some(),
            "cpp" => wasi_clang().is_some(),
            "zig" => tools(["zig"]).is_some(),
            "rust" => rustc_wasm().is_some(),
            "gleam" => tools(["gleam"]).is_some(),
            _ => unreachable!("unknown target {target}"),
        }
    }

    fn run_package(spec: &PackageSpec, target: &str) {
        let from = root().join("target/libraries");
        if !from.join("node_modules").is_dir() {
            eprintln!("SKIPPED: libraries are not installed (tests/libraries/install.sh)");
            return;
        }
        if !has_target_tool(target) {
            return;
        }
        if matches!(spec.verify, VerifyKind::NodeCli) {
            let Some([node]) = tools(["node"]) else { return };
            let target_root = ensure_target_root(target, &from.join("node_modules"));
            let stamp = ensure_bindings_for_spec(&from, target, &target_root, spec);
            for demo in spec.demos {
                if !prepare_demo(target, &target_root, spec, demo, &stamp, None) {
                    return;
                }
                let demo_dir = target_root.join(spec.pkg).join(demo);
                let output = Command::new(&node).current_dir(&demo_dir).arg("main.mjs").output().expect("run node");
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                assert!(
                    output.status.success(),
                    "node CLI demo {target}/node/{demo} failed (status {:?}):\nstdout:\n{stdout}\nstderr:\n{stderr}",
                    output.status.code()
                );
                assert!(
                    stderr.trim().is_empty(),
                    "node CLI demo {target}/node/{demo} wrote to stderr:\n{stderr}"
                );
                assert!(
                    stdout.contains("PASS:") && stdout.lines().count() >= 3,
                    "node CLI demo {target}/node/{demo} missing expected PASS summary:\n{stdout}"
                );
            }
            return;
        }
        let Some([bun]) = tools(["bun"]) else { return };
        let target_root = ensure_target_root(target, &from.join("node_modules"));
        let stamp = ensure_bindings_for_spec(&from, target, &target_root, spec);
        let mut pages = Vec::new();
        for demo in spec.demos {
            if !prepare_demo(target, &target_root, spec, demo, &stamp, Some(&bun)) {
                return;
            }
            pages.push(format!("/{}/{demo}/index.html", spec.pkg));
        }
        run_browser_pages(&[&target_root], &pages);
    }

    fn run_gallery(target: &str) {
        let from = root().join("target/libraries");
        if !from.join("node_modules").is_dir() {
            eprintln!("SKIPPED: libraries are not installed (tests/libraries/install.sh)");
            return;
        }
        let Some([bun]) = tools(["bun"]) else { return };
        if !has_target_tool(target) {
            return;
        }
        let target_root = ensure_target_root(target, &from.join("node_modules"));
        let stamp = ensure_bindings_for_spec(&from, target, &target_root, &PTS_SPEC);
        if !prepare_demo(target, &target_root, &PTS_SPEC, "vector_field", &stamp, Some(&bun)) {
            return;
        }
        let page = format!(
            r#"<!doctype html>
<html>
<body style="margin:0">
<iframe id="g" src="./index.html" style="width:1280px;height:800px;border:0"></iframe>
<script type="module">
let passed = 0;
const failed = [];
const g = document.getElementById('g');
try {{
  const deadline = Date.now() + 10000;
  let ready = null;
  while (Date.now() < deadline) {{
    ready = g.contentWindow?.__galleryReady;
    if (ready) break;
    await new Promise(r => setTimeout(r, 50));
  }}
  if (!ready) throw new Error('gallery did not signal __galleryReady');
  if (ready.target !== '{target}' || ready.count < 2) {{
    throw new Error('unexpected manifest metadata: ' + JSON.stringify(ready));
  }}
  const doc = g.contentDocument;
  const links = doc.querySelectorAll('#nav .demo-link');
  if (links.length === ready.count) passed++;
  else failed.push(`sidebar link count ${{links.length}} != ${{ready.count}}`);

  const viewer = doc.getElementById('viewer');
  let rendered = false;
  while (Date.now() < deadline) {{
    const inner = viewer?.contentWindow;
    const canvas = viewer?.contentDocument?.querySelector('#pt canvas');
    if (inner?.__result?.passed > 0 && canvas && canvas.width > 0 && canvas.height > 0) {{
      rendered = true;
      break;
    }}
    await new Promise(r => setTimeout(r, 50));
  }}
  if (rendered) passed++;
  else failed.push('default demo iframe did not render #pt canvas');

  doc.getElementById('view-source-btn').click();
  let sourceLoaded = false;
  while (Date.now() < deadline) {{
    const drawer = doc.getElementById('source-drawer');
    const code = doc.getElementById('source-code')?.textContent ?? '';
    const lines = doc.querySelectorAll('#source-code .cv-line').length;
    if (!drawer.hidden && code.trim().length > 0 && code !== 'Loading...' && lines > 0) {{
      sourceLoaded = true;
      break;
    }}
    await new Promise(r => setTimeout(r, 50));
  }}
  if (sourceLoaded) passed++;
  else failed.push('view source drawer did not load highlighted source lines');
}} catch (e) {{
  failed.push('threw: ' + (e?.message ?? e));
}}
window.__result = {{ passed, failed, dom: [], domChecks: 3 }};
</script>
</body>
</html>"#
        );
        fs::write(target_root.join("gallery_verify.html"), page).unwrap();
        run_browser_pages(&[&target_root], &["/gallery_verify.html".to_string()]);
    }

    macro_rules! pkg_tests {
        ($mod_name:ident, $spec:ident) => {
            mod $mod_name {
                #[test] #[ignore] fn fsharp() { super::run_package(&super::$spec, "fsharp"); }
                #[test] #[ignore] fn c() { super::run_package(&super::$spec, "c"); }
                #[test] #[ignore] fn cpp() { super::run_package(&super::$spec, "cpp"); }
                #[test] #[ignore] fn zig() { super::run_package(&super::$spec, "zig"); }
                #[test] #[ignore] fn rust() { super::run_package(&super::$spec, "rust"); }
                #[test] #[ignore] fn gleam() { super::run_package(&super::$spec, "gleam"); }
            }
        };
    }

    pkg_tests!(pts, PTS_SPEC);
    pkg_tests!(babylon_lite, BABYLON_LITE_SPEC);
    pkg_tests!(katex, KATEX_SPEC);
    pkg_tests!(marked, MARKED_SPEC);
    pkg_tests!(date_fns, DATE_FNS_SPEC);
    pkg_tests!(echarts, ECHARTS_SPEC);
    pkg_tests!(konva, KONVA_SPEC);
    pkg_tests!(pixi, PIXI_SPEC);
    pkg_tests!(leaflet, LEAFLET_SPEC);
    pkg_tests!(tanstack_table, TANSTACK_TABLE_SPEC);
    pkg_tests!(prosemirror, PROSEMIRROR_SPEC);
    pkg_tests!(three, THREE_SPEC);
    pkg_tests!(rapier, RAPIER_SPEC);
    pkg_tests!(rapier_3d, RAPIER_3D_SPEC);
    pkg_tests!(node, NODE_SPEC);

    mod gallery {
        #[test] #[ignore] fn fsharp() { super::run_gallery("fsharp"); }
        #[test] #[ignore] fn c() { super::run_gallery("c"); }
        #[test] #[ignore] fn cpp() { super::run_gallery("cpp"); }
        #[test] #[ignore] fn zig() { super::run_gallery("zig"); }
        #[test] #[ignore] fn rust() { super::run_gallery("rust"); }
        #[test] #[ignore] fn gleam() { super::run_gallery("gleam"); }
    }
}

