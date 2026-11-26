const std = @import("std");
const b = @import("node");

extern "env" fn print(ptr: [*]const u8, len: u32) void;

const BusState = struct {
    passed: u32 = 0,
    metric_calls: u32 = 0,
    metric_sum: f64 = 0,
    once_calls: u32 = 0,
    shutdown_calls: u32 = 0,

    fn check(self: *BusState, ok: bool) void {
        if (!ok) @trap();
        self.passed += 1;
    }
};

fn println(s: []const u8) void {
    print(s.ptr, @intCast(s.len));
    print("\n", 1);
}

fn onMetric(state: *BusState, args: b.JsArray) void {
    state.metric_calls += 1;
    if (args.len() > 0) {
        state.metric_sum += args.get(0).toNumber();
    }
}

fn onOnce(state: *BusState, _: b.JsArray) void {
    state.once_calls += 1;
}

fn onShutdown(state: *BusState, _: b.JsArray) void {
    state.shutdown_calls += 1;
}

export fn start() void {
    const scope = b.JsScope.open();
    defer scope.deinit();

    var state: BusState = .{};
    const emitter = b.events.EventEmitter.new(null);
    _ = emitter.setMaxListeners(12);
    state.check(emitter.getMaxListeners() == 12);

    const metric_cb = b.any_array_fn.from(onMetric, &state);
    _ = emitter.on("metric", metric_cb);
    _ = emitter.once("metric", .{ onOnce, &state });
    _ = emitter.on("shutdown", .{ onShutdown, &state });

    state.check(emitter.listenerCount("metric") == 2);
    state.check(emitter.eventNames().len() == 2);

    const emitted1 = emitter.emitWith("metric", .{ 101, "ok" });
    state.check(emitted1 and state.metric_calls == 1 and state.once_calls == 1);
    state.check(emitter.listenerCount("metric") == 1);

    const emitted2 = emitter.emitWith("metric", .{ 202, "ok" });
    state.check(emitted2 and state.metric_calls == 2 and state.metric_sum == 303 and state.once_calls == 1);

    state.check(!emitter.emit("idle"));

    _ = emitter.removeListener("metric", metric_cb);
    state.check(emitter.listenerCount("metric") == 0 and !emitter.emitWith("metric", .{1}));
    state.check(state.metric_calls == 2);

    _ = emitter.removeAllListeners("shutdown");
    state.check(emitter.eventNames().len() == 0 and state.shutdown_calls == 0);

    var line_buf: [256]u8 = undefined;
    println(std.fmt.bufPrint(&line_buf, "maxListeners={d:.0} metricCalls={d} metricSum={d:.0}", .{
        emitter.getMaxListeners(),
        state.metric_calls,
        state.metric_sum,
    }) catch "");
    println(std.fmt.bufPrint(&line_buf, "onceCalls={d} shutdownCalls={d}", .{
        state.once_calls,
        state.shutdown_calls,
    }) catch "");
    println(std.fmt.bufPrint(&line_buf, "PASS: event_bus ({d} checks)", .{state.passed}) catch "");
}
