// Calls the generated C bindings; every CHECK must hold. A union is a tagged
// struct: `tag` says which member of `as` is set.
#include "bindings.h"

TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

static bool str_eq(JsString s, const char* expected) {
    char buf[128];
    uint32_t n = JsString_len(s);
    if (n != ts_strlen(expected) || n > sizeof buf) return false;
    JsString_read(s, buf);
    for (uint32_t i = 0; i < n; i++)
        if (buf[i] != expected[i]) return false;
    return true;
}

__attribute__((export_name("run"))) void run(void) {
    Widget widget = Widget_new("w");
    Options options = Options_create();
    Options_set_verbose(options, true);

    // Any member goes in.
    Value text = { .tag = Value_string, .as.string = JsString_from("text") };
    Value number = { .tag = Value_number, .as.number = 42 };
    Value w = { .tag = Value_Widget, .as.Widget = widget };
    Value o = { .tag = Value_Options, .as.Options = options };
    CHECK(str_eq(describe(text), "string text"));
    CHECK(str_eq(describe(number), "number 42"));
    CHECK(str_eq(describe(w), "widget w"));
    CHECK(str_eq(describe(o), "options verbose=true"));

    // What comes back says which member it is.
    Value first = pick(0);
    CHECK(first.tag == Value_string && str_eq(first.as.string, "text"));
    CHECK(pick(1).tag == Value_number && pick(1).as.number == 42);
    Value third = pick(2);
    CHECK(third.tag == Value_Widget && str_eq(Widget_get_name(third.as.Widget), "w"));

    // A tagged union, told apart by its `kind`.
    Shape square = makeShape("square", 3);
    CHECK(square.tag == Shape_Square && Square_get_side(square.as.Square) == 3);
    CHECK(area(square) == 9);

    // A union parameter of a function becomes overloads.
    CHECK(str_eq(setup(true), "bool true"));
    CHECK(str_eq(setup(options), "options verbose=true"));
    number_or_Widget found = lookup("abc");
    CHECK(found.tag == number_or_Widget_number && found.as.number == 3);
}
