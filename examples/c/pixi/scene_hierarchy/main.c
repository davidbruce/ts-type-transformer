#include "dom/dom.h"
#include "pixi_js/pixi_js.h"

#define TWO_PI_3 (2.0943951023931953)

static Application g_app;

static void stroke_g(Graphics g, double width, double color_hex) {
    StrokeStyle ss = StrokeStyle_create();
    StrokeAttributes_set_width(StrokeStyle_as_StrokeAttributes(ss), js_some_double(width));
    FillStyle_set_color(StrokeStyle_as_FillStyle(ss), js_number(color_hex));
    JsArray args = JsArray_new();
    JsArray_push(args, (JsValue){ ss.h });
    Graphics_stroke(g, args);
}

static void add_child_c(Container parent, Container child) {
    JsArray_Container kids = JsArray_Container_new();
    JsArray_Container_push(kids, child);
    Container_addChild(parent, kids);
}

static int append_str(char* buf, int pos, const char* s) {
    while (*s) buf[pos++] = *s++;
    return pos;
}

static int append_tenths(char* buf, int pos, double val) {
    if (val < 0.0) {
        buf[pos++] = '-';
        val = -val;
    }
    int scaled = (int)(val * 10.0 + 0.5);
    int whole = scaled / 10;
    int frac = scaled % 10;
    char tmp[16];
    int n = 0;
    if (whole == 0) tmp[n++] = '0';
    else {
        while (whole > 0) {
            tmp[n++] = (char)('0' + (whole % 10));
            whole /= 10;
        }
    }
    for (int i = n - 1; i >= 0; i--) buf[pos++] = tmp[i];
    buf[pos++] = '.';
    buf[pos++] = (char)('0' + frac);
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

    Graphics guide = Graphics_new__0();
    Graphics_moveTo(guide, 40.0, 200.0);
    Graphics_lineTo(guide, 640.0, 200.0);
    Graphics_moveTo(guide, 340.0, 24.0);
    Graphics_lineTo(guide, 340.0, 376.0);
    stroke_g(guide, 1.0, (double)0xe5e5e5);
    Graphics_circle(guide, 340.0, 200.0, 110.0);
    stroke_g(guide, 1.0, (double)0xd4d4d4);
    add_child_c(stage, Graphics_as_Container(guide));

    Container root = Container_new((ContainerOptions){ 0 });
    Container_set_x(root, 340.0);
    Container_set_y(root, 200.0);

    for (int i = 0; i < 3; i++) {
        Container arm = Container_new((ContainerOptions){ 0 });
        Container_set_rotation(arm, (double)i * TWO_PI_3);

        Graphics arm_bar = Graphics_new__0();
        Graphics_rect(arm_bar, 0.0, -4.0, 110.0, 8.0);
        Graphics_fill(arm_bar, js_number((double)0x737373));
        Graphics_circle(arm_bar, 110.0, 0.0, 10.0);
        Graphics_fill(arm_bar, js_number((double)0x111111));
        add_child_c(arm, Graphics_as_Container(arm_bar));

        Container sub = Container_new((ContainerOptions){ 0 });
        Container_set_x(sub, 110.0);
        Container_set_rotation(sub, -0.45);

        Graphics sub_bar = Graphics_new__0();
        Graphics_moveTo(sub_bar, -32.0, 0.0);
        Graphics_lineTo(sub_bar, 48.0, 0.0);
        stroke_g(sub_bar, 2.0, (double)0x404040);
        add_child_c(sub, Graphics_as_Container(sub_bar));

        Graphics sat_box = Graphics_new__0();
        Graphics_rect(sat_box, -12.0, -12.0, 24.0, 24.0);
        Graphics_fill(sat_box, js_number((double)0x262626));
        Container_set_x(sat_box, 48.0);
        add_child_c(sub, Graphics_as_Container(sat_box));

        Graphics sat_orb = Graphics_new__0();
        Graphics_circle(sat_orb, 0.0, 0.0, 14.0);
        Graphics_fill(sat_orb, js_number((double)0xd4d4d4));
        stroke_g(sat_orb, 2.0, (double)0x111111);
        Container_set_x(sat_orb, -32.0);
        add_child_c(sub, Graphics_as_Container(sat_orb));

        add_child_c(arm, sub);
        add_child_c(root, arm);
    }

    Graphics hub = Graphics_new__0();
    Graphics_circle(hub, 0.0, 0.0, 18.0);
    Graphics_fill(hub, js_number((double)0x111111));
    Graphics_circle(hub, 0.0, 0.0, 7.0);
    Graphics_fill(hub, js_number((double)0xffffff));
    add_child_c(root, Graphics_as_Container(hub));

    add_child_c(stage, root);
    Application_render(g_app);

    Bounds rb = MeasureMixin_getBounds(root, js_none_bool(), (Bounds){ 0 });
    char buf[160];
    int p = 0;
    p = append_str(buf, p, "3 primary arms · 6 nested satellites · bounds ");
    p = append_tenths(buf, p, Bounds_get_width(rb));
    p = append_str(buf, p, "x");
    p = append_tenths(buf, p, Bounds_get_height(rb));
    p = append_str(buf, p, " · 680x400 WebGL");
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
