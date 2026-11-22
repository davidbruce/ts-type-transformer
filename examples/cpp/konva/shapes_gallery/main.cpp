#include <string>

#include "dom/dom.hpp"
#include "konva/konva.hpp"

using namespace konva;

static void add_caption(const Layer& layer, double x, double y, const char* label) {
    layer.add(Text::create({{.x = x, .y = y, .fill = "#555555", .text = label, .fontFamily = "monospace", .fontSize = 11}}));
}

TS_EXPORT(start) void start() {
    Stage stage = Stage::create({{.width = 680, .height = 400, .container = "stage"}});

    Layer layer = Layer::create();
    stage.add(layer);

    layer.add(Rect::create({{.x = 0, .y = 0, .width = 680, .height = 400, .fill = "#ffffff"}}));

    // Row 1: Rounded filled Rect, dashed Rect, Circle, Hexagon.
    layer.add(Rect::create({{.x = 40, .y = 55, .width = 110, .height = 76, .fill = "#111111", .cornerRadius = 8.0}}));
    add_caption(layer, 40, 142, "Rect (filled)");

    layer.add(Rect::create({{.x = 195, .y = 55, .width = 110, .height = 76, .fill = "#ffffff", .stroke = "#111111", .strokeWidth = 2, .dash = ts::Array<double>::of(6.0, 4.0)}}));
    add_caption(layer, 195, 142, "Rect (dashed)");

    layer.add(Circle::create({{.x = 415, .y = 93, .fill = "#404040", .stroke = "#111111", .strokeWidth = 2, .radius = 40}}));
    add_caption(layer, 395, 142, "Circle");

    layer.add(RegularPolygon::create({{.x = 575, .y = 93, .fill = "#efefef", .stroke = "#111111", .strokeWidth = 2, .sides = 6, .radius = 42}}));
    add_caption(layer, 548, 142, "Hexagon");

    // Row 2: Star, Ring, Wedge, Framed Text.
    layer.add(Star::create({{.x = 95, .y = 268, .fill = "#737373", .stroke = "#111111", .strokeWidth = 2, .numPoints = 5, .innerRadius = 19, .outerRadius = 44}}));
    add_caption(layer, 62, 325, "Star (5pt)");

    layer.add(Ring::create({{.x = 250, .y = 268, .fill = "#d4d4d4", .stroke = "#111111", .strokeWidth = 2, .innerRadius = 22, .outerRadius = 42}}));
    add_caption(layer, 236, 325, "Ring");

    layer.add(Wedge::create({{.x = 415, .y = 268, .rotation = -90, .fill = "#111111", .stroke = "#555555", .strokeWidth = 1, .angle = 270, .radius = 44}}));
    add_caption(layer, 382, 325, "Wedge 270\xC2\xB0");

    layer.add(
        Rect::create({{.x = 500, .y = 224, .width = 150, .height = 88, .fill = "#f7f7f7", .stroke = "#111111", .strokeWidth = 1.5}}),
        Text::create({{.x = 514, .y = 240, .fill = "#111111", .text = "KONVA 2D\nMonochrome\nScene Graph", .fontFamily = "monospace", .fontSize = 13}})
    );
    add_caption(layer, 558, 325, "Text");

    layer.draw();

    dom::get_document().getElementById("meta")->textContent(
        "Stage: 680x400 \xC2\xB7 7 primitive types \xC2\xB7 " +
        std::to_string(layer.getChildren().size()) + " layer children"
    );
}
