#include <cmath>
#include <string>

#include "dom/dom.hpp"
#include "konva/konva.hpp"

extern "C" __attribute__((export_name("start"))) void start() {
    constexpr double PI = 3.14159265358979323846;
    constexpr double cx = 340.0;
    constexpr double cy = 200.0;

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

    const double ring_r[] = { 160.0, 125.0, 90.0, 52.0 };
    const char* ring_stroke[] = { "#d4d4d4", "#a3a3a3", "#555555", "#111111" };
    const double ring_sw[] = { 1.0, 1.0, 1.0, 1.5 };
    for (int i = 0; i < 4; i++) {
        konva::CircleConfig cc = konva::CircleConfig::create();
        cc.x(cx);
        cc.y(cy);
        cc.radius(ring_r[i]);
        cc.stroke(konva::string_or_CanvasGradient(std::string(ring_stroke[i])));
        cc.strokeWidth(ring_sw[i]);
        if (i == 3) cc.fill(konva::string_or_CanvasGradient(std::string("#f7f7f7")));
        layer.add(ts::Array<konva::Group_or_Shape>{ konva::Circle::create(cc) });
    }

    for (int i = 0; i < 12; i++) {
        double a = static_cast<double>(i) * PI / 6.0;
        konva::LineConfig lc = konva::LineConfig::create();
        lc.points(konva::LinePoints(ts::Array<double>{
            cx + 36.0 * std::cos(a), cy + 36.0 * std::sin(a),
            cx + 160.0 * std::cos(a), cy + 160.0 * std::sin(a)
        }));
        lc.stroke(konva::string_or_CanvasGradient(std::string("#d4d4d4")));
        lc.strokeWidth(1);
        layer.add(ts::Array<konva::Group_or_Shape>{ konva::Line<konva::LineConfig>::create(lc) });
    }

    for (int i = 0; i < 24; i++) {
        double a = static_cast<double>(i) * PI / 12.0;
        double r0 = (i % 2 == 0) ? 148.0 : 154.0;
        konva::LineConfig lc = konva::LineConfig::create();
        lc.points(konva::LinePoints(ts::Array<double>{
            cx + r0 * std::cos(a), cy + r0 * std::sin(a),
            cx + 166.0 * std::cos(a), cy + 166.0 * std::sin(a)
        }));
        lc.stroke(konva::string_or_CanvasGradient(std::string("#111111")));
        lc.strokeWidth(i % 2 == 0 ? 2.0 : 1.0);
        layer.add(ts::Array<konva::Group_or_Shape>{ konva::Line<konva::LineConfig>::create(lc) });
    }

    for (int i = 0; i < 8; i++) {
        konva::RectConfig pc = konva::RectConfig::create();
        pc.x(cx);
        pc.y(cy);
        pc.width(18);
        pc.height(52);
        pc.offsetX(9);
        pc.offsetY(108);
        pc.rotation(static_cast<double>(i) * 45.0);
        pc.fill(konva::string_or_CanvasGradient(std::string(i % 2 == 0 ? "#111111" : "#efefef")));
        pc.stroke(konva::string_or_CanvasGradient(std::string("#111111")));
        pc.strokeWidth(1.5);
        layer.add(ts::Array<konva::Group_or_Shape>{ konva::Rect::create(pc) });
    }

    for (int i = 0; i < 12; i++) {
        double a = static_cast<double>(i) * PI / 6.0;
        konva::CircleConfig oc = konva::CircleConfig::create();
        oc.x(cx + 125.0 * std::cos(a));
        oc.y(cy + 125.0 * std::sin(a));
        oc.radius(5);
        oc.fill(konva::string_or_CanvasGradient(std::string(i % 2 == 0 ? "#111111" : "#ffffff")));
        oc.stroke(konva::string_or_CanvasGradient(std::string("#111111")));
        oc.strokeWidth(1.5);
        layer.add(ts::Array<konva::Group_or_Shape>{ konva::Circle::create(oc) });
    }

    konva::RegularPolygonConfig cp = konva::RegularPolygonConfig::create();
    cp.x(cx);
    cp.y(cy);
    cp.sides(8);
    cp.radius(26);
    cp.fill(konva::string_or_CanvasGradient(std::string("#111111")));
    layer.add(ts::Array<konva::Group_or_Shape>{ konva::RegularPolygon::create(cp) });

    konva::CircleConfig cd = konva::CircleConfig::create();
    cd.x(cx);
    cd.y(cy);
    cd.radius(9);
    cd.fill(konva::string_or_CanvasGradient(std::string("#ffffff")));
    layer.add(ts::Array<konva::Group_or_Shape>{ konva::Circle::create(cd) });

    const double lx[] = { 516, 332, 140, 328 };
    const double ly[] = { 194, 376, 194, 14 };
    const char* labels[] = { "0\xC2\xB0", "90\xC2\xB0", "180\xC2\xB0", "270\xC2\xB0" };
    for (int i = 0; i < 4; i++) {
        konva::TextConfig tc = konva::TextConfig::create();
        tc.x(lx[i]);
        tc.y(ly[i]);
        tc.text(labels[i]);
        tc.fontSize(11);
        tc.fontFamily("monospace");
        tc.fill(konva::string_or_CanvasGradient(std::string("#555555")));
        layer.add(ts::Array<konva::Group_or_Shape>{ konva::Text::create(tc) });
    }

    layer.draw();

    dom::Document doc = dom::get_document();
    dom::HTMLElement meta = *doc.getElementById("meta");
    meta.textContent(
        "Stage: 680x400 \xC2\xB7 Procedural rosette \xC2\xB7 " +
        std::to_string(layer.getChildren().size()) + " shapes on layer"
    );
}
