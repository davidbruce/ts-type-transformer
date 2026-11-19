const std = @import("std");
const dom = @import("dom");
const rp = @import("rapier");

const TWO_PI: f64 = 2.0 * std.math.pi;
const RAY_COUNT: usize = 37;
const MAX_TOI: f64 = 16.0;
const EMITTER_X: f64 = -5.6;
const EMITTER_Y: f64 = 4.6;

fn toCx(wx: f64) f64 { return 340.0 + wx * 46.0; }
fn toCy(wy: f64) f64 { return 350.0 - wy * 46.0; }

var g_ctx: dom.CanvasRenderingContext2D = undefined;
var g_meta: dom.HTMLElement = undefined;
var g_sweep_btn: dom.HTMLElement = undefined;
var g_raf_cb: dom.FrameRequestCallback = undefined;
var g_world: rp.World = undefined;

var g_c_white: dom.string_or_CanvasGradient_or_CanvasPattern = undefined;
var g_c_grid: dom.string_or_CanvasGradient_or_CanvasPattern = undefined;
var g_c_f7: dom.string_or_CanvasGradient_or_CanvasPattern = undefined;
var g_c_e5: dom.string_or_CanvasGradient_or_CanvasPattern = undefined;
var g_c_d4: dom.string_or_CanvasGradient_or_CanvasPattern = undefined;
var g_c_73: dom.string_or_CanvasGradient_or_CanvasPattern = undefined;
var g_c_11: dom.string_or_CanvasGradient_or_CanvasPattern = undefined;

var g_auto_sweep: bool = true;
var g_phase: f64 = 0.0;
var g_manual_offset: f64 = 0.0;

fn mkColor(hex: []const u8) dom.string_or_CanvasGradient_or_CanvasPattern {
    return .{ .string = dom.JsString.from(hex) };
}

fn drawBox(cx: f64, cy: f64, theta: f64, hx: f64, hy: f64, fill: dom.string_or_CanvasGradient_or_CanvasPattern) void {
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
    g_ctx.setFillStyle(fill);
    g_ctx.beginPath(); g_ctx.moveTo(xs[0], ys[0]);
    for (1..4) |i| g_ctx.lineTo(xs[i], ys[i]);
    g_ctx.closePath(); g_ctx.fill(null);

    g_ctx.setStrokeStyle(g_c_11);
    g_ctx.setLineWidth(1.5);
    g_ctx.beginPath(); g_ctx.moveTo(xs[0], ys[0]);
    for (1..4) |i| g_ctx.lineTo(xs[i], ys[i]);
    g_ctx.closePath(); g_ctx.stroke();
}

fn drawCircle(wx: f64, wy: f64, r: f64, fill: dom.string_or_CanvasGradient_or_CanvasPattern) void {
    const cx = toCx(wx);
    const cy = toCy(wy);
    const rp_px = r * 46.0;
    g_ctx.setFillStyle(fill);
    g_ctx.beginPath(); g_ctx.arc(cx, cy, rp_px, 0.0, TWO_PI, .{}); g_ctx.fill(null);
    g_ctx.setStrokeStyle(g_c_11);
    g_ctx.setLineWidth(1.5);
    g_ctx.beginPath(); g_ctx.arc(cx, cy, rp_px, 0.0, TWO_PI, .{}); g_ctx.stroke();
}

fn addBox(cx: f64, cy: f64, theta: f64, hx: f64, hy: f64) void {
    const q = rp.Quaternion.new(0.0, 0.0, @sin(theta * 0.5), @cos(theta * 0.5));
    defer rp.jsRelease(q);
    const desc = rp.RigidBodyDesc.fixed().setTranslation2(cx, cy, 0.0).setRotation2(q.asRotation());
    const rb = g_world.createRigidBody(desc);
    _ = g_world.createCollider(rp.ColliderDesc.cuboid(hx, hy, 1.0), .{ .parent = rb });
}

fn addSphere(cx: f64, cy: f64, r: f64) void {
    const rb = g_world.createRigidBody(rp.RigidBodyDesc.fixed().setTranslation2(cx, cy, 0.0));
    _ = g_world.createCollider(rp.ColliderDesc.ball(r), .{ .parent = rb });
}

fn render() void {
    g_ctx.setFillStyle(g_c_white);
    g_ctx.fillRect(0.0, 0.0, 680.0, 400.0);

    g_ctx.setStrokeStyle(g_c_grid);
    g_ctx.setLineWidth(1.0);
    var gx: i32 = -6;
    while (gx <= 6) : (gx += 1) {
        const x = toCx(@floatFromInt(gx));
        g_ctx.beginPath(); g_ctx.moveTo(x, 0.0); g_ctx.lineTo(x, 350.0); g_ctx.stroke();
    }
    var gy: i32 = 1;
    while (gy <= 7) : (gy += 1) {
        const y = toCy(@floatFromInt(gy));
        g_ctx.beginPath(); g_ctx.moveTo(0.0, y); g_ctx.lineTo(680.0, y); g_ctx.stroke();
    }

    drawBox(0.0, -0.35, 0.0, 7.0, 0.35, g_c_f7);
    drawBox(-1.6, 0.85, 0.0, 0.70, 0.85, g_c_grid);
    drawBox(2.2, 1.35, 0.0, 0.65, 1.35, g_c_grid);
    drawBox(0.2, 1.2, -0.42, 1.3, 0.18, g_c_d4);
    drawBox(0.4, 4.4, 0.0, 1.9, 0.22, g_c_grid);
    drawCircle(-2.5, 3.6, 0.65, g_c_d4);
    drawCircle(4.2, 3.2, 0.75, g_c_d4);

    const base_angle = -0.35 + g_manual_offset + 0.28 * @sin(g_phase);
    const ox = toCx(EMITTER_X);
    const oy = toCy(EMITTER_Y);
    const origin = rp.Vector3.new(EMITTER_X, EMITTER_Y, 0.0);
    defer rp.jsRelease(origin);

    var hits: usize = 0;
    var min_toi: f64 = MAX_TOI;
    for (0..RAY_COUNT) |i| {
        const frac = @as(f64, @floatFromInt(i)) / @as(f64, @floatFromInt(RAY_COUNT - 1)) - 0.5;
        const angle = base_angle + frac * 1.35;
        const dx = @cos(angle);
        const dy = @sin(angle);
        const dir = rp.Vector3.new(dx, dy, 0.0);
        defer rp.jsRelease(dir);
        const ray = rp.Ray.new(origin.asVector(), dir.asVector());
        defer rp.jsRelease(ray);

        if (g_world.castRayAndGetNormal(ray, MAX_TOI, true, .{})) |hit| {
            defer rp.jsRelease(hit);
            hits += 1;
            const toi = hit.getTimeOfImpact();
            if (toi < min_toi) min_toi = toi;
            const pt = ray.pointAt(toi);
            defer rp.jsRelease(pt);
            const ptx = pt.getX();
            const pty = pt.getY();
            const hx = toCx(ptx);
            const hy = toCy(pty);

            g_ctx.setStrokeStyle(g_c_73);
            g_ctx.setLineWidth(1.0);
            g_ctx.beginPath(); g_ctx.moveTo(ox, oy); g_ctx.lineTo(hx, hy); g_ctx.stroke();

            const n = hit.getNormal();
            defer rp.jsRelease(n);
            g_ctx.setStrokeStyle(g_c_11);
            g_ctx.setLineWidth(1.4);
            g_ctx.beginPath();
            g_ctx.moveTo(hx, hy);
            g_ctx.lineTo(toCx(ptx + n.getX() * 0.45), toCy(pty + n.getY() * 0.45));
            g_ctx.stroke();

            g_ctx.setFillStyle(g_c_11);
            g_ctx.beginPath(); g_ctx.arc(hx, hy, 2.8, 0.0, TWO_PI, .{}); g_ctx.fill(null);
        } else {
            g_ctx.setStrokeStyle(g_c_e5);
            g_ctx.setLineWidth(0.9);
            g_ctx.beginPath();
            g_ctx.moveTo(ox, oy);
            g_ctx.lineTo(toCx(EMITTER_X + dx * MAX_TOI), toCy(EMITTER_Y + dy * MAX_TOI));
            g_ctx.stroke();
        }
    }

    g_ctx.setFillStyle(g_c_11);
    g_ctx.beginPath(); g_ctx.arc(ox, oy, 6.0, 0.0, TWO_PI, .{}); g_ctx.fill(null);
    g_ctx.setStrokeStyle(g_c_white);
    g_ctx.setLineWidth(1.5);
    g_ctx.beginPath(); g_ctx.arc(ox, oy, 3.0, 0.0, TWO_PI, .{}); g_ctx.stroke();

    const deg: i32 = @intFromFloat(@round(base_angle * 180.0 / std.math.pi));
    var mbuf: [256]u8 = undefined;
    const msg = std.fmt.bufPrint(
        &mbuf,
        "37 rays · hits={d}/37 · nearest_toi={d:.2} m · center={d} deg ({s})",
        .{ hits, min_toi, deg, if (g_auto_sweep) "auto" else "manual" },
    ) catch return;
    g_meta.asNode().setTextContent(msg);
}

fn onRaf(_: ?*anyopaque, _: f64) void {
    if (g_auto_sweep) {
        g_phase += 0.025;
        render();
    }
    _ = dom.requestAnimationFrame(g_raf_cb);
}

const BTN_TAGS = [3]usize{ 0, 1, 2 };
fn onBtn(ctx: ?*anyopaque, _: dom.PointerEvent) void {
    const id = @as(*const usize, @ptrCast(@alignCast(ctx.?))).*;
    if (id == 0) {
        g_auto_sweep = !g_auto_sweep;
        g_sweep_btn.asNode().setTextContent(if (g_auto_sweep) "Sweep: Auto" else "Sweep: Manual");
        render();
    } else if (id == 1) {
        g_manual_offset += std.math.pi / 12.0;
        if (g_manual_offset > 0.8) g_manual_offset = -0.6;
        render();
    } else {
        g_auto_sweep = true;
        g_phase = 0.0;
        g_manual_offset = 0.0;
        g_sweep_btn.asNode().setTextContent("Sweep: Auto");
        render();
    }
}

fn onInit(_: ?*anyopaque, ok: bool, _: rp.JsValue) void {
    if (!ok) return;
    const doc = dom.getDocument();
    const canvas = doc.querySelector(.canvas).?;
    g_ctx = canvas.getContext2d(null).?;
    g_meta = doc.getElementById("meta").?;
    g_sweep_btn = doc.getElementById("btn-sweep").?;

    g_c_white = mkColor("#ffffff");
    g_c_grid = mkColor("#efefef");
    g_c_f7 = mkColor("#f7f7f7");
    g_c_e5 = mkColor("#e5e5e5");
    g_c_d4 = mkColor("#d4d4d4");
    g_c_73 = mkColor("#737373");
    g_c_11 = mkColor("#111111");

    const gv = rp.Vector3.new(0.0, 0.0, 0.0);
    defer rp.jsRelease(gv);
    g_world = rp.World.new(gv.asVector(), .{});

    addBox(0.0, -0.35, 0.0, 7.0, 0.35);
    addBox(-1.6, 0.85, 0.0, 0.70, 0.85);
    addBox(2.2, 1.35, 0.0, 0.65, 1.35);
    addBox(0.2, 1.2, -0.42, 1.3, 0.18);
    addBox(0.4, 4.4, 0.0, 1.9, 0.22);
    addSphere(-2.5, 3.6, 0.65);
    addSphere(4.2, 3.2, 0.75);
    g_world.step(.{});

    render();

    const ids = [3][]const u8{ "btn-sweep", "btn-angle", "btn-reset" };
    for (ids, 0..) |id, i| {
        const b = doc.getElementById(id).?;
        b.addEventListener(.click, dom.JsListener(dom.PointerEvent).from(onBtn, @constCast(&BTN_TAGS[i])), .{});
    }
    g_raf_cb = dom.FrameRequestCallback.from(onRaf, null);
    _ = dom.requestAnimationFrame(g_raf_cb);
}

export fn start() void {
    const p = rp.init();
    defer rp.jsRelease(p);
    p.then(onInit, null);
}
