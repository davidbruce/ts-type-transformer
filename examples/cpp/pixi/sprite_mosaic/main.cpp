#include <string>

#include "dom/dom.hpp"
#include "pixi_js/pixi_js.hpp"

using namespace pixi_js;

static ts::Value val_of(const ts::Handle& h) {
    ts::Value v;
    static_cast<ts::Handle&>(v) = h;
    return v;
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

    Graphics frame = Graphics::create();
    frame.rect(32.0, 20.0, 616.0, 360.0);
    StrokeStyle ss = StrokeStyle::create();
    ss.width(1.5);
    ss.color(ColorSource(double(0xd4d4d4)));
    frame.stroke(ts::Array<>{val_of(ss)});
    stage.addChild(ts::Array<>{val_of(frame)});

    static constexpr int shades[8] = {
        0x111111, 0x262626, 0x404040, 0x555555,
        0x737373, 0xa3a3a3, 0xd4d4d4, 0xe5e5e5
    };
    Texture<TextureSource<ts::Value>> white = Texture<BufferImageSource>::WHITE();

    for (int row = 0; row < 6; row++) {
        for (int col = 0; col < 10; col++) {
            Sprite sprite = Sprite::from(Sprite_from_source(white));
            int idx = (row * 3 + col * 5) % 8;
            sprite.width(52.0);
            sprite.height(48.0);
            sprite.anchor().set(0.5, 0.5);
            sprite.position().set(70.0 + double(col) * 60.0, 56.0 + double(row) * 56.0);
            sprite.rotation((double(col) - 4.5) * 0.04 + (double(row) - 2.5) * 0.03);
            sprite.alpha(0.65 + 0.35 * double((col + row) % 3) / 2.0);
            sprite.tint(ColorSource(double(shades[idx])));
            stage.addChild(ts::Array<>{val_of(sprite)});
        }
    }

    app.render();

    if (auto meta = doc.getElementById("meta")) {
        meta->textContent("60 Texture.WHITE sprites · 10x6 mosaic · 8 monochrome tints · 680x400 WebGL");
    }
}

extern "C" __attribute__((export_name("start"))) void start() {
    run_app();
}
