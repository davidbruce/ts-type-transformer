#![no_main]

use std::cell::Cell;
use std::rc::Rc;

use node::events::EventEmitter;
use node::{AnyArrayFn, BufferEncoding, JsArray, JsValue, fs, js, path};

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

    let bus = EventEmitter::<JsValue>::new(None);
    let stages = Rc::new(Cell::new(0u32));
    let done_count = Rc::new(Cell::new(0u32));

    let on_stage = AnyArrayFn::new({
        let stages = stages.clone();
        move |_: JsArray| stages.set(stages.get() + 1)
    });
    let on_done = AnyArrayFn::new({
        let done_count = done_count.clone();
        move |_: JsArray| done_count.set(done_count.get() + 1)
    });
    bus.on::<JsValue>("stage", &on_stage);
    bus.once::<JsValue>("done", &on_done);

    let root = fs::mkdtemp_sync("/tmp/tsrs-node-pipe-", None);
    let in_dir = path::join([root.as_str(), "in"]);
    let out_dir = path::join([root.as_str(), "out"]);
    let mk = fs::MakeDirectoryOptions::default().with_recursive(true);
    fs::mkdir_sync_with_opt_number_or_string_or_make_directory_options(&*in_dir, &mk);
    fs::mkdir_sync_with_opt_number_or_string_or_make_directory_options(&*out_dir, &mk);

    // Stage 1: seed
    fs::write_file_sync(&*path::join([in_dir.as_str(), "a.log"]), "100 OK\n", None);
    fs::write_file_sync(&*path::join([in_dir.as_str(), "b.log"]), "200 OK\n", None);
    fs::write_file_sync(&*path::join([in_dir.as_str(), "c.log"]), "300 OK\n", None);
    bus.emit::<JsValue>("stage", [JsValue::from("seed")]);
    c.check(stages.get() == 1);

    // Stage 2: bundle
    let mut logs: Vec<String> = fs::readdir_sync(&*in_dir, None).iter().collect();
    logs.sort();
    c.check(logs == ["a.log", "b.log", "c.log"]);

    let combined = path::join([out_dir.as_str(), "combined.log"]);
    for name in &logs {
        c.check(path::extname(name) == ".log");
        let chunk = fs::read_file_sync_or_buffer_encoding(
            &*path::join([in_dir.as_str(), name.as_str()]),
            BufferEncoding::Utf8,
        );
        fs::append_file_sync(&*combined, &*chunk, None);
    }
    bus.emit::<JsValue>("stage", [JsValue::from("bundle")]);
    c.check(stages.get() == 2);

    // Stage 3: verify
    let comb_text = fs::read_file_sync_or_buffer_encoding(&*combined, BufferEncoding::Utf8);
    c.check(fs::stat_sync(&*combined).size() == 21.0 && comb_text.contains("200 OK"));

    let missing = js::attempt(|| fs::read_file_sync(&*path::join([out_dir.as_str(), "missing.log"]), None));
    let enoent_caught = missing.is_err_and(|e| e.message().contains("ENOENT"));
    c.check(enoent_caught);

    bus.emit::<JsValue>("stage", [JsValue::from("verify")]);
    bus.emit::<JsValue>("done", [JsValue::from(comb_text.len() as f64)]);
    c.check(stages.get() == 3 && done_count.get() == 1);

    let rm = fs::RmOptions::default().with_recursive(true).with_force(true);
    fs::rm_sync(&*root, Some(&rm));
    c.check(!fs::exists_sync(&*root));

    println(&format!("pipeline_root={root} combined_bytes={}", comb_text.len()));
    println(&format!(
        "stages={} done={} enoent={}",
        stages.get(),
        done_count.get(),
        enoent_caught
    ));
    println(&format!("PASS: file_pipeline ({} checks)", c.0));
}
