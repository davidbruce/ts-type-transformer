const std = @import("std");
const dom = @import("dom");
const rp = @import("rapier");

const TWO_PI: f64 = 2.0 * std.math.pi;
const BODY_COUNT = 6;
const IS_BOX = [BODY_COUNT]bool{ false, false, true, false, true, false };

fn toCx(wx: f64) f64 {
    return 340.0 + wx * 46.0;
}
fn toCy(wy: f64) f64 {
    return 350.0 - wy * 46.0;
}

const App = struct {
    ctx: dom.CanvasRenderingContext2D = undefined,
    meta: dom.HTMLElement = undefined,
    pause_btn: dom.HTMLElement = undefined,
    raf_cb: dom.FrameRequestCallback = undefined,
    col_cb: rp.DrainCollisionEventsFn = undefined,
    world: ?rp.World = null,
    queue: ?rp.EventQueue = null,
    bodies: [BODY_COUNT]rp.RigidBody = undefined,
    btns: [3]BtnCtx = undefined,
    sensor_handle: f64 = 0.0,
    step_count: usize = 0,
    solid_contacts: usize = 0,
    sensor_crossings: usize = 0,
    active_in_sensor: usize = 0,
    paused: bool = false,
};

const BtnCtx = struct { app: *App, id: usize };

var g_app: App = .{};

fn drawRotBox(ctx: dom.CanvasRenderingContext2D, cx: f64, cy: f64, qx: f64, qy: f64, qz: f64, qw: f64, hx: f64, hy: f64, fill: []const u8) void {
    const r00 = 1.0 - 2.0 * (qy * qy + qz * qz);
    const r01 = 2.0 * (qx * qy - qw * qz);
    const r10 = 2.0 * (qx * qy + qw * qz);
    const r11 = 1.0 - 2.0 * (qx * qx + qz * qz);

    const xs = [4]f64{
        toCx(cx + r00 * (-hx) + r01 * (-hy)),
        toCx(cx + r00 * hx + r01 * (-hy)),
        toCx(cx + r00 * hx + r01 * hy),
        toCx(cx + r00 * (-hx) + r01 * hy),
    };
    const ys = [4]f64{
        toCy(cy + r10 * (-hx) + r11 * (-hy)),
        toCy(cy + r10 * hx + r11 * (-hy)),
        toCy(cy + r10 * hx + r11 * hy),
        toCy(cy + r10 * (-hx) + r11 * hy),
    };

    ctx.setFillStyle(fill);
    ctx.beginPath();
    ctx.moveTo(xs[0], ys[0]);
    for (1..4) |i| ctx.lineTo(xs[i], ys[i]);
    ctx.closePath();
    ctx.fill(null);

    ctx.setStrokeStyle("#111111");
    ctx.setLineWidth(1.5);
    ctx.beginPath();
    ctx.moveTo(xs[0], ys[0]);
    for (1..4) |i| ctx.lineTo(xs[i], ys[i]);
    ctx.closePath();
    ctx.stroke();
}

fn drawSegs(ctx: dom.CanvasRenderingContext2D, x1: f64, y1: f64, x2: f64, y2: f64, n: usize) void {
    const dx = x2 - x1;
    const dy = y2 - y1;
    var i: usize = 0;
    while (i < n) : (i += 2) {
        const t0 = @as(f64, @floatFromInt(i)) / @as(f64, @floatFromInt(n));
        const t1 = @as(f64, @floatFromInt(i + 1)) / @as(f64, @floatFromInt(n));
        ctx.beginPath();
        ctx.moveTo(x1 + dx * t0, y1 + dy * t0);
        ctx.lineTo(x1 + dx * t1, y1 + dy * t1);
        ctx.stroke();
    }
}

fn addRamp(w: rp.World, cx: f64, cy: f64, hx: f64, hy: f64, theta: f64) void {
    const q = rp.Quaternion.new(0.0, 0.0, @sin(theta * 0.5), @cos(theta * 0.5));
    const rb = w.createRigidBody(rp.RigidBodyDesc.fixed().setTranslation(cx, cy, 0.0).setRotation(q));
    _ = w.createColliderWith(rp.ColliderDesc.cuboid(hx, hy, 1.0).setFriction(0.15).setRestitution(0.35), .{ .parent = rb });
}

fn onCollision(app: *App, h1: f64, h2: f64, started: bool) void {
    if (h1 == app.sensor_handle or h2 == app.sensor_handle) {
        if (started) {
            app.sensor_crossings += 1;
            app.active_in_sensor += 1;
        } else if (app.active_in_sensor > 0) {
            app.active_in_sensor -= 1;
        }
    } else if (started) {
        app.solid_contacts += 1;
    }
}

fn buildScene(app: *App) void {
    const scope = rp.JsScope.open();
    defer scope.deinit();

    if (app.queue) |q| {
        q.free();
        q.deinit();
    }
    if (app.world) |w| {
        for (&app.bodies) |b| b.deinit();
        w.free();
        w.deinit();
    }

    const w = rp.World.new(rp.Vector3.new(0.0, -9.81, 0.0)).retain();
    app.world = w;
    app.queue = rp.EventQueue.new(true).retain();

    addRamp(w, 0.0, -0.35, 6.8, 0.35, 0.0);
    addRamp(w, -5.4, 2.8, 0.25, 3.2, 0.0);
    addRamp(w, 5.4, 2.8, 0.25, 3.2, 0.0);
    addRamp(w, -2.1, 4.2, 2.2, 0.18, -0.36);
    addRamp(w, 1.8, 2.6, 2.2, 0.18, 0.36);

    const sbody = w.createRigidBody(rp.RigidBodyDesc.fixed().setTranslation(0.0, 1.35, 0.0));
    const scd = rp.ColliderDesc.cuboid(1.85, 0.30, 1.2).setSensor(true).setActiveEvents(.COLLISION_EVENTS);
    app.sensor_handle = w.createColliderWith(scd, .{ .parent = sbody }).getHandle();

    for (0..BODY_COUNT) |i| {
        const x = -3.7 + @as(f64, @floatFromInt(i)) * 0.52;
        const y = 5.4 + @as(f64, @floatFromInt(i % 3)) * 0.58;
        app.bodies[i] = w.createRigidBody(rp.RigidBodyDesc.dynamic().setTranslation(x, y, 0.0)).retain();
        const base_cd = if (IS_BOX[i]) rp.ColliderDesc.cuboid(0.28, 0.28, 0.28) else rp.ColliderDesc.ball(0.32);
        const cd = base_cd.setRestitution(0.35).setFriction(0.25).setActiveEvents(.COLLISION_EVENTS);
        _ = w.createColliderWith(cd, .{ .parent = app.bodies[i] });
    }

    app.step_count = 0;
    app.solid_contacts = 0;
    app.sensor_crossings = 0;
    app.active_in_sensor = 0;
}

fn stepSim(app: *App) void {
    const scope = rp.JsScope.open();
    defer scope.deinit();

    app.world.?.step(.{ .eventQueue = app.queue.? });
    app.step_count += 1;
    app.queue.?.drainCollisionEvents(app.col_cb);
}

fn render(app: *App) void {
    const scope = rp.JsScope.open();
    defer scope.deinit();

    const ctx = app.ctx;
    ctx.setFillStyle("#ffffff");
    ctx.fillRect(0.0, 0.0, 680.0, 400.0);

    ctx.setStrokeStyle("#efefef");
    ctx.setLineWidth(1.0);
    var gx: i32 = -6;
    while (gx <= 6) : (gx += 1) {
        const x = toCx(@floatFromInt(gx));
        ctx.beginPath();
        ctx.moveTo(x, 0.0);
        ctx.lineTo(x, 350.0);
        ctx.stroke();
    }
    var gy: i32 = 1;
    while (gy <= 7) : (gy += 1) {
        const y = toCy(@floatFromInt(gy));
        ctx.beginPath();
        ctx.moveTo(0.0, y);
        ctx.lineTo(680.0, y);
        ctx.stroke();
    }

    const sx = toCx(-1.85);
    const sy = toCy(1.65);
    const sw = 3.70 * 46.0;
    const sh = 0.60 * 46.0;
    if (app.active_in_sensor > 0) {
        ctx.setFillStyle("#efefef");
        ctx.fillRect(sx, sy, sw, sh);
        ctx.setStrokeStyle("#111111");
        ctx.setLineWidth(2.0);
        ctx.strokeRect(sx, sy, sw, sh);
    } else {
        ctx.setStrokeStyle("#737373");
        ctx.setLineWidth(1.4);
        drawSegs(ctx, sx, sy, sx + sw, sy, 14);
        drawSegs(ctx, sx + sw, sy, sx + sw, sy + sh, 4);
        drawSegs(ctx, sx + sw, sy + sh, sx, sy + sh, 14);
        drawSegs(ctx, sx, sy + sh, sx, sy, 4);
    }

    var gbuf: [64]u8 = undefined;
    const gcap = std.fmt.bufPrint(&gbuf, "SENSOR GATE ({d} inside)", .{app.active_in_sensor}) catch return;
    ctx.setFillStyle("#555555");
    ctx.setFont("11px ui-monospace, monospace");
    ctx.fillText(gcap, sx + 8.0, sy + 17.0);

    drawRotBox(ctx, 0.0, -0.35, 0.0, 0.0, 0.0, 1.0, 6.8, 0.35, "#f7f7f7");
    drawRotBox(ctx, -5.4, 2.8, 0.0, 0.0, 0.0, 1.0, 0.25, 3.2, "#efefef");
    drawRotBox(ctx, 5.4, 2.8, 0.0, 0.0, 0.0, 1.0, 0.25, 3.2, "#efefef");
    drawRotBox(ctx, -2.1, 4.2, 0.0, 0.0, @sin(-0.18), @cos(-0.18), 2.2, 0.18, "#e5e5e5");
    drawRotBox(ctx, 1.8, 2.6, 0.0, 0.0, @sin(0.18), @cos(0.18), 2.2, 0.18, "#e5e5e5");

    for (0..BODY_COUNT) |i| {
        const p = app.bodies[i].translation(null);
        const q = app.bodies[i].rotation(null);
        const px = p.getX();
        const py = p.getY();
        if (IS_BOX[i]) {
            drawRotBox(ctx, px, py, q.getX(), q.getY(), q.getZ(), q.getW(), 0.28, 0.28, "#d4d4d4");
        } else {
            const cx = toCx(px);
            const cy = toCy(py);
            ctx.setFillStyle("#262626");
            ctx.beginPath();
            ctx.arc(cx, cy, 0.32 * 46.0, 0.0, TWO_PI);
            ctx.fill(null);
            ctx.setStrokeStyle("#111111");
            ctx.setLineWidth(1.5);
            ctx.beginPath();
            ctx.arc(cx, cy, 0.32 * 46.0, 0.0, TWO_PI);
            ctx.stroke();
        }
    }

    var mbuf: [256]u8 = undefined;
    const msg = std.fmt.bufPrint(
        &mbuf,
        "EventQueue · solid_contacts={d} · sensor_crossings={d} · inside={d} · step={d} ({s})",
        .{ app.solid_contacts, app.sensor_crossings, app.active_in_sensor, app.step_count, if (app.paused) "paused" else "running" },
    ) catch return;
    app.meta.setTextContent(msg);
}

fn onRaf(app: *App, _: f64) void {
    if (!app.paused) {
        stepSim(app);
        render(app);
    }
    _ = dom.requestAnimationFrame(app.raf_cb);
}

fn onBtn(ctx: *const BtnCtx, _: dom.PointerEvent) void {
    const scope = rp.JsScope.open();
    defer scope.deinit();

    const app = ctx.app;
    if (ctx.id == 0) {
        const v = rp.Vector3.new(0.4, 0.0, 0.0);
        for (0..BODY_COUNT) |i| {
            const x = -3.7 + @as(f64, @floatFromInt(i)) * 0.52;
            const y = 5.4 + @as(f64, @floatFromInt(i % 3)) * 0.58;
            app.bodies[i].setTranslation(rp.Vector3.new(x, y, 0.0), true);
            app.bodies[i].setLinvel(v, true);
        }
        render(app);
    } else if (ctx.id == 1) {
        app.paused = !app.paused;
        app.pause_btn.setTextContent(if (app.paused) "Resume" else "Pause");
        render(app);
    } else {
        app.paused = false;
        app.pause_btn.setTextContent("Pause");
        buildScene(app);
        for (0..25) |_| stepSim(app);
        render(app);
    }
}

fn onInit(app: *App, ok: bool, _: rp.JsValue) void {
    if (!ok) return;
    const scope = rp.JsScope.open();
    defer scope.deinit();

    const doc = dom.getDocument();
    const canvas = doc.querySelector(.canvas) orelse return;
    app.ctx = canvas.getContext2d(null).?.retain();
    app.meta = doc.getElementById("meta").?.retain();
    app.pause_btn = doc.getElementById("btn-pause").?.retain();
    app.col_cb = rp.DrainCollisionEventsFn.from(onCollision, app).retain();

    buildScene(app);
    for (0..40) |_| stepSim(app);
    render(app);

    const ids = [3][]const u8{ "btn-drop", "btn-pause", "btn-reset" };
    for (ids, 0..) |id, i| {
        app.btns[i] = .{ .app = app, .id = i };
        doc.getElementById(id).?.addEventListener(.click, .{ onBtn, &app.btns[i] });
    }
    app.raf_cb = dom.FrameRequestCallback.from(onRaf, app).retain();
    _ = dom.requestAnimationFrame(app.raf_cb);
}

export fn start() void {
    const scope = rp.JsScope.open();
    defer scope.deinit();
    rp.init().then(onInit, &g_app);
}
