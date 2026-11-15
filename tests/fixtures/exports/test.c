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
    CHECK(Point_get_x(origin()) == 0);

    // Exported under other names: called and constructed by them.
    CHECK(area(3, 4) == 12);
    Figure f = Figure_new(2);
    CHECK(Figure_grow(f) == 3 && Figure_get_size(f) == 3);
    CHECK(str_eq(Badge_get_label(Badge_new("x")), "x"));

    // Constants are read, never assigned: there is no set_LIMIT.
    CHECK(get_LIMIT() == 10);
    CHECK(get_counter() == 0 && bump() == 1 && get_counter() == 1);

    // A union told apart by the classes as exported.
    CHECK(pick(0).tag == Figure_or_Badge_Figure && Figure_get_size(pick(0).as.Figure) == 2);
    CHECK(pick(1).tag == Figure_or_Badge_Badge);
}
