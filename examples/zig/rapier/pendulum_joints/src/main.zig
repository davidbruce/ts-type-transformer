const std = @import("std");
const dom = @import("dom");
const rp = @import("rapier");

const TWO_PI: f64 = 2.0 * std.math.pi;
const TRAIL_LEN = 36;
const LINK_HX: f64 = 0.14;
const LINK_HY: f64 = 0.62;

fn toCx(wx: f64) f64 { return 340.0 + wx * 46.0; }
fn toCy(wy: f64) f64 { return 350.0 - wy * 46.0; }

var g_ctx: dom.CanvasRenderingContext2D = undefined;
var g_meta: dom.HTMLElement = undefined;
var g_pause_btn: dom.HTMLElement = undefined;
var g_raf_cb: dom.FrameRequestCallback = undefined;

var g_c_white: dom.string_or_CanvasGradient_or_CanvasPattern = undefined;
var g_c_grid: dom.string_or_CanvasGradient_or_CanvasPattern = undefined;
var g_c_f7: dom.string_or_CanvasGradient_or_CanvasPattern = undefined;
var g_c_d4: dom.string_or_CanvasGradient_or_CanvasPattern = undefined;
var g_c_b5: dom.string_or_CanvasGradient_or_CanvasPattern = undefined;
var g_c_55: dom.string_or_CanvasGradient_or_CanvasPattern = undefined;
var g_c_26: dom.string_or_CanvasGradient_or_CanvasPattern = undefined;
var g_c_11: dom.string_or_CanvasGradient_or_CanvasPattern = undefined;

var g_world: ?rp.World = null;
var g_chain: [3]rp.RigidBody = undefined;
var g_arm: rp.RigidBody = undefined;
var g_bob: rp.RigidBody = undefined;
var g_ax: [TRAIL_LEN]f64 = std.mem.zeroes([TRAIL_LEN]f64);
var g_ay: [TRAIL_LEN]f64 = std.mem.zeroes([TRAIL_LEN]f64);
var g_bx: [TRAIL_LEN]f64 = std.mem.zeroes([TRAIL_LEN]f64);
var g_by: [TRAIL_LEN]f64 = std.mem.zeroes([TRAIL_LEN]f64);
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

fn drawPivot(wx: f64, wy: f64) void {
    const cx = toCx(wx);
    const cy = toCy(wy);
    g_ctx.setFillStyle(g_c_white);
    g_ctx.beginPath(); g_ctx.arc(cx, cy, 4.0, 0.0, TWO_PI, .{}); g_ctx.fill(null);
    g_ctx.setStrokeStyle(g_c_11);
    g_ctx.setLineWidth(1.6);
    g_ctx.beginPath(); g_ctx.arc(cx, cy, 4.0, 0.0, TWO_PI, .{}); g_ctx.stroke();
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

    const gantry = w.createRigidBody(rp.RigidBodyDesc.fixed().setTranslation2(0.0, 6.2, 0.0));
    _ = w.createCollider(rp.ColliderDesc.cuboid(6.0, 0.22, 0.6), .{ .parent = gantry });
    const z_axis = rp.Vector3.new(0.0, 0.0, 1.0);
    defer rp.jsRelease(z_axis);

    const cx = [3]f64{ -2.8, -3.3, -3.8 };
    const cy = [3]f64{ 5.4, 4.1, 2.8 };
    for (0..3) |i| {
        g_chain[i] = w.createRigidBody(rp.RigidBodyDesc.dynamic().setTranslation2(cx[i], cy[i], 0.0));
        _ = w.createCollider(rp.ColliderDesc.cuboid(LINK_HX, LINK_HY, 0.2).setDensity2(if (i == 2) 1.6 else 1.4), .{ .parent = g_chain[i] });
    }

    const a1 = rp.Vector3.new(-2.2, -0.22, 0.0);
    const a2 = rp.Vector3.new(0.0, LINK_HY + 0.06, 0.0);
    defer rp.jsRelease(a1);
    defer rp.jsRelease(a2);
    _ = w.createImpulseJoint(rp.JointData.revolute(a1.asVector(), a2.asVector(), z_axis.asVector()), gantry, g_chain[0], true);

    const b1 = rp.Vector3.new(0.0, -LINK_HY - 0.06, 0.0);
    const b2 = rp.Vector3.new(0.0, LINK_HY + 0.06, 0.0);
    defer rp.jsRelease(b1);
    defer rp.jsRelease(b2);
    _ = w.createImpulseJoint(rp.JointData.revolute(b1.asVector(), b2.asVector(), z_axis.asVector()), g_chain[0], g_chain[1], true);
    _ = w.createImpulseJoint(rp.JointData.revolute(b1.asVector(), b2.asVector(), z_axis.asVector()), g_chain[1], g_chain[2], true);

    const kick_a = rp.Vector3.new(-1.4, 0.0, 0.0);
    defer rp.jsRelease(kick_a);
    g_chain[2].applyImpulse(kick_a.asVector(), true);

    g_arm = w.createRigidBody(rp.RigidBodyDesc.dynamic().setTranslation2(3.1, 5.2, 0.0));
    _ = w.createCollider(rp.ColliderDesc.cuboid(0.14, 0.85, 0.2).setDensity2(1.2), .{ .parent = g_arm });
    g_bob = w.createRigidBody(rp.RigidBodyDesc.dynamic().setTranslation2(3.8, 3.6, 0.0));
    _ = w.createCollider(rp.ColliderDesc.ball(0.42).setDensity2(3.0), .{ .parent = g_bob });

    const c1 = rp.Vector3.new(2.3, -0.22, 0.0);
    const c2 = rp.Vector3.new(0.0, 0.88, 0.0);
    defer rp.jsRelease(c1);
    defer rp.jsRelease(c2);
    _ = w.createImpulseJoint(rp.JointData.revolute(c1.asVector(), c2.asVector(), z_axis.asVector()), gantry, g_arm, true);

    const d1 = rp.Vector3.new(0.0, -0.88, 0.0);
    const d2 = rp.Vector3.new(0.0, 0.72, 0.0);
    defer rp.jsRelease(d1);
    defer rp.jsRelease(d2);
    _ = w.createImpulseJoint(rp.JointData.revolute(d1.asVector(), d2.asVector(), z_axis.asVector()), g_arm, g_bob, true);

    const kick_b = rp.Vector3.new(1.8, 0.2, 0.0);
    defer rp.jsRelease(kick_b);
    g_bob.applyImpulse(kick_b.asVector(), true);

    g_step_count = 0;
    g_trail_count = 0;
}

fn stepSim() void {
    g_world.?.step(.{});
    g_step_count += 1;
    const pa = g_chain[2].translation(null);
    defer rp.jsRelease(pa);
    const pb = g_bob.translation(null);
    defer rp.jsRelease(pb);
    const idx = (g_step_count - 1) % TRAIL_LEN;
    g_ax[idx] = pa.getX();
    g_ay[idx] = pa.getY();
    g_bx[idx] = pb.getX();
    g_by[idx] = pb.getY();
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
        g_ctx.beginPath(); g_ctx.moveTo(x, 0.0); g_ctx.lineTo(x, 400.0); g_ctx.stroke();
    }
    var gy: i32 = 0;
    while (gy <= 7) : (gy += 1) {
        const y = toCy(@floatFromInt(gy));
        g_ctx.beginPath(); g_ctx.moveTo(0.0, y); g_ctx.lineTo(680.0, y); g_ctx.stroke();
    }

    drawRotBox(0.0, 6.2, 0.0, 0.0, 0.0, 1.0, 6.0, 0.22, g_c_grid);

    g_ctx.setFillStyle(g_c_b5);
    for (0..g_trail_count) |i| {
        g_ctx.beginPath(); g_ctx.arc(toCx(g_ax[i]), toCy(g_ay[i]), 1.6, 0.0, TWO_PI, .{}); g_ctx.fill(null);
        g_ctx.beginPath(); g_ctx.arc(toCx(g_bx[i]), toCy(g_by[i]), 1.6, 0.0, TWO_PI, .{}); g_ctx.fill(null);
    }

    var pin_x: [2]f64 = undefined;
    var pin_y: [2]f64 = undefined;
    var tip_x: f64 = 0.0;
    var tip_y: f64 = 0.0;
    for (0..3) |i| {
        const p = g_chain[i].translation(null);
        defer rp.jsRelease(p);
        const q = g_chain[i].rotation(null);
        defer rp.jsRelease(q);
        const px = p.getX();
        const py = p.getY();
        const qx = q.getX();
        const qy = q.getY();
        const qz = q.getZ();
        const qw = q.getW();
        drawRotBox(px, py, qx, qy, qz, qw, LINK_HX, LINK_HY, if (i == 2) g_c_d4 else g_c_f7);
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

    const ap = g_arm.translation(null);
    defer rp.jsRelease(ap);
    const aq = g_arm.rotation(null);
    defer rp.jsRelease(aq);
    const apx = ap.getX();
    const apy = ap.getY();
    const aqx = aq.getX();
    const aqy = aq.getY();
    const aqz = aq.getZ();
    const aqw = aq.getW();
    drawRotBox(apx, apy, aqx, aqy, aqz, aqw, 0.14, 0.85, g_c_f7);

    const bp = g_bob.translation(null);
    defer rp.jsRelease(bp);
    const bpx = bp.getX();
    const bpy = bp.getY();
    const bcx = toCx(bpx);
    const bcy = toCy(bpy);
    g_ctx.setFillStyle(g_c_26);
    g_ctx.beginPath(); g_ctx.arc(bcx, bcy, 0.42 * 46.0, 0.0, TWO_PI, .{}); g_ctx.fill(null);
    g_ctx.setStrokeStyle(g_c_11);
    g_ctx.setLineWidth(1.6);
    g_ctx.beginPath(); g_ctx.arc(bcx, bcy, 0.42 * 46.0, 0.0, TWO_PI, .{}); g_ctx.stroke();

    drawPivot(-2.2, 5.98);
    drawPivot(pin_x[0], pin_y[0]);
    drawPivot(pin_x[1], pin_y[1]);
    drawPivot(2.3, 5.98);
    const arm_r01 = 2.0 * (aqx * aqy - aqw * aqz);
    const arm_r11 = 1.0 - 2.0 * (aqx * aqx + aqz * aqz);
    drawPivot(apx - arm_r01 * 0.88, apy - arm_r11 * 0.88);

    g_ctx.setFillStyle(g_c_55);
    g_ctx.setFont("11px ui-monospace, monospace");
    g_ctx.fillText("3-Link Revolute Chain", toCx(-3.5), 36.0, .{});
    g_ctx.fillText("2-Body Bob Pendulum", toCx(1.1), 36.0, .{});

    var mbuf: [256]u8 = undefined;
    const msg = std.fmt.bufPrint(
        &mbuf,
        "5 revolute joints · chain_tip=({d:.2}, {d:.2}) · bob=({d:.2}, {d:.2}) · step={d} ({s})",
        .{ tip_x, tip_y, bpx, bpy, g_step_count, if (g_paused) "paused" else "running" },
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
        const k1 = rp.Vector3.new(1.6, 0.3, 0.0);
        const k2 = rp.Vector3.new(-2.0, 0.4, 0.0);
        defer rp.jsRelease(k1);
        defer rp.jsRelease(k2);
        g_chain[2].applyImpulse(k1.asVector(), true);
        g_bob.applyImpulse(k2.asVector(), true);
        render();
    } else if (id == 1) {
        g_paused = !g_paused;
        g_pause_btn.asNode().setTextContent(if (g_paused) "Resume" else "Pause");
        render();
    } else {
        g_paused = false;
        g_pause_btn.asNode().setTextContent("Pause");
        buildScene();
        for (0..15) |_| stepSim();
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
    g_c_d4 = mkColor("#d4d4d4");
    g_c_b5 = mkColor("#b5b5b5");
    g_c_55 = mkColor("#555555");
    g_c_26 = mkColor("#262626");
    g_c_11 = mkColor("#111111");

    buildScene();
    for (0..20) |_| stepSim();
    render();

    const ids = [3][]const u8{ "btn-kick", "btn-pause", "btn-reset" };
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
