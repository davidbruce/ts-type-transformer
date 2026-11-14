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
    // Class pairing: a constructor, a static method, a constant.
    let s = Shape::new(Shape_kind::square, 3.0);
    check!(s.area() == 9.0);
    check!(Shape::unit().area() == 1.0);
    check!(Shape::SIDES == 4.0);

    // Literal union -> enum, in both directions.
    check!(s.get_kind() == Shape_kind::square);
    check!(Shape::new(Shape_kind::circle, 1.0).get_kind() == Shape_kind::circle);

    // Properties.
    s.set_size(4.0);
    check!(s.get_size() == 4.0);

    // Inherited members come from the ancestor's trait; a Shape is a Named.
    check!(s.get_name() == "square-3");
    check!(s.describe() == "square-3 (16)");
    check!(nameOf(&s) == "square-3");

    // Overloads are renamed.
    let big = s.scale(2.0);
    check!(big.get_size() == 8.0);
    check!(s.scale_with_name(2.0, "big").get_name() == "big");

    // Optional parameter: an Option.
    s.resize(2.6, None);
    check!(s.get_size() == 2.6);
    s.resize(2.6, Some(true));
    check!(s.get_size() == 3.0);

    // Union parameter: an enum.
    check!(s.label(string_or_number::string("a".into())) == "string:a");
    check!(s.label(string_or_number::number(7.0)) == "number:7");

    // Nullable: an Option. One object is one handle.
    check!(s.get_parent().is_none());
    check!(big.get_parent() == Some(s.clone()));
    big.set_parent(None);
    check!(big.get_parent().is_none());

    // Free functions.
    check!(totalArea(&s, &Shape::unit()) == 10.0);
    check!(largest(&s, None) == Some(s.clone()));
    check!(largest(&Shape::unit(), Some(s.clone())) == Some(s.clone()));

    // `undefined` rather than `null`.
    check!(orUnit(None).area() == 1.0);
    check!(orUnit(Some(s.clone())) == s);
    check!(find("nothing").is_none());
    check!(find("unit").map(|u| u.area()) == Some(1.0));

    // Handles are counted: what goes out of scope is freed.
    let live = live_handles();
    for i in 0..500 {
        let t = Shape::new(Shape_kind::circle, i as f64);
        let copy = t.clone();
        check!(copy.get_size() == i as f64);
    }
    check!(live_handles() == live);
}
