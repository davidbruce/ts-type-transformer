#include <cmath>
#include <string>

#include "dom/dom.hpp"
#include "konva/konva.hpp"

using namespace konva;

static Line make_line(double x1, double y1, double x2, double y2, const char* stroke, double sw) {
    return Line::create({{.stroke = stroke, .strokeWidth = sw, .points = ts::Array<double>::of(x1, y1, x2, y2)}});
}

static Rect make_rect(double x, double y, double w, double h, const char* fill, const char* stroke, double sw) {
    RectConfig rc({.x = x, .y = y, .width = w, .height = h, .fill = fill});
    if (stroke) {
        rc.stroke(stroke);
        rc.strokeWidth(sw);
    }
    return Rect(rc);
}

static Text make_text(double x, double y, const std::string& txt, double size, const char* fill) {
    return Text::create({{.x = x, .y = y, .fill = fill, .text = txt, .fontFamily = "monospace", .fontSize = size}});
}

TS_EXPORT(start) void start() {
    Stage stage = Stage::create({{.width = 680, .height = 400, .container = "stage"}});

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
    Group group_a = Group::create({{.x = 40, .y = 55}});
    group_a.add(
        make_rect(0, 0, 160, 120, "#f7f7f7", "#111111", 1.5),
        Circle::create({{.x = 80, .y = 60, .fill = "#111111", .radius = 24}}),
        make_text(12, 10, "Group A \xC2\xB7 0\xC2\xB0", 11, "#111111")
    );
    layer.add(group_a);

    // Group B: rotated 24 degrees.
    Group group_b = Group::create({{.x = 340, .y = 75, .rotation = 24}});
    group_b.add(
        make_rect(0, 0, 150, 110, "#efefef", "#111111", 2),
        Rect::create({{.x = 22, .y = 22, .width = 106, .height = 66, .fill = "#ffffff", .stroke = "#555555", .strokeWidth = 1.5, .dash = ts::Array<double>::of(4.0, 3.0)}}),
        Circle::create({{.x = 75, .y = 55, .fill = "#404040", .radius = 14}}),
        make_text(10, 8, "Group B \xC2\xB7 24\xC2\xB0", 11, "#111111")
    );
    layer.add(group_b);

    // Group C: scaled + nested rotated child group.
    Group group_c = Group::create({{.x = 495, .y = 55, .scaleX = 1.15, .scaleY = 1.15}});

    Group inner_group = Group::create({{.x = 25, .y = 25, .rotation = 15}});
    Circle marker = Circle::create({{.x = 40, .y = 30, .fill = "#111111", .radius = 16}});
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
