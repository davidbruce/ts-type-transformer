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
    js_scope_open();
    int idx = *(const int*)ctx;
    Konva_Rect card = g_cards[idx];
    Konva_Rect_moveToTop(card);
    Konva_Group_moveToTop(g_overlay);
    Konva_Layer_draw(g_fg_layer);
    g_clicks++;
    int z = (int)Konva_Rect_getZIndex(card);

    Konva_Vector2d pt = Konva_Vector2d_create();
    Konva_Vector2d_set_x(pt, 250);
    Konva_Vector2d_set_y(pt, 160);
    Konva_Shape hit = Konva_Stage_getIntersection(g_stage, pt);
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
    js_scope_close();
}

static Konva_Line make_line(double x1, double y1, double x2, double y2, const char* color, double width) {
    Konva_LineConfig lc = Konva_LineConfig_create();
    Konva_LineConfig_set_points(lc, Konva_LinePoints_of(JsArray_double_from((double[]){ x1, y1, x2, y2 }, 4)));
    Konva_LineConfig_set_stroke(lc, Konva_string_or_CanvasGradient_of(color));
    Konva_LineConfig_set_strokeWidth(lc, width);
    return Konva_Line_new(lc);
}

static Konva_Rect make_card(const char* id, double x, double y, const char* fill) {
    Konva_RectConfig rc = Konva_RectConfig_create();
    Konva_RectConfig_set_id(rc, id);
    Konva_RectConfig_set_name(rc, "card");
    Konva_RectConfig_set_x(rc, x);
    Konva_RectConfig_set_y(rc, y);
    Konva_RectConfig_set_width(rc, 200);
    Konva_RectConfig_set_height(rc, 130);
    Konva_RectConfig_set_fill(rc, Konva_string_or_CanvasGradient_of(fill));
    Konva_RectConfig_set_stroke(rc, Konva_string_or_CanvasGradient_of("#111111"));
    Konva_RectConfig_set_strokeWidth(rc, 2);
    return Konva_Rect_new(rc);
}

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();

    Konva_StageConfig sc = Konva_StageConfig_create();
    Konva_StageConfig_set_container(sc, Konva_HTMLDivElement_or_string_of("stage"));
    Konva_StageConfig_set_width(sc, 680);
    Konva_StageConfig_set_height(sc, 400);
    Konva_Stage stage = Konva_Stage_new(sc);

    Konva_Layer bg_layer = Konva_Layer_new();
    Konva_Layer fg_layer = Konva_Layer_new();
    Konva_Stage_add(stage, bg_layer);
    Konva_Stage_add(stage, fg_layer);

    // Layer 1: Blueprint Grid.
    JsArray_Konva_Node bg_items = JsArray_Konva_Node_new();
    Konva_RectConfig bgr = Konva_RectConfig_create();
    Konva_RectConfig_set_x(bgr, 0);
    Konva_RectConfig_set_y(bgr, 0);
    Konva_RectConfig_set_width(bgr, 680);
    Konva_RectConfig_set_height(bgr, 400);
    Konva_RectConfig_set_fill(bgr, Konva_string_or_CanvasGradient_of("#ffffff"));
    JsArray_Konva_Node_push(bg_items, AS_Konva_Node(Konva_Rect_new(bgr)));

    for (int x = 40; x <= 640; x += 40) {
        JsArray_Konva_Node_push(bg_items, AS_Konva_Node(make_line((double)x, 20, (double)x, 380, "#e5e5e5", 1)));
    }
    for (int y = 40; y <= 360; y += 40) {
        JsArray_Konva_Node_push(bg_items, AS_Konva_Node(make_line(20, (double)y, 660, (double)y, "#e5e5e5", 1)));
    }

    Konva_RectConfig frame = Konva_RectConfig_create();
    Konva_RectConfig_set_x(frame, 20);
    Konva_RectConfig_set_y(frame, 20);
    Konva_RectConfig_set_width(frame, 640);
    Konva_RectConfig_set_height(frame, 360);
    Konva_RectConfig_set_stroke(frame, Konva_string_or_CanvasGradient_of("#b5b5b5"));
    Konva_RectConfig_set_strokeWidth(frame, 1.5);
    Konva_RectConfig_set_dash(frame, JsArray_double_from((double[]){ 6.0, 4.0 }, 2));
    JsArray_Konva_Node_push(bg_items, AS_Konva_Node(Konva_Rect_new(frame)));

    Konva_TextConfig l1 = Konva_TextConfig_create();
    Konva_TextConfig_set_x(l1, 32);
    Konva_TextConfig_set_y(l1, 32);
    Konva_TextConfig_set_text(l1, "Layer 1: Blueprint Grid (z=0)");
    Konva_TextConfig_set_fontSize(l1, 12);
    Konva_TextConfig_set_fontFamily(l1, "monospace");
    Konva_TextConfig_set_fill(l1, Konva_string_or_CanvasGradient_of("#737373"));
    JsArray_Konva_Node_push(bg_items, AS_Konva_Node(Konva_Text_new(l1)));
    Konva_Layer_add(bg_layer, bg_items);

    // Layer 2: Stacked cards & badge.
    Konva_Rect card_a = make_card("card-a", 110, 80, "#efefef");
    Konva_Rect card_b = make_card("card-b", 210, 125, "#404040");
    Konva_Rect card_c = make_card("card-c", 310, 170, "#ffffff");
    Konva_CircleConfig bc = Konva_CircleConfig_create();
    Konva_CircleConfig_set_id(bc, "badge");
    Konva_CircleConfig_set_x(bc, 555);
    Konva_CircleConfig_set_y(bc, 135);
    Konva_CircleConfig_set_radius(bc, 38);
    Konva_CircleConfig_set_fill(bc, Konva_string_or_CanvasGradient_of("#111111"));
    Konva_Circle badge = Konva_Circle_new(bc);

    // Crosshair & foreground label (non-listening overlay group kept above raised cards).
    Konva_GroupConfig gc = Konva_GroupConfig_create();
    Konva_GroupConfig_set_listening(gc, false);
    Konva_Group overlay = Konva_Group_new(gc);
    JsArray_Konva_Node overlay_items = JsArray_Konva_Node_new();
    Konva_CircleConfig dc = Konva_CircleConfig_create();
    Konva_CircleConfig_set_x(dc, 250);
    Konva_CircleConfig_set_y(dc, 160);
    Konva_CircleConfig_set_radius(dc, 7);
    Konva_CircleConfig_set_fill(dc, Konva_string_or_CanvasGradient_of("#ffffff"));
    Konva_CircleConfig_set_stroke(dc, Konva_string_or_CanvasGradient_of("#111111"));
    Konva_CircleConfig_set_strokeWidth(dc, 2);
    JsArray_Konva_Node_push(overlay_items, AS_Konva_Node(Konva_Circle_new(dc)));
    JsArray_Konva_Node_push(overlay_items, AS_Konva_Node(make_line(238, 160, 262, 160, "#111111", 1.5)));
    JsArray_Konva_Node_push(overlay_items, AS_Konva_Node(make_line(250, 148, 250, 172, "#111111", 1.5)));

    Konva_TextConfig l2 = Konva_TextConfig_create();
    Konva_TextConfig_set_x(l2, 32);
    Konva_TextConfig_set_y(l2, 354);
    Konva_TextConfig_set_text(l2, "Layer 2: Elevated Stack (z=1) \xC2\xB7 Probe (250,160)");
    Konva_TextConfig_set_fontSize(l2, 12);
    Konva_TextConfig_set_fontFamily(l2, "monospace");
    Konva_TextConfig_set_fill(l2, Konva_string_or_CanvasGradient_of("#111111"));
    JsArray_Konva_Node_push(overlay_items, AS_Konva_Node(Konva_Text_new(l2)));
    Konva_Group_add(overlay, overlay_items);

    Konva_Node fg_arr[5] = { AS_Konva_Node(card_a), AS_Konva_Node(card_b), AS_Konva_Node(card_c), AS_Konva_Node(badge), AS_Konva_Node(overlay) };
    Konva_Layer_add(fg_layer, JsArray_Konva_Node_from(fg_arr, 5));

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
    js_scope_close();
}
