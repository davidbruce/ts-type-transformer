// Calls the generated Rust bindings; every check! must hold.
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
    // Global values.
    let s = settings();
    check!(s.level() == 2.0 && !s.debug());
    s.set_debug(true);
    check!(settings().debug());
    check!(version() == "1.2.3");

    // A global with members of its own.
    check!(Clock::now() == 1000.0);
    check!(Clock::ZERO == 0.0);

    // Namespaces, nested ones included.
    let a = geometry::point(0.0, 0.0);
    let b = geometry::point(3.0, 4.0);
    check!(geometry::distance(&a, &b) == 5.0);
    check!(b.x() == 3.0);
    check!(geometry::unit() == 1.0);
    check!(geometry::deep::twice(4.0) == 8.0);

    // Rest parameters take an array: a Rust array, a Vec, a slice or a JsArray.
    check!(join("-", ["a", "b", "c"]) == "a-b-c");
    check!(join("-", vec!["a".to_string(), "b".to_string()]) == "a-b");
    check!(join("-", JsArray::new()) == "");

    // Numeric enums.
    check!(bump(Level::Low) == Level::High);
    check!(bump(Level::High) == Level::High);
}
