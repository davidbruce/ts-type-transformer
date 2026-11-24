#include <cmath>
#include <string>

#include "dom/dom.hpp"
#include "konva/konva.hpp"

using namespace konva;

TS_EXPORT(start) void start() {
    constexpr double PI = 3.14159265358979323846;
    constexpr double cx = 340.0;
    constexpr double cy = 200.0;

    Stage stage = Stage::create({{.width = 680, .height = 400, .container = "stage"}});

    Layer layer = Layer::create();
    stage.add(layer);

    layer.add(Rect::create({{.x = 0, .y = 0, .width = 680, .height = 400, .fill = "#ffffff"}}));

    const double ring_r[] = { 160.0, 125.0, 90.0, 52.0 };
    const char* ring_stroke[] = { "#d4d4d4", "#a3a3a3", "#555555", "#111111" };
    const double ring_sw[] = { 1.0, 1.0, 1.0, 1.5 };
    for (int i = 0; i < 4; i++) {
        CircleConfig cc({.x = cx, .y = cy, .stroke = ring_stroke[i], .strokeWidth = ring_sw[i], .radius = ring_r[i]});
        if (i == 3) cc.fill("#f7f7f7");
        layer.add(Circle(cc));
    }

    for (int i = 0; i < 12; i++) {
        double a = static_cast<double>(i) * PI / 6.0;
        layer.add(Line::create({{.stroke = "#d4d4d4", .strokeWidth = 1, .points = ts::Array<double>::of(
            cx + 36.0 * std::cos(a), cy + 36.0 * std::sin(a),
            cx + 160.0 * std::cos(a), cy + 160.0 * std::sin(a)
        )}}));
    }

    for (int i = 0; i < 24; i++) {
        double a = static_cast<double>(i) * PI / 12.0;
        double r0 = (i % 2 == 0) ? 148.0 : 154.0;
        layer.add(Line::create({{.stroke = "#111111", .strokeWidth = i % 2 == 0 ? 2.0 : 1.0, .points = ts::Array<double>::of(
            cx + r0 * std::cos(a), cy + r0 * std::sin(a),
            cx + 166.0 * std::cos(a), cy + 166.0 * std::sin(a)
        )}}));
    }

    for (int i = 0; i < 8; i++) {
        layer.add(Rect::create({{
            .x = cx, .y = cy, .width = 18, .height = 52, .rotation = static_cast<double>(i) * 45.0, .offsetX = 9, .offsetY = 108,
            .fill = i % 2 == 0 ? "#111111" : "#efefef", .stroke = "#111111", .strokeWidth = 1.5
        }}));
    }

    for (int i = 0; i < 12; i++) {
        double a = static_cast<double>(i) * PI / 6.0;
        layer.add(Circle::create({{
            .x = cx + 125.0 * std::cos(a), .y = cy + 125.0 * std::sin(a),
            .fill = i % 2 == 0 ? "#111111" : "#ffffff", .stroke = "#111111", .strokeWidth = 1.5, .radius = 5
        }}));
    }

    layer.add(
        RegularPolygon::create({{.x = cx, .y = cy, .fill = "#111111", .sides = 8, .radius = 26}}),
        Circle::create({{.x = cx, .y = cy, .fill = "#ffffff", .radius = 9}})
    );

    const double lx[] = { 516, 332, 140, 328 };
    const double ly[] = { 194, 376, 194, 14 };
    const char* labels[] = { "0\xC2\xB0", "90\xC2\xB0", "180\xC2\xB0", "270\xC2\xB0" };
    for (int i = 0; i < 4; i++) {
        layer.add(Text::create({{.x = lx[i], .y = ly[i], .fill = "#555555", .text = labels[i], .fontFamily = "monospace", .fontSize = 11}}));
    }

    layer.draw();

    dom::get_document().getElementById("meta")->textContent(
        "Stage: 680x400 \xC2\xB7 Procedural rosette \xC2\xB7 " +
        std::to_string(layer.getChildren().size()) + " shapes on layer"
    );
}
