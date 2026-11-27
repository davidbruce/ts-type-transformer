#![no_main]
use bindings::*;

#[link(wasm_import_module = "env")]
unsafe extern "C" {
    #[link_name = "check"]
    fn check_at(ok: i32, line: u32);
}
macro_rules! check { ($e:expr) => { unsafe { check_at($e as i32, line!()) } }; }

#[unsafe(no_mangle)]
pub extern "C" fn run() {
    let rw = Perms::Read | Perms::Write;
    check!(rw == Perms::ReadWrite);
    check!(rw.contains(Perms::Read));
    check!(rw.contains(Perms::Write));
    check!(!rw.contains(Perms::Exec));
    check!((rw & Perms::Read) == Perms::Read);
    check!((rw ^ Perms::Read) == Perms::Write);
    check!(Perms::empty().bits() == 0);
    check!(rw.intersects(Perms::Read | Perms::Exec));

    let mut acc = Perms::Read;
    acc |= Perms::Exec;
    check!(acc == (Perms::Read | Perms::Exec));

    let re = combine_perms(Perms::Read, Perms::Exec);
    check!(has_perm(re, Perms::Read) && has_perm(re, Perms::Exec) && !has_perm(re, Perms::Write));
    check!(default_perms() == (Perms::Read | Perms::Exec));
    check!(default_perms_with(Perms::Write) == Perms::Write);
    check!(js::live_handles() == 0);
}
