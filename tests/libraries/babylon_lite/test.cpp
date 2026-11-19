// @babylonjs/lite from C++, in Node: vectors, cameras, lights, materials,
// procedural mesh buffers, accessibility tree, and math tuples. Every CHECK must hold.
#include <cmath>
#include <vector>

#include "bindings.hpp"

extern "C" TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

static bool near(double a, double b, double eps = 1e-6) { return std::fabs(a - b) < eps; }

extern "C" __attribute__((export_name("run"))) void run() {
    {
        // Vectors.
        Vec3 target = vec3(1.0, 2.0, 3.0);
        CHECK(target.x() == 1.0 && target.y() == 2.0 && target.z() == 3.0);
        target.y(1.6);
        CHECK(near(target.y(), 1.6));

        // Camera.
        ArcRotateCamera camera = createArcRotateCamera(0.75, 1.0, 8.5, target);
        CHECK(near(camera.alpha(), 0.75) && near(camera.beta(), 1.0));
        CHECK(camera.radius() == 8.5 && near(camera.target().y(), 1.6));

        // Lights.
        HemisphericLight hemi = createHemisphericLight(ts::Tuple<double, double, double>(0.0, 1.0, 0.2), 0.75);
        hemi.diffuseColor(ts::Tuple<double, double, double>(0.75, 0.85, 1.0));
        hemi.groundColor(ts::Tuple<double, double, double>(0.12, 0.14, 0.22));
        auto dc = hemi.diffuseColor();
        auto gc = hemi.groundColor();
        CHECK(near(hemi.intensity(), 0.75));
        CHECK(near(dc.get<0>(), 0.75) && near(dc.get<1>(), 0.85) && near(dc.get<2>(), 1.0));
        CHECK(near(gc.get<0>(), 0.12));

        DirectionalLight dir = createDirectionalLight(ts::Tuple<double, double, double>(-1.0, -2.0, -1.2), 2.2);
        dir.diffuse(ts::Tuple<double, double, double>(1.0, 0.96, 0.9));
        auto dd = dir.diffuse();
        CHECK(near(dir.intensity(), 2.2) && near(dd.get<1>(), 0.96));

        PointLight pt = createPointLight(ts::Tuple<double, double, double>(2.8, 1.6, 0.0), 8.0);
        pt.range(12.0);
        CHECK(near(pt.intensity(), 8.0) && near(pt.range(), 12.0));

        // Materials.
        createPbrMaterial_props props = createPbrMaterial_props::create();
        props.baseColorFactor(ts::Tuple<double, double, double, double>(0.18, 0.78, 0.95, 1.0));
        props.metallicFactor(0.75);
        props.roughnessFactor(0.22);
        PbrMaterialProps pbr = createPbrMaterial(props);
        CHECK(pbr.metallicFactor().has_value() && near(*pbr.metallicFactor(), 0.75));
        CHECK(pbr.roughnessFactor().has_value() && near(*pbr.roughnessFactor(), 0.22));

        ShaderMaterial grid = createGridMaterial(GridMaterialOptions::create());
        CHECK(!grid.vertexSource().empty() && !grid.fragmentSource().empty());

        // Procedural geometry data.
        SphereOptions sopts = SphereOptions::create();
        sopts.diameter(2.0);
        sopts.segments(8.0);
        SphereMeshData sphere = createSphereData(sopts);
        CHECK(sphere.vertexCount() > 0 && sphere.indexCount() > 0);
        std::vector<float> pos = sphere.positions().to_vector();
        CHECK(!pos.empty() && sphere.indices().size() > 0);

        TorusKnotOptions topts = TorusKnotOptions::create();
        topts.radius(1.0);
        topts.tube(0.3);
        TorusKnotData knot = createTorusKnotData(topts);
        CHECK(knot.positions().size() > 0 && knot.indices().size() > 0);

        // Accessibility tree.
        AccessibilityTree tree = createAccessibilityTree();
        AccessibilityTag tag = AccessibilityTag::create();
        tag.name("hero");
        tag.role("button");
        AccessibilityNodeOptions nopts = AccessibilityNodeOptions::create();
        nopts.tag(tag);
        AccessibilityNode node = addAccessibilityNode(tree, nopts);
        CHECK(tree.roots().size() == 1 && !tree.disposed());
        CHECK(node.tag().has_value() && node.tag()->name() == "hero" && node.tag()->role() == "button");

        // Quaternion -> Euler tuple conversion.
        auto euler = quatToEulerXYZTuple(0.0, 0.0, 0.0, 1.0);
        CHECK(near(euler.get<0>(), 0.0) && near(euler.get<1>(), 0.0) && near(euler.get<2>(), 0.0));
    }
    CHECK(ts::live_handles() == 0);
}
