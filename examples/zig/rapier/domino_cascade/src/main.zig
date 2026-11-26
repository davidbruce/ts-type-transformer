const std = @import("std");
const dom = @import("dom");
const rp = @import("rapier");

const TWO_PI: f64 = 2.0 * std.math.pi;
const DOMINO_COUNT = 9;
const DOM_HX: f64 = 0.13;
const DOM_HY: f64 = 0.85;
const TRAIL_LEN = 22;

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
    dominoes: [DOMINO_COUNT]rp.RigidBody = undefined,
    striker: rp.RigidBody = undefined,
    btns: [3]BtnCtx = undefined,
    trail_x: [TRAIL_LEN]f64 = std.mem.zeroes([TRAIL_LEN]f64),
    trail_y: [TRAIL_LEN]f64 = std.mem.zeroes([TRAIL_LEN]f64),
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

fn buildScene(app: *App) void {
    const scope = rp.JsScope.open();
    defer scope.deinit();

    if (app.world) |w| {
        for (&app.dominoes) |d| d.deinit();
        app.striker.deinit();
        w.free();
        w.deinit();
    }

    const w = rp.World.new(rp.Vector3.new(0.0, -9.81, 0.0)).retain();
    app.world = w;

    const floor = w.createRigidBody(rp.RigidBodyDesc.fixed().setTranslation(0.0, -0.35, 0.0));
    _ = w.createColliderWith(rp.ColliderDesc.cuboid(7.2, 0.35, 2.0).setFriction(0.8), .{ .parent = floor });

    for (0..DOMINO_COUNT) |i| {
        const x = -2.4 + @as(f64, @floatFromInt(i)) * 0.88;
        app.dominoes[i] = w.createRigidBody(rp.RigidBodyDesc.dynamic().setTranslation(x, DOM_HY, 0.0)).retain();
        const cd = rp.ColliderDesc.cuboid(DOM_HX, DOM_HY, 0.35).setFriction(0.55).setRestitution(0.15).setDensity(1.0);
        _ = w.createColliderWith(cd, .{ .parent = app.dominoes[i] });
    }

    app.striker = w.createRigidBody(rp.RigidBodyDesc.dynamic().setTranslation(-5.3, 1.05, 0.0)).retain();
    _ = w.createColliderWith(rp.ColliderDesc.ball(0.40).setDensity(2.8).setRestitution(0.45).setFriction(0.25), .{ .parent = app.striker });
    app.striker.applyImpulse(rp.Vector3.new(4.8, 0.2, 0.0), true);

    app.step_count = 0;
    app.trail_count = 0;
}

fn stepSim(app: *App) void {
    const scope = rp.JsScope.open();
    defer scope.deinit();

    app.world.?.step(null);
    app.step_count += 1;
    const sp = app.striker.translation(null);
    const idx = (app.step_count - 1) % TRAIL_LEN;
    app.trail_x[idx] = sp.getX();
    app.trail_y[idx] = sp.getY();
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
    ctx.setStrokeStyle("#111111");
    ctx.setLineWidth(1.5);
    ctx.beginPath();
    ctx.moveTo(0.0, 350.0);
    ctx.lineTo(680.0, 350.0);
    ctx.stroke();

    ctx.setFillStyle("#b5b5b5");
    for (0..app.trail_count) |i| {
        ctx.beginPath();
        ctx.arc(toCx(app.trail_x[i]), toCy(app.trail_y[i]), 2.0, 0.0, TWO_PI);
        ctx.fill(null);
    }

    var toppled: usize = 0;
    for (0..DOMINO_COUNT) |i| {
        const p = app.dominoes[i].translation(null);
        const q = app.dominoes[i].rotation(null);
        const qz = q.getZ();
        const tilted = @abs(qz) > 0.25;
        if (tilted) toppled += 1;
        drawRotBox(ctx, p.getX(), p.getY(), q.getX(), q.getY(), qz, q.getW(), DOM_HX, DOM_HY, if (tilted) "#d4d4d4" else "#f7f7f7");
    }

    const sp = app.striker.translation(null);
    const spx = sp.getX();
    const scx = toCx(spx);
    const scy = toCy(sp.getY());
    ctx.setFillStyle("#262626");
    ctx.beginPath();
    ctx.arc(scx, scy, 0.40 * 46.0, 0.0, TWO_PI);
    ctx.fill(null);
    ctx.setStrokeStyle("#111111");
    ctx.setLineWidth(1.6);
    ctx.beginPath();
    ctx.arc(scx, scy, 0.40 * 46.0, 0.0, TWO_PI);
    ctx.stroke();
    ctx.setFillStyle("#e5e5e5");
    ctx.beginPath();
    ctx.arc(scx - 5.0, scy - 5.0, 2.5, 0.0, TWO_PI);
    ctx.fill(null);

    var cbuf: [64]u8 = undefined;
    const cap = std.fmt.bufPrint(&cbuf, "Dominoes toppled: {d} / 9", .{toppled}) catch return;
    ctx.setFillStyle("#555555");
    ctx.setFont("11px ui-monospace, monospace");
    ctx.fillText(cap, 18.0, 24.0);

    var mbuf: [256]u8 = undefined;
    const msg = std.fmt.bufPrint(
        &mbuf,
        "9 cuboids + striker sphere · toppled={d}/9 · striker_x={d:.2} m · step={d} ({s})",
        .{ toppled, spx, app.step_count, if (app.paused) "paused" else "running" },
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
        app.striker.applyImpulse(rp.Vector3.new(3.5, 0.6, 0.0), true);
        render(app);
    } else if (ctx.id == 1) {
        app.paused = !app.paused;
        app.pause_btn.setTextContent(if (app.paused) "Resume" else "Pause");
        render(app);
    } else {
        app.paused = false;
        app.pause_btn.setTextContent("Pause");
        buildScene(app);
        for (0..12) |_| stepSim(app);
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
    for (0..22) |_| stepSim(app);
    render(app);

    const ids = [3][]const u8{ "btn-push", "btn-pause", "btn-reset" };
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
