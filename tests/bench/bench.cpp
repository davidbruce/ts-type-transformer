// The benchmark through the C++ bindings.
#include "bindings.hpp"

#define BENCH(name) extern "C" __attribute__((export_name(#name))) double name(int n)

BENCH(calls) {
    double s = 0;
    for (int i = 0; i < n; i++) s = add(s, i + 0.5);
    return s;
}

BENCH(objects) {
    double s = 0;
    for (int i = 0; i < n; i++) s += Vec2::create(i, 1).length();
    return s;
}

BENCH(properties) {
    Vec2 v = Vec2::create(0, 0);
    double s = 0;
    for (int i = 0; i < n; i++) {
        v.x(i);
        s += v.x();
    }
    return s;
}

BENCH(strings_in) {
    double s = 0;
    for (int i = 0; i < n; i++) s += count("hello world");
    return s;
}

BENCH(strings_fresh) {
    double s = 0;
    std::string text = "hello world";
    for (int i = 0; i < n; i++) {
        text[0] = 'a' + i % 26;
        s += count(text);
    }
    return s;
}

BENCH(strings_out) {
    double s = 0;
    for (int i = 0; i < n; i++) {
        std::string g = greet("you");
        s += g.size() + g[0];
    }
    return s;
}

BENCH(callbacks) {
    double s = 0;
    ts::Scoped<each_f> f = [&](double i) { s += i; };
    each(n, f);
    return s;
}

BENCH(compute) {
    uint32_t x = 1;
    double s = 0;
    for (int i = 0; i < n; i++) {
        x = x * 1664525u + 1013904223u;
        s += x >> 24;
    }
    return s;
}
