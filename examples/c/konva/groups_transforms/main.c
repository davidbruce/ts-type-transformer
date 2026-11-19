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

static Line make_grid_line(double x1, double y1, double x2, double y2) {
    JsArray_double pts = JsArray_double_new();
    JsArray_double_push(pts, x1);
    JsArray_double_push(pts, y1);
    JsArray_double_push(pts, x2);
    JsArray_double_push(pts, y2);
    LineConfig lc = LineConfig_create();
    LineConfig_set_points(lc, LinePoints_of_array(pts));
    ShapeConfig_set_stroke(lc, string_or_CanvasGradient_of_string(JsString_from("#e5e5e5")));
    ShapeConfig_set_strokeWidth(lc, js_some_double(1));
    return Line_new(lc);
}

__attribute__((export_name("start"))) void start(void) {
    StageConfig sc = StageConfig_create();
    StageConfig_set_container(sc, HTMLDivElement_or_string_of(JsString_from("stage")));
    NodeConfig_set_width(sc, js_some_double(680));
    NodeConfig_set_height(sc, js_some_double(400));
    Stage stage = Stage_new(sc);

    Layer layer = Layer_new((LayerConfig){ 0 });
    Stage_add(stage, layer, JsArray_new());
    JsArray_Node_2 top_items = JsArray_Node_2_new();

    RectConfig bg = RectConfig_create();
    NodeConfig_set_x(bg, js_some_double(0));
    NodeConfig_set_y(bg, js_some_double(0));
    NodeConfig_set_width(bg, js_some_double(680));
    NodeConfig_set_height(bg, js_some_double(400));
    ShapeConfig_set_fill(bg, string_or_CanvasGradient_of_string(JsString_from("#ffffff")));
    JsArray_Node_2_push(top_items, AS_Node_2(Rect_new(bg)));

    for (int x = 40; x <= 640; x += 40) {
        JsArray_Node_2_push(top_items, AS_Node_2(make_grid_line((double)x, 0, (double)x, 400)));
    }
    for (int y = 40; y <= 360; y += 40) {
        JsArray_Node_2_push(top_items, AS_Node_2(make_grid_line(0, (double)y, 680, (double)y)));
    }

    // Group A: unrotated base frame.
    GroupConfig gac = GroupConfig_create();
    NodeConfig_set_x(gac, js_some_double(40));
    NodeConfig_set_y(gac, js_some_double(55));
    Group group_a = Group_new(gac);
    JsArray_Node_2 ga_ch = JsArray_Node_2_new();

    RectConfig gab = RectConfig_create();
    NodeConfig_set_width(gab, js_some_double(160));
    NodeConfig_set_height(gab, js_some_double(120));
    ShapeConfig_set_fill(gab, string_or_CanvasGradient_of_string(JsString_from("#f7f7f7")));
    ShapeConfig_set_stroke(gab, string_or_CanvasGradient_of_string(JsString_from("#111111")));
    ShapeConfig_set_strokeWidth(gab, js_some_double(1.5));
    JsArray_Node_2_push(ga_ch, AS_Node_2(Rect_new(gab)));

    CircleConfig gad = CircleConfig_create();
    NodeConfig_set_x(gad, js_some_double(80));
    NodeConfig_set_y(gad, js_some_double(60));
    CircleConfig_set_radius(gad, js_some_double(24));
    ShapeConfig_set_fill(gad, string_or_CanvasGradient_of_string(JsString_from("#111111")));
    JsArray_Node_2_push(ga_ch, AS_Node_2(Circle_new(gad)));

    TextConfig gal = TextConfig_create();
    NodeConfig_set_x(gal, js_some_double(12));
    NodeConfig_set_y(gal, js_some_double(10));
    TextConfig_set_text(gal, "Group A \xC2\xB7 0\xC2\xB0");
    TextConfig_set_fontSize(gal, js_some_double(11));
    TextConfig_set_fontFamily(gal, "monospace");
    ShapeConfig_set_fill(gal, string_or_CanvasGradient_of_string(JsString_from("#111111")));
    JsArray_Node_2_push(ga_ch, AS_Node_2(Text_2_new(gal)));
    Container_add(group_a, ga_ch);
    JsArray_Node_2_push(top_items, AS_Node_2(group_a));

    // Group B: rotated 24 deg.
    GroupConfig gbc = GroupConfig_create();
    NodeConfig_set_x(gbc, js_some_double(340));
    NodeConfig_set_y(gbc, js_some_double(75));
    NodeConfig_set_rotation(gbc, js_some_double(24));
    Group group_b = Group_new(gbc);
    JsArray_Node_2 gb_ch = JsArray_Node_2_new();

    RectConfig gbo = RectConfig_create();
    NodeConfig_set_width(gbo, js_some_double(150));
    NodeConfig_set_height(gbo, js_some_double(110));
    ShapeConfig_set_fill(gbo, string_or_CanvasGradient_of_string(JsString_from("#efefef")));
    ShapeConfig_set_stroke(gbo, string_or_CanvasGradient_of_string(JsString_from("#111111")));
    ShapeConfig_set_strokeWidth(gbo, js_some_double(2));
    JsArray_Node_2_push(gb_ch, AS_Node_2(Rect_new(gbo)));

    JsArray_double dash = JsArray_double_new();
    JsArray_double_push(dash, 4.0);
    JsArray_double_push(dash, 3.0);
    RectConfig gbi = RectConfig_create();
    NodeConfig_set_x(gbi, js_some_double(22));
    NodeConfig_set_y(gbi, js_some_double(22));
    NodeConfig_set_width(gbi, js_some_double(106));
    NodeConfig_set_height(gbi, js_some_double(66));
    ShapeConfig_set_fill(gbi, string_or_CanvasGradient_of_string(JsString_from("#ffffff")));
    ShapeConfig_set_stroke(gbi, string_or_CanvasGradient_of_string(JsString_from("#555555")));
    ShapeConfig_set_strokeWidth(gbi, js_some_double(1.5));
    ShapeConfig_set_dash(gbi, dash);
    JsArray_Node_2_push(gb_ch, AS_Node_2(Rect_new(gbi)));

    CircleConfig gbd = CircleConfig_create();
    NodeConfig_set_x(gbd, js_some_double(75));
    NodeConfig_set_y(gbd, js_some_double(55));
    CircleConfig_set_radius(gbd, js_some_double(14));
    ShapeConfig_set_fill(gbd, string_or_CanvasGradient_of_string(JsString_from("#404040")));
    JsArray_Node_2_push(gb_ch, AS_Node_2(Circle_new(gbd)));

    TextConfig gbl = TextConfig_create();
    NodeConfig_set_x(gbl, js_some_double(10));
    NodeConfig_set_y(gbl, js_some_double(8));
    TextConfig_set_text(gbl, "Group B \xC2\xB7 24\xC2\xB0");
    TextConfig_set_fontSize(gbl, js_some_double(11));
    TextConfig_set_fontFamily(gbl, "monospace");
    ShapeConfig_set_fill(gbl, string_or_CanvasGradient_of_string(JsString_from("#111111")));
    JsArray_Node_2_push(gb_ch, AS_Node_2(Text_2_new(gbl)));
    Container_add(group_b, gb_ch);
    JsArray_Node_2_push(top_items, AS_Node_2(group_b));

    // Group C: scaled container with nested rotated child group.
    GroupConfig gcc = GroupConfig_create();
    NodeConfig_set_x(gcc, js_some_double(495));
    NodeConfig_set_y(gcc, js_some_double(55));
    NodeConfig_set_scaleX(gcc, js_some_double(1.15));
    NodeConfig_set_scaleY(gcc, js_some_double(1.15));
    Group group_c = Group_new(gcc);
    JsArray_Node_2 gc_ch = JsArray_Node_2_new();

    RectConfig gco = RectConfig_create();
    NodeConfig_set_width(gco, js_some_double(130));
    NodeConfig_set_height(gco, js_some_double(110));
    ShapeConfig_set_fill(gco, string_or_CanvasGradient_of_string(JsString_from("#ffffff")));
    ShapeConfig_set_stroke(gco, string_or_CanvasGradient_of_string(JsString_from("#111111")));
    ShapeConfig_set_strokeWidth(gco, js_some_double(2));
    JsArray_Node_2_push(gc_ch, AS_Node_2(Rect_new(gco)));

    TextConfig gcl = TextConfig_create();
    NodeConfig_set_x(gcl, js_some_double(8));
    NodeConfig_set_y(gcl, js_some_double(8));
    TextConfig_set_text(gcl, "Group C \xC2\xB7 1.15x");
    TextConfig_set_fontSize(gcl, js_some_double(10));
    TextConfig_set_fontFamily(gcl, "monospace");
    ShapeConfig_set_fill(gcl, string_or_CanvasGradient_of_string(JsString_from("#111111")));
    JsArray_Node_2_push(gc_ch, AS_Node_2(Text_2_new(gcl)));

    GroupConfig icc = GroupConfig_create();
    NodeConfig_set_x(icc, js_some_double(25));
    NodeConfig_set_y(icc, js_some_double(25));
    NodeConfig_set_rotation(icc, js_some_double(15));
    Group inner_group = Group_new(icc);

    CircleConfig mc = CircleConfig_create();
    NodeConfig_set_x(mc, js_some_double(40));
    NodeConfig_set_y(mc, js_some_double(30));
    CircleConfig_set_radius(mc, js_some_double(16));
    ShapeConfig_set_fill(mc, string_or_CanvasGradient_of_string(JsString_from("#111111")));
    Circle marker = Circle_new(mc);
    JsArray_Node_2 ig_ch = JsArray_Node_2_new();
    JsArray_Node_2_push(ig_ch, AS_Node_2(marker));
    Container_add(inner_group, ig_ch);
    JsArray_Node_2_push(gc_ch, AS_Node_2(inner_group));
    Container_add(group_c, gc_ch);
    JsArray_Node_2_push(top_items, AS_Node_2(group_c));

    Node_getAbsolutePosition_result abs = Node_2_getAbsolutePosition(marker);
    IRect bbox = Node_2_getClientRect(group_b);
    int32_t ax = (int32_t)(Node_getAbsolutePosition_result_get_x(abs) + 0.5);
    int32_t ay = (int32_t)(Node_getAbsolutePosition_result_get_y(abs) + 0.5);
    int32_t bw = (int32_t)(IRect_get_width(bbox) + 0.5);
    int32_t bh = (int32_t)(IRect_get_height(bbox) + 0.5);

    char strip_txt[160];
    uint32_t sp = append_cstr(strip_txt, 0, sizeof(strip_txt), "Marker abs=(");
    sp = append_i32(strip_txt, sp, sizeof(strip_txt), ax);
    sp = append_cstr(strip_txt, sp, sizeof(strip_txt), ", ");
    sp = append_i32(strip_txt, sp, sizeof(strip_txt), ay);
    sp = append_cstr(strip_txt, sp, sizeof(strip_txt), ") \xC2\xB7 Group B bbox=");
    sp = append_i32(strip_txt, sp, sizeof(strip_txt), bw);
    sp = append_cstr(strip_txt, sp, sizeof(strip_txt), "x");
    append_i32(strip_txt, sp, sizeof(strip_txt), bh);

    RectConfig strip = RectConfig_create();
    NodeConfig_set_x(strip, js_some_double(28));
    NodeConfig_set_y(strip, js_some_double(308));
    NodeConfig_set_width(strip, js_some_double(624));
    NodeConfig_set_height(strip, js_some_double(60));
    ShapeConfig_set_fill(strip, string_or_CanvasGradient_of_string(JsString_from("#f7f7f7")));
    ShapeConfig_set_stroke(strip, string_or_CanvasGradient_of_string(JsString_from("#111111")));
    ShapeConfig_set_strokeWidth(strip, js_some_double(1.5));
    JsArray_Node_2_push(top_items, AS_Node_2(Rect_new(strip)));

    TextConfig sum = TextConfig_create();
    NodeConfig_set_x(sum, js_some_double(44));
    NodeConfig_set_y(sum, js_some_double(331));
    TextConfig_set_text(sum, strip_txt);
    TextConfig_set_fontSize(sum, js_some_double(12));
    TextConfig_set_fontFamily(sum, "monospace");
    ShapeConfig_set_fill(sum, string_or_CanvasGradient_of_string(JsString_from("#111111")));
    JsArray_Node_2_push(top_items, AS_Node_2(Text_2_new(sum)));

    Container_add(layer, top_items);
    Node_2_draw(layer);

    char meta[192];
    uint32_t mp = append_cstr(meta, 0, sizeof(meta), "Stage: 680x400 \xC2\xB7 Nested marker abs=(");
    mp = append_i32(meta, mp, sizeof(meta), ax);
    mp = append_cstr(meta, mp, sizeof(meta), ", ");
    mp = append_i32(meta, mp, sizeof(meta), ay);
    mp = append_cstr(meta, mp, sizeof(meta), ") \xC2\xB7 Rotated Group B rect=");
    mp = append_i32(meta, mp, sizeof(meta), bw);
    mp = append_cstr(meta, mp, sizeof(meta), "x");
    append_i32(meta, mp, sizeof(meta), bh);

    Document doc = get_document();
    HTMLElement meta_el = Document_getElementById(doc, "meta");
    Node_set_textContent(meta_el, meta);
}
