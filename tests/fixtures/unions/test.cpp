// Calls the generated C++ bindings; every CHECK must hold.
#include "bindings.hpp"

extern "C" TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

// Values coming back, told apart by std::visit.
static std::string kind(const Value& v) {
    struct {
        std::string operator()(const std::string&) { return "string"; }
        std::string operator()(double) { return "number"; }
        std::string operator()(Widget) { return "widget"; }
        std::string operator()(Options) { return "options"; }
    } visitor;
    return std::visit(visitor, v);
}

extern "C" __attribute__((export_name("run"))) void run() {
    // Values made here, one per member.
    Widget widget = Widget::create("w");
    Options options = Options::create();
    options.verbose(true);
    CHECK(describe(std::string("text")) == "string text");
    CHECK(describe(42.0) == "number 42");
    CHECK(describe(widget) == "widget w");
    CHECK(describe(options) == "options verbose=true");

    // Values made by JavaScript: typeof, instanceof, and a property only one member has.
    CHECK(kind(pick(0)) == "string" && std::get<std::string>(pick(0)) == "text");
    CHECK(kind(pick(1)) == "number" && std::get<double>(pick(1)) == 42);
    CHECK(kind(pick(2)) == "widget" && std::get<Widget>(pick(2)).name() == "w");
    CHECK(kind(pick(3)) == "options" && std::get<Options>(pick(3)).verbose());

    // A tagged union: told apart by its `kind` property.
    Shape circle = makeShape("circle", 2);
    CHECK(std::holds_alternative<Circle>(circle) && std::get<Circle>(circle).radius() == 2);
    Shape square = makeShape("square", 3);
    CHECK(std::holds_alternative<Square>(square) && std::get<Square>(square).side() == 3);
    CHECK(area(square) == 9);

    // Unions written inline are named after their members.
    CHECK(setup(true) == "bool true");
    CHECK(setup(options) == "options verbose=true");
    CHECK(std::get<Widget>(lookup("w")).name() == "w");
    CHECK(std::get<double>(lookup("abc")) == 3);

    // Members no test tells apart: typed where a value goes in, in an array too; the array of one member, or of any, goes where the union's does.
    LooseA a({.a = 1});
    LooseB b({.b = 2});
    CHECK(looseAll(ts::Array<Loose>::of(a, b)) == "loose {\"a\":1};loose {\"b\":2}");
    CHECK(looseAll(ts::Array<LooseB>::of(b, b)) == "loose {\"b\":2};loose {\"b\":2}");
    CHECK(looseAll(ts::Array<>::of(a)) == "loose {\"a\":1}");
}
