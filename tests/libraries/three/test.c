// three.js from C, in Node: vector, matrix and rotation math, boxes and
// spheres, a scene graph with world positions, a mesh hit by a ray, and a
// geometry's attributes. Every CHECK must hold.
#include "bindings.h"

TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

// A number to within what doubles of this size agree on.
static bool near(double a, double b) { return (a > b ? a - b : b - a) < 1e-9; }
static bool near_f(double a, double b) { return (a > b ? a - b : b - a) < 1e-5; }

// Optional numbers, as a vector's constructor takes them.
static Vector3 V3(double x, double y, double z) { return Vector3_new(js_some_double(x), js_some_double(y), js_some_double(z)); }
static bool is(Vector3 v, double x, double y, double z) { return near(Vector3_get_x(v), x) && near(Vector3_get_y(v), y) && near(Vector3_get_z(v), z); }

static bool str_is(JsString s, const char* expected) {
    bool ok = JsString_equals(s, expected);
    js_release(s);
    return ok;
}

__attribute__((export_name("run"))) void run(void) {
    const double PI = 3.14159265358979323846;

    // ---- Vector3 ----
    Vector3 a = V3(1, 2, 3), b = V3(4, 5, 6);
    CHECK(Vector3_get_x(a) == 1 && Vector3_get_y(a) == 2 && Vector3_get_z(a) == 3);
    Vector3 sum = Vector3_clone(a);
    Vector3_add(sum, b);
    CHECK(is(sum, 5, 7, 9));
    CHECK(is(a, 1, 2, 3));
    CHECK(Vector3_dot(a, b) == 32);
    Vector3 cross = Vector3_cross(V3(1, 0, 0), V3(0, 1, 0));
    CHECK(is(cross, 0, 0, 1));
    CHECK(Vector3_length(V3(3, 4, 0)) == 5);
    CHECK(Vector3_distanceTo(a, b) > 5.19 && Vector3_distanceTo(a, b) < 5.2);
    Vector3 unit = Vector3_normalize(V3(0, 0, 9));
    CHECK(is(unit, 0, 0, 1));
    Vector3 mid = Vector3_lerp(V3(0, 0, 0), V3(10, 20, 30), 0.5);
    CHECK(is(mid, 5, 10, 15));
    Vector3 scaled = Vector3_multiplyScalar(V3(1, 2, 3), 2);
    CHECK(is(scaled, 2, 4, 6));
    Vector3 neg = Vector3_negate(V3(1, -2, 3));
    CHECK(is(neg, -1, 2, -3));
    Vector3_set(scaled, 7, 8);
    CHECK(Vector3_get_x(scaled) == 7 && Vector3_get_y(scaled) == 8);
    Vector3_set_z(scaled, 9);
    CHECK(Vector3_getComponent(scaled, 2) == 9);
    // A vector iterates by its components.
    JsIterator it = Vector3_iterate(b);
    double comp, total = 0;
    int n = 0;
    while (Vector3_iterate_next(it, &comp)) { total += comp; n++; }
    js_release(it);
    CHECK(n == 3 && total == 15);

    // ---- Matrix4 ----
    Matrix4 m = Matrix4_new();
    CHECK(Matrix4_determinant(m) == 1);
    Matrix4 t = Matrix4_makeTranslation_with_x_and_y_and_z(Matrix4_new(), 10, 20, 30);
    JsArray els = Matrix4_get_elements(t);
    CHECK(JsArray_length(els) == 16);
    CHECK(js_to_number(JsArray_get(els, 12)) == 10 && js_to_number(JsArray_get(els, 14)) == 30);
    Vector3 moved = Vector3_applyMatrix4(V3(1, 1, 1), t);
    CHECK(is(moved, 11, 21, 31));
    Matrix4 s = Matrix4_makeScale(Matrix4_new(), 2, 3, 4);
    CHECK(Matrix4_determinant(s) == 24);
    Matrix4 ts = Matrix4_multiply(Matrix4_clone(t), s);
    Vector3 both = Vector3_applyMatrix4(V3(1, 1, 1), ts);
    CHECK(is(both, 12, 23, 34));
    Matrix4 inv = Matrix4_invert(Matrix4_clone(ts));
    Vector3 back = Vector3_applyMatrix4(V3(12, 23, 34), inv);
    CHECK(is(back, 1, 1, 1));
    CHECK(Matrix4_equals(Matrix4_clone(t), t));
    CHECK(!Matrix4_equals(s, t));
    // Compose and decompose.
    Quaternion qy = Quaternion_setFromAxisAngle(Quaternion_new(js_none_double(), js_none_double(), js_none_double(), js_none_double()), V3(0, 1, 0), PI / 2);
    Matrix4 composed = Matrix4_compose(Matrix4_new(), V3(1, 2, 3), qy, V3(2, 2, 2));
    Vector3 pos = V3(0, 0, 0), scl = V3(1, 1, 1);
    Quaternion rot = Quaternion_new(js_none_double(), js_none_double(), js_none_double(), js_none_double());
    Matrix4_decompose(composed, pos, rot, scl);
    CHECK(is(pos, 1, 2, 3));
    CHECK(near_f(Vector3_get_x(scl), 2) && near_f(Vector3_get_y(scl), 2));
    CHECK(near_f(Quaternion_get_y(rot), Quaternion_get_y(qy)));

    // ---- Quaternion and Euler ----
    CHECK(near(Quaternion_length(qy), 1));
    Vector3 turned = Vector3_applyQuaternion(V3(1, 0, 0), qy);
    CHECK(near_f(Vector3_get_x(turned), 0) && near_f(Vector3_get_z(turned), -1));
    Euler e = Euler_new(js_some_double(0.1), js_some_double(0.2), js_some_double(0.3), EulerOrder_XYZ);
    CHECK(Euler_get_order(e) == EulerOrder_XYZ && near(Euler_get_y(e), 0.2));
    Quaternion qe = Quaternion_setFromEuler(Quaternion_new(js_none_double(), js_none_double(), js_none_double(), js_none_double()), e);
    Euler e2 = Euler_setFromQuaternion(Euler_new(js_none_double(), js_none_double(), js_none_double(), EulerOrder_XYZ), qe, EulerOrder_XYZ);
    CHECK(near_f(Euler_get_x(e2), 0.1) && near_f(Euler_get_y(e2), 0.2) && near_f(Euler_get_z(e2), 0.3));
    Quaternion qid = Quaternion_identity(Quaternion_new(js_none_double(), js_none_double(), js_none_double(), js_none_double()));
    CHECK(Quaternion_get_w(qid) == 1);
    Quaternion halfway = Quaternion_slerp(Quaternion_clone(qid), qy, 0.5);
    CHECK(near_f(Quaternion_angleTo(qid, halfway), PI / 4));

    // ---- Color ----
    ColorRepresentation orange = { .tag = ColorRepresentation_number, .as.number = 0xff8000 };
    Color c = Color_new(orange);
    CHECK(Color_get_r(c) == 1 && Color_get_b(c) == 0);
    CHECK(Color_getHex(c) == 0xff8000);
    CHECK(str_is(Color_getHexString(c), "ff8000"));
    Color named = Color_new(ColorRepresentation_of_string(JsString_from("red")));
    CHECK(Color_get_r(named) == 1 && Color_get_g(named) == 0);
    Color rgb = Color_new_with_r_and_g_and_b(0.25, 0.5, 0.75);
    CHECK(Color_get_g(rgb) == 0.5);
    Color_setRGB(rgb, 1, 1, 1, (const char*)0);
    CHECK(Color_getHex(rgb) == 0xffffff);
    Color_lerp(rgb, Color_new(ColorRepresentation_of_number(0)), 0.5);
    CHECK(near(Color_get_r(rgb), 0.5));

    // ---- Box3 and Sphere ----
    Box3 box = Box3_new(V3(-1, -1, -1), V3(1, 2, 3));
    Vector3 size = Box3_getSize(box, V3(0, 0, 0));
    CHECK(is(size, 2, 3, 4));
    Vector3 center = Box3_getCenter(box, V3(0, 0, 0));
    CHECK(is(center, 0, 0.5, 1));
    CHECK(Box3_containsPoint(box, V3(0, 0, 0)) && !Box3_containsPoint(box, V3(5, 0, 0)));
    Box3_expandByPoint(box, V3(5, 0, 0));
    CHECK(Box3_containsPoint(box, V3(5, 0, 0)));
    CHECK(Vector3_get_x(Box3_get_max(box)) == 5);
    Box3 other = Box3_new(V3(4, 0, 0), V3(8, 1, 1));
    CHECK(Box3_intersectsBox(box, other));
    Box3 far_box = Box3_new(V3(50, 50, 50), V3(60, 60, 60));
    CHECK(!Box3_intersectsBox(box, far_box));
    Sphere ball = Sphere_new(V3(0, 0, 0), js_some_double(2));
    CHECK(Sphere_get_radius(ball) == 2);
    CHECK(Sphere_containsPoint(ball, V3(1, 1, 0)) && !Sphere_containsPoint(ball, V3(2, 2, 0)));
    CHECK(near(Sphere_distanceToPoint(ball, V3(5, 0, 0)), 3));
    CHECK(Sphere_intersectsBox(ball, Box3_new(V3(1, 1, 1), V3(3, 3, 3))));
    Box3 ball_box = Sphere_getBoundingBox(ball, Box3_new(V3(0, 0, 0), V3(0, 0, 0)));
    CHECK(is(Box3_get_max(ball_box), 2, 2, 2));

    // ---- Scene graph ----
    Group group = Group_new();
    Object3D_set_name(group, "pivot");
    Vector3_set(Object3D_get_position(group), 10, 0, js_some_double(0));
    // A mesh is an Object3D: it goes where a base is wanted.
    BoxGeometry geometry = BoxGeometry_new(js_some_double(2), js_some_double(2), js_some_double(2), js_none_double(), js_none_double(), js_none_double());
    MeshBasicMaterialParameters params = MeshBasicMaterialParameters_create();
    MeshBasicMaterialParameters_set_color(params, ColorRepresentation_of_number(0x00ff00));
    MeshBasicMaterial material = MeshBasicMaterial_new(params);
    Mesh mesh = Mesh_new(geometry, material);
    Object3D_set_name(mesh, "box");
    Vector3_set(Object3D_get_position(mesh), 1, 2, js_some_double(3));
    JsArray_Object3D kids = JsArray_Object3D_new();
    JsArray_Object3D_push(kids, Mesh_as_Object3D(mesh));
    Object3D_add(group, kids);
    CHECK(JsArray_Object3D_length(Object3D_get_children(group)) == 1);
    CHECK(js_same(Object3D_get_parent(mesh), group));
    CHECK(js_same(Object3D_getObjectByName(group, "box"), mesh));
    CHECK(Object3D_getObjectByName(group, "nothing").h == 0);
    CHECK(str_is(Object3D_get_name(group), "pivot"));
    CHECK(str_is(Mesh_get_type(mesh), "Mesh"));
    CHECK(str_is(Object3D_get_type(group), "Group"));
    CHECK(Object3D_get_visible(mesh));

    // World positions need the matrices updated.
    Object3D_updateMatrixWorld(group);
    Vector3 world = Object3D_getWorldPosition(mesh, V3(0, 0, 0));
    CHECK(is(world, 11, 2, 3));
    Vector3 local_to_world = Object3D_localToWorld(mesh, V3(0, 0, 0));
    CHECK(is(local_to_world, 11, 2, 3));
    // Rotating the group a half turn about Y swings the mesh.
    Object3D_rotateY(group, PI);
    Object3D_updateMatrixWorld(group);
    Vector3 swung = Object3D_getWorldPosition(mesh, V3(0, 0, 0));
    CHECK(near_f(Vector3_get_x(swung), 9) && near_f(Vector3_get_z(swung), -3));
    Object3D_rotateY(group, PI);
    Object3D_updateMatrixWorld(group);
    // Scale.
    Vector3 gs = Object3D_getWorldScale(mesh, V3(0, 0, 0));
    CHECK(is(gs, 1, 1, 1));

    // ---- Geometry attributes ----
    CHECK(str_is(BoxGeometry_get_type(geometry), "BoxGeometry"));
    CHECK(BufferGeometry_hasAttribute(geometry, "position"));
    NormalOrGLBufferAttributes_index_result pos_attr = BufferGeometry_getAttribute(geometry, "position");
    CHECK(pos_attr.tag == 0);
    BufferAttribute positions = pos_attr.as.BufferAttribute;
    CHECK(BufferAttribute_get_count(positions) == 24 && BufferAttribute_get_itemSize(positions) == 3);
    CHECK(BufferGeometry_hasAttribute(geometry, "normal") && BufferGeometry_hasAttribute(geometry, "uv"));
    CHECK(!BufferGeometry_hasAttribute(geometry, "color"));
    BufferAttribute index = BufferGeometry_getIndex(geometry);
    CHECK(BufferAttribute_get_count(index) == 36);
    BufferGeometry_computeBoundingBox(geometry);
    Box3 bb = BufferGeometry_get_boundingBox(geometry);
    CHECK(is(Box3_get_min(bb), -1, -1, -1) && is(Box3_get_max(bb), 1, 1, 1));
    BufferGeometry_computeBoundingSphere(geometry);
    CHECK(near_f(Sphere_get_radius(BufferGeometry_get_boundingSphere(geometry)), 1.7320508));
    BoxGeometry_parameters bp = BoxGeometry_get_parameters(geometry);
    CHECK(bp.h != 0);

    // ---- Raycaster ----
    // From in front of the box, along -z: it enters the front face at z = 4 + 1.
    Object3D_updateMatrixWorld(mesh);
    Object3D_updateMatrixWorld(group);
    Raycaster ray = Raycaster_new(V3(11, 2, 10), V3(0, 0, -1), js_none_double(), js_none_double());
    JsArray_Intersection hits = Raycaster_intersectObject(ray, mesh);
    CHECK(JsArray_Intersection_length(hits) == 1);
    Intersection hit = JsArray_Intersection_get(hits, 0);
    CHECK(near_f(Intersection_get_distance(hit), 6));
    CHECK(is(Intersection_get_point(hit), 11, 2, 4));
    CHECK(js_same(Intersection_get_object(hit), mesh));
    CHECK(Intersection_get_faceIndex(hit).has);
    // A ray past the box misses.
    Raycaster miss = Raycaster_new(V3(30, 2, 10), V3(0, 0, -1), js_none_double(), js_none_double());
    CHECK(JsArray_Intersection_length(Raycaster_intersectObject(miss, mesh)) == 0);
    // Through the group, recursively.
    JsArray_Intersection deep = Raycaster_intersectObject(ray, group, js_some_bool(true));
    CHECK(JsArray_Intersection_length(deep) == 1);
    JsArray_Intersection shallow = Raycaster_intersectObject(ray, group, js_some_bool(false));
    CHECK(JsArray_Intersection_length(shallow) == 0);
    // near and far clip the ray.
    Raycaster_set_far(ray, 3);
    CHECK(JsArray_Intersection_length(Raycaster_intersectObject(ray, mesh)) == 0);

    // Removing from the parent.
    Object3D_removeFromParent(mesh);
    CHECK(JsArray_Object3D_length(Object3D_get_children(group)) == 0);
    CHECK(Object3D_get_parent(mesh).h == 0);
}
