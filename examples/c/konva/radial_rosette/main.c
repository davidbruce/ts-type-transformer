#include "dom/dom.h"
#include "konva/konva.h"
#include <math.h>
#include <stdio.h>

static const double PI = 3.14159265358979323846;

__attribute__((export_name("start"))) void start(void) {
    js_scope();

    Konva_StageConfig sc = Konva_StageConfig_create(
        .container = "stage", .width = 680, .height = 400
    );
    Konva_Stage stage = Konva_Stage_new(sc);

    Konva_Layer layer = Konva_Layer_new();
    Konva_Stage_add(stage, layer);
    JsArray_Konva_Node items = JsArray_Konva_Node_new();

    Konva_RectConfig bg = Konva_RectConfig_create(
        .x = 0, .y = 0, .width = 680, .height = 400, .fill = "#ffffff"
    );
    JsArray_Konva_Node_push(items, AS_Konva_Node(Konva_Rect_new(bg)));

    const double cx = 340.0;
    const double cy = 200.0;

    const double ring_r[] = { 160.0, 125.0, 90.0, 52.0 };
    const char* ring_stroke[] = { "#d4d4d4", "#a3a3a3", "#555555", "#111111" };
    const double ring_sw[] = { 1.0, 1.0, 1.0, 1.5 };
    for (int i = 0; i < 4; i++) {
        Konva_CircleConfig cc = Konva_CircleConfig_create(
            .x = cx, .y = cy, .radius = ring_r[i], .stroke = ring_stroke[i],
            .strokeWidth = ring_sw[i]
        );
        if (i == 3) Konva_CircleConfig_set_fill(cc, Konva_string_or_CanvasGradient_of("#f7f7f7"));
        JsArray_Konva_Node_push(items, AS_Konva_Node(Konva_Circle_new(cc)));
    }

    for (int i = 0; i < 12; i++) {
        double a = (double)i * PI / 6.0;
        double pts[] = { cx + 36.0 * cos(a), cy + 36.0 * sin(a), cx + 160.0 * cos(a), cy + 160.0 * sin(a) };
        Konva_LineConfig lc = Konva_LineConfig_create(
            .points_array = JsArray_double_from(pts, 4),
            .stroke = "#d4d4d4", .strokeWidth = 1
        );
        JsArray_Konva_Node_push(items, AS_Konva_Node(Konva_Line_new(lc)));
    }

    for (int i = 0; i < 24; i++) {
        double a = (double)i * PI / 12.0;
        double r0 = (i % 2 == 0) ? 148.0 : 154.0;
        double pts[] = { cx + r0 * cos(a), cy + r0 * sin(a), cx + 166.0 * cos(a), cy + 166.0 * sin(a) };
        Konva_LineConfig lc = Konva_LineConfig_create(
            .points_array = JsArray_double_from(pts, 4),
            .stroke = "#111111", .strokeWidth = i % 2 == 0 ? 2.0 : 1.0
        );
        JsArray_Konva_Node_push(items, AS_Konva_Node(Konva_Line_new(lc)));
    }

    for (int i = 0; i < 8; i++) {
        Konva_RectConfig pc = Konva_RectConfig_create(
            .x = cx, .y = cy, .width = 18, .height = 52, .offsetX = 9, .offsetY = 108, .rotation = (double)i * 45.0,
            .fill = i % 2 == 0 ? "#111111" : "#efefef",
            .stroke = "#111111", .strokeWidth = 1.5
        );
        JsArray_Konva_Node_push(items, AS_Konva_Node(Konva_Rect_new(pc)));
    }

    for (int i = 0; i < 12; i++) {
        double a = (double)i * PI / 6.0;
        Konva_CircleConfig oc = Konva_CircleConfig_create(
            .x = cx + 125.0 * cos(a), .y = cy + 125.0 * sin(a), .radius = 5,
            .fill = i % 2 == 0 ? "#111111" : "#ffffff",
            .stroke = "#111111", .strokeWidth = 1.5
        );
        JsArray_Konva_Node_push(items, AS_Konva_Node(Konva_Circle_new(oc)));
    }

    Konva_RegularPolygonConfig cp = Konva_RegularPolygonConfig_create(
        .x = cx, .y = cy, .sides = 8, .radius = 26, .fill = "#111111"
    );
    JsArray_Konva_Node_push(items, AS_Konva_Node(Konva_RegularPolygon_new(cp)));

    Konva_CircleConfig cd = Konva_CircleConfig_create(
        .x = cx, .y = cy, .radius = 9, .fill = "#ffffff"
    );
    JsArray_Konva_Node_push(items, AS_Konva_Node(Konva_Circle_new(cd)));

    const double lx[] = { 516, 332, 140, 328 };
    const double ly[] = { 194, 376, 194, 14 };
    const char* labels[] = { "0\xC2\xB0", "90\xC2\xB0", "180\xC2\xB0", "270\xC2\xB0" };
    for (int i = 0; i < 4; i++) {
        Konva_TextConfig tc = Konva_TextConfig_create(
            .x = lx[i], .y = ly[i], .text = labels[i], .fontSize = 11, .fontFamily = "monospace",
            .fill = "#555555"
        );
        JsArray_Konva_Node_push(items, AS_Konva_Node(Konva_Text_new(tc)));
    }

    Konva_Layer_add(layer, items);
    Konva_Layer_draw(layer);

    char meta[128];
    snprintf(meta, sizeof meta, "Stage: 680x400 \xC2\xB7 Procedural rosette \xC2\xB7 %d shapes on layer",
        (int)JsArray_Konva_Node_length(Konva_Layer_getChildren(layer)));
    HTMLElement_set_textContent(Document_getElementById(get_document(), "meta"), meta);
}
