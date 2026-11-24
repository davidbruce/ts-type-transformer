#include <string>

#include "dom/dom.hpp"
#include "konva/konva.hpp"

using namespace konva;

static void add_stage_box(const Layer& layer, double x, const char* title, const char* sub, const char* fill, const char* ink) {
    Group g = Group::create({{.x = x, .y = 95}});
    g.add(
        Rect::create({{.width = 126, .height = 68, .fill = fill, .stroke = "#111111", .strokeWidth = 2, .cornerRadius = 3.0}}),
        Text::create({{.x = 12, .y = 16, .fill = ink, .text = title, .fontFamily = "monospace", .fontSize = 12, .fontStyle = "bold"}}),
        Text::create({{.x = 12, .y = 38, .fill = ink, .text = sub, .fontFamily = "monospace", .fontSize = 11}})
    );
    layer.add(g);
}

static void add_arrow(const Layer& layer, double x1, double y1, double x2, double y2) {
    layer.add(Arrow::create({{.fill = "#111111", .stroke = "#111111", .strokeWidth = 2, .points = ts::Array<double>::of(x1, y1, x2, y2), .pointerLength = 8, .pointerWidth = 8}}));
}

TS_EXPORT(start) void start() {
    Stage stage = Stage::create({{.width = 680, .height = 400, .container = "stage"}});

    Layer layer = Layer::create();
    stage.add(layer);

    layer.add(Rect::create({{.x = 0, .y = 0, .width = 680, .height = 400, .fill = "#ffffff"}}));

    add_stage_box(layer, 28, "01 PARSE", ".d.ts AST", "#ffffff", "#111111");
    add_stage_box(layer, 194, "02 LOWER", "Typed IR", "#f7f7f7", "#111111");
    add_stage_box(layer, 360, "03 EMIT", "6 Targets", "#efefef", "#111111");
    add_stage_box(layer, 526, "04 RUN", "WASM / JS", "#111111", "#ffffff");

    add_arrow(layer, 154, 129, 190, 129);
    add_arrow(layer, 320, 129, 356, 129);
    add_arrow(layer, 486, 129, 522, 129);
    add_arrow(layer, 257, 163, 257, 251);
    add_arrow(layer, 423, 163, 423, 251);

    layer.add(
        Line::create({{.stroke = "#737373", .strokeWidth = 1.5, .dash = ts::Array<double>::of(6.0, 4.0), .points = ts::Array<double>::of(589, 163, 589, 212, 91, 212, 91, 168)}}),
        Rect::create({{.x = 28, .y = 255, .width = 624, .height = 58, .fill = "#f7f7f7", .stroke = "#111111", .strokeWidth = 1.5, .cornerRadius = 3.0}}),
        Text::create({{.x = 180, .y = 278, .fill = "#111111", .text = "Shared DOM & Package Handle Runtime", .fontFamily = "monospace", .fontSize = 12}})
    );
    layer.draw();

    size_t rects = layer.find(ts::Value::string("Rect")).size();
    size_t arrows = layer.find(ts::Value::string("Arrow")).size();
    size_t json_len = layer.toJSON().size();

    dom::get_document().getElementById("meta")->textContent(
        "Pipeline diagram \xC2\xB7 " + std::to_string(rects) + " rects \xC2\xB7 " +
        std::to_string(arrows) + " arrows \xC2\xB7 JSON " + std::to_string(json_len) + " bytes"
    );
}
