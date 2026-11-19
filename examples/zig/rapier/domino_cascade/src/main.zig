const std = @import("std");
const dom = @import("dom");
const rp = @import("rapier");

const TWO_PI: f64 = 2.0 * std.math.pi;
const DOMINO_COUNT = 9;
const DOM_HX: f64 = 0.13;
const DOM_HY: f64 = 0.85;
const TRAIL_LEN = 22;

fn toCx(wx: f64) f64 { return 340.0 + wx * 46.0; }
fn toCy(wy: f64) f64 { return 350.0 - wy * 46.0; }

var g_ctx: dom.CanvasRenderingContext2D = undefined;
var g_meta: dom.HTMLElement = undefined;
var g_pause_btn: dom.HTMLElement = undefined;
var g_raf_cb: dom.FrameRequestCallback = undefined;

var g_c_white: dom.string_or_CanvasGradient_or_CanvasPattern = undefined;
var g_c_grid: dom.string_or_CanvasGradient_or_CanvasPattern = undefined;
var g_c_f7: dom.string_or_CanvasGradient_or_CanvasPattern = undefined;
var g_c_e5: dom.string_or_CanvasGradient_or_CanvasPattern = undefined;
var g_c_d4: dom.string_or_CanvasGradient_or_CanvasPattern = undefined;
var g_c_b5: dom.string_or_CanvasGradient_or_CanvasPattern = undefined;
var g_c_55: dom.string_or_CanvasGradient_or_CanvasPattern = undefined;
var g_c_26: dom.string_or_CanvasGradient_or_CanvasPattern = undefined;
var g_c_11: dom.string_or_CanvasGradient_or_CanvasPattern = undefined;

var g_world: ?rp.World = null;
var g_dominoes: [DOMINO_COUNT]rp.RigidBody = undefined;
var g_striker: rp.RigidBody = undefined;
var g_trail_x: [TRAIL_LEN]f64 = std.mem.zeroes([TRAIL_LEN]f64);
var g_trail_y: [TRAIL_LEN]f64 = std.mem.zeroes([TRAIL_LEN]f64);
var g_trail_count: usize = 0;
var g_step_count: usize = 0;
var g_paused: bool = false;

fn mkColor(hex: []const u8) dom.string_or_CanvasGradient_or_CanvasPattern {
    return .{ .string = dom.JsString.from(hex) };
}

fn drawRotBox(cx: f64, cy: f64, qx: f64, qy: f64, qz: f64, qw: f64, hx: f64, hy: f64, fill: dom.string_or_CanvasGradient_or_CanvasPattern) void {
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

    g_ctx.setFillStyle(fill);
    g_ctx.beginPath();
    g_ctx.moveTo(xs[0], ys[0]);
    for (1..4) |i| g_ctx.lineTo(xs[i], ys[i]);
    g_ctx.closePath();
    g_ctx.fill(null);

    g_ctx.setStrokeStyle(g_c_11);
    g_ctx.setLineWidth(1.5);
    g_ctx.beginPath();
    g_ctx.moveTo(xs[0], ys[0]);
    for (1..4) |i| g_ctx.lineTo(xs[i], ys[i]);
    g_ctx.closePath();
    g_ctx.stroke();
}

fn buildScene() void {
    if (g_world) |w| {
        w.free();
        rp.jsRelease(w);
    }
    const gv = rp.Vector3.new(0.0, -9.81, 0.0);
    defer rp.jsRelease(gv);
    g_world = rp.World.new(gv.asVector(), .{});
    const w = g_world.?;

    const floor = w.createRigidBody(rp.RigidBodyDesc.fixed().setTranslation2(0.0, -0.35, 0.0));
    _ = w.createCollider(rp.ColliderDesc.cuboid(7.2, 0.35, 2.0).setFriction2(0.8), .{ .parent = floor });

    for (0..DOMINO_COUNT) |i| {
        const x = -2.4 + @as(f64, @floatFromInt(i)) * 0.88;
        g_dominoes[i] = w.createRigidBody(rp.RigidBodyDesc.dynamic().setTranslation2(x, DOM_HY, 0.0));
        const cd = rp.ColliderDesc.cuboid(DOM_HX, DOM_HY, 0.35).setFriction2(0.55).setRestitution2(0.15).setDensity2(1.0);
        _ = w.createCollider(cd, .{ .parent = g_dominoes[i] });
    }

    g_striker = w.createRigidBody(rp.RigidBodyDesc.dynamic().setTranslation2(-5.3, 1.05, 0.0));
    _ = w.createCollider(rp.ColliderDesc.ball(0.40).setDensity2(2.8).setRestitution2(0.45).setFriction2(0.25), .{ .parent = g_striker });
    const imp = rp.Vector3.new(4.8, 0.2, 0.0);
    defer rp.jsRelease(imp);
    g_striker.applyImpulse(imp.asVector(), true);

    g_step_count = 0;
    g_trail_count = 0;
}

fn stepSim() void {
    g_world.?.step(.{});
    g_step_count += 1;
    const sp = g_striker.translation(null);
    defer rp.jsRelease(sp);
    const idx = (g_step_count - 1) % TRAIL_LEN;
    g_trail_x[idx] = sp.getX();
    g_trail_y[idx] = sp.getY();
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
        g_ctx.beginPath(); g_ctx.moveTo(x, 0.0); g_ctx.lineTo(x, 350.0); g_ctx.stroke();
    }
    var gy: i32 = 1;
    while (gy <= 7) : (gy += 1) {
        const y = toCy(@floatFromInt(gy));
        g_ctx.beginPath(); g_ctx.moveTo(0.0, y); g_ctx.lineTo(680.0, y); g_ctx.stroke();
    }

    g_ctx.setFillStyle(g_c_f7);
    g_ctx.fillRect(0.0, 350.0, 680.0, 50.0);
    g_ctx.setStrokeStyle(g_c_11);
    g_ctx.setLineWidth(1.5);
    g_ctx.beginPath(); g_ctx.moveTo(0.0, 350.0); g_ctx.lineTo(680.0, 350.0); g_ctx.stroke();

    g_ctx.setFillStyle(g_c_b5);
    for (0..g_trail_count) |i| {
        g_ctx.beginPath();
        g_ctx.arc(toCx(g_trail_x[i]), toCy(g_trail_y[i]), 2.0, 0.0, TWO_PI, .{});
        g_ctx.fill(null);
    }

    var toppled: usize = 0;
    for (0..DOMINO_COUNT) |i| {
        const p = g_dominoes[i].translation(null);
        defer rp.jsRelease(p);
        const q = g_dominoes[i].rotation(null);
        defer rp.jsRelease(q);
        const qz = q.getZ();
        const tilted = @abs(qz) > 0.25;
        if (tilted) toppled += 1;
        drawRotBox(p.getX(), p.getY(), q.getX(), q.getY(), qz, q.getW(), DOM_HX, DOM_HY, if (tilted) g_c_d4 else g_c_f7);
    }

    const sp = g_striker.translation(null);
    defer rp.jsRelease(sp);
    const spx = sp.getX();
    const scx = toCx(spx);
    const scy = toCy(sp.getY());
    g_ctx.setFillStyle(g_c_26);
    g_ctx.beginPath(); g_ctx.arc(scx, scy, 0.40 * 46.0, 0.0, TWO_PI, .{}); g_ctx.fill(null);
    g_ctx.setStrokeStyle(g_c_11);
    g_ctx.setLineWidth(1.6);
    g_ctx.beginPath(); g_ctx.arc(scx, scy, 0.40 * 46.0, 0.0, TWO_PI, .{}); g_ctx.stroke();
    g_ctx.setFillStyle(g_c_e5);
    g_ctx.beginPath(); g_ctx.arc(scx - 5.0, scy - 5.0, 2.5, 0.0, TWO_PI, .{}); g_ctx.fill(null);

    var cbuf: [64]u8 = undefined;
    const cap = std.fmt.bufPrint(&cbuf, "Dominoes toppled: {d} / 9", .{toppled}) catch return;
    g_ctx.setFillStyle(g_c_55);
    g_ctx.setFont("11px ui-monospace, monospace");
    g_ctx.fillText(cap, 18.0, 24.0, .{});

    var mbuf: [256]u8 = undefined;
    const msg = std.fmt.bufPrint(
        &mbuf,
        "9 cuboids + striker sphere · toppled={d}/9 · striker_x={d:.2} m · step={d} ({s})",
        .{ toppled, spx, g_step_count, if (g_paused) "paused" else "running" },
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

const BTN_TAGS = [3]usize{ 0, 1, 2 };
fn onBtn(ctx: ?*anyopaque, _: dom.PointerEvent) void {
    const id = @as(*const usize, @ptrCast(@alignCast(ctx.?))).*;
    if (id == 0) {
        const imp = rp.Vector3.new(3.5, 0.6, 0.0);
        defer rp.jsRelease(imp);
        g_striker.applyImpulse(imp.asVector(), true);
        render();
    } else if (id == 1) {
        g_paused = !g_paused;
        g_pause_btn.asNode().setTextContent(if (g_paused) "Resume" else "Pause");
        render();
    } else {
        g_paused = false;
        g_pause_btn.asNode().setTextContent("Pause");
        buildScene();
        for (0..12) |_| stepSim();
        render();
    }
}

fn onInit(_: ?*anyopaque, ok: bool, _: rp.JsValue) void {
    if (!ok) return;
    const doc = dom.getDocument();
    const canvas = doc.querySelector(.canvas).?;
    g_ctx = canvas.getContext2d(null).?;
    g_meta = doc.getElementById("meta").?;
    g_pause_btn = doc.getElementById("btn-pause").?;

    g_c_white = mkColor("#ffffff");
    g_c_grid = mkColor("#efefef");
    g_c_f7 = mkColor("#f7f7f7");
    g_c_e5 = mkColor("#e5e5e5");
    g_c_d4 = mkColor("#d4d4d4");
    g_c_b5 = mkColor("#b5b5b5");
    g_c_55 = mkColor("#555555");
    g_c_26 = mkColor("#262626");
    g_c_11 = mkColor("#111111");

    buildScene();
    for (0..22) |_| stepSim();
    render();

    const ids = [3][]const u8{ "btn-push", "btn-pause", "btn-reset" };
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
