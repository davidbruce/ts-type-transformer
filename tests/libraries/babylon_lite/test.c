// @babylonjs/lite from C, in Node: vectors, cameras, lights, materials,
// procedural mesh buffers, accessibility tree, and math tuples. Every CHECK must hold.
#include "bindings.h"

TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

static bool near(double a, double b) {
    double d = a - b;
    return (d < 0 ? -d : d) < 1e-6;
}

__attribute__((export_name("run"))) void run(void) {
    // Vectors.
    Babylon_Vec3 target = babylon_vec3(1.0, 2.0, 3.0);
    CHECK(Babylon_Vec3_get_x(target) == 1.0 && Babylon_Vec3_get_y(target) == 2.0 && Babylon_Vec3_get_z(target) == 3.0);
    Babylon_Vec3_set_y(target, 1.6);
    CHECK(near(Babylon_Vec3_get_y(target), 1.6));

    // Camera.
    Babylon_ArcRotateCamera camera = babylon_createArcRotateCamera(0.75, 1.0, 8.5, target);
    CHECK(near(Babylon_ArcRotateCamera_get_alpha(camera), 0.75) && near(Babylon_ArcRotateCamera_get_beta(camera), 1.0));
    Babylon_Vec3 cam_target = Babylon_ArcRotateCamera_get_target(camera);
    CHECK(Babylon_ArcRotateCamera_get_radius(camera) == 8.5 && near(Babylon_Vec3_get_y(cam_target), 1.6));
    js_release(cam_target);
    js_release(camera);
    js_release(target);

    // Lights.
    JsTuple_double_double_double hemi_dir = JsTuple_double_double_double_new(0.0, 1.0, 0.2);
    Babylon_HemisphericLight hemi = babylon_createHemisphericLight(hemi_dir, js_some_double(0.75));
    JsTuple_double_double_double dc_in = JsTuple_double_double_double_new(0.75, 0.85, 1.0);
    JsTuple_double_double_double gc_in = JsTuple_double_double_double_new(0.12, 0.14, 0.22);
    Babylon_HemisphericLight_set_diffuseColor(hemi, dc_in);
    Babylon_HemisphericLight_set_groundColor(hemi, gc_in);
    JsTuple_double_double_double dc = Babylon_HemisphericLight_get_diffuseColor(hemi);
    JsTuple_double_double_double gc = Babylon_HemisphericLight_get_groundColor(hemi);
    CHECK(near(Babylon_HemisphericLight_get_intensity(hemi), 0.75));
    CHECK(near(JsTuple_double_double_double_get_0(dc), 0.75)
        && near(JsTuple_double_double_double_get_1(dc), 0.85)
        && near(JsTuple_double_double_double_get_2(dc), 1.0));
    CHECK(near(JsTuple_double_double_double_get_0(gc), 0.12));
    js_release(gc); js_release(dc); js_release(gc_in); js_release(dc_in); js_release(hemi); js_release(hemi_dir);

    JsTuple_double_double_double dir_v = JsTuple_double_double_double_new(-1.0, -2.0, -1.2);
    Babylon_DirectionalLight dir = babylon_createDirectionalLight(dir_v, js_some_double(2.2));
    JsTuple_double_double_double dd_in = JsTuple_double_double_double_new(1.0, 0.96, 0.9);
    Babylon_DirectionalLight_set_diffuse(dir, dd_in);
    JsTuple_double_double_double dd = Babylon_DirectionalLight_get_diffuse(dir);
    CHECK(near(Babylon_DirectionalLight_get_intensity(dir), 2.2) && near(JsTuple_double_double_double_get_1(dd), 0.96));
    js_release(dd); js_release(dd_in); js_release(dir); js_release(dir_v);

    JsTuple_double_double_double pt_v = JsTuple_double_double_double_new(2.8, 1.6, 0.0);
    Babylon_PointLight pt = babylon_createPointLight(pt_v, js_some_double(8.0));
    Babylon_PointLight_set_range(pt, 12.0);
    CHECK(near(Babylon_PointLight_get_intensity(pt), 8.0) && near(Babylon_PointLight_get_range(pt), 12.0));
    js_release(pt); js_release(pt_v);

    // Materials.
    Babylon_createPbrMaterial_props props = Babylon_createPbrMaterial_props_create();
    JsTuple_double_double_double_double bcf = JsTuple_double_double_double_double_new(0.18, 0.78, 0.95, 1.0);
    Babylon_createPbrMaterial_props_set_baseColorFactor(props, bcf);
    Babylon_createPbrMaterial_props_set_metallicFactor(props, js_some_double(0.75));
    Babylon_createPbrMaterial_props_set_roughnessFactor(props, js_some_double(0.22));
    Babylon_PbrMaterialProps pbr = babylon_createPbrMaterial(props);
    CHECK(Babylon_PbrMaterialProps_get_metallicFactor(pbr).has && near(Babylon_PbrMaterialProps_get_metallicFactor(pbr).value, 0.75));
    CHECK(Babylon_PbrMaterialProps_get_roughnessFactor(pbr).has && near(Babylon_PbrMaterialProps_get_roughnessFactor(pbr).value, 0.22));
    js_release(pbr); js_release(bcf); js_release(props);

    Babylon_ShaderMaterial grid = babylon_createGridMaterial((Babylon_GridMaterialOptions){ 0 });
    JsString vs = Babylon_ShaderMaterial_get_vertexSource(grid);
    JsString fs = Babylon_ShaderMaterial_get_fragmentSource(grid);
    CHECK(JsString_len(vs) > 0 && JsString_len(fs) > 0);
    js_release(fs); js_release(vs); js_release(grid);

    // Procedural geometry data.
    Babylon_SphereOptions sopts = Babylon_SphereOptions_create();
    Babylon_SphereOptions_set_diameter(sopts, js_some_double(2.0));
    Babylon_SphereOptions_set_segments(sopts, js_some_double(8.0));
    Babylon_SphereMeshData sphere = babylon_createSphereData(sopts);
    Float32Array sp = Babylon_SphereMeshData_get_positions(sphere);
    Uint32Array si = Babylon_SphereMeshData_get_indices(sphere);
    CHECK(Babylon_SphereMeshData_get_vertexCount(sphere) > 0.0 && Babylon_SphereMeshData_get_indexCount(sphere) > 0.0);
    CHECK(Float32Array_length(sp) > 0 && Uint32Array_length(si) > 0);
    float sample[3] = { 0 };
    CHECK(Float32Array_read(sp, 0, sample, 3) == 3);
    js_release(si); js_release(sp); js_release(sphere); js_release(sopts);

    Babylon_TorusKnotOptions topts = Babylon_TorusKnotOptions_create();
    Babylon_TorusKnotOptions_set_radius(topts, js_some_double(1.0));
    Babylon_TorusKnotOptions_set_tube(topts, js_some_double(0.3));
    Babylon_TorusKnotData knot = babylon_createTorusKnotData(topts);
    Float32Array kp = Babylon_TorusKnotData_get_positions(knot);
    Uint32Array ki = Babylon_TorusKnotData_get_indices(knot);
    CHECK(Float32Array_length(kp) > 0 && Uint32Array_length(ki) > 0);
    js_release(ki); js_release(kp); js_release(knot); js_release(topts);

    // Accessibility tree.
    Babylon_AccessibilityTree atree = babylon_createAccessibilityTree();
    Babylon_AccessibilityTag tag = Babylon_AccessibilityTag_create();
    Babylon_AccessibilityTag_set_name(tag, "hero");
    Babylon_AccessibilityTag_set_role(tag, "button");
    Babylon_AccessibilityNodeOptions nopts = Babylon_AccessibilityNodeOptions_create();
    Babylon_AccessibilityNodeOptions_set_tag(nopts, tag);
    Babylon_AccessibilityNode node = babylon_addAccessibilityNode(atree, nopts);
    JsArray_Babylon_AccessibilityNode roots = Babylon_AccessibilityTree_get_roots(atree);
    CHECK(JsArray_Babylon_AccessibilityNode_length(roots) == 1 && !Babylon_AccessibilityTree_get_disposed(atree));
    Babylon_AccessibilityTag got_tag = Babylon_AccessibilityNode_get_tag(node);
    JsString got_name = Babylon_AccessibilityTag_get_name(got_tag);
    JsString got_role = Babylon_AccessibilityTag_get_role(got_tag);
    CHECK(JsString_equals(got_name, "hero") && JsString_equals(got_role, "button"));
    js_release(got_role); js_release(got_name); js_release(got_tag);
    js_release(roots); js_release(node); js_release(nopts); js_release(tag); js_release(atree);

    // Quaternion -> Euler tuple conversion.
    JsTuple_double_double_double euler = babylon_quatToEulerXYZTuple(0.0, 0.0, 0.0, 1.0);
    CHECK(near(JsTuple_double_double_double_get_0(euler), 0.0)
        && near(JsTuple_double_double_double_get_1(euler), 0.0)
        && near(JsTuple_double_double_double_get_2(euler), 0.0));
    js_release(euler);

    CHECK(js_live_handles() == 0);
}
