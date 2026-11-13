// Calls the generated C bindings; every CHECK must hold.
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
    // Global values.
    Settings s = get_settings();
    CHECK(Settings_get_level(s) == 2);
    CHECK(!Settings_get_debug(s));
    Settings_set_debug(s, true);
    CHECK(Settings_get_debug(get_settings()));
    CHECK(str_eq(get_VERSION(), "1.2.3"));

    // A global with members of its own.
    CHECK(Clock_now() == 1000);
    CHECK(Clock_ZERO == 0);

    // Namespaces, nested ones included.
    Geometry_Point a = Geometry_point(0, 0);
    Geometry_Point b = Geometry_point(3, 4);
    CHECK(Geometry_distance(a, b) == 5);
    CHECK(Geometry_Point_get_x(b) == 3);
    CHECK(get_Geometry_UNIT() == 1);
    CHECK(Geometry_Deep_twice(4) == 8);

    // Rest parameters take an array.
    JsArray parts = JsArray_new();
    JsArray_push(parts, js_string("a"));
    JsArray_push(parts, js_string("b"));
    JsArray_push(parts, js_string("c"));
    CHECK(str_eq(join_with_parts("-", parts), "a-b-c"));
    CHECK(str_eq(join("-"), ""));

    // Numeric enums.
    CHECK(bump(Level_Low) == Level_High);
    CHECK(bump(Level_High) == Level_High);
}
