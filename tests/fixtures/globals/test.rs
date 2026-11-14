// Calls the generated Rust bindings; every check! must hold.
#![no_main]
mod bindings;
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
    // Global values.
    let s = get_settings();
    check!(s.get_level() == 2.0 && !s.get_debug());
    s.set_debug(true);
    check!(get_settings().get_debug());
    check!(get_VERSION() == "1.2.3");

    // A global with members of its own.
    check!(Clock::now() == 1000.0);
    check!(Clock::ZERO == 0.0);

    // Namespaces, nested ones included.
    let a = Geometry_point(0.0, 0.0);
    let b = Geometry_point(3.0, 4.0);
    check!(Geometry_distance(&a, &b) == 5.0);
    check!(b.get_x() == 3.0);
    check!(get_Geometry_UNIT() == 1.0);
    check!(Geometry_Deep_twice(4.0) == 8.0);

    // Rest parameters take an array.
    check!(join("-", ["a", "b", "c"].into_iter().collect()) == "a-b-c");
    check!(join("-", JsArray::new()) == "");

    // Numeric enums.
    check!(bump(Level::Low) == Level::High);
    check!(bump(Level::High) == Level::High);
}
