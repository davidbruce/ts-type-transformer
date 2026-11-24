#include <cmath>
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

    static constexpr double rings[4] = { 42.0, 82.0, 122.0, 162.0 };
    static constexpr int ring_colors[4] = { 0x111111, 0x555555, 0xa3a3a3, 0xe5e5e5 };

    Graphics guides = Graphics::create();
    for (double r : rings) {
        guides.circle(340.0, 200.0, r);
        stroke_g(guides, 1.0, 0xe5e5e5);
    }

    pixi_js::Point unit_x = pixi_js::Point(1.0, 0.0);
    for (int step = 0; step < 12; step++) {
        double theta = double(step) * (2.0 * M_PI / 12.0);
        Matrix spoke = Matrix::create().rotate(theta);
        pixi_js::Point dir = spoke.apply(unit_x);
        guides.moveTo(340.0, 200.0).lineTo(340.0 + dir.x() * 182.0, 200.0 + dir.y() * 182.0);
    }
    stroke_g(guides, 1.0, 0xd4d4d4);

    Graphics tiles = Graphics::create();
    for (int r_idx = 0; r_idx < 4; r_idx++) {
        double radius = rings[r_idx];
        double s = 8.0 + double(r_idx) * 2.5;
        double cx[4] = { 0.0, s * 0.72, 0.0, -s * 0.72 };
        double cy[4] = { -s, 0.0, s, 0.0 };

        for (int step = 0; step < 12; step++) {
            double theta = double(step) * (2.0 * M_PI / 12.0) + double(r_idx) * 0.13;
            pixi_js::Point dir = Matrix::create().rotate(theta).apply(unit_x);
            Matrix tile_m = Matrix::create()
                .rotate(theta + double(r_idx) * 0.2)
                .translate(340.0 + radius * dir.x(), 200.0 + radius * dir.y());

            pixi_js::Point p0 = tile_m.apply(pixi_js::Point(cx[0], cy[0]));
            tiles.moveTo(p0.x(), p0.y());
            for (int k = 1; k < 4; k++) {
                pixi_js::Point pk = tile_m.apply(pixi_js::Point(cx[k], cy[k]));
                tiles.lineTo(pk.x(), pk.y());
            }
            tiles.closePath().fill(ring_colors[r_idx]);
            stroke_g(tiles, 1.2, 0x111111);
        }
    }

    Graphics center_dot = Graphics::create();
    center_dot.circle(340.0, 200.0, 8.0).fill(0x111111);

    stage.addChild(guides, tiles, center_dot);
    app.render();

    if (auto meta = doc.getElementById("meta")) {
        meta->textContent("48 matrix-transformed diamond tiles · 4 rings · 12 radial spokes · 680x400 WebGL");
    }
}

TS_EXPORT(start) void start() {
    run_app();
}
