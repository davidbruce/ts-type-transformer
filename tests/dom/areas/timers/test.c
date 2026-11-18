// Timers from C: setTimeout and setInterval and their clearing, microtasks,
// animation frames and the performance clock. The callbacks run later: busy()
// waits for them and finish() checks what they left. Every CHECK must hold.
#include "bindings.h"

TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)
#define STR(s, text) JsString_equals((s), (text))

static TimerHandler handler(Function_fn fn, void* ctx) {
    return TimerHandler_of_Function(Function_from(fn, ctx));
}
static double after(double ms, Function_fn fn, void* ctx) { return setTimeout(handler(fn, ctx), js_some_double(ms), (JsArray){ 0 }); }

static char order[32];
static int order_len;
static void note(char c) { if (order_len < 31) { order[order_len++] = c; order[order_len] = 0; } }
static bool order_is(const char* text) {
    int i = 0;
    while (text[i] && i < order_len && order[i] == text[i]) i++;
    return text[i] == 0 && i == order_len;
}

static int pending;
__attribute__((export_name("busy"))) int busy(void) { return pending; }

static int sequence_len;
static char sequence[8];
static JsValue tick(void* ctx, JsArray args) { (void)args; sequence[sequence_len++] = *(char*)ctx; pending--; return (JsValue){ 0 }; }
static char tag_a = 'a', tag_b = 'b', tag_c = 'c';

static int timeout_fired, cancelled_fired;
static double started, fired_at;
static JsValue on_timeout(void* ctx, JsArray args) { (void)ctx; (void)args; timeout_fired++; fired_at = Performance_now(get_performance()); pending--; return (JsValue){ 0 }; }
static JsValue on_cancelled(void* ctx, JsArray args) { (void)ctx; (void)args; cancelled_fired++; return (JsValue){ 0 }; }

static int interval_count, count_after_clear;
static double interval_id;
static double tick_times[3];
static JsValue on_interval(void* ctx, JsArray args) {
    (void)ctx;
    (void)args;
    tick_times[interval_count] = Performance_now(get_performance());
    interval_count++;
    if (interval_count == 3) {
        clearInterval(js_some_double(interval_id));
        pending--;
    }
    return (JsValue){ 0 };
}
static JsValue after_clear(void* ctx, JsArray args) { (void)ctx; (void)args; count_after_clear = interval_count; pending--; return (JsValue){ 0 }; }

static char n_tag = 'n';
static void micro_inner(void* ctx) { note(*(char*)ctx); }
static void micro(void* ctx) { note(*(char*)ctx); queueMicrotask(VoidFunction_from(micro_inner, &n_tag)); }
static JsValue zero_timeout(void* ctx, JsArray args) { (void)args; note(*(char*)ctx); pending--; return (JsValue){ 0 }; }
static char m = 'm', t = 't';

static int frames;
static double frame_times[2];
static double frame_id[2];
static int cancelled_frame_fired;
static void on_frame2(void* ctx, double time) { (void)ctx; frame_times[1] = time; frames++; pending--; }
static void on_frame(void* ctx, double time) {
    (void)ctx;
    frame_times[0] = time;
    frames++;
    frame_id[1] = requestAnimationFrame(FrameRequestCallback_from(on_frame2, 0));
}
static void on_cancelled_frame(void* ctx, double time) { (void)ctx; (void)time; cancelled_frame_fired++; }

static int idle_fired;
static bool idle_remaining_ok;
static void on_idle(void* ctx, IdleDeadline deadline) {
    (void)ctx;
    idle_fired++;
    idle_remaining_ok = IdleDeadline_timeRemaining(deadline) >= 0;
    pending--;
}

__attribute__((export_name("run"))) void run(void) {
    Performance perf = get_performance();
    started = Performance_now(perf);

    // The clock never runs backwards, and starts after the time origin.
    double last = started;
    bool monotonic = true;
    for (int i = 0; i < 2000; i++) {
        double now = Performance_now(perf);
        if (now < last) monotonic = false;
        last = now;
    }
    CHECK(monotonic);
    CHECK(started > 0 && Performance_get_timeOrigin(perf) > 1e12);
    CHECK(Performance_get_timeOrigin(perf) + last <= js_date_now() + 5);

    // Microtasks run after the code that queued them, before any timeout.
    note('A');
    pending++;
    queueMicrotask(VoidFunction_from(micro, &m));
    after(0, zero_timeout, &t);
    note('B');
    CHECK(order_is("AB"));

    // One timeout fires once, after about the time asked; a cleared one never fires.
    pending++;
    after(40, on_timeout, 0);
    double doomed = after(10, on_cancelled, 0);
    CHECK(doomed > 0);
    clearTimeout(js_some_double(doomed));
    clearTimeout(js_some_double(987654));  // an id that is not live is ignored

    // Timeouts fire in order of their delay, not of their making.
    pending += 3;
    double id_a = after(60, tick, &tag_a);
    double id_b = after(20, tick, &tag_b);
    double id_c = after(40, tick, &tag_c);
    CHECK(id_a != id_b && id_b != id_c && id_a != id_c);

    // An interval ticks until it is cleared, then stays quiet.
    pending += 2;
    interval_id = setInterval(handler(on_interval, 0), js_some_double(15), (JsArray){ 0 });
    CHECK(interval_id > 0);
    after(250, after_clear, 0);

    // Animation frames: the callback gets a timestamp; a cancelled frame never runs.
    pending++;
    frame_id[0] = requestAnimationFrame(FrameRequestCallback_from(on_frame, 0));
    double dropped = requestAnimationFrame(FrameRequestCallback_from(on_cancelled_frame, 0));
    cancelAnimationFrame(dropped);
    CHECK(frame_id[0] > 0 && dropped != frame_id[0]);

    // Idle callbacks get a deadline.
    pending++;
    double idle_id = requestIdleCallback(IdleRequestCallback_from(on_idle, 0));
    CHECK(idle_id > 0);

    // Marks and measures on the performance timeline.
    Performance_mark(perf, "t-start");
    Performance_mark(perf, "t-end");
    PerformanceMeasure span = Performance_measure(perf, "t-span", "t-start", "t-end");
    CHECK(PerformanceEntry_get_duration(span) >= 0 && STR(PerformanceEntry_get_name(span), "t-span"));
    CHECK(JsArray_length((JsArray){ Performance_getEntriesByName(perf, "t-start", "mark").h }) == 1);
    CHECK(JsArray_length((JsArray){ Performance_getEntriesByType(perf, "measure").h }) == 1);
    Performance_clearMarks(perf, 0);
    CHECK(JsArray_length((JsArray){ Performance_getEntriesByType(perf, "mark").h }) == 0);
}

__attribute__((export_name("finish"))) void finish(void) {
    CHECK(pending == 0);
    // The microtask ran before the zero-delay timeout.
    CHECK(order_is("ABmnt"));
    CHECK(timeout_fired == 1 && cancelled_fired == 0);
    CHECK(fired_at - started >= 35);
    CHECK(sequence_len == 3 && sequence[0] == 'b' && sequence[1] == 'c' && sequence[2] == 'a');
    CHECK(interval_count == 3 && count_after_clear == 3);
    CHECK(frames == 2 && cancelled_frame_fired == 0);
    CHECK(frame_times[0] > 0 && frame_times[1] > frame_times[0]);
    double now = Performance_now(get_performance());
    CHECK(frame_times[1] <= now);
    CHECK(now - started >= 245);
    CHECK(tick_times[1] - tick_times[0] >= 10 && tick_times[2] - tick_times[1] >= 10);
    CHECK(tick_times[0] > started && tick_times[2] < fired_at + 250);
    CHECK(idle_fired == 1 && idle_remaining_ok);
}
