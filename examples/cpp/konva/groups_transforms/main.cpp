#include <cmath>
#include <string>

#include "dom/dom.hpp"
#include "konva/konva.hpp"

using namespace konva;

static Line<LineConfig> make_line(double x1, double y1, double x2, double y2, const char* stroke, double sw) {
    LineConfig lc = LineConfig::create();
    lc.points(ts::Array<double>::of(x1, y1, x2, y2));
    lc.stroke(stroke);
    lc.strokeWidth(sw);
    return Line<LineConfig>::create(lc);
}

static Rect make_rect(double x, double y, double w, double h, const char* fill, const char* stroke, double sw) {
    RectConfig rc = RectConfig::create();
    rc.x(x);
    rc.y(y);
    rc.width(w);
    rc.height(h);
    rc.fill(fill);
    if (stroke) {
        rc.stroke(stroke);
        rc.strokeWidth(sw);
    }
    return Rect::create(rc);
}

static Text make_text(double x, double y, const std::string& txt, double size, const char* fill) {
    TextConfig tc = TextConfig::create();
    tc.x(x);
    tc.y(y);
    tc.text(txt);
    tc.fontSize(size);
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

    Layer layer = Layer::create();
    stage.add(layer);

    layer.add(make_rect(0, 0, 680, 400, "#ffffff", nullptr, 0));
    for (int x = 40; x <= 640; x += 40) {
        layer.add(make_line(x, 0, x, 400, "#e5e5e5", 1));
    }
    for (int y = 40; y <= 360; y += 40) {
        layer.add(make_line(0, y, 680, y, "#e5e5e5", 1));
    }

    // Group A: unrotated base frame.
    GroupConfig gac = GroupConfig::create();
    gac.x(40);
    gac.y(55);
    Group group_a = Group::create(gac);
    CircleConfig gad = CircleConfig::create();
    gad.x(80);
    gad.y(60);
    gad.radius(24);
    gad.fill("#111111");
    group_a.add(
        make_rect(0, 0, 160, 120, "#f7f7f7", "#111111", 1.5),
        Circle::create(gad),
        make_text(12, 10, "Group A \xC2\xB7 0\xC2\xB0", 11, "#111111")
    );
    layer.add(group_a);

    // Group B: rotated 24 degrees.
    GroupConfig gbc = GroupConfig::create();
    gbc.x(340);
    gbc.y(75);
    gbc.rotation(24);
    Group group_b = Group::create(gbc);
    RectConfig gbi = RectConfig::create();
    gbi.x(22);
    gbi.y(22);
    gbi.width(106);
    gbi.height(66);
    gbi.fill("#ffffff");
    gbi.stroke("#555555");
    gbi.strokeWidth(1.5);
    gbi.dash(ts::Array<double>::of(4.0, 3.0));
    CircleConfig gbd = CircleConfig::create();
    gbd.x(75);
    gbd.y(55);
    gbd.radius(14);
    gbd.fill("#404040");
    group_b.add(
        make_rect(0, 0, 150, 110, "#efefef", "#111111", 2),
        Rect::create(gbi),
        Circle::create(gbd),
        make_text(10, 8, "Group B \xC2\xB7 24\xC2\xB0", 11, "#111111")
    );
    layer.add(group_b);

    // Group C: scaled + nested rotated child group.
    GroupConfig gcc = GroupConfig::create();
    gcc.x(495);
    gcc.y(55);
    gcc.scaleX(1.15);
    gcc.scaleY(1.15);
    Group group_c = Group::create(gcc);

    GroupConfig icc = GroupConfig::create();
    icc.x(25);
    icc.y(25);
    icc.rotation(15);
    Group inner_group = Group::create(icc);
    CircleConfig mc = CircleConfig::create();
    mc.x(40);
    mc.y(30);
    mc.radius(16);
    mc.fill("#111111");
    Circle marker = Circle::create(mc);
    inner_group.add(marker);
    group_c.add(
        make_rect(0, 0, 130, 110, "#ffffff", "#111111", 2),
        make_text(8, 8, "Group C \xC2\xB7 1.15x", 10, "#111111"),
        inner_group
    );
    layer.add(group_c);

    auto abs = marker.getAbsolutePosition();
    IRect bbox = group_b.getClientRect();
    long ax = std::lround(abs.x());
    long ay = std::lround(abs.y());
    long bw = std::lround(bbox.width());
    long bh = std::lround(bbox.height());
    std::string strip_txt =
        "Marker abs=(" + std::to_string(ax) + ", " + std::to_string(ay) +
        ") \xC2\xB7 Group B bbox=" + std::to_string(bw) + "x" + std::to_string(bh);

    layer.add(
        make_rect(28, 308, 624, 60, "#f7f7f7", "#111111", 1.5),
        make_text(44, 331, strip_txt, 12, "#111111")
    );
    layer.draw();

    dom::get_document().getElementById("meta")->textContent(
        "Stage: 680x400 \xC2\xB7 Nested marker abs=(" + std::to_string(ax) + ", " + std::to_string(ay) +
        ") \xC2\xB7 Rotated Group B rect=" + std::to_string(bw) + "x" + std::to_string(bh)
    );
}
