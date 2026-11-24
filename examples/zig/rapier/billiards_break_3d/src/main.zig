const std = @import("std");
const dom = @import("dom");
const b = @import("babylonjs_lite");
const rp = @import("rapier");

const RACK_COUNT = 10;
const BALL_COUNT = 11;
const BALL_RADIUS: f64 = 0.36;

const BtnCtx = struct { app: *App, id: usize };

const App = struct {
    canvas: dom.HTMLCanvasElement = undefined,
    meta: dom.HTMLElement = undefined,
    pause_btn: dom.HTMLElement = undefined,
    engine: b.EngineContext = undefined,
    world: ?rp.World = null,
    balls: [BALL_COUNT]rp.RigidBody = undefined,
    has_bodies: bool = false,
    ball_pos: [BALL_COUNT]b.ObservableVec3 = undefined,
    ball_rot: [BALL_COUNT]b.ObservableQuat = undefined,
    has_meshes: bool = false,
    btns: [4]BtnCtx = undefined,
    paused: bool = false,
    step_count: usize = 0,
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

fn buildWorld(app: *App, shoot_cue: bool) void {
    const scope = dom.JsScope.open();
    defer scope.deinit();

    if (app.world) |w| {
        if (app.has_bodies) {
            for (&app.balls) |*rb| rb.deinit();
        }
        w.free();
        w.deinit();
        app.world = null;
        app.has_bodies = false;
    }

    const world = rp.World.new(rp.Vector3.new(0.0, -9.81, 0.0), null).retain();

    const bed = world.createRigidBody(rp.RigidBodyDesc.fixed().setTranslation(0.0, -0.25, 0.0));
    _ = world.createCollider(
        rp.ColliderDesc.cuboid(4.6, 0.25, 7.6).setFriction(0.14).setRestitution(0.35),
        bed,
    );

    const RailsSpec = struct { rx: f64, ry: f64, rz: f64, hx: f64, hy: f64, hz: f64 };
    const rails = [4]RailsSpec{
        .{ .rx = -4.35, .ry = 0.45, .rz = 0.0, .hx = 0.25, .hy = 0.45, .hz = 7.6 },
        .{ .rx = 4.35, .ry = 0.45, .rz = 0.0, .hx = 0.25, .hy = 0.45, .hz = 7.6 },
        .{ .rx = 0.0, .ry = 0.45, .rz = -7.35, .hx = 4.6, .hy = 0.45, .hz = 0.25 },
        .{ .rx = 0.0, .ry = 0.45, .rz = 7.35, .hx = 4.6, .hy = 0.45, .hz = 0.25 },
    };
    for (rails) |r| {
        const rb = world.createRigidBody(rp.RigidBodyDesc.fixed().setTranslation(r.rx, r.ry, r.rz));
        _ = world.createCollider(
            rp.ColliderDesc.cuboid(r.hx, r.hy, r.hz).setFriction(0.12).setRestitution(0.88),
            rb,
        );
    }

    var idx: usize = 0;
    for (0..4) |row| {
        for (0..row + 1) |col| {
            const frow = @as(f64, @floatFromInt(row));
            const fcol = @as(f64, @floatFromInt(col));
            const x = (fcol - frow * 0.5) * 0.74;
            const z = -1.6 - frow * 0.645;
            const rb = world.createRigidBody(
                rp.RigidBodyDesc.dynamic()
                    .setTranslation(x, BALL_RADIUS, z)
                    .setLinearDamping(0.30)
                    .setAngularDamping(0.30),
            );
            _ = world.createCollider(
                rp.ColliderDesc.ball(BALL_RADIUS)
                    .setRestitution(0.92)
                    .setFriction(0.10)
                    .setDensity(1.0),
                rb,
            );
            app.balls[idx] = rb.retain();
            idx += 1;
        }
    }

    var cue_desc = rp.RigidBodyDesc.dynamic()
        .setTranslation(0.04, BALL_RADIUS, 4.4)
        .setLinearDamping(0.22)
        .setAngularDamping(0.22);
    if (shoot_cue) {
        cue_desc = cue_desc.setLinvel(-0.18, 0.0, -16.0);
    }
    const cue = world.createRigidBody(cue_desc);
    _ = world.createCollider(
        rp.ColliderDesc.ball(BALL_RADIUS)
            .setRestitution(0.94)
            .setFriction(0.10)
            .setDensity(1.25),
        cue,
    );
    app.balls[RACK_COUNT] = cue.retain();
    app.has_bodies = true;
    app.world = world;
    app.step_count = 0;
}

fn syncMeshes(app: *App) void {
    if (!app.has_meshes or !app.has_bodies) return;
    const scope = dom.JsScope.open();
    defer scope.deinit();

    for (0..BALL_COUNT) |i| {
        const p = app.balls[i].translation(null);
        const q = app.balls[i].rotation(null);
        app.ball_pos[i].set(p.getX(), p.getY(), p.getZ());
        app.ball_rot[i].set(q.getX(), q.getY(), q.getZ(), q.getW());
    }
}

fn updateMeta(app: *App) void {
    if (!app.has_bodies) return;
    const scope = dom.JsScope.open();
    defer scope.deinit();

    var moving: usize = 0;
    var ke: f64 = 0.0;
    for (0..BALL_COUNT) |i| {
        const v = app.balls[i].linvel(null);
        const vx = v.getX();
        const vy = v.getY();
        const vz = v.getZ();
        const s2 = vx * vx + vy * vy + vz * vz;
        if (s2 > 0.05) moving += 1;
        ke += 0.5 * s2;
    }

    var buf: [160]u8 = undefined;
    const text = std.fmt.bufPrint(
        &buf,
        "11 spheres · moving={d} · ke={d:.1} · step={d} ({s})",
        .{ moving, ke, app.step_count, if (app.paused) "paused" else "running" },
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
        app.paused = false;
        app.pause_btn.setTextContent("Pause");
        buildWorld(app, true);
        syncMeshes(app);
        updateMeta(app);
    } else if (ctx.id == 1) {
        if (app.has_bodies) {
            for (0..BALL_COUNT) |i| {
                const ang = @as(f64, @floatFromInt(i)) * 1.15;
                app.balls[i].applyImpulse(rp.Vector3.new(@cos(ang) * 1.4, 0.25, @sin(ang) * 1.4), true);
            }
        }
        updateMeta(app);
    } else if (ctx.id == 2) {
        app.paused = !app.paused;
        app.pause_btn.setTextContent(if (app.paused) "Resume" else "Pause");
        updateMeta(app);
    } else {
        app.paused = false;
        app.pause_btn.setTextContent("Pause");
        buildWorld(app, false);
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
    scene.setClearColor(.{ .r = 0.05, .g = 0.07, .b = 0.09, .a = 1.0 });

    const camera = b.createArcRotateCamera(1.18, 0.92, 13.5, b.vec3(0.0, 0.3, -0.4));
    b.addToScene(scene, camera);
    _ = b.attachControl(camera, app.canvas, .{ .scene = scene });

    const hemi = b.createHemisphericLight(.{ .direction = .{ 0.0, 1.0, 0.2 }, .intensity = 0.72 });
    hemi.setDiffuseColor(.{ 0.85, 0.9, 0.95 });
    hemi.setGroundColor(.{ 0.12, 0.14, 0.16 });
    b.addToScene(scene, hemi);

    const dir = b.createDirectionalLight(.{ -1.0, -2.0, -1.0 }, .{ .intensity = 2.1 });
    dir.setDiffuse(.{ 1.0, 0.97, 0.92 });
    b.addToScene(scene, dir);

    const table = b.createBox(app.engine, b.BoxOptions.init(.{ .width = 9.2, .height = 0.5, .depth = 15.2 }));
    table.getPosition().set(0.0, -0.25, 0.0);
    table.setMaterial(mkPbr(0.08, 0.24, 0.18, 0.05, 0.78));
    b.addToScene(scene, table);

    const rail_mat = mkPbr(0.18, 0.18, 0.20, 0.35, 0.42);
    const RailMeshSpec = struct { rx: f64, ry: f64, rz: f64, w: f64, h: f64, d: f64 };
    const rails = [4]RailMeshSpec{
        .{ .rx = -4.35, .ry = 0.45, .rz = 0.0, .w = 0.5, .h = 0.9, .d = 15.2 },
        .{ .rx = 4.35, .ry = 0.45, .rz = 0.0, .w = 0.5, .h = 0.9, .d = 15.2 },
        .{ .rx = 0.0, .ry = 0.45, .rz = -7.35, .w = 9.2, .h = 0.9, .d = 0.5 },
        .{ .rx = 0.0, .ry = 0.45, .rz = 7.35, .w = 9.2, .h = 0.9, .d = 0.5 },
    };
    for (rails) |rm| {
        const r = b.createBox(app.engine, b.BoxOptions.init(.{ .width = rm.w, .height = rm.h, .depth = rm.d }));
        r.getPosition().set(rm.rx, rm.ry, rm.rz);
        r.setMaterial(rail_mat);
        b.addToScene(scene, r);
    }

    const cue_mat = mkPbr(0.96, 0.96, 0.95, 0.15, 0.18);
    const rack_mats = [4]b.PbrMaterialProps{
        mkPbr(0.85, 0.24, 0.22, 0.2, 0.22),
        mkPbr(0.22, 0.48, 0.85, 0.2, 0.22),
        mkPbr(0.92, 0.68, 0.18, 0.2, 0.22),
        mkPbr(0.20, 0.20, 0.22, 0.25, 0.2),
    };

    const sopts = b.SphereOptions.init(.{ .diameter = BALL_RADIUS * 2.0, .segments = 24.0 });
    for (0..BALL_COUNT) |i| {
        const m = b.createSphere(app.engine, sopts);
        m.setMaterial(if (i == RACK_COUNT) cue_mat else rack_mats[i % 4]);
        b.addToScene(scene, m);
        app.ball_pos[i] = m.getPosition().retain();
        app.ball_rot[i] = m.getRotationQuaternion().retain();
    }
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

    buildWorld(app, true);
    for (0..26) |_| {
        if (app.world) |w| w.step(null);
        app.step_count += 1;
    }
    updateMeta(app);

    const ids = [4][]const u8{ "btn-break", "btn-scatter", "btn-pause", "btn-reset" };
    for (ids, 0..) |id, i| {
        app.btns[i] = .{ .app = app, .id = i };
        doc.getElementById(id).?.addEventListener(.click, .{ onBtn, &app.btns[i] }, null);
    }

    const canvas = doc.querySelector(.canvas) orelse {
        setStatusText("WebGPU / Babylon Lite error: missing <canvas> element");
        return;
    };
    app.canvas = canvas.retain();
    b.createEngine(app.canvas, null).then(onEngine, app);
}

export fn start() void {
    const scope = dom.JsScope.open();
    defer scope.deinit();
    rp.init().then(onRapierReady, &g_app);
}
