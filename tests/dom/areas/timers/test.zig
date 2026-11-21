// Timers from Zig: timeouts, intervals, microtasks, animation frames and the
// clock. Callbacks run after `run` returns: `busy` is nonzero while any is
// still due, and `finish` checks what ran. Every expect must hold.
const std = @import("std");
const b = @import("bindings.zig");

extern "env" fn check(ok: i32, line: u32) void;

fn expect(ok: bool, src: std.builtin.SourceLocation) void {
    check(@intFromBool(ok), src.line);
}

// Callbacks still due; `busy` reports it.
var pending: u32 = 0;
export fn busy() u32 {
    return pending;
}

// The order things ran in, one letter each.
var order: [32]u8 = undefined;
var order_len: usize = 0;
fn mark(c: u8) void {
    order[order_len] = c;
    order_len += 1;
}

var perf: b.Performance = undefined;

fn handler(comptime f: b.Function.Fn) b.TimerHandler {
    return .{ .Function = b.Function.from(f, null) };
}

fn after(ms: f64, comptime f: b.Function.Fn) f64 {
    pending += 1;
    return b.setTimeout(handler(f), .{ .timeout = ms });
}

// Timeouts.
var timeout_fired: u32 = 0;
var cancelled_fired: u32 = 0;
var started: f64 = 0;
var elapsed: f64 = 0;
fn onTimeout(_: ?*anyopaque, _: b.JsArray) b.JsValue {
    timeout_fired += 1;
    elapsed = perf.now() - started;
    mark('t');
    // A timeout set from a timeout runs after it.
    _ = after(0, onChained);
    pending -= 1;
    return .{ .h = 0 };
}
fn onChained(_: ?*anyopaque, _: b.JsArray) b.JsValue {
    mark('c');
    pending -= 1;
    return .{ .h = 0 };
}
fn onNoDelay(_: ?*anyopaque, _: b.JsArray) b.JsValue {
    mark('u');
    pending -= 1;
    return .{ .h = 0 };
}
fn onCancelled(_: ?*anyopaque, _: b.JsArray) b.JsValue {
    cancelled_fired += 1;
    pending -= 1;
    return .{ .h = 0 };
}
fn onZero(_: ?*anyopaque, _: b.JsArray) b.JsValue {
    mark('z');
    pending -= 1;
    return .{ .h = 0 };
}
fn onLate(_: ?*anyopaque, _: b.JsArray) b.JsValue {
    mark('l');
    pending -= 1;
    return .{ .h = 0 };
}

// An interval that stops itself after three ticks.
var interval_id: f64 = 0;
var ticks: u32 = 0;
var tick_times: [3]f64 = undefined;
fn onTick(_: ?*anyopaque, _: b.JsArray) b.JsValue {
    tick_times[ticks] = perf.now();
    ticks += 1;
    if (ticks == 3) {
        b.clearInterval(interval_id);
        pending -= 1;
    }
    return .{ .h = 0 };
}
// An interval cleared before it ever ticks.
var dead_ticks: u32 = 0;
fn onDeadTick(_: ?*anyopaque, _: b.JsArray) b.JsValue {
    dead_ticks += 1;
    return .{ .h = 0 };
}

// Microtasks.
fn onMicro(_: ?*anyopaque) void {
    mark('m');
    b.queueMicrotask(b.VoidFunction.from(onNestedMicro, null));
}
fn onNestedMicro(_: ?*anyopaque) void {
    mark('n');
    pending -= 1;
}
fn onMicroFirst(_: ?*anyopaque) void {
    mark('1');
    pending -= 1;
}

// Animation frames.
var frames: u32 = 0;
var frame_stamps: [2]f64 = .{ 0, 0 };
var cancelled_frame_fired = false;
fn onFrame(_: ?*anyopaque, stamp: f64) void {
    frame_stamps[frames] = stamp;
    frames += 1;
    if (frames == 1) {
        _ = b.requestAnimationFrame(b.FrameRequestCallback.from(onFrame, null));
    } else {
        pending -= 1;
    }
}
fn onCancelledFrame(_: ?*anyopaque, _: f64) void {
    cancelled_frame_fired = true;
}

export fn run() void {
    perf = b.getPerformance();

    // The clock moves forward, never back.
    const t0 = perf.now();
    var spin: u32 = 0;
    var last = t0;
    var monotonic = true;
    while (spin < 2000) : (spin += 1) {
        const t = perf.now();
        if (t < last) monotonic = false;
        last = t;
    }
    expect(monotonic and last >= t0, @src());
    expect(t0 >= 0 and perf.getTimeOrigin() > 1.0e12, @src());
    expect(@abs(perf.getTimeOrigin() + perf.now() - b.jsDateNow()) < 1000, @src());

    // User timing marks: named, timed, and measurable.
    const mark_a = perf.mark("zig-a", .{});
    const mark_b = perf.mark("zig-b", .{});
    expect(mark_a.asPerformanceEntry().getName().eql("zig-a") and mark_a.asPerformanceEntry().getEntryType().eql("mark"), @src());
    expect(mark_b.asPerformanceEntry().getStartTime() >= mark_a.asPerformanceEntry().getStartTime(), @src());
    expect(perf.getEntriesByName("zig-a", .{}).len() == 1, @src());
    expect(perf.getEntriesByType("mark").len() >= 2, @src());
    const span = perf.measure("zig-span", .{ .startOrMeasureOptions = .{ .string = b.JsString.from("zig-a") }, .endMark = "zig-b" });
    expect(span.asPerformanceEntry().getDuration() >= 0 and span.asPerformanceEntry().getEntryType().eql("measure"), @src());
    perf.clearMarks("zig-a");
    expect(perf.getEntriesByName("zig-a", .{}).len() == 0, @src());

    // Order: this code, then microtasks (and what they queue), then timeouts.
    started = perf.now();
    pending += 2;
    mark('s');
    b.queueMicrotask(b.VoidFunction.from(onMicro, null));
    b.queueMicrotask(b.VoidFunction.from(onMicroFirst, null));
    mark('S');
    _ = after(0, onZero);
    // No delay given is the same as none.
    pending += 1;
    _ = b.setTimeout(handler(onNoDelay), .{});
    // A later timeout is not run before an earlier one.
    _ = after(30, onLate);
    // A timeout of 40 ms runs once, no sooner than that.
    const id = after(40, onTimeout);
    expect(id > 0, @src());
    // Handles are distinct, and clearing one stops it.
    const doomed = after(10, onCancelled);
    expect(doomed != id and doomed > 0, @src());
    b.clearTimeout(doomed);
    pending -= 1;
    // Clearing what is not a timer, or twice, does nothing.
    b.clearTimeout(doomed);
    b.clearTimeout(null);
    b.clearTimeout(123456789);

    // An interval ticks until cleared.
    pending += 1;
    interval_id = b.setInterval(handler(onTick), .{ .timeout = 10 });
    expect(interval_id > 0 and interval_id != id, @src());
    // One cleared at once never ticks.
    const dead = b.setInterval(handler(onDeadTick), .{ .timeout = 5 });
    b.clearInterval(dead);

    // Frames: two in a row; a cancelled one never comes.
    pending += 1;
    _ = b.requestAnimationFrame(b.FrameRequestCallback.from(onFrame, null));
    const frame = b.requestAnimationFrame(b.FrameRequestCallback.from(onCancelledFrame, null));
    expect(frame > 0, @src());
    b.cancelAnimationFrame(frame);

    // Nothing has run yet: callbacks wait for this code to return.
    expect(order_len == 2 and timeout_fired == 0 and ticks == 0 and frames == 0, @src());
}

export fn finish() void {
    // s S, then both microtasks in order 1 and the one queued from m after them (m 1 n),
    // then the timeouts by due time: z and u (0 ms), l (30 ms), t (40 ms), c (set by t).
    expect(std.mem.eql(u8, order[0..order_len], "sSm1nzultc"), @src());
    expect(timeout_fired == 1 and cancelled_fired == 0, @src());
    // Timers may fire a tick early by rounding but not by much.
    expect(elapsed >= 38, @src());
    expect(ticks == 3 and dead_ticks == 0, @src());
    expect(tick_times[0] < tick_times[1] and tick_times[1] < tick_times[2], @src());
    expect(tick_times[2] - tick_times[0] >= 15, @src());
    expect(frames == 2 and !cancelled_frame_fired, @src());
    expect(frame_stamps[0] > 0 and frame_stamps[1] > frame_stamps[0], @src());
    expect(pending == 0, @src());
}
