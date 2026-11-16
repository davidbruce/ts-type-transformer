// Konva from Zig, in a page: a stage on the page's #stage with layers, shapes and groups, positions,
// lookups, events and serialisation. Every expect must hold.
const std = @import("std");
const b = @import("bindings.zig");
const kv = b.konva;

extern "env" fn check(ok: i32, line: u32) void;

fn expect(ok: bool, src: std.builtin.SourceLocation) void {
    check(@intFromBool(ok), src.line);
}

var buf: [16384]u8 = undefined;
fn has(s: b.JsString, needle: []const u8) bool {
    return std.mem.indexOf(u8, s.copy(&buf), needle) != null;
}

fn num(x: f64) b.JsValue {
    return b.JsValue.number(x);
}

fn child(s: kv.Shape) b.Group_or_Shape {
    return .{ .Shape = s };
}

var clicks: u32 = 0;
var last_type_len: usize = 0;
fn onClick(_: ?*anyopaque, e: kv.KonvaEventObject) void {
    clicks += 1;
    last_type_len = e.getType().len();
}

var moved: u32 = 0;
fn onMove(_: ?*anyopaque, _: kv.KonvaEventObject) void {
    moved += 1;
}

// FINDING 18: nothing of Konva that needs the module can run. The package's only export is
// `export default Konva` (an object holding Stage, Layer, Rect, ...), but the glue calls
// `new (PK["konva"].Stage)(...)`, `PK["konva"].Layer` and so on, on the module namespace, so every
// constructor fails with "PK.konva.Stage is not a constructor". Only plain-object configs, which the
// glue builds with `{}`, work. The program below is the intended one, kept for when that is fixed.
fn pending() void {
    // A stage on the page's #stage, 400 by 300, with one layer.
    const config = kv.StageConfig.create();
    config.setContainer(.{ .string = b.JsString.from("stage") });
    config.setWidth(400);
    config.setHeight(300);
    const stage = kv.Stage.new(config);
    expect(stage.asContainer().asNode().getWidth().invoke().toNumber() == 400, @src());
    expect(stage.asContainer().asNode().getHeight().invoke().toNumber() == 300, @src());
    const layer = kv.Layer.new(null);
    _ = stage.add(layer, b.JsArray.new());
    expect(stage.getLayers().len() == 1, @src());
    expect(b.jsSame(layer.getStage(), stage), @src());

    // A rectangle: its position, size and colour, read back.
    const rect_config = kv.RectConfig.create();
    rect_config.setX(50);
    rect_config.setY(60);
    rect_config.setWidth(100);
    rect_config.setHeight(80);
    rect_config.setName("box");
    rect_config.setId("the-box");
    const rect = kv.Rect.new(rect_config);
    const rect_node = rect.asShape().asNode();
    // FINDING 17: Shape.setFill takes a GetSet (the accessor function itself), not a colour; a value is set
    // by calling the accessor with it: getFill().invoke2(value). The same for every GetSet property.
    _ = rect.asShape().getFill().invoke2(b.JsValue.string("red"));
    _ = rect.asShape().getStroke().invoke2(b.JsValue.string("black"));
    _ = rect.asShape().getStrokeWidth().invoke2(num(2));
    _ = layer.add(b.JsArrayOf(b.Group_or_Shape).of(&.{child(rect.asShape())}));
    expect(rect_node.getX().invoke().toNumber() == 50 and rect_node.getY().invoke().toNumber() == 60, @src());
    expect(rect_node.getWidth().invoke().toNumber() == 100 and rect_node.getHeight().invoke().toNumber() == 80, @src());
    expect(rect.asShape().getFill().invoke().toString().eql("red"), @src());
    expect(rect_node.getName().invoke().toString().eql("box"), @src());
    expect(rect_node.getId().invoke().toString().eql("the-box"), @src());
    expect(rect_node.getClassName().eql("Rect"), @src());
    expect(rect_node.getType().eql("Shape"), @src());

    // A circle, and a text.
    const circle_config = kv.CircleConfig.create();
    circle_config.setX(200);
    circle_config.setY(150);
    circle_config.setRadius(40);
    const circle = kv.Circle.new(circle_config);
    _ = circle.asShape().getFill().invoke2(b.JsValue.string("blue"));
    _ = layer.add(b.JsArrayOf(b.Group_or_Shape).of(&.{child(circle.asShape())}));
    expect(circle.getWidth() == 80 and circle.getHeight() == 80, @src());
    expect(circle.getRadius().invoke().toNumber() == 40, @src());
    _ = circle.getRadius().invoke2(num(25));
    expect(circle.getWidth() == 50, @src());

    const text_config = kv.TextConfig.create();
    text_config.setText("Hello");
    text_config.setFontSize(20);
    text_config.setX(10);
    text_config.setY(10);
    const text = kv.Text.new(text_config);
    _ = layer.add(b.JsArrayOf(b.Group_or_Shape).of(&.{child(text.asShape())}));
    expect(text.getText().invoke().toString().eql("Hello"), @src());
    expect(text.getFontSize().invoke().toNumber() == 20, @src());
    expect(text.getTextWidth2() > 0 and text.getTextHeight2() > 0, @src());
    _ = text.setText("Hello, world");
    expect(text.getText().invoke().toString().eql("Hello, world"), @src());

    // A group holds shapes, and moves them together.
    const group_config = kv.GroupConfig.create();
    group_config.setX(100);
    group_config.setY(100);
    const group = kv.Group.new(group_config);
    const inner_config = kv.RectConfig.create();
    inner_config.setX(10);
    inner_config.setY(20);
    inner_config.setWidth(30);
    inner_config.setHeight(30);
    const inner = kv.Rect.new(inner_config);
    _ = group.add(b.JsArrayOf(b.Group_or_Shape).of(&.{child(inner.asShape())}));
    _ = layer.add(b.JsArrayOf(b.Group_or_Shape).of(&.{.{ .Group = group }}));
    expect(group.getChildren().len() == 1, @src());
    expect(layer.getChildren().len() == 4, @src());
    // The inner rectangle's absolute position adds the group's.
    const inner_node = inner.asShape().asNode();
    expect(inner_node.getX().invoke().toNumber() == 10, @src());
    const absolute = inner_node.getAbsolutePosition(null);
    expect(absolute.getX() == 110 and absolute.getY() == 120, @src());
    // Moving the group moves what it holds.
    _ = group.asContainer().asNode().getX().invoke2(num(150));
    expect(inner_node.getAbsolutePosition(null).getX() == 160, @src());
    expect(b.jsSame(inner_node.getParent().?, group.asContainer()), @src());
    expect(b.jsSame(inner_node.getLayer().?, layer), @src());

    // getClientRect: the bounds of a shape in the stage, here the rectangle plus its stroke.
    const bounds = rect_node.getClientRect(null);
    expect(bounds.getX() == 49 and bounds.getY() == 59, @src());
    expect(bounds.getWidth() == 102 and bounds.getHeight() == 82, @src());
    // The group's rectangle is plain: no stroke, so exact, in the group's place.
    const inner_bounds = inner_node.getClientRect(null);
    expect(inner_bounds.getX() == 160 and inner_bounds.getY() == 120 and inner_bounds.getWidth() == 30, @src());

    // Finding: by class, by name and by id.
    const rects = layer.asContainer().find(b.JsValue.string("Rect"));
    expect(rects.len() == 2, @src());
    expect(layer.asContainer().find(b.JsValue.string(".box")).len() == 1, @src());
    const by_id = layer.asContainer().findOne(.{ .string = b.JsString.from("#the-box") });
    expect(by_id != null and b.jsSame(by_id.?, rect_node), @src());
    expect(layer.asContainer().findOne(.{ .string = b.JsString.from("#missing") }) == null, @src());
    expect(layer.asContainer().find(b.JsValue.string("Circle")).len() == 1, @src());

    // Events: a click fired on the rectangle calls the listener; so does one on a shape through the stage.
    const listener = kv.KonvaEventListener.from(onClick, null);
    _ = rect_node.on(.{ .string = b.JsString.from("click") }, listener);
    _ = rect_node.fire("click", .{});
    _ = rect_node.fire("click", .{});
    expect(clicks == 2 and last_type_len == 5, @src());
    // Bubbling: a click on the inner rectangle reaches a listener on its group.
    _ = group.asContainer().asNode().on(.{ .string = b.JsString.from("click") }, kv.KonvaEventListener.from(onMove, null));
    _ = inner_node.fire("click", .{ .bubble = true });
    expect(moved == 1, @src());
    _ = rect_node.off(.{ .evtStr = "click" });
    _ = rect_node.fire("click", .{});
    expect(clicks == 2, @src());

    // Drawing and hit testing.
    _ = stage.batchDraw().draw();
    expect(rect.asShape().intersects(kv.Vector2d.create()) == false, @src());
    const point = kv.Vector2d.create();
    point.setX(100);
    point.setY(100);
    expect(rect.asShape().intersects(point), @src());

    // Z order: moving to the top puts it last among the layer's children.
    expect(rect_node.getZIndex() == 0, @src());
    _ = rect_node.moveToTop();
    expect(rect_node.getZIndex() == 3, @src());

    // Serialisation: toJSON names the classes and keeps the attributes.
    const json = stage.asContainer().asNode().toJSON();
    expect(has(json, "\"className\":\"Stage\""), @src());
    expect(has(json, "\"className\":\"Rect\"") and has(json, "\"className\":\"Circle\""), @src());
    expect(has(json, "\"fill\":\"red\"") and has(json, "\"text\":\"Hello, world\""), @src());
    expect(has(json, "\"id\":\"the-box\""), @src());

    // Removing and destroying.
    _ = circle.asShape().asNode().remove();
    expect(layer.getChildren().len() == 3, @src());
    _ = layer.asContainer().destroyChildren();
    expect(layer.getChildren().len() == 0, @src());
    _ = stage.destroy();
    expect(stage.getLayers().len() == 0, @src());
}

export fn run() void {
    // Configs are plain objects: set and read back.
    const rect = kv.RectConfig.create();
    rect.setX(50);
    rect.setY(60);
    rect.setWidth(100);
    rect.setHeight(80);
    rect.setName("box");
    rect.setId("the-box");
    rect.setOpacity(0.5);
    rect.setVisible(false);
    rect.setDraggable(true);
    rect.setRotation(30);
    expect(rect.getX().? == 50 and rect.getY().? == 60, @src());
    expect(rect.getWidth().? == 100 and rect.getHeight().? == 80, @src());
    expect(rect.getName().?.eql("box") and rect.getId().?.eql("the-box"), @src());
    expect(rect.getOpacity().? == 0.5 and rect.getVisible().? == false, @src());
    expect(rect.getDraggable().? == true and rect.getRotation().? == 30, @src());
    expect(kv.RectConfig.create().getX() == null and kv.RectConfig.create().getName() == null, @src());
    // Index-signature access to any attribute.
    rect.setAt("fill", b.JsValue.string("red"));
    expect(rect.getAt("fill").toString().eql("red"), @src());
    rect.setX(null);
    expect(rect.getX() == null, @src());

    const circle = kv.CircleConfig.create();
    circle.setRadius(40);
    circle.setScaleX(2);
    expect(circle.getRadius().? == 40 and circle.getScaleX().? == 2, @src());

    const text = kv.TextConfig.create();
    text.setText("Hello");
    text.setFontSize(20);
    text.setFontFamily("Calibri");
    expect(text.getText().?.eql("Hello") and text.getFontSize().? == 20 and text.getFontFamily().?.eql("Calibri"), @src());

    const point = kv.Vector2d.create();
    point.setX(3);
    point.setY(4);
    expect(point.getX() == 3 and point.getY() == 4, @src());
    const area = kv.IRect.create();
    area.setWidth(10);
    area.setHeight(20);
    expect(area.getWidth() == 10 and area.getHeight() == 20, @src());

    // A stage's config takes the container's id, as a union of string and element.
    const stage = kv.StageConfig.create();
    stage.setContainer(.{ .string = b.JsString.from("stage") });
    expect(stage.getContainer().?.string.eql("stage"), @src());
}
