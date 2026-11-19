#include <string>

#include "dom/dom.hpp"
#include "konva/konva.hpp"

static konva::Line<konva::LineConfig> make_line(double x1, double y1, double x2, double y2, const std::string& color, double width) {
    konva::LineConfig lc = konva::LineConfig::create();
    lc.points(konva::LinePoints(ts::Array<double>{ x1, y1, x2, y2 }));
    lc.stroke(konva::string_or_CanvasGradient(color));
    lc.strokeWidth(width);
    return konva::Line<konva::LineConfig>::create(lc);
}

static konva::Rect make_card(const std::string& id, double x, double y, const std::string& fill) {
    konva::RectConfig rc = konva::RectConfig::create();
    rc.id(id);
    rc.name("card");
    rc.x(x);
    rc.y(y);
    rc.width(200);
    rc.height(130);
    rc.fill(konva::string_or_CanvasGradient(fill));
    rc.stroke(konva::string_or_CanvasGradient(std::string("#111111")));
    rc.strokeWidth(2);
    return konva::Rect::create(rc);
}

extern "C" __attribute__((export_name("start"))) void start() {
    konva::StageConfig sc = konva::StageConfig::create();
    sc.container(konva::HTMLDivElement_or_string(std::string("stage")));
    sc.width(680);
    sc.height(400);
    konva::Stage stage = konva::Stage::create(sc);

    konva::Layer bg_layer = konva::Layer::create();
    konva::Layer fg_layer = konva::Layer::create();
    stage.add(bg_layer, ts::Array<>::create());
    stage.add(fg_layer, ts::Array<>::create());

    // Layer 1: Blueprint Grid.
    konva::RectConfig bgr = konva::RectConfig::create();
    bgr.x(0);
    bgr.y(0);
    bgr.width(680);
    bgr.height(400);
    bgr.fill(konva::string_or_CanvasGradient(std::string("#ffffff")));
    bg_layer.add(ts::Array<konva::Group_or_Shape>{ konva::Rect::create(bgr) });

    for (int x = 40; x <= 640; x += 40) {
        bg_layer.add(ts::Array<konva::Group_or_Shape>{ make_line(x, 20, x, 380, "#e5e5e5", 1) });
    }
    for (int y = 40; y <= 360; y += 40) {
        bg_layer.add(ts::Array<konva::Group_or_Shape>{ make_line(20, y, 660, y, "#e5e5e5", 1) });
    }

    konva::RectConfig frame = konva::RectConfig::create();
    frame.x(20);
    frame.y(20);
    frame.width(640);
    frame.height(360);
    frame.stroke(konva::string_or_CanvasGradient(std::string("#b5b5b5")));
    frame.strokeWidth(1.5);
    frame.dash(ts::Array<double>{ 6.0, 4.0 });
    konva::TextConfig l1 = konva::TextConfig::create();
    l1.x(32);
    l1.y(32);
    l1.text("Layer 1: Blueprint Grid (z=0)");
    l1.fontSize(12);
    l1.fontFamily("monospace");
    l1.fill(konva::string_or_CanvasGradient(std::string("#737373")));
    bg_layer.add(ts::Array<konva::Group_or_Shape>{ konva::Rect::create(frame), konva::Text::create(l1) });

    // Layer 2: Stacked cards & badge.
    konva::Rect card_a = make_card("card-a", 110, 80, "#efefef");
    konva::Rect card_b = make_card("card-b", 210, 125, "#404040");
    konva::Rect card_c = make_card("card-c", 310, 170, "#ffffff");
    konva::CircleConfig bc = konva::CircleConfig::create();
    bc.id("badge");
    bc.x(555);
    bc.y(135);
    bc.radius(38);
    bc.fill(konva::string_or_CanvasGradient(std::string("#111111")));
    fg_layer.add(ts::Array<konva::Group_or_Shape>{ card_a, card_b, card_c, konva::Circle::create(bc) });

    card_a.moveToTop();
    int card_a_z = static_cast<int>(card_a.getZIndex());

    int clicks = 0;
    konva::KonvaEventListener<konva::Node<konva::RectConfig>, konva::PointerEvent> on_click =
        [&](konva::KonvaEventObject<konva::PointerEvent, konva::Node<konva::RectConfig>>) { clicks++; };
    card_a.on_click(on_click);
    card_a.fire("click");
    card_a.fire("click");
    on_click.release();

    bg_layer.draw();
    fg_layer.draw();

    konva::Vector2d pt = konva::Vector2d::create();
    pt.x(250);
    pt.y(160);
    auto hit = stage.getIntersection(pt);
    std::string hit_id = hit ? hit->id().invoke() : std::string("none");

    konva::CircleConfig dc = konva::CircleConfig::create();
    dc.x(250);
    dc.y(160);
    dc.radius(7);
    dc.fill(konva::string_or_CanvasGradient(std::string("#ffffff")));
    dc.stroke(konva::string_or_CanvasGradient(std::string("#111111")));
    dc.strokeWidth(2);
    konva::TextConfig l2 = konva::TextConfig::create();
    l2.x(32);
    l2.y(354);
    l2.text("Layer 2: Elevated Stack (z=1) \xC2\xB7 Probe (250,160)");
    l2.fontSize(12);
    l2.fontFamily("monospace");
    l2.fill(konva::string_or_CanvasGradient(std::string("#111111")));
    fg_layer.add(ts::Array<konva::Group_or_Shape>{
        konva::Circle::create(dc),
        make_line(238, 160, 262, 160, "#111111", 1.5),
        make_line(250, 148, 250, 172, "#111111", 1.5),
        konva::Text::create(l2)
    });
    fg_layer.draw();

    dom::Document doc = dom::get_document();
    dom::HTMLElement meta = *doc.getElementById("meta");
    meta.textContent(
        "Layers: " + std::to_string(stage.getLayers().size()) +
        " \xC2\xB7 card-a zIndex=" + std::to_string(card_a_z) +
        " \xC2\xB7 hit@(250,160)=" + hit_id +
        " \xC2\xB7 clicks=" + std::to_string(clicks)
    );
}
