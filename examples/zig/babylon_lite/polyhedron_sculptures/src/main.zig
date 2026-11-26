const std = @import("std");
const dom = @import("dom");
const b = @import("babylonjs_lite");

const App = struct {
    canvas: dom.HTMLCanvasElement = undefined,
    engine: b.EngineContext = undefined,
    spot_pos: b.ObservableVec3 = undefined,
    spot_dir: b.ObservableVec3 = undefined,
    poly_pos: [5]b.ObservableVec3 = undefined,
    poly_rot: [5]b.EulerProxy = undefined,
    poly_x: [5]f64 = undefined,
    poly_z: [5]f64 = undefined,
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
    const sx = @sin(app.elapsed * 0.65) * 5.2;
    app.spot_pos.set(sx, 6.2, 3.0);
    app.spot_dir.set(-sx * 0.15, -1.0, -0.55);
    for (0..5) |i| {
        const fi: f64 = @floatFromInt(i);
        app.poly_pos[i].set(app.poly_x[i], 2.05 + @sin(app.elapsed * 1.3 + fi * 1.1) * 0.18, app.poly_z[i]);
        app.poly_rot[i].setY(app.elapsed * (0.45 + 0.12 * fi));
        app.poly_rot[i].setX(@sin(app.elapsed * 0.7 + fi) * 0.35);
    }
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

fn mkStd(diff: f64, spec: f64, power: f64) b.StandardMaterialProps {
    const mat = b.createStandardMaterial();
    mat.setDiffuseColor(.{ diff, diff, diff });
    mat.setSpecularColor(.{ spec, spec, spec });
    mat.setSpecularPower(power);
    return mat;
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
    scene.setClearColor(.{ .r = 0.05, .g = 0.05, .b = 0.05, .a = 1.0 });

    const camera = b.createArcRotateCamera(std.math.pi / 2.0, std.math.pi / 2.8, 11.5, b.vec3(0.0, 1.6, 0.0));
    b.addToScene(scene, camera);
    _ = b.attachControlWith(camera, app.canvas, .{ .scene = scene });

    const hemi = b.createHemisphericLight(.{ .direction = .{ 0.0, 1.0, 0.0 }, .intensity = 0.42 });
    hemi.setDiffuseColor(.{ 0.70, 0.70, 0.70 });
    hemi.setGroundColor(.{ 0.08, 0.08, 0.08 });
    b.addToScene(scene, hemi);

    const spot = b.createSpotLightWith(.{ 0.0, 6.2, 3.0 }, .{ 0.0, -1.0, -0.55 }, std.math.pi / 3.2, 3.0, .{ .intensity = 26.0 });
    spot.setDiffuse(.{ 1.0, 1.0, 1.0 });
    spot.setRange(22.0);
    b.addToScene(scene, spot);
    app.spot_pos = spot.getPosition().retain();
    app.spot_dir = spot.getDirection().retain();

    const fill = b.createDirectionalLightWith(.{ 1.2, -1.5, -1.0 }, .{ .intensity = 0.65 });
    fill.setDiffuse(.{ 0.80, 0.80, 0.80 });
    b.addToScene(scene, fill);

    const floor_m = b.createGroundWith(app.engine, .{ .width = 18.0, .height = 18.0 });
    floor_m.setMaterial(mkStd(0.10, 0.25, 24.0));
    b.addToScene(scene, floor_m);

    const plinth_mat = mkStd(0.18, 0.30, 32.0);
    const copts = b.CylinderOptions.init(.{ .height = 1.1, .diameter = 1.15, .tessellation = 28.0 });

    for (0..5) |i| {
        const fi: f64 = @floatFromInt(i);
        const x = (fi - 2.0) * 2.25;
        const z = -@abs(fi - 2.0) * 0.55;
        app.poly_x[i] = x;
        app.poly_z[i] = z;

        const plinth = b.createCylinderWith(app.engine, copts);
        plinth.getPosition().set(x, 0.55, z);
        plinth.setMaterial(plinth_mat);
        b.addToScene(scene, plinth);

        const v = 0.90 - fi * 0.13;
        const mesh = b.createPolyhedronWith(app.engine, .{ .type = fi, .size = 0.62, .flat = true });
        app.poly_pos[i] = mesh.getPosition().retain();
        app.poly_pos[i].set(x, 2.05, z);
        app.poly_rot[i] = mesh.getRotation().retain();
        mesh.setMaterial(mkStd(v, 0.95, 18.0 + fi * 24.0));
        b.addToScene(scene, mesh);
    }

    b.onBeforeRender(scene, .{ onBeforeRender, app });
    b.registerScene(scene).then(onRegistered, app);
}

export fn start() void {
    const scope = dom.JsScope.open();
    defer scope.deinit();
    g_app.canvas = (dom.getDocument().querySelector(.canvas) orelse return).retain();
    b.createEngine(g_app.canvas).then(onEngine, &g_app);
}
