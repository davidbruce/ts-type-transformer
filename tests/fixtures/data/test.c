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

static JsValue doubled(void* ctx, JsValue value) { (void)ctx; return js_number(js_to_number(value) * 2); }

__attribute__((export_name("run"))) void run(void) {
    // Option objects: built here, read by JavaScript.
    Options o = Options_create();
    Options_set_width(o, 3);
    Options_set_mode(o, Options_mode_fill);
    CHECK(str_eq(describe(o), "3x? fill"));
    Options_set_height(o, js_some_double(2));
    Options_set_title(o, "hi");
    CHECK(str_eq(describe(o), "3x2 fill \"hi\""));

    // Built by JavaScript, read here. A missing optional property is null.
    Options d = defaults();
    CHECK(Options_get_width(d) == 4);
    CHECK(Options_get_mode(d) == Options_mode_fit);
    CHECK(!Options_get_height(d).has);

    // Partial<Options>.
    patch_changes changes = patch_changes_create();
    patch_changes_set_width(changes, js_some_double(9));
    Options patched = patch(d, changes);
    CHECK(Options_get_width(patched) == 9);
    CHECK(Options_get_mode(patched) == Options_mode_fit);

    // Pick<Options, "width" | "height">.
    area_size size = area_size_create();
    area_size_set_width(size, 3);
    CHECK(area(size) == 9);
    area_size_set_height(size, js_some_double(5));
    CHECK(area(size) == 15);

    // keyof Options.
    CHECK(field() == field_result_title);

    // Record<string, number>: ordered pair list and get/set/has.
    JsRecord_double table = js_number_record({ "a", 7 });
    CHECK(lookup(table, "a") == 7);
    CHECK(lookup(table, "b") == -1);
    CHECK(JsRecord_double_get(table, "a").value == 7);
    CHECK(!JsRecord_double_get(table, "b").has);
    JsRecord_double_set(table, "c", 9);
    CHECK(JsRecord_double_get(table, "c").value == 9);

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

    // Arrays are typed: number[] is JsArray_double.
    JsArray_double r = range(4);
    CHECK(JsArray_double_length(r) == 4);
    CHECK(JsArray_double_get(r, 3) == 3);
    CHECK(sum(r) == 6);
    JsArray_double mine = JsArray_double_new();
    JsArray_double_push(mine, 10);
    JsArray_double_push(mine, 5);
    CHECK(sum(mine) == 15);
    JsArray_JsString w = words("a bc");
    CHECK(JsArray_JsString_length(w) == 2);
    CHECK(str_eq(JsArray_JsString_get(w, 1), "bc"));

    // Tuples are typed: [number, string] is JsTuple_double_JsString.
    JsTuple_double_JsString p = pair(1, "one");
    CHECK(JsTuple_double_JsString_get_0(p) == 1 && str_eq(JsTuple_double_JsString_get_1(p), "one"));
    JsTuple_double_JsString made = JsTuple_double_JsString_new(2, "two");
    JsTuple_JsString_double swapped = swap(made);
    CHECK(str_eq(JsTuple_JsString_double_get_0(swapped), "two"));
    CHECK(JsTuple_JsString_double_get_1(swapped) == 2);

    // Generic aliases are the types they stand for: List<number> is
    // JsArray_double, Pair<string, number> JsTuple_JsString_double.
    CHECK(total(mine) == 15);
    JsTuple_double_JsString flipped = flip(JsTuple_JsString_double_new("x", 7));
    CHECK(JsTuple_double_JsString_get_0(flipped) == 7 && str_eq(JsTuple_double_JsString_get_1(flipped), "x"));
    Box named = boxedName();
    CHECK(named.h != 0);

    // Arrays of objects.
    JsArray_Item all = items();
    CHECK(JsArray_Item_length(all) == 2);
    Item item = JsArray_Item_get(all, 0);
    CHECK(Item_get_id(item) == 1);
    CHECK(JsArray_JsString_length(Item_get_tags(item)) == 2);

    // Generics are erased: values are untyped.
    Box b = wrap(js_number(21));
    CHECK(js_to_number(Box_get_value(b)) == 21);
    CHECK(js_to_number(Box_get_value(Box_map(b, MapFn_from(doubled, 0)))) == 42);
    CHECK(js_to_number(first(JsArray_double_any(r))) == 0);
    CHECK(first(JsArray_new()).h == 0);

    // A bag is made in one expression, with the properties it names: `false`, 0 and "" are present, the rest absent.
    CHECK(str_eq(marked(Marks_create()), ""));
    CHECK(str_eq(marked(Marks_create(.on = false, .count = 0, .label = "")), "on=false,count=0,label="));
    CHECK(str_eq(marked(Marks_create(.label = "x", .mode = Marks_mode_narrow)), "label=x,mode=narrow"));
    // A property of a base is a field too.
    CHECK(str_eq(marked(Marks_create(.count = 2, .id = 7)), "count=2,id=7"));
    CHECK(str_eq(marked(Marks_create(.count = 1.5, .on = true)), "on=true,count=1.5"));
    CHECK(str_eq(describe(Options_create(.width = 3, .title = "hi", .mode = Options_mode_fill)), "3x? fill \"hi\""));

    // A handle a function returns is the caller's to release.
    uint32_t live = js_live_handles();
    for (int i = 0; i < 1000; i++) {
        // Made here, so released here; wrap only borrows it.
        JsValue n = js_number(i);
        Box t = wrap(n);
        js_release(n);
        JsValue v = Box_get_value(t);
        JsArray_JsString parts = words("a b");
        JsString second = JsArray_JsString_get(parts, 1);
        CHECK(js_to_number(v) == i && str_eq(second, "b"));
        js_release(second);
        js_release(parts);
        js_release(v);
        js_release(t);
    }
    CHECK(js_live_handles() == live);
}
