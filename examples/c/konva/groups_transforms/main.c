#include "dom/dom.h"
#include "konva/konva.h"
#include <stdio.h>

static Line make_grid_line(double x1, double y1, double x2, double y2) {
    LineConfig lc = LineConfig_create();
    LineConfig_set_points(lc, LinePoints_of(JsArray_double_from((double[]){ x1, y1, x2, y2 }, 4)));
    LineConfig_set_stroke(lc, string_or_CanvasGradient_of("#e5e5e5"));
    LineConfig_set_strokeWidth(lc, 1);
    return Line_new(lc);
}

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();

    StageConfig sc = StageConfig_create();
    StageConfig_set_container(sc, HTMLDivElement_or_string_of("stage"));
    StageConfig_set_width(sc, 680);
    StageConfig_set_height(sc, 400);
    Stage stage = Stage_new(sc);

    Layer layer = Layer_new();
    Stage_add(stage, layer);
    JsArray_Node_2 top_items = JsArray_Node_2_new();

    RectConfig bg = RectConfig_create();
    RectConfig_set_x(bg, 0);
    RectConfig_set_y(bg, 0);
    RectConfig_set_width(bg, 680);
    RectConfig_set_height(bg, 400);
    RectConfig_set_fill(bg, string_or_CanvasGradient_of("#ffffff"));
    JsArray_Node_2_push(top_items, AS_Node_2(Rect_new(bg)));

    for (int x = 40; x <= 640; x += 40) {
        JsArray_Node_2_push(top_items, AS_Node_2(make_grid_line((double)x, 0, (double)x, 400)));
    }
    for (int y = 40; y <= 360; y += 40) {
        JsArray_Node_2_push(top_items, AS_Node_2(make_grid_line(0, (double)y, 680, (double)y)));
    }

    // Group A: unrotated base frame.
    GroupConfig gac = GroupConfig_create();
    GroupConfig_set_x(gac, 40);
    GroupConfig_set_y(gac, 55);
    Group group_a = Group_new(gac);
    JsArray_Node_2 ga_ch = JsArray_Node_2_new();

    RectConfig gab = RectConfig_create();
    RectConfig_set_width(gab, 160);
    RectConfig_set_height(gab, 120);
    RectConfig_set_fill(gab, string_or_CanvasGradient_of("#f7f7f7"));
    RectConfig_set_stroke(gab, string_or_CanvasGradient_of("#111111"));
    RectConfig_set_strokeWidth(gab, 1.5);
    JsArray_Node_2_push(ga_ch, AS_Node_2(Rect_new(gab)));

    CircleConfig gad = CircleConfig_create();
    CircleConfig_set_x(gad, 80);
    CircleConfig_set_y(gad, 60);
    CircleConfig_set_radius(gad, 24);
    CircleConfig_set_fill(gad, string_or_CanvasGradient_of("#111111"));
    JsArray_Node_2_push(ga_ch, AS_Node_2(Circle_new(gad)));

    TextConfig gal = TextConfig_create();
    TextConfig_set_x(gal, 12);
    TextConfig_set_y(gal, 10);
    TextConfig_set_text(gal, "Group A \xC2\xB7 0\xC2\xB0");
    TextConfig_set_fontSize(gal, 11);
    TextConfig_set_fontFamily(gal, "monospace");
    TextConfig_set_fill(gal, string_or_CanvasGradient_of("#111111"));
    JsArray_Node_2_push(ga_ch, AS_Node_2(Text_2_new(gal)));
    Group_add(group_a, ga_ch);
    JsArray_Node_2_push(top_items, AS_Node_2(group_a));

    // Group B: rotated 24 deg.
    GroupConfig gbc = GroupConfig_create();
    GroupConfig_set_x(gbc, 340);
    GroupConfig_set_y(gbc, 75);
    GroupConfig_set_rotation(gbc, 24);
    Group group_b = Group_new(gbc);
    JsArray_Node_2 gb_ch = JsArray_Node_2_new();

    RectConfig gbo = RectConfig_create();
    RectConfig_set_width(gbo, 150);
    RectConfig_set_height(gbo, 110);
    RectConfig_set_fill(gbo, string_or_CanvasGradient_of("#efefef"));
    RectConfig_set_stroke(gbo, string_or_CanvasGradient_of("#111111"));
    RectConfig_set_strokeWidth(gbo, 2);
    JsArray_Node_2_push(gb_ch, AS_Node_2(Rect_new(gbo)));

    RectConfig gbi = RectConfig_create();
    RectConfig_set_x(gbi, 22);
    RectConfig_set_y(gbi, 22);
    RectConfig_set_width(gbi, 106);
    RectConfig_set_height(gbi, 66);
    RectConfig_set_fill(gbi, string_or_CanvasGradient_of("#ffffff"));
    RectConfig_set_stroke(gbi, string_or_CanvasGradient_of("#555555"));
    RectConfig_set_strokeWidth(gbi, 1.5);
    RectConfig_set_dash(gbi, JsArray_double_from((double[]){ 4.0, 3.0 }, 2));
    JsArray_Node_2_push(gb_ch, AS_Node_2(Rect_new(gbi)));

    CircleConfig gbd = CircleConfig_create();
    CircleConfig_set_x(gbd, 75);
    CircleConfig_set_y(gbd, 55);
    CircleConfig_set_radius(gbd, 14);
    CircleConfig_set_fill(gbd, string_or_CanvasGradient_of("#404040"));
    JsArray_Node_2_push(gb_ch, AS_Node_2(Circle_new(gbd)));

    TextConfig gbl = TextConfig_create();
    TextConfig_set_x(gbl, 10);
    TextConfig_set_y(gbl, 8);
    TextConfig_set_text(gbl, "Group B \xC2\xB7 24\xC2\xB0");
    TextConfig_set_fontSize(gbl, 11);
    TextConfig_set_fontFamily(gbl, "monospace");
    TextConfig_set_fill(gbl, string_or_CanvasGradient_of("#111111"));
    JsArray_Node_2_push(gb_ch, AS_Node_2(Text_2_new(gbl)));
    Group_add(group_b, gb_ch);
    JsArray_Node_2_push(top_items, AS_Node_2(group_b));

    // Group C: scaled container with nested rotated child group.
    GroupConfig gcc = GroupConfig_create();
    GroupConfig_set_x(gcc, 495);
    GroupConfig_set_y(gcc, 55);
    GroupConfig_set_scaleX(gcc, 1.15);
    GroupConfig_set_scaleY(gcc, 1.15);
    Group group_c = Group_new(gcc);
    JsArray_Node_2 gc_ch = JsArray_Node_2_new();

    RectConfig gco = RectConfig_create();
    RectConfig_set_width(gco, 130);
    RectConfig_set_height(gco, 110);
    RectConfig_set_fill(gco, string_or_CanvasGradient_of("#ffffff"));
    RectConfig_set_stroke(gco, string_or_CanvasGradient_of("#111111"));
    RectConfig_set_strokeWidth(gco, 2);
    JsArray_Node_2_push(gc_ch, AS_Node_2(Rect_new(gco)));

    TextConfig gcl = TextConfig_create();
    TextConfig_set_x(gcl, 8);
    TextConfig_set_y(gcl, 8);
    TextConfig_set_text(gcl, "Group C \xC2\xB7 1.15x");
    TextConfig_set_fontSize(gcl, 10);
    TextConfig_set_fontFamily(gcl, "monospace");
    TextConfig_set_fill(gcl, string_or_CanvasGradient_of("#111111"));
    JsArray_Node_2_push(gc_ch, AS_Node_2(Text_2_new(gcl)));

    GroupConfig icc = GroupConfig_create();
    GroupConfig_set_x(icc, 25);
    GroupConfig_set_y(icc, 25);
    GroupConfig_set_rotation(icc, 15);
    Group inner_group = Group_new(icc);

    CircleConfig mc = CircleConfig_create();
    CircleConfig_set_x(mc, 40);
    CircleConfig_set_y(mc, 30);
    CircleConfig_set_radius(mc, 16);
    CircleConfig_set_fill(mc, string_or_CanvasGradient_of("#111111"));
    Circle marker = Circle_new(mc);
    Node_2 marker_node = AS_Node_2(marker);
    Group_add(inner_group, JsArray_Node_2_from(&marker_node, 1));
    JsArray_Node_2_push(gc_ch, AS_Node_2(inner_group));
    Group_add(group_c, gc_ch);
    JsArray_Node_2_push(top_items, AS_Node_2(group_c));

    Node_getAbsolutePosition_result abs = Circle_getAbsolutePosition(marker);
    IRect bbox = Group_getClientRect(group_b);
    int ax = (int)(Node_getAbsolutePosition_result_get_x(abs) + 0.5);
    int ay = (int)(Node_getAbsolutePosition_result_get_y(abs) + 0.5);
    int bw = (int)(IRect_get_width(bbox) + 0.5);
    int bh = (int)(IRect_get_height(bbox) + 0.5);

    char strip_txt[160];
    snprintf(strip_txt, sizeof strip_txt, "Marker abs=(%d, %d) \xC2\xB7 Group B bbox=%dx%d", ax, ay, bw, bh);

    RectConfig strip = RectConfig_create();
    RectConfig_set_x(strip, 28);
    RectConfig_set_y(strip, 308);
    RectConfig_set_width(strip, 624);
    RectConfig_set_height(strip, 60);
    RectConfig_set_fill(strip, string_or_CanvasGradient_of("#f7f7f7"));
    RectConfig_set_stroke(strip, string_or_CanvasGradient_of("#111111"));
    RectConfig_set_strokeWidth(strip, 1.5);
    JsArray_Node_2_push(top_items, AS_Node_2(Rect_new(strip)));

    TextConfig sum = TextConfig_create();
    TextConfig_set_x(sum, 44);
    TextConfig_set_y(sum, 331);
    TextConfig_set_text(sum, strip_txt);
    TextConfig_set_fontSize(sum, 12);
    TextConfig_set_fontFamily(sum, "monospace");
    TextConfig_set_fill(sum, string_or_CanvasGradient_of("#111111"));
    JsArray_Node_2_push(top_items, AS_Node_2(Text_2_new(sum)));

    Layer_add(layer, top_items);
    Layer_draw(layer);

    char meta[192];
    snprintf(meta, sizeof meta, "Stage: 680x400 \xC2\xB7 Nested marker abs=(%d, %d) \xC2\xB7 Rotated Group B rect=%dx%d", ax, ay, bw, bh);
    HTMLElement_set_textContent(Document_getElementById(get_document(), "meta"), meta);
    js_scope_close();
}
