#include <string>

#include "dom/dom.hpp"
#include "konva/konva.hpp"

using namespace konva;

static Line<LineConfig> make_line(double x1, double y1, double x2, double y2, const char* color, double width) {
    LineConfig lc = LineConfig::create();
    lc.points(ts::Array<double>::of(x1, y1, x2, y2));
    lc.stroke(color);
    lc.strokeWidth(width);
    return Line<LineConfig>::create(lc);
}

static Rect make_card(const char* id, double x, double y, const char* fill) {
    RectConfig rc = RectConfig::create();
    rc.id(id);
    rc.name("card");
    rc.x(x);
    rc.y(y);
    rc.width(200);
    rc.height(130);
    rc.fill(fill);
    rc.stroke("#111111");
    rc.strokeWidth(2);
    return Rect::create(rc);
}

static Text make_label(double x, double y, const char* txt, const char* fill) {
    TextConfig tc = TextConfig::create();
    tc.x(x);
    tc.y(y);
    tc.text(txt);
    tc.fontSize(12);
    tc.fontFamily("monospace");
    tc.fill(fill);
    return Text::create(tc);
}

TS_EXPORT(start) void start() {
    StageConfig sc = StageConfig::create();
    sc.container("stage");
    sc.width(680);
    sc.height(400);
    Stage stage = Stage::create(sc);

    Layer bg_layer = Layer::create();
    Layer fg_layer = Layer::create();
    stage.add(bg_layer);
    stage.add(fg_layer);

    // Layer 1: Blueprint Grid.
    RectConfig bgr = RectConfig::create();
    bgr.x(0);
    bgr.y(0);
    bgr.width(680);
    bgr.height(400);
    bgr.fill("#ffffff");
    bg_layer.add(Rect::create(bgr));

    for (int x = 40; x <= 640; x += 40) {
        bg_layer.add(make_line(x, 20, x, 380, "#e5e5e5", 1));
    }
    for (int y = 40; y <= 360; y += 40) {
        bg_layer.add(make_line(20, y, 660, y, "#e5e5e5", 1));
    }

    RectConfig frame = RectConfig::create();
    frame.x(20);
    frame.y(20);
    frame.width(640);
    frame.height(360);
    frame.stroke("#b5b5b5");
    frame.strokeWidth(1.5);
    frame.dash(ts::Array<double>::of(6.0, 4.0));
    bg_layer.add(Rect::create(frame), make_label(32, 32, "Layer 1: Blueprint Grid (z=0)", "#737373"));

    // Layer 2: Stacked cards & badge.
    Rect card_a = make_card("card-a", 110, 80, "#efefef");
    Rect card_b = make_card("card-b", 210, 125, "#404040");
    Rect card_c = make_card("card-c", 310, 170, "#ffffff");
    CircleConfig bc = CircleConfig::create();
    bc.id("badge");
    bc.x(555);
    bc.y(135);
    bc.radius(38);
    bc.fill("#111111");
    fg_layer.add(card_a, card_b, card_c, Circle::create(bc));

    card_a.moveToTop();
    int card_a_z = static_cast<int>(card_a.getZIndex());

    int clicks = 0;
    {
        ts::Scoped<KonvaEventListener<Node<RectConfig>, PointerEvent>> on_click =
            [&](KonvaEventObject<PointerEvent, Node<RectConfig>>) { clicks++; };
        card_a.on_click(on_click);
        card_a.fire("click");
        card_a.fire("click");
    }

    bg_layer.draw();
    fg_layer.draw();

    Vector2d pt = Vector2d::create();
    pt.x(250);
    pt.y(160);
    auto hit = stage.getIntersection(pt);
    std::string hit_id = hit ? hit->id()() : std::string("none");

    CircleConfig dc = CircleConfig::create();
    dc.x(250);
    dc.y(160);
    dc.radius(7);
    dc.fill("#ffffff");
    dc.stroke("#111111");
    dc.strokeWidth(2);
    fg_layer.add(
        Circle::create(dc),
        make_line(238, 160, 262, 160, "#111111", 1.5),
        make_line(250, 148, 250, 172, "#111111", 1.5),
        make_label(32, 354, "Layer 2: Elevated Stack (z=1) \xC2\xB7 Probe (250,160)", "#111111")
    );
    fg_layer.draw();

    dom::get_document().getElementById("meta")->textContent(
        "Layers: " + std::to_string(stage.getLayers().size()) +
        " \xC2\xB7 card-a zIndex=" + std::to_string(card_a_z) +
        " \xC2\xB7 hit@(250,160)=" + hit_id +
        " \xC2\xB7 clicks=" + std::to_string(clicks)
    );
}
