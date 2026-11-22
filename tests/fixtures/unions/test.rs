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
    let widget = Widget::new("w");
    let options = Options::create();
    options.set_verbose(true);

    // A union is an enum: any member goes in.
    check!(describe("text") == "string text");
    check!(describe(42.0) == "number 42");
    check!(describe(&widget) == "widget w");
    check!(describe(Value::Options(options.clone())) == "options verbose=true");

    // What comes back says which member it is.
    check!(matches!(pick(0.0), Value::String(s) if s == "text"));
    check!(matches!(pick(1.0), Value::Number(n) if n == 42.0));
    check!(matches!(pick(2.0), Value::Widget(w) if w.name() == "w"));

    // A tagged union, told apart by its `kind`.
    let square = make_shape("square", 3.0);
    check!(matches!(&square, Shape::Square(s) if s.side() == 3.0));
    check!(area(square) == 9.0);

    // An inline union.
    check!(setup(true) == "bool true");
    check!(setup(&options) == "options verbose=true");
    check!(matches!(lookup("abc"), NumberOrWidget::Number(n) if n == 3.0));

    // Members no test tells apart: typed going in, made from a member; a value goes in as it is.
    let a = LooseA::create();
    a.set_a(Some(1.0));
    check!(loose(&a) == "loose {\"a\":1}");
    check!(loose(Loose::LooseA(a.clone())) == "loose {\"a\":1}");
    check!(loose(any_loose()) == "loose {\"b\":2}");
}
