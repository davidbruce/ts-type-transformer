// Names that would be one Rust identifier, each given its own. Every check! must hold.
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
    // The static property and the method are both `is_transforming`: the method gives way.
    let g = Gizmo::new();
    check!(g.is_transforming_2());
    let _f: GizmoIsTransforming = Gizmo::is_transforming();
    // `fooBar` and `foo_bar` are one `foo_bar`.
    check!(g.foo_bar() == 1.0 && g.foo_bar_2() == 2.0);

    let s = shape();
    check!(s.area() == 3.0 && shape_area(&s) == 30.0);

    // The property `create` gives way to the option bag's constructor.
    let p = point(4.0);
    check!(p.x() == 4.0);
    let _c: PointCreate = p.create_2();
    let _empty: Point = Point::create();

    check!(NsNode::new().id() == 2.0);
    check!(NsNode2::new().id() == 1.0);
}
