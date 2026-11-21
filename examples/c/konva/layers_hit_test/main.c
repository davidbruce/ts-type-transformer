#include "dom/dom.h"
#include "konva/konva.h"
#include <stdio.h>

static int32_t g_clicks = 0;
static void on_card_click(void* ctx, KonvaEventObject e) {
    (void)ctx;
    (void)e;
    g_clicks++;
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

    Konva_Node fg_arr[4] = { AS_Konva_Node(card_a), AS_Konva_Node(card_b), AS_Konva_Node(card_c), AS_Konva_Node(badge) };
    Konva_Layer_add(fg_layer, JsArray_Konva_Node_from(fg_arr, 4));

    Konva_Rect_moveToTop(card_a);
    int card_a_z = (int)Konva_Rect_getZIndex(card_a);

    Konva_Rect_on_click(card_a, KonvaEventListener_from(on_card_click, 0));
    Konva_Rect_fire(card_a, "click");
    Konva_Rect_fire(card_a, "click");

    Konva_Layer_draw(bg_layer);
    Konva_Layer_draw(fg_layer);

    Konva_Vector2d pt = Konva_Vector2d_create();
    Konva_Vector2d_set_x(pt, 250);
    Konva_Vector2d_set_y(pt, 160);
    Konva_Shape hit = Konva_Stage_getIntersection(stage, pt);
    char hit_buf[64] = "none";
    if (hit.h != 0) {
        JsString_copy(js_to_string(Konva_GetSet_invoke(Konva_Shape_get_id(hit))), hit_buf, sizeof hit_buf);
    }

    // Crosshair & foreground label.
    JsArray_Konva_Node overlay = JsArray_Konva_Node_new();
    Konva_CircleConfig dc = Konva_CircleConfig_create();
    Konva_CircleConfig_set_x(dc, 250);
    Konva_CircleConfig_set_y(dc, 160);
    Konva_CircleConfig_set_radius(dc, 7);
    Konva_CircleConfig_set_fill(dc, Konva_string_or_CanvasGradient_of("#ffffff"));
    Konva_CircleConfig_set_stroke(dc, Konva_string_or_CanvasGradient_of("#111111"));
    Konva_CircleConfig_set_strokeWidth(dc, 2);
    JsArray_Konva_Node_push(overlay, AS_Konva_Node(Konva_Circle_new(dc)));
    JsArray_Konva_Node_push(overlay, AS_Konva_Node(make_line(238, 160, 262, 160, "#111111", 1.5)));
    JsArray_Konva_Node_push(overlay, AS_Konva_Node(make_line(250, 148, 250, 172, "#111111", 1.5)));

    Konva_TextConfig l2 = Konva_TextConfig_create();
    Konva_TextConfig_set_x(l2, 32);
    Konva_TextConfig_set_y(l2, 354);
    Konva_TextConfig_set_text(l2, "Layer 2: Elevated Stack (z=1) \xC2\xB7 Probe (250,160)");
    Konva_TextConfig_set_fontSize(l2, 12);
    Konva_TextConfig_set_fontFamily(l2, "monospace");
    Konva_TextConfig_set_fill(l2, Konva_string_or_CanvasGradient_of("#111111"));
    JsArray_Konva_Node_push(overlay, AS_Konva_Node(Konva_Text_new(l2)));
    Konva_Layer_add(fg_layer, overlay);
    Konva_Layer_draw(fg_layer);

    int layers = (int)JsArray_Konva_Layer_length(Konva_Stage_getLayers(stage));
    char meta[192];
    snprintf(meta, sizeof meta, "Layers: %d \xC2\xB7 card-a zIndex=%d \xC2\xB7 hit@(250,160)=%s \xC2\xB7 clicks=%d",
        layers, card_a_z, hit_buf, (int)g_clicks);
    HTMLElement_set_textContent(Document_getElementById(get_document(), "meta"), meta);
    js_scope_close();
}
