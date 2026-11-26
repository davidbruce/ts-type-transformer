// Calls the generated C bindings; every CHECK must hold.
#include "bindings.h"

TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

static int calls;
static void on_value(void* ctx, double value) { *(double*)ctx = value; calls++; }
static double fold(void* ctx, double total, JsString label) { (void)ctx; return total * 2 + JsString_len(label); }
static bool is_five(void* ctx, Counter c) { (void)ctx; return Counter_get_count(c) == 5; }
static void bump(void* ctx) { (*(int*)ctx)++; }

static bool str_eq(JsString s, const char* expected) {
    char buf[64];
    uint32_t n = JsString_len(s);
    if (n != ts_strlen(expected) || n > sizeof buf) return false;
    JsString_read(s, buf);
    for (uint32_t i = 0; i < n; i++)
        if (buf[i] != expected[i]) return false;
    return true;
}

// What the promises settled with, checked by `finish`.
static double awaited;
static bool reason_ok;
static void on_delayed(void* ctx, bool ok, JsValue v) { (void)ctx; if (ok) awaited = js_to_number(v); }
static void on_rejected(void* ctx, bool ok, JsValue v) {
    (void)ctx;
    if (ok) return;
    JsString m = js_error_message(v);
    reason_ok = str_eq(m, "nope");
    js_release(m);
}

// Sums what an async iterator yields, asking for each value when the last arrives.
static double ticks;
static JsIterator tick_it;
static void on_tick(void* ctx, bool ok, JsValue v) {
    (void)ctx;
    if (!ok || v.h == 0) {
        js_release(tick_it);
        return;
    }
    ticks += js_to_number(v);
    Promise p = JsIterator_next_async(tick_it);
    Promise_then(p, on_tick, 0);
    js_release(p);
}

// A union argument arrives as a lent handle.
static int ids_seen;
static bool first_is_one;
static void on_id(void* ctx, JsValue id) { (void)ctx; first_is_one |= ids_seen++ == 0 && js_to_number(id) == 1; }

__attribute__((export_name("finish"))) void finish(void) {
    CHECK(ticks == 6);
    CHECK(awaited == 42);
    CHECK(reason_ok);
}

__attribute__((export_name("run"))) void run(void) {
    // A promise calls a function once it settles.
    Promise p = delayed(21);
    Promise_then(p, on_delayed, 0);
    js_release(p);
    Promise q = rejected("nope");
    Promise_then(q, on_rejected, 0);
    js_release(q);
    CHECK(awaited == 0);
    CHECK(eachId(EachIdFn_from(on_id, 0)) == 2);
    CHECK(ids_seen == 2 && first_is_one);
    Ticker tk = ticker(3);
    tick_it = Ticker_iterate_async(tk);
    js_release(tk);
    Promise first = JsIterator_next_async(tick_it);
    Promise_then(first, on_tick, 0);
    js_release(first);

    // An object that is also a function, and can be constructed.
    Scaler s = scaler(2);
    Scaler t = Scaler_construct(s, 3);
    CHECK(Scaler_invoke(s, 5) == 10 && Scaler_get_factor(s) == 2 && Scaler_invoke(t, 5) == 15);
    js_release(t);
    js_release(s);

    // A call that throws returns zero; its exception is kept to take.
    CHECK(explode("boom") == 0);
    JsValue e = js_take_error();
    JsString m = js_error_message(e);
    CHECK(e.h != 0 && str_eq(m, "boom"));
    js_release(m);
    js_release(e);
    CHECK(js_take_error().h == 0 && !js_failed());
    explode("again");
    CHECK(js_failed());
    js_release(js_take_error());

    Counter c = Counter_new();
    double last = 0;

    // A callback is called with its context.
    Counter_subscribe(c, Listener_from(on_value, &last));
    CHECK(Counter_increment(c) == 1);
    CHECK(last == 1);
    Counter_increment(c, js_some_double(4));
    CHECK(last == 5 && calls == 2);

    // A callback that takes an object and returns a value.
    CHECK(Counter_visit(c, VisitFn_from(is_five, 0)));

    // The same function and context is the same JavaScript function.
    Counter_unsubscribe(c, Listener_from(on_value, &last));
    Counter_increment(c);
    CHECK(last == 5 && calls == 2);

    // A nullable callback property.
    CHECK(Counter_get_onchange(c).h == 0);
    double prop = 0;
    Counter_set_onchange(c, Listener_from(on_value, &prop));
    CHECK(Counter_get_onchange(c).h != 0);
    Counter_increment(c);
    CHECK(prop == 7);
    Counter_set_onchange(c, (Listener){ 0 });
    Counter_increment(c);
    CHECK(prop == 7);

    // Several arguments, a string among them, and a result.
    CHECK(Counter_fold(c, FoldFn_from(fold, 0)) == 8 * 2 + 5);

    // A callback kept and called later.
    int ran = 0;
    later(LaterFn_from(bump, &ran));
    later(LaterFn_from(bump, &ran));
    CHECK(ran == 0);
    CHECK(flush() == 2);
    CHECK(ran == 2);
}
