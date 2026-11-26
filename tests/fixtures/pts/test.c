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

static Pts_Pt pt(double x, double y) { return Pts_Pt_new(pair(x, y)); }

__attribute__((export_name("run"))) void run(void) {
    uint32_t live = js_live_handles();
    js_scope_open();

    // A Pt is an array of coordinates, with names for the first four.
    Pts_Pt p = pt(3, 4);
    CHECK(Pts_Pt_get_x(p) == 3 && Pts_Pt_get_y(p) == 4);
    CHECK(Pts_Pt_magnitude(p) == 5);
    CHECK(JsString_equals(Pts_Pt_toString(p), "Pt(3, 4)"));
    Pts_Pt_set_id(p, "corner");
    CHECK(JsString_equals(Pts_Pt_get_id(p), "corner"));

    // A method that changes a Pt returns it; its `$` twin returns another.
    Pts_Pt moved = Pts_Pt__add(p, pair(1, 1));
    CHECK(!js_same(moved, p) && Pts_Pt_get_x(moved) == 4 && Pts_Pt_get_x(p) == 3);
    CHECK(js_same(Pts_Pt_add(p, pair(1, 1)), p) && Pts_Pt_get_x(p) == 4);
    // A union is made from a member; a trailing optional argument may be left out.
    CHECK(Pts_Pt_equals(p, Pts_PtLike_of(moved)));
    CHECK(Pts_Pt_equals(Pts_Pt_clone(p), Pts_PtLike_of_Pt(p), js_some_double(0.001)) && !js_same(Pts_Pt_clone(p), p));

    // Statics.
    CHECK(Pts_Num_lerp(0, 10, 0.5) == 5);
    CHECK(Pts_Num_clamp(12, 0, 10) == 10);
    CHECK(Pts_Geom_toDegree(3.141592653589793) == 180);
    CHECK(JsString_equals(Pts_Pt_toString(Pts_Pt_make(3, js_some_double(2))), "Pt(2, 2, 2)"));

    // A Group is an array of Pt.
    Pts_Group square = Pts_Group_new(pt(0, 0), pt(4, 0), pt(4, 4), pt(0, 4));
    CHECK(JsArray_Pts_Pt_length(Pts_Group_as_array(square)) == 4 && Pts_Pt_get_y(JsArray_Pts_Pt_get(Pts_Group_as_array(square), 2)) == 4);
    CHECK(Pts_Pt_get_x(Pts_Group_get_p1(square)) == 0);
    CHECK(JsString_equals(Pts_Pt_toString(Pts_Group_centroid(square)), "Pt(2, 2)"));
    CHECK(Pts_Pt_get_x(Pts_Group_get_p2(Pts_Group_boundingBox(square))) == 4);

    // A union parameter takes any of its members: a Group where points are wanted.
    CHECK(Pts_Pt_get_x(Pts_Geom_centroid(Pts_PtLikeIterable_of(square))) == 2);
    JsArray_Pts_Pt ends = Pts_Pt_list(pt(0, 0), pt(3, 4));
    CHECK(Pts_Line_magnitude(Pts_PtIterable_of(Pts_Group_new_list(ends))) == 5);
    Pts_Group ray = Pts_Line_fromAngle(Pts_PtLike_of(pt(1, 1)), 0, 5);
    CHECK(Pts_Pt_get_x(Pts_Group_get_p2(ray)) == 6 && Pts_Pt_get_y(Pts_Group_get_p2(ray)) == 1);

    // A scope frees every handle made in it.
    js_scope_close();
    CHECK(js_live_handles() == live);
}
