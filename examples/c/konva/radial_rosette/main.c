#include "dom/dom.h"
#include "konva/konva.h"

#define PI 3.14159265358979323846
#define TWO_PI (2.0 * PI)
#define HALF_PI (0.5 * PI)

static double approx_sin(double x) {
    while (x > PI) x -= TWO_PI;
    while (x < -PI) x += TWO_PI;
    double x2 = x * x;
    return x * (1.0 - x2 / 6.0 * (1.0 - x2 / 20.0 * (1.0 - x2 / 42.0)));
}

static double approx_cos(double x) {
    return approx_sin(x + HALF_PI);
}

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

    const double cx = 340.0;
    const double cy = 200.0;

    const double ring_r[] = { 160.0, 125.0, 90.0, 52.0 };
    const char* ring_stroke[] = { "#d4d4d4", "#a3a3a3", "#555555", "#111111" };
    const double ring_sw[] = { 1.0, 1.0, 1.0, 1.5 };
    for (int i = 0; i < 4; i++) {
        CircleConfig cc = CircleConfig_create();
        NodeConfig_set_x(cc, js_some_double(cx));
        NodeConfig_set_y(cc, js_some_double(cy));
        CircleConfig_set_radius(cc, js_some_double(ring_r[i]));
        ShapeConfig_set_stroke(cc, string_or_CanvasGradient_of_string(JsString_from(ring_stroke[i])));
        ShapeConfig_set_strokeWidth(cc, js_some_double(ring_sw[i]));
        if (i == 3) ShapeConfig_set_fill(cc, string_or_CanvasGradient_of_string(JsString_from("#f7f7f7")));
        JsArray_Node_2_push(items, AS_Node_2(Circle_new(cc)));
    }

    for (int i = 0; i < 12; i++) {
        double a = (double)i * PI / 6.0;
        JsArray_double pts = JsArray_double_new();
        JsArray_double_push(pts, cx + 36.0 * approx_cos(a));
        JsArray_double_push(pts, cy + 36.0 * approx_sin(a));
        JsArray_double_push(pts, cx + 160.0 * approx_cos(a));
        JsArray_double_push(pts, cy + 160.0 * approx_sin(a));
        LineConfig lc = LineConfig_create();
        LineConfig_set_points(lc, LinePoints_of_array(pts));
        ShapeConfig_set_stroke(lc, string_or_CanvasGradient_of_string(JsString_from("#d4d4d4")));
        ShapeConfig_set_strokeWidth(lc, js_some_double(1));
        JsArray_Node_2_push(items, AS_Node_2(Line_new(lc)));
    }

    for (int i = 0; i < 24; i++) {
        double a = (double)i * PI / 12.0;
        double r0 = (i % 2 == 0) ? 148.0 : 154.0;
        JsArray_double pts = JsArray_double_new();
        JsArray_double_push(pts, cx + r0 * approx_cos(a));
        JsArray_double_push(pts, cy + r0 * approx_sin(a));
        JsArray_double_push(pts, cx + 166.0 * approx_cos(a));
        JsArray_double_push(pts, cy + 166.0 * approx_sin(a));
        LineConfig lc = LineConfig_create();
        LineConfig_set_points(lc, LinePoints_of_array(pts));
        ShapeConfig_set_stroke(lc, string_or_CanvasGradient_of_string(JsString_from("#111111")));
        ShapeConfig_set_strokeWidth(lc, js_some_double(i % 2 == 0 ? 2.0 : 1.0));
        JsArray_Node_2_push(items, AS_Node_2(Line_new(lc)));
    }

    for (int i = 0; i < 8; i++) {
        RectConfig pc = RectConfig_create();
        NodeConfig_set_x(pc, js_some_double(cx));
        NodeConfig_set_y(pc, js_some_double(cy));
        NodeConfig_set_width(pc, js_some_double(18));
        NodeConfig_set_height(pc, js_some_double(52));
        NodeConfig_set_offsetX(pc, js_some_double(9));
        NodeConfig_set_offsetY(pc, js_some_double(108));
        NodeConfig_set_rotation(pc, js_some_double((double)i * 45.0));
        ShapeConfig_set_fill(pc, string_or_CanvasGradient_of_string(JsString_from(i % 2 == 0 ? "#111111" : "#efefef")));
        ShapeConfig_set_stroke(pc, string_or_CanvasGradient_of_string(JsString_from("#111111")));
        ShapeConfig_set_strokeWidth(pc, js_some_double(1.5));
        JsArray_Node_2_push(items, AS_Node_2(Rect_new(pc)));
    }

    for (int i = 0; i < 12; i++) {
        double a = (double)i * PI / 6.0;
        CircleConfig oc = CircleConfig_create();
        NodeConfig_set_x(oc, js_some_double(cx + 125.0 * approx_cos(a)));
        NodeConfig_set_y(oc, js_some_double(cy + 125.0 * approx_sin(a)));
        CircleConfig_set_radius(oc, js_some_double(5));
        ShapeConfig_set_fill(oc, string_or_CanvasGradient_of_string(JsString_from(i % 2 == 0 ? "#111111" : "#ffffff")));
        ShapeConfig_set_stroke(oc, string_or_CanvasGradient_of_string(JsString_from("#111111")));
        ShapeConfig_set_strokeWidth(oc, js_some_double(1.5));
        JsArray_Node_2_push(items, AS_Node_2(Circle_new(oc)));
    }

    RegularPolygonConfig cp = RegularPolygonConfig_create();
    NodeConfig_set_x(cp, js_some_double(cx));
    NodeConfig_set_y(cp, js_some_double(cy));
    RegularPolygonConfig_set_sides(cp, 8);
    RegularPolygonConfig_set_radius(cp, 26);
    ShapeConfig_set_fill(cp, string_or_CanvasGradient_of_string(JsString_from("#111111")));
    JsArray_Node_2_push(items, AS_Node_2(RegularPolygon_new(cp)));

    CircleConfig cd = CircleConfig_create();
    NodeConfig_set_x(cd, js_some_double(cx));
    NodeConfig_set_y(cd, js_some_double(cy));
    CircleConfig_set_radius(cd, js_some_double(9));
    ShapeConfig_set_fill(cd, string_or_CanvasGradient_of_string(JsString_from("#ffffff")));
    JsArray_Node_2_push(items, AS_Node_2(Circle_new(cd)));

    const double lx[] = { 516, 332, 140, 328 };
    const double ly[] = { 194, 376, 194, 14 };
    const char* labels[] = { "0\xC2\xB0", "90\xC2\xB0", "180\xC2\xB0", "270\xC2\xB0" };
    for (int i = 0; i < 4; i++) {
        TextConfig tc = TextConfig_create();
        NodeConfig_set_x(tc, js_some_double(lx[i]));
        NodeConfig_set_y(tc, js_some_double(ly[i]));
        TextConfig_set_text(tc, labels[i]);
        TextConfig_set_fontSize(tc, js_some_double(11));
        TextConfig_set_fontFamily(tc, "monospace");
        ShapeConfig_set_fill(tc, string_or_CanvasGradient_of_string(JsString_from("#555555")));
        JsArray_Node_2_push(items, AS_Node_2(Text_2_new(tc)));
    }

    Container_add(layer, items);
    Node_2_draw(layer);

    int32_t count = (int32_t)JsArray_Node_2_length(Container_getChildren(layer));
    char meta[128];
    uint32_t p = append_cstr(meta, 0, sizeof(meta), "Stage: 680x400 \xC2\xB7 Procedural rosette \xC2\xB7 ");
    p = append_i32(meta, p, sizeof(meta), count);
    append_cstr(meta, p, sizeof(meta), " shapes on layer");

    Document doc = get_document();
    HTMLElement meta_el = Document_getElementById(doc, "meta");
    Node_set_textContent(meta_el, meta);
}
