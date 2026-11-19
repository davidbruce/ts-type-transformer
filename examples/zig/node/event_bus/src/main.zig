const std = @import("std");
const b = @import("node");

extern "env" fn print(ptr: [*]const u8, len: u32) void;

var passed: u32 = 0;

fn check(ok: bool) void {
    if (!ok) @trap();
    passed += 1;
}

fn println(s: []const u8) void {
    print(s.ptr, @intCast(s.len));
    print("\n", 1);
}

var metric_calls: u32 = 0;
var metric_sum: f64 = 0;
var once_calls: u32 = 0;
var shutdown_calls: u32 = 0;

fn onMetric(_: ?*anyopaque, args: b.JsArray) void {
    metric_calls += 1;
    if (args.len() > 0) {
        metric_sum += args.get(0).toNumber();
    }
}

fn onOnce(_: ?*anyopaque, _: b.JsArray) void {
    once_calls += 1;
}

fn onShutdown(_: ?*anyopaque, _: b.JsArray) void {
    shutdown_calls += 1;
}

export fn start() void {
    const emitter = b.events.EventEmitter.new(null);
    _ = emitter.setMaxListeners(12);
    check(emitter.getMaxListeners() == 12);

    const ev_metric = b.JsValue.string("metric");
    const ev_shutdown = b.JsValue.string("shutdown");
    const ev_idle = b.JsValue.string("idle");

    const metric_cb = b.any_array_fn.from(onMetric, null);
    const once_cb = b.any_array_fn.from(onOnce, null);
    const shutdown_cb = b.any_array_fn.from(onShutdown, null);

    _ = emitter.on(ev_metric, metric_cb);
    _ = emitter.once(ev_metric, once_cb);
    _ = emitter.on(ev_shutdown, shutdown_cb);

    check(emitter.listenerCount(ev_metric, .{}) == 2);
    check(emitter.eventNames().len() == 2);

    const a1 = b.JsArray.new();
    a1.push(b.JsValue.number(101));
    a1.push(b.JsValue.string("ok"));
    const emitted1 = emitter.emit(ev_metric, a1);
    check(emitted1 and metric_calls == 1 and once_calls == 1);
    check(emitter.listenerCount(ev_metric, .{}) == 1);

    const a2 = b.JsArray.new();
    a2.push(b.JsValue.number(202));
    a2.push(b.JsValue.string("ok"));
    const emitted2 = emitter.emit(ev_metric, a2);
    check(emitted2 and metric_calls == 2 and metric_sum == 303 and once_calls == 1);

    check(!emitter.emit(ev_idle, b.JsArray.new()));

    _ = emitter.removeListener(ev_metric, metric_cb);
    const a3 = b.JsArray.new();
    a3.push(b.JsValue.number(1));
    check(emitter.listenerCount(ev_metric, .{}) == 0 and !emitter.emit(ev_metric, a3));
    check(metric_calls == 2);

    _ = emitter.removeAllListeners(ev_shutdown);
    check(emitter.eventNames().len() == 0 and shutdown_calls == 0);

    var line_buf: [256]u8 = undefined;
    println(std.fmt.bufPrint(&line_buf, "maxListeners={d:.0} metricCalls={d} metricSum={d:.0}", .{
        emitter.getMaxListeners(),
        metric_calls,
        metric_sum,
    }) catch "");
    println(std.fmt.bufPrint(&line_buf, "onceCalls={d} shutdownCalls={d}", .{
        once_calls,
        shutdown_calls,
    }) catch "");
    println(std.fmt.bufPrint(&line_buf, "PASS: event_bus ({d} checks)", .{passed}) catch "");
}
