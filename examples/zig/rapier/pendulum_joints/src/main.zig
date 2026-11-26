const std = @import("std");
const dom = @import("dom");
const rp = @import("rapier");

const TWO_PI: f64 = 2.0 * std.math.pi;
const TRAIL_LEN = 36;
const LINK_HX: f64 = 0.14;
const LINK_HY: f64 = 0.62;

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
    world: ?rp.World = null,
    chain: [3]rp.RigidBody = undefined,
    arm: rp.RigidBody = undefined,
    bob: rp.RigidBody = undefined,
    btns: [3]BtnCtx = undefined,
    ax: [TRAIL_LEN]f64 = std.mem.zeroes([TRAIL_LEN]f64),
    ay: [TRAIL_LEN]f64 = std.mem.zeroes([TRAIL_LEN]f64),
    bx: [TRAIL_LEN]f64 = std.mem.zeroes([TRAIL_LEN]f64),
    by: [TRAIL_LEN]f64 = std.mem.zeroes([TRAIL_LEN]f64),
    trail_count: usize = 0,
    step_count: usize = 0,
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

fn drawPivot(ctx: dom.CanvasRenderingContext2D, wx: f64, wy: f64) void {
    const cx = toCx(wx);
    const cy = toCy(wy);
    ctx.setFillStyle("#ffffff");
    ctx.beginPath();
    ctx.arc(cx, cy, 4.0, 0.0, TWO_PI);
    ctx.fill(null);
    ctx.setStrokeStyle("#111111");
    ctx.setLineWidth(1.6);
    ctx.beginPath();
    ctx.arc(cx, cy, 4.0, 0.0, TWO_PI);
    ctx.stroke();
}

fn buildScene(app: *App) void {
    const scope = rp.JsScope.open();
    defer scope.deinit();

    if (app.world) |w| {
        for (&app.chain) |c| c.deinit();
        app.arm.deinit();
        app.bob.deinit();
        w.free();
        w.deinit();
    }

    const w = rp.World.new(rp.Vector3.new(0.0, -9.81, 0.0)).retain();
    app.world = w;

    const gantry = w.createRigidBody(rp.RigidBodyDesc.fixed().setTranslation(0.0, 6.2, 0.0));
    _ = w.createColliderWith(rp.ColliderDesc.cuboid(6.0, 0.22, 0.6), .{ .parent = gantry });
    const z_axis = rp.Vector3.new(0.0, 0.0, 1.0);

    const cx = [3]f64{ -2.8, -3.3, -3.8 };
    const cy = [3]f64{ 5.4, 4.1, 2.8 };
    for (0..3) |i| {
        app.chain[i] = w.createRigidBody(rp.RigidBodyDesc.dynamic().setTranslation(cx[i], cy[i], 0.0)).retain();
        _ = w.createColliderWith(rp.ColliderDesc.cuboid(LINK_HX, LINK_HY, 0.2).setDensity(@as(f64, if (i == 2) 1.6 else 1.4)), .{ .parent = app.chain[i] });
    }

    _ = w.createImpulseJoint(rp.JointData.revolute(rp.Vector3.new(-2.2, -0.22, 0.0), rp.Vector3.new(0.0, LINK_HY + 0.06, 0.0), z_axis), gantry, app.chain[0], true);

    const b1 = rp.Vector3.new(0.0, -LINK_HY - 0.06, 0.0);
    const b2 = rp.Vector3.new(0.0, LINK_HY + 0.06, 0.0);
    _ = w.createImpulseJoint(rp.JointData.revolute(b1, b2, z_axis), app.chain[0], app.chain[1], true);
    _ = w.createImpulseJoint(rp.JointData.revolute(b1, b2, z_axis), app.chain[1], app.chain[2], true);

    app.chain[2].applyImpulse(rp.Vector3.new(-1.4, 0.0, 0.0), true);

    app.arm = w.createRigidBody(rp.RigidBodyDesc.dynamic().setTranslation(3.1, 5.2, 0.0)).retain();
    _ = w.createColliderWith(rp.ColliderDesc.cuboid(0.14, 0.85, 0.2).setDensity(1.2), .{ .parent = app.arm });
    app.bob = w.createRigidBody(rp.RigidBodyDesc.dynamic().setTranslation(3.8, 3.6, 0.0)).retain();
    _ = w.createColliderWith(rp.ColliderDesc.ball(0.42).setDensity(3.0), .{ .parent = app.bob });

    _ = w.createImpulseJoint(rp.JointData.revolute(rp.Vector3.new(2.3, -0.22, 0.0), rp.Vector3.new(0.0, 0.88, 0.0), z_axis), gantry, app.arm, true);
    _ = w.createImpulseJoint(rp.JointData.revolute(rp.Vector3.new(0.0, -0.88, 0.0), rp.Vector3.new(0.0, 0.72, 0.0), z_axis), app.arm, app.bob, true);

    app.bob.applyImpulse(rp.Vector3.new(1.8, 0.2, 0.0), true);

    app.step_count = 0;
    app.trail_count = 0;
}

fn stepSim(app: *App) void {
    const scope = rp.JsScope.open();
    defer scope.deinit();

    app.world.?.step(null);
    app.step_count += 1;
    const pa = app.chain[2].translation(null);
    const pb = app.bob.translation(null);
    const idx = (app.step_count - 1) % TRAIL_LEN;
    app.ax[idx] = pa.getX();
    app.ay[idx] = pa.getY();
    app.bx[idx] = pb.getX();
    app.by[idx] = pb.getY();
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
        ctx.lineTo(x, 400.0);
        ctx.stroke();
    }
    var gy: i32 = 0;
    while (gy <= 7) : (gy += 1) {
        const y = toCy(@floatFromInt(gy));
        ctx.beginPath();
        ctx.moveTo(0.0, y);
        ctx.lineTo(680.0, y);
        ctx.stroke();
    }

    drawRotBox(ctx, 0.0, 6.2, 0.0, 0.0, 0.0, 1.0, 6.0, 0.22, "#efefef");

    ctx.setFillStyle("#b5b5b5");
    for (0..app.trail_count) |i| {
        ctx.beginPath();
        ctx.arc(toCx(app.ax[i]), toCy(app.ay[i]), 1.6, 0.0, TWO_PI);
        ctx.fill(null);
        ctx.beginPath();
        ctx.arc(toCx(app.bx[i]), toCy(app.by[i]), 1.6, 0.0, TWO_PI);
        ctx.fill(null);
    }

    var pin_x: [2]f64 = undefined;
    var pin_y: [2]f64 = undefined;
    var tip_x: f64 = 0.0;
    var tip_y: f64 = 0.0;
    for (0..3) |i| {
        const p = app.chain[i].translation(null);
        const q = app.chain[i].rotation(null);
        const px = p.getX();
        const py = p.getY();
        const qx = q.getX();
        const qy = q.getY();
        const qz = q.getZ();
        const qw = q.getW();
        drawRotBox(ctx, px, py, qx, qy, qz, qw, LINK_HX, LINK_HY, if (i == 2) "#d4d4d4" else "#f7f7f7");
        if (i < 2) {
            const r01 = 2.0 * (qx * qy - qw * qz);
            const r11 = 1.0 - 2.0 * (qx * qx + qz * qz);
            pin_x[i] = px - r01 * (LINK_HY + 0.06);
            pin_y[i] = py - r11 * (LINK_HY + 0.06);
        } else {
            tip_x = px;
            tip_y = py;
        }
    }

    const ap = app.arm.translation(null);
    const aq = app.arm.rotation(null);
    const apx = ap.getX();
    const apy = ap.getY();
    const aqx = aq.getX();
    const aqy = aq.getY();
    const aqz = aq.getZ();
    const aqw = aq.getW();
    drawRotBox(ctx, apx, apy, aqx, aqy, aqz, aqw, 0.14, 0.85, "#f7f7f7");

    const bp = app.bob.translation(null);
    const bpx = bp.getX();
    const bpy = bp.getY();
    const bcx = toCx(bpx);
    const bcy = toCy(bpy);
    ctx.setFillStyle("#262626");
    ctx.beginPath();
    ctx.arc(bcx, bcy, 0.42 * 46.0, 0.0, TWO_PI);
    ctx.fill(null);
    ctx.setStrokeStyle("#111111");
    ctx.setLineWidth(1.6);
    ctx.beginPath();
    ctx.arc(bcx, bcy, 0.42 * 46.0, 0.0, TWO_PI);
    ctx.stroke();

    drawPivot(ctx, -2.2, 5.98);
    drawPivot(ctx, pin_x[0], pin_y[0]);
    drawPivot(ctx, pin_x[1], pin_y[1]);
    drawPivot(ctx, 2.3, 5.98);
    const arm_r01 = 2.0 * (aqx * aqy - aqw * aqz);
    const arm_r11 = 1.0 - 2.0 * (aqx * aqx + aqz * aqz);
    drawPivot(ctx, apx - arm_r01 * 0.88, apy - arm_r11 * 0.88);

    ctx.setFillStyle("#555555");
    ctx.setFont("11px ui-monospace, monospace");
    ctx.fillText("3-Link Revolute Chain", toCx(-3.5), 36.0);
    ctx.fillText("2-Body Bob Pendulum", toCx(1.1), 36.0);

    var mbuf: [256]u8 = undefined;
    const msg = std.fmt.bufPrint(
        &mbuf,
        "5 revolute joints · chain_tip=({d:.2}, {d:.2}) · bob=({d:.2}, {d:.2}) · step={d} ({s})",
        .{ tip_x, tip_y, bpx, bpy, app.step_count, if (app.paused) "paused" else "running" },
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
        app.chain[2].applyImpulse(rp.Vector3.new(1.6, 0.3, 0.0), true);
        app.bob.applyImpulse(rp.Vector3.new(-2.0, 0.4, 0.0), true);
        render(app);
    } else if (ctx.id == 1) {
        app.paused = !app.paused;
        app.pause_btn.setTextContent(if (app.paused) "Resume" else "Pause");
        render(app);
    } else {
        app.paused = false;
        app.pause_btn.setTextContent("Pause");
        buildScene(app);
        for (0..15) |_| stepSim(app);
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

    buildScene(app);
    for (0..20) |_| stepSim(app);
    render(app);

    const ids = [3][]const u8{ "btn-kick", "btn-pause", "btn-reset" };
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
