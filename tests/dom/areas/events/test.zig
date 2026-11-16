// Events from Zig: listeners and their options, events made and dispatched,
// propagation, cancelation, aborting. Every expect must hold.
const std = @import("std");
const b = @import("bindings.zig");

extern "env" fn check(ok: i32, line: u32) void;

fn expect(ok: bool, src: std.builtin.SourceLocation) void {
    check(@intFromBool(ok), src.line);
}

// What listeners write to: a count and the order things ran in.
var count: u32 = 0;
var order: [16]u8 = undefined;
var order_len: usize = 0;
var seen_type: [32]u8 = undefined;
var seen_type_len: usize = 0;
var seen_phase: f64 = -1;
var seen_target_is_child = false;
var seen_current_is_parent = false;

fn mark(c: u8) void {
    order[order_len] = c;
    order_len += 1;
}
fn order_is(s: []const u8) bool {
    return std.mem.eql(u8, order[0..order_len], s);
}

// A listener's context carries the letter it records.
fn onMark(ctx: ?*anyopaque, _: b.Event) void {
    const ch: *const u8 = @ptrCast(@alignCast(ctx));
    mark(ch.*);
}
fn onCount(_: ?*anyopaque, _: b.Event) void {
    count += 1;
}
fn onClick(_: ?*anyopaque, _: b.PointerEvent) void {
    count += 1;
}
fn onStop(ctx: ?*anyopaque, ev: b.Event) void {
    onMark(ctx, ev);
    ev.stopPropagation();
}
fn onStopImmediate(ctx: ?*anyopaque, ev: b.Event) void {
    onMark(ctx, ev);
    ev.stopImmediatePropagation();
}
fn onPrevent(_: ?*anyopaque, ev: b.Event) void {
    ev.preventDefault();
}
fn onInspect(ctx: ?*anyopaque, ev: b.Event) void {
    const parent: *const b.HTMLElement = @ptrCast(@alignCast(ctx));
    seen_type_len = ev.getType().copy(&seen_type).len;
    seen_phase = ev.getEventPhase();
    if (ev.getCurrentTarget()) |t| seen_current_is_parent = b.jsSame(t, parent.*);
    if (ev.getTarget()) |t| seen_target_is_child = !b.jsSame(t, parent.*);
}

var detail_number: f64 = 0;
var detail_text: [16]u8 = undefined;
var detail_len: usize = 0;
// FINDING 2 (see cssom): a listener added by event name as a string gets a plain Event, and
// reaching CustomEvent/MouseEvent means rebuilding the handle by hand.
fn onDetail(_: ?*anyopaque, ev: b.Event) void {
    const custom = b.CustomEvent{ .h = ev.h };
    const d = custom.getDetail();
    detail_number = d.toNumber();
    detail_len = d.toString().copy(&detail_text).len;
}

var mouse_x: f64 = 0;
var mouse_y: f64 = 0;
var mouse_button: f64 = 0;
var mouse_shift = false;
fn onMouse(_: ?*anyopaque, ev: b.Event) void {
    const m = b.MouseEvent{ .h = ev.h };
    mouse_x = m.getClientX();
    mouse_y = m.getClientY();
    mouse_button = m.getButton();
    mouse_shift = m.getShiftKey();
}

var key_buf: [16]u8 = undefined;
var key_len: usize = 0;
var key_ctrl = false;
var key_repeat = false;
fn onKey(_: ?*anyopaque, ev: b.KeyboardEvent) void {
    key_len = ev.getKey().copy(&key_buf).len;
    key_ctrl = ev.getCtrlKey();
    key_repeat = ev.getRepeat();
}

var letters = "abcdefg".*;
fn letter(i: usize) *u8 {
    return &letters[i];
}

fn reset() void {
    count = 0;
    order_len = 0;
}

fn init(bubbles: bool, cancelable: bool) b.EventInit {
    const i = b.EventInit.create();
    i.setBubbles(bubbles);
    i.setCancelable(cancelable);
    return i;
}

var parent_el: b.HTMLElement = undefined;

export fn run() void {
    const doc = b.getDocument();
    const body = doc.getBody();
    const parent = doc.createElement(.div, .{}).asHTMLElement();
    parent_el = parent;
    const child = doc.createElement(.button, .{}).asHTMLElement();
    _ = parent.appendChild(child.asNode());
    _ = body.appendChild(parent.asNode());

    // A listener added, called by real clicks, and removed.
    const click = b.JsListener(b.PointerEvent).from(onClick, null);
    child.addEventListener(.click, click, .{});
    child.click();
    child.click();
    expect(count == 2, @src());
    child.removeEventListener(.click, click, .{});
    child.click();
    expect(count == 2, @src());

    // Events made here carry the type and flags they were given.
    reset();
    const plain = b.Event.new("ping", .{});
    expect(plain.getType().eql("ping"), @src());
    expect(!plain.getBubbles() and !plain.getCancelable(), @src());
    expect(!plain.getIsTrusted(), @src());
    expect(plain.getTarget() == null, @src());
    const ping = b.EventListener.from(onCount, null);
    child.addEventListenerWithString("ping", .{ .EventListener = ping }, .{});
    expect(child.dispatchEvent(plain), @src());
    expect(count == 1, @src());
    expect(b.jsSame(plain.getTarget().?, child), @src());

    // once: the second dispatch finds no listener.
    reset();
    const once = b.AddEventListenerOptions.create();
    once.setOnce(true);
    child.addEventListenerWithString("once", .{ .EventListener = b.EventListener.from(onCount, null) }, .{ .options = .{ .AddEventListenerOptions = once } });
    _ = child.dispatchEvent(b.Event.new("once", .{}));
    _ = child.dispatchEvent(b.Event.new("once", .{}));
    expect(count == 1, @src());

    // The same listener added twice runs once.
    reset();
    const same = b.EventListener.from(onCount, null);
    child.addEventListenerWithString("same", .{ .EventListener = same }, .{});
    child.addEventListenerWithString("same", .{ .EventListener = same }, .{});
    _ = child.dispatchEvent(b.Event.new("same", .{}));
    expect(count == 1, @src());
    child.removeEventListenerWithString("same", .{ .EventListener = same }, .{});
    _ = child.dispatchEvent(b.Event.new("same", .{}));
    expect(count == 1, @src());

    // An event bubbles only if it was made to; capture runs first.
    reset();
    const capture = b.AddEventListenerOptions.create();
    capture.setCapture(true);
    parent.addEventListenerWithString("walk", .{ .EventListener = b.EventListener.from(onMark, letter(0)) }, .{ .options = .{ .AddEventListenerOptions = capture } });
    parent.addEventListenerWithString("walk", .{ .EventListener = b.EventListener.from(onMark, letter(1)) }, .{});
    child.addEventListenerWithString("walk", .{ .EventListener = b.EventListener.from(onMark, letter(2)) }, .{});
    _ = child.dispatchEvent(b.Event.new("walk", .{ .eventInitDict = init(true, false) }));
    expect(order_is("acb"), @src());
    order_len = 0;
    _ = child.dispatchEvent(b.Event.new("walk", .{ .eventInitDict = init(false, false) }));
    expect(order_is("ac"), @src());

    // stopPropagation keeps an event from the next element; stopImmediatePropagation from the next listener.
    reset();
    child.addEventListenerWithString("halt", .{ .EventListener = b.EventListener.from(onStop, letter(0)) }, .{});
    parent.addEventListenerWithString("halt", .{ .EventListener = b.EventListener.from(onMark, letter(1)) }, .{});
    _ = child.dispatchEvent(b.Event.new("halt", .{ .eventInitDict = init(true, false) }));
    expect(order_is("a"), @src());
    reset();
    child.addEventListenerWithString("now", .{ .EventListener = b.EventListener.from(onStopImmediate, letter(2)) }, .{});
    child.addEventListenerWithString("now", .{ .EventListener = b.EventListener.from(onMark, letter(3)) }, .{});
    _ = child.dispatchEvent(b.Event.new("now", .{}));
    expect(order_is("c"), @src());

    // preventDefault on a cancelable event is reported by dispatchEvent.
    child.addEventListenerWithString("veto", .{ .EventListener = b.EventListener.from(onPrevent, null) }, .{});
    const cancelable = b.Event.new("veto", .{ .eventInitDict = init(false, true) });
    expect(!cancelable.getDefaultPrevented(), @src());
    expect(!child.dispatchEvent(cancelable), @src());
    expect(cancelable.getDefaultPrevented(), @src());
    const fixed = b.Event.new("veto", .{});
    expect(child.dispatchEvent(fixed), @src());
    expect(!fixed.getDefaultPrevented(), @src());

    // A passive listener cannot cancel.
    const passive = b.AddEventListenerOptions.create();
    passive.setPassive(true);
    expect(passive.getPassive().? and passive.getOnce() == null, @src());
    child.addEventListenerWithString("soft", .{ .EventListener = b.EventListener.from(onPrevent, null) }, .{ .options = .{ .AddEventListenerOptions = passive } });
    const soft = b.Event.new("soft", .{ .eventInitDict = init(false, true) });
    expect(child.dispatchEvent(soft), @src());
    expect(!soft.getDefaultPrevented(), @src());

    // What a listener can see of the event.
    parent.addEventListenerWithString("look", .{ .EventListener = b.EventListener.from(onInspect, &parent_el) }, .{});
    _ = child.dispatchEvent(b.Event.new("look", .{ .eventInitDict = init(true, false) }));
    expect(std.mem.eql(u8, seen_type[0..seen_type_len], "look"), @src());
    expect(seen_phase == b.Event.BUBBLING_PHASE, @src());
    expect(seen_target_is_child and seen_current_is_parent, @src());
    const path = b.Event.new("path", .{ .eventInitDict = init(true, false) });
    expect(path.composedPath().len() == 0, @src());

    // An AbortController's signal removes a listener.
    reset();
    const controller = b.AbortController.new();
    const signal = controller.getSignal();
    const bound = b.AddEventListenerOptions.create();
    bound.setSignal(signal);
    child.addEventListenerWithString("bound", .{ .EventListener = b.EventListener.from(onCount, null) }, .{ .options = .{ .AddEventListenerOptions = bound } });
    _ = child.dispatchEvent(b.Event.new("bound", .{}));
    expect(count == 1 and !signal.getAborted(), @src());
    controller.abort(null);
    expect(signal.getAborted(), @src());
    _ = child.dispatchEvent(b.Event.new("bound", .{}));
    expect(count == 1, @src());
    expect(signal.getReason().toString().len() > 0, @src());

    // A CustomEvent carries a detail of any value.
    const custom_init = b.CustomEventInit.create();
    custom_init.setDetail(b.JsValue.number(42));
    custom_init.setBubbles(true);
    child.addEventListenerWithString("custom", .{ .EventListener = b.EventListener.from(onDetail, null) }, .{});
    const custom = b.CustomEvent.new("custom", .{ .eventInitDict = custom_init });
    expect(custom.getBubbles() and custom.getDetail().toNumber() == 42, @src());
    _ = child.dispatchEvent(custom.asEvent());
    expect(detail_number == 42, @src());
    const text_init = b.CustomEventInit.create();
    text_init.setDetail(b.JsValue.string("payload"));
    _ = child.dispatchEvent(b.CustomEvent.new("custom", .{ .eventInitDict = text_init }).asEvent());
    expect(std.mem.eql(u8, detail_text[0..detail_len], "payload"), @src());
    expect(b.CustomEvent.new("bare", .{}).getDetail().isNull(), @src());

    // A MouseEvent made with an init dictionary.
    const mi = b.MouseEventInit.create();
    mi.setClientX(12);
    mi.setClientY(34);
    mi.setButton(2);
    mi.setShiftKey(true);
    mi.setBubbles(true);
    child.addEventListenerWithString("mouse", .{ .EventListener = b.EventListener.from(onMouse, null) }, .{});
    const mouse = b.MouseEvent.new("mouse", .{ .eventInitDict = mi });
    expect(mouse.getClientX() == 12 and mouse.getButton() == 2, @src());
    _ = child.dispatchEvent(mouse.asEvent());
    expect(mouse_x == 12 and mouse_y == 34 and mouse_button == 2 and mouse_shift, @src());

    // A KeyboardEvent, heard through the typed key listener.
    const ki = b.KeyboardEventInit.create();
    ki.setKey("Enter");
    ki.setCtrlKey(true);
    ki.setRepeat(true);
    ki.setBubbles(true);
    child.addEventListener(.keydown, b.JsListener(b.KeyboardEvent).from(onKey, null), .{});
    expect(child.dispatchEvent(b.KeyboardEvent.new("keydown", .{ .eventInitDict = ki }).asEvent()), @src());
    expect(std.mem.eql(u8, key_buf[0..key_len], "Enter") and key_ctrl and key_repeat, @src());

    // Events dispatched on the document and the window too.
    reset();
    doc.addEventListenerWithString("doc-event", .{ .EventListener = b.EventListener.from(onCount, null) }, .{});
    _ = doc.dispatchEvent(b.Event.new("doc-event", .{}));
    expect(count == 1, @src());
    const win = b.getWindow();
    win.addEventListenerWithString("win-event", .{ .EventListener = b.EventListener.from(onCount, null) }, .{});
    _ = win.dispatchEvent(b.Event.new("win-event", .{}));
    expect(count == 2, @src());
}
