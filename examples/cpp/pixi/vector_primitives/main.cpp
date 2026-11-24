#include <string>

#include "dom/dom.hpp"
#include "pixi_js/pixi_js.hpp"

using namespace pixi_js;

static void stroke_g(const Graphics& g, double width, double color_hex) {
    g.stroke(StrokeStyle({.color = color_hex, .width = width}));
}

static ts::Task run_app() {
    Application app = Application::create();
    if (auto res = co_await app.init({{.backgroundColor = 0xffffff, .preserveDrawingBuffer = true, .preference = "webgl", .width = 680, .height = 400, .antialias = true, .autoStart = false}}); !res) co_return;

    dom::Document doc = dom::get_document();
    if (auto stage_el = doc.getElementById("stage")) {
        stage_el->appendChild(app.canvas().unsafe_as<dom::Node>());
    }

    Container stage = app.stage();

    Graphics grid = Graphics::create();
    for (int x = 40; x <= 640; x += 40) {
        grid.moveTo(double(x), 0.0).lineTo(double(x), 400.0);
    }
    for (int y = 40; y <= 360; y += 40) {
        grid.moveTo(0.0, double(y)).lineTo(680.0, double(y));
    }
    stroke_g(grid, 1.0, 0xe5e5e5);

    Graphics rect_g = Graphics::create();
    rect_g.rect(40.0, 46.0, 140.0, 88.0).fill(0x111111);

    Graphics round_g = Graphics::create();
    round_g.roundRect(216.0, 46.0, 156.0, 88.0, 16.0).fill(0xf7f7f7);
    stroke_g(round_g, 2.0, 0x111111);

    Graphics circle_g = Graphics::create();
    circle_g.circle(458.0, 90.0, 44.0).fill(0x404040);
    circle_g.circle(458.0, 90.0, 22.0);
    stroke_g(circle_g, 2.0, 0xffffff);
    circle_g.circle(458.0, 90.0, 52.0);
    stroke_g(circle_g, 1.5, 0x111111);

    Graphics ellipse_g = Graphics::create();
    ellipse_g.ellipse(588.0, 90.0, 56.0, 36.0).fill(0xd4d4d4);
    stroke_g(ellipse_g, 1.5, 0x111111);

    Graphics poly_g = Graphics::create();
    poly_g.regularPoly(130.0, 274.0, 58.0, 6.0, 0.25).fill(0x737373);
    stroke_g(poly_g, 2.0, 0x111111);

    Graphics star_g = Graphics::create();
    star_g.star(340.0, 274.0, 5.0, 62.0, 26.0, -0.314).fill(0x111111);

    Graphics diamond_g = Graphics::create();
    diamond_g.moveTo(548.0, 212.0).lineTo(618.0, 274.0).lineTo(548.0, 336.0).lineTo(478.0, 274.0).closePath();
    diamond_g.fill(0xb5b5b5);
    stroke_g(diamond_g, 1.5, 0x111111);

    stage.addChild(grid, rect_g, round_g, circle_g, ellipse_g, poly_g, star_g, diamond_g);
    app.render();

    if (auto meta = doc.getElementById("meta")) {
        meta->textContent("7 vector primitives · rect · roundRect · circle · ellipse · regularPoly · star · path · 680x400 WebGL");
    }
}

TS_EXPORT(start) void start() {
    run_app();
}
