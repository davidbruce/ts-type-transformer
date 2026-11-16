// Observers from Zig: MutationObserver (records taken at once and delivered
// later), ResizeObserver and IntersectionObserver (called once the page lays
// out) and AbortSignal.timeout. `busy` is nonzero until the async ones have
// reported, and `finish` checks. Every expect must hold.
const std = @import("std");
const b = @import("bindings.zig");

extern "env" fn check(ok: i32, line: u32) void;

fn expect(ok: bool, src: std.builtin.SourceLocation) void {
    check(@intFromBool(ok), src.line);
}

// What is still to arrive.
var delivered_calls: u32 = 0;
var delivered_records: u32 = 0;
var delivered_added: u32 = 0;
var filtered_calls: u32 = 0;
var filtered_name_ok = true;
var disconnected_calls: u32 = 0;
var resize_calls: u32 = 0;
var resize_widths: [4]f64 = .{ 0, 0, 0, 0 };
var resize_heights: [4]f64 = .{ 0, 0, 0, 0 };
var resize_target_ok = true;
var resize_inline_size: f64 = 0;
var near_seen = false;
var near_intersecting = false;
var near_ratio: f64 = 0;
var far_seen = false;
var far_intersecting = true;
var timeout_events: u32 = 0;
var timeout_signal: b.AbortSignal = undefined;
var any_events: u32 = 0;

export fn busy() u32 {
    const waiting = resize_calls < 2 or !near_seen or !far_seen or timeout_events == 0 or delivered_calls == 0;
    return @intFromBool(waiting);
}

var near: b.HTMLElement = undefined;
var far: b.HTMLElement = undefined;
var box: b.HTMLElement = undefined;

// A MutationObserver's callback gets every record queued since the last one.
fn onMutations(_: ?*anyopaque, records: b.JsArrayOf(b.MutationRecord), _: b.MutationObserver) void {
    delivered_calls += 1;
    var i: usize = 0;
    while (i < records.len()) : (i += 1) {
        const r = records.get(i);
        delivered_records += 1;
        delivered_added += @intFromFloat(r.getAddedNodes().getLength());
    }
}

fn onFiltered(_: ?*anyopaque, records: b.JsArrayOf(b.MutationRecord), _: b.MutationObserver) void {
    filtered_calls += 1;
    var i: usize = 0;
    while (i < records.len()) : (i += 1) {
        if (!records.get(i).getAttributeName().?.eql("watched")) filtered_name_ok = false;
    }
}

fn onNever(_: ?*anyopaque, _: b.JsArrayOf(b.MutationRecord), _: b.MutationObserver) void {
    disconnected_calls += 1;
}

fn onResize(_: ?*anyopaque, entries: b.JsArrayOf(b.ResizeObserverEntry), _: b.ResizeObserver) void {
    var i: usize = 0;
    while (i < entries.len()) : (i += 1) {
        const e = entries.get(i);
        if (resize_calls < 4) {
            resize_widths[resize_calls] = e.getContentRect().getWidth();
            resize_heights[resize_calls] = e.getContentRect().getHeight();
        }
        if (!b.jsSame(e.getTarget(), box)) resize_target_ok = false;
        resize_inline_size = e.getContentBoxSize().get(0).getInlineSize();
        resize_calls += 1;
    }
    // The first report is answered with a resize, which is reported in turn.
    if (resize_calls == 1) box.getStyle().setProperty("width", "150px", .{});
}

fn onIntersect(_: ?*anyopaque, entries: b.JsArrayOf(b.IntersectionObserverEntry), _: b.IntersectionObserver) void {
    var i: usize = 0;
    while (i < entries.len()) : (i += 1) {
        const e = entries.get(i);
        if (b.jsSame(e.getTarget(), near)) {
            near_seen = true;
            near_intersecting = e.getIsIntersecting();
            near_ratio = e.getIntersectionRatio();
        } else if (b.jsSame(e.getTarget(), far)) {
            far_seen = true;
            far_intersecting = e.getIsIntersecting();
        }
    }
}

fn onAbort(_: ?*anyopaque, _: b.Event) void {
    timeout_events += 1;
}
fn onAnyAbort(_: ?*anyopaque, _: b.Event) void {
    any_events += 1;
}

fn recordsHave(records: b.JsArrayOf(b.MutationRecord), i: usize, kind: b.MutationRecordType) bool {
    return records.get(i).getType() == kind;
}

export fn run() void {
    const doc = b.getDocument();
    const body = doc.getBody();

    // A MutationObserver watching a subtree, for everything.
    const target = doc.createElement(.div, .{}).asHTMLElement();
    _ = body.appendChild(target.asNode());
    const watch = b.MutationObserverInit.create();
    watch.setChildList(true);
    watch.setAttributes(true);
    watch.setCharacterData(true);
    watch.setSubtree(true);
    watch.setAttributeOldValue(true);
    watch.setCharacterDataOldValue(true);
    const taker = b.MutationObserver.new(b.MutationCallback.from(onMutations, null));
    taker.observe(target.asNode(), .{ .options = watch });
    expect(taker.takeRecords().len() == 0, @src());

    const child = doc.createElement(.span, .{}).asHTMLElement();
    _ = target.appendChild(child.asNode());
    child.setAttribute("data-x", "1");
    child.setAttribute("data-x", "2");
    const text = doc.createTextNode("before");
    _ = child.appendChild(text.asNode());
    text.asNode().setNodeValue("after");
    child.remove();

    // takeRecords hands over what is queued, now, and leaves the callback nothing.
    const records = taker.takeRecords();
    expect(records.len() == 6, @src());
    expect(recordsHave(records, 0, .childList), @src());
    expect(records.get(0).getAddedNodes().getLength() == 1 and records.get(0).getRemovedNodes().getLength() == 0, @src());
    expect(b.jsSame(records.get(0).getTarget(), target) and b.jsSame(records.get(0).getAddedNodes().item(0).?, child), @src());
    expect(recordsHave(records, 1, .attributes) and records.get(1).getAttributeName().?.eql("data-x"), @src());
    expect(records.get(1).getOldValue() == null, @src());
    expect(records.get(2).getOldValue().?.eql("1"), @src());
    expect(recordsHave(records, 3, .childList) and b.jsSame(records.get(3).getTarget(), child), @src());
    expect(recordsHave(records, 4, .characterData) and records.get(4).getOldValue().?.eql("before"), @src());
    expect(recordsHave(records, 5, .childList) and records.get(5).getRemovedNodes().getLength() == 1, @src());
    expect(records.get(5).getPreviousSibling() == null and records.get(5).getType() != .attributes, @src());
    expect(taker.takeRecords().len() == 0, @src());

    // An observer whose callback is called: all the records of a turn arrive together.
    const seen = doc.createElement(.div, .{}).asHTMLElement();
    _ = body.appendChild(seen.asNode());
    const children = b.MutationObserverInit.create();
    children.setChildList(true);
    const heard = b.MutationObserver.new(b.MutationCallback.from(onMutations, null));
    heard.observe(seen.asNode(), .{ .options = children });
    _ = seen.appendChild(doc.createElement(.i, .{}).asNode());
    _ = seen.appendChild(doc.createElement(.i, .{}).asNode());
    seen.setAttribute("ignored", "attributes were not asked for");
    expect(delivered_calls == 0, @src());

    // An attribute filter names the attributes that count.
    const filter = doc.createElement(.div, .{}).asHTMLElement();
    _ = body.appendChild(filter.asNode());
    const names = b.JsArrayOf(b.JsString).new();
    names.pushString("watched");
    const only = b.MutationObserverInit.create();
    only.setAttributes(true);
    only.setAttributeFilter(names);
    const filtered = b.MutationObserver.new(b.MutationCallback.from(onFiltered, null));
    filtered.observe(filter.asNode(), .{ .options = only });
    filter.setAttribute("other", "1");
    filter.setAttribute("watched", "1");
    expect(filtered.takeRecords().len() == 1, @src());
    filter.setAttribute("watched", "2");

    // A disconnected observer sees nothing, and has nothing queued.
    const gone = doc.createElement(.div, .{}).asHTMLElement();
    _ = body.appendChild(gone.asNode());
    const forgot = b.MutationObserver.new(b.MutationCallback.from(onNever, null));
    forgot.observe(gone.asNode(), .{ .options = children });
    forgot.disconnect();
    _ = gone.appendChild(doc.createElement(.b, .{}).asNode());
    expect(forgot.takeRecords().len() == 0, @src());

    // A ResizeObserver reports the size on the first layout, and each change.
    box = doc.createElement(.div, .{}).asHTMLElement();
    box.getStyle().setProperty("width", "100px", .{});
    box.getStyle().setProperty("height", "50px", .{});
    _ = body.appendChild(box.asNode());
    const resizes = b.ResizeObserver.new(b.ResizeObserverCallback.from(onResize, null));
    resizes.observe(box.asElement(), .{});

    // An IntersectionObserver reports in-view and out-of-view targets, once each.
    near = doc.createElement(.div, .{}).asHTMLElement();
    near.getStyle().setProperty("height", "40px", .{});
    near.getStyle().setProperty("background", "red", .{});
    _ = body.appendChild(near.asNode());
    far = doc.createElement(.div, .{}).asHTMLElement();
    far.getStyle().setProperty("position", "absolute", .{});
    far.getStyle().setProperty("top", "20000px", .{});
    far.getStyle().setProperty("height", "40px", .{});
    far.getStyle().setProperty("width", "40px", .{});
    _ = body.appendChild(far.asNode());
    const intersections = b.IntersectionObserver.new(b.IntersectionObserverCallback.from(onIntersect, null), .{});
    intersections.observe(near.asElement());
    intersections.observe(far.asElement());
    expect(intersections.getRootMargin().eql("0px 0px 0px 0px"), @src());
    expect(intersections.getRoot() == null, @src());

    // AbortSignal.timeout aborts on its own, with a TimeoutError.
    timeout_signal = b.AbortSignal.timeout(30);
    expect(!timeout_signal.getAborted(), @src());
    timeout_signal.addEventListener(.abort, b.JsListener(b.Event).from(onAbort, null), .{});

    // A signal that is already aborted says so; any() follows the first of several.
    const early = b.AbortSignal.abort(null);
    expect(early.getAborted(), @src());
    expect(early.getReason().toString().len() > 0, @src());
    early.throwIfAborted();
    expect(b.jsTakeError() != null, @src());
    const one = b.AbortController.new();
    const two = b.AbortController.new();
    const signals = b.JsArrayOf(b.AbortSignal).new();
    signals.push(one.getSignal());
    signals.push(two.getSignal());
    const either = b.AbortSignal.any(signals);
    either.addEventListener(.abort, b.JsListener(b.Event).from(onAnyAbort, null), .{});
    expect(!either.getAborted(), @src());
    two.abort(b.JsValue.string("second"));
    expect(either.getAborted() and either.getReason().toString().eql("second"), @src());
    one.abort(null);
    expect(any_events == 1 and one.getSignal().getAborted(), @src());
}

export fn finish() void {
    // Mutation callbacks run in a microtask: the two children arrive together.
    expect(delivered_calls == 1 and delivered_records == 2 and delivered_added == 2, @src());
    expect(filtered_calls == 1 and filtered_name_ok, @src());
    expect(disconnected_calls == 0, @src());
    expect(resize_calls == 2, @src());
    expect(resize_widths[0] == 100 and resize_heights[0] == 50, @src());
    expect(resize_widths[1] == 150 and resize_heights[1] == 50, @src());
    expect(resize_target_ok and resize_inline_size == 150, @src());
    expect(near_seen and near_intersecting and near_ratio > 0.9, @src());
    expect(far_seen and !far_intersecting, @src());
    expect(timeout_signal.getAborted() and timeout_events == 1, @src());
    expect(b.jsErrorName(timeout_signal.getReason()).eql("TimeoutError"), @src());
}
