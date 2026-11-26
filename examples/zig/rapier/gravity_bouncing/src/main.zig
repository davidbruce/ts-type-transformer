const std = @import("std");
const dom = @import("dom");
const rp = @import("rapier");

const TWO_PI: f64 = 2.0 * std.math.pi;
const TRAIL_LEN = 20;
const RESTITUTIONS = [5]f64{ 0.00, 0.35, 0.60, 0.80, 0.95 };
const X_COORDS = [5]f64{ -4.2, -2.1, 0.0, 2.1, 4.2 };
const BALL_HEX = [5][]const u8{ "#efefef", "#d4d4d4", "#b5b5b5", "#737373", "#262626" };
const REST_LABELS = [5][]const u8{ "e=0.00", "e=0.35", "e=0.60", "e=0.80", "e=0.95" };

fn toCx(wx: f64) f64 {
    return 340.0 + wx * 46.0;
}
fn toCy(wy: f64) f64 {
    return 350.0 - wy * 46.0;
}

const App = struct {
    ctx: dom.CanvasRenderingContext2D = undefined,
    meta: dom.HTMLElement = undefined,
    grav_btn: dom.HTMLElement = undefined,
    pause_btn: dom.HTMLElement = undefined,
    raf_cb: dom.FrameRequestCallback = undefined,
    world: ?rp.World = null,
    balls: [5]rp.RigidBody = undefined,
    btns: [4]BtnCtx = undefined,
    trail_x: [5][TRAIL_LEN]f64 = std.mem.zeroes([5][TRAIL_LEN]f64),
    trail_y: [5][TRAIL_LEN]f64 = std.mem.zeroes([5][TRAIL_LEN]f64),
    trail_count: usize = 0,
    step_count: usize = 0,
    earth: bool = true,
    paused: bool = false,
};

const BtnCtx = struct { app: *App, id: usize };

var g_app: App = .{};

fn drawDashed(ctx: dom.CanvasRenderingContext2D, x1: f64, y1: f64, x2: f64, y2: f64, segs: usize) void {
    const dx = x2 - x1;
    const dy = y2 - y1;
    var i: usize = 0;
    while (i < segs) : (i += 2) {
        const t0 = @as(f64, @floatFromInt(i)) / @as(f64, @floatFromInt(segs));
        const t1 = @as(f64, @floatFromInt(i + 1)) / @as(f64, @floatFromInt(segs));
        ctx.beginPath();
        ctx.moveTo(x1 + dx * t0, y1 + dy * t0);
        ctx.lineTo(x1 + dx * t1, y1 + dy * t1);
        ctx.stroke();
    }
}

fn buildScene(app: *App) void {
    const scope = rp.JsScope.open();
    defer scope.deinit();

    if (app.world) |w| {
        for (&app.balls) |b| b.deinit();
        w.free();
        w.deinit();
    }

    const w = rp.World.new(rp.Vector3.new(0.0, @as(f64, if (app.earth) -9.81 else -1.62), 0.0)).retain();
    app.world = w;

    const ground = w.createRigidBody(rp.RigidBodyDesc.fixed().setTranslation(0.0, -0.4, 0.0));
    _ = w.createColliderWith(rp.ColliderDesc.cuboid(7.0, 0.4, 2.0), .{ .parent = ground });
    const lw = w.createRigidBody(rp.RigidBodyDesc.fixed().setTranslation(-6.8, 3.5, 0.0));
    _ = w.createColliderWith(rp.ColliderDesc.cuboid(0.25, 4.0, 2.0), .{ .parent = lw });
    const rw = w.createRigidBody(rp.RigidBodyDesc.fixed().setTranslation(6.8, 3.5, 0.0));
    _ = w.createColliderWith(rp.ColliderDesc.cuboid(0.25, 4.0, 2.0), .{ .parent = rw });

    for (0..5) |i| {
        app.balls[i] = w.createRigidBody(rp.RigidBodyDesc.dynamic().setTranslation(X_COORDS[i], 5.6, 0.0)).retain();
        const cd = rp.ColliderDesc.ball(0.42).setRestitution(RESTITUTIONS[i]).setFriction(0.2).setDensity(1.2);
        _ = w.createColliderWith(cd, .{ .parent = app.balls[i] });
    }
    app.step_count = 0;
    app.trail_count = 0;
}

fn stepSim(app: *App) void {
    const scope = rp.JsScope.open();
    defer scope.deinit();

    app.world.?.step(null);
    app.step_count += 1;
    const idx = (app.step_count - 1) % TRAIL_LEN;
    for (0..5) |i| {
        const p = app.balls[i].translation(null);
        app.trail_x[i][idx] = p.getX();
        app.trail_y[i][idx] = p.getY();
    }
    if (app.trail_count < TRAIL_LEN) app.trail_count += 1;
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

    ctx.setFillStyle("#f7f7f7");
    ctx.fillRect(0.0, 350.0, 680.0, 50.0);
    ctx.setStrokeStyle("#d4d4d4");
    var hx: f64 = -20.0;
    while (hx < 700.0) : (hx += 18.0) {
        ctx.beginPath();
        ctx.moveTo(hx, 350.0);
        ctx.lineTo(hx - 24.0, 400.0);
        ctx.stroke();
    }
    ctx.setStrokeStyle("#111111");
    ctx.setLineWidth(1.5);
    ctx.beginPath();
    ctx.moveTo(0.0, 350.0);
    ctx.lineTo(680.0, 350.0);
    ctx.stroke();

    ctx.setStrokeStyle("#d4d4d4");
    ctx.setLineWidth(1.0);
    drawDashed(ctx, toCx(-5.0), toCy(5.6), toCx(5.0), toCy(5.6), 40);
    for (0..5) |i| {
        const cx = toCx(X_COORDS[i]);
        drawDashed(ctx, cx, toCy(5.6), cx, 350.0, 20);
    }

    var energy: f64 = 0.0;
    var max_y: f64 = 0.0;
    ctx.setFont("11px ui-monospace, monospace");
    for (0..5) |i| {
        ctx.setFillStyle("#b5b5b5");
        for (0..app.trail_count) |k| {
            ctx.beginPath();
            ctx.arc(toCx(app.trail_x[i][k]), toCy(app.trail_y[i][k]), 1.6, 0.0, TWO_PI);
            ctx.fill(null);
        }

        const p = app.balls[i].translation(null);
        const v = app.balls[i].linvel(null);
        const px = p.getX();
        const py = p.getY();
        const vx = v.getX();
        const vy = v.getY();
        energy += 0.5 * (vx * vx + vy * vy);
        if (py > max_y) max_y = py;

        const cx = toCx(px);
        const cy = toCy(py);
        const rpx: f64 = 0.42 * 46.0;

        ctx.setFillStyle(BALL_HEX[i]);
        ctx.beginPath();
        ctx.arc(cx, cy, rpx, 0.0, TWO_PI);
        ctx.fill(null);

        ctx.setStrokeStyle("#111111");
        ctx.setLineWidth(1.6);
        ctx.beginPath();
        ctx.arc(cx, cy, rpx, 0.0, TWO_PI);
        ctx.stroke();

        ctx.setFillStyle(if (i >= 3) "#d4d4d4" else "#737373");
        ctx.beginPath();
        ctx.arc(cx - 5.0, cy - 5.0, 2.8, 0.0, TWO_PI);
        ctx.fill(null);

        ctx.setFillStyle("#111111");
        ctx.fillText(REST_LABELS[i], cx - 22.0, 368.0);
    }

    var buf: [256]u8 = undefined;
    const msg = std.fmt.bufPrint(
        &buf,
        "5 spheres · g={s} m/s² · max_y={d:.2} m · ke={d:.1} · step={d} ({s})",
        .{ if (app.earth) "Earth (-9.81)" else "Moon (-1.62)", max_y, energy, app.step_count, if (app.paused) "paused" else "running" },
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
        app.earth = !app.earth;
        app.world.?.setGravity(rp.Vector3.new(0.0, @as(f64, if (app.earth) -9.81 else -1.62), 0.0));
        const imp = rp.Vector3.new(0.0, 0.25, 0.0);
        for (0..5) |i| app.balls[i].applyImpulse(imp, true);
        app.grav_btn.setTextContent(if (app.earth) "Gravity: Earth" else "Gravity: Moon");
        render(app);
    } else if (ctx.id == 1) {
        for (0..5) |i| {
            const dx = (@as(f64, @floatFromInt(i)) - 2.0) * 0.28;
            app.balls[i].applyImpulse(rp.Vector3.new(dx, 3.4, 0.0), true);
        }
        render(app);
    } else if (ctx.id == 2) {
        app.paused = !app.paused;
        app.pause_btn.setTextContent(if (app.paused) "Resume" else "Pause");
        render(app);
    } else {
        app.paused = false;
        app.pause_btn.setTextContent("Pause");
        buildScene(app);
        for (0..18) |_| stepSim(app);
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
    app.grav_btn = doc.getElementById("btn-gravity").?.retain();
    app.pause_btn = doc.getElementById("btn-pause").?.retain();

    buildScene(app);
    for (0..28) |_| stepSim(app);
    render(app);

    const ids = [4][]const u8{ "btn-gravity", "btn-kick", "btn-pause", "btn-reset" };
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
