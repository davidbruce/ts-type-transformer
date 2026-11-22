// Calls the generated C++ bindings; every CHECK must hold.
#include "bindings.hpp"

extern "C" TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

extern "C" __attribute__((export_name("run"))) void run() {
    // Global values.
    Settings s = get_settings();
    CHECK(s.level() == 2 && !s.debug());
    s.debug(true);
    CHECK(get_settings().debug());
    CHECK(get_VERSION() == "1.2.3");

    // A global with members of its own.
    CHECK(Clock::now() == 1000);
    CHECK(Clock::ZERO == 0);

    // Namespaces, nested ones included.
    Geometry::Point a = Geometry::point(0, 0);
    Geometry::Point b = Geometry::point(3, 4);
    CHECK(Geometry::distance(a, b) == 5);
    CHECK(b.x() == 3);
    CHECK(Geometry::get_UNIT() == 1);
    CHECK(Geometry::Deep::twice(4) == 8);

    // Rest parameters take an array; overloads keep the name.
    CHECK(join("-", {"a", "b", "c"}) == "a-b-c");
    CHECK(join("-", {}) == "");
    // An array of anything, given alone, is the argument list.
    CHECK(arity(1, 2, 3) == 3 && arity() == 0);
    CHECK(arity(ts::Array<double>::of(1, 2, 3)) == 3 && arity(ts::Array<>::of(1, 2)) == 2);
    CHECK(arity(ts::Value(ts::Array<double>::of(1, 2, 3))) == 1);

    // Numeric enums.
    CHECK(bump(Level::Low) == Level::High);
    CHECK(bump(Level::High) == Level::High);
}
