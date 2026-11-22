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
static Three_Vector3 V3(double x, double y, double z) { return Three_Vector3_new(js_some_double(x), js_some_double(y), js_some_double(z)); }
static bool is(Three_Vector3 v, double x, double y, double z) { return near(Three_Vector3_get_x(v), x) && near(Three_Vector3_get_y(v), y) && near(Three_Vector3_get_z(v), z); }

static bool str_is(JsString s, const char* expected) {
    bool ok = JsString_equals(s, expected);
    js_release(s);
    return ok;
}

__attribute__((export_name("run"))) void run(void) {
    const double PI = 3.14159265358979323846;

    // ---- Vector3 ----
    Three_Vector3 a = V3(1, 2, 3), b = V3(4, 5, 6);
    CHECK(Three_Vector3_get_x(a) == 1 && Three_Vector3_get_y(a) == 2 && Three_Vector3_get_z(a) == 3);
    Three_Vector3 sum = Three_Vector3_clone(a);
    Three_Vector3_add(sum, b);
    CHECK(is(sum, 5, 7, 9));
    CHECK(is(a, 1, 2, 3));
    CHECK(Three_Vector3_dot(a, b) == 32);
    Three_Vector3 cross = Three_Vector3_cross(V3(1, 0, 0), V3(0, 1, 0));
    CHECK(is(cross, 0, 0, 1));
    CHECK(Three_Vector3_length(V3(3, 4, 0)) == 5);
    CHECK(Three_Vector3_distanceTo(a, b) > 5.19 && Three_Vector3_distanceTo(a, b) < 5.2);
    Three_Vector3 unit = Three_Vector3_normalize(V3(0, 0, 9));
    CHECK(is(unit, 0, 0, 1));
    Three_Vector3 mid = Three_Vector3_lerp(V3(0, 0, 0), V3(10, 20, 30), 0.5);
    CHECK(is(mid, 5, 10, 15));
    Three_Vector3 scaled = Three_Vector3_multiplyScalar(V3(1, 2, 3), 2);
    CHECK(is(scaled, 2, 4, 6));
    Three_Vector3 neg = Three_Vector3_negate(V3(1, -2, 3));
    CHECK(is(neg, -1, 2, -3));
    Three_Vector3_set(scaled, 7, 8);
    CHECK(Three_Vector3_get_x(scaled) == 7 && Three_Vector3_get_y(scaled) == 8);
    Three_Vector3_set_z(scaled, 9);
    CHECK(Three_Vector3_getComponent(scaled, 2) == 9);
    // A vector iterates by its components.
    JsIterator it = Three_Vector3_iterate(b);
    double comp, total = 0;
    int n = 0;
    while (Three_Vector3_iterate_next(it, &comp)) { total += comp; n++; }
    js_release(it);
    CHECK(n == 3 && total == 15);

    // ---- Matrix4 ----
    Three_Matrix4 m = Three_Matrix4_new();
    CHECK(Three_Matrix4_determinant(m) == 1);
    Three_Matrix4 t = Three_Matrix4_makeTranslation_with_x_and_y_and_z(Three_Matrix4_new(), 10, 20, 30);
    JsArray els = Three_Matrix4_get_elements(t);
    CHECK(JsArray_length(els) == 16);
    CHECK(js_to_number(JsArray_get(els, 12)) == 10 && js_to_number(JsArray_get(els, 14)) == 30);
    Three_Vector3 moved = Three_Vector3_applyMatrix4(V3(1, 1, 1), t);
    CHECK(is(moved, 11, 21, 31));
    Three_Matrix4 s = Three_Matrix4_makeScale(Three_Matrix4_new(), 2, 3, 4);
    CHECK(Three_Matrix4_determinant(s) == 24);
    Three_Matrix4 ts = Three_Matrix4_multiply(Three_Matrix4_clone(t), s);
    Three_Vector3 both = Three_Vector3_applyMatrix4(V3(1, 1, 1), ts);
    CHECK(is(both, 12, 23, 34));
    Three_Matrix4 inv = Three_Matrix4_invert(Three_Matrix4_clone(ts));
    Three_Vector3 back = Three_Vector3_applyMatrix4(V3(12, 23, 34), inv);
    CHECK(is(back, 1, 1, 1));
    CHECK(Three_Matrix4_equals(Three_Matrix4_clone(t), t));
    CHECK(!Three_Matrix4_equals(s, t));
    // Compose and decompose.
    Three_Quaternion qy = Three_Quaternion_setFromAxisAngle(Three_Quaternion_new(js_none_double(), js_none_double(), js_none_double(), js_none_double()), V3(0, 1, 0), PI / 2);
    Three_Matrix4 composed = Three_Matrix4_compose(Three_Matrix4_new(), V3(1, 2, 3), qy, V3(2, 2, 2));
    Three_Vector3 pos = V3(0, 0, 0), scl = V3(1, 1, 1);
    Three_Quaternion rot = Three_Quaternion_new(js_none_double(), js_none_double(), js_none_double(), js_none_double());
    Three_Matrix4_decompose(composed, pos, rot, scl);
    CHECK(is(pos, 1, 2, 3));
    CHECK(near_f(Three_Vector3_get_x(scl), 2) && near_f(Three_Vector3_get_y(scl), 2));
    CHECK(near_f(Three_Quaternion_get_y(rot), Three_Quaternion_get_y(qy)));

    // ---- Quaternion and Euler ----
    CHECK(near(Three_Quaternion_length(qy), 1));
    Three_Vector3 turned = Three_Vector3_applyQuaternion(V3(1, 0, 0), qy);
    CHECK(near_f(Three_Vector3_get_x(turned), 0) && near_f(Three_Vector3_get_z(turned), -1));
    Three_Euler e = Three_Euler_new(js_some_double(0.1), js_some_double(0.2), js_some_double(0.3), Three_EulerOrder_XYZ);
    CHECK(Three_Euler_get_order(e) == Three_EulerOrder_XYZ && near(Three_Euler_get_y(e), 0.2));
    Three_Quaternion qe = Three_Quaternion_setFromEuler(Three_Quaternion_new(js_none_double(), js_none_double(), js_none_double(), js_none_double()), e);
    Three_Euler e2 = Three_Euler_setFromQuaternion(Three_Euler_new(js_none_double(), js_none_double(), js_none_double(), Three_EulerOrder_XYZ), qe, Three_EulerOrder_XYZ);
    CHECK(near_f(Three_Euler_get_x(e2), 0.1) && near_f(Three_Euler_get_y(e2), 0.2) && near_f(Three_Euler_get_z(e2), 0.3));
    Three_Quaternion qid = Three_Quaternion_identity(Three_Quaternion_new(js_none_double(), js_none_double(), js_none_double(), js_none_double()));
    CHECK(Three_Quaternion_get_w(qid) == 1);
    Three_Quaternion halfway = Three_Quaternion_slerp(Three_Quaternion_clone(qid), qy, 0.5);
    CHECK(near_f(Three_Quaternion_angleTo(qid, halfway), PI / 4));

    // ---- Color ----
    Three_ColorRepresentation orange = { .tag = Three_ColorRepresentation_number, .as.number = 0xff8000 };
    Three_Color c = Three_Color_new(orange);
    CHECK(Three_Color_get_r(c) == 1 && Three_Color_get_b(c) == 0);
    CHECK(Three_Color_getHex(c) == 0xff8000);
    CHECK(str_is(Three_Color_getHexString(c), "ff8000"));
    Three_Color named = Three_Color_new(Three_ColorRepresentation_of_string(JsString_from("red")));
    CHECK(Three_Color_get_r(named) == 1 && Three_Color_get_g(named) == 0);
    Three_Color rgb = Three_Color_new_with_r_and_g_and_b(0.25, 0.5, 0.75);
    CHECK(Three_Color_get_g(rgb) == 0.5);
    Three_Color_setRGB(rgb, 1, 1, 1, (const char*)0);
    CHECK(Three_Color_getHex(rgb) == 0xffffff);
    Three_Color_lerp(rgb, Three_Color_new(Three_ColorRepresentation_of_number(0)), 0.5);
    CHECK(near(Three_Color_get_r(rgb), 0.5));

    // ---- Box3 and Sphere ----
    Three_Box3 box = Three_Box3_new(V3(-1, -1, -1), V3(1, 2, 3));
    Three_Vector3 size = Three_Box3_getSize(box, V3(0, 0, 0));
    CHECK(is(size, 2, 3, 4));
    Three_Vector3 center = Three_Box3_getCenter(box, V3(0, 0, 0));
    CHECK(is(center, 0, 0.5, 1));
    CHECK(Three_Box3_containsPoint(box, V3(0, 0, 0)) && !Three_Box3_containsPoint(box, V3(5, 0, 0)));
    Three_Box3_expandByPoint(box, V3(5, 0, 0));
    CHECK(Three_Box3_containsPoint(box, V3(5, 0, 0)));
    CHECK(Three_Vector3_get_x(Three_Box3_get_max(box)) == 5);
    Three_Box3 other = Three_Box3_new(V3(4, 0, 0), V3(8, 1, 1));
    CHECK(Three_Box3_intersectsBox(box, other));
    Three_Box3 far_box = Three_Box3_new(V3(50, 50, 50), V3(60, 60, 60));
    CHECK(!Three_Box3_intersectsBox(box, far_box));
    Three_Sphere ball = Three_Sphere_new(V3(0, 0, 0), js_some_double(2));
    CHECK(Three_Sphere_get_radius(ball) == 2);
    CHECK(Three_Sphere_containsPoint(ball, V3(1, 1, 0)) && !Three_Sphere_containsPoint(ball, V3(2, 2, 0)));
    CHECK(near(Three_Sphere_distanceToPoint(ball, V3(5, 0, 0)), 3));
    CHECK(Three_Sphere_intersectsBox(ball, Three_Box3_new(V3(1, 1, 1), V3(3, 3, 3))));
    Three_Box3 ball_box = Three_Sphere_getBoundingBox(ball, Three_Box3_new(V3(0, 0, 0), V3(0, 0, 0)));
    CHECK(is(Three_Box3_get_max(ball_box), 2, 2, 2));

    // ---- Scene graph ----
    Three_Group group = Three_Group_new();
    Three_Object3D_set_name(group, "pivot");
    Three_Vector3_set(Three_Object3D_get_position(group), 10, 0, js_some_double(0));
    // A mesh is an Object3D: it goes where a base is wanted.
    Three_BoxGeometry geometry = Three_BoxGeometry_new(js_some_double(2), js_some_double(2), js_some_double(2), js_none_double(), js_none_double(), js_none_double());
    Three_MeshBasicMaterialParameters params = Three_MeshBasicMaterialParameters_create();
    Three_MeshBasicMaterialParameters_set_color(params, Three_ColorRepresentation_of_number(0x00ff00));
    Three_MeshBasicMaterial material = Three_MeshBasicMaterial_new(params);
    Three_Mesh mesh = Three_Mesh_new(geometry, material);
    Three_Object3D_set_name(mesh, "box");
    Three_Vector3_set(Three_Object3D_get_position(mesh), 1, 2, js_some_double(3));
    JsArray_Three_Object3D kids = JsArray_Three_Object3D_new();
    JsArray_Three_Object3D_push(kids, Three_Mesh_as_Three_Object3D(mesh));
    Three_Object3D_add(group, kids);
    CHECK(JsArray_Three_Object3D_length(Three_Object3D_get_children(group)) == 1);
    CHECK(js_same(Three_Object3D_get_parent(mesh), group));
    CHECK(js_same(Three_Object3D_getObjectByName(group, "box"), mesh));
    CHECK(Three_Object3D_getObjectByName(group, "nothing").h == 0);
    CHECK(str_is(Three_Object3D_get_name(group), "pivot"));
    CHECK(str_is(Three_Mesh_get_type(mesh), "Mesh"));
    CHECK(str_is(Three_Object3D_get_type(group), "Group"));
    CHECK(Three_Object3D_get_visible(mesh));

    // World positions need the matrices updated.
    Three_Object3D_updateMatrixWorld(group);
    Three_Vector3 world = Three_Object3D_getWorldPosition(mesh, V3(0, 0, 0));
    CHECK(is(world, 11, 2, 3));
    Three_Vector3 local_to_world = Three_Object3D_localToWorld(mesh, V3(0, 0, 0));
    CHECK(is(local_to_world, 11, 2, 3));
    // Rotating the group a half turn about Y swings the mesh.
    Three_Object3D_rotateY(group, PI);
    Three_Object3D_updateMatrixWorld(group);
    Three_Vector3 swung = Three_Object3D_getWorldPosition(mesh, V3(0, 0, 0));
    CHECK(near_f(Three_Vector3_get_x(swung), 9) && near_f(Three_Vector3_get_z(swung), -3));
    Three_Object3D_rotateY(group, PI);
    Three_Object3D_updateMatrixWorld(group);
    // Scale.
    Three_Vector3 gs = Three_Object3D_getWorldScale(mesh, V3(0, 0, 0));
    CHECK(is(gs, 1, 1, 1));

    // ---- Geometry attributes ----
    CHECK(str_is(Three_BoxGeometry_get_type(geometry), "BoxGeometry"));
    CHECK(Three_BufferGeometry_hasAttribute(geometry, "position"));
    Three_NormalOrGLBufferAttributes_index_result pos_attr = Three_BufferGeometry_getAttribute(geometry, "position");
    CHECK(pos_attr.tag == 0);
    Three_BufferAttribute positions = pos_attr.as.BufferAttribute;
    CHECK(Three_BufferAttribute_get_count(positions) == 24 && Three_BufferAttribute_get_itemSize(positions) == 3);
    CHECK(Three_BufferGeometry_hasAttribute(geometry, "normal") && Three_BufferGeometry_hasAttribute(geometry, "uv"));
    CHECK(!Three_BufferGeometry_hasAttribute(geometry, "color"));
    Three_BufferAttribute index = Three_BufferGeometry_getIndex(geometry);
    CHECK(Three_BufferAttribute_get_count(index) == 36);
    Three_BufferGeometry_computeBoundingBox(geometry);
    Three_Box3 bb = Three_BufferGeometry_get_boundingBox(geometry);
    CHECK(is(Three_Box3_get_min(bb), -1, -1, -1) && is(Three_Box3_get_max(bb), 1, 1, 1));
    Three_BufferGeometry_computeBoundingSphere(geometry);
    CHECK(near_f(Three_Sphere_get_radius(Three_BufferGeometry_get_boundingSphere(geometry)), 1.7320508));
    Three_BoxGeometry_parameters bp = Three_BoxGeometry_get_parameters(geometry);
    CHECK(bp.h != 0);

    // ---- Raycaster ----
    // From in front of the box, along -z: it enters the front face at z = 4 + 1.
    Three_Object3D_updateMatrixWorld(mesh);
    Three_Object3D_updateMatrixWorld(group);
    Three_Raycaster ray = Three_Raycaster_new(V3(11, 2, 10), V3(0, 0, -1), js_none_double(), js_none_double());
    JsArray_Three_Intersection hits = Three_Raycaster_intersectObject(ray, mesh);
    CHECK(JsArray_Three_Intersection_length(hits) == 1);
    Three_Intersection hit = JsArray_Three_Intersection_get(hits, 0);
    CHECK(near_f(Three_Intersection_get_distance(hit), 6));
    CHECK(is(Three_Intersection_get_point(hit), 11, 2, 4));
    CHECK(js_same(Three_Intersection_get_object(hit), mesh));
    CHECK(Three_Intersection_get_faceIndex(hit).has);
    // A ray past the box misses.
    Three_Raycaster miss = Three_Raycaster_new(V3(30, 2, 10), V3(0, 0, -1), js_none_double(), js_none_double());
    CHECK(JsArray_Three_Intersection_length(Three_Raycaster_intersectObject(miss, mesh)) == 0);
    // Through the group, recursively.
    JsArray_Three_Intersection deep = Three_Raycaster_intersectObject(ray, group, js_some_bool(true));
    CHECK(JsArray_Three_Intersection_length(deep) == 1);
    JsArray_Three_Intersection shallow = Three_Raycaster_intersectObject(ray, group, js_some_bool(false));
    CHECK(JsArray_Three_Intersection_length(shallow) == 0);
    // near and far clip the ray.
    Three_Raycaster_set_far(ray, 3);
    CHECK(JsArray_Three_Intersection_length(Three_Raycaster_intersectObject(ray, mesh)) == 0);

    // Removing from the parent.
    Three_Object3D_removeFromParent(mesh);
    CHECK(JsArray_Three_Object3D_length(Three_Object3D_get_children(group)) == 0);
    CHECK(Three_Object3D_get_parent(mesh).h == 0);
}
