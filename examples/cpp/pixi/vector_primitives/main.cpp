#include <string>

#include "dom/dom.hpp"
#include "pixi_js/pixi_js.hpp"

using namespace pixi_js;

static ts::Value val_of(const ts::Handle& h) {
    ts::Value v;
    static_cast<ts::Handle&>(v) = h;
    return v;
}

static void stroke_g(const Graphics& g, double width, double color_hex) {
    StrokeStyle ss = StrokeStyle::create();
    ss.width(width);
    ss.color(ColorSource(color_hex));
    g.stroke(ts::Array<>{val_of(ss)});
}

static ts::Task run_app() {
    Application<> app = Application<>::create();
    Application_init_options opts = Application_init_options::create();
    opts.width(680.0);
    opts.height(400.0);
    opts.backgroundColor(ColorSource(double(0xffffff)));
    opts.preference(string_or_RendererPreference_array(std::string("webgl")));
    opts.preserveDrawingBuffer(true);
    opts.antialias(true);
    opts.autoStart(false);

    ts::Result<void> res = co_await app.init(opts);
    if (!res) co_return;

    dom::Document doc = dom::get_document();
    if (auto stage_el = doc.getElementById("stage")) {
        dom::Node canvas_node;
        static_cast<ts::Handle&>(canvas_node) = app.canvas();
        stage_el->appendChild(canvas_node);
    }

    Container<ts::Value> stage = app.stage();

    Graphics grid = Graphics::create();
    for (int x = 40; x <= 640; x += 40) {
        grid.moveTo(double(x), 0.0).lineTo(double(x), 400.0);
    }
    for (int y = 40; y <= 360; y += 40) {
        grid.moveTo(0.0, double(y)).lineTo(680.0, double(y));
    }
    stroke_g(grid, 1.0, double(0xe5e5e5));
    stage.addChild(ts::Array<>{val_of(grid)});

    Graphics rect_g = Graphics::create();
    rect_g.rect(40.0, 46.0, 140.0, 88.0).fill(ColorSource(double(0x111111)));
    stage.addChild(ts::Array<>{val_of(rect_g)});

    Graphics round_g = Graphics::create();
    round_g.roundRect(216.0, 46.0, 156.0, 88.0, 16.0).fill(ColorSource(double(0xf7f7f7)));
    stroke_g(round_g, 2.0, double(0x111111));
    stage.addChild(ts::Array<>{val_of(round_g)});

    Graphics circle_g = Graphics::create();
    circle_g.circle(458.0, 90.0, 44.0).fill(ColorSource(double(0x404040)));
    circle_g.circle(458.0, 90.0, 22.0);
    stroke_g(circle_g, 2.0, double(0xffffff));
    circle_g.circle(458.0, 90.0, 52.0);
    stroke_g(circle_g, 1.5, double(0x111111));
    stage.addChild(ts::Array<>{val_of(circle_g)});

    Graphics ellipse_g = Graphics::create();
    ellipse_g.ellipse(588.0, 90.0, 56.0, 36.0).fill(ColorSource(double(0xd4d4d4)));
    stroke_g(ellipse_g, 1.5, double(0x111111));
    stage.addChild(ts::Array<>{val_of(ellipse_g)});

    Graphics poly_g = Graphics::create();
    poly_g.regularPoly(130.0, 274.0, 58.0, 6.0, 0.25).fill(ColorSource(double(0x737373)));
    stroke_g(poly_g, 2.0, double(0x111111));
    stage.addChild(ts::Array<>{val_of(poly_g)});

    Graphics star_g = Graphics::create();
    star_g.star(340.0, 274.0, 5.0, 62.0, 26.0, -0.314).fill(ColorSource(double(0x111111)));
    stage.addChild(ts::Array<>{val_of(star_g)});

    Graphics diamond_g = Graphics::create();
    diamond_g.moveTo(548.0, 212.0).lineTo(618.0, 274.0).lineTo(548.0, 336.0).lineTo(478.0, 274.0).closePath();
    diamond_g.fill(ColorSource(double(0xb5b5b5)));
    stroke_g(diamond_g, 1.5, double(0x111111));
    stage.addChild(ts::Array<>{val_of(diamond_g)});

    app.render();

    if (auto meta = doc.getElementById("meta")) {
        meta->textContent("7 vector primitives · rect · roundRect · circle · ellipse · regularPoly · star · path · 680x400 WebGL");
    }
}

extern "C" __attribute__((export_name("start"))) void start() {
    run_app();
}
