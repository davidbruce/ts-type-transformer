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

static int32_t g_clicks = 0;
static void on_card_click(void* ctx, KonvaEventObject e) {
    (void)ctx;
    (void)e;
    g_clicks++;
}

static Line make_line(double x1, double y1, double x2, double y2, const char* color, double width) {
    JsArray_double pts = JsArray_double_new();
    JsArray_double_push(pts, x1);
    JsArray_double_push(pts, y1);
    JsArray_double_push(pts, x2);
    JsArray_double_push(pts, y2);
    LineConfig lc = LineConfig_create();
    LineConfig_set_points(lc, LinePoints_of_array(pts));
    ShapeConfig_set_stroke(lc, string_or_CanvasGradient_of_string(JsString_from(color)));
    ShapeConfig_set_strokeWidth(lc, js_some_double(width));
    return Line_new(lc);
}

static Rect make_card(const char* id, double x, double y, const char* fill) {
    RectConfig rc = RectConfig_create();
    NodeConfig_set_id(rc, id);
    NodeConfig_set_name(rc, "card");
    NodeConfig_set_x(rc, js_some_double(x));
    NodeConfig_set_y(rc, js_some_double(y));
    NodeConfig_set_width(rc, js_some_double(200));
    NodeConfig_set_height(rc, js_some_double(130));
    ShapeConfig_set_fill(rc, string_or_CanvasGradient_of_string(JsString_from(fill)));
    ShapeConfig_set_stroke(rc, string_or_CanvasGradient_of_string(JsString_from("#111111")));
    ShapeConfig_set_strokeWidth(rc, js_some_double(2));
    return Rect_new(rc);
}

__attribute__((export_name("start"))) void start(void) {
    StageConfig sc = StageConfig_create();
    StageConfig_set_container(sc, HTMLDivElement_or_string_of(JsString_from("stage")));
    NodeConfig_set_width(sc, js_some_double(680));
    NodeConfig_set_height(sc, js_some_double(400));
    Stage stage = Stage_new(sc);

    Layer bg_layer = Layer_new((LayerConfig){ 0 });
    Layer fg_layer = Layer_new((LayerConfig){ 0 });
    Stage_add(stage, bg_layer, JsArray_new());
    Stage_add(stage, fg_layer, JsArray_new());

    // Layer 1: Blueprint Grid.
    JsArray_Node_2 bg_items = JsArray_Node_2_new();
    RectConfig bgr = RectConfig_create();
    NodeConfig_set_x(bgr, js_some_double(0));
    NodeConfig_set_y(bgr, js_some_double(0));
    NodeConfig_set_width(bgr, js_some_double(680));
    NodeConfig_set_height(bgr, js_some_double(400));
    ShapeConfig_set_fill(bgr, string_or_CanvasGradient_of_string(JsString_from("#ffffff")));
    JsArray_Node_2_push(bg_items, AS_Node_2(Rect_new(bgr)));

    for (int x = 40; x <= 640; x += 40) {
        JsArray_Node_2_push(bg_items, AS_Node_2(make_line((double)x, 20, (double)x, 380, "#e5e5e5", 1)));
    }
    for (int y = 40; y <= 360; y += 40) {
        JsArray_Node_2_push(bg_items, AS_Node_2(make_line(20, (double)y, 660, (double)y, "#e5e5e5", 1)));
    }

    JsArray_double dash = JsArray_double_new();
    JsArray_double_push(dash, 6.0);
    JsArray_double_push(dash, 4.0);
    RectConfig frame = RectConfig_create();
    NodeConfig_set_x(frame, js_some_double(20));
    NodeConfig_set_y(frame, js_some_double(20));
    NodeConfig_set_width(frame, js_some_double(640));
    NodeConfig_set_height(frame, js_some_double(360));
    ShapeConfig_set_stroke(frame, string_or_CanvasGradient_of_string(JsString_from("#b5b5b5")));
    ShapeConfig_set_strokeWidth(frame, js_some_double(1.5));
    ShapeConfig_set_dash(frame, dash);
    JsArray_Node_2_push(bg_items, AS_Node_2(Rect_new(frame)));

    TextConfig l1 = TextConfig_create();
    NodeConfig_set_x(l1, js_some_double(32));
    NodeConfig_set_y(l1, js_some_double(32));
    TextConfig_set_text(l1, "Layer 1: Blueprint Grid (z=0)");
    TextConfig_set_fontSize(l1, js_some_double(12));
    TextConfig_set_fontFamily(l1, "monospace");
    ShapeConfig_set_fill(l1, string_or_CanvasGradient_of_string(JsString_from("#737373")));
    JsArray_Node_2_push(bg_items, AS_Node_2(Text_2_new(l1)));
    Container_add(bg_layer, bg_items);

    // Layer 2: Stacked cards & badge.
    JsArray_Node_2 fg_items = JsArray_Node_2_new();
    Rect card_a = make_card("card-a", 110, 80, "#efefef");
    Rect card_b = make_card("card-b", 210, 125, "#404040");
    Rect card_c = make_card("card-c", 310, 170, "#ffffff");
    CircleConfig bc = CircleConfig_create();
    NodeConfig_set_id(bc, "badge");
    NodeConfig_set_x(bc, js_some_double(555));
    NodeConfig_set_y(bc, js_some_double(135));
    CircleConfig_set_radius(bc, js_some_double(38));
    ShapeConfig_set_fill(bc, string_or_CanvasGradient_of_string(JsString_from("#111111")));
    Circle badge = Circle_new(bc);

    JsArray_Node_2_push(fg_items, AS_Node_2(card_a));
    JsArray_Node_2_push(fg_items, AS_Node_2(card_b));
    JsArray_Node_2_push(fg_items, AS_Node_2(card_c));
    JsArray_Node_2_push(fg_items, AS_Node_2(badge));
    Container_add(fg_layer, fg_items);

    Node_2_moveToTop(card_a);
    int32_t card_a_z = (int32_t)Node_2_getZIndex(card_a);

    Node_2_on_click(card_a, KonvaEventListener_from(on_card_click, 0));
    Node_2_fire(card_a, "click");
    Node_2_fire(card_a, "click");

    Node_2_draw(bg_layer);
    Node_2_draw(fg_layer);

    Vector2d pt = Vector2d_create();
    Vector2d_set_x(pt, 250);
    Vector2d_set_y(pt, 160);
    Shape hit = Stage_getIntersection(stage, pt);
    char hit_buf[64];
    if (hit.h != 0) {
        JsString hid = js_to_string(GetSet_invoke(Node_2_get_id(hit)));
        uint32_t n = JsString_copy(hid, hit_buf, sizeof(hit_buf) - 1);
        hit_buf[n] = '\0';
    } else {
        append_cstr(hit_buf, 0, sizeof(hit_buf), "none");
    }

    // Crosshair & foreground label.
    JsArray_Node_2 overlay = JsArray_Node_2_new();
    CircleConfig dc = CircleConfig_create();
    NodeConfig_set_x(dc, js_some_double(250));
    NodeConfig_set_y(dc, js_some_double(160));
    CircleConfig_set_radius(dc, js_some_double(7));
    ShapeConfig_set_fill(dc, string_or_CanvasGradient_of_string(JsString_from("#ffffff")));
    ShapeConfig_set_stroke(dc, string_or_CanvasGradient_of_string(JsString_from("#111111")));
    ShapeConfig_set_strokeWidth(dc, js_some_double(2));
    JsArray_Node_2_push(overlay, AS_Node_2(Circle_new(dc)));
    JsArray_Node_2_push(overlay, AS_Node_2(make_line(238, 160, 262, 160, "#111111", 1.5)));
    JsArray_Node_2_push(overlay, AS_Node_2(make_line(250, 148, 250, 172, "#111111", 1.5)));

    TextConfig l2 = TextConfig_create();
    NodeConfig_set_x(l2, js_some_double(32));
    NodeConfig_set_y(l2, js_some_double(354));
    TextConfig_set_text(l2, "Layer 2: Elevated Stack (z=1) \xC2\xB7 Probe (250,160)");
    TextConfig_set_fontSize(l2, js_some_double(12));
    TextConfig_set_fontFamily(l2, "monospace");
    ShapeConfig_set_fill(l2, string_or_CanvasGradient_of_string(JsString_from("#111111")));
    JsArray_Node_2_push(overlay, AS_Node_2(Text_2_new(l2)));
    Container_add(fg_layer, overlay);
    Node_2_draw(fg_layer);

    int32_t layers = (int32_t)JsArray_Layer_length(Stage_getLayers(stage));
    char meta[192];
    uint32_t p = append_cstr(meta, 0, sizeof(meta), "Layers: ");
    p = append_i32(meta, p, sizeof(meta), layers);
    p = append_cstr(meta, p, sizeof(meta), " \xC2\xB7 card-a zIndex=");
    p = append_i32(meta, p, sizeof(meta), card_a_z);
    p = append_cstr(meta, p, sizeof(meta), " \xC2\xB7 hit@(250,160)=");
    p = append_cstr(meta, p, sizeof(meta), hit_buf);
    p = append_cstr(meta, p, sizeof(meta), " \xC2\xB7 clicks=");
    append_i32(meta, p, sizeof(meta), g_clicks);

    Document doc = get_document();
    HTMLElement meta_el = Document_getElementById(doc, "meta");
    Node_set_textContent(meta_el, meta);
}
