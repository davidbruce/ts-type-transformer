#![no_main]

use node::path;
use node::*;

#[link(wasm_import_module = "env")]
unsafe extern "C" {
    #[link_name = "print"]
    fn host_print(ptr: *const u8, len: usize);
}

fn println(s: &str) {
    unsafe {
        host_print(s.as_ptr(), s.len());
        host_print(b"\n".as_ptr(), 1);
    }
}

struct Checks(u32);
impl Checks {
    fn check(&mut self, ok: bool) {
        if !ok {
            panic!("check failed");
        }
        self.0 += 1;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let mut c = Checks(0);

    let joined = path::join(["/workspace", "packages", "core", "..", "cli", "index.ts"]);
    c.check(joined == "/workspace/packages/cli/index.ts");

    let norm = path::normalize("/var//log/./app/../http/access.log");
    c.check(norm == "/var/log/http/access.log");

    let resolved = path::resolve(["/srv/app", "../shared", "config/prod.json"]);
    c.check(resolved == "/srv/shared/config/prod.json");

    let rel = path::relative("/srv/app/src/ui", "/srv/app/dist/assets");
    c.check(rel == "../../dist/assets");

    c.check(path::dirname("/srv/app/archive.tar.gz") == "/srv/app");
    c.check(path::basename("/srv/app/archive.tar.gz", None) == "archive.tar.gz");
    c.check(path::basename("/srv/app/archive.tar.gz", Some(".gz")) == "archive.tar");
    c.check(path::extname("/srv/app/archive.tar.gz") == ".gz");
    c.check(path::is_absolute("/srv/app") && !path::is_absolute("src/app"));

    let parsed = path::parse("/srv/app/archive.tar.gz");
    c.check(
        parsed.root() == "/"
            && parsed.dir() == "/srv/app"
            && parsed.base() == "archive.tar.gz"
            && parsed.name() == "archive.tar"
            && parsed.ext() == ".gz",
    );

    let fmt = path::FormatInputPathObject::create();
    fmt.set_dir(Some("/etc/service"));
    fmt.set_name(Some("daemon"));
    fmt.set_ext(Some(".conf"));
    let formatted = path::format(&fmt);
    c.check(formatted == "/etc/service/daemon.conf");

    c.check(
        path::matches_glob("/src/components/Button.tsx", "/src/**/*.tsx")
            && !path::matches_glob("/src/components/Button.css", "/src/**/*.tsx"),
    );

    println(&format!("join={} norm={}", joined, norm));
    println(&format!("resolve={} relative={} format={}", resolved, rel, formatted));
    println(&format!(
        "parsed=(root={} dir={} base={} ext={})",
        parsed.root(),
        parsed.dir(),
        parsed.base(),
        parsed.ext()
    ));
    println(&format!("PASS: path_inspector ({} checks)", c.0));
}
