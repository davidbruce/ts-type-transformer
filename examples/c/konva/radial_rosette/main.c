#include "dom/dom.h"
#include "konva/konva.h"
#include <math.h>
#include <stdio.h>

static const double PI = 3.14159265358979323846;

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();

    Konva_StageConfig sc = Konva_StageConfig_create();
    Konva_StageConfig_set_container(sc, Konva_HTMLDivElement_or_string_of("stage"));
    Konva_StageConfig_set_width(sc, 680);
    Konva_StageConfig_set_height(sc, 400);
    Konva_Stage stage = Konva_Stage_new(sc);

    Konva_Layer layer = Konva_Layer_new();
    Konva_Stage_add(stage, layer);
    JsArray_Konva_Node items = JsArray_Konva_Node_new();

    Konva_RectConfig bg = Konva_RectConfig_create();
    Konva_RectConfig_set_x(bg, 0);
    Konva_RectConfig_set_y(bg, 0);
    Konva_RectConfig_set_width(bg, 680);
    Konva_RectConfig_set_height(bg, 400);
    Konva_RectConfig_set_fill(bg, Konva_string_or_CanvasGradient_of("#ffffff"));
    JsArray_Konva_Node_push(items, AS_Konva_Node(Konva_Rect_new(bg)));

    const double cx = 340.0;
    const double cy = 200.0;

    const double ring_r[] = { 160.0, 125.0, 90.0, 52.0 };
    const char* ring_stroke[] = { "#d4d4d4", "#a3a3a3", "#555555", "#111111" };
    const double ring_sw[] = { 1.0, 1.0, 1.0, 1.5 };
    for (int i = 0; i < 4; i++) {
        Konva_CircleConfig cc = Konva_CircleConfig_create();
        Konva_CircleConfig_set_x(cc, cx);
        Konva_CircleConfig_set_y(cc, cy);
        Konva_CircleConfig_set_radius(cc, ring_r[i]);
        Konva_CircleConfig_set_stroke(cc, Konva_string_or_CanvasGradient_of(ring_stroke[i]));
        Konva_CircleConfig_set_strokeWidth(cc, ring_sw[i]);
        if (i == 3) Konva_CircleConfig_set_fill(cc, Konva_string_or_CanvasGradient_of("#f7f7f7"));
        JsArray_Konva_Node_push(items, AS_Konva_Node(Konva_Circle_new(cc)));
    }

    for (int i = 0; i < 12; i++) {
        double a = (double)i * PI / 6.0;
        double pts[] = { cx + 36.0 * cos(a), cy + 36.0 * sin(a), cx + 160.0 * cos(a), cy + 160.0 * sin(a) };
        Konva_LineConfig lc = Konva_LineConfig_create();
        Konva_LineConfig_set_points(lc, Konva_LinePoints_of(JsArray_double_from(pts, 4)));
        Konva_LineConfig_set_stroke(lc, Konva_string_or_CanvasGradient_of("#d4d4d4"));
        Konva_LineConfig_set_strokeWidth(lc, 1);
        JsArray_Konva_Node_push(items, AS_Konva_Node(Konva_Line_new(lc)));
    }

    for (int i = 0; i < 24; i++) {
        double a = (double)i * PI / 12.0;
        double r0 = (i % 2 == 0) ? 148.0 : 154.0;
        double pts[] = { cx + r0 * cos(a), cy + r0 * sin(a), cx + 166.0 * cos(a), cy + 166.0 * sin(a) };
        Konva_LineConfig lc = Konva_LineConfig_create();
        Konva_LineConfig_set_points(lc, Konva_LinePoints_of(JsArray_double_from(pts, 4)));
        Konva_LineConfig_set_stroke(lc, Konva_string_or_CanvasGradient_of("#111111"));
        Konva_LineConfig_set_strokeWidth(lc, i % 2 == 0 ? 2.0 : 1.0);
        JsArray_Konva_Node_push(items, AS_Konva_Node(Konva_Line_new(lc)));
    }

    for (int i = 0; i < 8; i++) {
        Konva_RectConfig pc = Konva_RectConfig_create();
        Konva_RectConfig_set_x(pc, cx);
        Konva_RectConfig_set_y(pc, cy);
        Konva_RectConfig_set_width(pc, 18);
        Konva_RectConfig_set_height(pc, 52);
        Konva_RectConfig_set_offsetX(pc, 9);
        Konva_RectConfig_set_offsetY(pc, 108);
        Konva_RectConfig_set_rotation(pc, (double)i * 45.0);
        Konva_RectConfig_set_fill(pc, Konva_string_or_CanvasGradient_of(i % 2 == 0 ? "#111111" : "#efefef"));
        Konva_RectConfig_set_stroke(pc, Konva_string_or_CanvasGradient_of("#111111"));
        Konva_RectConfig_set_strokeWidth(pc, 1.5);
        JsArray_Konva_Node_push(items, AS_Konva_Node(Konva_Rect_new(pc)));
    }

    for (int i = 0; i < 12; i++) {
        double a = (double)i * PI / 6.0;
        Konva_CircleConfig oc = Konva_CircleConfig_create();
        Konva_CircleConfig_set_x(oc, cx + 125.0 * cos(a));
        Konva_CircleConfig_set_y(oc, cy + 125.0 * sin(a));
        Konva_CircleConfig_set_radius(oc, 5);
        Konva_CircleConfig_set_fill(oc, Konva_string_or_CanvasGradient_of(i % 2 == 0 ? "#111111" : "#ffffff"));
        Konva_CircleConfig_set_stroke(oc, Konva_string_or_CanvasGradient_of("#111111"));
        Konva_CircleConfig_set_strokeWidth(oc, 1.5);
        JsArray_Konva_Node_push(items, AS_Konva_Node(Konva_Circle_new(oc)));
    }

    Konva_RegularPolygonConfig cp = Konva_RegularPolygonConfig_create();
    Konva_RegularPolygonConfig_set_x(cp, cx);
    Konva_RegularPolygonConfig_set_y(cp, cy);
    Konva_RegularPolygonConfig_set_sides(cp, 8);
    Konva_RegularPolygonConfig_set_radius(cp, 26);
    Konva_RegularPolygonConfig_set_fill(cp, Konva_string_or_CanvasGradient_of("#111111"));
    JsArray_Konva_Node_push(items, AS_Konva_Node(Konva_RegularPolygon_new(cp)));

    Konva_CircleConfig cd = Konva_CircleConfig_create();
    Konva_CircleConfig_set_x(cd, cx);
    Konva_CircleConfig_set_y(cd, cy);
    Konva_CircleConfig_set_radius(cd, 9);
    Konva_CircleConfig_set_fill(cd, Konva_string_or_CanvasGradient_of("#ffffff"));
    JsArray_Konva_Node_push(items, AS_Konva_Node(Konva_Circle_new(cd)));

    const double lx[] = { 516, 332, 140, 328 };
    const double ly[] = { 194, 376, 194, 14 };
    const char* labels[] = { "0\xC2\xB0", "90\xC2\xB0", "180\xC2\xB0", "270\xC2\xB0" };
    for (int i = 0; i < 4; i++) {
        Konva_TextConfig tc = Konva_TextConfig_create();
        Konva_TextConfig_set_x(tc, lx[i]);
        Konva_TextConfig_set_y(tc, ly[i]);
        Konva_TextConfig_set_text(tc, labels[i]);
        Konva_TextConfig_set_fontSize(tc, 11);
        Konva_TextConfig_set_fontFamily(tc, "monospace");
        Konva_TextConfig_set_fill(tc, Konva_string_or_CanvasGradient_of("#555555"));
        JsArray_Konva_Node_push(items, AS_Konva_Node(Konva_Text_new(tc)));
    }

    Konva_Layer_add(layer, items);
    Konva_Layer_draw(layer);

    char meta[128];
    snprintf(meta, sizeof meta, "Stage: 680x400 \xC2\xB7 Procedural rosette \xC2\xB7 %d shapes on layer",
        (int)JsArray_Konva_Node_length(Konva_Layer_getChildren(layer)));
    HTMLElement_set_textContent(Document_getElementById(get_document(), "meta"), meta);
    js_scope_close();
}
