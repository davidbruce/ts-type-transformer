#include <string>

#include "dom/dom.hpp"
#include "pixi_js/pixi_js.hpp"

using namespace pixi_js;

static ts::Task run_app() {
    Application app = Application::create();
    if (auto res = co_await app.init({{.backgroundColor = 0xffffff, .preserveDrawingBuffer = true, .preference = "webgl", .width = 680, .height = 400, .antialias = true, .autoStart = false}}); !res) co_return;

    dom::Document doc = dom::get_document();
    if (auto stage_el = doc.getElementById("stage")) {
        stage_el->appendChild(app.canvas().unsafe_as<dom::Node>());
    }

    Container stage = app.stage();

    Graphics frame = Graphics::create();
    frame.rect(32.0, 20.0, 616.0, 360.0);
    frame.stroke(StrokeStyle({.color = 0xd4d4d4, .width = 1.5}));
    stage.addChild(frame);

    static constexpr int shades[8] = {
        0x111111, 0x262626, 0x404040, 0x555555,
        0x737373, 0xa3a3a3, 0xd4d4d4, 0xe5e5e5
    };
    Texture white = Texture::WHITE();

    for (int row = 0; row < 6; row++) {
        for (int col = 0; col < 10; col++) {
            Sprite sprite = Sprite::from(white);
            int idx = (row * 3 + col * 5) % 8;
            sprite.width(52.0);
            sprite.height(48.0);
            sprite.anchor().set(0.5, 0.5);
            sprite.position().set(70.0 + double(col) * 60.0, 56.0 + double(row) * 56.0);
            sprite.rotation((double(col) - 4.5) * 0.04 + (double(row) - 2.5) * 0.03);
            sprite.alpha(0.65 + 0.35 * double((col + row) % 3) / 2.0);
            sprite.tint(shades[idx]);
            stage.addChild(sprite);
        }
    }

    app.render();

    if (auto meta = doc.getElementById("meta")) {
        meta->textContent("60 Texture.WHITE sprites · 10x6 mosaic · 8 monochrome tints · 680x400 WebGL");
    }
}

TS_EXPORT(start) void start() {
    run_app();
}
