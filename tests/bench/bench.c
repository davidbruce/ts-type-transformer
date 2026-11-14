// The benchmark through the C bindings.
#include "bindings.h"

#define BENCH(name) __attribute__((export_name(#name))) double name(int n)

BENCH(calls) {
    double s = 0;
    for (int i = 0; i < n; i++) s = add(s, i + 0.5);
    return s;
}

BENCH(objects) {
    double s = 0;
    for (int i = 0; i < n; i++) {
        Vec2 v = Vec2_new(i, 1);
        s += Vec2_length(v);
        js_release(v);
    }
    return s;
}

BENCH(properties) {
    Vec2 v = Vec2_new(0, 0);
    double s = 0;
    for (int i = 0; i < n; i++) {
        Vec2_set_x(v, i);
        s += Vec2_get_x(v);
    }
    js_release(v);
    return s;
}

BENCH(strings_in) {
    double s = 0;
    for (int i = 0; i < n; i++) s += count("hello world");
    return s;
}

BENCH(strings_fresh) {
    double s = 0;
    char text[] = "hello world";
    for (int i = 0; i < n; i++) {
        text[0] = 'a' + i % 26;
        s += count(text);
    }
    return s;
}

BENCH(strings_out) {
    double s = 0;
    char buf[32];
    for (int i = 0; i < n; i++) {
        JsString g = greet("you");
        s += JsString_copy(g, buf, sizeof buf) + buf[0];
        js_release(g);
    }
    return s;
}

static void on_i(void* ctx, double i) { *(double*)ctx += i; }

BENCH(callbacks) {
    double s = 0;
    each_f f = each_f_from(on_i, &s);
    each(n, f);
    js_forget_callback(f);
    js_release(f);
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
