// Calls the generated C bindings; every CHECK must hold.
#include "bindings.h"

TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

static int calls;
static void on_value(void* ctx, double value) { *(double*)ctx = value; calls++; }
static double fold(void* ctx, double total, JsString label) { return total * 2 + JsString_len(label); }
static bool is_five(void* ctx, Counter c) { return Counter_get_count(c) == 5; }
static void bump(void* ctx) { (*(int*)ctx)++; }

__attribute__((export_name("run"))) void run(void) {
    Counter c = Counter_new();
    double last = 0;

    // A callback is called with its context.
    Counter_subscribe(c, Listener_from(on_value, &last));
    CHECK(Counter_increment(c) == 1);
    CHECK(last == 1);
    Counter_increment_with_by(c, 4);
    CHECK(last == 5 && calls == 2);

    // A callback that takes an object and returns a value.
    CHECK(Counter_visit(c, Counter_visit_f_from(is_five, 0)));

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
    CHECK(Counter_fold(c, Counter_fold_f_from(fold, 0)) == 8 * 2 + 5);

    // A callback kept and called later.
    int ran = 0;
    later(later_f_from(bump, &ran));
    later(later_f_from(bump, &ran));
    CHECK(ran == 0);
    CHECK(flush() == 2);
    CHECK(ran == 2);
}
