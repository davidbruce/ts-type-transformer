// Calls a module library through the generated C bindings; every CHECK must hold.
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
    // A default export, reached as `default`.
    CHECK(Exports_Point_get_x(exports_origin()) == 0);

    // Exported under other names: called and constructed by them.
    CHECK(exports_area(3, 4) == 12);
    Exports_Figure f = Exports_Figure_new(2);
    CHECK(Exports_Figure_grow(f) == 3 && Exports_Figure_get_size(f) == 3);
    CHECK(str_eq(Exports_Badge_get_label(Exports_Badge_new("x")), "x"));

    // Constants are read, never assigned: there is no set_LIMIT.
    CHECK(exports_get_LIMIT() == 10);
    CHECK(exports_get_counter() == 0 && exports_bump() == 1 && exports_get_counter() == 1);

    // A union told apart by the classes as exported.
    CHECK(exports_pick(0).tag == Exports_PickResult_Figure && Exports_Figure_get_size(exports_pick(0).as.Figure) == 2);
    CHECK(exports_pick(1).tag == Exports_PickResult_Badge);

    // A tuple is told apart as an array, not by `toString`, which every object has.
    CHECK(exports_locate(0).tag == Exports_LocateResult_array);
    CHECK(exports_locate(1).tag == Exports_LocateResult_Point);

    // Rollup's namespace, by the name it is exported as: `util.half`.
    CHECK(exports_util_half(4) == 2 && exports_get_util_twice().h != 0);
}
