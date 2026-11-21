#include <cstdio>
#include <string>

#include "dom/dom.hpp"
#include "pixi_js/pixi_js.hpp"

using namespace pixi_js;

static void stroke_g(const Graphics& g, double width, double color_hex) {
    StrokeStyle ss = StrokeStyle::create();
    ss.width(width);
    ss.color(color_hex);
    g.stroke(ts::Array<>::of(ss));
}

static ts::Task run_app() {
    Application<> app = Application<>::create();
    auto opts = Application_init_options::create()
        .width(680)
        .height(400)
        .backgroundColor(0xffffff)
        .preference("webgl")
        .preserveDrawingBuffer(true)
        .antialias(true)
        .autoStart(false);

    if (auto res = co_await app.init(opts); !res) co_return;

    dom::Document doc = dom::get_document();
    if (auto stage_el = doc.getElementById("stage")) {
        stage_el->appendChild(app.canvas().unsafe_as<dom::Node>());
    }

    Container<ts::Value> stage = app.stage();

    Rectangle query = Rectangle::create(190, 105, 260, 170);
    Rectangle padded = query.clone().pad(10, 10);
    Rectangle hull = query.clone();

    Rectangle cands[8] = {
        Rectangle::create(210, 125, 90, 60),
        Rectangle::create(330, 165, 95, 80),
        Rectangle::create(110, 80, 130, 85),
        Rectangle::create(390, 75, 130, 90),
        Rectangle::create(135, 225, 110, 90),
        Rectangle::create(40, 40, 100, 110),
        Rectangle::create(510, 48, 125, 115),
        Rectangle::create(505, 245, 130, 110),
    };

    int contained = 0;
    int intersecting = 0;
    int disjoint = 0;

    Graphics g = Graphics::create();
    for (const Rectangle& c : cands) {
        hull.enlarge(c);
        g.rect(c.x(), c.y(), c.width(), c.height());
        if (query.containsRect(c)) {
            contained++;
            g.fill(0x111111);
        } else if (query.intersects(c)) {
            intersecting++;
            g.fill(0x737373);
            stroke_g(g, 2.0, 0x111111);
        } else {
            disjoint++;
            g.fill(0xe5e5e5);
            stroke_g(g, 1.0, 0xa3a3a3);
        }
    }

    g.rect(hull.x(), hull.y(), hull.width(), hull.height());
    stroke_g(g, 1.5, 0xa3a3a3);
    g.rect(padded.x(), padded.y(), padded.width(), padded.height());
    stroke_g(g, 1.0, 0x737373);
    g.rect(query.x(), query.y(), query.width(), query.height());
    stroke_g(g, 2.5, 0x111111);

    stage.addChild(g);
    app.render();

    char buf[160];
    std::snprintf(
        buf, sizeof(buf),
        "query 260x170 · %d contained · %d intersecting · %d disjoint · enlarged hull %.0fx%.0f",
        contained, intersecting, disjoint, hull.width(), hull.height()
    );
    if (auto meta = doc.getElementById("meta")) {
        meta->textContent(buf);
    }
}

TS_EXPORT(start) void start() {
    run_app();
}
