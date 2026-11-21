#include "dom/dom.h"
#include "konva/konva.h"
#include <stdio.h>

static void add_caption(JsArray_Konva_Node list, double x, double y, const char* caption) {
    Konva_TextConfig tc = Konva_TextConfig_create();
    Konva_TextConfig_set_x(tc, x);
    Konva_TextConfig_set_y(tc, y);
    Konva_TextConfig_set_text(tc, caption);
    Konva_TextConfig_set_fontSize(tc, 11);
    Konva_TextConfig_set_fontFamily(tc, "monospace");
    Konva_TextConfig_set_fill(tc, Konva_string_or_CanvasGradient_of("#555555"));
    JsArray_Konva_Node_push(list, AS_Konva_Node(Konva_Text_new(tc)));
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
    JsArray_Konva_Node children = JsArray_Konva_Node_new();

    Konva_RectConfig bg = Konva_RectConfig_create();
    Konva_RectConfig_set_x(bg, 0);
    Konva_RectConfig_set_y(bg, 0);
    Konva_RectConfig_set_width(bg, 680);
    Konva_RectConfig_set_height(bg, 400);
    Konva_RectConfig_set_fill(bg, Konva_string_or_CanvasGradient_of("#ffffff"));
    JsArray_Konva_Node_push(children, AS_Konva_Node(Konva_Rect_new(bg)));

    // Row 1: Rounded filled Rect, dashed Rect, Circle, Hexagon.
    Konva_RectConfig r1 = Konva_RectConfig_create();
    Konva_RectConfig_set_x(r1, 40);
    Konva_RectConfig_set_y(r1, 55);
    Konva_RectConfig_set_width(r1, 110);
    Konva_RectConfig_set_height(r1, 76);
    Konva_RectConfig_set_cornerRadius(r1, Konva_number_or_number_array_of(8));
    Konva_RectConfig_set_fill(r1, Konva_string_or_CanvasGradient_of("#111111"));
    JsArray_Konva_Node_push(children, AS_Konva_Node(Konva_Rect_new(r1)));
    add_caption(children, 40, 142, "Rect (filled)");

    Konva_RectConfig r2 = Konva_RectConfig_create();
    Konva_RectConfig_set_x(r2, 195);
    Konva_RectConfig_set_y(r2, 55);
    Konva_RectConfig_set_width(r2, 110);
    Konva_RectConfig_set_height(r2, 76);
    Konva_RectConfig_set_fill(r2, Konva_string_or_CanvasGradient_of("#ffffff"));
    Konva_RectConfig_set_stroke(r2, Konva_string_or_CanvasGradient_of("#111111"));
    Konva_RectConfig_set_strokeWidth(r2, 2);
    Konva_RectConfig_set_dash(r2, JsArray_double_from((double[]){ 6.0, 4.0 }, 2));
    JsArray_Konva_Node_push(children, AS_Konva_Node(Konva_Rect_new(r2)));
    add_caption(children, 195, 142, "Rect (dashed)");

    Konva_CircleConfig cc = Konva_CircleConfig_create();
    Konva_CircleConfig_set_x(cc, 415);
    Konva_CircleConfig_set_y(cc, 93);
    Konva_CircleConfig_set_radius(cc, 40);
    Konva_CircleConfig_set_fill(cc, Konva_string_or_CanvasGradient_of("#404040"));
    Konva_CircleConfig_set_stroke(cc, Konva_string_or_CanvasGradient_of("#111111"));
    Konva_CircleConfig_set_strokeWidth(cc, 2);
    JsArray_Konva_Node_push(children, AS_Konva_Node(Konva_Circle_new(cc)));
    add_caption(children, 395, 142, "Circle");

    Konva_RegularPolygonConfig pc = Konva_RegularPolygonConfig_create();
    Konva_RegularPolygonConfig_set_x(pc, 575);
    Konva_RegularPolygonConfig_set_y(pc, 93);
    Konva_RegularPolygonConfig_set_sides(pc, 6);
    Konva_RegularPolygonConfig_set_radius(pc, 42);
    Konva_RegularPolygonConfig_set_fill(pc, Konva_string_or_CanvasGradient_of("#efefef"));
    Konva_RegularPolygonConfig_set_stroke(pc, Konva_string_or_CanvasGradient_of("#111111"));
    Konva_RegularPolygonConfig_set_strokeWidth(pc, 2);
    JsArray_Konva_Node_push(children, AS_Konva_Node(Konva_RegularPolygon_new(pc)));
    add_caption(children, 548, 142, "Hexagon");

    // Row 2: Star, Ring, Wedge, Framed Text.
    Konva_StarConfig st = Konva_StarConfig_create();
    Konva_StarConfig_set_x(st, 95);
    Konva_StarConfig_set_y(st, 268);
    Konva_StarConfig_set_numPoints(st, 5);
    Konva_StarConfig_set_innerRadius(st, 19);
    Konva_StarConfig_set_outerRadius(st, 44);
    Konva_StarConfig_set_fill(st, Konva_string_or_CanvasGradient_of("#737373"));
    Konva_StarConfig_set_stroke(st, Konva_string_or_CanvasGradient_of("#111111"));
    Konva_StarConfig_set_strokeWidth(st, 2);
    JsArray_Konva_Node_push(children, AS_Konva_Node(Konva_Star_new(st)));
    add_caption(children, 62, 325, "Star (5pt)");

    Konva_RingConfig rc = Konva_RingConfig_create();
    Konva_RingConfig_set_x(rc, 250);
    Konva_RingConfig_set_y(rc, 268);
    Konva_RingConfig_set_innerRadius(rc, 22);
    Konva_RingConfig_set_outerRadius(rc, 42);
    Konva_RingConfig_set_fill(rc, Konva_string_or_CanvasGradient_of("#d4d4d4"));
    Konva_RingConfig_set_stroke(rc, Konva_string_or_CanvasGradient_of("#111111"));
    Konva_RingConfig_set_strokeWidth(rc, 2);
    JsArray_Konva_Node_push(children, AS_Konva_Node(Konva_Ring_new(rc)));
    add_caption(children, 236, 325, "Ring");

    Konva_WedgeConfig wc = Konva_WedgeConfig_create();
    Konva_WedgeConfig_set_x(wc, 415);
    Konva_WedgeConfig_set_y(wc, 268);
    Konva_WedgeConfig_set_radius(wc, 44);
    Konva_WedgeConfig_set_angle(wc, 270);
    Konva_WedgeConfig_set_rotation(wc, -90);
    Konva_WedgeConfig_set_fill(wc, Konva_string_or_CanvasGradient_of("#111111"));
    Konva_WedgeConfig_set_stroke(wc, Konva_string_or_CanvasGradient_of("#555555"));
    Konva_WedgeConfig_set_strokeWidth(wc, 1);
    JsArray_Konva_Node_push(children, AS_Konva_Node(Konva_Wedge_new(wc)));
    add_caption(children, 382, 325, "Wedge 270\xC2\xB0");

    Konva_RectConfig tf = Konva_RectConfig_create();
    Konva_RectConfig_set_x(tf, 500);
    Konva_RectConfig_set_y(tf, 224);
    Konva_RectConfig_set_width(tf, 150);
    Konva_RectConfig_set_height(tf, 88);
    Konva_RectConfig_set_fill(tf, Konva_string_or_CanvasGradient_of("#f7f7f7"));
    Konva_RectConfig_set_stroke(tf, Konva_string_or_CanvasGradient_of("#111111"));
    Konva_RectConfig_set_strokeWidth(tf, 1.5);
    JsArray_Konva_Node_push(children, AS_Konva_Node(Konva_Rect_new(tf)));

    Konva_TextConfig txt = Konva_TextConfig_create();
    Konva_TextConfig_set_x(txt, 514);
    Konva_TextConfig_set_y(txt, 240);
    Konva_TextConfig_set_text(txt, "KONVA 2D\nMonochrome\nScene Graph");
    Konva_TextConfig_set_fontSize(txt, 13);
    Konva_TextConfig_set_fontFamily(txt, "monospace");
    Konva_TextConfig_set_fill(txt, Konva_string_or_CanvasGradient_of("#111111"));
    JsArray_Konva_Node_push(children, AS_Konva_Node(Konva_Text_new(txt)));
    add_caption(children, 558, 325, "Text");

    Konva_Layer_add(layer, children);
    Konva_Layer_draw(layer);

    char meta[128];
    snprintf(meta, sizeof meta, "Stage: 680x400 \xC2\xB7 7 primitive types \xC2\xB7 %d layer children",
        (int)JsArray_Konva_Node_length(Konva_Layer_getChildren(layer)));
    HTMLElement_set_textContent(Document_getElementById(get_document(), "meta"), meta);
    js_scope_close();
}
