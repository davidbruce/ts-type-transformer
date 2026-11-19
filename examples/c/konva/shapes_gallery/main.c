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
    if (v < 0) {
        if (pos + 1 < cap) dst[pos++] = '-';
        v = -v;
    }
    if (v == 0) tmp[len++] = '0';
    else while (v > 0 && len < sizeof(tmp)) { tmp[len++] = (char)('0' + (v % 10)); v /= 10; }
    for (uint32_t i = 0; i < len && pos + 1 < cap; i++) dst[pos++] = tmp[len - 1 - i];
    dst[pos] = '\0';
    return pos;
}

static void add_node(JsArray_Node_2 list, Node_2 n) {
    JsArray_Node_2_push(list, n);
}

static void add_caption(JsArray_Node_2 list, double x, double y, const char* caption) {
    TextConfig tc = TextConfig_create();
    NodeConfig_set_x(tc, js_some_double(x));
    NodeConfig_set_y(tc, js_some_double(y));
    TextConfig_set_text(tc, caption);
    TextConfig_set_fontSize(tc, js_some_double(11));
    TextConfig_set_fontFamily(tc, "monospace");
    ShapeConfig_set_fill(tc, string_or_CanvasGradient_of_string(JsString_from("#555555")));
    add_node(list, AS_Node_2(Text_2_new(tc)));
}

__attribute__((export_name("start"))) void start(void) {
    StageConfig sc = StageConfig_create();
    StageConfig_set_container(sc, HTMLDivElement_or_string_of(JsString_from("stage")));
    NodeConfig_set_width(sc, js_some_double(680));
    NodeConfig_set_height(sc, js_some_double(400));
    Stage stage = Stage_new(sc);

    Layer layer = Layer_new((LayerConfig){ 0 });
    Stage_add(stage, layer, JsArray_new());
    JsArray_Node_2 children = JsArray_Node_2_new();

    RectConfig bg = RectConfig_create();
    NodeConfig_set_x(bg, js_some_double(0));
    NodeConfig_set_y(bg, js_some_double(0));
    NodeConfig_set_width(bg, js_some_double(680));
    NodeConfig_set_height(bg, js_some_double(400));
    ShapeConfig_set_fill(bg, string_or_CanvasGradient_of_string(JsString_from("#ffffff")));
    add_node(children, AS_Node_2(Rect_new(bg)));

    // Row 1: Rounded filled Rect, dashed Rect, Circle, Hexagon.
    RectConfig r1 = RectConfig_create();
    NodeConfig_set_x(r1, js_some_double(40));
    NodeConfig_set_y(r1, js_some_double(55));
    NodeConfig_set_width(r1, js_some_double(110));
    NodeConfig_set_height(r1, js_some_double(76));
    RectConfig_set_cornerRadius(r1, number_or_number_array_of_number(8));
    ShapeConfig_set_fill(r1, string_or_CanvasGradient_of_string(JsString_from("#111111")));
    add_node(children, AS_Node_2(Rect_new(r1)));
    add_caption(children, 40, 142, "Rect (filled)");

    JsArray_double dash = JsArray_double_new();
    JsArray_double_push(dash, 6.0);
    JsArray_double_push(dash, 4.0);
    RectConfig r2 = RectConfig_create();
    NodeConfig_set_x(r2, js_some_double(195));
    NodeConfig_set_y(r2, js_some_double(55));
    NodeConfig_set_width(r2, js_some_double(110));
    NodeConfig_set_height(r2, js_some_double(76));
    ShapeConfig_set_fill(r2, string_or_CanvasGradient_of_string(JsString_from("#ffffff")));
    ShapeConfig_set_stroke(r2, string_or_CanvasGradient_of_string(JsString_from("#111111")));
    ShapeConfig_set_strokeWidth(r2, js_some_double(2));
    ShapeConfig_set_dash(r2, dash);
    add_node(children, AS_Node_2(Rect_new(r2)));
    add_caption(children, 195, 142, "Rect (dashed)");

    CircleConfig cc = CircleConfig_create();
    NodeConfig_set_x(cc, js_some_double(415));
    NodeConfig_set_y(cc, js_some_double(93));
    CircleConfig_set_radius(cc, js_some_double(40));
    ShapeConfig_set_fill(cc, string_or_CanvasGradient_of_string(JsString_from("#404040")));
    ShapeConfig_set_stroke(cc, string_or_CanvasGradient_of_string(JsString_from("#111111")));
    ShapeConfig_set_strokeWidth(cc, js_some_double(2));
    add_node(children, AS_Node_2(Circle_new(cc)));
    add_caption(children, 395, 142, "Circle");

    RegularPolygonConfig pc = RegularPolygonConfig_create();
    NodeConfig_set_x(pc, js_some_double(575));
    NodeConfig_set_y(pc, js_some_double(93));
    RegularPolygonConfig_set_sides(pc, 6);
    RegularPolygonConfig_set_radius(pc, 42);
    ShapeConfig_set_fill(pc, string_or_CanvasGradient_of_string(JsString_from("#efefef")));
    ShapeConfig_set_stroke(pc, string_or_CanvasGradient_of_string(JsString_from("#111111")));
    ShapeConfig_set_strokeWidth(pc, js_some_double(2));
    add_node(children, AS_Node_2(RegularPolygon_new(pc)));
    add_caption(children, 548, 142, "Hexagon");

    // Row 2: Star, Ring, Wedge, Framed Text.
    StarConfig st = StarConfig_create();
    NodeConfig_set_x(st, js_some_double(95));
    NodeConfig_set_y(st, js_some_double(268));
    StarConfig_set_numPoints(st, 5);
    StarConfig_set_innerRadius(st, 19);
    StarConfig_set_outerRadius(st, 44);
    ShapeConfig_set_fill(st, string_or_CanvasGradient_of_string(JsString_from("#737373")));
    ShapeConfig_set_stroke(st, string_or_CanvasGradient_of_string(JsString_from("#111111")));
    ShapeConfig_set_strokeWidth(st, js_some_double(2));
    add_node(children, AS_Node_2(Star_new(st)));
    add_caption(children, 62, 325, "Star (5pt)");

    RingConfig rc = RingConfig_create();
    NodeConfig_set_x(rc, js_some_double(250));
    NodeConfig_set_y(rc, js_some_double(268));
    RingConfig_set_innerRadius(rc, 22);
    RingConfig_set_outerRadius(rc, 42);
    ShapeConfig_set_fill(rc, string_or_CanvasGradient_of_string(JsString_from("#d4d4d4")));
    ShapeConfig_set_stroke(rc, string_or_CanvasGradient_of_string(JsString_from("#111111")));
    ShapeConfig_set_strokeWidth(rc, js_some_double(2));
    add_node(children, AS_Node_2(Ring_new(rc)));
    add_caption(children, 236, 325, "Ring");

    WedgeConfig wc = WedgeConfig_create();
    NodeConfig_set_x(wc, js_some_double(415));
    NodeConfig_set_y(wc, js_some_double(268));
    WedgeConfig_set_radius(wc, 44);
    WedgeConfig_set_angle(wc, 270);
    NodeConfig_set_rotation(wc, js_some_double(-90));
    ShapeConfig_set_fill(wc, string_or_CanvasGradient_of_string(JsString_from("#111111")));
    ShapeConfig_set_stroke(wc, string_or_CanvasGradient_of_string(JsString_from("#555555")));
    ShapeConfig_set_strokeWidth(wc, js_some_double(1));
    add_node(children, AS_Node_2(Wedge_new(wc)));
    add_caption(children, 382, 325, "Wedge 270\xC2\xB0");

    RectConfig tf = RectConfig_create();
    NodeConfig_set_x(tf, js_some_double(500));
    NodeConfig_set_y(tf, js_some_double(224));
    NodeConfig_set_width(tf, js_some_double(150));
    NodeConfig_set_height(tf, js_some_double(88));
    ShapeConfig_set_fill(tf, string_or_CanvasGradient_of_string(JsString_from("#f7f7f7")));
    ShapeConfig_set_stroke(tf, string_or_CanvasGradient_of_string(JsString_from("#111111")));
    ShapeConfig_set_strokeWidth(tf, js_some_double(1.5));
    add_node(children, AS_Node_2(Rect_new(tf)));

    TextConfig txt = TextConfig_create();
    NodeConfig_set_x(txt, js_some_double(514));
    NodeConfig_set_y(txt, js_some_double(240));
    TextConfig_set_text(txt, "KONVA 2D\nMonochrome\nScene Graph");
    TextConfig_set_fontSize(txt, js_some_double(13));
    TextConfig_set_fontFamily(txt, "monospace");
    ShapeConfig_set_fill(txt, string_or_CanvasGradient_of_string(JsString_from("#111111")));
    add_node(children, AS_Node_2(Text_2_new(txt)));
    add_caption(children, 558, 325, "Text");

    Container_add(layer, children);
    Node_2_draw(layer);

    int32_t count = (int32_t)JsArray_Node_2_length(Container_getChildren(layer));
    char meta[128];
    uint32_t p = append_cstr(meta, 0, sizeof(meta), "Stage: 680x400 \xC2\xB7 7 primitive types \xC2\xB7 ");
    p = append_i32(meta, p, sizeof(meta), count);
    append_cstr(meta, p, sizeof(meta), " layer children");
    Document doc = get_document();
    HTMLElement meta_el = Document_getElementById(doc, "meta");
    Node_set_textContent(meta_el, meta);
}
