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
static double my_abs(double x) { return x < 0.0 ? -x : x; }
static double my_exp(double x) {
    if (x < -30.0) return 0.0;
    int k = (int)(x / 0.6931471805599453 + (x >= 0.0 ? 0.5 : -0.5));
    double r = x - (double)k * 0.6931471805599453;
    double er = 1.0 + r * (1.0 + r * (0.5 + r * (1.0 / 6.0 + r * (1.0 / 24.0 + r / 120.0))));
    if (k > 0) while (k--) er *= 2.0;
    else if (k < 0) while (k++) er *= 0.5;
    return er;
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

static double tower_height(int mode, int gx, int gz) {
    double fx = (double)gx - 4.5;
    double fz = (double)gz - 4.5;
    if (mode == 1) {
        double r = my_sqrt(fx * fx + fz * fz);
        return 0.35 + 0.42 * r + 0.25 * my_cos(fx * 0.8);
    } else if (mode == 2) {
        double diag = my_abs(fx - fz);
        return 0.3 + 2.8 * my_exp(-diag * diag / 5.5) + 0.25 * my_sin(fz * 0.9);
    }
    double r2 = fx * fx + fz * fz;
    return 0.35 + 3.1 * my_exp(-r2 / 9.5) + 0.2 * my_cos(fx * 1.4) * my_cos(fz * 1.4);
}

static const char* layout_name(int mode) {
    if (mode == 1) return "Amphitheater";
    if (mode == 2) return "Diagonal";
    return "Plaza";
}

static WebGLRenderer g_renderer;
static Scene g_scene;
static OrthographicCamera g_ortho;
static PerspectiveCamera g_persp;
static InstancedMesh g_inst;
static HTMLElement g_cam_btn;
static HTMLElement g_meta;
static bool g_use_ortho = true;
static int g_layout_mode = 0;
static double g_cam_yaw = 0.785398;
static char g_cam_buf[64];
static char g_meta_buf[256];

static void render_city(void) {
    Vector3 pos = v3(0.0, 0.0, 0.0);
    Quaternion rot_q = Quaternion_new(js_none_double(), js_none_double(), js_none_double(), js_none_double());
    Vector3 scale = v3(1.0, 1.0, 1.0);
    Matrix4 mtx = Matrix4_new();
    Vector3 axis_y = v3(0.0, 1.0, 0.0);
    Color col = Color_new(ColorRepresentation_of_number(0xffffff));

    double max_h = 0.0;
    int idx = 0;
    for (int gz = 0; gz < 10; gz++) {
        for (int gx = 0; gx < 10; gx++) {
            double x = ((double)gx - 4.5) * 0.78;
            double z = ((double)gz - 4.5) * 0.78;
            double h = tower_height(g_layout_mode, gx, gz);
            if (h > max_h) max_h = h;
            Vector3_set(pos, x, h * 0.5, js_some_double(z));
            Quaternion_setFromAxisAngle(rot_q, axis_y, 0.08 * (double)((gx + gz) % 4));
            Vector3_set(scale, 1.0, h, js_some_double(1.0));
            Matrix4_compose(mtx, pos, rot_q, scale);
            InstancedMesh_setMatrixAt(g_inst, (double)idx, mtx);

            double t = (h - 0.3) / 3.2;
            if (t < 0.0) t = 0.0;
            if (t > 1.0) t = 1.0;
            double shade = 0.82 - 0.70 * t;
            Color_setRGB(col, shade, shade, shade);
            InstancedMesh_setColorAt(g_inst, (double)idx, col);
            idx++;
        }
    }

    js_release(pos); js_release(rot_q); js_release(scale);
    js_release(mtx); js_release(axis_y); js_release(col);

    InstancedBufferAttribute imat = InstancedMesh_get_instanceMatrix(g_inst);
    BufferAttribute_set_needsUpdate(imat, true);
    js_release(imat);
    InstancedBufferAttribute icol = InstancedMesh_get_instanceColor(g_inst);
    if (icol.h) {
        BufferAttribute_set_needsUpdate(icol, true);
        js_release(icol);
    }

    double radius = 11.5;
    double cx = radius * my_cos(g_cam_yaw);
    double cz = radius * my_sin(g_cam_yaw);
    Vector3 target = v3(0.0, 0.9, 0.0);

    Vector3_set(Object3D_get_position(OrthographicCamera_as_Object3D(g_ortho)), cx, 8.2, js_some_double(cz));
    Object3D_lookAt(OrthographicCamera_as_Object3D(g_ortho), target);
    Vector3_set(Object3D_get_position(PerspectiveCamera_as_Object3D(g_persp)), cx, 7.6, js_some_double(cz));
    Object3D_lookAt(PerspectiveCamera_as_Object3D(g_persp), target);
    js_release(target);

    if (g_use_ortho) {
        WebGLRenderer_render(g_renderer, g_scene, g_ortho);
    } else {
        WebGLRenderer_render(g_renderer, g_scene, g_persp);
    }

    const char* cname = g_use_ortho ? "Orthographic" : "Perspective";
    StrBuf csb = { .pos = 0 };
    sb_str(&csb, "Camera: "); sb_str(&csb, cname); csb.buf[csb.pos] = 0;
    for (int i = 0; i <= csb.pos; i++) g_cam_buf[i] = csb.buf[i];
    Node_set_textContent(g_cam_btn, g_cam_buf);

    StrBuf sb = { .pos = 0 };
    sb_str(&sb, "100 InstancedMesh towers \xC2\xB7 layout: "); sb_str(&sb, layout_name(g_layout_mode));
    sb_str(&sb, " \xC2\xB7 peak height: "); sb_fixed2(&sb, max_h);
    sb_str(&sb, " \xC2\xB7 camera: "); sb_str(&sb, cname);
    sb.buf[sb.pos] = 0;
    for (int i = 0; i <= sb.pos; i++) g_meta_buf[i] = sb.buf[i];
    Node_set_textContent(g_meta, g_meta_buf);
}

static const int BTN_TAGS[5] = { 0, 1, 2, 3, 4 };
static void on_btn(void* ctx, PointerEvent ev) {
    (void)ev;
    int id = *(const int*)ctx;
    if (id == 0) g_use_ortho = !g_use_ortho;
    else if (id >= 1 && id <= 3) g_layout_mode = id - 1;
    else g_cam_yaw += 3.141592653589793 / 4.0;
    render_city();
}

__attribute__((export_name("start"))) void start(void) {
    Document doc = get_document();
    double width = 680.0, height = 400.0, aspect = width / height;

    WebGLRendererParameters rp = WebGLRendererParameters_create();
    WebGLRendererParameters_set_antialias(rp, js_some_bool(true));
    WebGLRendererParameters_set_preserveDrawingBuffer(rp, js_some_bool(true));
    g_renderer = WebGLRenderer_new(rp);
    WebGLRenderer_setPixelRatio(g_renderer, 1.0);
    WebGLRenderer_setSize(g_renderer, width, height);
    Node_appendChild(Document_getElementById(doc, "stage"), WebGLRenderer_get_domElement(g_renderer));

    g_scene = Scene_new();
    Scene_set_background(g_scene, Color_or_Texture_of_Color(Color_new(ColorRepresentation_of_number(0xf7f7f7))));
    g_ortho = OrthographicCamera_new(js_some_double(-5.4 * aspect), js_some_double(5.4 * aspect),
                                     js_some_double(5.4), js_some_double(-5.4),
                                     js_some_double(0.1), js_some_double(100.0));
    g_persp = PerspectiveCamera_new(js_some_double(40.0), js_some_double(aspect),
                                    js_some_double(0.1), js_some_double(100.0));

    add_obj(Scene_as_Object3D(g_scene), AmbientLight_as_Object3D(AmbientLight_new(ColorRepresentation_of_number(0xffffff), js_some_double(1.25))));
    DirectionalLight key = DirectionalLight_new(ColorRepresentation_of_number(0xffffff), js_some_double(2.1));
    Vector3_set(Object3D_get_position(DirectionalLight_as_Object3D(key)), 8.0, 12.0, js_some_double(6.0));
    add_obj(Scene_as_Object3D(g_scene), DirectionalLight_as_Object3D(key));
    DirectionalLight rim = DirectionalLight_new(ColorRepresentation_of_number(0xffffff), js_some_double(0.9));
    Vector3_set(Object3D_get_position(DirectionalLight_as_Object3D(rim)), -7.0, 6.0, js_some_double(-8.0));
    add_obj(Scene_as_Object3D(g_scene), DirectionalLight_as_Object3D(rim));

    MeshStandardMaterialParameters pmp = MeshStandardMaterialParameters_create();
    MeshStandardMaterialParameters_set_color(pmp, ColorRepresentation_of_number(0xe5e5e5));
    MeshStandardMaterialParameters_set_roughness(pmp, js_some_double(0.85));
    Mesh plinth = Mesh_new(PlaneGeometry_new(js_some_double(8.6), js_some_double(8.6)), MeshStandardMaterial_new(pmp));
    Euler_set_x(Object3D_get_rotation(Mesh_as_Object3D(plinth)), -3.141592653589793 / 2.0);
    Vector3_set_y(Object3D_get_position(Mesh_as_Object3D(plinth)), -0.01);
    add_obj(Scene_as_Object3D(g_scene), Mesh_as_Object3D(plinth));

    GridHelper grid = GridHelper_new(js_some_double(9.0), js_some_double(10.0),
                                     ColorRepresentation_of_number(0x999999), ColorRepresentation_of_number(0xd4d4d4));
    add_obj(Scene_as_Object3D(g_scene), GridHelper_as_Object3D(grid));

    BoxGeometry tgeo = BoxGeometry_new(js_some_double(0.54), js_some_double(1.0), js_some_double(0.54));
    MeshStandardMaterialParameters tmp = MeshStandardMaterialParameters_create();
    MeshStandardMaterialParameters_set_color(tmp, ColorRepresentation_of_number(0xffffff));
    MeshStandardMaterialParameters_set_roughness(tmp, js_some_double(0.65));
    MeshStandardMaterialParameters_set_metalness(tmp, js_some_double(0.08));
    MeshStandardMaterial tmat = MeshStandardMaterial_new(tmp);
    g_inst = InstancedMesh_new(BoxGeometry_as_BufferGeometry(tgeo), MeshStandardMaterial_as_Material(tmat), 100.0);
    add_obj(Scene_as_Object3D(g_scene), InstancedMesh_as_Object3D(g_inst));

    g_cam_btn = Document_getElementById(doc, "btn-cam");
    g_meta = Document_getElementById(doc, "meta");

    static const char* ids[5] = { "btn-cam", "btn-lay0", "btn-lay1", "btn-lay2", "btn-rot" };
    for (int i = 0; i < 5; i++) {
        HTMLElement b = Document_getElementById(doc, ids[i]);
        HTMLElement_addEventListener_click(b, HTMLElement_PointerEvent_fn_from(on_btn, (void*)&BTN_TAGS[i]));
    }
    render_city();
}
