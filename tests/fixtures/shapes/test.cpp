// Calls the generated C++ bindings; every CHECK must hold.
#include "bindings.hpp"

extern "C" TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

extern "C" __attribute__((export_name("run"))) void run() {
    // Class pairing: a constructor, a static method, a constant.
    Shape s = Shape::create(Shape_kind::square, 3);
    CHECK(s.area() == 9);
    CHECK(Shape::unit().area() == 1);
    CHECK(Shape::SIDES == 4);

    // Literal union -> enum class, in both directions.
    CHECK(s.kind() == Shape_kind::square);
    CHECK(Shape::create(Shape_kind::circle, 1).kind() == Shape_kind::circle);

    // Properties: a getter and a setter of the same name.
    s.size(4);
    CHECK(s.size() == 4);

    // Inherited members, and a Shape where a Named is wanted.
    CHECK(s.name() == "square-3");
    CHECK(s.describe() == "square-3 (16)");
    CHECK(nameOf(s) == "square-3");

    // Overloads keep their name.
    Shape big = s.scale(2);
    CHECK(big.size() == 8);
    CHECK(s.scale(2, "big").name() == "big");

    // Optional parameter: an overload without it.
    s.resize(2.6);
    CHECK(s.size() == 2.6);
    s.resize(2.6, true);
    CHECK(s.size() == 3);

    // Union parameter: a std::variant.
    CHECK(s.label(std::string("a")) == "string:a");
    CHECK(s.label(7.0) == "number:7");

    // Nullable: std::optional.
    CHECK(!s.parent());
    CHECK(big.parent() == s);
    big.parent(std::nullopt);
    CHECK(!big.parent());

    // Free functions.
    CHECK(totalArea(s, Shape::unit()) == 10);
    CHECK(largest(s, std::nullopt) == s);
    CHECK(largest(Shape::unit(), s) == s);

    // `undefined` rather than `null`.
    CHECK(orUnit(std::nullopt).area() == 1);
    CHECK(orUnit(s) == s);
    CHECK(!find("nothing"));
    CHECK(find("unit")->area() == 1);
}
