#![no_main]

use node::{BufferEncoding, fs, path};

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

    let dir = fs::mkdtemp_sync("/tmp/tsrs-node-ws-");
    c.check(fs::exists_sync(&dir) && fs::stat_sync(&dir).is_directory());

    let notes = path::join([dir.as_str(), "notes.txt"]);
    fs::write_file_sync(&notes, "alpha\nbeta\n");
    let st1 = fs::stat_sync(&notes);
    c.check(fs::exists_sync(&notes) && st1.is_file() && st1.size() == 11.0);

    fs::append_file_sync(&notes, "gamma\n");
    let text = fs::read_file_sync_string(&notes, BufferEncoding::Utf8);
    let st2 = fs::stat_sync(&notes);
    c.check(text == "alpha\nbeta\ngamma\n" && st2.size() == 17.0);

    let backup = path::join([dir.as_str(), "backup.txt"]);
    let archive = path::join([dir.as_str(), "archive.txt"]);
    fs::copy_file_sync(&notes, &backup);
    c.check(fs::exists_sync(&backup));

    fs::rename_sync(&backup, &archive);
    c.check(!fs::exists_sync(&backup) && fs::exists_sync(&archive));

    let arch_text = fs::read_file_sync_string(&archive, BufferEncoding::Utf8);
    c.check(arch_text == text);

    fs::unlink_sync(&archive);
    c.check(!fs::exists_sync(&archive));

    let rm = fs::RmOptions::default()
        .with_recursive(true)
        .with_force(true);
    fs::rm_sync_with(&dir, &rm);
    c.check(!fs::exists_sync(&dir));
    c.check(true);

    println(&format!("workspace={dir}"));
    println(&format!(
        "size_before={:.0} size_after={:.0}",
        st1.size(),
        st2.size()
    ));
    println(&format!("PASS: temp_workspace ({} checks)", c.0));
}
