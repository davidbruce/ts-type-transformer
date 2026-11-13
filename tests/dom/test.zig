// Drives a real page through the generated DOM bindings; every expect must hold.
const std = @import("std");
const b = @import("bindings.zig");

extern "env" fn check(ok: i32, line: u32) void;

fn expect(ok: bool, src: std.builtin.SourceLocation) void {
    check(@intFromBool(ok), src.line);
}

fn strEq(s: b.JsString, expected: []const u8) bool {
    var buf: [128]u8 = undefined;
    if (s.len() > buf.len) return false;
    return std.mem.eql(u8, s.read(&buf), expected);
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

fn onClick(ctx: ?*anyopaque, _: b.PointerEvent) callconv(.c) b.JsValue {
    Counter.bump(@ptrCast(@alignCast(ctx)));
    return .{ .h = 0 };
}

fn onPing(ctx: ?*anyopaque, _: b.Event) callconv(.c) void {
    Counter.bump(@ptrCast(@alignCast(ctx)));
}

var clicks: Counter = undefined;
var pings: Counter = undefined;

export fn run() void {
    const doc = b.get_document();
    const body = doc.get_body();

    // Create an element and put it in the page.
    // One method per tag, returning the tag's own element type.
    const div: b.HTMLDivElement = doc.createElement_div();
    const box = div.asHTMLElement();
    box.set_id("made-by-zig");
    box.set_textContent("hello from zig");
    const classes = b.JsArray.new();
    classes.push(b.JsValue.string("box"));
    box.get_classList().add_with_tokens(classes);
    _ = body.appendChild(box.asNode());
    expect(doc.getElementById("made-by-zig").?.h == box.h, @src());
    expect(box.get_classList().contains("box"), @src());

    // Read an element that was already in the page, and copy its text.
    const seed = doc.getElementById("seed").?;
    expect(strEq(seed.get_textContent(), "seed text"), @src());
    expect(strEq(seed.getAttribute("data-x").toString(), "42"), @src());
    expect(seed.getAttribute("data-missing").isNull(), @src());
    var text: [64]u8 = undefined;
    const copy = doc.createElement_p().asHTMLElement();
    copy.set_textContent(seed.get_textContent().read(&text));
    _ = box.appendChild(copy.asNode());

    // A listener called by real clicks, then removed.
    const button = doc.createElement_button().asHTMLElement();
    button.set_textContent("press");
    _ = box.appendChild(button.asNode());
    clicks = .{ .box = box, .attribute = "data-count" };
    // One method per event, with the event's own type.
    const listener = b.HTMLElement_PointerEvent_fn.from(onClick, &clicks);
    button.addEventListener_click(listener);
    button.click();
    button.click();
    expect(clicks.count == 2, @src());
    button.removeEventListener_click(listener);
    button.click();
    expect(clicks.count == 2, @src());

    // An options object, { once: true }, and events made and dispatched here.
    // Both union-typed parameters are tagged unions.
    pings = .{ .box = box, .attribute = "data-pings" };
    const once = b.AddEventListenerOptions.create();
    once.set_once(b.JsValue.boolean(true));
    box.addEventListener_with_options(
        "ping",
        .{ .EventListener = b.EventListener.from(onPing, &pings) },
        .{ .AddEventListenerOptions = once },
    );
    _ = box.dispatchEvent(b.Event.new("ping"));
    _ = box.dispatchEvent(b.Event.new("ping"));
    expect(pings.count == 1, @src());

    // Style, layout and the window.
    box.get_style().setProperty("color", b.JsValue.string("red"));
    expect(strEq(box.get_style().get_color(), "red"), @src());
    expect(box.getBoundingClientRect().get_width() > 0, @src());
    expect(box.get_children().get_length() == 2, @src());
    expect(b.get_innerWidth() > 0, @src());
    doc.set_title("driven by zig");
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
