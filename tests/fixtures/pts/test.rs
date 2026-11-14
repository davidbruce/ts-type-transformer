// Calls Pts, a real library, through its generated Rust bindings; every check! must hold.
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
    // A Pt is an array of coordinates, with names for the first four.
    let p = Pt::new([3.0, 4.0]);
    check!(p.x() == 3.0 && p.y() == 4.0);
    check!(p.magnitude() == 5.0);
    check!(p.to_string() == "Pt(3, 4)");
    p.set_id("corner");
    check!(p.id() == "corner");

    // A method that changes a Pt returns it; its `$` twin returns another.
    let moved = p._add([1.0, 1.0]);
    check!(moved != p && moved.x() == 4.0 && p.x() == 3.0);
    check!(p.add([1.0, 1.0]) == p && p.x() == 4.0);
    check!(p.equals(&moved, None));
    check!(p.js_clone().equals(&p, Some(0.001)) && p.js_clone() != p);

    // Statics.
    check!(Num::lerp(0.0, 10.0, 0.5) == 5.0);
    check!(Num::clamp(12.0, 0.0, 10.0) == 10.0);
    check!(Geom::to_degree(core::f64::consts::PI) == 180.0);
    check!(Pt::make(3.0, Some(2.0), None).to_string() == "Pt(2, 2, 2)");

    // A Group is an array of Pt.
    let square = Group::new([Pt::new([0.0, 0.0]), Pt::new([4.0, 0.0]), Pt::new([4.0, 4.0]), Pt::new([0.0, 4.0])]);
    check!(square.as_array().len() == 4 && square.as_array().get(2).y() == 4.0);
    check!(square.p1().x() == 0.0);
    check!(square.centroid().to_string() == "Pt(2, 2)");
    check!(square.bounding_box().p2().x() == 4.0);

    // A union parameter takes any of its members: a Group where points are wanted.
    check!(Geom::centroid(&square).x() == 2.0);
    check!(Line::magnitude(&Group::new([Pt::new([0.0, 0.0]), Pt::new([3.0, 4.0])])) == 5.0);
    let ray = Line::from_angle(&Pt::new([1.0, 1.0]), 0.0, 5.0);
    check!(ray.p2().x() == 6.0 && ray.p2().y() == 1.0);

    // Handles are counted: what goes out of scope is freed.
    let live = js::live_handles();
    for i in 0..200 {
        let q = Pt::new([i as f64, 1.0]);
        check!(q._add([1.0, 0.0]).x() == i as f64 + 1.0);
    }
    check!(js::live_handles() == live);
}
