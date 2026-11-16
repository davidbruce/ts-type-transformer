// Konva from C, in a page. FINDING 16: none of Konva's classes can be
// constructed: Stage_new, Layer_new, Rect_new, ... run `new PK.konva.Stage`,
// but the package exports only `default` (the Konva object that has Stage on
// it); PK.konva.Stage is undefined ("PK.konva.Stage is not a constructor").
// `Konva_get_Stage()` gives the class as a bare JsValue, and a JsValue cannot
// be constructed. So no stage, layer, shape, group, find, event or JSON can be
// made. The program that was meant (a Stage on the "stage" element, a Layer,
// Rect/Circle/Text/Group, positions, getClientRect, find/findOne, on/fire,
// toJSON, destroy) is left out; what can be reached is the
// Konva object itself: settings and the Util and Easings helpers.
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
}
