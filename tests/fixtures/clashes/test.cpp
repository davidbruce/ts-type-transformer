// Names that would be one C++ identifier, each given its own. Every CHECK must hold.
#include "bindings.hpp"

extern "C" TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

extern "C" __attribute__((export_name("run"))) void run() {
    Gizmo g = Gizmo::create();
    CHECK(g.isTransforming());
    Gizmo_isTransforming f = Gizmo::isTransforming_static();
    CHECK(f.h != 0);
    CHECK(g.fooBar() == 1 && g.foo_bar() == 2);

    Shape s = shape();
    CHECK(s.area() == 3 && Shape_area(s) == 30);

    // The property `create` keeps its name; the option bag's constructor is `make`.
    Point p = point(4);
    CHECK(p.x() == 4);
    CHECK(p.create().h != 0);
    CHECK(Point::make().h != 0);

    CHECK(ns_Node::create().id() == 2);
    // The namespace's `Node`, by its own name in a namespace of it.
    CHECK(ns::Node::create().id() == 1);
}
