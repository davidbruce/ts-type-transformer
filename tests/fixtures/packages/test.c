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
    Node_or_string a = pick(1);
    CHECK(a.tag == Node_or_string_Node && str_eq(Node_name(a.as.Node), "a"));
    CHECK(pick(0).tag == Node_or_string_string);
    // Told apart by pb's class, not pa's.
    pb_Node_or_string b = pb_pick(1);
    CHECK(b.tag == pb_Node_or_string_pb_Node && pb_Node_size(b.as.pb_Node) == 10);
    CHECK(pb_pick(0).tag == pb_Node_or_string_string);

    // pa's own Point, and geo's, a type declared elsewhere of the same name.
    Point own = Point_new();
    CHECK(Point_get_x(own) == 3);
    Point_2 geo = toGeo(own);
    CHECK(geo.h != 0);

    // Namespaces of one name.
    CHECK(str_eq(util_tag(), "pa"));
    CHECK(str_eq(pb_util_tag(), "pb"));
}
