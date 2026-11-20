#include "dom/dom.h"
#include "konva/konva.h"
#include <stdio.h>

static int32_t g_clicks = 0;
static void on_card_click(void* ctx, KonvaEventObject e) {
    (void)ctx;
    (void)e;
    g_clicks++;
}

static Line make_line(double x1, double y1, double x2, double y2, const char* color, double width) {
    LineConfig lc = LineConfig_create();
    LineConfig_set_points(lc, LinePoints_of(JsArray_double_from((double[]){ x1, y1, x2, y2 }, 4)));
    LineConfig_set_stroke(lc, string_or_CanvasGradient_of(color));
    LineConfig_set_strokeWidth(lc, width);
    return Line_new(lc);
}

static Rect make_card(const char* id, double x, double y, const char* fill) {
    RectConfig rc = RectConfig_create();
    RectConfig_set_id(rc, id);
    RectConfig_set_name(rc, "card");
    RectConfig_set_x(rc, x);
    RectConfig_set_y(rc, y);
    RectConfig_set_width(rc, 200);
    RectConfig_set_height(rc, 130);
    RectConfig_set_fill(rc, string_or_CanvasGradient_of(fill));
    RectConfig_set_stroke(rc, string_or_CanvasGradient_of("#111111"));
    RectConfig_set_strokeWidth(rc, 2);
    return Rect_new(rc);
}

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();

    StageConfig sc = StageConfig_create();
    StageConfig_set_container(sc, HTMLDivElement_or_string_of("stage"));
    StageConfig_set_width(sc, 680);
    StageConfig_set_height(sc, 400);
    Stage stage = Stage_new(sc);

    Layer bg_layer = Layer_new();
    Layer fg_layer = Layer_new();
    Stage_add(stage, bg_layer);
    Stage_add(stage, fg_layer);

    // Layer 1: Blueprint Grid.
    JsArray_Node_2 bg_items = JsArray_Node_2_new();
    RectConfig bgr = RectConfig_create();
    RectConfig_set_x(bgr, 0);
    RectConfig_set_y(bgr, 0);
    RectConfig_set_width(bgr, 680);
    RectConfig_set_height(bgr, 400);
    RectConfig_set_fill(bgr, string_or_CanvasGradient_of("#ffffff"));
    JsArray_Node_2_push(bg_items, AS_Node_2(Rect_new(bgr)));

    for (int x = 40; x <= 640; x += 40) {
        JsArray_Node_2_push(bg_items, AS_Node_2(make_line((double)x, 20, (double)x, 380, "#e5e5e5", 1)));
    }
    for (int y = 40; y <= 360; y += 40) {
        JsArray_Node_2_push(bg_items, AS_Node_2(make_line(20, (double)y, 660, (double)y, "#e5e5e5", 1)));
    }

    RectConfig frame = RectConfig_create();
    RectConfig_set_x(frame, 20);
    RectConfig_set_y(frame, 20);
    RectConfig_set_width(frame, 640);
    RectConfig_set_height(frame, 360);
    RectConfig_set_stroke(frame, string_or_CanvasGradient_of("#b5b5b5"));
    RectConfig_set_strokeWidth(frame, 1.5);
    RectConfig_set_dash(frame, JsArray_double_from((double[]){ 6.0, 4.0 }, 2));
    JsArray_Node_2_push(bg_items, AS_Node_2(Rect_new(frame)));

    TextConfig l1 = TextConfig_create();
    TextConfig_set_x(l1, 32);
    TextConfig_set_y(l1, 32);
    TextConfig_set_text(l1, "Layer 1: Blueprint Grid (z=0)");
    TextConfig_set_fontSize(l1, 12);
    TextConfig_set_fontFamily(l1, "monospace");
    TextConfig_set_fill(l1, string_or_CanvasGradient_of("#737373"));
    JsArray_Node_2_push(bg_items, AS_Node_2(Text_2_new(l1)));
    Layer_add(bg_layer, bg_items);

    // Layer 2: Stacked cards & badge.
    Rect card_a = make_card("card-a", 110, 80, "#efefef");
    Rect card_b = make_card("card-b", 210, 125, "#404040");
    Rect card_c = make_card("card-c", 310, 170, "#ffffff");
    CircleConfig bc = CircleConfig_create();
    CircleConfig_set_id(bc, "badge");
    CircleConfig_set_x(bc, 555);
    CircleConfig_set_y(bc, 135);
    CircleConfig_set_radius(bc, 38);
    CircleConfig_set_fill(bc, string_or_CanvasGradient_of("#111111"));
    Circle badge = Circle_new(bc);

    Node_2 fg_arr[4] = { AS_Node_2(card_a), AS_Node_2(card_b), AS_Node_2(card_c), AS_Node_2(badge) };
    Layer_add(fg_layer, JsArray_Node_2_from(fg_arr, 4));

    Rect_moveToTop(card_a);
    int card_a_z = (int)Rect_getZIndex(card_a);

    Rect_on_click(card_a, KonvaEventListener_from(on_card_click, 0));
    Rect_fire(card_a, "click");
    Rect_fire(card_a, "click");

    Layer_draw(bg_layer);
    Layer_draw(fg_layer);

    Vector2d pt = Vector2d_create();
    Vector2d_set_x(pt, 250);
    Vector2d_set_y(pt, 160);
    Shape hit = Stage_getIntersection(stage, pt);
    char hit_buf[64] = "none";
    if (hit.h != 0) {
        JsString_copy(js_to_string(GetSet_invoke(Shape_get_id(hit))), hit_buf, sizeof hit_buf);
    }

    // Crosshair & foreground label.
    JsArray_Node_2 overlay = JsArray_Node_2_new();
    CircleConfig dc = CircleConfig_create();
    CircleConfig_set_x(dc, 250);
    CircleConfig_set_y(dc, 160);
    CircleConfig_set_radius(dc, 7);
    CircleConfig_set_fill(dc, string_or_CanvasGradient_of("#ffffff"));
    CircleConfig_set_stroke(dc, string_or_CanvasGradient_of("#111111"));
    CircleConfig_set_strokeWidth(dc, 2);
    JsArray_Node_2_push(overlay, AS_Node_2(Circle_new(dc)));
    JsArray_Node_2_push(overlay, AS_Node_2(make_line(238, 160, 262, 160, "#111111", 1.5)));
    JsArray_Node_2_push(overlay, AS_Node_2(make_line(250, 148, 250, 172, "#111111", 1.5)));

    TextConfig l2 = TextConfig_create();
    TextConfig_set_x(l2, 32);
    TextConfig_set_y(l2, 354);
    TextConfig_set_text(l2, "Layer 2: Elevated Stack (z=1) \xC2\xB7 Probe (250,160)");
    TextConfig_set_fontSize(l2, 12);
    TextConfig_set_fontFamily(l2, "monospace");
    TextConfig_set_fill(l2, string_or_CanvasGradient_of("#111111"));
    JsArray_Node_2_push(overlay, AS_Node_2(Text_2_new(l2)));
    Layer_add(fg_layer, overlay);
    Layer_draw(fg_layer);

    int layers = (int)JsArray_Layer_length(Stage_getLayers(stage));
    char meta[192];
    snprintf(meta, sizeof meta, "Layers: %d \xC2\xB7 card-a zIndex=%d \xC2\xB7 hit@(250,160)=%s \xC2\xB7 clicks=%d",
        layers, card_a_z, hit_buf, (int)g_clicks);
    HTMLElement_set_textContent(Document_getElementById(get_document(), "meta"), meta);
    js_scope_close();
}
