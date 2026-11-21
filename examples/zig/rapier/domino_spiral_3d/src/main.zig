const std = @import("std");
const dom = @import("dom");
const b = @import("babylonjs_lite");
const rp = @import("rapier");

const DOM_COUNT = 14;
const CYL_COUNT = 3;
const DOM_HX: f64 = 0.09;
const DOM_HY: f64 = 0.65;
const DOM_HZ: f64 = 0.34;
const MARBLE_RADIUS: f64 = 0.36;
const RAMP_ANGLE: f64 = -0.38;
const RAMP_X: f64 = -3.55;
const RAMP_Y: f64 = 1.02;
const RAMP_Z: f64 = -2.5;

const Pose = struct { x: f64, y: f64, z: f64, yaw: f64 };
const BtnCtx = struct { app: *App, id: usize };

fn spiralPose(i: usize) Pose {
    const fi = @as(f64, @floatFromInt(i));
    const t = fi * 0.255;
    const r = 2.6 - fi * 0.045;
    return .{
        .x = -1.35 + r * @sin(t),
        .y = DOM_HY,
        .z = -2.5 + 2.6 - r * @cos(t),
        .yaw = -t,
    };
}

const App = struct {
    canvas: dom.HTMLCanvasElement = undefined,
    meta: dom.HTMLElement = undefined,
    pause_btn: dom.HTMLElement = undefined,
    engine: b.EngineContext = undefined,
    world: ?rp.World = null,
    marble: rp.RigidBody = undefined,
    doms: [DOM_COUNT]rp.RigidBody = undefined,
    cyls: [CYL_COUNT]rp.RigidBody = undefined,
    has_bodies: bool = false,
    marble_pos: b.ObservableVec3 = undefined,
    marble_rot: b.ObservableQuat = undefined,
    dom_pos: [DOM_COUNT]b.ObservableVec3 = undefined,
    dom_rot: [DOM_COUNT]b.ObservableQuat = undefined,
    cyl_pos: [CYL_COUNT]b.ObservableVec3 = undefined,
    cyl_rot: [CYL_COUNT]b.ObservableQuat = undefined,
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

fn buildWorld(app: *App) void {
    const scope = dom.JsScope.open();
    defer scope.deinit();

    if (app.world) |w| {
        if (app.has_bodies) {
            app.marble.deinit();
            for (&app.doms) |*rb| rb.deinit();
            for (&app.cyls) |*rb| rb.deinit();
        }
        w.free();
        w.deinit();
        app.world = null;
        app.has_bodies = false;
    }

    const world = rp.World.new(rp.Vector3.new(0.0, -9.81, 0.0), null).retain();

    const floor_body = world.createRigidBody(rp.RigidBodyDesc.fixed().setTranslation2(0.0, -0.3, 0.0));
    _ = world.createCollider(
        rp.ColliderDesc.cuboid(10.0, 0.3, 10.0).setFriction2(0.75).setRestitution2(0.1),
        floor_body,
    );

    const ramp_rot = rp.Quaternion.new(0.0, 0.0, @sin(RAMP_ANGLE * 0.5), @cos(RAMP_ANGLE * 0.5));
    const ramp_body = world.createRigidBody(
        rp.RigidBodyDesc.fixed()
            .setTranslation2(RAMP_X, RAMP_Y, RAMP_Z)
            .setRotation2(ramp_rot),
    );
    _ = world.createCollider(
        rp.ColliderDesc.cuboid(1.75, 0.10, 0.65).setFriction2(0.15).setRestitution2(0.2),
        ramp_body,
    );

    const marble = world.createRigidBody(
        rp.RigidBodyDesc.dynamic()
            .setTranslation2(-4.75, 2.02, -2.5)
            .setLinvel2(3.0, 0.0, 0.0),
    );
    _ = world.createCollider(
        rp.ColliderDesc.ball(MARBLE_RADIUS)
            .setDensity2(3.2)
            .setRestitution2(0.32)
            .setFriction2(0.18),
        marble,
    );
    app.marble = marble.retain();

    for (0..DOM_COUNT) |i| {
        const pose = spiralPose(i);
        const q = rp.Quaternion.new(0.0, @sin(pose.yaw * 0.5), 0.0, @cos(pose.yaw * 0.5));
        const rb = world.createRigidBody(
            rp.RigidBodyDesc.dynamic().setTranslation2(pose.x, pose.y, pose.z).setRotation2(q),
        );
        _ = world.createCollider(
            rp.ColliderDesc.cuboid(DOM_HX, DOM_HY, DOM_HZ)
                .setFriction2(0.55)
                .setRestitution2(0.18)
                .setDensity2(1.0),
            rb,
        );
        app.doms[i] = rb.retain();
    }

    const end_pose = spiralPose(DOM_COUNT - 1);
    const fwd_x = @cos(-end_pose.yaw);
    const fwd_z = @sin(-end_pose.yaw);
    const perp_x = -fwd_z;
    const perp_z = fwd_x;
    for (0..CYL_COUNT) |j| {
        const spread = (@as(f64, @floatFromInt(j)) - 1.0) * 0.52;
        const cx = end_pose.x + fwd_x * 0.72 + perp_x * spread;
        const cz = end_pose.z + fwd_z * 0.72 + perp_z * spread;
        const rb = world.createRigidBody(rp.RigidBodyDesc.dynamic().setTranslation2(cx, 0.42, cz));
        _ = world.createCollider(
            rp.ColliderDesc.cylinder(0.42, 0.25)
                .setFriction2(0.55)
                .setRestitution2(0.25)
                .setDensity2(1.0),
            rb,
        );
        app.cyls[j] = rb.retain();
    }

    app.has_bodies = true;
    app.world = world;
    app.step_count = 0;
}

fn syncPair(rb: rp.RigidBody, pos: b.ObservableVec3, rot: b.ObservableQuat) void {
    const p = rb.translation(null);
    const q = rb.rotation(null);
    pos.set(p.getX(), p.getY(), p.getZ());
    rot.set(q.getX(), q.getY(), q.getZ(), q.getW());
}

fn syncMeshes(app: *App) void {
    if (!app.has_meshes or !app.has_bodies) return;
    const scope = dom.JsScope.open();
    defer scope.deinit();

    syncPair(app.marble, app.marble_pos, app.marble_rot);
    for (0..DOM_COUNT) |i| syncPair(app.doms[i], app.dom_pos[i], app.dom_rot[i]);
    for (0..CYL_COUNT) |j| syncPair(app.cyls[j], app.cyl_pos[j], app.cyl_rot[j]);
}

fn bodyKe(rb: rp.RigidBody) f64 {
    const v = rb.linvel(null);
    const vx = v.getX();
    const vy = v.getY();
    const vz = v.getZ();
    return 0.5 * (vx * vx + vy * vy + vz * vz);
}

fn updateMeta(app: *App) void {
    if (!app.has_bodies) return;
    const scope = dom.JsScope.open();
    defer scope.deinit();

    var fallen: usize = 0;
    var ke = bodyKe(app.marble);
    for (0..DOM_COUNT) |i| {
        const p = app.doms[i].translation(null);
        if (p.getY() < 0.48) fallen += 1;
        ke += bodyKe(app.doms[i]);
    }
    for (0..CYL_COUNT) |j| {
        ke += bodyKe(app.cyls[j]);
    }

    var buf: [192]u8 = undefined;
    const text = std.fmt.bufPrint(
        &buf,
        "18 bodies (marble + 14 dominoes + 3 cylinders) · fallen={d} · ke={d:.1} · step={d} ({s})",
        .{ fallen, ke, app.step_count, if (app.paused) "paused" else "running" },
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
        if (app.has_bodies) {
            app.marble.setTranslation(rp.Vector3.new(-4.75, 2.02, -2.5), true);
            app.marble.setLinvel(rp.Vector3.new(3.0, 0.0, 0.0), true);
            app.marble.setAngvel(rp.Vector3.new(0.0, 0.0, 0.0), true);
        }
        syncMeshes(app);
        updateMeta(app);
    } else if (ctx.id == 1) {
        if (app.has_bodies) {
            app.doms[0].applyImpulse(rp.Vector3.new(1.6, 0.0, 0.0), true);
        }
        updateMeta(app);
    } else if (ctx.id == 2) {
        app.paused = !app.paused;
        app.pause_btn.setTextContent(if (app.paused) "Resume" else "Pause");
        updateMeta(app);
    } else {
        app.paused = false;
        app.pause_btn.setTextContent("Pause");
        buildWorld(app);
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

    const camera = b.createArcRotateCamera(0.65, 1.05, 12.0, b.vec3(-0.4, 0.8, 0.0));
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

    const ramp_mesh = b.createBox(app.engine, b.BoxOptions.init(.{
        .width = 3.5,
        .height = 0.2,
        .depth = 1.3,
    }));
    ramp_mesh.getPosition().set(RAMP_X, RAMP_Y, RAMP_Z);
    ramp_mesh.getRotationQuaternion().set(0.0, 0.0, @sin(RAMP_ANGLE * 0.5), @cos(RAMP_ANGLE * 0.5));
    ramp_mesh.setMaterial(mkPbr(0.28, 0.30, 0.34, 0.35, 0.48));
    b.addToScene(scene, ramp_mesh);

    const mb = b.createSphere(app.engine, .{ .diameter = MARBLE_RADIUS * 2.0, .segments = 24.0 });
    mb.setMaterial(mkPbr(0.95, 0.42, 0.28, 0.55, 0.2));
    b.addToScene(scene, mb);
    app.marble_pos = mb.getPosition().retain();
    app.marble_rot = mb.getRotationQuaternion().retain();

    const mat_a = mkPbr(0.88, 0.88, 0.88, 0.15, 0.4);
    const mat_b = mkPbr(0.45, 0.46, 0.48, 0.25, 0.38);
    const dom_opts = b.BoxOptions.init(.{
        .width = DOM_HX * 2.0,
        .height = DOM_HY * 2.0,
        .depth = DOM_HZ * 2.0,
    });
    for (0..DOM_COUNT) |i| {
        const m = b.createBox(app.engine, dom_opts);
        m.setMaterial(if (i % 2 == 0) mat_a else mat_b);
        b.addToScene(scene, m);
        app.dom_pos[i] = m.getPosition().retain();
        app.dom_rot[i] = m.getRotationQuaternion().retain();
    }

    const cyl_mat = mkPbr(0.26, 0.62, 0.88, 0.45, 0.25);
    const cyl_opts = b.CylinderOptions.init(.{
        .height = 0.84,
        .diameter = 0.5,
        .tessellation = 24.0,
    });
    for (0..CYL_COUNT) |j| {
        const m = b.createCylinder(app.engine, cyl_opts);
        m.setMaterial(cyl_mat);
        b.addToScene(scene, m);
        app.cyl_pos[j] = m.getPosition().retain();
        app.cyl_rot[j] = m.getRotationQuaternion().retain();
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

    buildWorld(app);
    for (0..20) |_| {
        if (app.world) |w| w.step(null);
        app.step_count += 1;
    }
    updateMeta(app);

    const ids = [4][]const u8{ "btn-launch", "btn-nudge", "btn-pause", "btn-reset" };
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
