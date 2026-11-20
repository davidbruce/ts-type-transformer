#include <cstdint>
#include <string>
#include <string_view>

#include "node/node.hpp"

using namespace node;

extern "C" TS_IMPORT(print) void host_print(const char* ptr, uint32_t len);

static void println(std::string_view s) {
    host_print(s.data(), static_cast<uint32_t>(s.size()));
    host_print("\n", 1);
}

TS_EXPORT(start) void start() {
    int passed = 0;
    auto check = [&](bool ok) {
        if (!ok) __builtin_trap();
        passed++;
    };

    auto emitter = EventEmitter<ts::Value>::create();
    emitter.setMaxListeners(12);
    check(emitter.getMaxListeners() == 12);

    int metric_calls = 0;
    double metric_sum = 0.0;
    int once_calls = 0;
    int shutdown_calls = 0;

    auto ev_metric = ts::Value::string("metric");
    auto ev_shutdown = ts::Value::string("shutdown");
    auto ev_idle = ts::Value::string("idle");

    ts::Scoped<any_array_fn> on_metric = [&](ts::Array<> args) {
        metric_calls++;
        if (args.size() > 0) {
            metric_sum += args[0].to_number();
        }
    };
    ts::Scoped<any_array_fn> on_once = [&](ts::Array<>) { once_calls++; };
    ts::Scoped<any_array_fn> on_shutdown = [&](ts::Array<>) { shutdown_calls++; };

    emitter.on(ev_metric, on_metric);
    emitter.once(ev_metric, on_once);
    emitter.on(ev_shutdown, on_shutdown);

    check(emitter.listenerCount(ev_metric) == 2);
    check(emitter.eventNames().size() == 2);

    bool emitted1 = emitter.emit(ev_metric, 101, "ok");
    check(emitted1 && metric_calls == 1 && once_calls == 1);
    check(emitter.listenerCount(ev_metric) == 1);

    bool emitted2 = emitter.emit(ev_metric, 202, "ok");
    check(emitted2 && metric_calls == 2 && metric_sum == 303.0 && once_calls == 1);

    check(!emitter.emit(ev_idle));

    emitter.removeListener(ev_metric, on_metric);
    check(emitter.listenerCount(ev_metric) == 0 && !emitter.emit(ev_metric, 1));
    check(metric_calls == 2);

    emitter.removeAllListeners(ev_shutdown);
    check(emitter.eventNames().size() == 0 && shutdown_calls == 0);

    println("maxListeners=" + std::to_string(static_cast<int>(emitter.getMaxListeners())) +
            " metricCalls=" + std::to_string(metric_calls) +
            " metricSum=" + std::to_string(static_cast<int>(metric_sum)));
    println("onceCalls=" + std::to_string(once_calls) +
            " shutdownCalls=" + std::to_string(shutdown_calls));
    println("PASS: event_bus (" + std::to_string(passed) + " checks)");
}
