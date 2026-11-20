#include <string>

#include "dom/dom.hpp"
#include "konva/konva.hpp"

using namespace konva;

static void add_stage_box(const Layer& layer, double x, const char* title, const char* sub, const char* fill, const char* ink) {
    GroupConfig gc = GroupConfig::create();
    gc.x(x);
    gc.y(95);
    Group g = Group::create(gc);

    RectConfig rc = RectConfig::create();
    rc.width(126);
    rc.height(68);
    rc.cornerRadius(3.0);
    rc.fill(fill);
    rc.stroke("#111111");
    rc.strokeWidth(2);

    TextConfig t1 = TextConfig::create();
    t1.x(12);
    t1.y(16);
    t1.text(title);
    t1.fontSize(12);
    t1.fontStyle("bold");
    t1.fontFamily("monospace");
    t1.fill(ink);

    TextConfig t2 = TextConfig::create();
    t2.x(12);
    t2.y(38);
    t2.text(sub);
    t2.fontSize(11);
    t2.fontFamily("monospace");
    t2.fill(ink);

    g.add(Rect::create(rc), Text::create(t1), Text::create(t2));
    layer.add(g);
}

static void add_arrow(const Layer& layer, double x1, double y1, double x2, double y2) {
    ArrowConfig ac = ArrowConfig::create();
    ac.points(ts::Array<double>::of(x1, y1, x2, y2));
    ac.pointerLength(8);
    ac.pointerWidth(8);
    ac.fill("#111111");
    ac.stroke("#111111");
    ac.strokeWidth(2);
    layer.add(Arrow::create(ac));
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

    add_stage_box(layer, 28, "01 PARSE", ".d.ts AST", "#ffffff", "#111111");
    add_stage_box(layer, 194, "02 LOWER", "Typed IR", "#f7f7f7", "#111111");
    add_stage_box(layer, 360, "03 EMIT", "6 Targets", "#efefef", "#111111");
    add_stage_box(layer, 526, "04 RUN", "WASM / JS", "#111111", "#ffffff");

    add_arrow(layer, 154, 129, 190, 129);
    add_arrow(layer, 320, 129, 356, 129);
    add_arrow(layer, 486, 129, 522, 129);
    add_arrow(layer, 257, 163, 257, 251);
    add_arrow(layer, 423, 163, 423, 251);

    LineConfig fbc = LineConfig::create();
    fbc.points(ts::Array<double>::of(589, 163, 589, 212, 91, 212, 91, 168));
    fbc.stroke("#737373");
    fbc.strokeWidth(1.5);
    fbc.dash(ts::Array<double>::of(6.0, 4.0));

    RectConfig bar = RectConfig::create();
    bar.x(28);
    bar.y(255);
    bar.width(624);
    bar.height(58);
    bar.cornerRadius(3.0);
    bar.fill("#f7f7f7");
    bar.stroke("#111111");
    bar.strokeWidth(1.5);

    TextConfig btxt = TextConfig::create();
    btxt.x(180);
    btxt.y(278);
    btxt.text("Shared DOM & Package Handle Runtime");
    btxt.fontSize(12);
    btxt.fontFamily("monospace");
    btxt.fill("#111111");

    layer.add(Line<LineConfig>::create(fbc), Rect::create(bar), Text::create(btxt));
    layer.draw();

    size_t rects = layer.find<Rect>(ts::Value::string("Rect")).size();
    size_t arrows = layer.find<Arrow>(ts::Value::string("Arrow")).size();
    size_t json_len = layer.toJSON().size();

    dom::get_document().getElementById("meta")->textContent(
        "Pipeline diagram \xC2\xB7 " + std::to_string(rects) + " rects \xC2\xB7 " +
        std::to_string(arrows) + " arrows \xC2\xB7 JSON " + std::to_string(json_len) + " bytes"
    );
}
