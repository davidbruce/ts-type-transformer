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
    // Option objects: built here, read by JavaScript.
    let o = Options::create();
    o.set_width(3.0);
    o.set_mode(OptionsMode::Fill);
    check!(describe(&o) == "3x? fill");
    o.set_height(Some(2.0));
    o.set_title(Some("hi"));
    check!(describe(&o) == "3x2 fill \"hi\"");

    // A nullable string is a `&str`, a `&String` or None; a nullable object a reference or None.
    let name = String::from("bob");
    check!(greeting("ann", None) == "ann");
    check!(greeting(&name, &o) == "bob 3");
    check!(greeting(None, Some(&o)) == "nobody 3");

    // A parameter of any type takes a handle, a number, a string; a JsValue goes as it is.
    check!(echo(&o) == "object:3");
    check!(echo(5.0) == "number:5");
    check!(echo("x") == "string:x");
    check!(echo(JsValue::from(true)) == "boolean:true");

    // Built by JavaScript, read here. A missing optional property is None.
    let d = defaults();
    check!(d.width() == 4.0);
    check!(d.mode() == OptionsMode::Fit);
    check!(d.height().is_none());
    check!(d.title().is_none());

    // Partial<Options>.
    let changes = PatchChanges::create();
    changes.set_width(Some(9.0));
    let patched = patch(&d, &changes);
    check!(patched.width() == 9.0 && patched.mode() == OptionsMode::Fit);

    // Pick<Options, "width" | "height">.
    let size = AreaSize::create();
    size.set_width(3.0);
    check!(area(&size) == 9.0);
    size.set_height(Some(5.0));
    check!(area(&size) == 15.0);

    // keyof Options.
    check!(field() == FieldResult::Title);

    // Record<string, number>.
    let table = LookupTable::create();
    table.set_at("a", 7.0);
    check!(lookup(&table, "a") == 7.0);
    check!(lookup(&table, "b") == -1.0);
    check!(table.at("a") == Some(7.0));
    check!(table.at("b").is_none());

    // An intersection of object types.
    let tagged = Tagged::create();
    tagged.set_name("x");
    tagged.set_id(5.0);
    check!(show(&tagged) == "x#5");

    // Inline object types.
    let point = NormPoint::create();
    point.set_x(3.0);
    point.set_y(4.0);
    check!(norm(&point) == 5.0);
    check!(origin().x() == 0.0);

    // Arrays are typed: number[] is JsArray<f64>, made from an array, a Vec or an iterator.
    let r = range(4.0);
    check!(r.len() == 4 && r.get(3) == 3.0);
    check!(sum(&r) == 6.0);
    check!(sum([10.0, 5.0]) == 15.0);
    check!(words("a bc").get(1) == "bc");

    // Tuples: [number, string] is JsTuple2<f64, String>.
    let p = pair(1.0, "one");
    check!(p.get0() == 1.0 && p.get1() == "one");
    let swapped = swap(&JsTuple2::new(2.0, "two"));
    check!(swapped.get0() == "two" && swapped.get1() == 2.0);

    // Generic aliases are the types they stand for: List<number> is
    // JsArray<f64>, Pair<string, number> JsTuple2<String, f64>, Boxed<string> Box<String>.
    check!(total([1.0, 2.0]) == 3.0);
    let flipped = flip(&JsTuple2::new("x", 7.0));
    check!(flipped.get0() == 7.0 && flipped.get1() == "x");
    check!(boxed_name().value() == "name");

    // Arrays of objects.
    let all = items();
    check!(all.len() == 2 && all.get(0).id() == 1.0 && all.get(0).tags().get(1) == "b");

    // Generics are generic: a T crosses as a handle and comes back a T.
    let b = wrap(21.0);
    check!(b.value() == 21.0);
    check!(b.map(&MapFn::new(|v: f64| v * 2.0)).value() == 42.0);
    let text: Box<String> = b.map(&MapFn::new(|v: f64| (v as i32).to_string()));
    check!(text.value() == "21");
    check!(first(&r) == Some(0.0));
    check!(first(JsArray::<f64>::new()).is_none());
    check!(b.replace("x".to_string()).value() == "x");

    // A generic union is an enum; a bare T is told apart by what T is.
    check!(unbox(Either::T(5.0)) == 5.0);
    check!(unbox(&b) == 21.0);
    check!(matches!(either(5.0, false), Either::T(v) if v == 5.0));
    check!(matches!(either(5.0, true), Either::Box(x) if x.value() == 5.0));

    // A union as a type argument is the union.
    let some = ids();
    check!(
        matches!(some.get(0), Id::Number(n) if n == 1.0)
            && matches!(some.get(1), Id::String(s) if s == "two")
    );
    check!(matches!(boxed_id().value(), Id::String(s) if s == "three"));

    // A constraint, and a constrained interface.
    check!(largest(&all).id() == 2.0);
    check!(hold(&all.get(1)).held().id() == 2.0);

    // A name both generic bases have is the type's own.
    let pp = pipe();
    check!(pp.label() == "pipe");
    pp.put(4.0);
    check!(pp.take() == 4.0);

    // Handles are counted: what goes out of scope is freed.
    let live = js::live_handles();
    for i in 0..500 {
        let t = wrap(i as f64);
        let copy = t.clone();
        check!(copy.value() == i as f64 && words("a b").get(1) == "b");
    }
    check!(js::live_handles() == live);
}
