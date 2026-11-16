// A 2D canvas from Zig: shapes drawn, pixels read back, paths, text metrics,
// state and transforms. Every expect must hold.
const std = @import("std");
const b = @import("bindings.zig");

extern "env" fn check(ok: i32, line: u32) void;

fn expect(ok: bool, src: std.builtin.SourceLocation) void {
    check(@intFromBool(ok), src.line);
}

const Rgba = [4]u8;

fn color(s: []const u8) b.string_or_CanvasGradient_or_CanvasPattern {
    return .{ .string = b.JsString.from(s) };
}

// The pixel at x, y.
fn pixel(ctx: b.CanvasRenderingContext2D, x: f64, y: f64) Rgba {
    const data = ctx.getImageData(x, y, 1, 1, .{});
    var out: Rgba = undefined;
    _ = data.getData().readBytes(0, &out);
    return out;
}

fn is(p: Rgba, r: u8, g: u8, bl: u8, a: u8) bool {
    return p[0] == r and p[1] == g and p[2] == bl and p[3] == a;
}

fn fillStyleIs(ctx: b.CanvasRenderingContext2D, want: []const u8) bool {
    return switch (ctx.getFillStyle()) {
        .string => |s| s.eql(want),
        else => false,
    };
}

export fn run() void {
    const doc = b.getDocument();
    const canvas = doc.createElement(.canvas, .{});
    canvas.setWidth(100);
    canvas.setHeight(60);
    _ = doc.getBody().asNode().appendChild(canvas.asHTMLElement().asNode());
    expect(canvas.getWidth() == 100 and canvas.getHeight() == 60, @src());
    const ctx = canvas.getContext2d(null).?;
    expect(b.jsSame(ctx.getCanvas(), canvas), @src());

    // A new canvas is transparent; the default fill is black.
    expect(is(pixel(ctx, 5, 5), 0, 0, 0, 0), @src());
    expect(fillStyleIs(ctx, "#000000"), @src());

    // A filled rectangle shows its color inside and nothing outside.
    ctx.setFillStyle(color("red"));
    expect(fillStyleIs(ctx, "#ff0000"), @src());
    ctx.fillRect(10, 10, 20, 20);
    expect(is(pixel(ctx, 15, 15), 255, 0, 0, 255), @src());
    expect(is(pixel(ctx, 29, 29), 255, 0, 0, 255), @src());
    expect(is(pixel(ctx, 30, 30), 0, 0, 0, 0), @src());
    expect(is(pixel(ctx, 9, 9), 0, 0, 0, 0), @src());

    // Clearing makes it transparent again.
    ctx.clearRect(12, 12, 6, 6);
    expect(is(pixel(ctx, 14, 14), 0, 0, 0, 0), @src());
    expect(is(pixel(ctx, 20, 20), 255, 0, 0, 255), @src());

    // An outline: the line is centred on the edge.
    ctx.setStrokeStyle(color("blue"));
    ctx.setLineWidth(2);
    expect(ctx.getLineWidth() == 2, @src());
    ctx.strokeRect(50, 10, 20, 20);
    expect(is(pixel(ctx, 50, 20), 0, 0, 255, 255), @src());
    expect(is(pixel(ctx, 49, 20), 0, 0, 255, 255), @src());
    expect(is(pixel(ctx, 60, 20), 0, 0, 0, 0), @src());

    // Pixels written with putImageData and read back.
    const patch = b.ImageData.new(2, 1, .{});
    expect(patch.getWidth() == 2 and patch.getHeight() == 1 and patch.getData().len() == 8, @src());
    _ = patch.getData().writeBytes(0, &[_]u8{ 1, 2, 3, 255, 9, 8, 7, 255 });
    ctx.putImageData(patch, 80, 40);
    expect(is(pixel(ctx, 80, 40), 1, 2, 3, 255), @src());
    expect(is(pixel(ctx, 81, 40), 9, 8, 7, 255), @src());
    const area = ctx.getImageData(80, 40, 2, 1, .{});
    expect(area.getData().byteLength() == 8, @src());
    var read: [8]u8 = undefined;
    expect(area.getData().readBytes(0, &read) == 8 and read[4] == 9, @src());
    expect(ctx.createImageData(3, 3, .{}).getData().len() == 36, @src());

    // A path: a filled circle and a stroked line.
    ctx.setFillStyle(color("#00ff00"));
    ctx.beginPath();
    ctx.arc(60, 45, 8, 0, 2 * std.math.pi, .{});
    ctx.fill(null);
    expect(is(pixel(ctx, 60, 45), 0, 255, 0, 255), @src());
    expect(is(pixel(ctx, 60, 38), 0, 255, 0, 255), @src());
    expect(is(pixel(ctx, 52, 38), 0, 0, 0, 0), @src());
    expect(ctx.isPointInPath(60, 45, .{}), @src());
    expect(!ctx.isPointInPath(0, 0, .{}), @src());
    ctx.setStrokeStyle(color("black"));
    ctx.setLineWidth(4);
    ctx.beginPath();
    ctx.moveTo(0, 55);
    ctx.lineTo(40, 55);
    ctx.stroke();
    expect(pixel(ctx, 20, 55)[3] == 255 and pixel(ctx, 20, 50)[3] == 0, @src());

    // A closed triangle of lines, filled.
    ctx.setFillStyle(color("#0000ff"));
    ctx.beginPath();
    ctx.moveTo(0, 0);
    ctx.lineTo(8, 0);
    ctx.lineTo(0, 8);
    ctx.closePath();
    ctx.fill(null);
    expect(is(pixel(ctx, 1, 1), 0, 0, 255, 255), @src());
    expect(is(pixel(ctx, 7, 7), 0, 0, 0, 0), @src());

    // save and restore bring state back.
    ctx.setFillStyle(color("#112233"));
    ctx.save();
    ctx.setFillStyle(color("#ffffff"));
    ctx.setGlobalAlpha(0.5);
    expect(fillStyleIs(ctx, "#ffffff") and ctx.getGlobalAlpha() == 0.5, @src());
    ctx.restore();
    expect(fillStyleIs(ctx, "#112233") and ctx.getGlobalAlpha() == 1, @src());

    // Transforms move what is drawn after them.
    ctx.reset();
    expect(ctx.getTransform().getIsIdentity(), @src());
    ctx.translate(30, 10);
    expect(ctx.getTransform().getE() == 30 and ctx.getTransform().getF() == 10, @src());
    ctx.setFillStyle(color("red"));
    ctx.fillRect(0, 0, 5, 5);
    expect(is(pixel(ctx, 32, 12), 255, 0, 0, 255), @src());
    expect(is(pixel(ctx, 2, 2), 0, 0, 0, 0), @src());
    ctx.scale(2, 3);
    expect(ctx.getTransform().getA() == 2 and ctx.getTransform().getD() == 3, @src());
    ctx.resetTransform();
    expect(ctx.getTransform().getIsIdentity(), @src());
    ctx.setTransform(1, 0, 0, 1, 50, 50);
    expect(ctx.getTransform().getE() == 50, @src());
    ctx.rotate(std.math.pi);
    ctx.fillRect(0, 0, 4, 4);
    expect(is(pixel(ctx, 48, 48), 255, 0, 0, 255), @src());
    ctx.resetTransform();

    // Half-transparent paint.
    ctx.clearRect(0, 0, 100, 60);
    ctx.setGlobalAlpha(0.5);
    ctx.setFillStyle(color("black"));
    ctx.fillRect(0, 0, 4, 4);
    const alpha = pixel(ctx, 1, 1)[3];
    expect(alpha >= 126 and alpha <= 129, @src());
    ctx.setGlobalAlpha(1);

    // A gradient is a fill style.
    ctx.clearRect(0, 0, 100, 60);
    const gradient = ctx.createLinearGradient(0, 0, 100, 0);
    gradient.addColorStop(0, "black");
    gradient.addColorStop(1, "white");
    ctx.setFillStyle(.{ .CanvasGradient = gradient });
    ctx.fillRect(0, 0, 100, 10);
    const mid = pixel(ctx, 50, 5);
    expect(mid[0] >= 120 and mid[0] <= 135 and mid[3] == 255, @src());
    expect(pixel(ctx, 2, 5)[0] < 10 and pixel(ctx, 97, 5)[0] > 245, @src());

    // Text measured: wider text is wider, and a monospace font is even.
    ctx.setFont("20px monospace");
    expect(ctx.getFont().eql("20px monospace"), @src());
    const short = ctx.measureText("ii").getWidth();
    const wide = ctx.measureText("WW").getWidth();
    expect(short > 0 and short == wide, @src());
    expect(ctx.measureText("hello world").getWidth() > ctx.measureText("hello").getWidth(), @src());
    expect(ctx.measureText("").getWidth() == 0, @src());
    ctx.setFillStyle(color("black"));
    ctx.fillText("Hi", 2, 50, .{});

    // The canvas as an image.
    var head: [32]u8 = undefined;
    const png = canvas.toDataURL(.{}).copy(&head);
    expect(std.mem.startsWith(u8, png, "data:image/png;base64,"), @src());
    expect(canvas.toDataURL(.{}).len() > 100, @src());
    const jpeg = canvas.toDataURL(.{ .type = "image/jpeg", .quality = 0.5 }).copy(&head);
    expect(std.mem.startsWith(u8, jpeg, "data:image/jpeg;base64,"), @src());

    // Resizing a canvas clears it and changes its size.
    canvas.setWidth(50);
    expect(canvas.getWidth() == 50, @src());
    expect(is(pixel(ctx, 5, 55), 0, 0, 0, 0), @src());
}
