#include <cmath>
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
        stage_el->appendChild(app.canvas().as<dom::Node>());
    }

    Container<ts::Value> stage = app.stage();

    Graphics guide = Graphics::create();
    guide.moveTo(40.0, 200.0).lineTo(640.0, 200.0).moveTo(340.0, 24.0).lineTo(340.0, 376.0);
    stroke_g(guide, 1.0, 0xe5e5e5);
    guide.circle(340.0, 200.0, 110.0);
    stroke_g(guide, 1.0, 0xd4d4d4);

    Container<ts::Value> root = Container<ts::Value>::create();
    root.position().set(340.0, 200.0);

    for (int i = 0; i < 3; i++) {
        Container<ts::Value> arm = Container<ts::Value>::create();
        arm.rotation(double(i) * 2.0 * M_PI / 3.0);

        Graphics arm_bar = Graphics::create();
        arm_bar.rect(0.0, -4.0, 110.0, 8.0).fill(0x737373);
        arm_bar.circle(110.0, 0.0, 10.0).fill(0x111111);

        Container<ts::Value> sub = Container<ts::Value>::create();
        sub.x(110.0);
        sub.rotation(-0.45);

        Graphics sub_bar = Graphics::create();
        sub_bar.moveTo(-32.0, 0.0).lineTo(48.0, 0.0);
        stroke_g(sub_bar, 2.0, 0x404040);

        Graphics sat_box = Graphics::create();
        sat_box.rect(-12.0, -12.0, 24.0, 24.0).fill(0x262626);
        sat_box.x(48.0);

        Graphics sat_orb = Graphics::create();
        sat_orb.circle(0.0, 0.0, 14.0).fill(0xd4d4d4);
        stroke_g(sat_orb, 2.0, 0x111111);
        sat_orb.x(-32.0);

        sub.addChild(ts::Array<>::of(sub_bar, sat_box, sat_orb));
        arm.addChild(ts::Array<>::of(arm_bar, sub));
        root.addChild(ts::Array<>::of(arm));
    }

    Graphics hub = Graphics::create();
    hub.circle(0.0, 0.0, 18.0).fill(0x111111);
    hub.circle(0.0, 0.0, 7.0).fill(0xffffff);
    root.addChild(ts::Array<>::of(hub));

    stage.addChild(ts::Array<>::of(guide, root));
    app.render();

    Bounds rb = root.getBounds();
    char buf[160];
    std::snprintf(
        buf, sizeof(buf),
        "3 primary arms · 6 nested satellites · bounds %.1fx%.1f · 680x400 WebGL",
        rb.width(), rb.height()
    );
    if (auto meta = doc.getElementById("meta")) {
        meta->textContent(buf);
    }
}

TS_EXPORT(start) void start() {
    run_app();
}
