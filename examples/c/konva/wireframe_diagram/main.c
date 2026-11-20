#include "dom/dom.h"
#include "konva/konva.h"
#include <stdio.h>

static void add_stage_box(JsArray_Node_2 list, double x, const char* title, const char* sub, const char* fill, const char* ink) {
    GroupConfig gc = GroupConfig_create();
    GroupConfig_set_x(gc, x);
    GroupConfig_set_y(gc, 95);
    Group g = Group_new(gc);
    JsArray_Node_2 ch = JsArray_Node_2_new();

    RectConfig rc = RectConfig_create();
    RectConfig_set_width(rc, 126);
    RectConfig_set_height(rc, 68);
    RectConfig_set_cornerRadius(rc, number_or_number_array_of(3));
    RectConfig_set_fill(rc, string_or_CanvasGradient_of(fill));
    RectConfig_set_stroke(rc, string_or_CanvasGradient_of("#111111"));
    RectConfig_set_strokeWidth(rc, 2);
    JsArray_Node_2_push(ch, AS_Node_2(Rect_new(rc)));

    TextConfig t1 = TextConfig_create();
    TextConfig_set_x(t1, 12);
    TextConfig_set_y(t1, 16);
    TextConfig_set_text(t1, title);
    TextConfig_set_fontSize(t1, 12);
    TextConfig_set_fontStyle(t1, "bold");
    TextConfig_set_fontFamily(t1, "monospace");
    TextConfig_set_fill(t1, string_or_CanvasGradient_of(ink));
    JsArray_Node_2_push(ch, AS_Node_2(Text_2_new(t1)));

    TextConfig t2 = TextConfig_create();
    TextConfig_set_x(t2, 12);
    TextConfig_set_y(t2, 38);
    TextConfig_set_text(t2, sub);
    TextConfig_set_fontSize(t2, 11);
    TextConfig_set_fontFamily(t2, "monospace");
    TextConfig_set_fill(t2, string_or_CanvasGradient_of(ink));
    JsArray_Node_2_push(ch, AS_Node_2(Text_2_new(t2)));

    Group_add(g, ch);
    JsArray_Node_2_push(list, AS_Node_2(g));
}

static void add_arrow(JsArray_Node_2 list, double x1, double y1, double x2, double y2) {
    ArrowConfig ac = ArrowConfig_create();
    ArrowConfig_set_points(ac, LinePoints_of(JsArray_double_from((double[]){ x1, y1, x2, y2 }, 4)));
    ArrowConfig_set_pointerLength(ac, 8);
    ArrowConfig_set_pointerWidth(ac, 8);
    ArrowConfig_set_fill(ac, string_or_CanvasGradient_of("#111111"));
    ArrowConfig_set_stroke(ac, string_or_CanvasGradient_of("#111111"));
    ArrowConfig_set_strokeWidth(ac, 2);
    JsArray_Node_2_push(list, AS_Node_2(Arrow_new(ac)));
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
    JsArray_Node_2 items = JsArray_Node_2_new();

    RectConfig bg = RectConfig_create();
    RectConfig_set_x(bg, 0);
    RectConfig_set_y(bg, 0);
    RectConfig_set_width(bg, 680);
    RectConfig_set_height(bg, 400);
    RectConfig_set_fill(bg, string_or_CanvasGradient_of("#ffffff"));
    JsArray_Node_2_push(items, AS_Node_2(Rect_new(bg)));

    add_stage_box(items, 28, "01 PARSE", ".d.ts AST", "#ffffff", "#111111");
    add_stage_box(items, 194, "02 LOWER", "Typed IR", "#f7f7f7", "#111111");
    add_stage_box(items, 360, "03 EMIT", "6 Targets", "#efefef", "#111111");
    add_stage_box(items, 526, "04 RUN", "WASM / JS", "#111111", "#ffffff");

    add_arrow(items, 154, 129, 190, 129);
    add_arrow(items, 320, 129, 356, 129);
    add_arrow(items, 486, 129, 522, 129);
    add_arrow(items, 257, 163, 257, 251);
    add_arrow(items, 423, 163, 423, 251);

    LineConfig fbc = LineConfig_create();
    LineConfig_set_points(fbc, LinePoints_of(JsArray_double_from((double[]){ 589, 163, 589, 212, 91, 212, 91, 168 }, 8)));
    LineConfig_set_stroke(fbc, string_or_CanvasGradient_of("#737373"));
    LineConfig_set_strokeWidth(fbc, 1.5);
    LineConfig_set_dash(fbc, JsArray_double_from((double[]){ 6.0, 4.0 }, 2));
    JsArray_Node_2_push(items, AS_Node_2(Line_new(fbc)));

    RectConfig bar = RectConfig_create();
    RectConfig_set_x(bar, 28);
    RectConfig_set_y(bar, 255);
    RectConfig_set_width(bar, 624);
    RectConfig_set_height(bar, 58);
    RectConfig_set_cornerRadius(bar, number_or_number_array_of(3));
    RectConfig_set_fill(bar, string_or_CanvasGradient_of("#f7f7f7"));
    RectConfig_set_stroke(bar, string_or_CanvasGradient_of("#111111"));
    RectConfig_set_strokeWidth(bar, 1.5);
    JsArray_Node_2_push(items, AS_Node_2(Rect_new(bar)));

    TextConfig btxt = TextConfig_create();
    TextConfig_set_x(btxt, 180);
    TextConfig_set_y(btxt, 278);
    TextConfig_set_text(btxt, "Shared DOM & Package Handle Runtime");
    TextConfig_set_fontSize(btxt, 12);
    TextConfig_set_fontFamily(btxt, "monospace");
    TextConfig_set_fill(btxt, string_or_CanvasGradient_of("#111111"));
    JsArray_Node_2_push(items, AS_Node_2(Text_2_new(btxt)));

    Layer_add(layer, items);
    Layer_draw(layer);

    int rects = (int)JsArray_Node_2_length(Layer_find(layer, js_string("Rect")));
    int arrows = (int)JsArray_Node_2_length(Layer_find(layer, js_string("Arrow")));
    int jlen = (int)JsString_len(Layer_toJSON(layer));

    char meta[160];
    snprintf(meta, sizeof meta, "Pipeline diagram \xC2\xB7 %d rects \xC2\xB7 %d arrows \xC2\xB7 JSON %d bytes", rects, arrows, jlen);
    HTMLElement_set_textContent(Document_getElementById(get_document(), "meta"), meta);
    js_scope_close();
}
