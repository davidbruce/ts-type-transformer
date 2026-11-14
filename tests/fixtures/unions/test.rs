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
    let widget = Widget::new("w");
    let options = Options::create();
    options.set_verbose(true);

    // A union is an enum: any member goes in.
    check!(describe(Value::string("text".into())) == "string text");
    check!(describe(Value::number(42.0)) == "number 42");
    check!(describe(Value::Widget(widget.clone())) == "widget w");
    check!(describe(Value::Options(options.clone())) == "options verbose=true");

    // What comes back says which member it is.
    check!(matches!(pick(0.0), Value::string(s) if s == "text"));
    check!(matches!(pick(1.0), Value::number(n) if n == 42.0));
    check!(matches!(pick(2.0), Value::Widget(w) if w.get_name() == "w"));

    // A tagged union, told apart by its `kind`.
    let square = makeShape("square", 3.0);
    check!(matches!(&square, Shape::Square(s) if s.get_side() == 3.0));
    check!(area(square) == 9.0);

    // An inline union.
    check!(setup(boolean_or_Options::boolean(true)) == "bool true");
    check!(setup(boolean_or_Options::Options(options.clone())) == "options verbose=true");
    check!(matches!(lookup("abc"), number_or_Widget::number(n) if n == 3.0));
}
