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

    // Arrays.
    ts::Array r = range(4);
    CHECK(r.size() == 4 && r[3].to_number() == 3);
    CHECK(sum(r) == 6);
    ts::Array mine = ts::Array::create();
    mine.push(10.0);
    mine.push(5.0);
    CHECK(sum(mine) == 15);
    CHECK(words("a bc")[1].to_string() == "bc");

    // Tuples are arrays.
    ts::Array p = pair(1, "one");
    CHECK(p[0].to_number() == 1);
    ts::Array swapped = swap(p);
    CHECK(swapped[0].to_string() == "one" && swapped[1].to_number() == 1);

    // Arrays of objects.
    ts::Array all = items();
    Item item(ts::handle, all[0].h);
    CHECK(all.size() == 2 && item.id() == 1 && item.tags().size() == 2);

    // Generics are templates: a T crosses as a handle and comes back a T.
    Box<double> b = wrap(21.0);
    CHECK(b.value() == 21);
    CHECK(b.map<double>([](double v) { return v * 2; }).value() == 42);
    Box<std::string> text = b.map<std::string>([](double v) { return std::to_string(int(v)); });
    CHECK(text.value() == "21");
    CHECK(first<double>(r) == 0.0);
    CHECK(!first<double>(ts::Array::create()));
}
