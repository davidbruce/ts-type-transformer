// Node's fs, path and events from Rust: files made and read in a scratch
// directory, paths taken apart and joined, an EventEmitter used. Every
// check! must hold.
#![no_main]
use bindings::events::*;
use bindings::fs;
use bindings::path;
use bindings::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[link(wasm_import_module = "env")]
unsafe extern "C" {
    #[link_name = "check"]
    fn check_at(ok: i32, line: u32);
}
macro_rules! check {
    ($c:expr) => {
        unsafe { check_at($c as i32, line!()) }
    };
}

#[unsafe(no_mangle)]
pub extern "C" fn run() {
    paths();
    files();
    emitter();
}

fn paths() {
    // Taking paths apart and putting them together.
    check!(path::join(["/usr", "local", "..", "lib", "x.txt"]) == "/usr/lib/x.txt");
    check!(path::normalize("/a//b/./c/..") == "/a/b");
    check!(path::basename("/tmp/out/report.txt") == "report.txt");
    check!(path::basename_with("/tmp/out/report.txt", Some(".txt")) == "report");
    check!(path::dirname("/tmp/out/report.txt") == "/tmp/out");
    check!(path::extname("report.tar.gz") == ".gz" && path::extname("README") == "");
    check!(path::is_absolute("/tmp") && !path::is_absolute("tmp/x"));
    check!(path::resolve(["/a/b", "../c", "d"]) == "/a/c/d");
    check!(path::relative("/a/b/c", "/a/d") == "../../d");
    check!(
        path::matches_glob("/foo/bar.txt", "/foo/*.txt")
            && !path::matches_glob("/foo/bar.md", "/foo/*.txt")
    );
    check!(path::to_namespaced_path("/a/b") == "/a/b");
    check!(path::sep().as_str() == "/" && path::delimiter().as_str() == ":");
    let p = path::parse("/home/user/dir/file.txt");
    check!(p.root() == "/" && p.dir() == "/home/user/dir" && p.base() == "file.txt");
    check!(p.ext() == ".txt" && p.name() == "file");

    // What the module declares as plain objects:
    let back = path::FormatInputPathObject::create();
    back.set_dir(Some("/tmp/out"));
    back.set_name(Some("report"));
    back.set_ext(Some(".txt"));
    check!(back.dir().as_deref() == Some("/tmp/out"));
    check!(back.name().as_deref() == Some("report") && back.ext().as_deref() == Some(".txt"));
    check!(back.root().is_none());
    check!(path::format(&back) == "/tmp/out/report.txt");
    let parsed = path::ParsedPath::create();
    parsed.set_base("notes.md");
    parsed.set_ext(".md");
    check!(parsed.base() == "notes.md" && parsed.ext() == ".md");
}

fn files() {
    // A scratch directory, made under target/e2e and removed at the end.
    let dir = fs::mkdtemp_sync("target/e2e/node-fs-");
    check!(dir.starts_with("target/e2e/node-fs-"));
    check!(fs::exists_sync(dir.as_str()));
    check!(fs::stat_sync(dir.as_str()).is_directory());

    let file = format!("{}/{}", dir.as_str(), "hello.txt");
    check!(!fs::exists_sync(file.as_str()));
    fs::write_file_sync(file.as_str(), "hello, node\nsecond line\n");
    check!(fs::exists_sync(file.as_str()));

    // Read back as bytes (`Buffer<T>` derefs to `Uint8Array<T>`), and as text with an encoding.
    let bytes = fs::read_file_sync(file.as_str());
    check!(bytes.len() == 24);
    check!(String::from_utf8(bytes.to_vec()).unwrap() == "hello, node\nsecond line\n");
    let text = fs::read_file_sync_with_read_file_sync_arg(file.as_str(), BufferEncoding::Utf8);
    check!(text == "hello, node\nsecond line\n");

    fs::append_file_sync(file.as_str(), "third\n");
    let text = fs::read_file_sync_with_read_file_sync_arg(file.as_str(), BufferEncoding::Utf8);
    check!(text.lines().count() == 3 && text.ends_with("third\n"));
    let stats = fs::stat_sync(file.as_str());
    check!(stats.is_file() && !stats.is_directory());
    check!(stats.size() == 30.0);

    // A directory made, with a file in it, listed.
    let sub = format!("{}/{}", dir.as_str(), "sub");
    let mk = fs::MakeDirectoryOptions::create();
    mk.set_recursive(Some(true));
    fs::mkdir_sync_with_opt_make_directory_options_arg_with(sub.as_str(), &mk);
    fs::write_file_sync(format!("{}/{}", sub.as_str(), "b.txt").as_str(), "b");
    fs::copy_file_sync(
        file.as_str(),
        format!("{}/{}", dir.as_str(), "copy.txt").as_str(),
    );
    let mut names: Vec<String> = fs::readdir_sync(dir.as_str()).iter().collect();
    names.sort();
    check!(names == ["copy.txt", "hello.txt", "sub"]);
    check!(fs::stat_sync(sub.as_str()).is_directory());

    // Renamed and removed.
    let moved = format!("{}/{}", dir.as_str(), "moved.txt");
    fs::rename_sync(file.as_str(), moved.as_str());
    check!(!fs::exists_sync(file.as_str()) && fs::exists_sync(moved.as_str()));
    fs::unlink_sync(moved.as_str());
    check!(!fs::exists_sync(moved.as_str()));

    // A missing file throws; the call returns a zero value and the error is kept.
    let missing =
        js::attempt(|| fs::read_file_sync(format!("{}/{}", dir.as_str(), "nope").as_str()));
    check!(missing.is_err_and(|e| e.message().contains("ENOENT")));

    // Removed with everything in it.
    let rm = fs::RmOptions::create();
    rm.set_recursive(Some(true));
    fs::rm_sync_with(dir.as_str(), Some(&rm));
    check!(!fs::exists_sync(dir.as_str()));
}

fn emitter() {
    let em = EventEmitter::new();
    let log: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let on_greet = Listener::new({
        let log = log.clone();
        move |args: JsArray| {
            log.borrow_mut()
                .push(format!("greet {} {}", args.get(0).as_string(), args.len()));
        }
    });
    em.on("greet", &on_greet);
    check!(em.listener_count("greet") == 1.0);
    check!(em.emit("greet", [JsValue::from("world")]));
    check!(log.borrow().len() == 1 && log.borrow()[0] == "greet world 1");

    // once fires one time only; emit says whether anyone listened.
    let once_count = Rc::new(Cell::new(0));
    let once = Listener::new({
        let c = once_count.clone();
        move |_: JsArray| c.set(c.get() + 1)
    });
    em.once("ping", &once);
    check!(em.listener_count("ping") == 1.0);
    check!(em.emit("ping", Vec::<JsValue>::new()));
    check!(!em.emit("ping", Vec::<JsValue>::new()));
    check!(once_count.get() == 1);
    check!(em.listener_count("ping") == 0.0);

    // remove_listener stops a listener; event_names lists what is left.
    check!(em.event_names().len() == 1);
    em.remove_listener("greet", &on_greet);
    check!(em.listener_count("greet") == 0.0);
    check!(!em.emit("greet", [JsValue::from("again")]));
    check!(log.borrow().len() == 1);

    em.set_max_listeners(3.0);
    check!(em.get_max_listeners() == 3.0);
    // An "error" event nobody listens for throws.
    let thrown = js::attempt(|| em.emit("error", [JsValue::from(&Error::new("boom"))]));
    check!(thrown.is_err_and(|e| e.message().contains("boom")));
}
