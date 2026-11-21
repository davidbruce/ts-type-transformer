#include "dom/dom.h"
#include "konva/konva.h"
#include <stdio.h>

static void add_stage_box(JsArray_Konva_Node list, double x, const char* title, const char* sub, const char* fill, const char* ink) {
    Konva_GroupConfig gc = Konva_GroupConfig_create();
    Konva_GroupConfig_set_x(gc, x);
    Konva_GroupConfig_set_y(gc, 95);
    Konva_Group g = Konva_Group_new(gc);
    JsArray_Konva_Node ch = JsArray_Konva_Node_new();

    Konva_RectConfig rc = Konva_RectConfig_create();
    Konva_RectConfig_set_width(rc, 126);
    Konva_RectConfig_set_height(rc, 68);
    Konva_RectConfig_set_cornerRadius(rc, Konva_number_or_number_array_of(3));
    Konva_RectConfig_set_fill(rc, Konva_string_or_CanvasGradient_of(fill));
    Konva_RectConfig_set_stroke(rc, Konva_string_or_CanvasGradient_of("#111111"));
    Konva_RectConfig_set_strokeWidth(rc, 2);
    JsArray_Konva_Node_push(ch, AS_Konva_Node(Konva_Rect_new(rc)));

    Konva_TextConfig t1 = Konva_TextConfig_create();
    Konva_TextConfig_set_x(t1, 12);
    Konva_TextConfig_set_y(t1, 16);
    Konva_TextConfig_set_text(t1, title);
    Konva_TextConfig_set_fontSize(t1, 12);
    Konva_TextConfig_set_fontStyle(t1, "bold");
    Konva_TextConfig_set_fontFamily(t1, "monospace");
    Konva_TextConfig_set_fill(t1, Konva_string_or_CanvasGradient_of(ink));
    JsArray_Konva_Node_push(ch, AS_Konva_Node(Konva_Text_new(t1)));

    Konva_TextConfig t2 = Konva_TextConfig_create();
    Konva_TextConfig_set_x(t2, 12);
    Konva_TextConfig_set_y(t2, 38);
    Konva_TextConfig_set_text(t2, sub);
    Konva_TextConfig_set_fontSize(t2, 11);
    Konva_TextConfig_set_fontFamily(t2, "monospace");
    Konva_TextConfig_set_fill(t2, Konva_string_or_CanvasGradient_of(ink));
    JsArray_Konva_Node_push(ch, AS_Konva_Node(Konva_Text_new(t2)));

    Konva_Group_add(g, ch);
    JsArray_Konva_Node_push(list, AS_Konva_Node(g));
}

static void add_arrow(JsArray_Konva_Node list, double x1, double y1, double x2, double y2) {
    Konva_ArrowConfig ac = Konva_ArrowConfig_create();
    Konva_ArrowConfig_set_points(ac, Konva_LinePoints_of(JsArray_double_from((double[]){ x1, y1, x2, y2 }, 4)));
    Konva_ArrowConfig_set_pointerLength(ac, 8);
    Konva_ArrowConfig_set_pointerWidth(ac, 8);
    Konva_ArrowConfig_set_fill(ac, Konva_string_or_CanvasGradient_of("#111111"));
    Konva_ArrowConfig_set_stroke(ac, Konva_string_or_CanvasGradient_of("#111111"));
    Konva_ArrowConfig_set_strokeWidth(ac, 2);
    JsArray_Konva_Node_push(list, AS_Konva_Node(Konva_Arrow_new(ac)));
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
    JsArray_Konva_Node items = JsArray_Konva_Node_new();

    Konva_RectConfig bg = Konva_RectConfig_create();
    Konva_RectConfig_set_x(bg, 0);
    Konva_RectConfig_set_y(bg, 0);
    Konva_RectConfig_set_width(bg, 680);
    Konva_RectConfig_set_height(bg, 400);
    Konva_RectConfig_set_fill(bg, Konva_string_or_CanvasGradient_of("#ffffff"));
    JsArray_Konva_Node_push(items, AS_Konva_Node(Konva_Rect_new(bg)));

    add_stage_box(items, 28, "01 PARSE", ".d.ts AST", "#ffffff", "#111111");
    add_stage_box(items, 194, "02 LOWER", "Typed IR", "#f7f7f7", "#111111");
    add_stage_box(items, 360, "03 EMIT", "6 Targets", "#efefef", "#111111");
    add_stage_box(items, 526, "04 RUN", "WASM / JS", "#111111", "#ffffff");

    add_arrow(items, 154, 129, 190, 129);
    add_arrow(items, 320, 129, 356, 129);
    add_arrow(items, 486, 129, 522, 129);
    add_arrow(items, 257, 163, 257, 251);
    add_arrow(items, 423, 163, 423, 251);

    Konva_LineConfig fbc = Konva_LineConfig_create();
    Konva_LineConfig_set_points(fbc, Konva_LinePoints_of(JsArray_double_from((double[]){ 589, 163, 589, 212, 91, 212, 91, 168 }, 8)));
    Konva_LineConfig_set_stroke(fbc, Konva_string_or_CanvasGradient_of("#737373"));
    Konva_LineConfig_set_strokeWidth(fbc, 1.5);
    Konva_LineConfig_set_dash(fbc, JsArray_double_from((double[]){ 6.0, 4.0 }, 2));
    JsArray_Konva_Node_push(items, AS_Konva_Node(Konva_Line_new(fbc)));

    Konva_RectConfig bar = Konva_RectConfig_create();
    Konva_RectConfig_set_x(bar, 28);
    Konva_RectConfig_set_y(bar, 255);
    Konva_RectConfig_set_width(bar, 624);
    Konva_RectConfig_set_height(bar, 58);
    Konva_RectConfig_set_cornerRadius(bar, Konva_number_or_number_array_of(3));
    Konva_RectConfig_set_fill(bar, Konva_string_or_CanvasGradient_of("#f7f7f7"));
    Konva_RectConfig_set_stroke(bar, Konva_string_or_CanvasGradient_of("#111111"));
    Konva_RectConfig_set_strokeWidth(bar, 1.5);
    JsArray_Konva_Node_push(items, AS_Konva_Node(Konva_Rect_new(bar)));

    Konva_TextConfig btxt = Konva_TextConfig_create();
    Konva_TextConfig_set_x(btxt, 180);
    Konva_TextConfig_set_y(btxt, 278);
    Konva_TextConfig_set_text(btxt, "Shared DOM & Package Handle Runtime");
    Konva_TextConfig_set_fontSize(btxt, 12);
    Konva_TextConfig_set_fontFamily(btxt, "monospace");
    Konva_TextConfig_set_fill(btxt, Konva_string_or_CanvasGradient_of("#111111"));
    JsArray_Konva_Node_push(items, AS_Konva_Node(Konva_Text_new(btxt)));

    Konva_Layer_add(layer, items);
    Konva_Layer_draw(layer);

    int rects = (int)JsArray_Konva_Node_length(Konva_Layer_find(layer, js_string("Rect")));
    int arrows = (int)JsArray_Konva_Node_length(Konva_Layer_find(layer, js_string("Arrow")));
    int jlen = (int)JsString_len(Konva_Layer_toJSON(layer));

    char meta[160];
    snprintf(meta, sizeof meta, "Pipeline diagram \xC2\xB7 %d rects \xC2\xB7 %d arrows \xC2\xB7 JSON %d bytes", rects, arrows, jlen);
    HTMLElement_set_textContent(Document_getElementById(get_document(), "meta"), meta);
    js_scope_close();
}
