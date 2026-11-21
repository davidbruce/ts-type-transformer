#include "dom/dom.h"
#include "konva/konva.h"
#include <stdio.h>

static Konva_Line make_grid_line(double x1, double y1, double x2, double y2) {
    Konva_LineConfig lc = Konva_LineConfig_create();
    Konva_LineConfig_set_points(lc, Konva_LinePoints_of(JsArray_double_from((double[]){ x1, y1, x2, y2 }, 4)));
    Konva_LineConfig_set_stroke(lc, Konva_string_or_CanvasGradient_of("#e5e5e5"));
    Konva_LineConfig_set_strokeWidth(lc, 1);
    return Konva_Line_new(lc);
}

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();

    Konva_StageConfig sc = Konva_StageConfig_create();
    Konva_StageConfig_set_container(sc, Konva_HTMLDivElement_or_string_of("stage"));
    Konva_StageConfig_set_width(sc, 680);
    Konva_StageConfig_set_height(sc, 400);
    Konva_Stage stage = Konva_Stage_new(sc);

    Konva_Layer layer = Konva_Layer_new();
    Konva_Stage_add(stage, layer);
    JsArray_Konva_Node top_items = JsArray_Konva_Node_new();

    Konva_RectConfig bg = Konva_RectConfig_create();
    Konva_RectConfig_set_x(bg, 0);
    Konva_RectConfig_set_y(bg, 0);
    Konva_RectConfig_set_width(bg, 680);
    Konva_RectConfig_set_height(bg, 400);
    Konva_RectConfig_set_fill(bg, Konva_string_or_CanvasGradient_of("#ffffff"));
    JsArray_Konva_Node_push(top_items, AS_Konva_Node(Konva_Rect_new(bg)));

    for (int x = 40; x <= 640; x += 40) {
        JsArray_Konva_Node_push(top_items, AS_Konva_Node(make_grid_line((double)x, 0, (double)x, 400)));
    }
    for (int y = 40; y <= 360; y += 40) {
        JsArray_Konva_Node_push(top_items, AS_Konva_Node(make_grid_line(0, (double)y, 680, (double)y)));
    }

    // Group A: unrotated base frame.
    Konva_GroupConfig gac = Konva_GroupConfig_create();
    Konva_GroupConfig_set_x(gac, 40);
    Konva_GroupConfig_set_y(gac, 55);
    Konva_Group group_a = Konva_Group_new(gac);
    JsArray_Konva_Node ga_ch = JsArray_Konva_Node_new();

    Konva_RectConfig gab = Konva_RectConfig_create();
    Konva_RectConfig_set_width(gab, 160);
    Konva_RectConfig_set_height(gab, 120);
    Konva_RectConfig_set_fill(gab, Konva_string_or_CanvasGradient_of("#f7f7f7"));
    Konva_RectConfig_set_stroke(gab, Konva_string_or_CanvasGradient_of("#111111"));
    Konva_RectConfig_set_strokeWidth(gab, 1.5);
    JsArray_Konva_Node_push(ga_ch, AS_Konva_Node(Konva_Rect_new(gab)));

    Konva_CircleConfig gad = Konva_CircleConfig_create();
    Konva_CircleConfig_set_x(gad, 80);
    Konva_CircleConfig_set_y(gad, 60);
    Konva_CircleConfig_set_radius(gad, 24);
    Konva_CircleConfig_set_fill(gad, Konva_string_or_CanvasGradient_of("#111111"));
    JsArray_Konva_Node_push(ga_ch, AS_Konva_Node(Konva_Circle_new(gad)));

    Konva_TextConfig gal = Konva_TextConfig_create();
    Konva_TextConfig_set_x(gal, 12);
    Konva_TextConfig_set_y(gal, 10);
    Konva_TextConfig_set_text(gal, "Group A \xC2\xB7 0\xC2\xB0");
    Konva_TextConfig_set_fontSize(gal, 11);
    Konva_TextConfig_set_fontFamily(gal, "monospace");
    Konva_TextConfig_set_fill(gal, Konva_string_or_CanvasGradient_of("#111111"));
    JsArray_Konva_Node_push(ga_ch, AS_Konva_Node(Konva_Text_new(gal)));
    Konva_Group_add(group_a, ga_ch);
    JsArray_Konva_Node_push(top_items, AS_Konva_Node(group_a));

    // Group B: rotated 24 deg.
    Konva_GroupConfig gbc = Konva_GroupConfig_create();
    Konva_GroupConfig_set_x(gbc, 340);
    Konva_GroupConfig_set_y(gbc, 75);
    Konva_GroupConfig_set_rotation(gbc, 24);
    Konva_Group group_b = Konva_Group_new(gbc);
    JsArray_Konva_Node gb_ch = JsArray_Konva_Node_new();

    Konva_RectConfig gbo = Konva_RectConfig_create();
    Konva_RectConfig_set_width(gbo, 150);
    Konva_RectConfig_set_height(gbo, 110);
    Konva_RectConfig_set_fill(gbo, Konva_string_or_CanvasGradient_of("#efefef"));
    Konva_RectConfig_set_stroke(gbo, Konva_string_or_CanvasGradient_of("#111111"));
    Konva_RectConfig_set_strokeWidth(gbo, 2);
    JsArray_Konva_Node_push(gb_ch, AS_Konva_Node(Konva_Rect_new(gbo)));

    Konva_RectConfig gbi = Konva_RectConfig_create();
    Konva_RectConfig_set_x(gbi, 22);
    Konva_RectConfig_set_y(gbi, 22);
    Konva_RectConfig_set_width(gbi, 106);
    Konva_RectConfig_set_height(gbi, 66);
    Konva_RectConfig_set_fill(gbi, Konva_string_or_CanvasGradient_of("#ffffff"));
    Konva_RectConfig_set_stroke(gbi, Konva_string_or_CanvasGradient_of("#555555"));
    Konva_RectConfig_set_strokeWidth(gbi, 1.5);
    Konva_RectConfig_set_dash(gbi, JsArray_double_from((double[]){ 4.0, 3.0 }, 2));
    JsArray_Konva_Node_push(gb_ch, AS_Konva_Node(Konva_Rect_new(gbi)));

    Konva_CircleConfig gbd = Konva_CircleConfig_create();
    Konva_CircleConfig_set_x(gbd, 75);
    Konva_CircleConfig_set_y(gbd, 55);
    Konva_CircleConfig_set_radius(gbd, 14);
    Konva_CircleConfig_set_fill(gbd, Konva_string_or_CanvasGradient_of("#404040"));
    JsArray_Konva_Node_push(gb_ch, AS_Konva_Node(Konva_Circle_new(gbd)));

    Konva_TextConfig gbl = Konva_TextConfig_create();
    Konva_TextConfig_set_x(gbl, 10);
    Konva_TextConfig_set_y(gbl, 8);
    Konva_TextConfig_set_text(gbl, "Group B \xC2\xB7 24\xC2\xB0");
    Konva_TextConfig_set_fontSize(gbl, 11);
    Konva_TextConfig_set_fontFamily(gbl, "monospace");
    Konva_TextConfig_set_fill(gbl, Konva_string_or_CanvasGradient_of("#111111"));
    JsArray_Konva_Node_push(gb_ch, AS_Konva_Node(Konva_Text_new(gbl)));
    Konva_Group_add(group_b, gb_ch);
    JsArray_Konva_Node_push(top_items, AS_Konva_Node(group_b));

    // Group C: scaled container with nested rotated child group.
    Konva_GroupConfig gcc = Konva_GroupConfig_create();
    Konva_GroupConfig_set_x(gcc, 495);
    Konva_GroupConfig_set_y(gcc, 55);
    Konva_GroupConfig_set_scaleX(gcc, 1.15);
    Konva_GroupConfig_set_scaleY(gcc, 1.15);
    Konva_Group group_c = Konva_Group_new(gcc);
    JsArray_Konva_Node gc_ch = JsArray_Konva_Node_new();

    Konva_RectConfig gco = Konva_RectConfig_create();
    Konva_RectConfig_set_width(gco, 130);
    Konva_RectConfig_set_height(gco, 110);
    Konva_RectConfig_set_fill(gco, Konva_string_or_CanvasGradient_of("#ffffff"));
    Konva_RectConfig_set_stroke(gco, Konva_string_or_CanvasGradient_of("#111111"));
    Konva_RectConfig_set_strokeWidth(gco, 2);
    JsArray_Konva_Node_push(gc_ch, AS_Konva_Node(Konva_Rect_new(gco)));

    Konva_TextConfig gcl = Konva_TextConfig_create();
    Konva_TextConfig_set_x(gcl, 8);
    Konva_TextConfig_set_y(gcl, 8);
    Konva_TextConfig_set_text(gcl, "Group C \xC2\xB7 1.15x");
    Konva_TextConfig_set_fontSize(gcl, 10);
    Konva_TextConfig_set_fontFamily(gcl, "monospace");
    Konva_TextConfig_set_fill(gcl, Konva_string_or_CanvasGradient_of("#111111"));
    JsArray_Konva_Node_push(gc_ch, AS_Konva_Node(Konva_Text_new(gcl)));

    Konva_GroupConfig icc = Konva_GroupConfig_create();
    Konva_GroupConfig_set_x(icc, 25);
    Konva_GroupConfig_set_y(icc, 25);
    Konva_GroupConfig_set_rotation(icc, 15);
    Konva_Group inner_group = Konva_Group_new(icc);

    Konva_CircleConfig mc = Konva_CircleConfig_create();
    Konva_CircleConfig_set_x(mc, 40);
    Konva_CircleConfig_set_y(mc, 30);
    Konva_CircleConfig_set_radius(mc, 16);
    Konva_CircleConfig_set_fill(mc, Konva_string_or_CanvasGradient_of("#111111"));
    Konva_Circle marker = Konva_Circle_new(mc);
    Konva_Node marker_node = AS_Konva_Node(marker);
    Konva_Group_add(inner_group, JsArray_Konva_Node_from(&marker_node, 1));
    JsArray_Konva_Node_push(gc_ch, AS_Konva_Node(inner_group));
    Konva_Group_add(group_c, gc_ch);
    JsArray_Konva_Node_push(top_items, AS_Konva_Node(group_c));

    Konva_Node_getAbsolutePosition_result abs = Konva_Circle_getAbsolutePosition(marker);
    Konva_IRect bbox = Konva_Group_getClientRect(group_b);
    int ax = (int)(Konva_Node_getAbsolutePosition_result_get_x(abs) + 0.5);
    int ay = (int)(Konva_Node_getAbsolutePosition_result_get_y(abs) + 0.5);
    int bw = (int)(Konva_IRect_get_width(bbox) + 0.5);
    int bh = (int)(Konva_IRect_get_height(bbox) + 0.5);

    char strip_txt[160];
    snprintf(strip_txt, sizeof strip_txt, "Marker abs=(%d, %d) \xC2\xB7 Group B bbox=%dx%d", ax, ay, bw, bh);

    Konva_RectConfig strip = Konva_RectConfig_create();
    Konva_RectConfig_set_x(strip, 28);
    Konva_RectConfig_set_y(strip, 308);
    Konva_RectConfig_set_width(strip, 624);
    Konva_RectConfig_set_height(strip, 60);
    Konva_RectConfig_set_fill(strip, Konva_string_or_CanvasGradient_of("#f7f7f7"));
    Konva_RectConfig_set_stroke(strip, Konva_string_or_CanvasGradient_of("#111111"));
    Konva_RectConfig_set_strokeWidth(strip, 1.5);
    JsArray_Konva_Node_push(top_items, AS_Konva_Node(Konva_Rect_new(strip)));

    Konva_TextConfig sum = Konva_TextConfig_create();
    Konva_TextConfig_set_x(sum, 44);
    Konva_TextConfig_set_y(sum, 331);
    Konva_TextConfig_set_text(sum, strip_txt);
    Konva_TextConfig_set_fontSize(sum, 12);
    Konva_TextConfig_set_fontFamily(sum, "monospace");
    Konva_TextConfig_set_fill(sum, Konva_string_or_CanvasGradient_of("#111111"));
    JsArray_Konva_Node_push(top_items, AS_Konva_Node(Konva_Text_new(sum)));

    Konva_Layer_add(layer, top_items);
    Konva_Layer_draw(layer);

    char meta[192];
    snprintf(meta, sizeof meta, "Stage: 680x400 \xC2\xB7 Nested marker abs=(%d, %d) \xC2\xB7 Rotated Group B rect=%dx%d", ax, ay, bw, bh);
    HTMLElement_set_textContent(Document_getElementById(get_document(), "meta"), meta);
    js_scope_close();
}
