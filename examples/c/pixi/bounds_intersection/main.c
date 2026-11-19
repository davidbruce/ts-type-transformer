#include "dom/dom.h"
#include "pixi_js/pixi_js.h"

static Application g_app;

static Rectangle mk_rect(double x, double y, double w, double h) {
    return Rectangle_new_with_number_and_number_and_number_and_number(
        js_some_double(x), js_some_double(y), js_some_double(w), js_some_double(h));
}

static void stroke_g(Graphics g, double width, double color_hex) {
    StrokeStyle ss = StrokeStyle_create();
    StrokeAttributes_set_width(StrokeStyle_as_StrokeAttributes(ss), js_some_double(width));
    FillStyle_set_color(StrokeStyle_as_FillStyle(ss), js_number(color_hex));
    JsArray args = JsArray_new();
    JsArray_push(args, (JsValue){ ss.h });
    Graphics_stroke(g, args);
}

static int append_str(char* buf, int pos, const char* s) {
    while (*s) buf[pos++] = *s++;
    return pos;
}

static int append_int(char* buf, int pos, int val) {
    if (val == 0) {
        buf[pos++] = '0';
        return pos;
    }
    char tmp[16];
    int n = 0;
    while (val > 0) {
        tmp[n++] = (char)('0' + (val % 10));
        val /= 10;
    }
    for (int i = n - 1; i >= 0; i--) buf[pos++] = tmp[i];
    return pos;
}

static void on_init(void* ctx, bool ok, JsValue v) {
    (void)ctx;
    (void)v;
    if (!ok) return;

    Document doc = get_document();
    HTMLElement stage_el = Document_getElementById(doc, "stage");
    if (stage_el.h != 0) {
        Node_appendChild(stage_el, Application_get_canvas(g_app));
    }

    Container stage = Application_get_stage(g_app);
    Rectangle query = mk_rect(190.0, 105.0, 260.0, 170.0);
    Rectangle padded = Rectangle_pad(Rectangle_clone(query), js_some_double(10.0), js_some_double(10.0));
    Rectangle hull = Rectangle_clone(query);

    Rectangle cands[8] = {
        mk_rect(210.0, 125.0, 90.0, 60.0),
        mk_rect(330.0, 165.0, 95.0, 80.0),
        mk_rect(110.0, 80.0, 130.0, 85.0),
        mk_rect(390.0, 75.0, 130.0, 90.0),
        mk_rect(135.0, 225.0, 110.0, 90.0),
        mk_rect(40.0, 40.0, 100.0, 110.0),
        mk_rect(510.0, 48.0, 125.0, 115.0),
        mk_rect(505.0, 245.0, 130.0, 110.0),
    };

    int contained = 0;
    int intersecting = 0;
    int disjoint = 0;

    Graphics g = Graphics_new__0();
    for (int i = 0; i < 8; i++) {
        Rectangle c = cands[i];
        Rectangle_enlarge(hull, c);
        Graphics_rect(g, Rectangle_get_x(c), Rectangle_get_y(c), Rectangle_get_width(c), Rectangle_get_height(c));
        if (Rectangle_containsRect(query, c)) {
            contained++;
            Graphics_fill(g, js_number((double)0x111111));
        } else if (Rectangle_intersects(query, c, (Matrix){ 0 })) {
            intersecting++;
            Graphics_fill(g, js_number((double)0x737373));
            stroke_g(g, 2.0, (double)0x111111);
        } else {
            disjoint++;
            Graphics_fill(g, js_number((double)0xe5e5e5));
            stroke_g(g, 1.0, (double)0xa3a3a3);
        }
    }

    Graphics_rect(g, Rectangle_get_x(hull), Rectangle_get_y(hull), Rectangle_get_width(hull), Rectangle_get_height(hull));
    stroke_g(g, 1.5, (double)0xa3a3a3);

    Graphics_rect(g, Rectangle_get_x(padded), Rectangle_get_y(padded), Rectangle_get_width(padded), Rectangle_get_height(padded));
    stroke_g(g, 1.0, (double)0x737373);

    Graphics_rect(g, Rectangle_get_x(query), Rectangle_get_y(query), Rectangle_get_width(query), Rectangle_get_height(query));
    stroke_g(g, 2.5, (double)0x111111);

    JsArray_Container kids = JsArray_Container_new();
    JsArray_Container_push(kids, Graphics_as_Container(g));
    Container_addChild(stage, kids);
    Application_render(g_app);

    char buf[160];
    int p = 0;
    p = append_str(buf, p, "query 260x170 · ");
    p = append_int(buf, p, contained);
    p = append_str(buf, p, " contained · ");
    p = append_int(buf, p, intersecting);
    p = append_str(buf, p, " intersecting · ");
    p = append_int(buf, p, disjoint);
    p = append_str(buf, p, " disjoint · enlarged hull ");
    p = append_int(buf, p, (int)(Rectangle_get_width(hull) + 0.5));
    p = append_str(buf, p, "x");
    p = append_int(buf, p, (int)(Rectangle_get_height(hull) + 0.5));
    buf[p] = '\0';

    HTMLElement meta = Document_getElementById(doc, "meta");
    if (meta.h != 0) {
        Node_set_textContent(meta, buf);
    }
}

__attribute__((export_name("start"))) void start(void) {
    g_app = Application_new();
    Application_init_options opts = Application_init_options_create();
    Application_init_options_set_width(opts, js_some_double(680.0));
    Application_init_options_set_height(opts, js_some_double(400.0));
    Application_init_options_set_backgroundColor(opts, js_number((double)0xffffff));
    Application_init_options_set_preference(opts, string_or_RendererPreference_array_of_string(JsString_from("webgl")));
    Application_init_options_set_preserveDrawingBuffer(opts, js_some_bool(true));
    Application_init_options_set_antialias(opts, js_some_bool(true));
    Application_init_options_set_autoStart(opts, js_some_bool(false));
    Promise p = Application_init(g_app, opts);
    Promise_then(p, on_init, 0);
}
