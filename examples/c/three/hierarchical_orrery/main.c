#include "dom/dom.h"
#include "three/three.h"

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

static MeshStandardMaterial std_mat(uint32_t hex, double rough, double metal, bool flat) {
    MeshStandardMaterialParameters mp = MeshStandardMaterialParameters_create();
    MeshStandardMaterialParameters_set_color(mp, ColorRepresentation_of_number((double)hex));
    MeshStandardMaterialParameters_set_roughness(mp, js_some_double(rough));
    MeshStandardMaterialParameters_set_metalness(mp, js_some_double(metal));
    MeshStandardMaterialParameters_set_flatShading(mp, js_some_bool(flat));
    return MeshStandardMaterial_new(mp);
}

static WebGLRenderer g_renderer;
static Scene g_scene;
static PerspectiveCamera g_camera;
static Group g_inner_pivot;
static Group g_moon_pivot;
static Group g_outer_pivot;
static Group g_outer_station;
static Mesh g_moon;
static Mesh g_station_body;
static HTMLElement g_tilt_btn;
static HTMLElement g_meta;
static double g_epoch = 0.6;
static bool g_tilted = false;
static char g_meta_buf[256];

static void update_orrery(void) {
    Euler_set_y(Object3D_get_rotation(Group_as_Object3D(g_inner_pivot)), g_epoch);
    Euler_set_y(Object3D_get_rotation(Group_as_Object3D(g_moon_pivot)), g_epoch * 2.5);
    Euler_set_y(Object3D_get_rotation(Group_as_Object3D(g_outer_pivot)), -g_epoch * 0.55);
    Euler_set_z(Object3D_get_rotation(Group_as_Object3D(g_outer_pivot)), g_tilted ? 0.38 : 0.0);
    Euler_set_y(Object3D_get_rotation(Mesh_as_Object3D(g_station_body)), g_epoch * 1.4);

    Object3D_updateMatrixWorld(Scene_as_Object3D(g_scene), js_some_bool(true));
    Vector3 mpos = Object3D_getWorldPosition(Mesh_as_Object3D(g_moon), v3(0.0, 0.0, 0.0));
    Vector3 spos = Object3D_getWorldPosition(Group_as_Object3D(g_outer_station), v3(0.0, 0.0, 0.0));

    Node_set_textContent(g_tilt_btn, g_tilted ? "Outer Tilt: On" : "Outer Tilt: Off");
    WebGLRenderer_render(g_renderer, g_scene, g_camera);

    int deg = (int)(g_epoch * 180.0 / 3.141592653589793 + 0.5);
    StrBuf sb = { .pos = 0 };
    sb_str(&sb, "epoch: "); sb_int(&sb, deg); sb_str(&sb, " deg \xC2\xB7 moon world: (");
    sb_fixed2(&sb, Vector3_get_x(mpos)); sb_str(&sb, ", ");
    sb_fixed2(&sb, Vector3_get_y(mpos)); sb_str(&sb, ", ");
    sb_fixed2(&sb, Vector3_get_z(mpos)); sb_str(&sb, ") \xC2\xB7 station world: (");
    sb_fixed2(&sb, Vector3_get_x(spos)); sb_str(&sb, ", ");
    sb_fixed2(&sb, Vector3_get_y(spos)); sb_str(&sb, ", ");
    sb_fixed2(&sb, Vector3_get_z(spos)); sb_str(&sb, ")");
    sb.buf[sb.pos] = 0;
    for (int i = 0; i <= sb.pos; i++) g_meta_buf[i] = sb.buf[i];
    Node_set_textContent(g_meta, g_meta_buf);
    js_release(mpos);
    js_release(spos);
}

static const int BTN_TAGS[3] = { 0, 1, 2 };
static void on_btn(void* ctx, PointerEvent ev) {
    (void)ev;
    int id = *(const int*)ctx;
    if (id == 0) g_epoch += 3.141592653589793 / 6.0;
    else if (id == 1) g_tilted = !g_tilted;
    else { g_epoch = 0.6; g_tilted = false; }
    update_orrery();
}

__attribute__((export_name("start"))) void start(void) {
    const double PI = 3.141592653589793;
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
    Vector3_set(Object3D_get_position(PerspectiveCamera_as_Object3D(g_camera)), 5.6, 5.2, js_some_double(7.6));
    Object3D_lookAt(PerspectiveCamera_as_Object3D(g_camera), v3(0.0, -0.1, 0.0));

    add_obj(Scene_as_Object3D(g_scene), AmbientLight_as_Object3D(AmbientLight_new(ColorRepresentation_of_number(0xffffff), js_some_double(1.2))));
    DirectionalLight key = DirectionalLight_new(ColorRepresentation_of_number(0xffffff), js_some_double(2.2));
    Vector3_set(Object3D_get_position(DirectionalLight_as_Object3D(key)), 6.0, 10.0, js_some_double(8.0));
    add_obj(Scene_as_Object3D(g_scene), DirectionalLight_as_Object3D(key));
    DirectionalLight rim = DirectionalLight_new(ColorRepresentation_of_number(0xffffff), js_some_double(0.9));
    Vector3_set(Object3D_get_position(DirectionalLight_as_Object3D(rim)), -7.0, 5.0, js_some_double(-6.0));
    add_obj(Scene_as_Object3D(g_scene), DirectionalLight_as_Object3D(rim));

    GridHelper grid = GridHelper_new(js_some_double(12.0), js_some_double(12.0),
                                     ColorRepresentation_of_number(0x999999), ColorRepresentation_of_number(0xe5e5e5));
    Vector3_set_y(Object3D_get_position(GridHelper_as_Object3D(grid)), -1.5);
    add_obj(Scene_as_Object3D(g_scene), GridHelper_as_Object3D(grid));

    Mesh pedestal = Mesh_new(CylinderGeometry_new(js_some_double(0.55), js_some_double(0.75), js_some_double(0.35), js_some_double(24.0)),
                             std_mat(0x262626, 0.7, 0.05, false));
    Vector3_set_y(Object3D_get_position(Mesh_as_Object3D(pedestal)), -1.3);
    add_obj(Scene_as_Object3D(g_scene), Mesh_as_Object3D(pedestal));

    Mesh core = Mesh_new(SphereGeometry_new(js_some_double(0.65), js_some_double(24.0), js_some_double(18.0)),
                         std_mat(0x111111, 0.45, 0.2, false));
    add_obj(Scene_as_Object3D(g_scene), Mesh_as_Object3D(core));

    g_inner_pivot = Group_new();
    add_obj(Scene_as_Object3D(g_scene), Group_as_Object3D(g_inner_pivot));

    MeshBasicMaterialParameters tp1 = MeshBasicMaterialParameters_create();
    MeshBasicMaterialParameters_set_color(tp1, ColorRepresentation_of_number(0x999999));
    Mesh inner_track = Mesh_new(TorusGeometry_new(js_some_double(2.1), js_some_double(0.025), js_some_double(8.0), js_some_double(64.0)),
                                MeshBasicMaterial_new(tp1));
    Euler_set_x(Object3D_get_rotation(Mesh_as_Object3D(inner_track)), PI / 2.0);
    add_obj(Group_as_Object3D(g_inner_pivot), Mesh_as_Object3D(inner_track));

    Group inner_planet = Group_new();
    Vector3_set(Object3D_get_position(Group_as_Object3D(inner_planet)), 2.1, 0.0, js_some_double(0.0));
    add_obj(Group_as_Object3D(g_inner_pivot), Group_as_Object3D(inner_planet));

    Mesh planet_mesh = Mesh_new(SphereGeometry_new(js_some_double(0.36), js_some_double(20.0), js_some_double(16.0)),
                                std_mat(0x555555, 0.55, 0.1, false));
    add_obj(Group_as_Object3D(inner_planet), Mesh_as_Object3D(planet_mesh));

    g_moon_pivot = Group_new();
    add_obj(Group_as_Object3D(inner_planet), Group_as_Object3D(g_moon_pivot));

    g_moon = Mesh_new(OctahedronGeometry_new(js_some_double(0.18), js_some_double(0.0)),
                      std_mat(0xd4d4d4, 0.4, 0.1, true));
    Vector3_set(Object3D_get_position(Mesh_as_Object3D(g_moon)), 0.75, 0.0, js_some_double(0.0));
    add_obj(Group_as_Object3D(g_moon_pivot), Mesh_as_Object3D(g_moon));

    g_outer_pivot = Group_new();
    add_obj(Scene_as_Object3D(g_scene), Group_as_Object3D(g_outer_pivot));

    MeshBasicMaterialParameters tp2 = MeshBasicMaterialParameters_create();
    MeshBasicMaterialParameters_set_color(tp2, ColorRepresentation_of_number(0xb5b5b5));
    Mesh outer_track = Mesh_new(TorusGeometry_new(js_some_double(3.8), js_some_double(0.025), js_some_double(8.0), js_some_double(80.0)),
                                MeshBasicMaterial_new(tp2));
    Euler_set_x(Object3D_get_rotation(Mesh_as_Object3D(outer_track)), PI / 2.0);
    add_obj(Group_as_Object3D(g_outer_pivot), Mesh_as_Object3D(outer_track));

    g_outer_station = Group_new();
    Vector3_set(Object3D_get_position(Group_as_Object3D(g_outer_station)), 3.8, 0.0, js_some_double(0.0));
    add_obj(Group_as_Object3D(g_outer_pivot), Group_as_Object3D(g_outer_station));

    g_station_body = Mesh_new(IcosahedronGeometry_new(js_some_double(0.42), js_some_double(0.0)),
                              std_mat(0x404040, 0.5, 0.1, true));
    add_obj(Group_as_Object3D(g_outer_station), Mesh_as_Object3D(g_station_body));

    Mesh halo = Mesh_new(TorusGeometry_new(js_some_double(0.68), js_some_double(0.06), js_some_double(10.0), js_some_double(32.0)),
                         std_mat(0x111111, 0.4, 0.1, false));
    Euler_set_x(Object3D_get_rotation(Mesh_as_Object3D(halo)), 0.8);
    add_obj(Group_as_Object3D(g_outer_station), Mesh_as_Object3D(halo));

    g_tilt_btn = Document_getElementById(doc, "btn-tilt");
    g_meta = Document_getElementById(doc, "meta");

    static const char* ids[3] = { "btn-step", "btn-tilt", "btn-reset" };
    for (int i = 0; i < 3; i++) {
        HTMLElement b = Document_getElementById(doc, ids[i]);
        HTMLElement_addEventListener_click(b, HTMLElement_PointerEvent_fn_from(on_btn, (void*)&BTN_TAGS[i]));
    }
    update_orrery();
}
