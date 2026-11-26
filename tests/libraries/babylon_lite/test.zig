// @babylonjs/lite from Zig, in Node: vectors, cameras, lights, materials,
// procedural mesh buffers, accessibility tree, and math tuples. Every expect must hold.
const b = @import("bindings.zig");
const std = @import("std");

extern "env" fn check(ok: i32, line: u32) void;

fn expect(ok: bool, src: std.builtin.SourceLocation) void {
    check(@intFromBool(ok), src.line);
}

fn near(a: f64, c: f64) bool {
    return @abs(a - c) < 1e-6;
}

export fn run() void {
    {
        // Vectors.
        const target = b.vec3(1.0, 2.0, 3.0);
        defer b.jsRelease(target);
        expect(target.getX() == 1.0 and target.getY() == 2.0 and target.getZ() == 3.0, @src());
        target.setY(1.6);
        expect(near(target.getY(), 1.6), @src());

        // Camera.
        const camera = b.createArcRotateCamera(0.75, 1.0, 8.5, target);
        defer b.jsRelease(camera);
        expect(near(camera.getAlpha(), 0.75) and near(camera.getBeta(), 1.0), @src());
        const cam_target = camera.getTarget();
        defer b.jsRelease(cam_target);
        expect(camera.getRadius() == 8.5 and near(cam_target.getY(), 1.6), @src());

        // Lights.
        const hemi_dir = b.JsTuple(&.{ f64, f64, f64 }).of(.{ 0.0, 1.0, 0.2 });
        defer b.jsRelease(hemi_dir);
        const hemi = b.createHemisphericLight(.{ .direction = hemi_dir, .intensity = 0.75 });
        defer b.jsRelease(hemi);
        const dc_in = b.JsTuple(&.{ f64, f64, f64 }).of(.{ 0.75, 0.85, 1.0 });
        defer b.jsRelease(dc_in);
        const gc_in = b.JsTuple(&.{ f64, f64, f64 }).of(.{ 0.12, 0.14, 0.22 });
        defer b.jsRelease(gc_in);
        hemi.setDiffuseColor(dc_in);
        hemi.setGroundColor(gc_in);
        const dc = hemi.getDiffuseColor();
        defer b.jsRelease(dc);
        const gc = hemi.getGroundColor();
        defer b.jsRelease(gc);
        expect(near(hemi.getIntensity(), 0.75), @src());
        expect(near(dc.get(0), 0.75) and near(dc.get(1), 0.85) and near(dc.get(2), 1.0), @src());
        expect(near(gc.get(0), 0.12), @src());

        const dir_v = b.JsTuple(&.{ f64, f64, f64 }).of(.{ -1.0, -2.0, -1.2 });
        defer b.jsRelease(dir_v);
        const dir = b.createDirectionalLightWith(dir_v, .{ .intensity = 2.2 });
        defer b.jsRelease(dir);
        const dd_in = b.JsTuple(&.{ f64, f64, f64 }).of(.{ 1.0, 0.96, 0.9 });
        defer b.jsRelease(dd_in);
        dir.setDiffuse(dd_in);
        const dd = dir.getDiffuse();
        defer b.jsRelease(dd);
        expect(near(dir.getIntensity(), 2.2) and near(dd.get(1), 0.96), @src());

        const pt_v = b.JsTuple(&.{ f64, f64, f64 }).of(.{ 2.8, 1.6, 0.0 });
        defer b.jsRelease(pt_v);
        const pt = b.createPointLightWith(pt_v, .{ .intensity = 8.0 });
        defer b.jsRelease(pt);
        pt.setRange(12.0);
        expect(near(pt.getIntensity(), 8.0) and near(pt.getRange(), 12.0), @src());

        // Materials.
        const props = b.createPbrMaterial_props.create();
        defer b.jsRelease(props);
        const bcf = b.JsTuple(&.{ f64, f64, f64, f64 }).of(.{ 0.18, 0.78, 0.95, 1.0 });
        defer b.jsRelease(bcf);
        props.setBaseColorFactor(bcf);
        props.setMetallicFactor(0.75);
        props.setRoughnessFactor(0.22);
        const pbr = b.createPbrMaterial(props);
        defer b.jsRelease(pbr);
        expect(near(pbr.getMetallicFactor().?, 0.75) and near(pbr.getRoughnessFactor().?, 0.22), @src());

        const grid = b.createGridMaterial(null);
        defer b.jsRelease(grid);
        const vs = grid.getVertexSource();
        defer b.jsRelease(vs);
        const fs = grid.getFragmentSource();
        defer b.jsRelease(fs);
        expect(vs.len() > 0 and fs.len() > 0, @src());

        // Procedural geometry data.
        const sopts = b.SphereOptions.create();
        defer b.jsRelease(sopts);
        sopts.setDiameter(2.0);
        sopts.setSegments(8.0);
        const sphere = b.createSphereData(sopts);
        defer b.jsRelease(sphere);
        expect(sphere.getVertexCount() > 0 and sphere.getIndexCount() > 0, @src());
        const sp = sphere.getPositions();
        defer b.jsRelease(sp);
        const si = sphere.getIndices();
        defer b.jsRelease(si);
        var sample: [3]f32 = undefined;
        expect(sp.len() > 0 and sp.read(0, &sample) == 3, @src());
        expect(si.len() > 0, @src());

        const topts = b.TorusKnotOptions.create();
        defer b.jsRelease(topts);
        topts.setRadius(1.0);
        topts.setTube(0.3);
        const knot = b.createTorusKnotData(topts);
        defer b.jsRelease(knot);
        const kp = knot.getPositions();
        defer b.jsRelease(kp);
        const ki = knot.getIndices();
        defer b.jsRelease(ki);
        expect(kp.len() > 0 and ki.len() > 0, @src());

        // Accessibility tree.
        const tree = b.createAccessibilityTree();
        defer b.jsRelease(tree);
        const tag = b.AccessibilityTag.create();
        defer b.jsRelease(tag);
        tag.setName("hero");
        tag.setRole("button");
        const nopts = b.AccessibilityNodeOptions.create();
        defer b.jsRelease(nopts);
        nopts.setTag(tag);
        const node = b.addAccessibilityNode(tree, nopts);
        defer b.jsRelease(node);
        const roots = tree.getRoots();
        defer b.jsRelease(roots);
        expect(roots.len() == 1 and !tree.getDisposed(), @src());
        const got_tag = node.getTag().?;
        defer b.jsRelease(got_tag);
        const got_name = got_tag.getName().?;
        defer b.jsRelease(got_name);
        const got_role = got_tag.getRole().?;
        defer b.jsRelease(got_role);
        expect(got_name.eql("hero") and got_role.eql("button"), @src());

        // Quaternion -> Euler tuple conversion.
        const euler = b.quatToEulerXYZTuple(0.0, 0.0, 0.0, 1.0);
        defer b.jsRelease(euler);
        expect(near(euler.get(0), 0.0) and near(euler.get(1), 0.0) and near(euler.get(2), 0.0), @src());
    }
    expect(b.jsLiveHandles() == 0, @src());
}
