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
    // Class pairing: constructor, static method, constant.
    Shape s = Shape_new(Shape_kind_square, 3);
    CHECK(Shape_area(s) == 9);
    CHECK(Shape_area(Shape_unit()) == 1);
    CHECK(Shape_SIDES == 4);

    // Literal union -> enum, in both directions.
    CHECK(Shape_get_kind(s) == Shape_kind_square);
    CHECK(Shape_get_kind(Shape_new(Shape_kind_circle, 1)) == Shape_kind_circle);

    // Properties.
    Shape_set_size(s, 4);
    CHECK(Shape_get_size(s) == 4);

    // Inherited members are called by the type that declares them, with a subtype.
    CHECK(str_eq(Named_get_name(s), "square-3"));
    CHECK(str_eq(Named_describe(s), "square-3 (16)"));
    CHECK(str_eq(nameOf(s), "square-3"));

    // Overloads.
    Shape big = Shape_scale(s, 2);
    CHECK(Shape_get_size(big) == 8);
    CHECK(str_eq(Named_get_name(Shape_scale_with_name(s, 2, "big")), "big"));

    // Optional parameter.
    Shape_resize(s, 2.6, js_none_bool());
    CHECK(Shape_get_size(s) == 2.6);
    Shape_resize(s, 2.6, js_some_bool(true));
    CHECK(Shape_get_size(s) == 3);

    // Union parameter.
    CHECK(str_eq(Shape_label(s, "a"), "string:a"));
    CHECK(str_eq(Shape_label_with_number(s, 7), "number:7"));

    // Nullable.
    CHECK(Shape_get_parent(s).h == 0);
    CHECK(js_same(Shape_get_parent(big), s));
    Shape_set_parent(big, (Shape){ 0 });
    CHECK(Shape_get_parent(big).h == 0);

    // Global functions.
    CHECK(totalArea(s, Shape_unit()) == 10);
    CHECK(js_same(largest(s, (Shape){ 0 }), s));
    CHECK(js_same(largest(Shape_unit(), s), s));

    // `undefined` rather than `null`.
    CHECK(Shape_area(orUnit((Shape){ 0 })) == 1);
    CHECK(js_same(orUnit(s), s));
    CHECK(find("nothing").h == 0);
    CHECK(Shape_area(find("unit")) == 1);
}
