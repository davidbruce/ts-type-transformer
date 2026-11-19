#include "node/node.h"

TS_IMPORT(print) void host_print(const char* ptr, uint32_t len);

static int g_passed = 0;
static int g_metric_calls = 0;
static double g_metric_sum = 0.0;
static int g_once_calls = 0;
static int g_shutdown_calls = 0;

static void check(int ok) {
    if (!ok) __builtin_trap();
    g_passed++;
}

static void emit(const char* s) {
    host_print(s, ts_strlen(s));
}

static void emit_int(int n) {
    char buf[24];
    int i = 0;
    if (n == 0) {
        buf[i++] = '0';
    } else {
        char rev[24];
        int r = 0;
        if (n < 0) {
            emit("-");
            n = -n;
        }
        while (n > 0) {
            rev[r++] = (char)('0' + (n % 10));
            n /= 10;
        }
        while (r > 0) buf[i++] = rev[--r];
    }
    host_print(buf, (uint32_t)i);
}

static void on_metric(void* ctx, JsArray args) {
    (void)ctx;
    g_metric_calls++;
    if (JsArray_length(args) > 0) {
        JsValue v = JsArray_get(args, 0);
        g_metric_sum += js_to_number(v);
        js_release(v);
    }
}

static void on_once(void* ctx, JsArray args) {
    (void)ctx;
    (void)args;
    g_once_calls++;
}

static void on_shutdown(void* ctx, JsArray args) {
    (void)ctx;
    (void)args;
    g_shutdown_calls++;
}

__attribute__((export_name("start"))) void start(void) {
    EventEmitter raw_emitter = EventEmitter_new((EventEmitterOptions){ 0 });
    NodeJS_EventEmitter emitter = EventEmitter_as_NodeJS_EventEmitter(raw_emitter);

    NodeJS_EventEmitter_setMaxListeners(emitter, 12.0);
    check(NodeJS_EventEmitter_getMaxListeners(emitter) == 12.0);

    any_array_fn metric_cb = any_array_fn_from(on_metric, 0);
    any_array_fn once_cb = any_array_fn_from(on_once, 0);
    any_array_fn shutdown_cb = any_array_fn_from(on_shutdown, 0);

    NodeJS_EventEmitter_on(emitter, "metric", metric_cb);
    NodeJS_EventEmitter_once(emitter, "metric", once_cb);
    NodeJS_EventEmitter_on(emitter, "shutdown", shutdown_cb);

    check(NodeJS_EventEmitter_listenerCount(emitter, "metric") == 2.0);
    JsArray names1 = NodeJS_EventEmitter_eventNames(emitter);
    check(JsArray_length(names1) == 2);
    js_release(names1);

    JsArray args1 = JsArray_new();
    JsValue n1 = js_number(101.0);
    JsValue s1 = js_string("ok");
    JsArray_push(args1, n1);
    JsArray_push(args1, s1);
    bool emitted1 = NodeJS_EventEmitter_emit(emitter, "metric", args1);
    js_release(s1); js_release(n1); js_release(args1);
    check(emitted1 && g_metric_calls == 1 && g_once_calls == 1);
    check(NodeJS_EventEmitter_listenerCount(emitter, "metric") == 1.0);

    JsArray args2 = JsArray_new();
    JsValue n2 = js_number(202.0);
    JsValue s2 = js_string("ok");
    JsArray_push(args2, n2);
    JsArray_push(args2, s2);
    bool emitted2 = NodeJS_EventEmitter_emit(emitter, "metric", args2);
    js_release(s2); js_release(n2); js_release(args2);
    check(emitted2 && g_metric_calls == 2 && g_metric_sum == 303.0 && g_once_calls == 1);

    JsArray empty_args = JsArray_new();
    check(!NodeJS_EventEmitter_emit(emitter, "idle", empty_args));

    NodeJS_EventEmitter_removeListener(emitter, "metric", metric_cb);
    check(NodeJS_EventEmitter_listenerCount(emitter, "metric") == 0.0 && !NodeJS_EventEmitter_emit(emitter, "metric", empty_args));
    check(g_metric_calls == 2);

    NodeJS_EventEmitter_removeAllListeners(emitter, "shutdown");
    JsArray names2 = NodeJS_EventEmitter_eventNames(emitter);
    check(JsArray_length(names2) == 0 && g_shutdown_calls == 0);
    js_release(names2);
    js_release(empty_args);

    emit("maxListeners="); emit_int((int)NodeJS_EventEmitter_getMaxListeners(emitter));
    emit(" metricCalls="); emit_int(g_metric_calls);
    emit(" metricSum="); emit_int((int)g_metric_sum); emit("\n");
    emit("onceCalls="); emit_int(g_once_calls); emit(" shutdownCalls="); emit_int(g_shutdown_calls); emit("\n");
    emit("PASS: event_bus ("); emit_int(g_passed); emit(" checks)\n");

    js_release(shutdown_cb); js_release(once_cb); js_release(metric_cb);
    js_release(raw_emitter);
}
