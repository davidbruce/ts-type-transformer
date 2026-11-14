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
    // Class pairing: a constructor, a static method, a constant.
    let s = Shape::new(ShapeKind::Square, 3.0);
    check!(s.area() == 9.0);
    check!(Shape::unit().area() == 1.0);
    check!(Shape::SIDES == 4.0);

    // Literal union -> enum, in both directions.
    check!(s.kind() == ShapeKind::Square);
    check!(Shape::new(ShapeKind::Circle, 1.0).kind() == ShapeKind::Circle);

    // Properties.
    s.set_size(4.0);
    check!(s.size() == 4.0);

    // Inherited members come from the ancestor's trait; a Shape is a Named.
    check!(s.name() == "square-3");
    check!(s.describe() == "square-3 (16)");
    check!(name_of(&s) == "square-3");

    // Overloads are renamed; names are in snake_case, a property `x` is `x()` and `set_x(..)`.
    let big = s.scale(2.0);
    check!(big.size() == 8.0);
    check!(s.scale_with_name(2.0, "big").name() == "big");

    // Optional parameter: an Option.
    s.resize(2.6, None);
    check!(s.size() == 2.6);
    s.resize(2.6, Some(true));
    check!(s.size() == 3.0);

    // Union parameter: an enum, made from any of its members.
    check!(s.label("a") == "string:a");
    check!(s.label(7.0) == "number:7");
    check!(s.label(StringOrNumber::Number(7.0)) == "number:7");

    // Nullable: an Option. One object is one handle.
    check!(s.parent().is_none());
    check!(big.parent() == Some(s.clone()));
    big.set_parent(None);
    check!(big.parent().is_none());

    // Free functions.
    check!(total_area(&s, &Shape::unit()) == 10.0);
    check!(largest(&s, None) == Some(s.clone()));
    check!(largest(&Shape::unit(), Some(&s)) == Some(s.clone()));

    // `undefined` rather than `null`.
    check!(or_unit(None).area() == 1.0);
    check!(or_unit(Some(&s)) == s);
    check!(find("nothing").is_none());
    check!(find("unit").map(|u| u.area()) == Some(1.0));

    // Handles are counted: what goes out of scope is freed.
    let live = js::live_handles();
    for i in 0..500 {
        let t = Shape::new(ShapeKind::Circle, i as f64);
        let copy = t.clone();
        check!(copy.size() == i as f64);
    }
    check!(js::live_handles() == live);
}
