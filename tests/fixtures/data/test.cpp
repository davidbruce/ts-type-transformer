// Calls the generated C++ bindings; every CHECK must hold.
#include "bindings.hpp"

extern "C" TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

extern "C" __attribute__((export_name("run"))) void run() {
    // Option objects: built here, read by JavaScript.
    Options o = Options::create();
    o.width(3);
    o.mode(Options_mode::fill);
    CHECK(describe(o) == "3x? fill");
    o.height(ts::Value::number(2));
    o.title("hi");
    CHECK(describe(o) == "3x2 fill \"hi\"");

    // Built by JavaScript, read here. A missing optional property is null.
    Options d = defaults();
    CHECK(d.width() == 4);
    CHECK(d.mode() == Options_mode::fit);
    CHECK(d.height().is_null());
    CHECK(!d.title());

    // Partial<Options>.
    patch_changes changes = patch_changes::create();
    changes.width(ts::Value::number(9));
    Options patched = patch(d, changes);
    CHECK(patched.width() == 9 && patched.mode() == Options_mode::fit);

    // Pick<Options, "width" | "height">.
    area_size size = area_size::create();
    size.width(3);
    CHECK(area(size) == 9);
    size.height(ts::Value::number(5));
    CHECK(area(size) == 15);

    // keyof Options.
    CHECK(field() == field_result::title);

    // Record<string, number>.
    lookup_table table = lookup_table::create();
    table.set_at("a", 7);
    CHECK(lookup(table, "a") == 7);
    CHECK(lookup(table, "b") == -1);
    CHECK(table.at("a").to_number() == 7);
    CHECK(table.at("b").is_null());

    // An intersection of object types.
    Tagged tagged = Tagged::create();
    tagged.name("x");
    tagged.id(5);
    CHECK(show(tagged) == "x#5");

    // Inline object types.
    norm_point point = norm_point::create();
    point.x(3);
    point.y(4);
    CHECK(norm(point) == 5);
    CHECK(origin().x() == 0);

    // Arrays are typed: T[] is ts::Array<T>.
    ts::Array<double> r = range(4);
    CHECK(r.size() == 4 && r[3] == 3);
    CHECK(sum(r) == 6);
    ts::Array<double> mine = ts::Array<double>::create();
    mine.push(10);
    mine.push(5);
    CHECK(sum(mine) == 15);
    CHECK(sum({1, 2, 3}) == 6);
    CHECK(words("a bc")[1] == "bc");

    // Tuples are untyped arrays.
    ts::Array<> p = pair(1, "one");
    CHECK(p[0].to_number() == 1);
    ts::Array<> swapped = swap(p);
    CHECK(swapped[0].to_string() == "one" && swapped[1].to_number() == 1);

    // Arrays of objects.
    ts::Array<Item> all = items();
    CHECK(all.size() == 2 && all[0].id() == 1 && all[0].tags().size() == 2 && all[0].tags()[1] == "b");

    // Generics are templates: a T crosses as a handle and comes back a T.
    Box<double> b = wrap(21.0);
    CHECK(b.value() == 21);
    CHECK(b.map<double>([](double v) { return v * 2; }).value() == 42);
    Box<std::string> text = b.map<std::string>([](double v) { return std::to_string(int(v)); });
    CHECK(text.value() == "21");
    CHECK(first(r) == 0.0);
    CHECK(!first(ts::Array<double>::create()));
    // A method's T, apart from its interface's.
    CHECK(b.replace<std::string>("x").value() == "x");
    // A generic union alias is an alias template: Either<double> is std::variant<double, Box<double>>.
    CHECK(unbox<double>(5.0) == 5);
    CHECK(unbox<double>(b) == 21);
    // A constraint is a requires-clause: largest takes arrays of Items.
    CHECK(largest(all).id() == 2);
    // A name both generic bases have is declared again on Pipe.
    Pipe pp = pipe();
    CHECK(pp.label() == "pipe");
    pp.put(4);
    CHECK(pp.take() == 4);
}
