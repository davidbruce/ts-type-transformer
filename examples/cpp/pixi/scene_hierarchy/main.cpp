#include <cmath>
#include <cstdio>
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

    Graphics guide = Graphics::create();
    guide.moveTo(40.0, 200.0).lineTo(640.0, 200.0).moveTo(340.0, 24.0).lineTo(340.0, 376.0);
    stroke_g(guide, 1.0, double(0xe5e5e5));
    guide.circle(340.0, 200.0, 110.0);
    stroke_g(guide, 1.0, double(0xd4d4d4));
    stage.addChild(ts::Array<>{val_of(guide)});

    Container<ts::Value> root = Container<ts::Value>::create();
    root.position().set(340.0, 200.0);

    for (int i = 0; i < 3; i++) {
        Container<ts::Value> arm = Container<ts::Value>::create();
        arm.rotation(double(i) * 2.0 * M_PI / 3.0);

        Graphics arm_bar = Graphics::create();
        arm_bar.rect(0.0, -4.0, 110.0, 8.0).fill(ColorSource(double(0x737373)));
        arm_bar.circle(110.0, 0.0, 10.0).fill(ColorSource(double(0x111111)));
        arm.addChild(ts::Array<>{val_of(arm_bar)});

        Container<ts::Value> sub = Container<ts::Value>::create();
        sub.x(110.0);
        sub.rotation(-0.45);

        Graphics sub_bar = Graphics::create();
        sub_bar.moveTo(-32.0, 0.0).lineTo(48.0, 0.0);
        stroke_g(sub_bar, 2.0, double(0x404040));
        sub.addChild(ts::Array<>{val_of(sub_bar)});

        Graphics sat_box = Graphics::create();
        sat_box.rect(-12.0, -12.0, 24.0, 24.0).fill(ColorSource(double(0x262626)));
        sat_box.x(48.0);
        sub.addChild(ts::Array<>{val_of(sat_box)});

        Graphics sat_orb = Graphics::create();
        sat_orb.circle(0.0, 0.0, 14.0).fill(ColorSource(double(0xd4d4d4)));
        stroke_g(sat_orb, 2.0, double(0x111111));
        sat_orb.x(-32.0);
        sub.addChild(ts::Array<>{val_of(sat_orb)});

        arm.addChild(ts::Array<>{val_of(sub)});
        root.addChild(ts::Array<>{val_of(arm)});
    }

    Graphics hub = Graphics::create();
    hub.circle(0.0, 0.0, 18.0).fill(ColorSource(double(0x111111)));
    hub.circle(0.0, 0.0, 7.0).fill(ColorSource(double(0xffffff)));
    root.addChild(ts::Array<>{val_of(hub)});

    stage.addChild(ts::Array<>{val_of(root)});
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

extern "C" __attribute__((export_name("start"))) void start() {
    run_app();
}
