#include "dom/dom.h"
#include "konva/konva.h"
#include <stdio.h>

static Konva_Group make_stage_box(double x, const char* title, const char* sub, const char* fill, const char* ink) {
    Konva_Group g = Konva_Group_new(Konva_GroupConfig_create(.x = x, .y = 95));

    Konva_RectConfig rc = Konva_RectConfig_create(
        .width = 126, .height = 68, .cornerRadius = 3,
        .fill = fill, .stroke = "#111111",
        .strokeWidth = 2
    );

    Konva_TextConfig t1 = Konva_TextConfig_create(
        .x = 12, .y = 16, .text = title, .fontSize = 12, .fontStyle = "bold", .fontFamily = "monospace",
        .fill = ink
    );

    Konva_TextConfig t2 = Konva_TextConfig_create(
        .x = 12, .y = 38, .text = sub, .fontSize = 11, .fontFamily = "monospace",
        .fill = ink
    );

    Konva_Group_add(g, Konva_Rect_new(rc), Konva_Text_new(t1), Konva_Text_new(t2));
    return g;
}

static Konva_Arrow make_arrow(double x1, double y1, double x2, double y2) {
    Konva_ArrowConfig ac = Konva_ArrowConfig_create(
        .points = js_numbers(x1, y1, x2, y2), .pointerLength = 8,
        .pointerWidth = 8, .fill = "#111111",
        .stroke = "#111111", .strokeWidth = 2
    );
    return Konva_Arrow_new(ac);
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

    Konva_LineConfig fbc = Konva_LineConfig_create(
        .points = js_numbers(589, 163, 589, 212, 91, 212, 91, 168),
        .stroke = "#737373", .strokeWidth = 1.5,
        .dash = js_numbers(6.0, 4.0)
    );

    Konva_RectConfig bar = Konva_RectConfig_create(
        .x = 28, .y = 255, .width = 624, .height = 58, .cornerRadius = 3,
        .fill = "#f7f7f7", .stroke = "#111111",
        .strokeWidth = 1.5
    );

    Konva_TextConfig btxt = Konva_TextConfig_create(
        .x = 180, .y = 278, .text = "Shared DOM & Package Handle Runtime", .fontSize = 12, .fontFamily = "monospace",
        .fill = "#111111"
    );

    Konva_Layer_add(layer,
        Konva_Rect_new(bg),
        make_stage_box(28, "01 PARSE", ".d.ts AST", "#ffffff", "#111111"),
        make_stage_box(194, "02 LOWER", "Typed IR", "#f7f7f7", "#111111"),
        make_stage_box(360, "03 EMIT", "6 Targets", "#efefef", "#111111"),
        make_stage_box(526, "04 RUN", "WASM / JS", "#111111", "#ffffff"),
        make_arrow(154, 129, 190, 129),
        make_arrow(320, 129, 356, 129),
        make_arrow(486, 129, 522, 129),
        make_arrow(257, 163, 257, 251),
        make_arrow(423, 163, 423, 251),
        Konva_Line_new(fbc), Konva_Rect_new(bar), Konva_Text_new(btxt));
    Konva_Layer_draw(layer);

    int rects = (int)JsArray_Konva_Node_length(Konva_Layer_find(layer, js_string("Rect")));
    int arrows = (int)JsArray_Konva_Node_length(Konva_Layer_find(layer, js_string("Arrow")));
    int jlen = (int)JsString_len(Konva_Layer_toJSON(layer));

    char meta[160];
    snprintf(meta, sizeof meta, "Pipeline diagram \xC2\xB7 %d rects \xC2\xB7 %d arrows \xC2\xB7 JSON %d bytes", rects, arrows, jlen);
    HTMLElement_set_textContent(Document_getElementById(get_document(), "meta"), meta);
}
