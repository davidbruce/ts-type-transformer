const std = @import("std");
const dom = @import("dom");
const rp = @import("rapier");

const TWO_PI: f64 = 2.0 * std.math.pi;
const RAY_COUNT: usize = 37;
const MAX_TOI: f64 = 16.0;
const EMITTER_X: f64 = -5.6;
const EMITTER_Y: f64 = 4.6;

fn toCx(wx: f64) f64 {
    return 340.0 + wx * 46.0;
}
fn toCy(wy: f64) f64 {
    return 350.0 - wy * 46.0;
}

const App = struct {
    ctx: dom.CanvasRenderingContext2D = undefined,
    meta: dom.HTMLElement = undefined,
    sweep_btn: dom.HTMLElement = undefined,
    raf_cb: dom.FrameRequestCallback = undefined,
    world: rp.World = undefined,
    btns: [3]BtnCtx = undefined,
    auto_sweep: bool = true,
    phase: f64 = 0.0,
    manual_offset: f64 = 0.0,
};

const BtnCtx = struct { app: *App, id: usize };

var g_app: App = .{};

fn drawBox(ctx: dom.CanvasRenderingContext2D, cx: f64, cy: f64, theta: f64, hx: f64, hy: f64, fill: []const u8) void {
    const c = @cos(theta);
    const s = @sin(theta);
    const xs = [4]f64{
        toCx(cx + c * (-hx) - s * (-hy)),
        toCx(cx + c * hx - s * (-hy)),
        toCx(cx + c * hx - s * hy),
        toCx(cx + c * (-hx) - s * hy),
    };
    const ys = [4]f64{
        toCy(cy + s * (-hx) + c * (-hy)),
        toCy(cy + s * hx + c * (-hy)),
        toCy(cy + s * hx + c * hy),
        toCy(cy + s * (-hx) + c * hy),
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

fn drawCircle(ctx: dom.CanvasRenderingContext2D, wx: f64, wy: f64, r: f64, fill: []const u8) void {
    const cx = toCx(wx);
    const cy = toCy(wy);
    const rp_px = r * 46.0;
    ctx.setFillStyle(fill);
    ctx.beginPath();
    ctx.arc(cx, cy, rp_px, 0.0, TWO_PI, null);
    ctx.fill(null);
    ctx.setStrokeStyle("#111111");
    ctx.setLineWidth(1.5);
    ctx.beginPath();
    ctx.arc(cx, cy, rp_px, 0.0, TWO_PI, null);
    ctx.stroke();
}

fn addBox(w: rp.World, cx: f64, cy: f64, theta: f64, hx: f64, hy: f64) void {
    const q = rp.Quaternion.new(0.0, 0.0, @sin(theta * 0.5), @cos(theta * 0.5));
    const desc = rp.RigidBodyDesc.fixed().setTranslation2(cx, cy, 0.0).setRotation2(q);
    const rb = w.createRigidBody(desc);
    _ = w.createCollider(rp.ColliderDesc.cuboid(hx, hy, 1.0), .{ .parent = rb });
}

fn addSphere(w: rp.World, cx: f64, cy: f64, r: f64) void {
    const rb = w.createRigidBody(rp.RigidBodyDesc.fixed().setTranslation2(cx, cy, 0.0));
    _ = w.createCollider(rp.ColliderDesc.ball(r), .{ .parent = rb });
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

    drawBox(ctx, 0.0, -0.35, 0.0, 7.0, 0.35, "#f7f7f7");
    drawBox(ctx, -1.6, 0.85, 0.0, 0.70, 0.85, "#efefef");
    drawBox(ctx, 2.2, 1.35, 0.0, 0.65, 1.35, "#efefef");
    drawBox(ctx, 0.2, 1.2, -0.42, 1.3, 0.18, "#d4d4d4");
    drawBox(ctx, 0.4, 4.4, 0.0, 1.9, 0.22, "#efefef");
    drawCircle(ctx, -2.5, 3.6, 0.65, "#d4d4d4");
    drawCircle(ctx, 4.2, 3.2, 0.75, "#d4d4d4");

    const base_angle = -0.35 + app.manual_offset + 0.28 * @sin(app.phase);
    const ox = toCx(EMITTER_X);
    const oy = toCy(EMITTER_Y);
    const origin = rp.Vector3.new(EMITTER_X, EMITTER_Y, 0.0);

    var hits: usize = 0;
    var min_toi: f64 = MAX_TOI;
    for (0..RAY_COUNT) |i| {
        const frac = @as(f64, @floatFromInt(i)) / @as(f64, @floatFromInt(RAY_COUNT - 1)) - 0.5;
        const angle = base_angle + frac * 1.35;
        const dx = @cos(angle);
        const dy = @sin(angle);
        const ray = rp.Ray.new(origin, rp.Vector3.new(dx, dy, 0.0));

        if (app.world.castRayAndGetNormal(ray, MAX_TOI, true, null)) |hit| {
            hits += 1;
            const toi = hit.getTimeOfImpact();
            if (toi < min_toi) min_toi = toi;
            const pt = ray.pointAt(toi);
            const ptx = pt.getX();
            const pty = pt.getY();
            const hx = toCx(ptx);
            const hy = toCy(pty);

            ctx.setStrokeStyle("#737373");
            ctx.setLineWidth(1.0);
            ctx.beginPath();
            ctx.moveTo(ox, oy);
            ctx.lineTo(hx, hy);
            ctx.stroke();

            const n = hit.getNormal();
            ctx.setStrokeStyle("#111111");
            ctx.setLineWidth(1.4);
            ctx.beginPath();
            ctx.moveTo(hx, hy);
            ctx.lineTo(toCx(ptx + n.getX() * 0.45), toCy(pty + n.getY() * 0.45));
            ctx.stroke();

            ctx.setFillStyle("#111111");
            ctx.beginPath();
            ctx.arc(hx, hy, 2.8, 0.0, TWO_PI, null);
            ctx.fill(null);
        } else {
            ctx.setStrokeStyle("#e5e5e5");
            ctx.setLineWidth(0.9);
            ctx.beginPath();
            ctx.moveTo(ox, oy);
            ctx.lineTo(toCx(EMITTER_X + dx * MAX_TOI), toCy(EMITTER_Y + dy * MAX_TOI));
            ctx.stroke();
        }
    }

    ctx.setFillStyle("#111111");
    ctx.beginPath();
    ctx.arc(ox, oy, 6.0, 0.0, TWO_PI, null);
    ctx.fill(null);
    ctx.setStrokeStyle("#ffffff");
    ctx.setLineWidth(1.5);
    ctx.beginPath();
    ctx.arc(ox, oy, 3.0, 0.0, TWO_PI, null);
    ctx.stroke();

    const deg: i32 = @intFromFloat(@round(base_angle * 180.0 / std.math.pi));
    var mbuf: [256]u8 = undefined;
    const msg = std.fmt.bufPrint(
        &mbuf,
        "37 rays · hits={d}/37 · nearest_toi={d:.2} m · center={d} deg ({s})",
        .{ hits, min_toi, deg, if (app.auto_sweep) "auto" else "manual" },
    ) catch return;
    app.meta.setTextContent(msg);
}

fn onRaf(app: *App, _: f64) void {
    if (app.auto_sweep) {
        app.phase += 0.025;
        render(app);
    }
    _ = dom.requestAnimationFrame(app.raf_cb);
}

fn onBtn(ctx: *const BtnCtx, _: dom.PointerEvent) void {
    const app = ctx.app;
    if (ctx.id == 0) {
        app.auto_sweep = !app.auto_sweep;
        app.sweep_btn.setTextContent(if (app.auto_sweep) "Sweep: Auto" else "Sweep: Manual");
        render(app);
    } else if (ctx.id == 1) {
        app.manual_offset += std.math.pi / 12.0;
        if (app.manual_offset > 0.8) app.manual_offset = -0.6;
        render(app);
    } else {
        app.auto_sweep = true;
        app.phase = 0.0;
        app.manual_offset = 0.0;
        app.sweep_btn.setTextContent("Sweep: Auto");
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
    app.sweep_btn = doc.getElementById("btn-sweep").?.retain();

    app.world = rp.World.new(rp.Vector3.new(0.0, 0.0, 0.0), null).retain();
    addBox(app.world, 0.0, -0.35, 0.0, 7.0, 0.35);
    addBox(app.world, -1.6, 0.85, 0.0, 0.70, 0.85);
    addBox(app.world, 2.2, 1.35, 0.0, 0.65, 1.35);
    addBox(app.world, 0.2, 1.2, -0.42, 1.3, 0.18);
    addBox(app.world, 0.4, 4.4, 0.0, 1.9, 0.22);
    addSphere(app.world, -2.5, 3.6, 0.65);
    addSphere(app.world, 4.2, 3.2, 0.75);
    app.world.step(null);

    render(app);

    const ids = [3][]const u8{ "btn-sweep", "btn-angle", "btn-reset" };
    for (ids, 0..) |id, i| {
        app.btns[i] = .{ .app = app, .id = i };
        doc.getElementById(id).?.addEventListener(.click, .{ onBtn, &app.btns[i] }, null);
    }
    app.raf_cb = dom.FrameRequestCallback.from(onRaf, app).retain();
    _ = dom.requestAnimationFrame(app.raf_cb);
}

export fn start() void {
    const scope = rp.JsScope.open();
    defer scope.deinit();
    rp.init().then(onInit, &g_app);
}
