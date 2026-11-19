#include <string>

#include "dom/dom.hpp"
#include "konva/konva.hpp"

static void add_stage_box(const konva::Layer& layer, double x, const std::string& title, const std::string& sub, const std::string& fill, const std::string& ink) {
    konva::GroupConfig gc = konva::GroupConfig::create();
    gc.x(x);
    gc.y(95);
    konva::Group g = konva::Group::create(gc);

    konva::RectConfig rc = konva::RectConfig::create();
    rc.width(126);
    rc.height(68);
    rc.cornerRadius(konva::number_or_number_array(3.0));
    rc.fill(konva::string_or_CanvasGradient(fill));
    rc.stroke(konva::string_or_CanvasGradient(std::string("#111111")));
    rc.strokeWidth(2);

    konva::TextConfig t1 = konva::TextConfig::create();
    t1.x(12);
    t1.y(16);
    t1.text(title);
    t1.fontSize(12);
    t1.fontStyle("bold");
    t1.fontFamily("monospace");
    t1.fill(konva::string_or_CanvasGradient(ink));

    konva::TextConfig t2 = konva::TextConfig::create();
    t2.x(12);
    t2.y(38);
    t2.text(sub);
    t2.fontSize(11);
    t2.fontFamily("monospace");
    t2.fill(konva::string_or_CanvasGradient(ink));

    g.add(ts::Array<konva::Group_or_Shape>{
        konva::Rect::create(rc), konva::Text::create(t1), konva::Text::create(t2)
    });
    layer.add(ts::Array<konva::Group_or_Shape>{ g });
}

static void add_arrow(const konva::Layer& layer, double x1, double y1, double x2, double y2) {
    konva::ArrowConfig ac = konva::ArrowConfig::create();
    ac.points(konva::LinePoints(ts::Array<double>{ x1, y1, x2, y2 }));
    ac.pointerLength(8);
    ac.pointerWidth(8);
    ac.fill(konva::string_or_CanvasGradient(std::string("#111111")));
    ac.stroke(konva::string_or_CanvasGradient(std::string("#111111")));
    ac.strokeWidth(2);
    layer.add(ts::Array<konva::Group_or_Shape>{ konva::Arrow::create(ac) });
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

    add_stage_box(layer, 28, "01 PARSE", ".d.ts AST", "#ffffff", "#111111");
    add_stage_box(layer, 194, "02 LOWER", "Typed IR", "#f7f7f7", "#111111");
    add_stage_box(layer, 360, "03 EMIT", "6 Targets", "#efefef", "#111111");
    add_stage_box(layer, 526, "04 RUN", "WASM / JS", "#111111", "#ffffff");

    add_arrow(layer, 154, 129, 190, 129);
    add_arrow(layer, 320, 129, 356, 129);
    add_arrow(layer, 486, 129, 522, 129);
    add_arrow(layer, 257, 163, 257, 251);
    add_arrow(layer, 423, 163, 423, 251);

    konva::LineConfig fbc = konva::LineConfig::create();
    fbc.points(konva::LinePoints(ts::Array<double>{ 589, 163, 589, 212, 91, 212, 91, 168 }));
    fbc.stroke(konva::string_or_CanvasGradient(std::string("#737373")));
    fbc.strokeWidth(1.5);
    fbc.dash(ts::Array<double>{ 6.0, 4.0 });
    layer.add(ts::Array<konva::Group_or_Shape>{ konva::Line<konva::LineConfig>::create(fbc) });

    konva::RectConfig bar = konva::RectConfig::create();
    bar.x(28);
    bar.y(255);
    bar.width(624);
    bar.height(58);
    bar.cornerRadius(konva::number_or_number_array(3.0));
    bar.fill(konva::string_or_CanvasGradient(std::string("#f7f7f7")));
    bar.stroke(konva::string_or_CanvasGradient(std::string("#111111")));
    bar.strokeWidth(1.5);
    konva::TextConfig btxt = konva::TextConfig::create();
    btxt.x(180);
    btxt.y(278);
    btxt.text("Shared DOM & Package Handle Runtime");
    btxt.fontSize(12);
    btxt.fontFamily("monospace");
    btxt.fill(konva::string_or_CanvasGradient(std::string("#111111")));
    layer.add(ts::Array<konva::Group_or_Shape>{ konva::Rect::create(bar), konva::Text::create(btxt) });

    layer.draw();

    size_t rects = layer.find<konva::Rect>(ts::Value::string("Rect")).size();
    size_t arrows = layer.find<konva::Arrow>(ts::Value::string("Arrow")).size();
    size_t json_len = layer.toJSON().size();

    dom::Document doc = dom::get_document();
    dom::HTMLElement meta = *doc.getElementById("meta");
    meta.textContent(
        "Pipeline diagram \xC2\xB7 " + std::to_string(rects) + " rects \xC2\xB7 " +
        std::to_string(arrows) + " arrows \xC2\xB7 JSON " + std::to_string(json_len) + " bytes"
    );
}
