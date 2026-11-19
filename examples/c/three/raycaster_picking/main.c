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

static const char* NAMES[9] = {
    "Box", "Cone", "Cylinder", "Sphere", "Octahedron",
    "Dodecahedron", "Torus", "Icosahedron", "Tetrahedron"
};

static WebGLRenderer g_renderer;
static HTMLCanvasElement g_canvas;
static Scene g_scene;
static PerspectiveCamera g_camera;
static MeshStandardMaterial g_mats[9];
static Object3D g_meshes[9];
static JsArray_Object3D g_mesh_arr;
static Box3 g_box;
static Box3Helper g_helper;
static Raycaster g_raycaster;
static HTMLElement g_meta;
static double g_yaw = 0.45;
static int g_selected = -1;
static char g_hit_buf[160] = "none";
static char g_meta_buf[256];

static void update_camera(void) {
    double r = 8.4;
    Vector3_set(Object3D_get_position(PerspectiveCamera_as_Object3D(g_camera)),
                r * my_sin(g_yaw), 5.1, js_some_double(r * my_cos(g_yaw)));
    Vector3 orig = v3(0.0, 0.0, 0.0);
    Object3D_lookAt(PerspectiveCamera_as_Object3D(g_camera), orig);
    js_release(orig);
    Object3D_updateMatrixWorld(PerspectiveCamera_as_Object3D(g_camera), js_some_bool(true));
    Object3D_updateMatrixWorld(Scene_as_Object3D(g_scene), js_some_bool(true));
}

static void render_scene(void) {
    for (int i = 0; i < 9; i++) {
        Color c = MeshStandardMaterialProperties_get_color(MeshStandardMaterial_as_MeshStandardMaterialProperties(g_mats[i]));
        Color_setHex(c, i == g_selected ? (double)0x111111 : (double)0xd4d4d4);
        js_release(c);
    }
    if (g_selected >= 0 && g_selected < 9) {
        Box3_setFromObject(g_box, g_meshes[g_selected], js_some_bool(true));
        Object3D_set_visible(Box3Helper_as_Object3D(g_helper), true);
    } else {
        Object3D_set_visible(Box3Helper_as_Object3D(g_helper), false);
    }
    WebGLRenderer_render(g_renderer, g_scene, g_camera);

    StrBuf sb = { .pos = 0 };
    sb_str(&sb, "picked: "); sb_str(&sb, g_hit_buf);
    sb_str(&sb, " \xC2\xB7 9 specimens \xC2\xB7 click canvas or buttons");
    sb.buf[sb.pos] = 0;
    for (int i = 0; i <= sb.pos; i++) g_meta_buf[i] = sb.buf[i];
    Node_set_textContent(g_meta, g_meta_buf);
}

static void cast_at_ndc(double nx, double ny) {
    Vector2 ndc = Vector2_new(js_some_double(nx), js_some_double(ny));
    Raycaster_setFromCamera(g_raycaster, ndc, g_camera);
    js_release(ndc);

    JsArray_Intersection hits = Raycaster_intersectObjects(g_raycaster, g_mesh_arr, js_some_bool(false));
    if (JsArray_Intersection_length(hits) > 0) {
        Intersection hit = JsArray_Intersection_get(hits, 0);
        Object3D hobj = Intersection_get_object(hit);
        int found = -1;
        for (int i = 0; i < 9; i++) {
            if (js_same(g_meshes[i], hobj)) { found = i; break; }
        }
        g_selected = found;
        if (found >= 0) {
            Box3_setFromObject(g_box, g_meshes[found], js_some_bool(true));
            Vector3 sz = Box3_getSize(g_box, v3(0.0, 0.0, 0.0));
            Vector3 pt = Intersection_get_point(hit);
            StrBuf sb = { .pos = 0 };
            sb_str(&sb, NAMES[found]); sb_str(&sb, " at d=");
            sb_fixed2(&sb, Intersection_get_distance(hit)); sb_str(&sb, " (");
            sb_fixed2(&sb, Vector3_get_x(pt)); sb_str(&sb, ", ");
            sb_fixed2(&sb, Vector3_get_y(pt)); sb_str(&sb, ", ");
            sb_fixed2(&sb, Vector3_get_z(pt)); sb_str(&sb, ") box=");
            sb_fixed2(&sb, Vector3_get_x(sz)); sb_str(&sb, "x");
            sb_fixed2(&sb, Vector3_get_y(sz)); sb_str(&sb, "x");
            sb_fixed2(&sb, Vector3_get_z(sz));
            sb.buf[sb.pos] = 0;
            for (int i = 0; i <= sb.pos && i < 159; i++) g_hit_buf[i] = sb.buf[i];
            g_hit_buf[159] = 0;
            js_release(sz); js_release(pt);
        }
        js_release(hobj); js_release(hit);
    } else {
        g_selected = -1;
        StrBuf sb = { .pos = 0 };
        sb_str(&sb, "miss at ndc ("); sb_fixed2(&sb, nx); sb_str(&sb, ", "); sb_fixed2(&sb, ny); sb_str(&sb, ")");
        sb.buf[sb.pos] = 0;
        for (int i = 0; i <= sb.pos && i < 159; i++) g_hit_buf[i] = sb.buf[i];
    }
    js_release(hits);
    render_scene();
}

static void pick_specimen(int idx) {
    update_camera();
    Vector3 wp = Object3D_getWorldPosition(g_meshes[idx], v3(0.0, 0.0, 0.0));
    Vector3 proj = Vector3_project(wp, g_camera);
    cast_at_ndc(Vector3_get_x(proj), Vector3_get_y(proj));
    js_release(proj);
}

static void on_canvas_click(void* ctx, PointerEvent ev) {
    (void)ctx;
    DOMRect rect = Element_getBoundingClientRect(g_canvas);
    double nx = ((MouseEvent_get_clientX(ev) - DOMRectReadOnly_get_left(rect)) / DOMRectReadOnly_get_width(rect)) * 2.0 - 1.0;
    double ny = -((MouseEvent_get_clientY(ev) - DOMRectReadOnly_get_top(rect)) / DOMRectReadOnly_get_height(rect)) * 2.0 + 1.0;
    js_release(rect);
    update_camera();
    cast_at_ndc(nx, ny);
}

static const int BTN_TAGS[5] = { 0, 1, 2, 3, 4 };
static void on_btn(void* ctx, PointerEvent ev) {
    (void)ev;
    int id = *(const int*)ctx;
    if (id == 0) pick_specimen(4);
    else if (id == 1) pick_specimen(0);
    else if (id == 2) pick_specimen(8);
    else if (id == 3) { g_yaw += 3.141592653589793 / 4.0; update_camera(); render_scene(); }
    else {
        g_selected = -1;
        g_hit_buf[0] = 'n'; g_hit_buf[1] = 'o'; g_hit_buf[2] = 'n'; g_hit_buf[3] = 'e'; g_hit_buf[4] = 0;
        update_camera(); render_scene();
    }
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
    g_canvas = WebGLRenderer_get_domElement(g_renderer);
    Node_appendChild(Document_getElementById(doc, "stage"), g_canvas);

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

    GridHelper grid = GridHelper_new(js_some_double(9.0), js_some_double(9.0),
                                     ColorRepresentation_of_number(0x999999), ColorRepresentation_of_number(0xd4d4d4));
    Vector3_set_y(Object3D_get_position(GridHelper_as_Object3D(grid)), -0.85);
    add_obj(Scene_as_Object3D(g_scene), GridHelper_as_Object3D(grid));

    BufferGeometry geos[9];
    geos[0] = BoxGeometry_as_BufferGeometry(BoxGeometry_new(js_some_double(1.05), js_some_double(1.05), js_some_double(1.05)));
    geos[1] = ConeGeometry_as_BufferGeometry(ConeGeometry_new(js_some_double(0.62), js_some_double(1.25), js_some_double(20.0)));
    geos[2] = CylinderGeometry_as_BufferGeometry(CylinderGeometry_new(js_some_double(0.48), js_some_double(0.48), js_some_double(1.2), js_some_double(24.0)));
    geos[3] = SphereGeometry_as_BufferGeometry(SphereGeometry_new(js_some_double(0.62), js_some_double(24.0), js_some_double(18.0)));
    geos[4] = OctahedronGeometry_as_BufferGeometry(OctahedronGeometry_new(js_some_double(0.68), js_some_double(0.0)));
    geos[5] = DodecahedronGeometry_as_BufferGeometry(DodecahedronGeometry_new(js_some_double(0.62), js_some_double(0.0)));
    geos[6] = TorusGeometry_as_BufferGeometry(TorusGeometry_new(js_some_double(0.5), js_some_double(0.18), js_some_double(14.0), js_some_double(36.0)));
    geos[7] = IcosahedronGeometry_as_BufferGeometry(IcosahedronGeometry_new(js_some_double(0.64), js_some_double(0.0)));
    geos[8] = TetrahedronGeometry_as_BufferGeometry(TetrahedronGeometry_new(js_some_double(0.72), js_some_double(0.0)));

    g_mesh_arr = JsArray_Object3D_new();
    for (int i = 0; i < 9; i++) {
        int gx = (i % 3) - 1;
        int gz = (i / 3) - 1;
        MeshStandardMaterialParameters mp = MeshStandardMaterialParameters_create();
        MeshStandardMaterialParameters_set_color(mp, ColorRepresentation_of_number(0xd4d4d4));
        MeshStandardMaterialParameters_set_roughness(mp, js_some_double(0.55));
        MeshStandardMaterialParameters_set_metalness(mp, js_some_double(0.1));
        MeshStandardMaterialParameters_set_flatShading(mp, js_some_bool(i >= 4));
        g_mats[i] = MeshStandardMaterial_new(mp);

        Mesh m = Mesh_new(geos[i], MeshStandardMaterial_as_Material(g_mats[i]));
        Object3D mo = Mesh_as_Object3D(m);
        Object3D_set_name(mo, NAMES[i]);
        Vector3_set(Object3D_get_position(mo), (double)gx * 2.1, 0.0, js_some_double((double)gz * 2.1));
        Euler_set_x(Object3D_get_rotation(mo), 0.2);
        Euler_set_y(Object3D_get_rotation(mo), 0.35 * (double)i);
        add_obj(Scene_as_Object3D(g_scene), mo);
        g_meshes[i] = mo;
        JsArray_Object3D_push(g_mesh_arr, mo);
    }

    g_box = Box3_new(v3(-0.5, -0.5, -0.5), v3(0.5, 0.5, 0.5));
    g_helper = Box3Helper_new(g_box, ColorRepresentation_of_number(0x111111));
    Object3D_set_visible(Box3Helper_as_Object3D(g_helper), false);
    add_obj(Scene_as_Object3D(g_scene), Box3Helper_as_Object3D(g_helper));

    g_raycaster = Raycaster_new(v3(0.0, 0.0, 0.0), v3(0.0, 0.0, -1.0));
    g_meta = Document_getElementById(doc, "meta");

    HTMLElement_addEventListener_click(g_canvas, HTMLElement_PointerEvent_fn_from(on_canvas_click, (void*)0));

    static const char* ids[5] = { "btn-p0", "btn-p1", "btn-p2", "btn-orbit", "btn-clear" };
    for (int i = 0; i < 5; i++) {
        HTMLElement b = Document_getElementById(doc, ids[i]);
        HTMLElement_addEventListener_click(b, HTMLElement_PointerEvent_fn_from(on_btn, (void*)&BTN_TAGS[i]));
    }
    pick_specimen(4);
}
