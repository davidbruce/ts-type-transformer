#include "dom/dom.h"
#include "konva/konva.h"
#include <math.h>
#include <stdio.h>

static const double PI = 3.14159265358979323846;

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();

    StageConfig sc = StageConfig_create();
    StageConfig_set_container(sc, HTMLDivElement_or_string_of("stage"));
    StageConfig_set_width(sc, 680);
    StageConfig_set_height(sc, 400);
    Stage stage = Stage_new(sc);

    Layer layer = Layer_new();
    Stage_add(stage, layer);
    JsArray_Node_2 items = JsArray_Node_2_new();

    RectConfig bg = RectConfig_create();
    RectConfig_set_x(bg, 0);
    RectConfig_set_y(bg, 0);
    RectConfig_set_width(bg, 680);
    RectConfig_set_height(bg, 400);
    RectConfig_set_fill(bg, string_or_CanvasGradient_of("#ffffff"));
    JsArray_Node_2_push(items, AS_Node_2(Rect_new(bg)));

    const double cx = 340.0;
    const double cy = 200.0;

    const double ring_r[] = { 160.0, 125.0, 90.0, 52.0 };
    const char* ring_stroke[] = { "#d4d4d4", "#a3a3a3", "#555555", "#111111" };
    const double ring_sw[] = { 1.0, 1.0, 1.0, 1.5 };
    for (int i = 0; i < 4; i++) {
        CircleConfig cc = CircleConfig_create();
        CircleConfig_set_x(cc, cx);
        CircleConfig_set_y(cc, cy);
        CircleConfig_set_radius(cc, ring_r[i]);
        CircleConfig_set_stroke(cc, string_or_CanvasGradient_of(ring_stroke[i]));
        CircleConfig_set_strokeWidth(cc, ring_sw[i]);
        if (i == 3) CircleConfig_set_fill(cc, string_or_CanvasGradient_of("#f7f7f7"));
        JsArray_Node_2_push(items, AS_Node_2(Circle_new(cc)));
    }

    for (int i = 0; i < 12; i++) {
        double a = (double)i * PI / 6.0;
        double pts[] = { cx + 36.0 * cos(a), cy + 36.0 * sin(a), cx + 160.0 * cos(a), cy + 160.0 * sin(a) };
        LineConfig lc = LineConfig_create();
        LineConfig_set_points(lc, LinePoints_of(JsArray_double_from(pts, 4)));
        LineConfig_set_stroke(lc, string_or_CanvasGradient_of("#d4d4d4"));
        LineConfig_set_strokeWidth(lc, 1);
        JsArray_Node_2_push(items, AS_Node_2(Line_new(lc)));
    }

    for (int i = 0; i < 24; i++) {
        double a = (double)i * PI / 12.0;
        double r0 = (i % 2 == 0) ? 148.0 : 154.0;
        double pts[] = { cx + r0 * cos(a), cy + r0 * sin(a), cx + 166.0 * cos(a), cy + 166.0 * sin(a) };
        LineConfig lc = LineConfig_create();
        LineConfig_set_points(lc, LinePoints_of(JsArray_double_from(pts, 4)));
        LineConfig_set_stroke(lc, string_or_CanvasGradient_of("#111111"));
        LineConfig_set_strokeWidth(lc, i % 2 == 0 ? 2.0 : 1.0);
        JsArray_Node_2_push(items, AS_Node_2(Line_new(lc)));
    }

    for (int i = 0; i < 8; i++) {
        RectConfig pc = RectConfig_create();
        RectConfig_set_x(pc, cx);
        RectConfig_set_y(pc, cy);
        RectConfig_set_width(pc, 18);
        RectConfig_set_height(pc, 52);
        RectConfig_set_offsetX(pc, 9);
        RectConfig_set_offsetY(pc, 108);
        RectConfig_set_rotation(pc, (double)i * 45.0);
        RectConfig_set_fill(pc, string_or_CanvasGradient_of(i % 2 == 0 ? "#111111" : "#efefef"));
        RectConfig_set_stroke(pc, string_or_CanvasGradient_of("#111111"));
        RectConfig_set_strokeWidth(pc, 1.5);
        JsArray_Node_2_push(items, AS_Node_2(Rect_new(pc)));
    }

    for (int i = 0; i < 12; i++) {
        double a = (double)i * PI / 6.0;
        CircleConfig oc = CircleConfig_create();
        CircleConfig_set_x(oc, cx + 125.0 * cos(a));
        CircleConfig_set_y(oc, cy + 125.0 * sin(a));
        CircleConfig_set_radius(oc, 5);
        CircleConfig_set_fill(oc, string_or_CanvasGradient_of(i % 2 == 0 ? "#111111" : "#ffffff"));
        CircleConfig_set_stroke(oc, string_or_CanvasGradient_of("#111111"));
        CircleConfig_set_strokeWidth(oc, 1.5);
        JsArray_Node_2_push(items, AS_Node_2(Circle_new(oc)));
    }

    RegularPolygonConfig cp = RegularPolygonConfig_create();
    RegularPolygonConfig_set_x(cp, cx);
    RegularPolygonConfig_set_y(cp, cy);
    RegularPolygonConfig_set_sides(cp, 8);
    RegularPolygonConfig_set_radius(cp, 26);
    RegularPolygonConfig_set_fill(cp, string_or_CanvasGradient_of("#111111"));
    JsArray_Node_2_push(items, AS_Node_2(RegularPolygon_new(cp)));

    CircleConfig cd = CircleConfig_create();
    CircleConfig_set_x(cd, cx);
    CircleConfig_set_y(cd, cy);
    CircleConfig_set_radius(cd, 9);
    CircleConfig_set_fill(cd, string_or_CanvasGradient_of("#ffffff"));
    JsArray_Node_2_push(items, AS_Node_2(Circle_new(cd)));

    const double lx[] = { 516, 332, 140, 328 };
    const double ly[] = { 194, 376, 194, 14 };
    const char* labels[] = { "0\xC2\xB0", "90\xC2\xB0", "180\xC2\xB0", "270\xC2\xB0" };
    for (int i = 0; i < 4; i++) {
        TextConfig tc = TextConfig_create();
        TextConfig_set_x(tc, lx[i]);
        TextConfig_set_y(tc, ly[i]);
        TextConfig_set_text(tc, labels[i]);
        TextConfig_set_fontSize(tc, 11);
        TextConfig_set_fontFamily(tc, "monospace");
        TextConfig_set_fill(tc, string_or_CanvasGradient_of("#555555"));
        JsArray_Node_2_push(items, AS_Node_2(Text_2_new(tc)));
    }

    Layer_add(layer, items);
    Layer_draw(layer);

    char meta[128];
    snprintf(meta, sizeof meta, "Stage: 680x400 \xC2\xB7 Procedural rosette \xC2\xB7 %d shapes on layer",
        (int)JsArray_Node_2_length(Layer_getChildren(layer)));
    HTMLElement_set_textContent(Document_getElementById(get_document(), "meta"), meta);
    js_scope_close();
}
