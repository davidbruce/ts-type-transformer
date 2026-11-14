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
    // Option objects: built here, read by JavaScript.
    let o = Options::create();
    o.set_width(3.0);
    o.set_mode(Options_mode::fill);
    check!(describe(&o) == "3x? fill");
    o.set_height(Some(2.0));
    o.set_title(Some("hi"));
    check!(describe(&o) == "3x2 fill \"hi\"");

    // Built by JavaScript, read here. A missing optional property is None.
    let d = defaults();
    check!(d.get_width() == 4.0);
    check!(d.get_mode() == Options_mode::fit);
    check!(d.get_height().is_none());
    check!(d.get_title().is_none());

    // Partial<Options>.
    let changes = patch_changes::create();
    changes.set_width(Some(9.0));
    let patched = patch(&d, &changes);
    check!(patched.get_width() == 9.0 && patched.get_mode() == Options_mode::fit);

    // Pick<Options, "width" | "height">.
    let size = area_size::create();
    size.set_width(3.0);
    check!(area(&size) == 9.0);
    size.set_height(Some(5.0));
    check!(area(&size) == 15.0);

    // keyof Options.
    check!(field() == field_result::title);

    // Record<string, number>.
    let table = lookup_table::create();
    table.set_at("a", 7.0);
    check!(lookup(&table, "a") == 7.0);
    check!(lookup(&table, "b") == -1.0);
    check!(table.get_at("a") == Some(7.0));
    check!(table.get_at("b").is_none());

    // An intersection of object types.
    let tagged = Tagged::create();
    tagged.set_name("x");
    tagged.set_id(5.0);
    check!(show(&tagged) == "x#5");

    // Inline object types.
    let point = norm_point::create();
    point.set_x(3.0);
    point.set_y(4.0);
    check!(norm(&point) == 5.0);
    check!(origin().get_x() == 0.0);

    // Arrays are typed: number[] is JsArray<f64>, made from any iterator.
    let r = range(4.0);
    check!(r.len() == 4 && r.get(3) == 3.0);
    check!(sum(r.clone()) == 6.0);
    check!(sum([10.0, 5.0].into_iter().collect()) == 15.0);
    check!(words("a bc").get(1) == "bc");

    // Tuples: [number, string] is Tuple2<f64, String>.
    let p = pair(1.0, "one");
    check!(p.get0() == 1.0 && p.get1() == "one");
    let swapped = swap(Tuple2::new(2.0, "two".to_string()));
    check!(swapped.get0() == "two" && swapped.get1() == 2.0);

    // Arrays of objects.
    let all = items();
    check!(all.len() == 2 && all.get(0).get_id() == 1.0 && all.get(0).get_tags().get(1) == "b");

    // Generics are generic: a T crosses as a handle and comes back a T.
    let b = wrap(21.0);
    check!(b.get_value() == 21.0);
    check!(b.map(Box_map_f::new(|v: f64| v * 2.0)).get_value() == 42.0);
    let text: Box<String> = b.map(Box_map_f::new(|v: f64| (v as i32).to_string()));
    check!(text.get_value() == "21");
    check!(first(r.clone()) == Some(0.0));
    check!(first::<f64>(JsArray::new()).is_none());
    check!(b.replace("x".to_string()).get_value() == "x");

    // A generic union is an enum; a bare T is told apart by what T is.
    check!(unbox(Either::T(5.0)) == 5.0);
    check!(unbox(Either::Box(b.clone())) == 21.0);
    check!(matches!(either(5.0, false), Either::T(v) if v == 5.0));
    check!(matches!(either(5.0, true), Either::Box(x) if x.get_value() == 5.0));

    // A union as a type argument is the union.
    let some = ids();
    check!(matches!(some.get(0), Id::number(n) if n == 1.0) && matches!(some.get(1), Id::string(s) if s == "two"));
    check!(matches!(boxedId().get_value(), Id::string(s) if s == "three"));

    // A constraint, and a constrained interface.
    check!(largest(all.clone()).get_id() == 2.0);
    check!(hold(&all.get(1)).get_held().get_id() == 2.0);

    // A name both generic bases have is the type's own.
    let pp = pipe();
    check!(pp.get_label() == "pipe");
    pp.put(4.0);
    check!(pp.take() == 4.0);

    // Handles are counted: what goes out of scope is freed.
    let live = live_handles();
    for i in 0..500 {
        let t = wrap(i as f64);
        let copy = t.clone();
        check!(copy.get_value() == i as f64 && words("a b").get(1) == "b");
    }
    check!(live_handles() == live);
}
