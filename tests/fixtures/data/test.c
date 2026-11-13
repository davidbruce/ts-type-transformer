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

static JsValue doubled(void* ctx, JsValue value) { return js_number(js_to_number(value) * 2); }

__attribute__((export_name("run"))) void run(void) {
    // Option objects: built here, read by JavaScript.
    Options o = Options_create();
    Options_set_width(o, 3);
    Options_set_mode(o, Options_mode_fill);
    CHECK(str_eq(describe(o), "3x? fill"));
    Options_set_height(o, js_number(2));
    Options_set_title(o, js_string("hi"));
    CHECK(str_eq(describe(o), "3x2 fill \"hi\""));

    // Built by JavaScript, read here. A missing optional property is null.
    Options d = defaults();
    CHECK(Options_get_width(d) == 4);
    CHECK(Options_get_mode(d) == Options_mode_fit);
    CHECK(Options_get_height(d).h == 0);

    // Partial<Options>.
    patch_changes changes = patch_changes_create();
    patch_changes_set_width(changes, js_number(9));
    Options patched = patch(d, changes);
    CHECK(Options_get_width(patched) == 9);
    CHECK(Options_get_mode(patched) == Options_mode_fit);

    // Pick<Options, "width" | "height">.
    area_size size = area_size_create();
    area_size_set_width(size, 3);
    CHECK(area(size) == 9);
    area_size_set_height(size, js_number(5));
    CHECK(area(size) == 15);

    // keyof Options.
    CHECK(field() == field_result_title);

    // Record<string, number>.
    lookup_table table = lookup_table_create();
    lookup_table_set_at(table, "a", 7);
    CHECK(lookup(table, "a") == 7);
    CHECK(lookup(table, "b") == -1);
    CHECK(js_to_number(lookup_table_get_at(table, "a")) == 7);
    CHECK(lookup_table_get_at(table, "b").h == 0);

    // An intersection of object types.
    Tagged tagged = Tagged_create();
    Tagged_set_name(tagged, "x");
    Tagged_set_id(tagged, 5);
    CHECK(str_eq(show(tagged), "x#5"));

    // Inline object types.
    norm_point point = norm_point_create();
    norm_point_set_x(point, 3);
    norm_point_set_y(point, 4);
    CHECK(norm(point) == 5);
    CHECK(norm_point_get_x(origin()) == 0);

    // Arrays.
    JsArray r = range(4);
    CHECK(JsArray_length(r) == 4);
    CHECK(js_to_number(JsArray_get(r, 3)) == 3);
    CHECK(sum(r) == 6);
    JsArray mine = JsArray_new();
    JsArray_push(mine, js_number(10));
    JsArray_push(mine, js_number(5));
    CHECK(sum(mine) == 15);
    JsArray w = words("a bc");
    CHECK(JsArray_length(w) == 2);
    CHECK(str_eq(js_to_string(JsArray_get(w, 1)), "bc"));

    // Tuples are arrays.
    JsArray p = pair(1, "one");
    CHECK(js_to_number(JsArray_get(p, 0)) == 1);
    JsArray swapped = swap(p);
    CHECK(str_eq(js_to_string(JsArray_get(swapped, 0)), "one"));
    CHECK(js_to_number(JsArray_get(swapped, 1)) == 1);

    // Arrays of objects.
    JsArray all = items();
    CHECK(JsArray_length(all) == 2);
    Item item = { JsArray_get(all, 0).h };
    CHECK(Item_get_id(item) == 1);
    CHECK(JsArray_length(Item_get_tags(item)) == 2);

    // Generics are erased: values are untyped.
    Box b = wrap(js_number(21));
    CHECK(js_to_number(Box_get_value(b)) == 21);
    CHECK(js_to_number(Box_get_value(Box_map(b, Box_map_f_from(doubled, 0)))) == 42);
    CHECK(js_to_number(first(r)) == 0);
    CHECK(first(JsArray_new()).h == 0);
}
