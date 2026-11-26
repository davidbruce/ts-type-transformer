const std = @import("std");
const dom = @import("dom");
const b = @import("babylonjs_lite");

const App = struct {
    canvas: dom.HTMLCanvasElement = undefined,
    engine: b.EngineContext = undefined,
    knot_rot: b.EulerProxy = undefined,
    orb_pos: b.ObservableVec3 = undefined,
    orb_light_pos: b.ObservableVec3 = undefined,
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
    app.knot_rot.setY(app.elapsed * 0.55);
    app.knot_rot.setX(@sin(app.elapsed * 0.4) * 0.25);
    const x = @cos(app.elapsed * 0.9) * 2.8;
    const y = 2.0 + @sin(app.elapsed * 1.8) * 0.45;
    const z = @sin(app.elapsed * 0.9) * 2.8;
    app.orb_pos.set(x, y, z);
    app.orb_light_pos.set(x, y, z);
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

fn onEngine(app: *App, ok: bool, val: b.JsValue) void {
    const scope = b.JsScope.open();
    defer scope.deinit();
    if (!ok) {
        setStatusError(val);
        return;
    }
    app.engine = val.as(b.EngineContext).retain();

    const scene = b.createSceneContext(app.engine);
    scene.setClearColor(.{ .r = 0.05, .g = 0.07, .b = 0.11, .a = 1.0 });

    const camera = b.createArcRotateCamera(std.math.pi / 4.0, std.math.pi / 3.0, 8.5, b.vec3(0.0, 1.6, 0.0));
    b.addToScene(scene, camera);
    _ = b.attachControlWith(camera, app.canvas, .{ .scene = scene });

    const hemi = b.createHemisphericLight(.{ .direction = .{ 0.0, 1.0, 0.2 }, .intensity = 0.75 });
    hemi.setDiffuseColor(.{ 0.75, 0.85, 1.0 });
    hemi.setGroundColor(.{ 0.12, 0.14, 0.22 });
    b.addToScene(scene, hemi);

    const dir = b.createDirectionalLightWith(.{ -1.0, -2.0, -1.2 }, .{ .intensity = 2.2 });
    dir.setDiffuse(.{ 1.0, 0.96, 0.90 });
    b.addToScene(scene, dir);

    const orb_light = b.createPointLightWith(.{ 2.8, 1.6, 0.0 }, .{ .intensity = 8.0 });
    orb_light.setDiffuse(.{ 1.0, 0.45, 0.35 });
    orb_light.setRange(12.0);
    b.addToScene(scene, orb_light);
    app.orb_light_pos = orb_light.getPosition().retain();

    const knot = b.createTorusKnotWith(app.engine, .{
        .radius = 1.15,
        .tube = 0.34,
        .radialSegments = 48.0,
        .tubularSegments = 160.0,
    });
    knot.getPosition().set(0.0, 2.0, 0.0);
    knot.setMaterial(b.createPbrMaterial(.{
        .baseColorFactor = .{ 0.18, 0.78, 0.95, 1.0 },
        .metallicFactor = 0.75,
        .roughnessFactor = 0.22,
    }));
    b.addToScene(scene, knot);
    app.knot_rot = knot.getRotation().retain();

    const orb = b.createSphereWith(app.engine, .{ .diameter = 0.55, .segments = 32.0 });
    orb.setMaterial(b.createPbrMaterial(.{
        .baseColorFactor = .{ 1.0, 0.42, 0.32, 1.0 },
        .metallicFactor = 0.2,
        .roughnessFactor = 0.35,
    }));
    b.addToScene(scene, orb);
    app.orb_pos = orb.getPosition().retain();

    const ground = b.createGroundWith(app.engine, .{ .width = 16.0, .height = 16.0 });
    ground.setMaterial(b.createGridMaterial(null));
    b.addToScene(scene, ground);

    b.onBeforeRender(scene, .{ onBeforeRender, app });
    b.registerScene(scene).then(onRegistered, app);
}

export fn start() void {
    const scope = dom.JsScope.open();
    defer scope.deinit();
    g_app.canvas = (dom.getDocument().querySelector(.canvas) orelse return).retain();
    b.createEngine(g_app.canvas).then(onEngine, &g_app);
}
