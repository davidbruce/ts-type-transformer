// Two packages that both export `Node`, `pick` and `util`: each call reaches
// its own package's. Every CHECK must hold.
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
    // Classes of one name, each constructed from its package.
    CHECK(str_eq(Node_name(Node_new("x")), "x"));
    CHECK(pb_Node_size(pb_Node_new(3)) == 3);

    // Functions of one name: each package's.
    PickResult a = pick(1);
    CHECK(a.tag == PickResult_Node && str_eq(Node_name(a.as.Node), "a"));
    CHECK(pick(0).tag == PickResult_string);
    // Told apart by pb's class, not pa's.
    PbPickResult b = pb_pick(1);
    CHECK(b.tag == PbPickResult_Node && pb_Node_size(b.as.Node) == 10);
    CHECK(pb_pick(0).tag == PbPickResult_string);

    // pa's own Point, and geo's (transitive dep), with its `lat` field.
    Point own = Point_new();
    CHECK(Point_get_x(own) == 3);
    geo_Point geo = toGeo(own);
    CHECK(geo_Point_get_lat(geo) == 3);

    // Namespaces of one name.
    CHECK(str_eq(util_tag(), "pa"));
    CHECK(str_eq(pb_util_tag(), "pb"));
}
