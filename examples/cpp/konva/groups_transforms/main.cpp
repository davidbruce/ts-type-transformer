#include <cmath>
#include <string>

#include "dom/dom.hpp"
#include "konva/konva.hpp"

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

    for (int x = 40; x <= 640; x += 40) {
        konva::LineConfig lc = konva::LineConfig::create();
        lc.points(konva::LinePoints(ts::Array<double>{ static_cast<double>(x), 0.0, static_cast<double>(x), 400.0 }));
        lc.stroke(konva::string_or_CanvasGradient(std::string("#e5e5e5")));
        lc.strokeWidth(1);
        layer.add(ts::Array<konva::Group_or_Shape>{ konva::Line<konva::LineConfig>::create(lc) });
    }
    for (int y = 40; y <= 360; y += 40) {
        konva::LineConfig lc = konva::LineConfig::create();
        lc.points(konva::LinePoints(ts::Array<double>{ 0.0, static_cast<double>(y), 680.0, static_cast<double>(y) }));
        lc.stroke(konva::string_or_CanvasGradient(std::string("#e5e5e5")));
        lc.strokeWidth(1);
        layer.add(ts::Array<konva::Group_or_Shape>{ konva::Line<konva::LineConfig>::create(lc) });
    }

    // Group A: unrotated base frame.
    konva::GroupConfig gac = konva::GroupConfig::create();
    gac.x(40);
    gac.y(55);
    konva::Group group_a = konva::Group::create(gac);

    konva::RectConfig gab = konva::RectConfig::create();
    gab.width(160);
    gab.height(120);
    gab.fill(konva::string_or_CanvasGradient(std::string("#f7f7f7")));
    gab.stroke(konva::string_or_CanvasGradient(std::string("#111111")));
    gab.strokeWidth(1.5);
    konva::CircleConfig gad = konva::CircleConfig::create();
    gad.x(80);
    gad.y(60);
    gad.radius(24);
    gad.fill(konva::string_or_CanvasGradient(std::string("#111111")));
    konva::TextConfig gal = konva::TextConfig::create();
    gal.x(12);
    gal.y(10);
    gal.text("Group A \xC2\xB7 0\xC2\xB0");
    gal.fontSize(11);
    gal.fontFamily("monospace");
    gal.fill(konva::string_or_CanvasGradient(std::string("#111111")));
    group_a.add(ts::Array<konva::Group_or_Shape>{
        konva::Rect::create(gab), konva::Circle::create(gad), konva::Text::create(gal)
    });
    layer.add(ts::Array<konva::Group_or_Shape>{ group_a });

    // Group B: rotated 24 degrees.
    konva::GroupConfig gbc = konva::GroupConfig::create();
    gbc.x(340);
    gbc.y(75);
    gbc.rotation(24);
    konva::Group group_b = konva::Group::create(gbc);

    konva::RectConfig gbo = konva::RectConfig::create();
    gbo.width(150);
    gbo.height(110);
    gbo.fill(konva::string_or_CanvasGradient(std::string("#efefef")));
    gbo.stroke(konva::string_or_CanvasGradient(std::string("#111111")));
    gbo.strokeWidth(2);
    konva::RectConfig gbi = konva::RectConfig::create();
    gbi.x(22);
    gbi.y(22);
    gbi.width(106);
    gbi.height(66);
    gbi.fill(konva::string_or_CanvasGradient(std::string("#ffffff")));
    gbi.stroke(konva::string_or_CanvasGradient(std::string("#555555")));
    gbi.strokeWidth(1.5);
    gbi.dash(ts::Array<double>{ 4.0, 3.0 });
    konva::CircleConfig gbd = konva::CircleConfig::create();
    gbd.x(75);
    gbd.y(55);
    gbd.radius(14);
    gbd.fill(konva::string_or_CanvasGradient(std::string("#404040")));
    konva::TextConfig gbl = konva::TextConfig::create();
    gbl.x(10);
    gbl.y(8);
    gbl.text("Group B \xC2\xB7 24\xC2\xB0");
    gbl.fontSize(11);
    gbl.fontFamily("monospace");
    gbl.fill(konva::string_or_CanvasGradient(std::string("#111111")));
    group_b.add(ts::Array<konva::Group_or_Shape>{
        konva::Rect::create(gbo), konva::Rect::create(gbi), konva::Circle::create(gbd), konva::Text::create(gbl)
    });
    layer.add(ts::Array<konva::Group_or_Shape>{ group_b });

    // Group C: scaled + nested rotated child group.
    konva::GroupConfig gcc = konva::GroupConfig::create();
    gcc.x(495);
    gcc.y(55);
    gcc.scaleX(1.15);
    gcc.scaleY(1.15);
    konva::Group group_c = konva::Group::create(gcc);

    konva::RectConfig gco = konva::RectConfig::create();
    gco.width(130);
    gco.height(110);
    gco.fill(konva::string_or_CanvasGradient(std::string("#ffffff")));
    gco.stroke(konva::string_or_CanvasGradient(std::string("#111111")));
    gco.strokeWidth(2);
    konva::TextConfig gcl = konva::TextConfig::create();
    gcl.x(8);
    gcl.y(8);
    gcl.text("Group C \xC2\xB7 1.15x");
    gcl.fontSize(10);
    gcl.fontFamily("monospace");
    gcl.fill(konva::string_or_CanvasGradient(std::string("#111111")));

    konva::GroupConfig icc = konva::GroupConfig::create();
    icc.x(25);
    icc.y(25);
    icc.rotation(15);
    konva::Group inner_group = konva::Group::create(icc);
    konva::CircleConfig mc = konva::CircleConfig::create();
    mc.x(40);
    mc.y(30);
    mc.radius(16);
    mc.fill(konva::string_or_CanvasGradient(std::string("#111111")));
    konva::Circle marker = konva::Circle::create(mc);
    inner_group.add(ts::Array<konva::Group_or_Shape>{ marker });
    group_c.add(ts::Array<konva::Group_or_Shape>{ konva::Rect::create(gco), konva::Text::create(gcl), inner_group });
    layer.add(ts::Array<konva::Group_or_Shape>{ group_c });

    konva::Node_getAbsolutePosition_result abs = marker.getAbsolutePosition();
    konva::IRect bbox = group_b.getClientRect();
    long ax = std::lround(abs.x());
    long ay = std::lround(abs.y());
    long bw = std::lround(bbox.width());
    long bh = std::lround(bbox.height());
    std::string strip_txt =
        "Marker abs=(" + std::to_string(ax) + ", " + std::to_string(ay) +
        ") \xC2\xB7 Group B bbox=" + std::to_string(bw) + "x" + std::to_string(bh);

    konva::RectConfig strip = konva::RectConfig::create();
    strip.x(28);
    strip.y(308);
    strip.width(624);
    strip.height(60);
    strip.fill(konva::string_or_CanvasGradient(std::string("#f7f7f7")));
    strip.stroke(konva::string_or_CanvasGradient(std::string("#111111")));
    strip.strokeWidth(1.5);
    konva::TextConfig sum = konva::TextConfig::create();
    sum.x(44);
    sum.y(331);
    sum.text(strip_txt);
    sum.fontSize(12);
    sum.fontFamily("monospace");
    sum.fill(konva::string_or_CanvasGradient(std::string("#111111")));
    layer.add(ts::Array<konva::Group_or_Shape>{ konva::Rect::create(strip), konva::Text::create(sum) });

    layer.draw();

    dom::Document doc = dom::get_document();
    dom::HTMLElement meta = *doc.getElementById("meta");
    meta.textContent(
        "Stage: 680x400 \xC2\xB7 Nested marker abs=(" + std::to_string(ax) + ", " + std::to_string(ay) +
        ") \xC2\xB7 Rotated Group B rect=" + std::to_string(bw) + "x" + std::to_string(bh)
    );
}
