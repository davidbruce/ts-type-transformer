// three.js from Zig, in Node: vectors, matrices, rotations, bounds, a scene graph, a mesh and a ray. Every expect must hold.
const std = @import("std");
const b = @import("bindings.zig");

extern "env" fn check(ok: i32, line: u32) void;

fn expect(ok: bool, src: std.builtin.SourceLocation) void {
    check(@intFromBool(ok), src.line);
}

fn near(a: f64, c: f64) bool {
    return @abs(a - c) < 1e-9;
}

fn like(v: b.Vector3) b.Vector3Like {
    return v.asVector3Like();
}
fn qlike(q: b.Quaternion) b.QuaternionLike {
    return q.asQuaternionLike();
}

fn vec(x: f64, y: f64, z: f64) b.Vector3 {
    return b.Vector3.new(.{ .x = x, .y = y, .z = z });
}

fn place(v: b.Vector3, x: f64, y: f64, z: f64) void {
    _ = v.set(x, y, .{ .z = z });
}

fn at(v: b.Vector3, x: f64, y: f64, z: f64) bool {
    return near(v.getX(), x) and near(v.getY(), y) and near(v.getZ(), z);
}

export fn run() void {
    // Vector3: constructors, arithmetic, products.
    const v = vec(1, 2, 3);
    expect(at(v, 1, 2, 3), @src());
    expect(at(b.Vector3.new(.{}), 0, 0, 0), @src());
    const sum = v.clone().add(like(vec(4, 5, 6)));
    expect(at(sum, 5, 7, 9), @src());
    expect(at(v, 1, 2, 3), @src());
    expect(v.dot(like(vec(4, 5, 6))) == 32, @src());
    expect(at(vec(1, 0, 0).cross(like(vec(0, 1, 0))), 0, 0, 1), @src());
    expect(vec(3, 4, 0).length() == 5 and vec(3, 4, 0).lengthSq() == 25, @src());
    expect(near(vec(0, 0, 9).normalize().getZ(), 1), @src());
    expect(at(vec(1, 2, 3).multiplyScalar(2), 2, 4, 6), @src());
    expect(at(vec(0, 0, 0).lerp(like(vec(10, 20, 30)), 0.5), 5, 10, 15), @src());
    expect(vec(1, 1, 1).distanceTo(like(vec(1, 1, 4))) == 3, @src());
    expect(vec(1, 2, 3).equals(like(vec(1, 2, 3))) and !vec(1, 2, 3).equals(like(vec(1, 2, 4))), @src());
    expect(at(vec(1, 2, 3).negate(), -1, -2, -3), @src());
    expect(at(vec(1, 2, 3).set(7, 8, .{ .z = 9 }), 7, 8, 9), @src());
    expect(near(vec(1, 0, 0).angleTo(vec(0, 1, 0)), std.math.pi / 2.0), @src());
    const arr = vec(7, 8, 9).toArray(.{});
    expect(arr.len() == 3 and arr.get(0) == 7 and arr.get(2) == 9, @src());
    // A Vector3 iterates as its components.
    var it = vec(4, 5, 6).iterate();
    var total: f64 = 0;
    while (it.next()) |c| total += c;
    expect(total == 15, @src());

    // Matrix4: translation, scale, products, inverse.
    const t = b.Matrix4.new().makeTranslation(vec(10, 0, 0));
    expect(at(vec(1, 2, 3).applyMatrix4(t), 11, 2, 3), @src());
    expect(b.Matrix4.new().determinant() == 1, @src());
    const s = b.Matrix4.new().makeScale(2, 3, 4);
    expect(s.determinant() == 24, @src());
    const ts = b.Matrix4.new().multiplyMatrices(t, s);
    expect(at(vec(1, 1, 1).applyMatrix4(ts), 12, 3, 4), @src());
    const inv = ts.clone().invert();
    expect(at(vec(12, 3, 4).applyMatrix4(inv), 1, 1, 1), @src());
    expect(ts.clone().multiply(inv).equals(b.Matrix4.new()), @src());
    const e = ts.getElements();
    expect(e.len() == 16 and e.get(0).toNumber() == 2 and e.get(12).toNumber() == 10, @src());
    expect(b.Matrix4.new().makeRotationZ(std.math.pi).determinant() > 0.999, @src());

    // Quaternion and Euler.
    const quarter = b.Quaternion.new(.{}).setFromAxisAngle(like(vec(0, 0, 1)), std.math.pi / 2.0);
    expect(near(quarter.getW(), @sqrt(0.5)) and near(quarter.getZ(), @sqrt(0.5)), @src());
    expect(at(vec(1, 0, 0).applyQuaternion(qlike(quarter)), 0, 1, 0), @src());
    expect(near(b.Quaternion.new(.{}).angleTo(quarter), std.math.pi / 2.0), @src());
    const halfway = b.Quaternion.new(.{}).slerp(quarter, 0.5);
    expect(near(halfway.angleTo(b.Quaternion.new(.{})), std.math.pi / 4.0), @src());
    expect(b.Quaternion.new(.{}).getW() == 1, @src());
    const euler = b.Euler.new(.{ .x = 0, .y = 0, .z = std.math.pi / 2.0, .order = .XYZ });
    expect(euler.getOrder() == .XYZ and near(euler.getZ(), std.math.pi / 2.0), @src());
    const from_euler = b.Quaternion.new(.{}).setFromEuler(euler, .{});
    expect(near(from_euler.getZ(), quarter.getZ()), @src());
    const back = b.Euler.new(.{}).setFromQuaternion(from_euler, .{});
    expect(near(back.getZ(), std.math.pi / 2.0) and near(back.getX(), 0), @src());
    euler.setOrder(.ZYX);
    expect(euler.getOrder() == .ZYX, @src());

    // compose and decompose are inverses.
    const composed = b.Matrix4.new().compose(vec(1, 2, 3), quarter, vec(2, 2, 2));
    const pos = vec(0, 0, 0);
    const rot = b.Quaternion.new(.{});
    const scl = vec(0, 0, 0);
    _ = composed.decompose(pos, rot, scl);
    expect(at(pos, 1, 2, 3) and at(scl, 2, 2, 2) and near(rot.getZ(), quarter.getZ()), @src());

    // Color: by name, number, string and channel.
    const red = b.Color.new(.{ .string = b.JsString.from("red") });
    expect(red.getR() == 1 and red.getG() == 0 and red.getB() == 0, @src());
    expect(red.getHex(null) == 0xff0000, @src());
    expect(red.getHexString(null).eql("ff0000"), @src());
    expect(b.Color.new(.{ .number = 0x00ff00 }).getHexString(null).eql("00ff00"), @src());
    const mix = b.Color.newWithRAndGAndB(0.5, 0.25, 1);
    expect(mix.getR() == 0.5 and mix.getG() == 0.25, @src());
    _ = mix.setHex(0x0000ff, .{});
    expect(mix.getB() == 1 and mix.getR() == 0, @src());
    _ = mix.setStyle("rgb(255, 0, 0)", .{});
    expect(mix.getHex(null) == 0xff0000, @src());
    expect(red.equals(mix), @src());
    expect(near(b.Color.new(null).multiplyScalar(0.5).getR(), 0.5), @src());

    // Box3 and Sphere.
    const points = b.JsArrayOf(b.Vector3Like).new();
    points.push(like(vec(-1, -2, -3)));
    points.push(like(vec(4, 5, 6)));
    const box = b.Box3.new(.{}).setFromPoints(points);
    expect(at(box.getMin(), -1, -2, -3) and at(box.getMax(), 4, 5, 6), @src());
    expect(at(box.getCenter(vec(0, 0, 0)), 1.5, 1.5, 1.5), @src());
    expect(at(box.getSize(vec(0, 0, 0)), 5, 7, 9), @src());
    expect(box.containsPoint(vec(0, 0, 0)) and !box.containsPoint(vec(10, 0, 0)), @src());
    expect(!box.isEmpty() and b.Box3.new(.{}).isEmpty(), @src());
    const other = b.Box3.new(.{ .min = vec(3, 3, 3), .max = vec(9, 9, 9) });
    expect(box.intersectsBox(other), @src());
    expect(at(box.clone().@"union"(other).getMax(), 9, 9, 9), @src());
    expect(at(box.clone().expandByScalar(1).getMin(), -2, -3, -4), @src());
    const sphere = b.Sphere.new(.{ .center = vec(0, 0, 0), .radius = 2 });
    expect(sphere.containsPoint(vec(1, 1, 0)) and !sphere.containsPoint(vec(2, 2, 0)), @src());
    expect(sphere.distanceToPoint(vec(5, 0, 0)) == 3, @src());
    expect(sphere.intersectsSphere(b.Sphere.new(.{ .center = vec(3, 0, 0), .radius = 1 })), @src());
    expect(at(sphere.getBoundingBox(b.Box3.new(.{})).getMax(), 2, 2, 2), @src());
    expect(sphere.intersectsBox(b.Box3.new(.{ .min = vec(1, 1, 1), .max = vec(5, 5, 5) })), @src());

    // A scene graph: a group at (10, 0, 0) holding a group at (0, 5, 0).
    const outer = b.Group.new();
    const inner = b.Group.new();
    place(outer.getPosition(), 10, 0, 0);
    place(inner.getPosition(), 0, 5, 0);
    inner.setName("inner");
    // A Group where an Object3D is wanted is its asObject3D.
    _ = outer.add(b.JsArrayOf(b.Object3D).of(&.{inner.asObject3D()}));
    expect(outer.getChildren().len() == 1, @src());
    expect(inner.getParent() != null and b.jsSame(inner.getParent().?, outer.asObject3D()), @src());
    outer.updateMatrixWorld(true);
    const world = inner.getWorldPosition(vec(0, 0, 0));
    expect(at(world, 10, 5, 0), @src());
    expect(at(inner.getPosition(), 0, 5, 0), @src());
    expect(at(inner.localToWorld(vec(1, 0, 0)), 11, 5, 0), @src());
    expect(at(inner.worldToLocal(vec(10, 5, 0)), 0, 0, 0), @src());
    expect(outer.getObjectByName("inner") != null, @src());
    expect(outer.getObjectByName("nobody") == null, @src());
    expect(inner.getType().eql("Group"), @src());
    expect(outer.getVisible() and outer.getId() != inner.getId(), @src());
    // Rotating the outer group carries the inner one around it.
    outer.getRotation().setZ(std.math.pi / 2.0);
    outer.updateMatrixWorld(true);
    expect(at(inner.getWorldPosition(vec(0, 0, 0)), 5, 0, 0), @src());
    // Removing it detaches.
    _ = outer.remove(b.JsArrayOf(b.Object3D).of(&.{inner.asObject3D()}));
    expect(outer.getChildren().len() == 0 and inner.getParent() == null, @src());

    // A mesh: a 2x2x2 box with a red basic material.
    const geometry = b.BoxGeometry.new(.{ .width = 2, .height = 2, .depth = 2 });
    const params = b.MeshBasicMaterialParameters.create();
    params.setColor(.{ .number = 0xff0000 });
    params.setTransparent(true);
    params.setOpacity(0.5);
    const material = b.MeshBasicMaterial.new(params);
    expect(material.getOpacity() == 0.5 and material.getTransparent(), @src());
    expect(material.asMaterial().getType().eql("MeshBasicMaterial"), @src());
    const mesh = b.Mesh.new(.{
        .geometry = geometry.asBufferGeometry(),
        .material = .{ .Material = material.asMaterial() },
    });
    expect(mesh.getType().eql("Mesh"), @src());
    expect(mesh.getMaterial() == .Material, @src());
    expect(geometry.getParameters().getWidth() == 2, @src());

    // The geometry's attributes: 24 vertices in 6 faces, and an index of 36.
    switch (geometry.getAttribute("position")) {
        .BufferAttribute => |p| {
            expect(p.getCount() == 24 and p.getItemSize() == 3, @src());
            expect(@abs(p.getX(0)) == 1, @src());
        },
        .InterleavedBufferAttribute => expect(false, @src()),
    }
    expect(geometry.asBufferGeometry().getIndex().?.getCount() == 36, @src());
    geometry.asBufferGeometry().computeBoundingBox();
    const bounds = geometry.asBufferGeometry().getBoundingBox().?;
    expect(at(bounds.getMin(), -1, -1, -1) and at(bounds.getMax(), 1, 1, 1), @src());

    // A ray down -z from (0, 0, 5) hits the front face at z = 1; one off to the side misses.
    mesh.updateMatrixWorld(true);
    const caster = b.Raycaster.new(.{ .origin = vec(0, 0, 5), .direction = vec(0, 0, -1) });
    const hits = caster.intersectObject(mesh.asObject3D(), .{});
    expect(hits.len() >= 1, @src());
    const first = hits.get(0);
    expect(near(first.getDistance(), 4), @src());
    expect(at(first.getPoint(), 0, 0, 1), @src());
    expect(b.jsSame(first.getObject(), mesh.asObject3D()), @src());
    expect(first.getFaceIndex() != null, @src());
    const miss = b.Raycaster.new(.{ .origin = vec(5, 5, 5), .direction = vec(0, 0, -1) });
    expect(miss.intersectObject(mesh.asObject3D(), .{}).len() == 0, @src());
    // Move the mesh and the same ray misses; move it in front and the distance shrinks.
    place(mesh.getPosition(), 0, 0, 3);
    mesh.updateMatrixWorld(true);
    expect(near(caster.intersectObject(mesh.asObject3D(), .{}).get(0).getDistance(), 1), @src());
    place(mesh.getPosition(), 20, 0, 0);
    mesh.updateMatrixWorld(true);
    expect(caster.intersectObject(mesh.asObject3D(), .{}).len() == 0, @src());
}
