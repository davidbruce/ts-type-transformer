// Timers from C++: timeouts, intervals, microtasks, animation frames and the
// clock. The callbacks run after `run` returns; `busy` says so and `finish`
// checks what they recorded. Every CHECK must hold.
#include <string>
#include <utility>
#include <vector>

#include "areas/core/Document.hpp"
#include "areas/core/Performance.hpp"
#include "areas/core/functions.hpp"
#include "types/FrameRequestCallback.hpp"
#include "types/VoidFunction.hpp"

extern "C" TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

// `Function` takes a lambda directly; the rest parameter `...arguments` is passed as an empty array.
static std::vector<Function> kept;
template <class F>
static Function function(F f) {
    Function fn([f = std::move(f)](ts::Array<>) mutable {
        f();
        return ts::Value();
    });
    kept.push_back(fn);
    return fn;
}
static double set_timeout(Function f, double ms) { return setTimeout(f, ms, ts::Array<>::create()); }
static double set_interval(Function f, double ms) { return setInterval(f, ms, ts::Array<>::create()); }

static std::string order;
static int pending = 0;
static int ticks = 0;
static double interval_id = 0;
static double started = 0;
static double elapsed = 0;
static double frame_time = 0;
static double frames = 0;
static bool cancelled_ran = false;
static bool frame_cancelled_ran = false;
static bool late_tick = false;
static double time10 = 0, time30 = 0;
static int chain = 0;
static double tick_start = 0, tick_span = 0;
static double chain_start = 0, chain_end = 0;
static std::string negative;

// Three timeouts, each set by the one before: 15ms apart, so 45ms at least.
static void link() {
    chain++;
    if (chain < 3) set_timeout(function([] { link(); }), 15);
    else {
        chain_end = get_performance().now();
        pending--;
    }
}

extern "C" __attribute__((export_name("busy"))) int busy() { return pending; }

extern "C" __attribute__((export_name("finish"))) void finish() {
    // The microtasks ran before the zero timeout, which ran before the later ones.
    CHECK(order == "sync1 sync2 micro1 micro2 nested t0 t10 t30 ");
    CHECK(order.find("micro1") < order.find("t0") && order.find("nested") < order.find("t0"));
    CHECK(time10 > 0 && time30 > time10);
    CHECK(time10 - started >= 9 && time30 - started >= 29);
    CHECK(chain == 3 && chain_end - chain_start >= 40);
    CHECK(negative == "neg ");
    CHECK(!cancelled_ran);
    // The interval ran three times and stopped: nothing after clearInterval.
    CHECK(ticks == 3 && !late_tick);
    CHECK(tick_span >= 18 && tick_span < 5000);
    CHECK(frame_time <= get_performance().now());
    CHECK(elapsed < 5000);
    // The clock moved with the timer.
    CHECK(elapsed >= 45);
    // Animation frames: a timestamp, in order, the cancelled one never ran.
    CHECK(frame_time > 0 && frames == 2 && !frame_cancelled_ran);
    CHECK(pending == 0);
    for (Function& f : kept) f.release();
}

extern "C" __attribute__((export_name("run"))) void run() {
    Performance perf = get_performance();

    // The clock never goes back.
    double last = perf.now();
    bool monotonic = last >= 0;
    for (int i = 0; i < 1000; i++) {
        double t = perf.now();
        monotonic = monotonic && t >= last;
        last = t;
    }
    CHECK(monotonic);
    CHECK(perf.timeOrigin() > 1e12);

    started = perf.now();

    // Microtasks run before timers, after the code that queued them.
    order += "sync1 ";
    pending++;
    set_timeout(function([] {
        order += "t0 ";
        pending--;
    }), 0);
    queueMicrotask([] {
        order += "micro1 ";
        // A microtask queued by a microtask runs before any timer.
        queueMicrotask([] { order += "nested "; });
    });
    queueMicrotask([] { order += "micro2 "; });
    order += "sync2 ";

    // Timers fire by time, not by the order they were made.
    pending += 2;
    set_timeout(function([] {
        order += "t30 ";
        time30 = get_performance().now();
        pending--;
    }), 30);
    set_timeout(function([] {
        order += "t10 ";
        time10 = get_performance().now();
        pending--;
    }), 10);

    // A chain of timeouts, each made by the last, and a negative delay (read as 0).
    pending++;
    chain_start = perf.now();
    set_timeout(function([] { link(); }), 15);
    pending++;
    set_timeout(function([] {
        negative += "neg ";
        pending--;
    }), -5);

    // The ids differ and are positive; a cleared timer never fires.
    double a = set_timeout(function([] { cancelled_ran = true; }), 20);
    double b = set_timeout(function([] { cancelled_ran = true; }), 20);
    CHECK(a > 0 && b > 0 && a != b);
    clearTimeout(a);
    clearTimeout(b);
    clearTimeout(std::nullopt);
    clearTimeout(99999999);

    // Elapsed time as measured by performance.now.
    pending++;
    set_timeout(function([] {
        elapsed = get_performance().now() - started;
        pending--;
    }), 50);

    // An interval counts, and clears itself after the third tick.
    pending++;
    interval_id = set_interval(function([] {
        ticks++;
        if (ticks == 1) tick_start = get_performance().now();
        if (ticks == 3) {
            tick_span = get_performance().now() - tick_start;
            clearInterval(interval_id);
            // Wait past a few more periods to see that none comes.
            set_timeout(function([] {
                pending--;
            }), 60);
        } else if (ticks > 3) {
            late_tick = true;
        }
    }), 10);
    CHECK(interval_id > 0);

    // Animation frames: called with a timestamp, and cancellable.
    pending++;
    requestAnimationFrame([](double t) {
        frame_time = t;
        frames++;
        requestAnimationFrame([](double t2) {
            frames++;
            CHECK(t2 >= frame_time);
            pending--;
        });
    });
    double cancel = requestAnimationFrame([](double) { frame_cancelled_ran = true; });
    CHECK(cancel > 0);
    cancelAnimationFrame(cancel);

    CHECK(order == "sync1 sync2 ");
    CHECK(pending > 0 && ticks == 0);
}
