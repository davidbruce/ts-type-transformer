const std = @import("std");
const dom = @import("dom");
const rp = @import("rapier");

const TWO_PI: f64 = 2.0 * std.math.pi;
const BODY_COUNT = 6;
const IS_BOX = [BODY_COUNT]bool{ false, false, true, false, true, false };

fn toCx(wx: f64) f64 { return 340.0 + wx * 46.0; }
fn toCy(wy: f64) f64 { return 350.0 - wy * 46.0; }

var g_ctx: dom.CanvasRenderingContext2D = undefined;
var g_meta: dom.HTMLElement = undefined;
var g_pause_btn: dom.HTMLElement = undefined;
var g_raf_cb: dom.FrameRequestCallback = undefined;
var g_col_cb: rp.EventQueue_drainCollisionEvents_f = undefined;

var g_c_white: dom.string_or_CanvasGradient_or_CanvasPattern = undefined;
var g_c_grid: dom.string_or_CanvasGradient_or_CanvasPattern = undefined;
var g_c_f7: dom.string_or_CanvasGradient_or_CanvasPattern = undefined;
var g_c_e5: dom.string_or_CanvasGradient_or_CanvasPattern = undefined;
var g_c_d4: dom.string_or_CanvasGradient_or_CanvasPattern = undefined;
var g_c_73: dom.string_or_CanvasGradient_or_CanvasPattern = undefined;
var g_c_55: dom.string_or_CanvasGradient_or_CanvasPattern = undefined;
var g_c_26: dom.string_or_CanvasGradient_or_CanvasPattern = undefined;
var g_c_11: dom.string_or_CanvasGradient_or_CanvasPattern = undefined;

var g_world: ?rp.World = null;
var g_queue: ?rp.EventQueue = null;
var g_bodies: [BODY_COUNT]rp.RigidBody = undefined;
var g_sensor_handle: f64 = 0.0;
var g_step_count: usize = 0;
var g_solid_contacts: usize = 0;
var g_sensor_crossings: usize = 0;
var g_active_in_sensor: usize = 0;
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
    g_ctx.beginPath(); g_ctx.moveTo(xs[0], ys[0]);
    for (1..4) |i| g_ctx.lineTo(xs[i], ys[i]);
    g_ctx.closePath(); g_ctx.fill(null);

    g_ctx.setStrokeStyle(g_c_11);
    g_ctx.setLineWidth(1.5);
    g_ctx.beginPath(); g_ctx.moveTo(xs[0], ys[0]);
    for (1..4) |i| g_ctx.lineTo(xs[i], ys[i]);
    g_ctx.closePath(); g_ctx.stroke();
}

fn drawSegs(x1: f64, y1: f64, x2: f64, y2: f64, n: usize) void {
    const dx = x2 - x1;
    const dy = y2 - y1;
    var i: usize = 0;
    while (i < n) : (i += 2) {
        const t0 = @as(f64, @floatFromInt(i)) / @as(f64, @floatFromInt(n));
        const t1 = @as(f64, @floatFromInt(i + 1)) / @as(f64, @floatFromInt(n));
        g_ctx.beginPath();
        g_ctx.moveTo(x1 + dx * t0, y1 + dy * t0);
        g_ctx.lineTo(x1 + dx * t1, y1 + dy * t1);
        g_ctx.stroke();
    }
}

fn addRamp(w: rp.World, cx: f64, cy: f64, hx: f64, hy: f64, theta: f64) void {
    const q = rp.Quaternion.new(0.0, 0.0, @sin(theta * 0.5), @cos(theta * 0.5));
    defer rp.jsRelease(q);
    const rb = w.createRigidBody(rp.RigidBodyDesc.fixed().setTranslation2(cx, cy, 0.0).setRotation2(q.asRotation()));
    _ = w.createCollider(rp.ColliderDesc.cuboid(hx, hy, 1.0).setFriction2(0.15).setRestitution2(0.35), .{ .parent = rb });
}

fn onCollision(_: ?*anyopaque, h1: f64, h2: f64, started: bool) void {
    if (h1 == g_sensor_handle or h2 == g_sensor_handle) {
        if (started) {
            g_sensor_crossings += 1;
            g_active_in_sensor += 1;
        } else if (g_active_in_sensor > 0) {
            g_active_in_sensor -= 1;
        }
    } else if (started) {
        g_solid_contacts += 1;
    }
}

fn buildScene() void {
    if (g_queue) |q| {
        q.free();
        rp.jsRelease(q);
    }
    if (g_world) |w| {
        w.free();
        rp.jsRelease(w);
    }
    const gv = rp.Vector3.new(0.0, -9.81, 0.0);
    defer rp.jsRelease(gv);
    g_world = rp.World.new(gv.asVector(), .{});
    g_queue = rp.EventQueue.new(true, .{});
    const w = g_world.?;

    addRamp(w, 0.0, -0.35, 6.8, 0.35, 0.0);
    addRamp(w, -5.4, 2.8, 0.25, 3.2, 0.0);
    addRamp(w, 5.4, 2.8, 0.25, 3.2, 0.0);
    addRamp(w, -2.1, 4.2, 2.2, 0.18, -0.36);
    addRamp(w, 1.8, 2.6, 2.2, 0.18, 0.36);

    const sbody = w.createRigidBody(rp.RigidBodyDesc.fixed().setTranslation2(0.0, 1.35, 0.0));
    const scd = rp.ColliderDesc.cuboid(1.85, 0.30, 1.2).setSensor(true).setActiveEvents2(.COLLISION_EVENTS);
    const sc = w.createCollider(scd, .{ .parent = sbody });
    g_sensor_handle = sc.getHandle();

    for (0..BODY_COUNT) |i| {
        const x = -3.7 + @as(f64, @floatFromInt(i)) * 0.52;
        const y = 5.4 + @as(f64, @floatFromInt(i % 3)) * 0.58;
        g_bodies[i] = w.createRigidBody(rp.RigidBodyDesc.dynamic().setTranslation2(x, y, 0.0));
        const base_cd = if (IS_BOX[i]) rp.ColliderDesc.cuboid(0.28, 0.28, 0.28) else rp.ColliderDesc.ball(0.32);
        const cd = base_cd.setRestitution2(0.35).setFriction2(0.25).setActiveEvents2(.COLLISION_EVENTS);
        _ = w.createCollider(cd, .{ .parent = g_bodies[i] });
    }

    g_step_count = 0;
    g_solid_contacts = 0;
    g_sensor_crossings = 0;
    g_active_in_sensor = 0;
}

fn stepSim() void {
    g_world.?.step(.{ .eventQueue = g_queue.? });
    g_step_count += 1;
    g_queue.?.drainCollisionEvents(g_col_cb);
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

    const sx = toCx(-1.85);
    const sy = toCy(1.65);
    const sw = 3.70 * 46.0;
    const sh = 0.60 * 46.0;
    if (g_active_in_sensor > 0) {
        g_ctx.setFillStyle(g_c_grid);
        g_ctx.fillRect(sx, sy, sw, sh);
        g_ctx.setStrokeStyle(g_c_11);
        g_ctx.setLineWidth(2.0);
        g_ctx.strokeRect(sx, sy, sw, sh);
    } else {
        g_ctx.setStrokeStyle(g_c_73);
        g_ctx.setLineWidth(1.4);
        drawSegs(sx, sy, sx + sw, sy, 14);
        drawSegs(sx + sw, sy, sx + sw, sy + sh, 4);
        drawSegs(sx + sw, sy + sh, sx, sy + sh, 14);
        drawSegs(sx, sy + sh, sx, sy, 4);
    }

    var gbuf: [64]u8 = undefined;
    const gcap = std.fmt.bufPrint(&gbuf, "SENSOR GATE ({d} inside)", .{g_active_in_sensor}) catch return;
    g_ctx.setFillStyle(g_c_55);
    g_ctx.setFont("11px ui-monospace, monospace");
    g_ctx.fillText(gcap, sx + 8.0, sy + 17.0, .{});

    drawRotBox(0.0, -0.35, 0.0, 0.0, 0.0, 1.0, 6.8, 0.35, g_c_f7);
    drawRotBox(-5.4, 2.8, 0.0, 0.0, 0.0, 1.0, 0.25, 3.2, g_c_grid);
    drawRotBox(5.4, 2.8, 0.0, 0.0, 0.0, 1.0, 0.25, 3.2, g_c_grid);
    drawRotBox(-2.1, 4.2, 0.0, 0.0, @sin(-0.18), @cos(-0.18), 2.2, 0.18, g_c_e5);
    drawRotBox(1.8, 2.6, 0.0, 0.0, @sin(0.18), @cos(0.18), 2.2, 0.18, g_c_e5);

    for (0..BODY_COUNT) |i| {
        const p = g_bodies[i].translation(null);
        defer rp.jsRelease(p);
        const q = g_bodies[i].rotation(null);
        defer rp.jsRelease(q);
        const px = p.getX();
        const py = p.getY();
        if (IS_BOX[i]) {
            drawRotBox(px, py, q.getX(), q.getY(), q.getZ(), q.getW(), 0.28, 0.28, g_c_d4);
        } else {
            const cx = toCx(px);
            const cy = toCy(py);
            g_ctx.setFillStyle(g_c_26);
            g_ctx.beginPath(); g_ctx.arc(cx, cy, 0.32 * 46.0, 0.0, TWO_PI, .{}); g_ctx.fill(null);
            g_ctx.setStrokeStyle(g_c_11);
            g_ctx.setLineWidth(1.5);
            g_ctx.beginPath(); g_ctx.arc(cx, cy, 0.32 * 46.0, 0.0, TWO_PI, .{}); g_ctx.stroke();
        }
    }

    var mbuf: [256]u8 = undefined;
    const msg = std.fmt.bufPrint(
        &mbuf,
        "EventQueue · solid_contacts={d} · sensor_crossings={d} · inside={d} · step={d} ({s})",
        .{ g_solid_contacts, g_sensor_crossings, g_active_in_sensor, g_step_count, if (g_paused) "paused" else "running" },
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
        const v = rp.Vector3.new(0.4, 0.0, 0.0);
        defer rp.jsRelease(v);
        for (0..BODY_COUNT) |i| {
            const x = -3.7 + @as(f64, @floatFromInt(i)) * 0.52;
            const y = 5.4 + @as(f64, @floatFromInt(i % 3)) * 0.58;
            const p = rp.Vector3.new(x, y, 0.0);
            defer rp.jsRelease(p);
            g_bodies[i].setTranslation(p.asVector(), true);
            g_bodies[i].setLinvel(v.asVector(), true);
        }
        render();
    } else if (id == 1) {
        g_paused = !g_paused;
        g_pause_btn.asNode().setTextContent(if (g_paused) "Resume" else "Pause");
        render();
    } else {
        g_paused = false;
        g_pause_btn.asNode().setTextContent("Pause");
        buildScene();
        for (0..25) |_| stepSim();
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
    g_col_cb = rp.EventQueue_drainCollisionEvents_f.from(onCollision, null);

    g_c_white = mkColor("#ffffff");
    g_c_grid = mkColor("#efefef");
    g_c_f7 = mkColor("#f7f7f7");
    g_c_e5 = mkColor("#e5e5e5");
    g_c_d4 = mkColor("#d4d4d4");
    g_c_73 = mkColor("#737373");
    g_c_55 = mkColor("#555555");
    g_c_26 = mkColor("#262626");
    g_c_11 = mkColor("#111111");

    buildScene();
    for (0..40) |_| stepSim();
    render();

    const ids = [3][]const u8{ "btn-drop", "btn-pause", "btn-reset" };
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
