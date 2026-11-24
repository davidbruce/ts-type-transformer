const std = @import("std");
const dom = @import("dom");
const b = @import("babylonjs_lite");
const rp = @import("rapier");

const LAYERS = 6;
const PER_LAYER = 3;
const PLANK_COUNT = LAYERS * PER_LAYER;
const HALF_LONG: f64 = 0.85;
const HALF_HEIGHT: f64 = 0.22;
const HALF_THIN: f64 = 0.26;
const SPHERE_RADIUS: f64 = 0.65;

const BtnCtx = struct { app: *App, id: usize };

const App = struct {
    canvas: dom.HTMLCanvasElement = undefined,
    meta: dom.HTMLElement = undefined,
    pause_btn: dom.HTMLElement = undefined,
    engine: b.EngineContext = undefined,
    world: ?rp.World = null,
    planks: [PLANK_COUNT]rp.RigidBody = undefined,
    striker: rp.RigidBody = undefined,
    has_bodies: bool = false,
    plank_pos: [PLANK_COUNT]b.ObservableVec3 = undefined,
    plank_rot: [PLANK_COUNT]b.ObservableQuat = undefined,
    striker_pos: b.ObservableVec3 = undefined,
    striker_rot: b.ObservableQuat = undefined,
    has_meshes: bool = false,
    btns: [3]BtnCtx = undefined,
    paused: bool = false,
    step_count: usize = 0,
    shot_idx: usize = 0,
};

var g_app: App = .{};

fn setStatusText(text: []const u8) void {
    const scope = dom.JsScope.open();
    defer scope.deinit();
    if (dom.getDocument().getElementById("status")) |el| {
        el.setTextContent(text);
    }
}

fn setStatusError(err: b.JsValue) void {
    const scope = dom.JsScope.open();
    defer scope.deinit();
    if (dom.getDocument().getElementById("status")) |el| {
        const msg = b.jsErrorMessage(err);
        var buf: [320]u8 = undefined;
        const text = std.fmt.bufPrint(&buf, "WebGPU / Babylon Lite error: {f}", .{msg}) catch return;
        el.setTextContent(text);
    }
}

fn buildWorld(app: *App) void {
    const scope = dom.JsScope.open();
    defer scope.deinit();

    if (app.world) |w| {
        if (app.has_bodies) {
            for (&app.planks) |*rb| rb.deinit();
            app.striker.deinit();
        }
        w.free();
        w.deinit();
        app.world = null;
        app.has_bodies = false;
    }

    const world = rp.World.new(rp.Vector3.new(0.0, -9.81, 0.0), null).retain();

    const floor_body = world.createRigidBody(rp.RigidBodyDesc.fixed().setTranslation(0.0, -0.3, 0.0));
    _ = world.createCollider(
        rp.ColliderDesc.cuboid(10.0, 0.3, 10.0).setFriction(0.75).setRestitution(0.1),
        floor_body,
    );

    for (0..PLANK_COUNT) |idx| {
        const layer = idx / PER_LAYER;
        const slot = idx % PER_LAYER;
        const offset = (@as(f64, @floatFromInt(slot)) - 1.0) * 0.57;
        const y = HALF_HEIGHT + @as(f64, @floatFromInt(layer)) * 0.445;
        const x: f64 = if (layer % 2 == 0) 0.0 else offset;
        const z: f64 = if (layer % 2 == 0) offset else 0.0;
        const hx: f64 = if (layer % 2 == 0) HALF_LONG else HALF_THIN;
        const hz: f64 = if (layer % 2 == 0) HALF_THIN else HALF_LONG;

        const rb = world.createRigidBody(rp.RigidBodyDesc.dynamic().setTranslation(x, y, z));
        _ = world.createCollider(
            rp.ColliderDesc.cuboid(hx, HALF_HEIGHT, hz)
                .setFriction(0.65)
                .setRestitution(0.12)
                .setDensity(1.0),
            rb,
        );
        app.planks[idx] = rb.retain();
    }

    const striker = world.createRigidBody(
        rp.RigidBodyDesc.dynamic()
            .setTranslation(-6.2, 1.55, 0.12)
            .setLinvel(13.5, 1.1, -0.25),
    );
    _ = world.createCollider(
        rp.ColliderDesc.ball(SPHERE_RADIUS)
            .setDensity(4.5)
            .setRestitution(0.35)
            .setFriction(0.3),
        striker,
    );
    app.striker = striker.retain();
    app.has_bodies = true;
    app.world = world;
    app.step_count = 0;
}

fn syncMeshes(app: *App) void {
    if (!app.has_meshes or !app.has_bodies) return;
    const scope = dom.JsScope.open();
    defer scope.deinit();

    for (0..PLANK_COUNT) |i| {
        const p = app.planks[i].translation(null);
        const q = app.planks[i].rotation(null);
        app.plank_pos[i].set(p.getX(), p.getY(), p.getZ());
        app.plank_rot[i].set(q.getX(), q.getY(), q.getZ(), q.getW());
    }
    const sp = app.striker.translation(null);
    const sq = app.striker.rotation(null);
    app.striker_pos.set(sp.getX(), sp.getY(), sp.getZ());
    app.striker_rot.set(sq.getX(), sq.getY(), sq.getZ(), sq.getW());
}

fn updateMeta(app: *App) void {
    if (!app.has_bodies) return;
    const scope = dom.JsScope.open();
    defer scope.deinit();

    var max_y: f64 = 0.0;
    var ke: f64 = 0.0;
    for (0..PLANK_COUNT) |i| {
        const p = app.planks[i].translation(null);
        const v = app.planks[i].linvel(null);
        const py = p.getY();
        if (py > max_y) max_y = py;
        const vx = v.getX();
        const vy = v.getY();
        const vz = v.getZ();
        ke += 0.5 * (vx * vx + vy * vy + vz * vz);
    }
    const sv = app.striker.linvel(null);
    const svx = sv.getX();
    const svy = sv.getY();
    const svz = sv.getZ();
    ke += 2.25 * (svx * svx + svy * svy + svz * svz);

    var buf: [192]u8 = undefined;
    const text = std.fmt.bufPrint(
        &buf,
        "19 bodies (18 planks + sphere) · max_y={d:.2} m · ke={d:.1} · step={d} ({s})",
        .{ max_y, ke, app.step_count, if (app.paused) "paused" else "running" },
    ) catch return;
    app.meta.setTextContent(text);
}

fn stepSim(app: *App) void {
    const scope = dom.JsScope.open();
    defer scope.deinit();
    if (app.world) |w| w.step(null);
    app.step_count += 1;
    syncMeshes(app);
    updateMeta(app);
}

fn onBeforeRender(app: *App, _: f64) void {
    if (!app.paused) stepSim(app);
}

fn onBtn(ctx: *const BtnCtx, _: dom.PointerEvent) void {
    const scope = dom.JsScope.open();
    defer scope.deinit();
    const app = ctx.app;
    if (ctx.id == 0) {
        app.shot_idx += 1;
        const angle = @as(f64, @floatFromInt(app.shot_idx)) * 0.9;
        const dist: f64 = 6.2;
        const sx = -dist * @cos(angle);
        const sz = dist * @sin(angle);
        const sy = 1.2 + @as(f64, @floatFromInt(app.shot_idx % 3)) * 0.45;
        if (app.has_bodies) {
            app.striker.setTranslation(rp.Vector3.new(sx, sy, sz), true);
            app.striker.setLinvel(rp.Vector3.new(14.4 * @cos(angle), 0.9, -14.4 * @sin(angle)), true);
            app.striker.setAngvel(rp.Vector3.new(0.0, 0.0, 0.0), true);
        }
        syncMeshes(app);
        updateMeta(app);
    } else if (ctx.id == 1) {
        app.paused = !app.paused;
        app.pause_btn.setTextContent(if (app.paused) "Resume" else "Pause");
        updateMeta(app);
    } else {
        app.paused = false;
        app.pause_btn.setTextContent("Pause");
        buildWorld(app);
        for (0..12) |_| {
            if (app.world) |w| w.step(null);
            app.step_count += 1;
        }
        syncMeshes(app);
        updateMeta(app);
    }
}

fn onStarted(ok: bool, val: b.JsValue) void {
    if (!ok) setStatusError(val);
}

fn onRegistered(app: *App, ok: bool, val: b.JsValue) void {
    const scope = dom.JsScope.open();
    defer scope.deinit();
    if (!ok) {
        setStatusError(val);
        return;
    }
    b.startEngine(app.engine).thenFn(onStarted);
}

fn mkPbr(r: f64, g: f64, bl: f64, metallic: f64, roughness: f64) b.PbrMaterialProps {
    return b.createPbrMaterial(.{
        .baseColorFactor = .{ r, g, bl, 1.0 },
        .metallicFactor = metallic,
        .roughnessFactor = roughness,
    });
}

fn onEngine(app: *App, ok: bool, val: b.JsValue) void {
    const scope = dom.JsScope.open();
    defer scope.deinit();
    if (!ok) {
        setStatusError(val);
        return;
    }
    app.engine = val.as(b.EngineContext).retain();

    const scene = b.createSceneContext(app.engine, null);
    scene.setClearColor(.{ .r = 0.06, .g = 0.07, .b = 0.09, .a = 1.0 });

    const camera = b.createArcRotateCamera(0.85, 1.12, 11.5, b.vec3(0.0, 1.5, 0.0));
    b.addToScene(scene, camera);
    _ = b.attachControl(camera, app.canvas, .{ .scene = scene });

    const hemi = b.createHemisphericLight(.{ .direction = .{ 0.0, 1.0, 0.2 }, .intensity = 0.72 });
    hemi.setDiffuseColor(.{ 0.85, 0.88, 0.95 });
    hemi.setGroundColor(.{ 0.14, 0.14, 0.16 });
    b.addToScene(scene, hemi);

    const dir = b.createDirectionalLight(.{ -1.0, -2.0, -1.2 }, .{ .intensity = 2.1 });
    dir.setDiffuse(.{ 1.0, 0.97, 0.92 });
    b.addToScene(scene, dir);

    const ground = b.createGround(app.engine, .{ .width = 20.0, .height = 20.0 });
    ground.setMaterial(b.createGridMaterial(null));
    b.addToScene(scene, ground);

    const palette = [3]b.PbrMaterialProps{
        mkPbr(0.86, 0.86, 0.86, 0.15, 0.45),
        mkPbr(0.58, 0.58, 0.60, 0.25, 0.38),
        mkPbr(0.30, 0.31, 0.33, 0.35, 0.35),
    };

    for (0..PLANK_COUNT) |idx| {
        const layer = idx / PER_LAYER;
        const w_box: f64 = if (layer % 2 == 0) HALF_LONG * 2.0 else HALF_THIN * 2.0;
        const d_box: f64 = if (layer % 2 == 0) HALF_THIN * 2.0 else HALF_LONG * 2.0;
        const m = b.createBox(app.engine, b.BoxOptions.init(.{
            .width = w_box,
            .height = HALF_HEIGHT * 2.0,
            .depth = d_box,
        }));
        m.setMaterial(palette[layer % 3]);
        b.addToScene(scene, m);
        app.plank_pos[idx] = m.getPosition().retain();
        app.plank_rot[idx] = m.getRotationQuaternion().retain();
    }

    const sm = b.createSphere(app.engine, .{ .diameter = SPHERE_RADIUS * 2.0, .segments = 24.0 });
    sm.setMaterial(mkPbr(0.95, 0.42, 0.28, 0.55, 0.22));
    b.addToScene(scene, sm);
    app.striker_pos = sm.getPosition().retain();
    app.striker_rot = sm.getRotationQuaternion().retain();
    app.has_meshes = true;

    syncMeshes(app);

    b.onBeforeRender(scene, .{ onBeforeRender, app });
    b.registerScene(scene).then(onRegistered, app);
}

fn onRapierReady(app: *App, ok: bool, _: rp.JsValue) void {
    if (!ok) {
        setStatusText("Rapier3D init failed");
        return;
    }
    const scope = dom.JsScope.open();
    defer scope.deinit();

    const doc = dom.getDocument();
    app.meta = doc.getElementById("meta").?.retain();
    app.pause_btn = doc.getElementById("btn-pause").?.retain();

    buildWorld(app);
    for (0..18) |_| {
        if (app.world) |w| w.step(null);
        app.step_count += 1;
    }
    updateMeta(app);

    const ids = [3][]const u8{ "btn-launch", "btn-pause", "btn-reset" };
    for (ids, 0..) |id, i| {
        app.btns[i] = .{ .app = app, .id = i };
        doc.getElementById(id).?.addEventListener(.click, .{ onBtn, &app.btns[i] }, null);
    }

    const canvas = doc.querySelector(.canvas) orelse {
        setStatusText("WebGPU / Babylon Lite error: missing <canvas> element");
        return;
    };
    app.canvas = canvas.retain();
    b.createEngineWithEngineOptions(app.canvas, null).then(onEngine, app);
}

export fn start() void {
    const scope = dom.JsScope.open();
    defer scope.deinit();
    rp.init().then(onRapierReady, &g_app);
}
