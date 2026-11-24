// Calls a module library through the generated C++ bindings; every CHECK must hold.
#include "bindings.hpp"

extern "C" TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

extern "C" __attribute__((export_name("run"))) void run() {
    // A default export, reached as `default`.
    CHECK(origin().x() == 0);

    // Exported under other names: called and constructed by them.
    CHECK(area(3, 4) == 12);
    Figure f = Figure::create(2);
    CHECK(f.grow() == 3 && f.size() == 3);
    CHECK(Badge::create("x").label() == "x");

    // Constants are read, never assigned: there is no LIMIT setter.
    CHECK(get_LIMIT() == 10);
    CHECK(get_counter() == 0 && bump() == 1 && get_counter() == 1);

    // A union told apart by the classes as exported.
    CHECK(std::holds_alternative<Figure>(pick(0)) && std::get<Figure>(pick(0)).size() == 2);
    CHECK(std::holds_alternative<Badge>(pick(1)));

    // A tuple is told apart as an array, not by `toString`, which every object has.
    CHECK((std::holds_alternative<ts::Tuple<double, double>>(locate(0))));
    CHECK(std::holds_alternative<Point>(locate(1)));

    // Rollup's namespace, by the name it is exported as: `util.half`.
    CHECK(util::half(4) == 2 && util::get_twice()(4) == 8);
}
