// Calls the generated C++ bindings; every CHECK must hold.
#include "bindings.hpp"

extern "C" TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

extern "C" __attribute__((export_name("run"))) void run() {
    Counter c = Counter::create();
    double last = 0;
    int calls = 0;

    // A callback is a lambda; keep it to pass the same one again.
    Listener onValue = [&](double value) {
        last = value;
        calls++;
    };
    c.subscribe(onValue);
    CHECK(c.increment() == 1);
    CHECK(last == 1);
    c.increment(4);
    CHECK(last == 5 && calls == 2);

    // A callback that takes an object and returns a value.
    CHECK(c.visit([](Counter counter) { return counter.count() == 5; }));

    // The same Listener is the same JavaScript function.
    c.unsubscribe(onValue);
    c.increment();
    CHECK(last == 5 && calls == 2);

    // A nullable callback property.
    CHECK(!c.onchange());
    double prop = 0;
    c.onchange(Listener([&](double value) { prop = value; }));
    CHECK(c.onchange().has_value());
    c.increment();
    CHECK(prop == 7);
    c.onchange(std::nullopt);
    c.increment();
    CHECK(prop == 7);

    // Several arguments, a string among them, and a result.
    CHECK(c.fold([](double total, std::string label) { return total * 2 + label.size(); }) == 8 * 2 + 5);

    // A callback kept and called later.
    int ran = 0;
    later([&] { ran++; });
    later([&] { ran++; });
    CHECK(ran == 0);
    CHECK(flush() == 2);
    CHECK(ran == 2);

    // A union argument arrives as the variant.
    double number = 0;
    std::string text;
    CHECK(eachId([&](Id id) {
        if (auto* n = std::get_if<double>(&id)) number = *n;
        else text = std::get<std::string>(id);
    }) == 2);
    CHECK(number == 1 && text == "two");
}
