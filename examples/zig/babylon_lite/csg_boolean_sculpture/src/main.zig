const std = @import("std");
const dom = @import("dom");
const b = @import("babylonjs_lite");

const App = struct {
    canvas: dom.HTMLCanvasElement = undefined,
    engine: b.EngineContext = undefined,
    mono_rot: b.EulerProxy = undefined,
    core_rot: b.EulerProxy = undefined,
    outer_rot: b.EulerProxy = undefined,
    inner_rot: b.EulerProxy = undefined,
    elapsed: f64 = 0.0,
};

var g_app: App = .{};

fn setStatusError(err: b.JsValue) void {
    const scope = b.JsScope.open();
    defer scope.deinit();
    if (dom.getDocument().getElementById("status")) |status| {
        const msg = b.jsErrorMessage(err);
        var buf: [320]u8 = undefined;
        const text = std.fmt.bufPrint(&buf, "WebGPU / Babylon Lite error: {f}", .{msg}) catch return;
        status.setTextContent(text);
    }
}

fn onBeforeRender(app: *App, delta_ms: f64) void {
    app.elapsed += delta_ms * 0.001;
    app.mono_rot.setY(app.elapsed * 0.35);
    app.mono_rot.setX(@sin(app.elapsed * 0.3) * 0.20);
    app.core_rot.setY(-app.elapsed * 0.90);
    app.core_rot.setZ(app.elapsed * 0.50);
    app.outer_rot.setX(app.elapsed * 0.42);
    app.outer_rot.setY(app.elapsed * 0.25);
    app.inner_rot.setZ(-app.elapsed * 0.58);
    app.inner_rot.setX(app.elapsed * 0.31);
}

fn onStarted(ok: bool, val: b.JsValue) void {
    if (!ok) setStatusError(val);
}

fn onRegistered(app: *App, ok: bool, val: b.JsValue) void {
    const scope = b.JsScope.open();
    defer scope.deinit();
    if (!ok) {
        setStatusError(val);
        return;
    }
    b.startEngine(app.engine).thenFn(onStarted);
}

fn mkPbr(v: f64, metallic: f64, roughness: f64) b.PbrMaterialProps {
    return b.createPbrMaterial(.{
        .baseColorFactor = .{ v, v, v, 1.0 },
        .metallicFactor = metallic,
        .roughnessFactor = roughness,
    });
}

fn onEngine(app: *App, ok: bool, val: b.JsValue) void {
    const scope = b.JsScope.open();
    defer scope.deinit();
    if (!ok) {
        setStatusError(val);
        return;
    }
    app.engine = val.as(b.EngineContext).retain();

    const scene = b.createSceneContext(app.engine);
    scene.setClearColor(.{ .r = 0.06, .g = 0.06, .b = 0.06, .a = 1.0 });

    const camera = b.createArcRotateCamera(std.math.pi / 4.2, std.math.pi / 2.9, 8.8, b.vec3(0.0, 2.1, 0.0));
    b.addToScene(scene, camera);
    _ = b.attachControlWith(camera, app.canvas, .{ .scene = scene });

    const hemi = b.createHemisphericLight(.{ .direction = .{ 0.0, 1.0, 0.2 }, .intensity = 0.72 });
    hemi.setDiffuseColor(.{ 0.85, 0.85, 0.85 });
    hemi.setGroundColor(.{ 0.14, 0.14, 0.14 });
    b.addToScene(scene, hemi);

    const dir = b.createDirectionalLightWith(.{ -1.2, -2.0, -1.1 }, .{ .intensity = 2.1 });
    dir.setDiffuse(.{ 1.0, 1.0, 1.0 });
    b.addToScene(scene, dir);

    const key_light = b.createPointLightWith(.{ 3.2, 3.6, 2.4 }, .{ .intensity = 8.0 });
    key_light.setDiffuse(.{ 0.92, 0.92, 0.92 });
    key_light.setRange(14.0);
    b.addToScene(scene, key_light);

    const floor_m = b.createGroundWith(app.engine, .{ .width = 16.0, .height = 16.0 });
    floor_m.setMaterial(mkPbr(0.09, 0.10, 0.88));
    b.addToScene(scene, floor_m);

    const disc = b.createDiscWith(app.engine, .{ .radius = 2.6, .tessellation = 48.0 });
    disc.getRotation().setX(std.math.pi / 2.0);
    disc.getPosition().set(0.0, 0.03, 0.0);
    disc.setMaterial(mkPbr(0.22, 0.25, 0.65));
    b.addToScene(scene, disc);

    const src_box = b.createBoxWith(app.engine, .{ .size = 2.1 });
    const copts = b.CylinderOptions.init(.{ .height = 3.0, .diameter = 1.18, .tessellation = 16.0 });
    const cyl_y = b.createCylinderWith(app.engine, copts);
    const cyl_x = b.createCylinderWith(app.engine, copts);
    cyl_x.getRotation().setZ(std.math.pi / 2.0);
    const cyl_z = b.createCylinderWith(app.engine, copts);
    cyl_z.getRotation().setX(std.math.pi / 2.0);

    const s1 = b.csgSubtract(b.createCsgFromMesh(src_box), b.createCsgFromMesh(cyl_y));
    const s2 = b.csgSubtract(s1, b.createCsgFromMesh(cyl_x));
    const s3 = b.csgSubtract(s2, b.createCsgFromMesh(cyl_z));
    const monolith = b.createMeshFromCsgWith(app.engine, s3, .{ .name = "monolith" });
    monolith.getPosition().set(0.0, 2.15, 0.0);
    app.mono_rot = monolith.getRotation().retain();

    const mprops = b.createPbrMaterial_props.init(.{
        .baseColorFactor = .{ 0.86, 0.86, 0.86, 1.0 },
        .metallicFactor = 0.65,
        .roughnessFactor = 0.24,
    });
    b.setPbrClearCoat(mprops, .{ .isEnabled = true, .intensity = 0.90, .roughness = 0.10 });
    monolith.setMaterial(b.createPbrMaterial(mprops));
    b.addToScene(scene, monolith);

    const core_box = b.createBoxWith(app.engine, .{ .size = 0.82 });
    const core_sphere = b.createSphereWith(app.engine, .{ .diameter = 1.02, .segments = 14.0 });
    const core_s = b.csgIntersect(b.createCsgFromMesh(core_box), b.createCsgFromMesh(core_sphere));
    const core = b.createMeshFromCsgWith(app.engine, core_s, .{ .name = "core" });
    core.getPosition().set(0.0, 2.15, 0.0);
    app.core_rot = core.getRotation().retain();
    core.setMaterial(mkPbr(0.18, 0.85, 0.16));
    b.addToScene(scene, core);

    const outer_ring = b.createTorusWith(app.engine, .{ .diameter = 4.2, .thickness = 0.09, .tessellation = 48.0 });
    outer_ring.getPosition().set(0.0, 2.15, 0.0);
    app.outer_rot = outer_ring.getRotation().retain();
    outer_ring.setMaterial(mkPbr(0.55, 0.80, 0.25));
    b.addToScene(scene, outer_ring);

    const inner_ring = b.createTorusWith(app.engine, .{ .diameter = 3.55, .thickness = 0.07, .tessellation = 48.0 });
    inner_ring.getPosition().set(0.0, 2.15, 0.0);
    app.inner_rot = inner_ring.getRotation().retain();
    inner_ring.setMaterial(mkPbr(0.88, 0.40, 0.30));
    b.addToScene(scene, inner_ring);

    b.onBeforeRender(scene, .{ onBeforeRender, app });
    b.registerScene(scene).then(onRegistered, app);
}

export fn start() void {
    const scope = dom.JsScope.open();
    defer scope.deinit();
    g_app.canvas = (dom.getDocument().querySelector(.canvas) orelse return).retain();
    b.createEngine(g_app.canvas).then(onEngine, &g_app);
}
