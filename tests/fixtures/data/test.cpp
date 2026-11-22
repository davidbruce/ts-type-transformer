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
    o.height(2);
    o.title("hi");
    CHECK(describe(o) == "3x2 fill \"hi\"");

    // Built by JavaScript, read here. A missing optional property is null.
    Options d = defaults();
    CHECK(d.width() == 4);
    CHECK(d.mode() == Options_mode::fit);
    CHECK(!d.height());
    CHECK(!d.title());

    // Partial<Options>.
    patch_changes changes = patch_changes::create();
    changes.width(9);
    Options patched = patch(d, changes);
    CHECK(patched.width() == 9 && patched.mode() == Options_mode::fit);

    // Pick<Options, "width" | "height">.
    area_size size = area_size::create();
    size.width(3);
    CHECK(area(size) == 9);
    size.height(5);
    CHECK(area(size) == 15);

    // keyof Options.
    CHECK(field() == field_result::title);

    // Record<string, number>.
    lookup_table table = lookup_table::create();
    table.set_at("a", 7);
    CHECK(lookup(table, "a") == 7);
    CHECK(lookup(table, "b") == -1);
    CHECK(table.at("a") == 7.0);
    CHECK(!table.at("b"));

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

    // Tuples are typed: [number, string] is ts::Tuple<double, std::string>.
    ts::Tuple<double, std::string> p = pair(1, "one");
    CHECK(p.get<0>() == 1 && p.get<1>() == "one");
    ts::Tuple<std::string, double> swapped = swap(ts::Tuple<double, std::string>(2, "two"));
    CHECK(swapped.get<0>() == "two" && swapped.get<1>() == 2);

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
    // Coming back, a bare T is told apart by what T is.
    CHECK(std::get<0>(either(5.0, false)) == 5);
    CHECK(std::get<1>(either(5.0, true)).value() == 5);
    CHECK(std::get<0>(either<std::string>("s", false)) == "s");
    // A union as a type argument is the union: Id is std::variant<double, std::string>.
    ts::Array<Id> some = ids();
    CHECK(std::get<double>(some[0]) == 1 && std::get<std::string>(some[1]) == "two");
    CHECK(std::get<std::string>(boxedId().value()) == "three");
    // A constraint is a requires-clause: largest takes arrays of Items.
    CHECK(largest(all).id() == 2);
    // A name both generic bases have is declared again on Pipe.
    Pipe pp = pipe();
    CHECK(pp.label() == "pipe");
    pp.put(4);
    CHECK(pp.take() == 4);

    // An option bag is made in one expression, and what it names is set, whatever the value: the rest is absent.
    // Inherited properties come first, then the type's own, as declared.
    CHECK(marked(Marks::create()) == "");
    CHECK(marked(Marks({.on = false, .count = 0, .label = ""})) == "on=false,count=0,label=");
    CHECK(marked(Marks({.label = "x", .mode = Marks_mode::narrow})) == "label=x,mode=narrow");
    CHECK(marked(Marks({.id = 7, .count = 2})) == "id=7,count=2");
    CHECK(marked({{.on = true, .count = 1.5}}) == "on=true,count=1.5");

    // A tuple or an array is a braced list.
    CHECK(spot({{.at = {1, 2, 3.5}, .tags = {"a", "b"}}}) == "{\"at\":[1,2,3.5],\"tags\":[\"a\",\"b\"]}");

    // A variable initialized from a template's value needs no argument list.
    Box deduced = wrap(21.0);
    Box again = deduced;
    auto inferred = wrap(2.0);
    CHECK(again.value() == 21 && inferred.value() == 2);

    // Handles are counted: what goes out of scope is freed in JavaScript too.
    uint32_t live = ts::live_handles();
    for (int i = 0; i < 1000; i++) {
        Box<double> t = wrap(double(i));
        Box<double> copy = t;
        CHECK(copy.value() == i && words("a b")[1] == "b");
    }
    CHECK(ts::live_handles() == live);
}
