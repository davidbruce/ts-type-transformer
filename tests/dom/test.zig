// Drives a real page through the generated DOM bindings; every expect must hold.
const std = @import("std");
const b = @import("bindings.zig");

extern "env" fn check(ok: i32, line: u32) void;

fn expect(ok: bool, src: std.builtin.SourceLocation) void {
    check(@intFromBool(ok), src.line);
}


// What a listener needs: where to count, and where to show the count.
const Counter = struct {
    box: b.HTMLElement,
    attribute: []const u8,
    count: u8 = 0,

    fn bump(self: *Counter) void {
        self.count += 1;
        const digits = [1]u8{'0' + self.count};
        self.box.setAttribute(self.attribute, &digits);
    }
};

fn onClick(ctx: ?*anyopaque, _: b.PointerEvent) void {
    Counter.bump(@ptrCast(@alignCast(ctx)));
}

fn onPing(ctx: ?*anyopaque, _: b.Event) void {
    Counter.bump(@ptrCast(@alignCast(ctx)));
}

var clicks: Counter = undefined;
var pings: Counter = undefined;

export fn run() void {
    const doc = b.get_document();
    const body = doc.get_body();

    // Create an element and put it in the page.
    // The key is known at compile time: createElement(.div) returns an HTMLDivElement.
    const div: b.HTMLDivElement = doc.createElement(.div, .{});
    const box = div.asHTMLElement();
    box.set_id("made-by-zig");
    box.set_textContent("hello from zig");
    const classes = b.JsArrayOf(b.JsString).new();
    defer b.jsRelease(classes);
    classes.pushString("box");
    box.get_classList().add(classes);
    _ = body.appendChild(box.asNode());
    expect(b.jsSame(doc.getElementById("made-by-zig").?, box), @src());
    expect(box.get_classList().contains("box"), @src());

    // Read an element that was already in the page, and copy its text.
    const seed = doc.getElementById("seed").?;
    expect(seed.get_textContent().eql("seed text"), @src());
    expect(seed.getAttribute("data-x").?.eql("42"), @src());
    expect(seed.getAttribute("data-missing") == null, @src());
    var text: [64]u8 = undefined;
    const copy = doc.createElement(.p, .{}).asHTMLElement();
    copy.set_textContent(seed.get_textContent().read(&text));
    _ = box.appendChild(copy.asNode());

    // A listener called by real clicks, then removed.
    const button = doc.createElement(.button, .{}).asHTMLElement();
    button.set_textContent("press");
    _ = box.appendChild(button.asNode());
    clicks = .{ .box = box, .attribute = "data-count" };
    // The listener's event type is computed from the key: "click" takes a PointerEvent.
    const listener = b.JsListener(b.PointerEvent).from(onClick, &clicks);
    button.addEventListener(.click, listener, .{});
    button.click();
    button.click();
    expect(clicks.count == 2, @src());
    button.removeEventListener(.click, listener, .{});
    button.click();
    expect(clicks.count == 2, @src());

    // An options object, { once: true }, and events made and dispatched here.
    // Both union-typed parameters are tagged unions.
    pings = .{ .box = box, .attribute = "data-pings" };
    const once = b.AddEventListenerOptions.create();
    once.set_once(true);
    box.addEventListener_with_string(
        "ping",
        .{ .EventListener = b.EventListener.from(onPing, &pings) },
        .{ .options = .{ .AddEventListenerOptions = once } },
    );
    _ = box.dispatchEvent(b.Event.new("ping", .{}));
    _ = box.dispatchEvent(b.Event.new("ping", .{}));
    expect(pings.count == 1, @src());

    // Style, layout and the window.
    box.get_style().setProperty("color", "red", .{});
    expect(box.get_style().get_color().eql("red"), @src());
    expect(box.getBoundingClientRect().get_width() > 0, @src());
    expect(box.get_children().get_length() == 2, @src());

    // Collections iterate.
    const it = box.get_children().iterate();
    defer b.jsRelease(it);
    var children: usize = 0;
    while (it.next()) |child| {
        defer b.jsRelease(child);
        children += 1;
    }
    expect(children == 2, @src());
    expect(b.get_innerWidth() > 0, @src());
    doc.set_title("driven by zig");

    // Strings: compared in JavaScript, or copied.
    var short: [4]u8 = undefined;
    const seed_text = doc.getElementById("seed").?.get_textContent();
    expect(std.mem.eql(u8, seed_text.copy(&short), "seed"), @src());
    var fba_buf: [64]u8 = undefined;
    var fba = std.heap.FixedBufferAllocator.init(&fba_buf);
    expect(std.mem.eql(u8, seed_text.toOwned(fba.allocator()) catch "", "seed text"), @src());

    // An invalid selector throws: the call returns null, and the exception is kept.
    expect(box.querySelector_with_string("<bad") == null, @src());
    const bad = b.jsTakeError();
    expect(bad != null, @src());
    if (bad) |e| b.jsRelease(e);

    // A handle a function returns is the caller's to release.
    const live = b.jsLiveHandles();
    for (0..500) |_| {
        // Everything that comes back while the scope is open is released with it.
        const scope = b.JsScope.open();
        defer scope.close();
        const found = doc.getElementById("made-by-zig").?;
        expect(found.get_classList().contains("box") and found.get_id().eql("made-by-zig"), @src());
    }
    expect(b.jsLiveHandles() == live, @src());
}

// Reference every declaration so the whole module is checked, not just what is used.
comptime {
    @setEvalBranchQuota(100_000_000);
    for (std.meta.declarations(b)) |name| {
        const v = @field(b, name);
        if (@TypeOf(v) == type and @typeInfo(v) == .@"struct") {
            for (std.meta.declarations(v)) |member| _ = &@field(v, member);
        } else _ = &v;
    }
}
