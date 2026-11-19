const std = @import("std");
const dom = @import("dom");
const rp = @import("rapier");

const TWO_PI: f64 = 2.0 * std.math.pi;
const TRAIL_LEN = 20;
const RESTITUTIONS = [5]f64{ 0.00, 0.35, 0.60, 0.80, 0.95 };
const X_COORDS = [5]f64{ -4.2, -2.1, 0.0, 2.1, 4.2 };
const BALL_HEX = [5][]const u8{ "#efefef", "#d4d4d4", "#b5b5b5", "#737373", "#262626" };
const REST_LABELS = [5][]const u8{ "e=0.00", "e=0.35", "e=0.60", "e=0.80", "e=0.95" };

fn toCx(wx: f64) f64 { return 340.0 + wx * 46.0; }
fn toCy(wy: f64) f64 { return 350.0 - wy * 46.0; }

var g_ctx: dom.CanvasRenderingContext2D = undefined;
var g_meta: dom.HTMLElement = undefined;
var g_grav_btn: dom.HTMLElement = undefined;
var g_pause_btn: dom.HTMLElement = undefined;
var g_raf_cb: dom.FrameRequestCallback = undefined;

var g_c_white: dom.string_or_CanvasGradient_or_CanvasPattern = undefined;
var g_c_grid: dom.string_or_CanvasGradient_or_CanvasPattern = undefined;
var g_c_f7: dom.string_or_CanvasGradient_or_CanvasPattern = undefined;
var g_c_d4: dom.string_or_CanvasGradient_or_CanvasPattern = undefined;
var g_c_b5: dom.string_or_CanvasGradient_or_CanvasPattern = undefined;
var g_c_73: dom.string_or_CanvasGradient_or_CanvasPattern = undefined;
var g_c_11: dom.string_or_CanvasGradient_or_CanvasPattern = undefined;
var g_c_balls: [5]dom.string_or_CanvasGradient_or_CanvasPattern = undefined;

var g_world: ?rp.World = null;
var g_balls: [5]rp.RigidBody = undefined;
var g_trail_x: [5][TRAIL_LEN]f64 = std.mem.zeroes([5][TRAIL_LEN]f64);
var g_trail_y: [5][TRAIL_LEN]f64 = std.mem.zeroes([5][TRAIL_LEN]f64);
var g_trail_count: usize = 0;
var g_step_count: usize = 0;
var g_earth: bool = true;
var g_paused: bool = false;

fn mkColor(hex: []const u8) dom.string_or_CanvasGradient_or_CanvasPattern {
    return .{ .string = dom.JsString.from(hex) };
}

fn drawDashed(x1: f64, y1: f64, x2: f64, y2: f64, segs: usize) void {
    const dx = x2 - x1;
    const dy = y2 - y1;
    var i: usize = 0;
    while (i < segs) : (i += 2) {
        const t0 = @as(f64, @floatFromInt(i)) / @as(f64, @floatFromInt(segs));
        const t1 = @as(f64, @floatFromInt(i + 1)) / @as(f64, @floatFromInt(segs));
        g_ctx.beginPath();
        g_ctx.moveTo(x1 + dx * t0, y1 + dy * t0);
        g_ctx.lineTo(x1 + dx * t1, y1 + dy * t1);
        g_ctx.stroke();
    }
}

fn buildScene() void {
    if (g_world) |w| {
        w.free();
        rp.jsRelease(w);
    }
    const gv = rp.Vector3.new(0.0, if (g_earth) -9.81 else -1.62, 0.0);
    defer rp.jsRelease(gv);
    g_world = rp.World.new(gv.asVector(), .{});
    const w = g_world.?;

    const ground = w.createRigidBody(rp.RigidBodyDesc.fixed().setTranslation2(0.0, -0.4, 0.0));
    _ = w.createCollider(rp.ColliderDesc.cuboid(7.0, 0.4, 2.0), .{ .parent = ground });
    const lw = w.createRigidBody(rp.RigidBodyDesc.fixed().setTranslation2(-6.8, 3.5, 0.0));
    _ = w.createCollider(rp.ColliderDesc.cuboid(0.25, 4.0, 2.0), .{ .parent = lw });
    const rw = w.createRigidBody(rp.RigidBodyDesc.fixed().setTranslation2(6.8, 3.5, 0.0));
    _ = w.createCollider(rp.ColliderDesc.cuboid(0.25, 4.0, 2.0), .{ .parent = rw });

    for (0..5) |i| {
        g_balls[i] = w.createRigidBody(rp.RigidBodyDesc.dynamic().setTranslation2(X_COORDS[i], 5.6, 0.0));
        const cd = rp.ColliderDesc.ball(0.42).setRestitution2(RESTITUTIONS[i]).setFriction2(0.2).setDensity2(1.2);
        _ = w.createCollider(cd, .{ .parent = g_balls[i] });
    }
    g_step_count = 0;
    g_trail_count = 0;
}

fn stepSim() void {
    g_world.?.step(.{});
    g_step_count += 1;
    const idx = (g_step_count - 1) % TRAIL_LEN;
    for (0..5) |i| {
        const p = g_balls[i].translation(null);
        defer rp.jsRelease(p);
        g_trail_x[i][idx] = p.getX();
        g_trail_y[i][idx] = p.getY();
    }
    if (g_trail_count < TRAIL_LEN) g_trail_count += 1;
}

fn render() void {
    g_ctx.setFillStyle(g_c_white);
    g_ctx.fillRect(0.0, 0.0, 680.0, 400.0);

    g_ctx.setStrokeStyle(g_c_grid);
    g_ctx.setLineWidth(1.0);
    var gx: i32 = -6;
    while (gx <= 6) : (gx += 1) {
        const x = toCx(@floatFromInt(gx));
        g_ctx.beginPath();
        g_ctx.moveTo(x, 0.0);
        g_ctx.lineTo(x, 350.0);
        g_ctx.stroke();
    }
    var gy: i32 = 1;
    while (gy <= 7) : (gy += 1) {
        const y = toCy(@floatFromInt(gy));
        g_ctx.beginPath();
        g_ctx.moveTo(0.0, y);
        g_ctx.lineTo(680.0, y);
        g_ctx.stroke();
    }

    g_ctx.setFillStyle(g_c_f7);
    g_ctx.fillRect(0.0, 350.0, 680.0, 50.0);
    g_ctx.setStrokeStyle(g_c_d4);
    var hx: f64 = -20.0;
    while (hx < 700.0) : (hx += 18.0) {
        g_ctx.beginPath();
        g_ctx.moveTo(hx, 350.0);
        g_ctx.lineTo(hx - 24.0, 400.0);
        g_ctx.stroke();
    }
    g_ctx.setStrokeStyle(g_c_11);
    g_ctx.setLineWidth(1.5);
    g_ctx.beginPath();
    g_ctx.moveTo(0.0, 350.0);
    g_ctx.lineTo(680.0, 350.0);
    g_ctx.stroke();

    g_ctx.setStrokeStyle(g_c_d4);
    g_ctx.setLineWidth(1.0);
    drawDashed(toCx(-5.0), toCy(5.6), toCx(5.0), toCy(5.6), 40);
    for (0..5) |i| {
        const cx = toCx(X_COORDS[i]);
        drawDashed(cx, toCy(5.6), cx, 350.0, 20);
    }

    var energy: f64 = 0.0;
    var max_y: f64 = 0.0;
    g_ctx.setFont("11px ui-monospace, monospace");
    for (0..5) |i| {
        g_ctx.setFillStyle(g_c_b5);
        for (0..g_trail_count) |k| {
            g_ctx.beginPath();
            g_ctx.arc(toCx(g_trail_x[i][k]), toCy(g_trail_y[i][k]), 1.6, 0.0, TWO_PI, .{});
            g_ctx.fill(null);
        }

        const p = g_balls[i].translation(null);
        defer rp.jsRelease(p);
        const v = g_balls[i].linvel(null);
        defer rp.jsRelease(v);
        const px = p.getX();
        const py = p.getY();
        const vx = v.getX();
        const vy = v.getY();
        energy += 0.5 * (vx * vx + vy * vy);
        if (py > max_y) max_y = py;

        const cx = toCx(px);
        const cy = toCy(py);
        const rpx: f64 = 0.42 * 46.0;

        g_ctx.setFillStyle(g_c_balls[i]);
        g_ctx.beginPath();
        g_ctx.arc(cx, cy, rpx, 0.0, TWO_PI, .{});
        g_ctx.fill(null);

        g_ctx.setStrokeStyle(g_c_11);
        g_ctx.setLineWidth(1.6);
        g_ctx.beginPath();
        g_ctx.arc(cx, cy, rpx, 0.0, TWO_PI, .{});
        g_ctx.stroke();

        g_ctx.setFillStyle(if (i >= 3) g_c_d4 else g_c_73);
        g_ctx.beginPath();
        g_ctx.arc(cx - 5.0, cy - 5.0, 2.8, 0.0, TWO_PI, .{});
        g_ctx.fill(null);

        g_ctx.setFillStyle(g_c_11);
        g_ctx.fillText(REST_LABELS[i], cx - 22.0, 368.0, .{});
    }

    var buf: [256]u8 = undefined;
    const msg = std.fmt.bufPrint(
        &buf,
        "5 spheres · g={s} m/s² · max_y={d:.2} m · ke={d:.1} · step={d} ({s})",
        .{ if (g_earth) "Earth (-9.81)" else "Moon (-1.62)", max_y, energy, g_step_count, if (g_paused) "paused" else "running" },
    ) catch return;
    g_meta.asNode().setTextContent(msg);
}

fn onRaf(_: ?*anyopaque, _: f64) void {
    if (!g_paused) {
        stepSim();
        render();
    }
    _ = dom.requestAnimationFrame(g_raf_cb);
}

const BTN_TAGS = [4]usize{ 0, 1, 2, 3 };
fn onBtn(ctx: ?*anyopaque, _: dom.PointerEvent) void {
    const id = @as(*const usize, @ptrCast(@alignCast(ctx.?))).*;
    if (id == 0) {
        g_earth = !g_earth;
        const gv = rp.Vector3.new(0.0, if (g_earth) -9.81 else -1.62, 0.0);
        defer rp.jsRelease(gv);
        g_world.?.setGravity(gv.asVector());
        const imp = rp.Vector3.new(0.0, 0.25, 0.0);
        defer rp.jsRelease(imp);
        for (0..5) |i| g_balls[i].applyImpulse(imp.asVector(), true);
        g_grav_btn.asNode().setTextContent(if (g_earth) "Gravity: Earth" else "Gravity: Moon");
        render();
    } else if (id == 1) {
        for (0..5) |i| {
            const dx = (@as(f64, @floatFromInt(i)) - 2.0) * 0.28;
            const imp = rp.Vector3.new(dx, 3.4, 0.0);
            defer rp.jsRelease(imp);
            g_balls[i].applyImpulse(imp.asVector(), true);
        }
        render();
    } else if (id == 2) {
        g_paused = !g_paused;
        g_pause_btn.asNode().setTextContent(if (g_paused) "Resume" else "Pause");
        render();
    } else {
        g_paused = false;
        g_pause_btn.asNode().setTextContent("Pause");
        buildScene();
        for (0..18) |_| stepSim();
        render();
    }
}

fn onInit(_: ?*anyopaque, ok: bool, _: rp.JsValue) void {
    if (!ok) return;
    const doc = dom.getDocument();
    const canvas = doc.querySelector(.canvas).?;
    g_ctx = canvas.getContext2d(null).?;
    g_meta = doc.getElementById("meta").?;
    g_grav_btn = doc.getElementById("btn-gravity").?;
    g_pause_btn = doc.getElementById("btn-pause").?;

    g_c_white = mkColor("#ffffff");
    g_c_grid = mkColor("#efefef");
    g_c_f7 = mkColor("#f7f7f7");
    g_c_d4 = mkColor("#d4d4d4");
    g_c_b5 = mkColor("#b5b5b5");
    g_c_73 = mkColor("#737373");
    g_c_11 = mkColor("#111111");
    for (0..5) |i| g_c_balls[i] = mkColor(BALL_HEX[i]);

    buildScene();
    for (0..28) |_| stepSim();
    render();

    const ids = [4][]const u8{ "btn-gravity", "btn-kick", "btn-pause", "btn-reset" };
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
