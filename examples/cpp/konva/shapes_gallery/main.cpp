#include <string>

#include "dom/dom.hpp"
#include "konva/konva.hpp"

static void add_caption(const konva::Layer& layer, double x, double y, const std::string& label) {
    konva::TextConfig tc = konva::TextConfig::create();
    tc.x(x);
    tc.y(y);
    tc.text(label);
    tc.fontSize(11);
    tc.fontFamily("monospace");
    tc.fill(konva::string_or_CanvasGradient(std::string("#555555")));
    layer.add(ts::Array<konva::Group_or_Shape>{ konva::Text::create(tc) });
}

extern "C" __attribute__((export_name("start"))) void start() {
    konva::StageConfig sc = konva::StageConfig::create();
    sc.container(konva::HTMLDivElement_or_string(std::string("stage")));
    sc.width(680);
    sc.height(400);
    konva::Stage stage = konva::Stage::create(sc);

    konva::Layer layer = konva::Layer::create();
    stage.add(layer, ts::Array<>::create());

    konva::RectConfig bg = konva::RectConfig::create();
    bg.x(0);
    bg.y(0);
    bg.width(680);
    bg.height(400);
    bg.fill(konva::string_or_CanvasGradient(std::string("#ffffff")));
    layer.add(ts::Array<konva::Group_or_Shape>{ konva::Rect::create(bg) });

    // Row 1: Rounded filled Rect, dashed Rect, Circle, Hexagon.
    konva::RectConfig r1 = konva::RectConfig::create();
    r1.x(40);
    r1.y(55);
    r1.width(110);
    r1.height(76);
    r1.cornerRadius(konva::number_or_number_array(8.0));
    r1.fill(konva::string_or_CanvasGradient(std::string("#111111")));
    layer.add(ts::Array<konva::Group_or_Shape>{ konva::Rect::create(r1) });
    add_caption(layer, 40, 142, "Rect (filled)");

    konva::RectConfig r2 = konva::RectConfig::create();
    r2.x(195);
    r2.y(55);
    r2.width(110);
    r2.height(76);
    r2.fill(konva::string_or_CanvasGradient(std::string("#ffffff")));
    r2.stroke(konva::string_or_CanvasGradient(std::string("#111111")));
    r2.strokeWidth(2);
    r2.dash(ts::Array<double>{ 6.0, 4.0 });
    layer.add(ts::Array<konva::Group_or_Shape>{ konva::Rect::create(r2) });
    add_caption(layer, 195, 142, "Rect (dashed)");

    konva::CircleConfig cc = konva::CircleConfig::create();
    cc.x(415);
    cc.y(93);
    cc.radius(40);
    cc.fill(konva::string_or_CanvasGradient(std::string("#404040")));
    cc.stroke(konva::string_or_CanvasGradient(std::string("#111111")));
    cc.strokeWidth(2);
    layer.add(ts::Array<konva::Group_or_Shape>{ konva::Circle::create(cc) });
    add_caption(layer, 395, 142, "Circle");

    konva::RegularPolygonConfig pc = konva::RegularPolygonConfig::create();
    pc.x(575);
    pc.y(93);
    pc.sides(6);
    pc.radius(42);
    pc.fill(konva::string_or_CanvasGradient(std::string("#efefef")));
    pc.stroke(konva::string_or_CanvasGradient(std::string("#111111")));
    pc.strokeWidth(2);
    layer.add(ts::Array<konva::Group_or_Shape>{ konva::RegularPolygon::create(pc) });
    add_caption(layer, 548, 142, "Hexagon");

    // Row 2: Star, Ring, Wedge, Framed Text.
    konva::StarConfig st = konva::StarConfig::create();
    st.x(95);
    st.y(268);
    st.numPoints(5);
    st.innerRadius(19);
    st.outerRadius(44);
    st.fill(konva::string_or_CanvasGradient(std::string("#737373")));
    st.stroke(konva::string_or_CanvasGradient(std::string("#111111")));
    st.strokeWidth(2);
    layer.add(ts::Array<konva::Group_or_Shape>{ konva::Star::create(st) });
    add_caption(layer, 62, 325, "Star (5pt)");

    konva::RingConfig rc = konva::RingConfig::create();
    rc.x(250);
    rc.y(268);
    rc.innerRadius(22);
    rc.outerRadius(42);
    rc.fill(konva::string_or_CanvasGradient(std::string("#d4d4d4")));
    rc.stroke(konva::string_or_CanvasGradient(std::string("#111111")));
    rc.strokeWidth(2);
    layer.add(ts::Array<konva::Group_or_Shape>{ konva::Ring::create(rc) });
    add_caption(layer, 236, 325, "Ring");

    konva::WedgeConfig wc = konva::WedgeConfig::create();
    wc.x(415);
    wc.y(268);
    wc.radius(44);
    wc.angle(270);
    wc.rotation(-90);
    wc.fill(konva::string_or_CanvasGradient(std::string("#111111")));
    wc.stroke(konva::string_or_CanvasGradient(std::string("#555555")));
    wc.strokeWidth(1);
    layer.add(ts::Array<konva::Group_or_Shape>{ konva::Wedge::create(wc) });
    add_caption(layer, 382, 325, "Wedge 270\xC2\xB0");

    konva::RectConfig tf = konva::RectConfig::create();
    tf.x(500);
    tf.y(224);
    tf.width(150);
    tf.height(88);
    tf.fill(konva::string_or_CanvasGradient(std::string("#f7f7f7")));
    tf.stroke(konva::string_or_CanvasGradient(std::string("#111111")));
    tf.strokeWidth(1.5);
    layer.add(ts::Array<konva::Group_or_Shape>{ konva::Rect::create(tf) });

    konva::TextConfig tc = konva::TextConfig::create();
    tc.x(514);
    tc.y(240);
    tc.text("KONVA 2D\nMonochrome\nScene Graph");
    tc.fontSize(13);
    tc.fontFamily("monospace");
    tc.fill(konva::string_or_CanvasGradient(std::string("#111111")));
    layer.add(ts::Array<konva::Group_or_Shape>{ konva::Text::create(tc) });
    add_caption(layer, 558, 325, "Text");

    layer.draw();

    dom::Document doc = dom::get_document();
    dom::HTMLElement meta = *doc.getElementById("meta");
    meta.textContent(
        "Stage: 680x400 \xC2\xB7 7 primitive types \xC2\xB7 " +
        std::to_string(layer.getChildren().size()) + " layer children"
    );
}
