// Names that would be one C identifier, each given its own. Every CHECK must hold.
#include "bindings.h"

TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

__attribute__((export_name("run"))) void run(void) {
    // The static and instance members keep distinct C names.
    Gizmo g = Gizmo_new();
    CHECK(!Gizmo_isTransforming_2());
    CHECK(Gizmo_isTransforming(g));
    CHECK(Gizmo_fooBar(g) == 1 && Gizmo_foo_bar(g) == 2);

    // The method `Shape_area` keeps its name; the function of that name gives way.
    Shape s = shape();
    CHECK(Shape_area(s) == 3 && Shape_area_2(s) == 30);

    // A property `create` beside the option bag's constructor.
    Point p = point(4);
    CHECK(Point_get_x(p) == 4);
    CHECK(Point_get_x(Point_create()) != 4);
    CHECK(Point_get_x(Point_create_2(p, 5)) == 5);

    // A class keeps its name; a namespace's class that flattens to it gives way.
    CHECK(ns_Node_id(ns_Node_new()) == 2);
    CHECK(ns_Node_2_id(ns_Node_2_new()) == 1);
}
