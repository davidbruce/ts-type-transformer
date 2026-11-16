// Konva from C++, in a page: a stage on the page's element, a layer with
// shapes and a group, their positions and bounds, finding, events, JSON.
// Every CHECK must hold.
#include <cmath>

#include "bindings.hpp"

extern "C" TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

static bool near(double a, double b, double eps = 1e-6) { return std::fabs(a - b) < eps; }

// FINDING 19: a Rect is a ::Shape<RectConfig>, which does not convert to the
// ::Shape<ShapeConfig> in `Group_or_Shape` (the same object in TypeScript), so
// layer.add(rect) does not compile: the handle is rewrapped as a Shape<ShapeConfig>.
static Group_or_Shape child(const ts::Handle& h) {
    ::Shape<ShapeConfig> shape;
    static_cast<ts::Handle&>(shape) = h;
    return Group_or_Shape(shape);
}
static Group_or_Shape child(const ::Group& g) { return Group_or_Shape(g); }

// FINDING 20: konva's `x`, `width`, `fill`... are methods on the prototype typed as
// GetSet properties (callable objects). The getter returns the bare method, and
// GetSet::invoke calls it without its receiver: "Cannot read properties of
// undefined (reading 'getX')". Nothing of the GetSet API works; attributes are
// read with getAttr and written with setAttrs.
#define ATTR(node, Cfg, key) (node).getAttr<Cfg, std::string>(std::string(key))

static ts::Value str(std::string_view s) { return ts::Value::string(s); }

static void exercise() {
    // A stage on the page's #stage, by its id, 400 by 300.
    StageConfig config = StageConfig::create();
    config.container(HTMLDivElement_or_string(std::string("stage")));
    config.width(400);
    config.height(300);
    ::Stage stage = ::Stage::create(config);
    CHECK(stage.getContent().h != 0);
    CHECK(stage.getType() == "Stage");
    CHECK(stage.getClassName() == "Stage");

    ::Layer layer = ::Layer::create();
    stage.add(layer, ts::Array<>::create());
    CHECK(stage.getChildren().size() == 1);
    CHECK(stage.children().size() == 1);
    CHECK(layer.getStage() == stage);
    CHECK(layer.getType() == "Layer");

    // A rectangle made from a config: position, size, colour, id and name.
    RectConfig rc = RectConfig::create();
    rc.x(20);
    rc.y(30);
    rc.width(100);
    rc.height(50);
    rc.fill(string_or_CanvasGradient(std::string("red")));
    rc.stroke(string_or_CanvasGradient(std::string("black")));
    rc.strokeWidth(4);
    rc.id("box");
    rc.name("shape box");
    ::Rect rect = ::Rect::create(rc);
    CHECK(ATTR(rect, RectConfig, "x").to_number() == 20);
    CHECK(ATTR(rect, RectConfig, "y").to_number() == 30);
    CHECK(ATTR(rect, RectConfig, "width").to_number() == 100);
    CHECK(ATTR(rect, RectConfig, "id").to_string() == "box");
    CHECK(ATTR(rect, RectConfig, "name").to_string() == "shape box");
    CHECK(rect.getClassName() == "Rect");
    layer.add(ts::Array<Group_or_Shape>{child(rect)});
    CHECK(layer.getChildren().size() == 1);
    CHECK(rect.getLayer() == layer);
    CHECK(rect.getStage() == stage);
    CHECK(rect.getParent().has_value());

    // A Circle and a Text, from their configs.
    CircleConfig cc = CircleConfig::create();
    cc.x(200);
    cc.y(100);
    cc.radius(40);
    cc.fill(string_or_CanvasGradient(std::string("green")));
    cc.name("shape dot");
    ::Circle circle = ::Circle::create(cc);
    CHECK(ATTR(circle, CircleConfig, "radius").to_number() == 40);
    CHECK(circle.getWidth() == 80 && circle.getHeight() == 80);

    TextConfig tc = TextConfig::create();
    tc.x(10);
    tc.y(200);
    tc.text("Hello Konva");
    tc.fontSize(24);
    tc.fontFamily("sans-serif");
    tc.name("shape text");
    ::Text label = ::Text::create(tc);
    CHECK(ATTR(label, TextConfig, "text").to_string() == "Hello Konva");
    CHECK(ATTR(label, TextConfig, "fontSize").to_number() == 24);
    CHECK(label.getTextWidth() > 50);
    CHECK(label.getTextHeight() > 0);
    label.setText("Hi");
    CHECK(ATTR(label, TextConfig, "text").to_string() == "Hi");

    // A group moves what is in it.
    GroupConfig gc = GroupConfig::create();
    gc.x(100);
    gc.y(100);
    gc.name("cluster");
    ::Group group = ::Group::create(gc);
    group.add(ts::Array<Group_or_Shape>{child(circle), child(label)});
    layer.add(ts::Array<Group_or_Shape>{child(group)});
    CHECK(group.getChildren().size() == 2);
    CHECK(layer.getChildren().size() == 2);
    CHECK(circle.getParent().has_value() && *circle.getParent() == group);

    // Positions: a node's own and its absolute one.
    CHECK(ATTR(circle, CircleConfig, "x").to_number() == 200);
    Node_getAbsolutePosition_result abs = circle.getAbsolutePosition();
    CHECK(near(abs.x(), 300) && near(abs.y(), 200));
    Node_getAbsolutePosition_result rel = circle.getPosition();
    CHECK(rel.x() == 200 && rel.y() == 100);
    RectConfig moved = RectConfig::create();
    moved.x(60);
    rect.setAttrs(moved);
    CHECK(ATTR(rect, RectConfig, "x").to_number() == 60);
    CHECK(near(rect.getClientRect().x(), 58));
    moved.x(20);
    moved.y(30);
    rect.setAttrs(moved);
    CHECK(ATTR(rect, RectConfig, "x").to_number() == 20 && ATTR(rect, RectConfig, "y").to_number() == 30);

    // Scale, rotation, opacity, visibility.
    RectConfig look = RectConfig::create();
    look.scaleX(2);
    look.rotation(90);
    look.opacity(0.5);
    rect.setAttrs(look);
    CHECK(ATTR(rect, RectConfig, "scaleX").to_number() == 2);
    CHECK(near(rect.getAbsoluteScale().x(), 2));
    CHECK(ATTR(rect, RectConfig, "rotation").to_number() == 90);
    CHECK(ATTR(rect, RectConfig, "opacity").to_number() == 0.5);
    rect.hide();
    CHECK(!rect.isVisible().to_bool());
    rect.show();
    CHECK(rect.isVisible().to_bool());

    // The client rect: the shape's bounds in its parent, stroke included.
    RectConfig plain = RectConfig::create();
    plain.scaleX(1);
    plain.rotation(0);
    plain.opacity(1);
    rect.setAttrs(plain);
    IRect box = rect.getClientRect();
    CHECK(near(box.x(), 18) && near(box.y(), 28));
    CHECK(near(box.width(), 104) && near(box.height(), 54));
    IRect dotbox = circle.getClientRect();
    CHECK(near(dotbox.width(), 80));

    // Finding by name, by id and by type. (FINDING 19 again: find<::Rect> fails the
    // constraint `extends Node<NodeConfig>`, as Rect is a Node<RectConfig>.)
    ts::Array<::Node<NodeConfig>> boxes = layer.find<::Node<NodeConfig>>(str("#box"));
    CHECK(boxes.size() == 1);
    CHECK(layer.find<::Node<NodeConfig>>(str(".shape")).size() == 3);
    CHECK(layer.find<::Node<NodeConfig>>(str("Circle")).size() == 1);
    CHECK(layer.findOne(string_or_Function(std::string("#box"))).has_value());
    CHECK(!layer.findOne(string_or_Function(std::string("#nothing"))).has_value());
    CHECK(group.find<::Node<NodeConfig>>(str(".shape")).size() == 2);

    // Events: on_click, fired by hand; bubbling to the layer.
    int clicks = 0;
    std::string target;
    KonvaEventListener<::Node<RectConfig>, PointerEvent> onClick = [&](KonvaEventObject<PointerEvent, ::Node<RectConfig>> e) {
        clicks++;
        target = e.type();
    };
    rect.on_click(onClick);
    rect.fire("click");
    rect.fire("click");
    CHECK(clicks == 2 && target == "click");
    rect.off("click");
    rect.fire("click");
    CHECK(clicks == 2);
    onClick.release();

    // Draw, and the draw's effect on the canvas.
    layer.draw();
    CHECK(layer.getNativeCanvasElement().h != 0);
    stage.draw();
    CHECK(stage.toDataURL().rfind("data:image/png", 0) == 0);

    // Z order.
    CHECK(rect.getZIndex() == 0);
    rect.moveToTop();
    CHECK(rect.getZIndex() == 1);
    rect.moveToBottom();
    CHECK(rect.getZIndex() == 0);

    // toJSON of a node holds its className and attrs.
    std::string json = rect.toJSON();
    CHECK(json.find("\"className\":\"Rect\"") != std::string::npos);
    CHECK(json.find("\"width\":100") != std::string::npos);
    CHECK(json.find("\"fill\":\"red\"") != std::string::npos);

    // Removing and destroying.
    group.destroy();
    CHECK(layer.getChildren().size() == 1);
    rect.remove();
    CHECK(layer.getChildren().size() == 0);
    stage.destroy();
    CHECK(!ts::take_error());
}

extern "C" __attribute__((export_name("run"))) void run() {
    exercise();
}
