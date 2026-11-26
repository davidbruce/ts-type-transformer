// Konva from C, in a page: the Konva object's settings, the Util and Easings
// helpers, and a stage on the page's element with a layer, shapes and a group,
// their positions and bounds, finding, events, JSON.
// Every CHECK must hold.
#include "bindings.h"

TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

static bool near(double a, double b, double eps) { return (a > b ? a - b : b - a) < eps; }
static bool contains(JsString s, const char* needle) {
    char buf[256];
    uint32_t n = JsString_copy(s, buf, sizeof buf), m = ts_strlen(needle);
    for (uint32_t i = 0; i + m <= n; i++) {
        uint32_t j = 0;
        while (j < m && buf[i + j] == needle[j]) j++;
        if (j == m) return true;
    }
    return false;
}
static double ease(JsValue v) { return js_to_number(v); }

static JsOptionalDouble D(double v) { return js_some_double(v); }
static double num(GetSet f) { return js_to_number(GetSet_invoke(f)); }
static bool text_is(JsValue v, const char* s) { return JsString_equals(js_to_string(v), s); }
static JsArray_Node nodes1(Node a) {
    JsArray_Node list = JsArray_Node_new();
    JsArray_Node_push(list, a);
    return list;
}

static int clicks;
static void on_click(void* ctx, KonvaEventObject e) { clicks++; }

static void shapes(void) {
    // A stage on the page's #stage, 400 by 300.
    StageConfig config = StageConfig_create();
    StageConfig_set_container(config, HTMLDivElement_or_string_of(JsString_from("stage")));
    NodeConfig_set_width(config, D(400));
    NodeConfig_set_height(config, D(300));
    Stage stage = Stage_new(config);
    CHECK(Stage_getContent(stage).h != 0);
    CHECK(JsString_equals(Node_getType(stage), "Stage"));
    CHECK(JsArray_Stage_length(Konva_get_stages()) == 1);

    Layer layer = Layer_new(LayerConfig_create());
    Stage_add(stage, layer, JsArray_new());
    CHECK(JsArray_Node_length(Container_getChildren(stage, (FilterFuncFn){ 0 })) == 1);
    CHECK(js_same((JsValue){ Node_getStage(layer).h }, (JsValue){ stage.h }));
    CHECK(JsString_equals(Node_getType(layer), "Layer"));

    // A rectangle from a config: position, size, colour, id and name.
    RectConfig rc = RectConfig_create();
    RectConfig_set_x(rc, D(20));
    RectConfig_set_y(rc, D(30));
    RectConfig_set_width(rc, D(100));
    RectConfig_set_height(rc, D(50));
    RectConfig_set_fill(rc, string_or_CanvasGradient_of_string(JsString_from("red")));
    RectConfig_set_stroke(rc, string_or_CanvasGradient_of_string(JsString_from("black")));
    RectConfig_set_strokeWidth(rc, D(4));
    RectConfig_set_id(rc, "box");
    RectConfig_set_name(rc, "shape box");
    Rect rect = Rect_new(rc);
    // The GetSet properties are read bound to their node.
    CHECK(num(Node_get_x(rect)) == 20 && num(Node_get_y(rect)) == 30);
    CHECK(num(Node_get_width(rect)) == 100 && num(Node_get_height(rect)) == 50);
    CHECK(text_is(GetSet_invoke(Node_get_id(rect)), "box"));
    CHECK(text_is(GetSet_invoke(Node_get_name(rect)), "shape box"));
    CHECK(text_is(GetSet_invoke(Shape_get_fill(rect)), "red"));
    CHECK(num(Shape_get_strokeWidth(rect)) == 4);
    CHECK(JsString_equals(Node_getClassName(rect), "Rect"));
    Container_add(layer, nodes1(AS_Node(rect)));
    CHECK(JsArray_Node_length(Container_getChildren(layer, (FilterFuncFn){ 0 })) == 1);
    CHECK(js_same((JsValue){ Node_getLayer(rect).h }, (JsValue){ layer.h }) && js_same((JsValue){ Node_getStage(rect).h }, (JsValue){ stage.h }));

    // A Circle and a Text, from their configs.
    CircleConfig cc = CircleConfig_create();
    NodeConfig_set_x(cc, D(200));
    NodeConfig_set_y(cc, D(100));
    CircleConfig_set_radius(cc, D(40));
    ShapeConfig_set_fill(cc, string_or_CanvasGradient_of_string(JsString_from("green")));
    NodeConfig_set_name(cc, "shape dot");
    Circle circle = Circle_new(cc);
    CHECK(num(Node_get_width(circle)) == 80 && num(Node_get_height(circle)) == 80);

    TextConfig tc = TextConfig_create();
    NodeConfig_set_x(tc, D(10));
    NodeConfig_set_y(tc, D(200));
    TextConfig_set_text(tc, "Hello Konva");
    TextConfig_set_fontSize(tc, D(24));
    TextConfig_set_fontFamily(tc, "sans-serif");
    NodeConfig_set_name(tc, "shape text");
    Text label = Text_new(tc);
    CHECK(text_is(GetSet_invoke(Text_get_text(label)), "Hello Konva"));
    CHECK(num(Text_get_fontSize(label)) == 24);
    CHECK(Text_getTextWidth(label) > 50 && Text_getTextHeight(label) > 0);
    Text_setText(label, "Hi");
    CHECK(text_is(GetSet_invoke(Text_get_text(label)), "Hi"));

    // A group moves what is in it.
    GroupConfig gc = GroupConfig_create();
    NodeConfig_set_x(gc, D(100));
    NodeConfig_set_y(gc, D(100));
    NodeConfig_set_name(gc, "cluster");
    Group group = Group_new(gc);
    JsArray_Node inside = JsArray_Node_new();
    JsArray_Node_push(inside, AS_Node(circle));
    JsArray_Node_push(inside, AS_Node(label));
    Container_add(group, inside);
    Container_add(layer, nodes1(AS_Node(group)));
    CHECK(JsArray_Node_length(Container_getChildren(group, (FilterFuncFn){ 0 })) == 2);
    CHECK(JsArray_Node_length(Container_getChildren(layer, (FilterFuncFn){ 0 })) == 2);
    CHECK(js_same((JsValue){ Node_getParent(circle).h }, (JsValue){ group.h }));

    // Positions: a node's own and its absolute one.
    Node_getAbsolutePosition_result abs = Node_getAbsolutePosition(circle);
    CHECK(near(Node_getAbsolutePosition_result_get_x(abs), 300, 1e-6) && near(Node_getAbsolutePosition_result_get_y(abs), 200, 1e-6));
    Node_getAbsolutePosition_result rel = Node_getPosition(circle);
    CHECK(Node_getAbsolutePosition_result_get_x(rel) == 200 && Node_getAbsolutePosition_result_get_y(rel) == 100);
    // A GetSet is set by calling it with the value.
    GetSet_invoke_2(Node_get_x(rect), js_number(60));
    CHECK(num(Node_get_x(rect)) == 60);
    CHECK(near(IRect_get_x(Node_getClientRect(rect)), 58, 1e-6));
    GetSet_invoke_2(Node_get_x(rect), js_number(20));
    CHECK(num(Node_get_x(rect)) == 20);

    // Rotation, opacity, visibility.
    GetSet_invoke_2(Node_get_rotation(rect), js_number(90));
    GetSet_invoke_2(Node_get_opacity(rect), js_number(0.5));
    CHECK(num(Node_get_rotation(rect)) == 90 && num(Node_get_opacity(rect)) == 0.5);
    Node_hide(rect);
    CHECK(!js_to_bool(Node_isVisible(rect)));
    Node_show(rect);
    CHECK(js_to_bool(Node_isVisible(rect)));
    GetSet_invoke_2(Node_get_rotation(rect), js_number(0));
    GetSet_invoke_2(Node_get_opacity(rect), js_number(1));

    // The client rect: the shape's bounds, stroke included.
    IRect box = Node_getClientRect(rect);
    CHECK(near(IRect_get_x(box), 18, 1e-6) && near(IRect_get_y(box), 28, 1e-6));
    CHECK(near(IRect_get_width(box), 104, 1e-6) && near(IRect_get_height(box), 54, 1e-6));
    CHECK(near(IRect_get_width(Node_getClientRect(circle)), 80, 1e-6));

    // Finding by name, by id and by type.
    CHECK(JsArray_Node_length(Container_find(layer, js_string("#box"))) == 1);
    CHECK(JsArray_Node_length(Container_find(layer, js_string(".shape"))) == 3);
    CHECK(JsArray_Node_length(Container_find(layer, js_string("Circle"))) == 1);
    CHECK(Container_findOne(layer, "#box").h != 0);
    CHECK(Container_findOne(layer, "#nothing").h == 0);
    CHECK(JsArray_Node_length(Container_find(group, js_string(".shape"))) == 2);

    // Events: a click fired by hand, then off.
    Node_on_click(rect, KonvaEventListener_from(on_click, 0));
    Node_fire(rect, "click");
    Node_fire(rect, "click");
    CHECK(clicks == 2);
    Node_removeEventListener(rect, "click");
    Node_fire(rect, "click");
    CHECK(clicks == 2);

    // Draw, and the stage as an image.
    Node_draw(layer);
    CHECK(Layer_getNativeCanvasElement(layer).h != 0);
    JsString png = Node_toDataURL(stage);
    char head[16];
    JsString_copy(png, head, sizeof head);
    CHECK(head[0] == 'd' && head[1] == 'a' && head[2] == 't' && head[3] == 'a' && head[4] == ':' && head[5] == 'i');

    // Z order.
    CHECK(Node_getZIndex(rect) == 0);
    Node_moveToTop(rect);
    CHECK(Node_getZIndex(rect) == 1);
    Node_moveToBottom(rect);
    CHECK(Node_getZIndex(rect) == 0);

    // toJSON holds the className and attrs.
    JsString json = Node_toJSON(rect);
    CHECK(contains(json, "\"className\":\"Rect\""));
    CHECK(contains(json, "\"width\":100"));
    CHECK(contains(json, "\"fill\":\"red\""));

    // Removing and destroying.
    Node_destroy(group);
    CHECK(JsArray_Node_length(Container_getChildren(layer, (FilterFuncFn){ 0 })) == 1);
    Node_remove(rect);
    CHECK(JsArray_Node_length(Container_getChildren(layer, (FilterFuncFn){ 0 })) == 0);
    Node_destroy(stage);
    CHECK(JsArray_Stage_length(Konva_get_stages()) == 0);
}

__attribute__((export_name("run"))) void run(void) {
    // ---- the Konva object ----
    JsString version = Konva_get_version();
    CHECK(contains(version, "10."));
    CHECK(Konva_get_isBrowser());
    CHECK(Konva_get_pixelRatio() > 0);
    CHECK(Konva_get_dragDistance() == 3);
    CHECK(Konva_get_dblClickWindow() == 400);
    CHECK(Konva_get_autoDrawEnabled() && Konva_get_hitOnDragEnabled() == false);
    CHECK(!Konva_isDragging() && !Konva_isTransforming() && !Konva_isDragReady());
    CHECK(JsArray_Stage_length(Konva_get_stages()) == 0);
    JsArray_double buttons = Konva_get_dragButtons();
    CHECK(JsArray_double_length(buttons) == 2 && JsArray_double_get(buttons, 0) == 0);
    // A setting can be changed and read back.
    Konva_set_dragDistance(10);
    CHECK(Konva_get_dragDistance() == 10);
    Konva_set_dragDistance(3);
    CHECK(Konva_get_dragDistance() == 3);
    Konva_set_showWarnings(false);
    CHECK(!Konva_get_showWarnings());
    // The shape classes are there, as values.
    CHECK(Konva_get_Stage().h != 0 && Konva_get_Layer().h != 0 && Konva_get_Rect().h != 0);

    // ---- Util ----
    Konva_Util util = Konva_get_Util();
    CHECK(near(Konva_Util_degToRad(util, 180), 3.14159265358979323846, 1e-12));
    CHECK(near(Konva_Util_radToDeg(util, 3.14159265358979323846 / 2), 90, 1e-12));
    CHECK(Konva_Util__sign(util, -5) == -1 && Konva_Util__sign(util, 7) == 1);
    JsString hex = Konva_Util__rgbToHex(util, 255, 128, 0);
    CHECK(JsString_equals(hex, "ff8000"));
    RGB rgb = Konva_Util__hexToRgb(util, "ff8000");
    CHECK(RGB_get_r(rgb) == 255 && RGB_get_g(rgb) == 128 && RGB_get_b(rgb) == 0);
    RGB named = Konva_Util_getRGB(util, "red");
    CHECK(RGB_get_r(named) == 255 && RGB_get_g(named) == 0 && RGB_get_b(named) == 0);
    Konva_Util__hex3ColorToRGBA_result rgba = Konva_Util_colorToRGBA(util, "#0000ff");
    CHECK(Konva_Util__hex3ColorToRGBA_result_get_b(rgba) == 255 && Konva_Util__hex3ColorToRGBA_result_get_a(rgba) == 1);
    Konva_Util__hex3ColorToRGBA_result short_hex = Konva_Util__hex3ColorToRGBA(util, "#f00");
    CHECK(Konva_Util__hex3ColorToRGBA_result_get_r(short_hex) == 255);
    JsString random = Konva_Util_getRandomColor(util);
    CHECK(JsString_len(random) == 7);
    CHECK(JsString_equals(Konva_Util__capitalize(util, "konva"), "Konva"));
    CHECK(Konva_Util__isNumber(util, js_number(3)) && !Konva_Util__isNumber(util, js_string("3")));
    CHECK(Konva_Util__isString(util, js_string("x")) && Konva_Util__isBoolean(util, js_bool(true)));
    CHECK(Konva_Util__isArray(util, (JsValue){ JsArray_new().h }));
    CHECK(Konva_Util_isValidSelector(util, js_string("#id")) && !Konva_Util_isValidSelector(util, js_number(1)));
    // Rectangles: do they overlap?
    IRect a = IRect_create(), b = IRect_create(), c = IRect_create();
    IRect_set_x(a, 0); IRect_set_y(a, 0); IRect_set_width(a, 10); IRect_set_height(a, 10);
    IRect_set_x(b, 5); IRect_set_y(b, 5); IRect_set_width(b, 10); IRect_set_height(b, 10);
    IRect_set_x(c, 50); IRect_set_y(c, 50); IRect_set_width(c, 10); IRect_set_height(c, 10);
    CHECK(Konva_Util_haveIntersection(util, a, b));
    CHECK(!Konva_Util_haveIntersection(util, a, c));
    HTMLCanvasElement canvas = Konva_Util_createCanvasElement(util);
    CHECK(canvas.h != 0);

    // ---- Easings: t, b (start), c (change), d (duration) ----
    Konva_Easings e = Konva_get_Easings();
    CHECK(ease(Konva_Easings_Linear(e, js_number(5), js_number(0), js_number(100), js_number(10))) == 50);
    CHECK(ease(Konva_Easings_EaseIn(e, js_number(0), js_number(10), js_number(20), js_number(4))) == 10);
    CHECK(near(ease(Konva_Easings_EaseIn(e, js_number(4), js_number(10), js_number(20), js_number(4))), 30, 1e-9));
    CHECK(near(ease(Konva_Easings_EaseInOut(e, js_number(2), js_number(0), js_number(100), js_number(4))), 50, 1e-9));
    CHECK(ease(Konva_Easings_StrongEaseIn(e, js_number(2), js_number(0), js_number(100), js_number(4))) < 50);
    CHECK(near(ease(Konva_Easings_BounceEaseOut(e, js_number(4), js_number(0), js_number(100), js_number(4))), 100, 1e-9));

    shapes();
}
