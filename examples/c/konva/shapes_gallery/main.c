#include "dom/dom.h"
#include "konva/konva.h"
#include <stdio.h>

static void add_caption(JsArray_Konva_Node list, double x, double y, const char* caption) {
    Konva_TextConfig tc = Konva_TextConfig_create(
        .x = x, .y = y, .text = caption, .fontSize = 11, .fontFamily = "monospace",
        .fill = "#555555"
    );
    JsArray_Konva_Node_push(list, Konva_Text_new(tc));
}

__attribute__((export_name("start"))) void start(void) {
    js_scope();

    Konva_StageConfig sc = Konva_StageConfig_create(
        .container = "stage", .width = 680, .height = 400
    );
    Konva_Stage stage = Konva_Stage_new(sc);

    Konva_Layer layer = Konva_Layer_new();
    Konva_Stage_add(stage, layer);

    Konva_RectConfig bg = Konva_RectConfig_create(
        .x = 0, .y = 0, .width = 680, .height = 400, .fill = "#ffffff"
    );

    // Row 1: Rounded filled Rect, dashed Rect, Circle, Hexagon.
    Konva_RectConfig r1 = Konva_RectConfig_create(
        .x = 40, .y = 55, .width = 110, .height = 76, .cornerRadius = 8,
        .fill = "#111111"
    );

    Konva_RectConfig r2 = Konva_RectConfig_create(
        .x = 195, .y = 55, .width = 110, .height = 76, .fill = "#ffffff",
        .stroke = "#111111", .strokeWidth = 2,
        .dash = js_numbers(6.0, 4.0)
    );

    Konva_CircleConfig cc = Konva_CircleConfig_create(
        .x = 415, .y = 93, .radius = 40, .fill = "#404040",
        .stroke = "#111111", .strokeWidth = 2
    );

    Konva_RegularPolygonConfig pc = Konva_RegularPolygonConfig_create(
        .x = 575, .y = 93, .sides = 6, .radius = 42, .fill = "#efefef",
        .stroke = "#111111", .strokeWidth = 2
    );

    // Row 2: Star, Ring, Wedge, Framed Text.
    Konva_StarConfig st = Konva_StarConfig_create(
        .x = 95, .y = 268, .numPoints = 5, .innerRadius = 19, .outerRadius = 44,
        .fill = "#737373", .stroke = "#111111",
        .strokeWidth = 2
    );

    Konva_RingConfig rc = Konva_RingConfig_create(
        .x = 250, .y = 268, .innerRadius = 22, .outerRadius = 42, .fill = "#d4d4d4",
        .stroke = "#111111", .strokeWidth = 2
    );

    Konva_WedgeConfig wc = Konva_WedgeConfig_create(
        .x = 415, .y = 268, .radius = 44, .angle = 270, .rotation = -90,
        .fill = "#111111", .stroke = "#555555",
        .strokeWidth = 1
    );

    Konva_RectConfig tf = Konva_RectConfig_create(
        .x = 500, .y = 224, .width = 150, .height = 88, .fill = "#f7f7f7",
        .stroke = "#111111", .strokeWidth = 1.5
    );

    Konva_TextConfig txt = Konva_TextConfig_create(
        .x = 514, .y = 240, .text = "KONVA 2D\nMonochrome\nScene Graph", .fontSize = 13, .fontFamily = "monospace",
        .fill = "#111111"
    );

    JsArray_Konva_Node children = Konva_Node_list(
        Konva_Rect_new(bg), Konva_Rect_new(r1), Konva_Rect_new(r2), Konva_Circle_new(cc),
        Konva_RegularPolygon_new(pc), Konva_Star_new(st), Konva_Ring_new(rc),
        Konva_Wedge_new(wc), Konva_Rect_new(tf), Konva_Text_new(txt)
    );
    add_caption(children, 40, 142, "Rect (filled)");
    add_caption(children, 195, 142, "Rect (dashed)");
    add_caption(children, 395, 142, "Circle");
    add_caption(children, 548, 142, "Hexagon");
    add_caption(children, 62, 325, "Star (5pt)");
    add_caption(children, 236, 325, "Ring");
    add_caption(children, 382, 325, "Wedge 270\xC2\xB0");
    add_caption(children, 558, 325, "Text");

    Konva_Layer_add_list(layer, children);
    Konva_Layer_draw(layer);

    char meta[128];
    snprintf(meta, sizeof meta, "Stage: 680x400 \xC2\xB7 7 primitive types \xC2\xB7 %d layer children",
        (int)JsArray_Konva_Node_length(Konva_Layer_getChildren(layer)));
    HTMLElement_set_textContent(Document_getElementById(get_document(), "meta"), meta);
}
