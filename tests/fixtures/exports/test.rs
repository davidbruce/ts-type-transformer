// Calls a module library through the generated Rust bindings; every check! must hold.
#![no_main]
use bindings::*;

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
    // A default export, reached as `default`.
    check!(origin().x() == 0.0);

    // Exported under other names: called and constructed by them.
    check!(area(3.0, 4.0) == 12.0);
    let f = Figure::new(2.0);
    check!(f.grow() == 3.0 && f.size() == 3.0);
    check!(Badge::new("x").label() == "x");

    // Constants are read, never assigned: there is no set_limit.
    check!(limit() == 10.0);
    check!(counter() == 0.0 && bump() == 1.0 && counter() == 1.0);

    // A union told apart by the classes as exported.
    check!(matches!(pick(0.0), FigureOrBadge::Figure(f) if f.size() == 2.0));
    check!(matches!(pick(1.0), FigureOrBadge::Badge(_)));
}
