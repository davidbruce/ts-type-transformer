const std = @import("std");
const dom = @import("dom");
const b = @import("babylonjs_lite");

const App = struct {
    canvas: dom.HTMLCanvasElement = undefined,
    engine: b.EngineContext = undefined,
    col_pos: [36]b.ObservableVec3 = undefined,
    col_scale: [36]b.ObservableVec3 = undefined,
    col_x: [36]f64 = undefined,
    col_z: [36]f64 = undefined,
    col_dist: [36]f64 = undefined,
    col_diag: [36]f64 = undefined,
    cap_pos: [4]b.ObservableVec3 = undefined,
    cap_rot: [4]b.EulerProxy = undefined,
    light1_pos: b.ObservableVec3 = undefined,
    light2_pos: b.ObservableVec3 = undefined,
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
    for (0..36) |idx| {
        const h = 0.55 +
            1.15 * (0.5 + 0.5 * @sin(app.elapsed * 2.2 - app.col_dist[idx] * 1.15)) +
            0.30 * @cos(app.elapsed * 1.3 + app.col_diag[idx] * 0.7);
        app.col_scale[idx].set(1.0, h, 1.0);
        app.col_pos[idx].set(app.col_x[idx], h * 0.5, app.col_z[idx]);
    }
    for (0..4) |k| {
        const fk: f64 = @floatFromInt(k);
        const theta = app.elapsed * 0.7 + fk * (std.math.pi * 0.5);
        app.cap_pos[k].set(@cos(theta) * 4.9, 2.6 + @sin(app.elapsed * 1.6 + fk * 1.57) * 0.55, @sin(theta) * 4.9);
        app.cap_rot[k].setZ(@sin(app.elapsed + fk) * 0.35);
        app.cap_rot[k].setY(-theta);
    }
    app.light1_pos.set(@cos(app.elapsed * 0.65) * 5.2, 4.2, @sin(app.elapsed * 0.65) * 5.2);
    app.light2_pos.set(@cos(-app.elapsed * 0.52) * 5.2, 3.4, @sin(-app.elapsed * 0.52) * 5.2);
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
    scene.setClearColor(.{ .r = 0.05, .g = 0.05, .b = 0.05, .a = 1.0 });

    const camera = b.createArcRotateCamera(std.math.pi / 4.0, std.math.pi / 3.1, 12.8, b.vec3(0.0, 1.4, 0.0));
    b.addToScene(scene, camera);
    _ = b.attachControl(camera, app.canvas, .{ .scene = scene });

    const hemi = b.createHemisphericLight(.{ .direction = .{ 0.0, 1.0, 0.15 }, .intensity = 0.55 });
    hemi.setDiffuseColor(.{ 0.76, 0.76, 0.76 });
    hemi.setGroundColor(.{ 0.10, 0.10, 0.10 });
    b.addToScene(scene, hemi);

    const dir = b.createDirectionalLight(.{ -1.0, -2.2, -0.8 }, .{ .intensity = 1.35 });
    dir.setDiffuse(.{ 0.90, 0.90, 0.90 });
    b.addToScene(scene, dir);

    const light1 = b.createPointLight(.{ 5.2, 4.2, 0.0 }, .{ .intensity = 10.0 });
    light1.setDiffuse(.{ 1.0, 1.0, 1.0 });
    light1.setRange(16.0);
    b.addToScene(scene, light1);
    app.light1_pos = light1.getPosition().retain();

    const light2 = b.createPointLight(.{ -5.2, 3.4, 0.0 }, .{ .intensity = 7.5 });
    light2.setDiffuse(.{ 0.78, 0.78, 0.78 });
    light2.setRange(16.0);
    b.addToScene(scene, light2);
    app.light2_pos = light2.getPosition().retain();

    const floor_m = b.createGround(app.engine, .{ .width = 18.0, .height = 18.0 });
    floor_m.setMaterial(mkPbr(0.09, 0.10, 0.88));
    b.addToScene(scene, floor_m);

    const col_mats = [_]b.PbrMaterialProps{
        mkPbr(0.22, 0.35, 0.36),
        mkPbr(0.44, 0.35, 0.36),
        mkPbr(0.68, 0.35, 0.36),
        mkPbr(0.88, 0.35, 0.36),
    };
    const bopts = b.BoxOptions.init(.{ .width = 0.72, .height = 1.0, .depth = 0.72 });

    for (0..6) |row| {
        for (0..6) |col| {
            const idx = row * 6 + col;
            const fx = @as(f64, @floatFromInt(col)) - 2.5;
            const fz = @as(f64, @floatFromInt(row)) - 2.5;
            const x = fx * 1.15;
            const z = fz * 1.15;
            app.col_x[idx] = x;
            app.col_z[idx] = z;
            app.col_dist[idx] = @sqrt(fx * fx + fz * fz);
            app.col_diag[idx] = fx - fz;

            const m = b.createBox(app.engine, bopts);
            app.col_pos[idx] = m.getPosition().retain();
            app.col_scale[idx] = m.getScaling().retain();
            app.col_pos[idx].set(x, 0.5, z);
            m.setMaterial(col_mats[(row + col) % 4]);
            b.addToScene(scene, m);
        }
    }

    const cprops = b.createPbrMaterial_props.init(.{
        .baseColorFactor = .{ 0.92, 0.92, 0.92, 1.0 },
        .metallicFactor = 0.50,
        .roughnessFactor = 0.15,
    });
    b.setPbrClearCoat(cprops, .{ .isEnabled = true, .intensity = 1.0, .roughness = 0.08 });
    b.setPbrEmissive(cprops, .{ 0.16, 0.16, 0.16 });
    const cap_mat = b.createPbrMaterial(cprops);

    const copts = b.CapsuleOptions.init(.{ .height = 1.15, .radius = 0.22, .tessellation = 20.0 });
    for (0..4) |k| {
        const cap = b.createCapsule(app.engine, copts);
        app.cap_pos[k] = cap.getPosition().retain();
        app.cap_rot[k] = cap.getRotation().retain();
        cap.setMaterial(cap_mat);
        b.addToScene(scene, cap);
    }

    b.onBeforeRender(scene, .{ onBeforeRender, app });
    b.registerScene(scene).then(onRegistered, app);
}

export fn start() void {
    const scope = dom.JsScope.open();
    defer scope.deinit();
    g_app.canvas = (dom.getDocument().querySelector(.canvas) orelse return).retain();
    b.createEngine(g_app.canvas, null).then(onEngine, &g_app);
}
