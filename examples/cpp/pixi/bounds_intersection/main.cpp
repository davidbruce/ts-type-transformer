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

static Rectangle mk_rect(double x, double y, double w, double h) {
    return Rectangle::create(
        number_or_string(x), number_or_string(y),
        number_or_string(w), number_or_string(h)
    );
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

    Rectangle query = mk_rect(190.0, 105.0, 260.0, 170.0);
    Rectangle padded = query.clone().pad(10.0, 10.0);
    Rectangle hull = query.clone();

    Rectangle cands[8] = {
        mk_rect(210.0, 125.0, 90.0, 60.0),
        mk_rect(330.0, 165.0, 95.0, 80.0),
        mk_rect(110.0, 80.0, 130.0, 85.0),
        mk_rect(390.0, 75.0, 130.0, 90.0),
        mk_rect(135.0, 225.0, 110.0, 90.0),
        mk_rect(40.0, 40.0, 100.0, 110.0),
        mk_rect(510.0, 48.0, 125.0, 115.0),
        mk_rect(505.0, 245.0, 130.0, 110.0),
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
            g.fill(ColorSource(double(0x111111)));
        } else if (query.intersects(c)) {
            intersecting++;
            g.fill(ColorSource(double(0x737373)));
            stroke_g(g, 2.0, double(0x111111));
        } else {
            disjoint++;
            g.fill(ColorSource(double(0xe5e5e5)));
            stroke_g(g, 1.0, double(0xa3a3a3));
        }
    }

    g.rect(hull.x(), hull.y(), hull.width(), hull.height());
    stroke_g(g, 1.5, double(0xa3a3a3));
    g.rect(padded.x(), padded.y(), padded.width(), padded.height());
    stroke_g(g, 1.0, double(0x737373));
    g.rect(query.x(), query.y(), query.width(), query.height());
    stroke_g(g, 2.5, double(0x111111));

    stage.addChild(ts::Array<>{val_of(g)});
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

extern "C" __attribute__((export_name("start"))) void start() {
    run_app();
}
