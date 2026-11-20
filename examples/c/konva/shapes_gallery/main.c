#include "dom/dom.h"
#include "konva/konva.h"
#include <stdio.h>

static void add_caption(JsArray_Node_2 list, double x, double y, const char* caption) {
    TextConfig tc = TextConfig_create();
    TextConfig_set_x(tc, x);
    TextConfig_set_y(tc, y);
    TextConfig_set_text(tc, caption);
    TextConfig_set_fontSize(tc, 11);
    TextConfig_set_fontFamily(tc, "monospace");
    TextConfig_set_fill(tc, string_or_CanvasGradient_of("#555555"));
    JsArray_Node_2_push(list, AS_Node_2(Text_2_new(tc)));
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
    JsArray_Node_2 children = JsArray_Node_2_new();

    RectConfig bg = RectConfig_create();
    RectConfig_set_x(bg, 0);
    RectConfig_set_y(bg, 0);
    RectConfig_set_width(bg, 680);
    RectConfig_set_height(bg, 400);
    RectConfig_set_fill(bg, string_or_CanvasGradient_of("#ffffff"));
    JsArray_Node_2_push(children, AS_Node_2(Rect_new(bg)));

    // Row 1: Rounded filled Rect, dashed Rect, Circle, Hexagon.
    RectConfig r1 = RectConfig_create();
    RectConfig_set_x(r1, 40);
    RectConfig_set_y(r1, 55);
    RectConfig_set_width(r1, 110);
    RectConfig_set_height(r1, 76);
    RectConfig_set_cornerRadius(r1, number_or_number_array_of(8));
    RectConfig_set_fill(r1, string_or_CanvasGradient_of("#111111"));
    JsArray_Node_2_push(children, AS_Node_2(Rect_new(r1)));
    add_caption(children, 40, 142, "Rect (filled)");

    RectConfig r2 = RectConfig_create();
    RectConfig_set_x(r2, 195);
    RectConfig_set_y(r2, 55);
    RectConfig_set_width(r2, 110);
    RectConfig_set_height(r2, 76);
    RectConfig_set_fill(r2, string_or_CanvasGradient_of("#ffffff"));
    RectConfig_set_stroke(r2, string_or_CanvasGradient_of("#111111"));
    RectConfig_set_strokeWidth(r2, 2);
    RectConfig_set_dash(r2, JsArray_double_from((double[]){ 6.0, 4.0 }, 2));
    JsArray_Node_2_push(children, AS_Node_2(Rect_new(r2)));
    add_caption(children, 195, 142, "Rect (dashed)");

    CircleConfig cc = CircleConfig_create();
    CircleConfig_set_x(cc, 415);
    CircleConfig_set_y(cc, 93);
    CircleConfig_set_radius(cc, 40);
    CircleConfig_set_fill(cc, string_or_CanvasGradient_of("#404040"));
    CircleConfig_set_stroke(cc, string_or_CanvasGradient_of("#111111"));
    CircleConfig_set_strokeWidth(cc, 2);
    JsArray_Node_2_push(children, AS_Node_2(Circle_new(cc)));
    add_caption(children, 395, 142, "Circle");

    RegularPolygonConfig pc = RegularPolygonConfig_create();
    RegularPolygonConfig_set_x(pc, 575);
    RegularPolygonConfig_set_y(pc, 93);
    RegularPolygonConfig_set_sides(pc, 6);
    RegularPolygonConfig_set_radius(pc, 42);
    RegularPolygonConfig_set_fill(pc, string_or_CanvasGradient_of("#efefef"));
    RegularPolygonConfig_set_stroke(pc, string_or_CanvasGradient_of("#111111"));
    RegularPolygonConfig_set_strokeWidth(pc, 2);
    JsArray_Node_2_push(children, AS_Node_2(RegularPolygon_new(pc)));
    add_caption(children, 548, 142, "Hexagon");

    // Row 2: Star, Ring, Wedge, Framed Text.
    StarConfig st = StarConfig_create();
    StarConfig_set_x(st, 95);
    StarConfig_set_y(st, 268);
    StarConfig_set_numPoints(st, 5);
    StarConfig_set_innerRadius(st, 19);
    StarConfig_set_outerRadius(st, 44);
    StarConfig_set_fill(st, string_or_CanvasGradient_of("#737373"));
    StarConfig_set_stroke(st, string_or_CanvasGradient_of("#111111"));
    StarConfig_set_strokeWidth(st, 2);
    JsArray_Node_2_push(children, AS_Node_2(Star_new(st)));
    add_caption(children, 62, 325, "Star (5pt)");

    RingConfig rc = RingConfig_create();
    RingConfig_set_x(rc, 250);
    RingConfig_set_y(rc, 268);
    RingConfig_set_innerRadius(rc, 22);
    RingConfig_set_outerRadius(rc, 42);
    RingConfig_set_fill(rc, string_or_CanvasGradient_of("#d4d4d4"));
    RingConfig_set_stroke(rc, string_or_CanvasGradient_of("#111111"));
    RingConfig_set_strokeWidth(rc, 2);
    JsArray_Node_2_push(children, AS_Node_2(Ring_new(rc)));
    add_caption(children, 236, 325, "Ring");

    WedgeConfig wc = WedgeConfig_create();
    WedgeConfig_set_x(wc, 415);
    WedgeConfig_set_y(wc, 268);
    WedgeConfig_set_radius(wc, 44);
    WedgeConfig_set_angle(wc, 270);
    WedgeConfig_set_rotation(wc, -90);
    WedgeConfig_set_fill(wc, string_or_CanvasGradient_of("#111111"));
    WedgeConfig_set_stroke(wc, string_or_CanvasGradient_of("#555555"));
    WedgeConfig_set_strokeWidth(wc, 1);
    JsArray_Node_2_push(children, AS_Node_2(Wedge_new(wc)));
    add_caption(children, 382, 325, "Wedge 270\xC2\xB0");

    RectConfig tf = RectConfig_create();
    RectConfig_set_x(tf, 500);
    RectConfig_set_y(tf, 224);
    RectConfig_set_width(tf, 150);
    RectConfig_set_height(tf, 88);
    RectConfig_set_fill(tf, string_or_CanvasGradient_of("#f7f7f7"));
    RectConfig_set_stroke(tf, string_or_CanvasGradient_of("#111111"));
    RectConfig_set_strokeWidth(tf, 1.5);
    JsArray_Node_2_push(children, AS_Node_2(Rect_new(tf)));

    TextConfig txt = TextConfig_create();
    TextConfig_set_x(txt, 514);
    TextConfig_set_y(txt, 240);
    TextConfig_set_text(txt, "KONVA 2D\nMonochrome\nScene Graph");
    TextConfig_set_fontSize(txt, 13);
    TextConfig_set_fontFamily(txt, "monospace");
    TextConfig_set_fill(txt, string_or_CanvasGradient_of("#111111"));
    JsArray_Node_2_push(children, AS_Node_2(Text_2_new(txt)));
    add_caption(children, 558, 325, "Text");

    Layer_add(layer, children);
    Layer_draw(layer);

    char meta[128];
    snprintf(meta, sizeof meta, "Stage: 680x400 \xC2\xB7 7 primitive types \xC2\xB7 %d layer children",
        (int)JsArray_Node_2_length(Layer_getChildren(layer)));
    HTMLElement_set_textContent(Document_getElementById(get_document(), "meta"), meta);
    js_scope_close();
}
