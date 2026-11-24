#include <array>
#include <memory>
#include <string>

#include "dom/dom.hpp"
#include "konva/konva.hpp"

using namespace konva;

static Line make_line(double x1, double y1, double x2, double y2, const char* color, double width) {
    return Line::create({{.stroke = color, .strokeWidth = width, .points = ts::Array<double>::of(x1, y1, x2, y2)}});
}

static Rect make_card(const char* id, double x, double y, const char* fill) {
    return Rect::create({{.x = x, .y = y, .width = 200, .height = 130, .id = id, .name = "card", .fill = fill, .stroke = "#111111", .strokeWidth = 2}});
}

static Text make_label(double x, double y, const char* txt, const char* fill) {
    return Text::create({{.x = x, .y = y, .fill = fill, .text = txt, .fontFamily = "monospace", .fontSize = 12}});
}

TS_EXPORT(start) void start() {
    Stage stage = Stage::create({{.width = 680, .height = 400, .container = "stage"}});

    Layer bg_layer = Layer::create();
    Layer fg_layer = Layer::create();
    stage.add(bg_layer);
    stage.add(fg_layer);

    // Layer 1: Blueprint Grid.
    bg_layer.add(Rect::create({{.x = 0, .y = 0, .width = 680, .height = 400, .fill = "#ffffff"}}));

    for (int x = 40; x <= 640; x += 40) {
        bg_layer.add(make_line(x, 20, x, 380, "#e5e5e5", 1));
    }
    for (int y = 40; y <= 360; y += 40) {
        bg_layer.add(make_line(20, y, 660, y, "#e5e5e5", 1));
    }

    RectConfig frame({.x = 20, .y = 20, .width = 640, .height = 360, .stroke = "#b5b5b5", .strokeWidth = 1.5, .dash = ts::Array<double>::of(6.0, 4.0)});
    bg_layer.add(Rect(frame), make_label(32, 32, "Layer 1: Blueprint Grid (z=0)", "#737373"));

    // Layer 2: Stacked cards & badge.
    Rect card_a = make_card("card-a", 110, 80, "#efefef");
    Rect card_b = make_card("card-b", 210, 125, "#404040");
    Rect card_c = make_card("card-c", 310, 170, "#ffffff");
    // Non-listening crosshair & label overlay kept above raised cards.
    Group overlay = Group::create({{.listening = false}});

    overlay.add(
        Circle::create({{.x = 250, .y = 160, .fill = "#ffffff", .stroke = "#111111", .strokeWidth = 2, .radius = 7}}),
        make_line(238, 160, 262, 160, "#111111", 1.5),
        make_line(250, 148, 250, 172, "#111111", 1.5),
        make_label(32, 354, "Layer 2: Elevated Stack (z=1) \xC2\xB7 Probe (250,160)", "#111111")
    );

    fg_layer.add(card_a, card_b, card_c, Circle::create({{.x = 555, .y = 135, .id = "badge", .fill = "#111111", .radius = 38}}), overlay);

    card_a.moveToTop();
    overlay.moveToTop();
    bg_layer.draw();
    fg_layer.draw();

    struct ClickState {
        Stage stage;
        Layer fg_layer;
        Group overlay;
        std::array<Rect, 3> cards;
        std::array<const char*, 3> ids;
        int clicks = 0;
    };
    auto st = std::make_shared<ClickState>(ClickState{
        stage, fg_layer, overlay, { card_a, card_b, card_c }, { "card-a", "card-b", "card-c" }, 0
    });

    for (size_t i = 0; i < st->cards.size(); i++) {
        KonvaEventListener<Node, PointerEvent> on_click(
            [st, i](KonvaEventObject<PointerEvent>) {
                st->cards[i].moveToTop();
                st->overlay.moveToTop();
                st->fg_layer.draw();
                st->clicks++;
                int z = static_cast<int>(st->cards[i].getZIndex());

                auto hit = st->stage.getIntersection(Vector2d({.x = 250, .y = 160}));
                std::string hit_id = hit ? hit->id()() : std::string("none");

                if (auto meta = dom::get_document().getElementById("meta")) {
                    meta->textContent(
                        "Layers: " + std::to_string(st->stage.getLayers().size()) +
                        " \xC2\xB7 top=" + st->ids[i] + " (z=" + std::to_string(z) + ")" +
                        " \xC2\xB7 hit@(250,160)=" + hit_id +
                        " \xC2\xB7 clicks=" + std::to_string(st->clicks)
                    );
                }
            }
        );
        st->cards[i].on_click(on_click);
    }

    card_a.fire("click");
    card_a.fire("click");
}
