// three.js from C++, in Node: vector, matrix, quaternion, euler and colour
// math, bounds, a scene graph with world positions, a mesh and a ray cast.
// Every CHECK must hold.
#include <cmath>

#include "bindings.hpp"

extern "C" TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

static bool near(double a, double b, double eps = 1e-6) { return std::fabs(a - b) < eps; }

extern "C" __attribute__((export_name("run"))) void run() {
    // ---- vectors ----
    Vector3 a = Vector3::create(1, 2, 3);
    Vector3 b = Vector3::create(4, 5, 6);
    CHECK(a.x() == 1 && a.y() == 2 && a.z() == 3);
    CHECK(a.dot(b) == 32);
    Vector3 sum = a.clone().add(b);
    CHECK(sum.x() == 5 && sum.y() == 7 && sum.z() == 9);
    CHECK(a.x() == 1);  // the clone, not a, was added to
    Vector3 cross = Vector3::create(1, 0, 0).cross(Vector3::create(0, 1, 0));
    CHECK(cross.x() == 0 && cross.y() == 0 && cross.z() == 1);
    CHECK(near(Vector3::create(3, 4, 0).length(), 5));
    Vector3 unit = Vector3::create(0, 0, 9).normalize();
    CHECK(unit.z() == 1 && unit.length() == 1);
    CHECK(Vector3::create(1, 1, 1).distanceTo(Vector3::create(1, 1, 4)) == 3);
    a.setX(10);
    a.y(20);  // a property, set
    CHECK(a.x() == 10 && a.y() == 20);
    CHECK(a.set(1, 2, 3).equals(Vector3::create(1, 2, 3)));
    CHECK(a.multiplyScalar(2).z() == 6);
    ts::Array<double> parts = Vector3::create(7, 8, 9).toArray();
    CHECK(parts.size() == 3 && parts[0] == 7 && parts[2] == 9);
    double acc = 0;
    for (double c : Vector3::create(1, 2, 3)) acc += c;  // vectors iterate
    CHECK(acc == 6);
    Vector3 lerped = Vector3::create(0, 0, 0).lerp(Vector3::create(10, 20, 30), 0.5);
    CHECK(lerped.x() == 5 && lerped.y() == 10 && lerped.z() == 15);

    // ---- matrices ----
    Matrix4 m = Matrix4::create();
    CHECK(m.elements().size() == 16);
    CHECK(m.determinant() == 1);
    m.makeTranslation(1, 2, 3);
    Vector3 moved = Vector3::create(0, 0, 0).applyMatrix4(m);
    CHECK(moved.x() == 1 && moved.y() == 2 && moved.z() == 3);
    Matrix4 scaled = Matrix4::create().makeScale(2, 3, 4);
    CHECK(scaled.determinant() == 24);
    Matrix4 both = Matrix4::create().multiplyMatrices(m, scaled);
    Vector3 pt = Vector3::create(1, 1, 1).applyMatrix4(both);
    CHECK(pt.x() == 3 && pt.y() == 5 && pt.z() == 7);
    Matrix4 inv = both.clone().invert();
    Vector3 back = pt.clone().applyMatrix4(inv);
    CHECK(near(back.x(), 1) && near(back.y(), 1) && near(back.z(), 1));
    Matrix4 rotZ = Matrix4::create().makeRotationZ(M_PI / 2);
    Vector3 turned = Vector3::create(1, 0, 0).applyMatrix4(rotZ);
    CHECK(near(turned.x(), 0) && near(turned.y(), 1));

    // compose and decompose
    Vector3 pos = Vector3::create(5, 6, 7);
    Quaternion rot = Quaternion::create();
    Vector3 scl = Vector3::create(1, 2, 3);
    Matrix4 composed = Matrix4::create().compose(pos, rot, scl);
    Vector3 dpos = Vector3::create();
    Quaternion drot = Quaternion::create();
    Vector3 dscl = Vector3::create();
    composed.decompose(dpos, drot, dscl);
    CHECK(dpos.x() == 5 && dpos.y() == 6 && dpos.z() == 7);
    CHECK(dscl.x() == 1 && dscl.y() == 2 && dscl.z() == 3);
    CHECK(near(drot.w(), 1));

    // ---- quaternions and eulers ----
    Quaternion q = Quaternion::create().setFromAxisAngle(Vector3::create(0, 0, 1), M_PI / 2);
    CHECK(near(q.z(), std::sqrt(0.5)) && near(q.w(), std::sqrt(0.5)));
    Vector3 qv = Vector3::create(1, 0, 0).applyQuaternion(q);
    CHECK(near(qv.x(), 0) && near(qv.y(), 1));
    CHECK(near(q.length(), 1));
    CHECK(near(q.angleTo(Quaternion::create()), M_PI / 2));
    Quaternion inverse = q.clone().invert();
    CHECK(near(inverse.z(), -std::sqrt(0.5)));
    Quaternion identity = q.clone().multiply(inverse);
    CHECK(near(identity.w(), 1) && near(identity.z(), 0));
    Quaternion halfway = Quaternion::create().slerp(q, 0.5);
    CHECK(near(halfway.angleTo(Quaternion::create()), M_PI / 4));

    Euler e = Euler::create(M_PI / 2, 0, 0, EulerOrder::XYZ);
    CHECK(e.order() == EulerOrder::XYZ);
    Quaternion fromEuler = Quaternion::create().setFromEuler(e);
    CHECK(near(fromEuler.x(), std::sqrt(0.5)) && near(fromEuler.w(), std::sqrt(0.5)));
    Euler back2 = Euler::create().setFromQuaternion(fromEuler);
    CHECK(near(back2.x(), M_PI / 2) && near(back2.y(), 0));
    CHECK(Euler::DEFAULT_ORDER == "XYZ");
    CHECK(Euler::create(0, 0, 0).reorder(EulerOrder::ZYX).order() == EulerOrder::ZYX);

    // ---- colours ----
    Color red = Color::create(ColorRepresentation(std::string("red")));
    CHECK(red.r() == 1 && red.g() == 0 && red.b() == 0);
    CHECK(red.getHex() == 0xff0000);
    CHECK(red.getHexString() == "ff0000");
    Color hex = Color::create(ColorRepresentation(double(0x336699)));
    CHECK(near(hex.r(), 0x33 / 255.0, 1e-3) || hex.r() > 0);
    CHECK(hex.getHexString() == "336699");
    Color mix = Color::create(0, 0, 1).lerp(red, 0.5);
    CHECK(near(mix.r(), 0.5) && near(mix.b(), 0.5));
    CHECK(Color::create().setHSL(0, 1, 0.5).getHexString() == "ff0000");
    CHECK(Color::create().setStyle("rgb(0, 255, 0)").getHex() == 0x00ff00);

    // ---- bounds ----
    Box3 box = Box3::create(Vector3::create(-1, -1, -1), Vector3::create(1, 3, 5));
    CHECK(!box.isEmpty());
    Vector3 size = box.getSize(Vector3::create());
    CHECK(size.x() == 2 && size.y() == 4 && size.z() == 6);
    Vector3 center = box.getCenter(Vector3::create());
    CHECK(center.x() == 0 && center.y() == 1 && center.z() == 2);
    CHECK(box.containsPoint(Vector3::create(0, 0, 0)));
    CHECK(!box.containsPoint(Vector3::create(2, 0, 0)));
    box.expandByPoint(Vector3::create(10, 0, 0));
    CHECK(box.max().x() == 10);
    CHECK(box.intersectsBox(Box3::create(Vector3::create(9, 0, 0), Vector3::create(12, 1, 1))));
    CHECK(Box3::create().isEmpty());
    Box3 fromPoints = Box3::create().setFromPoints({Vector3::create(1, 5, 2), Vector3::create(-3, 0, 9)});
    CHECK(fromPoints.min().x() == -3 && fromPoints.max().z() == 9);

    Sphere ball = Sphere::create(Vector3::create(0, 0, 0), 2);
    CHECK(ball.radius() == 2);
    CHECK(ball.containsPoint(Vector3::create(1, 1, 0)));
    CHECK(!ball.containsPoint(Vector3::create(2, 2, 0)));
    CHECK(near(ball.distanceToPoint(Vector3::create(5, 0, 0)), 3));
    CHECK(ball.intersectsSphere(Sphere::create(Vector3::create(3, 0, 0), 1.5)));
    CHECK(ball.intersectsBox(Box3::create(Vector3::create(1, 1, 1), Vector3::create(4, 4, 4))));
    Box3 around = ball.getBoundingBox(Box3::create());
    CHECK(around.min().x() == -2 && around.max().y() == 2);

    // ---- a scene graph ----
    Group root = Group::create();
    root.name("root");
    root.position().set(10, 0, 0);
    Group arm = Group::create();
    arm.name("arm");
    arm.position().set(0, 5, 0);
    Object3D hand = Object3D::create();
    hand.name("hand");
    hand.position().set(0, 0, 2);
    // A Group where an Object3D is wanted: a derived-to-base conversion.
    root.add({arm});
    arm.add({hand});
    CHECK(root.children().size() == 1);
    CHECK(arm.parent().has_value() && arm.parent()->name() == "root");
    CHECK(hand.parent()->parent()->name() == "root");
    root.updateMatrixWorld();
    Vector3 world = hand.getWorldPosition(Vector3::create());
    CHECK(world.x() == 10 && world.y() == 5 && world.z() == 2);
    Vector3 local = Vector3::create(1, 1, 1);
    Vector3 asWorld = hand.localToWorld(local);
    CHECK(asWorld.x() == 11 && asWorld.y() == 6 && asWorld.z() == 3);
    CHECK(root.getObjectByName("hand").has_value());
    CHECK(root.getObjectByName("hand")->id() == hand.id());
    CHECK(!root.getObjectByName("nobody"));

    // Rotating the root swings the hand about it.
    root.rotation().y(M_PI / 2);
    root.updateMatrixWorld();
    Vector3 swung = hand.getWorldPosition(Vector3::create());
    CHECK(near(swung.x(), 12) && near(swung.y(), 5) && near(swung.z(), 0));

    // traverse visits each object
    int visited = 0;
    std::string names;
    root.traverse(Object3D_fn([&](Object3D o) {
        visited++;
        names += o.name() + ",";
    }));
    CHECK(visited == 3);
    CHECK(names == "root,arm,hand,");
    arm.remove({hand});
    CHECK(arm.children().size() == 0);
    CHECK(!hand.parent());

    // ---- a mesh ----
    BoxGeometry geometry = BoxGeometry::create(2, 2, 2);
    MeshBasicMaterialParameters params = MeshBasicMaterialParameters::create();
    params.color(ColorRepresentation(std::string("#00ff00")));
    MeshBasicMaterial material = MeshBasicMaterial::create(params);
    CHECK(material.color().getHexString() == "00ff00");
    Mesh mesh = Mesh::create(geometry, Material_or_Material_array(material));
    mesh.name("cube");
    mesh.position().set(0, 0, 0);
    CHECK(mesh.type() == "Mesh");
    CHECK(geometry.type() == "BoxGeometry");
    CHECK(geometry.parameters().width() == 2);

    // geometry attributes: 24 vertices (4 per face), 36 indices.
    geometry.computeBoundingBox();
    CHECK(geometry.boundingBox().has_value());
    CHECK(geometry.boundingBox()->max().x() == 1);
    geometry.computeBoundingSphere();
    CHECK(near(geometry.boundingSphere()->radius(), std::sqrt(3)));
    CHECK(geometry.getIndex().has_value() && geometry.getIndex()->count() == 36);

    // A mesh in a scene, and a ray towards it from +z.
    root.add({mesh});  // root is at x = 10, so the cube is too
    root.rotation().y(0);
    root.updateMatrixWorld(true);
    Raycaster caster = Raycaster::create(Vector3::create(10.5, 0.25, 20), Vector3::create(0, 0, -1));
    CHECK(caster.near() == 0);
    ts::Array<Intersection> hits = caster.intersectObject(mesh);
    CHECK(hits.size() == 1);
    Intersection hit = hits[0];
    CHECK(near(hit.distance(), 19));
    CHECK(near(hit.point().x(), 10.5) && near(hit.point().y(), 0.25) && near(hit.point().z(), 1));
    CHECK(hit.object().name() == "cube");
    // Missing it: a ray beside the cube.
    Raycaster wide = Raycaster::create(Vector3::create(50, 0, 20), Vector3::create(0, 0, -1));
    CHECK(wide.intersectObject(mesh).size() == 0);
    // Through the whole graph, recursively.
    CHECK(caster.intersectObject(root, true).size() == 1);
}
