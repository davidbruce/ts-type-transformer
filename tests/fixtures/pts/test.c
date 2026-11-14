// Calls Pts, a real library, through its generated C bindings; every CHECK must hold.
#include "bindings.h"

TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

// Coordinates: a Pt's constructor, and its arithmetic, take any number of them.
static JsArray pair(double x, double y) {
    JsArray a = JsArray_new();
    JsArray_push(a, js_number(x));
    JsArray_push(a, js_number(y));
    return a;
}

static Pt pt(double x, double y) { return Pt_new(pair(x, y)); }

__attribute__((export_name("run"))) void run(void) {
    uint32_t live = js_live_handles();
    js_scope_open();

    // A Pt is an array of coordinates, with names for the first four.
    Pt p = pt(3, 4);
    CHECK(Pt_get_x(p) == 3 && Pt_get_y(p) == 4);
    CHECK(Pt_magnitude(p) == 5);
    CHECK(JsString_equals(Pt_toString(p), "Pt(3, 4)"));
    Pt_set_id(p, "corner");
    CHECK(JsString_equals(Pt_get_id(p), "corner"));

    // A method that changes a Pt returns it; its `$` twin returns another.
    Pt moved = Pt__add(p, pair(1, 1));
    CHECK(!js_same(moved, p) && Pt_get_x(moved) == 4 && Pt_get_x(p) == 3);
    CHECK(js_same(Pt_add(p, pair(1, 1)), p) && Pt_get_x(p) == 4);
    // A union is made from a member; a trailing optional argument may be left out.
    CHECK(Pt_equals(p, PtLike_of(moved)));
    CHECK(Pt_equals(Pt_clone(p), PtLike_of_Pt(p), js_some_double(0.001)) && !js_same(Pt_clone(p), p));

    // Statics.
    CHECK(Num_lerp(0, 10, 0.5) == 5);
    CHECK(Num_clamp(12, 0, 10) == 10);
    CHECK(Geom_toDegree(3.141592653589793) == 180);
    CHECK(JsString_equals(Pt_toString(Pt_make(3, js_some_double(2))), "Pt(2, 2, 2)"));

    // A Group is an array of Pt.
    JsArray_Pt corners = JsArray_Pt_new();
    JsArray_Pt_push(corners, pt(0, 0));
    JsArray_Pt_push(corners, pt(4, 0));
    JsArray_Pt_push(corners, pt(4, 4));
    JsArray_Pt_push(corners, pt(0, 4));
    Group square = Group_new(corners);
    CHECK(JsArray_Pt_length(Group_as_array(square)) == 4 && Pt_get_y(JsArray_Pt_get(Group_as_array(square), 2)) == 4);
    CHECK(Pt_get_x(Group_get_p1(square)) == 0);
    CHECK(JsString_equals(Pt_toString(Group_centroid(square)), "Pt(2, 2)"));
    CHECK(Pt_get_x(Group_get_p2(Group_boundingBox(square))) == 4);

    // A union parameter takes any of its members: a Group where points are wanted.
    CHECK(Pt_get_x(Geom_centroid(PtLikeIterable_of(square))) == 2);
    JsArray_Pt ends = JsArray_Pt_new();
    JsArray_Pt_push(ends, pt(0, 0));
    JsArray_Pt_push(ends, pt(3, 4));
    CHECK(Line_magnitude(PtIterable_of(Group_new(ends))) == 5);
    Group ray = Line_fromAngle(PtLike_of(pt(1, 1)), 0, 5);
    CHECK(Pt_get_x(Group_get_p2(ray)) == 6 && Pt_get_y(Group_get_p2(ray)) == 1);

    // A scope frees every handle made in it.
    js_scope_close();
    CHECK(js_live_handles() == live);
}
