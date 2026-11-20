#include <string>

#include "dom/dom.hpp"
#include "konva/konva.hpp"

using namespace konva;

static void add_caption(const Layer& layer, double x, double y, const char* label) {
    TextConfig tc = TextConfig::create();
    tc.x(x);
    tc.y(y);
    tc.text(label);
    tc.fontSize(11);
    tc.fontFamily("monospace");
    tc.fill("#555555");
    layer.add(Text::create(tc));
}

TS_EXPORT(start) void start() {
    StageConfig sc = StageConfig::create();
    sc.container("stage");
    sc.width(680);
    sc.height(400);
    Stage stage = Stage::create(sc);

    Layer layer = Layer::create();
    stage.add(layer);

    RectConfig bg = RectConfig::create();
    bg.x(0);
    bg.y(0);
    bg.width(680);
    bg.height(400);
    bg.fill("#ffffff");
    layer.add(Rect::create(bg));

    // Row 1: Rounded filled Rect, dashed Rect, Circle, Hexagon.
    RectConfig r1 = RectConfig::create();
    r1.x(40);
    r1.y(55);
    r1.width(110);
    r1.height(76);
    r1.cornerRadius(8.0);
    r1.fill("#111111");
    layer.add(Rect::create(r1));
    add_caption(layer, 40, 142, "Rect (filled)");

    RectConfig r2 = RectConfig::create();
    r2.x(195);
    r2.y(55);
    r2.width(110);
    r2.height(76);
    r2.fill("#ffffff");
    r2.stroke("#111111");
    r2.strokeWidth(2);
    r2.dash(ts::Array<double>::of(6.0, 4.0));
    layer.add(Rect::create(r2));
    add_caption(layer, 195, 142, "Rect (dashed)");

    CircleConfig cc = CircleConfig::create();
    cc.x(415);
    cc.y(93);
    cc.radius(40);
    cc.fill("#404040");
    cc.stroke("#111111");
    cc.strokeWidth(2);
    layer.add(Circle::create(cc));
    add_caption(layer, 395, 142, "Circle");

    RegularPolygonConfig pc = RegularPolygonConfig::create();
    pc.x(575);
    pc.y(93);
    pc.sides(6);
    pc.radius(42);
    pc.fill("#efefef");
    pc.stroke("#111111");
    pc.strokeWidth(2);
    layer.add(RegularPolygon::create(pc));
    add_caption(layer, 548, 142, "Hexagon");

    // Row 2: Star, Ring, Wedge, Framed Text.
    StarConfig st = StarConfig::create();
    st.x(95);
    st.y(268);
    st.numPoints(5);
    st.innerRadius(19);
    st.outerRadius(44);
    st.fill("#737373");
    st.stroke("#111111");
    st.strokeWidth(2);
    layer.add(Star::create(st));
    add_caption(layer, 62, 325, "Star (5pt)");

    RingConfig rc = RingConfig::create();
    rc.x(250);
    rc.y(268);
    rc.innerRadius(22);
    rc.outerRadius(42);
    rc.fill("#d4d4d4");
    rc.stroke("#111111");
    rc.strokeWidth(2);
    layer.add(Ring::create(rc));
    add_caption(layer, 236, 325, "Ring");

    WedgeConfig wc = WedgeConfig::create();
    wc.x(415);
    wc.y(268);
    wc.radius(44);
    wc.angle(270);
    wc.rotation(-90);
    wc.fill("#111111");
    wc.stroke("#555555");
    wc.strokeWidth(1);
    layer.add(Wedge::create(wc));
    add_caption(layer, 382, 325, "Wedge 270\xC2\xB0");

    RectConfig tf = RectConfig::create();
    tf.x(500);
    tf.y(224);
    tf.width(150);
    tf.height(88);
    tf.fill("#f7f7f7");
    tf.stroke("#111111");
    tf.strokeWidth(1.5);

    TextConfig tc = TextConfig::create();
    tc.x(514);
    tc.y(240);
    tc.text("KONVA 2D\nMonochrome\nScene Graph");
    tc.fontSize(13);
    tc.fontFamily("monospace");
    tc.fill("#111111");
    layer.add(Rect::create(tf), Text::create(tc));
    add_caption(layer, 558, 325, "Text");

    layer.draw();

    dom::get_document().getElementById("meta")->textContent(
        "Stage: 680x400 \xC2\xB7 7 primitive types \xC2\xB7 " +
        std::to_string(layer.getChildren().size()) + " layer children"
    );
}
