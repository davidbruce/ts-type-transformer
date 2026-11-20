const std = @import("std");
const dom = @import("dom");
const b = @import("babylonjs_lite");

const App = struct {
    canvas: dom.HTMLCanvasElement = undefined,
    engine: b.EngineContext = undefined,
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
    const x = @cos(app.elapsed * 0.85) * 4.5;
    const y = 3.0 + @sin(app.elapsed * 1.4) * 0.65;
    const z = @sin(app.elapsed * 0.85) * 4.5;
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

    const scene = b.createSceneContext(app.engine, null);
    scene.setClearColor(.{ .r = 0.06, .g = 0.06, .b = 0.06, .a = 1.0 });

    const camera = b.createArcRotateCamera(std.math.pi / 4.0, std.math.pi / 3.1, 12.0, b.vec3(0.0, 1.1, 0.0));
    b.addToScene(scene, camera);
    _ = b.attachControl(camera, app.canvas, .{ .scene = scene });

    const hemi = b.createHemisphericLight(.{ .direction = .{ 0.0, 1.0, 0.2 }, .intensity = 0.65 });
    hemi.setDiffuseColor(.{ 0.82, 0.82, 0.82 });
    hemi.setGroundColor(.{ 0.12, 0.12, 0.12 });
    b.addToScene(scene, hemi);

    const dir = b.createDirectionalLight(.{ -1.0, -2.0, -1.0 }, .{ .intensity = 1.8 });
    dir.setDiffuse(.{ 0.95, 0.95, 0.95 });
    b.addToScene(scene, dir);

    const orb_light = b.createPointLight(.{ 4.5, 3.0, 0.0 }, .{ .intensity = 12.0 });
    orb_light.setDiffuse(.{ 1.0, 1.0, 1.0 });
    orb_light.setRange(18.0);
    b.addToScene(scene, orb_light);
    app.orb_light_pos = orb_light.getPosition().retain();

    const floor_m = b.createGround(app.engine, .{ .width = 14.0, .height = 14.0 });
    floor_m.setMaterial(mkPbr(0.11, 0.08, 0.88));
    b.addToScene(scene, floor_m);

    const plinth = b.createBox(app.engine, .{ .width = 8.4, .height = 0.08, .depth = 8.4 });
    plinth.getPosition().set(0.0, 0.04, 0.0);
    plinth.setMaterial(mkPbr(0.17, 0.12, 0.78));
    b.addToScene(scene, plinth);

    const sopts = b.SphereOptions.init(.{ .diameter = 1.05, .segments = 24.0 });
    for (0..5) |row| {
        for (0..5) |col| {
            const iter = b.JsScope.open();
            defer iter.deinit();
            const frow: f64 = @floatFromInt(row);
            const fcol: f64 = @floatFromInt(col);
            const sphere = b.createSphere(app.engine, sopts);
            sphere.getPosition().set((fcol - 2.0) * 1.55, 0.75, (frow - 2.0) * 1.55);
            sphere.setMaterial(mkPbr(0.85, 0.25 * fcol, 0.08 + 0.22 * frow));
            b.addToScene(scene, sphere);
        }
    }

    const oprops = b.createPbrMaterial_props.init(.{
        .baseColorFactor = .{ 0.95, 0.95, 0.95, 1.0 },
        .metallicFactor = 0.0,
        .roughnessFactor = 0.2,
    });
    b.setPbrEmissive(oprops, .{ 0.95, 0.95, 0.95 });

    const orb = b.createSphere(app.engine, .{ .diameter = 0.36, .segments = 16.0 });
    app.orb_pos = orb.getPosition().retain();
    app.orb_pos.set(4.5, 3.0, 0.0);
    orb.setMaterial(b.createPbrMaterial(oprops));
    b.addToScene(scene, orb);

    b.onBeforeRender(scene, .{ onBeforeRender, app });
    b.registerScene(scene).then(onRegistered, app);
}

export fn start() void {
    const scope = dom.JsScope.open();
    defer scope.deinit();
    g_app.canvas = (dom.getDocument().querySelector(.canvas) orelse return).retain();
    b.createEngineWithEngineOptions(g_app.canvas, null).then(onEngine, &g_app);
}
