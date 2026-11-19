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

fn walk_dir(dir: &str, dirs: &mut u32, files: &mut u32, total_bytes: &mut f64) {
    let mut entries: Vec<String> = fs::readdir_sync(dir, None).iter().collect();
    entries.sort();
    for name in entries {
        let full = path::join([dir, name.as_str()]);
        let st = fs::stat_sync(full.as_str());
        if st.is_directory() {
            *dirs += 1;
            walk_dir(full.as_str(), dirs, files, total_bytes);
        } else if st.is_file() {
            *files += 1;
            *total_bytes += st.size();
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let mut c = Checks(0);

    let root = fs::mkdtemp_sync("/tmp/tsrs-node-tree-", None);
    c.check(fs::exists_sync(root.as_str()));

    let mk = fs::MakeDirectoryOptions::create();
    mk.set_recursive(Some(true));
    let _ = fs::mkdir_sync_with_opt_number_or_string_or_make_directory_options(
        path::join([root.as_str(), "src", "core"]).as_str(),
        &mk,
    );
    let _ = fs::mkdir_sync_with_opt_number_or_string_or_make_directory_options(
        path::join([root.as_str(), "src", "cli"]).as_str(),
        &mk,
    );
    let _ = fs::mkdir_sync_with_opt_number_or_string_or_make_directory_options(
        path::join([root.as_str(), "docs"]).as_str(),
        &mk,
    );

    fs::write_file_sync(path::join([root.as_str(), "README.md"]).as_str(), "node tree demo", None);
    fs::write_file_sync(
        path::join([root.as_str(), "src", "core", "engine.txt"]).as_str(),
        "engine core module!\n",
        None,
    );
    fs::write_file_sync(
        path::join([root.as_str(), "src", "cli", "args.txt"]).as_str(),
        "cli args parser\n",
        None,
    );
    fs::write_file_sync(
        path::join([root.as_str(), "docs", "guide.md"]).as_str(),
        "monochrome documentation\n",
        None,
    );

    let mut dir_count = 0u32;
    let mut file_count = 0u32;
    let mut total_bytes = 0.0f64;
    walk_dir(root.as_str(), &mut dir_count, &mut file_count, &mut total_bytes);

    c.check(dir_count == 4);
    c.check(file_count == 4);
    c.check(total_bytes == 75.0);

    let rm = fs::RmOptions::create();
    rm.set_recursive(Some(true));
    rm.set_force(Some(true));
    fs::rm_sync(root.as_str(), Some(&rm));
    c.check(!fs::exists_sync(root.as_str()));
    c.check(dir_count + file_count == 8);
    c.check(true);

    println(&format!("root={}", root));
    println(&format!(
        "dirs={} files={} total_bytes={:.0}",
        dir_count, file_count, total_bytes
    ));
    println(&format!("PASS: directory_tree ({} checks)", c.0));
}
