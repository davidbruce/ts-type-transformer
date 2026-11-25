#include "dom/dom.h"
#include "konva/konva.h"
#include <stdio.h>

static void add_caption(JsArray_Konva_Node list, double x, double y, const char* caption) {
    Konva_TextConfig tc = Konva_TextConfig_create(
        .x = x, .y = y, .text = caption, .fontSize = 11, .fontFamily = "monospace",
        .fill = "#555555"
    );
    JsArray_Konva_Node_push(list, AS_Konva_Node(Konva_Text_new(tc)));
}

__attribute__((export_name("start"))) void start(void) {
    js_scope();

    Konva_StageConfig sc = Konva_StageConfig_create(
        .container = "stage", .width = 680, .height = 400
    );
    Konva_Stage stage = Konva_Stage_new(sc);

    Konva_Layer layer = Konva_Layer_new();
    Konva_Stage_add(stage, layer);
    JsArray_Konva_Node children = JsArray_Konva_Node_new();

    Konva_RectConfig bg = Konva_RectConfig_create(
        .x = 0, .y = 0, .width = 680, .height = 400, .fill = "#ffffff"
    );
    JsArray_Konva_Node_push(children, AS_Konva_Node(Konva_Rect_new(bg)));

    // Row 1: Rounded filled Rect, dashed Rect, Circle, Hexagon.
    Konva_RectConfig r1 = Konva_RectConfig_create(
        .x = 40, .y = 55, .width = 110, .height = 76, .cornerRadius = 8,
        .fill = "#111111"
    );
    JsArray_Konva_Node_push(children, AS_Konva_Node(Konva_Rect_new(r1)));
    add_caption(children, 40, 142, "Rect (filled)");

    Konva_RectConfig r2 = Konva_RectConfig_create(
        .x = 195, .y = 55, .width = 110, .height = 76, .fill = "#ffffff",
        .stroke = "#111111", .strokeWidth = 2,
        .dash = JsArray_double_from((double[]){ 6.0, 4.0 }, 2)
    );
    JsArray_Konva_Node_push(children, AS_Konva_Node(Konva_Rect_new(r2)));
    add_caption(children, 195, 142, "Rect (dashed)");

    Konva_CircleConfig cc = Konva_CircleConfig_create(
        .x = 415, .y = 93, .radius = 40, .fill = "#404040",
        .stroke = "#111111", .strokeWidth = 2
    );
    JsArray_Konva_Node_push(children, AS_Konva_Node(Konva_Circle_new(cc)));
    add_caption(children, 395, 142, "Circle");

    Konva_RegularPolygonConfig pc = Konva_RegularPolygonConfig_create(
        .x = 575, .y = 93, .sides = 6, .radius = 42, .fill = "#efefef",
        .stroke = "#111111", .strokeWidth = 2
    );
    JsArray_Konva_Node_push(children, AS_Konva_Node(Konva_RegularPolygon_new(pc)));
    add_caption(children, 548, 142, "Hexagon");

    // Row 2: Star, Ring, Wedge, Framed Text.
    Konva_StarConfig st = Konva_StarConfig_create(
        .x = 95, .y = 268, .numPoints = 5, .innerRadius = 19, .outerRadius = 44,
        .fill = "#737373", .stroke = "#111111",
        .strokeWidth = 2
    );
    JsArray_Konva_Node_push(children, AS_Konva_Node(Konva_Star_new(st)));
    add_caption(children, 62, 325, "Star (5pt)");

    Konva_RingConfig rc = Konva_RingConfig_create(
        .x = 250, .y = 268, .innerRadius = 22, .outerRadius = 42, .fill = "#d4d4d4",
        .stroke = "#111111", .strokeWidth = 2
    );
    JsArray_Konva_Node_push(children, AS_Konva_Node(Konva_Ring_new(rc)));
    add_caption(children, 236, 325, "Ring");

    Konva_WedgeConfig wc = Konva_WedgeConfig_create(
        .x = 415, .y = 268, .radius = 44, .angle = 270, .rotation = -90,
        .fill = "#111111", .stroke = "#555555",
        .strokeWidth = 1
    );
    JsArray_Konva_Node_push(children, AS_Konva_Node(Konva_Wedge_new(wc)));
    add_caption(children, 382, 325, "Wedge 270\xC2\xB0");

    Konva_RectConfig tf = Konva_RectConfig_create(
        .x = 500, .y = 224, .width = 150, .height = 88, .fill = "#f7f7f7",
        .stroke = "#111111", .strokeWidth = 1.5
    );
    JsArray_Konva_Node_push(children, AS_Konva_Node(Konva_Rect_new(tf)));

    Konva_TextConfig txt = Konva_TextConfig_create(
        .x = 514, .y = 240, .text = "KONVA 2D\nMonochrome\nScene Graph", .fontSize = 13, .fontFamily = "monospace",
        .fill = "#111111"
    );
    JsArray_Konva_Node_push(children, AS_Konva_Node(Konva_Text_new(txt)));
    add_caption(children, 558, 325, "Text");

    Konva_Layer_add(layer, children);
    Konva_Layer_draw(layer);

    char meta[128];
    snprintf(meta, sizeof meta, "Stage: 680x400 \xC2\xB7 7 primitive types \xC2\xB7 %d layer children",
        (int)JsArray_Konva_Node_length(Konva_Layer_getChildren(layer)));
    HTMLElement_set_textContent(Document_getElementById(get_document(), "meta"), meta);
}
