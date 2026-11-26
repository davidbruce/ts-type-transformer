// pixi.js v8 from Zig, in a page: math, a scene graph, graphics and sprites. Every expect must hold.
const std = @import("std");
const b = @import("bindings.zig");
const px = b.pixi_js;

extern "env" fn check(ok: i32, line: u32) void;

fn expect(ok: bool, src: std.builtin.SourceLocation) void {
    check(@intFromBool(ok), src.line);
}

fn near(a: f64, c: f64) bool {
    return @abs(a - c) < 1e-6;
}

var app: px.Application = undefined;
var ready: bool = false;

export fn busy() i32 {
    return @intFromBool(!ready);
}

// An Application on the page's canvas: init is a promise.
fn onInit(_: ?*anyopaque, ok: bool, v: b.JsValue) void {
    defer ready = true;
    if (!ok) {
        const m = b.jsErrorMessage(v);
        defer b.jsRelease(m);
        expect(false, @src());
        return;
    }
    expect(app.getScreen().getWidth() == 400 and app.getScreen().getHeight() == 300, @src());
    expect(b.jsSame(app.getCanvas(), b.test_page.canvas()), @src());
    // The stage is a container: draw on it and render.
    const stage = app.getStage();
    const bar = px.Graphics.new(null);
    _ = bar.rect(0, 0, 50, 20);
    _ = bar.fillWithColorAndAlpha(.{ .number = 0x3366ff }, .{});
    _ = stage.asPixiMixins_Container().addChild(b.JsArrayOf(px.Container).of(&.{bar.asViewContainer().asContainer()}));
    expect(stage.getChildren().len() == 1, @src());
    app.render();
    // After a render the world transform of the child is current.
    expect(bar.asViewContainer().asContainer().getWorldTransform().getTx() == 0, @src());
    app.destroy(.{});
}

export fn run() void {
    // An Application on the page's canvas, started below.
    app = px.Application.new();
    const init_options = b.Application_init_options.create();
    init_options.setCanvas(.{ .h = b.test_page.canvas().h });
    init_options.setWidth(400);
    init_options.setHeight(300);
    init_options.setPreference(.{ .string = b.JsString.from("webgl") });
    init_options.setAutoStart(false);
    const promise = app.init(init_options);
    defer b.jsRelease(promise);
    promise.then(onInit, null);

    // Points. FINDING 15: Point's add, dot, magnitude, normalize and the rest of Vector2Math are declared
    // (by pixi.js/math-extras, a side-effect import) but absent at run time ("get(...).magnitude is not a
    // function"); the bindings expose them and nothing can load the extras, so they are left out.
    const p = px.Point.new(.{ .x = 3, .y = 4 });
    expect(p.getX() == 3 and p.getY() == 4, @src());
    const q = p.clone();
    q.setX(10);
    expect(p.getX() == 3 and q.getX() == 10, @src());
    expect(p.equals(.{ .h = px.Point.new(.{ .x = 3, .y = 4 }).h }), @src());
    expect(p.toString().len() > 0, @src());
    const origin = px.Point.new(.{});
    expect(origin.getX() == 0 and origin.getY() == 0, @src());
    _ = origin.set(.{ .x = 7, .y = 8 });
    expect(origin.getX() == 7 and origin.getY() == 8, @src());

    // Rectangles.
    const r = px.Rectangle.new(.{ .x = .{ .number = 10 }, .y = .{ .number = 20 }, .width = .{ .number = 30 }, .height = .{ .number = 40 } });
    expect(r.getX() == 10 and r.getRight() == 40 and r.getBottom() == 60, @src());
    expect(r.getLeft() == 10 and r.getTop() == 20, @src());
    expect(r.contains(15, 25) and !r.contains(5, 25), @src());
    expect(!r.isEmpty() and px.Rectangle.new(.{}).isEmpty(), @src());
    const other = px.Rectangle.new(.{ .x = .{ .number = 30 }, .y = .{ .number = 30 }, .width = .{ .number = 50 }, .height = .{ .number = 50 } });
    expect(r.intersects(other), @src());
    expect(!r.intersects(px.Rectangle.new(.{ .x = .{ .number = 100 }, .y = .{ .number = 100 }, .width = .{ .number = 5 }, .height = .{ .number = 5 } })), @src());
    const padded = r.clone().pad(.{ .paddingX = 5 });
    expect(padded.getX() == 5 and padded.getWidth() == 40 and padded.getHeight() == 50, @src());
    expect(r.containsRect(px.Rectangle.new(.{ .x = .{ .number = 12 }, .y = .{ .number = 22 }, .width = .{ .number = 5 }, .height = .{ .number = 5 } })), @src());
    expect(r.getWidth() == 30, @src());

    // Matrices: transform and invert.
    const m = px.Matrix.new(.{});
    expect(m.isIdentity() and m.getA() == 1 and m.getD() == 1, @src());
    _ = m.translate(10, 20).scale(2, 3);
    expect(m.getA() == 2 and m.getD() == 3 and m.getTx() == 20 and m.getTy() == 60, @src());
    const mapped = m.apply(.{ .h = px.Point.new(.{ .x = 1, .y = 1 }).h });
    expect(@as(px.Point, .{ .h = mapped.h }).getX() == 22 and @as(px.Point, .{ .h = mapped.h }).getY() == 63, @src());
    const back = m.applyInverse(.{ .h = px.Point.new(.{ .x = 22, .y = 63 }).h });
    expect(near(@as(px.Point, .{ .h = back.h }).getX(), 1) and near(@as(px.Point, .{ .h = back.h }).getY(), 1), @src());
    const inv = m.clone().invert();
    expect(near(inv.getA(), 0.5) and near(inv.getD(), 1.0 / 3.0), @src());
    expect(!m.isIdentity() and m.clone().equals(m), @src());
    _ = m.identity();
    expect(m.isIdentity(), @src());
    const spin = px.Matrix.new(.{}).rotate(std.math.pi / 2.0);
    expect(near(spin.getA(), 0) and near(spin.getB(), 1), @src());

    // A scene graph: positions compose into world transforms.
    const root = px.Container.new(null);
    const child = px.Container.new(null);
    const leaf = px.Container.new(null);
    root.setX(100);
    root.setY(50);
    child.setX(10);
    child.setY(5);
    leaf.setX(1);
    // addChild takes an array of children; a Container is the argument as it is.
    _ = root.asPixiMixins_Container().addChild(b.JsArrayOf(px.Container).of(&.{child}));
    _ = child.asPixiMixins_Container().addChild(b.JsArrayOf(px.Container).of(&.{leaf}));
    expect(root.getChildren().len() == 1 and child.getChildren().len() == 1, @src());
    expect(b.jsSame(root.asPixiMixins_Container().getChildAt(0), child), @src());
    expect(root.asPixiMixins_Container().getChildIndex(child) == 0, @src());
    expect(b.jsSame(leaf.getParent().?, child), @src());
    // The world position of the leaf is the sum of the positions above it.
    const world = leaf.asPixiMixins_Container().toGlobal(.{ .h = px.Point.new(.{}).h });
    const wp: px.Point = .{ .h = world.h };
    expect(wp.getX() == 111 and wp.getY() == 55, @src());
    // The scale of a parent carries down (not its own position).
    _ = root.getScale().set(.{ .x = 2, .y = 2 });
    const scaled_world: px.Point = .{ .h = leaf.asPixiMixins_Container().toGlobal(.{ .h = px.Point.new(.{}).h }).h };
    expect(scaled_world.getX() == 122 and scaled_world.getY() == 60, @src());
    // Properties of a container.
    child.setAlpha(0.5);
    child.setVisible(false);
    child.setRotation(1.5);
    expect(child.getAlpha() == 0.5 and !child.getVisible() and near(child.getRotation(), 1.5), @src());
    expect(near(child.getAngle(), 1.5 * 180.0 / std.math.pi), @src());
    // Removing a child detaches it.
    _ = root.asPixiMixins_Container().removeChild(b.JsArrayOf(px.Container).of(&.{child}));
    expect(root.getChildren().len() == 0 and child.getParent() == null, @src());

    // Graphics: a drawn rectangle has bounds.
    const g = px.Graphics.new(null);
    _ = g.rect(10, 20, 100, 50);
    _ = g.fillWithColorAndAlpha(.{ .number = 0xff0000 }, .{});
    const gb = g.asViewContainer().asContainer().asPixiMixins_Container().getBounds(.{});
    expect(gb.getMinX() == 10 and gb.getMinY() == 20, @src());
    expect(gb.getMaxX() == 110 and gb.getMaxY() == 70, @src());
    expect(gb.getRectangle().getWidth() == 100, @src());
    // A graphics object is a container: it can be placed, and its bounds move.
    g.asViewContainer().asContainer().setX(5);
    expect(g.asViewContainer().asContainer().asPixiMixins_Container().getBounds(.{}).getMinX() == 15, @src());
    expect(g.asViewContainer().asContainer().asPixiMixins_Container().getLocalBounds().getMinX() == 10, @src());
    _ = g.circle(200, 200, 10);
    _ = g.fillWithColorAndAlpha(.{ .number = 0x00ff00 }, .{});
    expect(g.asViewContainer().asContainer().asPixiMixins_Container().getLocalBounds().getMaxX() == 210, @src());
    _ = g.clear();
    expect(g.asViewContainer().asContainer().asPixiMixins_Container().getLocalBounds().getRectangle().getWidth() == 0, @src());

    // A sprite with the white texture: 1 by 1 until sized.
    const white = px.Texture.getWHITE();
    expect(white.getWidth() == 1 and white.getHeight() == 1, @src());
    const sprite = px.Sprite.new(.{ .Texture = white });
    expect(b.jsSame(sprite.getTexture(), white), @src());
    // FINDING 14: Graphics and Sprite convert only to their direct parent (asViewContainer), so reaching
    // Container methods takes a chain of conversions: asViewContainer().asContainer().
    const placed = sprite.asViewContainer().asContainer();
    placed.setX(30);
    placed.setY(40);
    _ = sprite.getAnchor().set(.{ .x = 0.5, .y = 0.5 });
    expect(sprite.getAnchor().get_X() == 0.5, @src());
    // Sized, a sprite's bounds are its texture frame scaled.
    placed.asPixiMixins_Container().setWidth(20);
    placed.asPixiMixins_Container().setHeight(10);
    const sb = placed.asPixiMixins_Container().getBounds(.{});
    expect(near(sb.getMinX(), 20) and near(sb.getMaxX(), 40), @src());
    expect(near(sb.getMinY(), 35) and near(sb.getMaxY(), 45), @src());
}
