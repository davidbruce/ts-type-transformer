#![no_main]

use node::{fs, path};
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

    let dir = fs::mkdtemp_sync("/tmp/tsrs-node-ws-", None);
    c.check(fs::exists_sync(dir.as_str()) && fs::stat_sync(dir.as_str()).is_directory());

    let notes = path::join([dir.as_str(), "notes.txt"]);
    fs::write_file_sync(notes.as_str(), "alpha\nbeta\n", None);
    let st1 = fs::stat_sync(notes.as_str());
    c.check(fs::exists_sync(notes.as_str()) && st1.is_file() && st1.size() == 11.0);

    fs::append_file_sync(notes.as_str(), "gamma\n", None);
    let utf8 = GlobalBufferEncoding(JsValue::from("utf8").0.clone());
    let text = fs::read_file_sync_or_buffer_encoding(notes.as_str(), &utf8);
    let st2 = fs::stat_sync(notes.as_str());
    c.check(text == "alpha\nbeta\ngamma\n" && st2.size() == 17.0);

    let backup = path::join([dir.as_str(), "backup.txt"]);
    let archive = path::join([dir.as_str(), "archive.txt"]);
    fs::copy_file_sync(notes.as_str(), backup.as_str(), None);
    c.check(fs::exists_sync(backup.as_str()));

    fs::rename_sync(backup.as_str(), archive.as_str());
    c.check(!fs::exists_sync(backup.as_str()) && fs::exists_sync(archive.as_str()));

    let arch_text = fs::read_file_sync_or_buffer_encoding(archive.as_str(), &utf8);
    c.check(arch_text == text);

    fs::unlink_sync(archive.as_str());
    c.check(!fs::exists_sync(archive.as_str()));

    let rm = fs::RmOptions::create();
    rm.set_recursive(Some(true));
    rm.set_force(Some(true));
    fs::rm_sync(dir.as_str(), Some(&rm));
    c.check(!fs::exists_sync(dir.as_str()));
    c.check(true);

    println(&format!("workspace={}", dir));
    println(&format!(
        "size_before={:.0} size_after={:.0}",
        st1.size(),
        st2.size()
    ));
    println(&format!("PASS: temp_workspace ({} checks)", c.0));
}
