#include "node/node.h"
#include <stdarg.h>
#include <stdio.h>

TS_IMPORT(print) void host_print(const char* ptr, uint32_t len);

static int g_passed = 0;
static int g_metric_calls = 0;
static double g_metric_sum = 0.0;
static int g_once_calls = 0;
static int g_shutdown_calls = 0;

static void check(bool ok) {
    if (!ok) __builtin_trap();
    g_passed++;
}

static void emitf(const char* fmt, ...) {
    char buf[512];
    va_list ap;
    va_start(ap, fmt);
    int n = vsnprintf(buf, sizeof buf, fmt, ap);
    va_end(ap);
    if (n > 0) host_print(buf, (uint32_t)(n < (int)sizeof buf ? n : (int)sizeof buf - 1));
}

static void on_metric(void* ctx, JsArray args) {
    (void)ctx;
    js_scope();
    g_metric_calls++;
    if (JsArray_length(args) > 0) {
        g_metric_sum += js_to_number(JsArray_get(args, 0));
    }
}

static void on_once(void* ctx, JsArray args) {
    (void)ctx; (void)args;
    g_once_calls++;
}

static void on_shutdown(void* ctx, JsArray args) {
    (void)ctx; (void)args;
    g_shutdown_calls++;
}

__attribute__((export_name("start"))) void start(void) {
    js_scope();
    Node_EventEmitter emitter = Node_EventEmitter_new();

    Node_EventEmitter_setMaxListeners(emitter, 12.0);
    check(Node_EventEmitter_getMaxListeners(emitter) == 12.0);

    Node_Listener metric_cb = Node_Listener_from(on_metric, 0);

    Node_EventEmitter_on(emitter, "metric", metric_cb);
    Node_EventEmitter_once(emitter, "metric", on_once);
    Node_EventEmitter_on(emitter, "shutdown", on_shutdown);

    check(Node_EventEmitter_listenerCount(emitter, "metric") == 2.0);
    check(js_len(Node_EventEmitter_eventNames(emitter)) == 2);

    check(Node_EventEmitter_emit(emitter, "metric", 101.0, "ok") && g_metric_calls == 1 && g_once_calls == 1);
    check(Node_EventEmitter_listenerCount(emitter, "metric") == 1.0);

    check(Node_EventEmitter_emit(emitter, "metric", 202.0, "ok") && g_metric_calls == 2 && g_metric_sum == 303.0 && g_once_calls == 1);

    check(!Node_EventEmitter_emit(emitter, "idle"));

    Node_EventEmitter_removeListener(emitter, "metric", metric_cb);
    check(Node_EventEmitter_listenerCount(emitter, "metric") == 0.0 && !Node_EventEmitter_emit(emitter, "metric"));
    check(g_metric_calls == 2);

    Node_EventEmitter_removeAllListeners(emitter, "shutdown");
    check(js_len(Node_EventEmitter_eventNames(emitter)) == 0 && g_shutdown_calls == 0);

    emitf("maxListeners=%d metricCalls=%d metricSum=%d\nonceCalls=%d shutdownCalls=%d\nPASS: event_bus (%d checks)\n",
        (int)Node_EventEmitter_getMaxListeners(emitter),
        g_metric_calls, (int)g_metric_sum, g_once_calls, g_shutdown_calls, g_passed);
}
