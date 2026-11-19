#include "dom/dom.h"
#include "konva/konva.h"

static uint32_t append_cstr(char* dst, uint32_t pos, uint32_t cap, const char* src) {
    for (uint32_t i = 0; src[i] && pos + 1 < cap; i++) dst[pos++] = src[i];
    dst[pos] = '\0';
    return pos;
}

static uint32_t append_i32(char* dst, uint32_t pos, uint32_t cap, int32_t v) {
    char tmp[16];
    uint32_t len = 0;
    if (v == 0) tmp[len++] = '0';
    else while (v > 0 && len < sizeof(tmp)) { tmp[len++] = (char)('0' + (v % 10)); v /= 10; }
    for (uint32_t i = 0; i < len && pos + 1 < cap; i++) dst[pos++] = tmp[len - 1 - i];
    dst[pos] = '\0';
    return pos;
}

static void add_stage_box(JsArray_Node_2 list, double x, const char* title, const char* sub, const char* fill, const char* ink) {
    GroupConfig gc = GroupConfig_create();
    NodeConfig_set_x(gc, js_some_double(x));
    NodeConfig_set_y(gc, js_some_double(95));
    Group g = Group_new(gc);
    JsArray_Node_2 ch = JsArray_Node_2_new();

    RectConfig rc = RectConfig_create();
    NodeConfig_set_width(rc, js_some_double(126));
    NodeConfig_set_height(rc, js_some_double(68));
    RectConfig_set_cornerRadius(rc, number_or_number_array_of_number(3));
    ShapeConfig_set_fill(rc, string_or_CanvasGradient_of_string(JsString_from(fill)));
    ShapeConfig_set_stroke(rc, string_or_CanvasGradient_of_string(JsString_from("#111111")));
    ShapeConfig_set_strokeWidth(rc, js_some_double(2));
    JsArray_Node_2_push(ch, AS_Node_2(Rect_new(rc)));

    TextConfig t1 = TextConfig_create();
    NodeConfig_set_x(t1, js_some_double(12));
    NodeConfig_set_y(t1, js_some_double(16));
    TextConfig_set_text(t1, title);
    TextConfig_set_fontSize(t1, js_some_double(12));
    TextConfig_set_fontStyle(t1, "bold");
    TextConfig_set_fontFamily(t1, "monospace");
    ShapeConfig_set_fill(t1, string_or_CanvasGradient_of_string(JsString_from(ink)));
    JsArray_Node_2_push(ch, AS_Node_2(Text_2_new(t1)));

    TextConfig t2 = TextConfig_create();
    NodeConfig_set_x(t2, js_some_double(12));
    NodeConfig_set_y(t2, js_some_double(38));
    TextConfig_set_text(t2, sub);
    TextConfig_set_fontSize(t2, js_some_double(11));
    TextConfig_set_fontFamily(t2, "monospace");
    ShapeConfig_set_fill(t2, string_or_CanvasGradient_of_string(JsString_from(ink)));
    JsArray_Node_2_push(ch, AS_Node_2(Text_2_new(t2)));

    Container_add(g, ch);
    JsArray_Node_2_push(list, AS_Node_2(g));
}

static void add_arrow(JsArray_Node_2 list, double x1, double y1, double x2, double y2) {
    JsArray_double pts = JsArray_double_new();
    JsArray_double_push(pts, x1);
    JsArray_double_push(pts, y1);
    JsArray_double_push(pts, x2);
    JsArray_double_push(pts, y2);
    ArrowConfig ac = ArrowConfig_create();
    LineConfig_set_points(ac, LinePoints_of_array(pts));
    ArrowConfig_set_pointerLength(ac, js_some_double(8));
    ArrowConfig_set_pointerWidth(ac, js_some_double(8));
    ShapeConfig_set_fill(ac, string_or_CanvasGradient_of_string(JsString_from("#111111")));
    ShapeConfig_set_stroke(ac, string_or_CanvasGradient_of_string(JsString_from("#111111")));
    ShapeConfig_set_strokeWidth(ac, js_some_double(2));
    JsArray_Node_2_push(list, AS_Node_2(Arrow_new(ac)));
}

__attribute__((export_name("start"))) void start(void) {
    StageConfig sc = StageConfig_create();
    StageConfig_set_container(sc, HTMLDivElement_or_string_of(JsString_from("stage")));
    NodeConfig_set_width(sc, js_some_double(680));
    NodeConfig_set_height(sc, js_some_double(400));
    Stage stage = Stage_new(sc);

    Layer layer = Layer_new((LayerConfig){ 0 });
    Stage_add(stage, layer, JsArray_new());
    JsArray_Node_2 items = JsArray_Node_2_new();

    RectConfig bg = RectConfig_create();
    NodeConfig_set_x(bg, js_some_double(0));
    NodeConfig_set_y(bg, js_some_double(0));
    NodeConfig_set_width(bg, js_some_double(680));
    NodeConfig_set_height(bg, js_some_double(400));
    ShapeConfig_set_fill(bg, string_or_CanvasGradient_of_string(JsString_from("#ffffff")));
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

    JsArray_double fb_pts = JsArray_double_new();
    double raw_fb[] = { 589, 163, 589, 212, 91, 212, 91, 168 };
    for (int i = 0; i < 8; i++) JsArray_double_push(fb_pts, raw_fb[i]);
    JsArray_double dash = JsArray_double_new();
    JsArray_double_push(dash, 6.0);
    JsArray_double_push(dash, 4.0);
    LineConfig fbc = LineConfig_create();
    LineConfig_set_points(fbc, LinePoints_of_array(fb_pts));
    ShapeConfig_set_stroke(fbc, string_or_CanvasGradient_of_string(JsString_from("#737373")));
    ShapeConfig_set_strokeWidth(fbc, js_some_double(1.5));
    ShapeConfig_set_dash(fbc, dash);
    JsArray_Node_2_push(items, AS_Node_2(Line_new(fbc)));

    RectConfig bar = RectConfig_create();
    NodeConfig_set_x(bar, js_some_double(28));
    NodeConfig_set_y(bar, js_some_double(255));
    NodeConfig_set_width(bar, js_some_double(624));
    NodeConfig_set_height(bar, js_some_double(58));
    RectConfig_set_cornerRadius(bar, number_or_number_array_of_number(3));
    ShapeConfig_set_fill(bar, string_or_CanvasGradient_of_string(JsString_from("#f7f7f7")));
    ShapeConfig_set_stroke(bar, string_or_CanvasGradient_of_string(JsString_from("#111111")));
    ShapeConfig_set_strokeWidth(bar, js_some_double(1.5));
    JsArray_Node_2_push(items, AS_Node_2(Rect_new(bar)));

    TextConfig btxt = TextConfig_create();
    NodeConfig_set_x(btxt, js_some_double(180));
    NodeConfig_set_y(btxt, js_some_double(278));
    TextConfig_set_text(btxt, "Shared DOM & Package Handle Runtime");
    TextConfig_set_fontSize(btxt, js_some_double(12));
    TextConfig_set_fontFamily(btxt, "monospace");
    ShapeConfig_set_fill(btxt, string_or_CanvasGradient_of_string(JsString_from("#111111")));
    JsArray_Node_2_push(items, AS_Node_2(Text_2_new(btxt)));

    Container_add(layer, items);
    Node_2_draw(layer);

    int32_t rects = (int32_t)JsArray_Node_2_length(Container_find(layer, js_string("Rect")));
    int32_t arrows = (int32_t)JsArray_Node_2_length(Container_find(layer, js_string("Arrow")));
    int32_t jlen = (int32_t)JsString_len(Node_2_toJSON(layer));

    char meta[160];
    uint32_t p = append_cstr(meta, 0, sizeof(meta), "Pipeline diagram \xC2\xB7 ");
    p = append_i32(meta, p, sizeof(meta), rects);
    p = append_cstr(meta, p, sizeof(meta), " rects \xC2\xB7 ");
    p = append_i32(meta, p, sizeof(meta), arrows);
    p = append_cstr(meta, p, sizeof(meta), " arrows \xC2\xB7 JSON ");
    p = append_i32(meta, p, sizeof(meta), jlen);
    append_cstr(meta, p, sizeof(meta), " bytes");

    Document doc = get_document();
    HTMLElement meta_el = Document_getElementById(doc, "meta");
    Node_set_textContent(meta_el, meta);
}
