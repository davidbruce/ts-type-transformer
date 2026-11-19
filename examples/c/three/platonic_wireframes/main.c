#include "dom/dom.h"
#include "three/three.h"

static double my_sin(double x) {
    const double PI2 = 6.283185307179586;
    while (x > 3.141592653589793) x -= PI2;
    while (x < -3.141592653589793) x += PI2;
    double x2 = x * x;
    return x * (1.0 - x2 / 6.0 * (1.0 - x2 / 20.0 * (1.0 - x2 / 42.0 * (1.0 - x2 / 72.0))));
}

static double my_cos(double x) {
    return my_sin(x + 1.5707963267948966);
}

typedef struct { char buf[256]; int pos; } StrBuf;
static void sb_ch(StrBuf* s, char c) { if (s->pos < 254) s->buf[s->pos++] = c; }
static void sb_str(StrBuf* s, const char* p) { while (*p) sb_ch(s, *p++); }
static void sb_int(StrBuf* s, int v) {
    if (v < 0) { sb_ch(s, '-'); v = -v; }
    char tmp[16]; int n = 0;
    do { tmp[n++] = (char)('0' + (v % 10)); v /= 10; } while (v > 0);
    while (n--) sb_ch(s, tmp[n]);
}

static Vector3 v3(double x, double y, double z) {
    return Vector3_new(js_some_double(x), js_some_double(y), js_some_double(z));
}

static void add_obj(Object3D parent, Object3D child) {
    JsArray_Object3D arr = JsArray_Object3D_new();
    JsArray_Object3D_push(arr, child);
    Object3D_add(parent, arr);
    js_release(arr);
}

static WebGLRenderer g_renderer;
static Scene g_scene;
static PerspectiveCamera g_camera;
static Group g_wire_group;
static HTMLElement g_wire_btn;
static HTMLElement g_meta;
static bool g_wire_visible = true;
static double g_azimuth = 0.35;
static char g_meta_buf[256];

static void add_solid(BufferGeometry geo, uint32_t hex, double rough, double metal, bool flat,
                      double x, double y, double z, double thresh, LineBasicMaterial line_mat) {
    MeshStandardMaterialParameters mp = MeshStandardMaterialParameters_create();
    MeshStandardMaterialParameters_set_color(mp, ColorRepresentation_of_number((double)hex));
    MeshStandardMaterialParameters_set_roughness(mp, js_some_double(rough));
    MeshStandardMaterialParameters_set_metalness(mp, js_some_double(metal));
    MeshStandardMaterialParameters_set_flatShading(mp, js_some_bool(flat));
    MeshStandardMaterial mat = MeshStandardMaterial_new(mp);

    Mesh mesh = Mesh_new(geo, mat);
    Object3D m_obj = Mesh_as_Object3D(mesh);
    Vector3_set(Object3D_get_position(m_obj), x, y, js_some_double(z));
    Euler_set_x(Object3D_get_rotation(m_obj), 0.25);
    Euler_set_y(Object3D_get_rotation(m_obj), 0.45);
    add_obj(Scene_as_Object3D(g_scene), m_obj);

    EdgesGeometry egeo = EdgesGeometry_new(geo, js_some_double(thresh));
    LineSegments segs = LineSegments_new(EdgesGeometry_as_BufferGeometry(egeo), LineBasicMaterial_as_Material(line_mat));
    Object3D s_obj = LineSegments_as_Object3D(segs);
    Vector3_set(Object3D_get_position(s_obj), x, y, js_some_double(z));
    Euler_set_x(Object3D_get_rotation(s_obj), 0.25);
    Euler_set_y(Object3D_get_rotation(s_obj), 0.45);
    add_obj(Group_as_Object3D(g_wire_group), s_obj);
}

static void update_view(void) {
    double r = 8.6;
    Vector3_set(Object3D_get_position(PerspectiveCamera_as_Object3D(g_camera)),
                r * my_sin(g_azimuth), 3.6, js_some_double(r * my_cos(g_azimuth)));
    Vector3 origin = v3(0.0, 0.0, 0.0);
    Object3D_lookAt(PerspectiveCamera_as_Object3D(g_camera), origin);
    js_release(origin);

    Object3D_set_visible(Group_as_Object3D(g_wire_group), g_wire_visible);
    Node_set_textContent(g_wire_btn, g_wire_visible ? "Wireframes: On" : "Wireframes: Off");
    WebGLRenderer_render(g_renderer, g_scene, g_camera);

    int deg = (int)(g_azimuth * 180.0 / 3.141592653589793 + (g_azimuth >= 0.0 ? 0.5 : -0.5));
    StrBuf sb = { .pos = 0 };
    sb_str(&sb, "5 monochrome solids \xC2\xB7 EdgesGeometry wireframes: ");
    sb_str(&sb, g_wire_visible ? "on" : "off");
    sb_str(&sb, " \xC2\xB7 azimuth: ");
    sb_int(&sb, deg);
    sb_str(&sb, " deg \xC2\xB7 680x400 WebGL");
    sb.buf[sb.pos] = 0;
    for (int i = 0; i <= sb.pos; i++) g_meta_buf[i] = sb.buf[i];
    Node_set_textContent(g_meta, g_meta_buf);
}

static const int BTN_TAGS[4] = { 0, 1, 2, 3 };
static void on_btn(void* ctx, PointerEvent ev) {
    (void)ev;
    int id = *(const int*)ctx;
    if (id == 0) g_wire_visible = !g_wire_visible;
    else if (id == 1) g_azimuth -= 3.141592653589793 / 6.0;
    else if (id == 2) g_azimuth += 3.141592653589793 / 6.0;
    else { g_azimuth = 0.35; g_wire_visible = true; }
    update_view();
}

__attribute__((export_name("start"))) void start(void) {
    Document doc = get_document();
    double width = 680.0, height = 400.0;

    WebGLRendererParameters rp = WebGLRendererParameters_create();
    WebGLRendererParameters_set_antialias(rp, js_some_bool(true));
    WebGLRendererParameters_set_preserveDrawingBuffer(rp, js_some_bool(true));
    g_renderer = WebGLRenderer_new(rp);
    WebGLRenderer_setPixelRatio(g_renderer, 1.0);
    WebGLRenderer_setSize(g_renderer, width, height);

    HTMLElement stage = Document_getElementById(doc, "stage");
    Node_appendChild(stage, WebGLRenderer_get_domElement(g_renderer));

    g_scene = Scene_new();
    Scene_set_background(g_scene, Color_or_Texture_of_Color(Color_new(ColorRepresentation_of_number(0xf7f7f7))));
    g_camera = PerspectiveCamera_new(js_some_double(42.0), js_some_double(width / height), js_some_double(0.1), js_some_double(100.0));

    add_obj(Scene_as_Object3D(g_scene), AmbientLight_as_Object3D(AmbientLight_new(ColorRepresentation_of_number(0xffffff), js_some_double(1.2))));
    DirectionalLight key = DirectionalLight_new(ColorRepresentation_of_number(0xffffff), js_some_double(2.2));
    Vector3_set(Object3D_get_position(DirectionalLight_as_Object3D(key)), 6.0, 10.0, js_some_double(8.0));
    add_obj(Scene_as_Object3D(g_scene), DirectionalLight_as_Object3D(key));
    DirectionalLight rim = DirectionalLight_new(ColorRepresentation_of_number(0xffffff), js_some_double(0.9));
    Vector3_set(Object3D_get_position(DirectionalLight_as_Object3D(rim)), -7.0, 5.0, js_some_double(-6.0));
    add_obj(Scene_as_Object3D(g_scene), DirectionalLight_as_Object3D(rim));

    GridHelper grid = GridHelper_new(js_some_double(12.0), js_some_double(12.0),
                                     ColorRepresentation_of_number(0x737373),
                                     ColorRepresentation_of_number(0xd4d4d4));
    Vector3_set_y(Object3D_get_position(GridHelper_as_Object3D(grid)), -1.2);
    add_obj(Scene_as_Object3D(g_scene), GridHelper_as_Object3D(grid));

    g_wire_group = Group_new();
    add_obj(Scene_as_Object3D(g_scene), Group_as_Object3D(g_wire_group));

    LineBasicMaterialParameters lmp = LineBasicMaterialParameters_create();
    LineBasicMaterialParameters_set_color(lmp, ColorRepresentation_of_number(0x111111));
    LineBasicMaterial line_mat = LineBasicMaterial_new(lmp);

    add_solid(BoxGeometry_as_BufferGeometry(BoxGeometry_new(js_some_double(1.45), js_some_double(1.45), js_some_double(1.45))),
              0x262626, 0.65, 0.1, false, -3.2, 0.0, -0.6, 1.0, line_mat);
    add_solid(IcosahedronGeometry_as_BufferGeometry(IcosahedronGeometry_new(js_some_double(0.95), js_some_double(0.0))),
              0xd4d4d4, 0.55, 0.08, true, -1.55, 0.1, 0.55, 1.0, line_mat);
    add_solid(TorusKnotGeometry_as_BufferGeometry(TorusKnotGeometry_new(js_some_double(0.72), js_some_double(0.24), js_some_double(96.0), js_some_double(16.0))),
              0x737373, 0.45, 0.15, false, 0.0, 0.25, -0.2, 24.0, line_mat);
    add_solid(OctahedronGeometry_as_BufferGeometry(OctahedronGeometry_new(js_some_double(0.95), js_some_double(0.0))),
              0xefefef, 0.5, 0.05, true, 1.6, 0.1, 0.55, 1.0, line_mat);
    add_solid(TorusGeometry_as_BufferGeometry(TorusGeometry_new(js_some_double(0.72), js_some_double(0.23), js_some_double(16.0), js_some_double(48.0))),
              0x404040, 0.55, 0.12, false, 3.2, 0.05, -0.5, 22.0, line_mat);

    g_wire_btn = Document_getElementById(doc, "btn-wire");
    g_meta = Document_getElementById(doc, "meta");

    static const char* ids[4] = { "btn-wire", "btn-left", "btn-right", "btn-reset" };
    for (int i = 0; i < 4; i++) {
        HTMLElement b = Document_getElementById(doc, ids[i]);
        HTMLElement_addEventListener_click(b, HTMLElement_PointerEvent_fn_from(on_btn, (void*)&BTN_TAGS[i]));
    }
    update_view();
}
