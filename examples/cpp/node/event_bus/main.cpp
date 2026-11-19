#include <cstdint>
#include <string>
#include <string_view>

#include "node/node.hpp"

using namespace node;

extern "C" __attribute__((import_module("env"), import_name("print")))
void host_print(const char* ptr, uint32_t len);

static int g_passed = 0;

static void check(bool ok) {
    if (!ok) __builtin_trap();
    g_passed++;
}

static void println(std::string_view s) {
    host_print(s.data(), static_cast<uint32_t>(s.size()));
    host_print("\n", 1);
}

extern "C" __attribute__((export_name("start"))) void start() {
    EventEmitter<ts::Value> emitter = EventEmitter<ts::Value>::create();
    emitter.setMaxListeners(12);
    check(emitter.getMaxListeners() == 12);

    int metric_calls = 0;
    double metric_sum = 0.0;
    int once_calls = 0;
    int shutdown_calls = 0;

    ts::Value ev_metric = ts::Value::string("metric");
    ts::Value ev_shutdown = ts::Value::string("shutdown");
    ts::Value ev_idle = ts::Value::string("idle");

    any_array_fn on_metric = [&](ts::Array<> args) {
        metric_calls++;
        if (args.size() > 0) {
            metric_sum += ts::Value(args[0]).to_number();
        }
    };
    any_array_fn on_once = [&](ts::Array<>) { once_calls++; };
    any_array_fn on_shutdown = [&](ts::Array<>) { shutdown_calls++; };

    emitter.on<void>(ev_metric, on_metric);
    emitter.once<void>(ev_metric, on_once);
    emitter.on<void>(ev_shutdown, on_shutdown);

    check(emitter.listenerCount<void>(ev_metric) == 2);
    check(emitter.eventNames().size() == 2);

    bool emitted1 = emitter.emit<void>(ev_metric, ts::Array<>{ts::Value::number(101), ts::Value::string("ok")});
    check(emitted1 && metric_calls == 1 && once_calls == 1);
    check(emitter.listenerCount<void>(ev_metric) == 1);

    bool emitted2 = emitter.emit<void>(ev_metric, ts::Array<>{ts::Value::number(202), ts::Value::string("ok")});
    check(emitted2 && metric_calls == 2 && metric_sum == 303.0 && once_calls == 1);

    check(!emitter.emit<void>(ev_idle, ts::Array<>{}));

    emitter.removeListener<void>(ev_metric, on_metric);
    check(emitter.listenerCount<void>(ev_metric) == 0 &&
          !emitter.emit<void>(ev_metric, ts::Array<>{ts::Value::number(1)}));
    check(metric_calls == 2);

    emitter.removeAllListeners<void>(ev_shutdown);
    check(emitter.eventNames().size() == 0 && shutdown_calls == 0);

    on_metric.release();
    on_once.release();
    on_shutdown.release();

    println("maxListeners=" + std::to_string(static_cast<int>(emitter.getMaxListeners())) +
            " metricCalls=" + std::to_string(metric_calls) +
            " metricSum=" + std::to_string(static_cast<int>(metric_sum)));
    println("onceCalls=" + std::to_string(once_calls) +
            " shutdownCalls=" + std::to_string(shutdown_calls));
    println("PASS: event_bus (" + std::to_string(g_passed) + " checks)");
}
