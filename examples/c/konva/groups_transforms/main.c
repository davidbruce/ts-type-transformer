#include "dom/dom.h"
#include "konva/konva.h"
#include <stdio.h>

static Konva_Line make_grid_line(double x1, double y1, double x2, double y2) {
    Konva_LineConfig lc = Konva_LineConfig_create(
        .points = Konva_LinePoints_of(JsArray_double_from((double[]){ x1, y1, x2, y2 }, 4)),
        .stroke = Konva_string_or_CanvasGradient_of("#e5e5e5"), .strokeWidth = 1
    );
    return Konva_Line_new(lc);
}

__attribute__((export_name("start"))) void start(void) {
    js_scope();

    Konva_StageConfig sc = Konva_StageConfig_create(
        .container = Konva_HTMLDivElement_or_string_of("stage"), .width = 680, .height = 400
    );
    Konva_Stage stage = Konva_Stage_new(sc);

    Konva_Layer layer = Konva_Layer_new();
    Konva_Stage_add(stage, layer);
    JsArray_Konva_Node top_items = JsArray_Konva_Node_new();

    Konva_RectConfig bg = Konva_RectConfig_create(
        .x = 0, .y = 0, .width = 680, .height = 400, .fill = Konva_string_or_CanvasGradient_of("#ffffff")
    );
    JsArray_Konva_Node_push(top_items, AS_Konva_Node(Konva_Rect_new(bg)));

    for (int x = 40; x <= 640; x += 40) {
        JsArray_Konva_Node_push(top_items, AS_Konva_Node(make_grid_line((double)x, 0, (double)x, 400)));
    }
    for (int y = 40; y <= 360; y += 40) {
        JsArray_Konva_Node_push(top_items, AS_Konva_Node(make_grid_line(0, (double)y, 680, (double)y)));
    }

    // Group A: unrotated base frame.
    Konva_Group group_a = Konva_Group_new(Konva_GroupConfig_create(.x = 40, .y = 55));
    JsArray_Konva_Node ga_ch = JsArray_Konva_Node_new();

    Konva_RectConfig gab = Konva_RectConfig_create(
        .width = 160, .height = 120, .fill = Konva_string_or_CanvasGradient_of("#f7f7f7"),
        .stroke = Konva_string_or_CanvasGradient_of("#111111"), .strokeWidth = 1.5
    );
    JsArray_Konva_Node_push(ga_ch, AS_Konva_Node(Konva_Rect_new(gab)));

    Konva_CircleConfig gad = Konva_CircleConfig_create(
        .x = 80, .y = 60, .radius = 24, .fill = Konva_string_or_CanvasGradient_of("#111111")
    );
    JsArray_Konva_Node_push(ga_ch, AS_Konva_Node(Konva_Circle_new(gad)));

    Konva_TextConfig gal = Konva_TextConfig_create(
        .x = 12, .y = 10, .text = "Group A \xC2\xB7 0\xC2\xB0", .fontSize = 11, .fontFamily = "monospace",
        .fill = Konva_string_or_CanvasGradient_of("#111111")
    );
    JsArray_Konva_Node_push(ga_ch, AS_Konva_Node(Konva_Text_new(gal)));
    Konva_Group_add(group_a, ga_ch);
    JsArray_Konva_Node_push(top_items, AS_Konva_Node(group_a));

    // Group B: rotated 24 deg.
    Konva_Group group_b = Konva_Group_new(Konva_GroupConfig_create(.x = 340, .y = 75, .rotation = 24));
    JsArray_Konva_Node gb_ch = JsArray_Konva_Node_new();

    Konva_RectConfig gbo = Konva_RectConfig_create(
        .width = 150, .height = 110, .fill = Konva_string_or_CanvasGradient_of("#efefef"),
        .stroke = Konva_string_or_CanvasGradient_of("#111111"), .strokeWidth = 2
    );
    JsArray_Konva_Node_push(gb_ch, AS_Konva_Node(Konva_Rect_new(gbo)));

    Konva_RectConfig gbi = Konva_RectConfig_create(
        .x = 22, .y = 22, .width = 106, .height = 66, .fill = Konva_string_or_CanvasGradient_of("#ffffff"),
        .stroke = Konva_string_or_CanvasGradient_of("#555555"), .strokeWidth = 1.5,
        .dash = JsArray_double_from((double[]){ 4.0, 3.0 }, 2)
    );
    JsArray_Konva_Node_push(gb_ch, AS_Konva_Node(Konva_Rect_new(gbi)));

    Konva_CircleConfig gbd = Konva_CircleConfig_create(
        .x = 75, .y = 55, .radius = 14, .fill = Konva_string_or_CanvasGradient_of("#404040")
    );
    JsArray_Konva_Node_push(gb_ch, AS_Konva_Node(Konva_Circle_new(gbd)));

    Konva_TextConfig gbl = Konva_TextConfig_create(
        .x = 10, .y = 8, .text = "Group B \xC2\xB7 24\xC2\xB0", .fontSize = 11, .fontFamily = "monospace",
        .fill = Konva_string_or_CanvasGradient_of("#111111")
    );
    JsArray_Konva_Node_push(gb_ch, AS_Konva_Node(Konva_Text_new(gbl)));
    Konva_Group_add(group_b, gb_ch);
    JsArray_Konva_Node_push(top_items, AS_Konva_Node(group_b));

    // Group C: scaled container with nested rotated child group.
    Konva_Group group_c = Konva_Group_new(Konva_GroupConfig_create(.x = 495, .y = 55, .scaleX = 1.15, .scaleY = 1.15));
    JsArray_Konva_Node gc_ch = JsArray_Konva_Node_new();

    Konva_RectConfig gco = Konva_RectConfig_create(
        .width = 130, .height = 110, .fill = Konva_string_or_CanvasGradient_of("#ffffff"),
        .stroke = Konva_string_or_CanvasGradient_of("#111111"), .strokeWidth = 2
    );
    JsArray_Konva_Node_push(gc_ch, AS_Konva_Node(Konva_Rect_new(gco)));

    Konva_TextConfig gcl = Konva_TextConfig_create(
        .x = 8, .y = 8, .text = "Group C \xC2\xB7 1.15x", .fontSize = 10, .fontFamily = "monospace",
        .fill = Konva_string_or_CanvasGradient_of("#111111")
    );
    JsArray_Konva_Node_push(gc_ch, AS_Konva_Node(Konva_Text_new(gcl)));

    Konva_Group inner_group = Konva_Group_new(Konva_GroupConfig_create(.x = 25, .y = 25, .rotation = 15));

    Konva_CircleConfig mc = Konva_CircleConfig_create(
        .x = 40, .y = 30, .radius = 16, .fill = Konva_string_or_CanvasGradient_of("#111111")
    );
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

    Konva_RectConfig strip = Konva_RectConfig_create(
        .x = 28, .y = 308, .width = 624, .height = 60, .fill = Konva_string_or_CanvasGradient_of("#f7f7f7"),
        .stroke = Konva_string_or_CanvasGradient_of("#111111"), .strokeWidth = 1.5
    );
    JsArray_Konva_Node_push(top_items, AS_Konva_Node(Konva_Rect_new(strip)));

    Konva_TextConfig sum = Konva_TextConfig_create(
        .x = 44, .y = 331, .text = strip_txt, .fontSize = 12, .fontFamily = "monospace",
        .fill = Konva_string_or_CanvasGradient_of("#111111")
    );
    JsArray_Konva_Node_push(top_items, AS_Konva_Node(Konva_Text_new(sum)));

    Konva_Layer_add(layer, top_items);
    Konva_Layer_draw(layer);

    char meta[192];
    snprintf(meta, sizeof meta, "Stage: 680x400 \xC2\xB7 Nested marker abs=(%d, %d) \xC2\xB7 Rotated Group B rect=%dx%d", ax, ay, bw, bh);
    HTMLElement_set_textContent(Document_getElementById(get_document(), "meta"), meta);
}
