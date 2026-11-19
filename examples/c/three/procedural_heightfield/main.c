#include "dom/dom.h"
#include "three/three.h"

static double my_sin(double x) {
    const double PI2 = 6.283185307179586;
    while (x > 3.141592653589793) x -= PI2;
    while (x < -3.141592653589793) x += PI2;
    double x2 = x * x;
    return x * (1.0 - x2 / 6.0 * (1.0 - x2 / 20.0 * (1.0 - x2 / 42.0 * (1.0 - x2 / 72.0))));
}
static double my_cos(double x) { return my_sin(x + 1.5707963267948966); }
static double my_sqrt(double x) { return __builtin_sqrt(x); }

typedef struct { char buf[256]; int pos; } StrBuf;
static void sb_ch(StrBuf* s, char c) { if (s->pos < 254) s->buf[s->pos++] = c; }
static void sb_str(StrBuf* s, const char* p) { while (*p) sb_ch(s, *p++); }
static void sb_int(StrBuf* s, int v) {
    if (v < 0) { sb_ch(s, '-'); v = -v; }
    char tmp[16]; int n = 0;
    do { tmp[n++] = (char)('0' + (v % 10)); v /= 10; } while (v > 0);
    while (n--) sb_ch(s, tmp[n]);
}
static void sb_fixed2(StrBuf* s, double v) {
    if (v < 0.0) { sb_ch(s, '-'); v = -v; }
    int scaled = (int)(v * 100.0 + 0.5);
    sb_int(s, scaled / 100);
    sb_ch(s, '.');
    int frac = scaled % 100;
    sb_ch(s, (char)('0' + (frac / 10)));
    sb_ch(s, (char)('0' + (frac % 10)));
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

static double wave_height(int mode, double x, double z) {
    if (mode == 1) {
        double r = my_sqrt(x * x + z * z);
        return 0.78 * my_cos(r * 2.2) / (1.0 + 0.18 * r * r);
    } else if (mode == 2) {
        return 0.065 * (x * x - z * z) + 0.18 * my_sin(x * 1.6);
    }
    return 0.55 * my_cos(x * 1.4) * my_sin(z * 1.4) + 0.25 * my_cos((x + z) * 1.1);
}

static const char* wave_name(int mode) {
    if (mode == 1) return "Radial Ripple";
    if (mode == 2) return "Saddle Ridge";
    return "Interference";
}

#define GRID_N 28

static WebGLRenderer g_renderer;
static Scene g_scene;
static PerspectiveCamera g_camera;
static BufferGeometry g_geo;
static Float32BufferAttribute g_pos_attr;
static Float32BufferAttribute g_pts_attr;
static Mesh g_wire_mesh;
static HTMLElement g_wire_btn;
static HTMLElement g_meta;
static int g_tri_count = 0;
static int g_wave_mode = 0;
static bool g_overlay_on = true;
static double g_azimuth = 0.62;
static char g_meta_buf[256];

static void rebuild_field(void) {
    double span = 7.6, half = span / 2.0;
    for (int iz = 0; iz < GRID_N; iz++) {
        for (int ix = 0; ix < GRID_N; ix++) {
            int idx = iz * GRID_N + ix;
            double x = -half + span * (double)ix / (double)(GRID_N - 1);
            double z = -half + span * (double)iz / (double)(GRID_N - 1);
            double y = wave_height(g_wave_mode, x, z);
            js_release(BufferAttribute_setXYZ(g_pos_attr, (double)idx, x, y, z));
        }
    }
    int pi = 0;
    for (int iz = 0; iz < GRID_N; iz += 2) {
        for (int ix = 0; ix < GRID_N; ix += 2) {
            double x = -half + span * (double)ix / (double)(GRID_N - 1);
            double z = -half + span * (double)iz / (double)(GRID_N - 1);
            double y = wave_height(g_wave_mode, x, z) + 0.06;
            js_release(BufferAttribute_setXYZ(g_pts_attr, (double)pi, x, y, z));
            pi++;
        }
    }
    BufferAttribute_set_needsUpdate(g_pos_attr, true);
    BufferAttribute_set_needsUpdate(g_pts_attr, true);
    BufferGeometry_computeVertexNormals(g_geo);
    BufferGeometry_computeBoundingBox(g_geo);

    double r = 9.2;
    Vector3_set(Object3D_get_position(PerspectiveCamera_as_Object3D(g_camera)),
                r * my_sin(g_azimuth), 5.4, js_some_double(r * my_cos(g_azimuth)));
    Vector3 orig = v3(0.0, 0.0, 0.0);
    Object3D_lookAt(PerspectiveCamera_as_Object3D(g_camera), orig);
    js_release(orig);

    Object3D_set_visible(Mesh_as_Object3D(g_wire_mesh), g_overlay_on);
    Node_set_textContent(g_wire_btn, g_overlay_on ? "Wireframe: On" : "Wireframe: Off");
    WebGLRenderer_render(g_renderer, g_scene, g_camera);

    Box3 bb = BufferGeometry_get_boundingBox(g_geo);
    double min_y = Vector3_get_y(Box3_get_min(bb));
    double max_y = Vector3_get_y(Box3_get_max(bb));

    StrBuf sb = { .pos = 0 };
    sb_str(&sb, "profile: "); sb_str(&sb, wave_name(g_wave_mode));
    sb_str(&sb, " \xC2\xB7 "); sb_int(&sb, GRID_N * GRID_N);
    sb_str(&sb, " vertices \xC2\xB7 "); sb_int(&sb, g_tri_count);
    sb_str(&sb, " triangles \xC2\xB7 y=["); sb_fixed2(&sb, min_y);
    sb_str(&sb, ", "); sb_fixed2(&sb, max_y);
    sb_str(&sb, "] \xC2\xB7 wireframe: "); sb_str(&sb, g_overlay_on ? "on" : "off");
    sb.buf[sb.pos] = 0;
    for (int i = 0; i <= sb.pos; i++) g_meta_buf[i] = sb.buf[i];
    Node_set_textContent(g_meta, g_meta_buf);
}

static const int BTN_TAGS[5] = { 0, 1, 2, 3, 4 };
static void on_btn(void* ctx, PointerEvent ev) {
    (void)ev;
    int id = *(const int*)ctx;
    if (id < 3) g_wave_mode = id;
    else if (id == 3) g_overlay_on = !g_overlay_on;
    else g_azimuth += 3.141592653589793 / 4.0;
    rebuild_field();
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
    Node_appendChild(Document_getElementById(doc, "stage"), WebGLRenderer_get_domElement(g_renderer));

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

    GridHelper grid = GridHelper_new(js_some_double(9.0), js_some_double(12.0),
                                     ColorRepresentation_of_number(0x999999), ColorRepresentation_of_number(0xe5e5e5));
    Vector3_set_y(Object3D_get_position(GridHelper_as_Object3D(grid)), -1.1);
    add_obj(Scene_as_Object3D(g_scene), GridHelper_as_Object3D(grid));

    g_geo = BufferGeometry_new();
    g_pos_attr = Float32BufferAttribute_new_with_number((double)(GRID_N * GRID_N * 3), 3.0);
    JsArray_double indices = JsArray_double_new();
    for (int iz = 0; iz < GRID_N - 1; iz++) {
        for (int ix = 0; ix < GRID_N - 1; ix++) {
            double a = (double)(iz * GRID_N + ix);
            double b = a + 1.0;
            double c = a + (double)GRID_N;
            double d = c + 1.0;
            JsArray_double_push(indices, a);
            JsArray_double_push(indices, c);
            JsArray_double_push(indices, b);
            JsArray_double_push(indices, b);
            JsArray_double_push(indices, c);
            JsArray_double_push(indices, d);
        }
    }
    g_tri_count = (GRID_N - 1) * (GRID_N - 1) * 2;
    BufferGeometry_setAttribute(g_geo, "position", Float32BufferAttribute_as_BufferAttribute(g_pos_attr));
    BufferGeometry_setIndex_with_opt_number_array(g_geo, indices);
    js_release(indices);

    MeshStandardMaterialParameters smp = MeshStandardMaterialParameters_create();
    MeshStandardMaterialParameters_set_color(smp, ColorRepresentation_of_number(0xb5b5b5));
    MeshStandardMaterialParameters_set_roughness(smp, js_some_double(0.72));
    MeshStandardMaterialParameters_set_metalness(smp, js_some_double(0.05));
    MeshStandardMaterialParameters_set_flatShading(smp, js_some_bool(true));
    Mesh surface_mesh = Mesh_new(g_geo, MeshStandardMaterial_new(smp));
    add_obj(Scene_as_Object3D(g_scene), Mesh_as_Object3D(surface_mesh));

    MeshBasicMaterialParameters wmp = MeshBasicMaterialParameters_create();
    MeshBasicMaterialParameters_set_color(wmp, ColorRepresentation_of_number(0x262626));
    MeshBasicMaterialParameters_set_wireframe(wmp, js_some_bool(true));
    g_wire_mesh = Mesh_new(g_geo, MeshBasicMaterial_new(wmp));
    Vector3_set_y(Object3D_get_position(Mesh_as_Object3D(g_wire_mesh)), 0.015);
    add_obj(Scene_as_Object3D(g_scene), Mesh_as_Object3D(g_wire_mesh));

    int pt_count = (GRID_N / 2) * (GRID_N / 2);
    BufferGeometry pts_geo = BufferGeometry_new();
    g_pts_attr = Float32BufferAttribute_new_with_number((double)(pt_count * 3), 3.0);
    BufferGeometry_setAttribute(pts_geo, "position", Float32BufferAttribute_as_BufferAttribute(g_pts_attr));
    PointsMaterialParameters pmp = PointsMaterialParameters_create();
    PointsMaterialParameters_set_color(pmp, ColorRepresentation_of_number(0x111111));
    PointsMaterialParameters_set_size(pmp, js_some_double(0.14));
    Points pts = Points_new(pts_geo, PointsMaterial_new(pmp));
    add_obj(Scene_as_Object3D(g_scene), Points_as_Object3D(pts));

    g_wire_btn = Document_getElementById(doc, "btn-wire");
    g_meta = Document_getElementById(doc, "meta");

    static const char* ids[5] = { "btn-p0", "btn-p1", "btn-p2", "btn-wire", "btn-orbit" };
    for (int i = 0; i < 5; i++) {
        HTMLElement b = Document_getElementById(doc, ids[i]);
        HTMLElement_addEventListener_click(b, HTMLElement_PointerEvent_fn_from(on_btn, (void*)&BTN_TAGS[i]));
    }
    rebuild_field();
}
