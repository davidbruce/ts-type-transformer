#include "dom/dom.h"
#include "konva/konva.h"
#include <stdio.h>

static Konva_Stage g_stage = { 0 };
static Konva_Layer g_fg_layer = { 0 };
static Konva_Group g_overlay = { 0 };
static Konva_Rect g_cards[3] = { { 0 }, { 0 }, { 0 } };
static const char* g_card_ids[3] = { "card-a", "card-b", "card-c" };
static const int g_card_indices[3] = { 0, 1, 2 };
static int32_t g_clicks = 0;

static void on_card_click(void* ctx, KonvaEventObject e) {
    (void)e;
    js_scope();
    int idx = *(const int*)ctx;
    Konva_Rect card = g_cards[idx];
    Konva_Rect_moveToTop(card);
    Konva_Group_moveToTop(g_overlay);
    Konva_Layer_draw(g_fg_layer);
    g_clicks++;
    int z = (int)Konva_Rect_getZIndex(card);

    Konva_Shape hit = Konva_Stage_getIntersection(g_stage, Konva_Vector2d_create(.x = 250, .y = 160));
    char hit_buf[64] = "none";
    if (hit.h != 0) {
        JsString_copy(js_to_string(Konva_GetSet_invoke(Konva_Shape_get_id(hit))), hit_buf, sizeof hit_buf);
    }

    int layers = (int)JsArray_Konva_Layer_length(Konva_Stage_getLayers(g_stage));
    char meta[192];
    snprintf(meta, sizeof meta, "Layers: %d \xC2\xB7 top=%s (z=%d) \xC2\xB7 hit@(250,160)=%s \xC2\xB7 clicks=%d",
        layers, g_card_ids[idx], z, hit_buf, (int)g_clicks);
    HTMLElement meta_el = Document_getElementById(get_document(), "meta");
    if (meta_el.h != 0) HTMLElement_set_textContent(meta_el, meta);
}

static Konva_Line make_line(double x1, double y1, double x2, double y2, const char* color, double width) {
    Konva_LineConfig lc = Konva_LineConfig_create(
        .points = js_numbers(x1, y1, x2, y2),
        .stroke = color, .strokeWidth = width
    );
    return Konva_Line_new(lc);
}

static Konva_Rect make_card(const char* id, double x, double y, const char* fill) {
    Konva_RectConfig rc = Konva_RectConfig_create(
        .id = id, .name = "card", .x = x, .y = y, .width = 200, .height = 130,
        .fill = fill, .stroke = "#111111",
        .strokeWidth = 2
    );
    return Konva_Rect_new(rc);
}

__attribute__((export_name("start"))) void start(void) {
    js_scope();

    Konva_StageConfig sc = Konva_StageConfig_create(
        .container = "stage", .width = 680, .height = 400
    );
    Konva_Stage stage = Konva_Stage_new(sc);

    Konva_Layer bg_layer = Konva_Layer_new();
    Konva_Layer fg_layer = Konva_Layer_new();
    Konva_Stage_add(stage, bg_layer);
    Konva_Stage_add(stage, fg_layer);

    // Layer 1: Blueprint Grid.
    JsArray_Konva_Node bg_items = JsArray_Konva_Node_new();
    Konva_RectConfig bgr = Konva_RectConfig_create(
        .x = 0, .y = 0, .width = 680, .height = 400, .fill = "#ffffff"
    );
    JsArray_Konva_Node_push(bg_items, Konva_Rect_new(bgr));

    for (int x = 40; x <= 640; x += 40) {
        JsArray_Konva_Node_push(bg_items, make_line((double)x, 20, (double)x, 380, "#e5e5e5", 1));
    }
    for (int y = 40; y <= 360; y += 40) {
        JsArray_Konva_Node_push(bg_items, make_line(20, (double)y, 660, (double)y, "#e5e5e5", 1));
    }

    Konva_RectConfig frame = Konva_RectConfig_create(
        .x = 20, .y = 20, .width = 640, .height = 360, .stroke = "#b5b5b5",
        .strokeWidth = 1.5, .dash = js_numbers(6.0, 4.0)
    );
    JsArray_Konva_Node_push(bg_items, Konva_Rect_new(frame));

    Konva_TextConfig l1 = Konva_TextConfig_create(
        .x = 32, .y = 32, .text = "Layer 1: Blueprint Grid (z=0)", .fontSize = 12, .fontFamily = "monospace",
        .fill = "#737373"
    );
    JsArray_Konva_Node_push(bg_items, Konva_Text_new(l1));
    Konva_Layer_add_list(bg_layer, bg_items);

    // Layer 2: Stacked cards & badge.
    Konva_Rect card_a = make_card("card-a", 110, 80, "#efefef");
    Konva_Rect card_b = make_card("card-b", 210, 125, "#404040");
    Konva_Rect card_c = make_card("card-c", 310, 170, "#ffffff");
    Konva_CircleConfig bc = Konva_CircleConfig_create(
        .id = "badge", .x = 555, .y = 135, .radius = 38, .fill = "#111111"
    );
    Konva_Circle badge = Konva_Circle_new(bc);

    // Crosshair & foreground label (non-listening overlay group kept above raised cards).
    Konva_Group overlay = Konva_Group_new(Konva_GroupConfig_create(.listening = false));
    Konva_CircleConfig dc = Konva_CircleConfig_create(
        .x = 250, .y = 160, .radius = 7, .fill = "#ffffff",
        .stroke = "#111111", .strokeWidth = 2
    );

    Konva_TextConfig l2 = Konva_TextConfig_create(
        .x = 32, .y = 354, .text = "Layer 2: Elevated Stack (z=1) \xC2\xB7 Probe (250,160)", .fontSize = 12,
        .fontFamily = "monospace", .fill = "#111111"
    );
    Konva_Group_add(overlay,
        Konva_Circle_new(dc), make_line(238, 160, 262, 160, "#111111", 1.5),
        make_line(250, 148, 250, 172, "#111111", 1.5), Konva_Text_new(l2));

    Konva_Layer_add(fg_layer, card_a, card_b, card_c, badge, overlay);

    Konva_Rect_moveToTop(card_a);
    Konva_Group_moveToTop(overlay);
    Konva_Layer_draw(bg_layer);
    Konva_Layer_draw(fg_layer);

    g_stage = stage; js_retain(g_stage);
    g_fg_layer = fg_layer; js_retain(g_fg_layer);
    g_overlay = overlay; js_retain(g_overlay);
    g_cards[0] = card_a; js_retain(g_cards[0]);
    g_cards[1] = card_b; js_retain(g_cards[1]);
    g_cards[2] = card_c; js_retain(g_cards[2]);

    for (int i = 0; i < 3; i++) {
        Konva_Rect_on_click(g_cards[i], KonvaEventListener_from(on_card_click, (void*)&g_card_indices[i]));
    }
    Konva_Rect_fire(card_a, "click");
    Konva_Rect_fire(card_a, "click");
}
