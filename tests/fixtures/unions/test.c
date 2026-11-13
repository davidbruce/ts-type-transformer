// Calls the generated C bindings; every CHECK must hold. The C config erases
// unions: they are untyped values, read with the js_to_* helpers.
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

    // Any value goes in.
    CHECK(str_eq(describe(js_string("text")), "string text"));
    CHECK(str_eq(describe(js_number(42)), "number 42"));
    CHECK(str_eq(describe((JsValue){ widget.h }), "widget w"));
    CHECK(str_eq(describe((JsValue){ options.h }), "options verbose=true"));

    // What comes back is read as the type the caller knows it is.
    CHECK(str_eq(js_to_string(pick(0)), "text"));
    CHECK(js_to_number(pick(1)) == 42);
    CHECK(str_eq(Widget_get_name((Widget){ pick(2).h }), "w"));

    // A tagged union, untyped.
    CHECK(area(makeShape("square", 3)) == 9);

    // A union parameter of a function becomes overloads.
    CHECK(str_eq(setup(true), "bool true"));
    CHECK(str_eq(setup_with_Options(options), "options verbose=true"));
    CHECK(js_to_number(lookup("abc")) == 3);
}
