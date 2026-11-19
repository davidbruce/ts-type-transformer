const std = @import("std");
const dom = @import("dom");
const three = @import("three");

fn v3(x: f64, y: f64, z: f64) three.Vector3 {
    return three.Vector3.new(.{ .x = x, .y = y, .z = z });
}

fn addObj(parent: three.Object3D, child: three.Object3D) void {
    _ = parent.add(three.JsArrayOf(three.Object3D).of(&.{child}));
}

fn towerHeight(mode: usize, gx: usize, gz: usize) f64 {
    const fx = @as(f64, @floatFromInt(gx)) - 4.5;
    const fz = @as(f64, @floatFromInt(gz)) - 4.5;
    if (mode == 1) {
        const r = @sqrt(fx * fx + fz * fz);
        return 0.35 + 0.42 * r + 0.25 * @cos(fx * 0.8);
    } else if (mode == 2) {
        const diag = @abs(fx - fz);
        return 0.3 + 2.8 * @exp(-diag * diag / 5.5) + 0.25 * @sin(fz * 0.9);
    }
    const r2 = fx * fx + fz * fz;
    return 0.35 + 3.1 * @exp(-r2 / 9.5) + 0.2 * @cos(fx * 1.4) * @cos(fz * 1.4);
}

fn layoutName(mode: usize) []const u8 {
    if (mode == 1) return "Amphitheater";
    if (mode == 2) return "Diagonal";
    return "Plaza";
}

var g_renderer: three.WebGLRenderer = undefined;
var g_scene: three.Scene = undefined;
var g_ortho: three.OrthographicCamera = undefined;
var g_persp: three.PerspectiveCamera = undefined;
var g_inst: three.InstancedMesh = undefined;
var g_cam_btn: dom.HTMLElement = undefined;
var g_meta: dom.HTMLElement = undefined;
var g_use_ortho: bool = true;
var g_layout_mode: usize = 0;
var g_cam_yaw: f64 = 0.785398;

fn renderCity() void {
    const pos = v3(0.0, 0.0, 0.0);
    const rot_q = three.Quaternion.new(.{});
    const scale = v3(1.0, 1.0, 1.0);
    const mtx = three.Matrix4.new();
    const axis_y = v3(0.0, 1.0, 0.0);
    const col = three.Color.new(null);

    var max_h: f64 = 0.0;
    var idx: usize = 0;
    var gz: usize = 0;
    while (gz < 10) : (gz += 1) {
        var gx: usize = 0;
        while (gx < 10) : (gx += 1) {
            const x = (@as(f64, @floatFromInt(gx)) - 4.5) * 0.78;
            const z = (@as(f64, @floatFromInt(gz)) - 4.5) * 0.78;
            const h = towerHeight(g_layout_mode, gx, gz);
            if (h > max_h) max_h = h;
            _ = pos.set(x, h * 0.5, .{ .z = z });
            _ = rot_q.setFromAxisAngle(axis_y.asVector3Like(), 0.08 * @as(f64, @floatFromInt((gx + gz) % 4)));
            _ = scale.set(1.0, h, .{ .z = 1.0 });
            _ = mtx.compose(pos, rot_q, scale);
            _ = g_inst.setMatrixAt(@floatFromInt(idx), mtx);

            var t = (h - 0.3) / 3.2;
            if (t < 0.0) t = 0.0;
            if (t > 1.0) t = 1.0;
            const shade = 0.82 - 0.70 * t;
            _ = col.setRGB(shade, shade, shade, .{});
            _ = g_inst.setColorAt(@floatFromInt(idx), col);
            idx += 1;
        }
    }

    const imat = g_inst.getInstanceMatrix();
    imat.asBufferAttribute().setNeedsUpdate(true);
    if (g_inst.getInstanceColor()) |icol| {
        icol.asBufferAttribute().setNeedsUpdate(true);
    }

    const radius = 11.5;
    const cx = radius * @cos(g_cam_yaw);
    const cz = radius * @sin(g_cam_yaw);
    const target = v3(0.0, 0.9, 0.0);

    _ = g_ortho.asObject3D().getPosition().set(cx, 8.2, .{ .z = cz });
    g_ortho.asObject3D().lookAt(target);
    _ = g_persp.asObject3D().getPosition().set(cx, 7.6, .{ .z = cz });
    g_persp.asObject3D().lookAt(target);

    if (g_use_ortho) {
        g_renderer.render(g_scene.asObject3D(), g_ortho.asCamera());
    } else {
        g_renderer.render(g_scene.asObject3D(), g_persp.asCamera());
    }

    const cname = if (g_use_ortho) "Orthographic" else "Perspective";
    var cbuf: [64]u8 = undefined;
    const cmsg = std.fmt.bufPrint(&cbuf, "Camera: {s}", .{cname}) catch return;
    g_cam_btn.asNode().setTextContent(cmsg);

    var buf: [256]u8 = undefined;
    const msg = std.fmt.bufPrint(
        &buf,
        "100 InstancedMesh towers · layout: {s} · peak height: {d:.2} · camera: {s}",
        .{ layoutName(g_layout_mode), max_h, cname },
    ) catch return;
    g_meta.asNode().setTextContent(msg);
}

const BTN_TAGS = [5]usize{ 0, 1, 2, 3, 4 };
fn onBtn(ctx: ?*anyopaque, _: dom.PointerEvent) void {
    const id = @as(*const usize, @ptrCast(@alignCast(ctx.?))).*;
    if (id == 0) {
        g_use_ortho = !g_use_ortho;
    } else if (id >= 1 and id <= 3) {
        g_layout_mode = id - 1;
    } else {
        g_cam_yaw += std.math.pi / 4.0;
    }
    renderCity();
}

export fn start() void {
    const doc = dom.getDocument();
    const width: f64 = 680.0;
    const height: f64 = 400.0;
    const aspect = width / height;

    const rp = three.WebGLRendererParameters.create();
    rp.setAntialias(true);
    rp.setPreserveDrawingBuffer(true);
    g_renderer = three.WebGLRenderer.new(rp);
    g_renderer.setPixelRatio(1.0);
    g_renderer.setSize(width, height, .{});
    _ = doc.getElementById("stage").?.asNode().appendChild(.{ .h = g_renderer.getDomElement().h });

    g_scene = three.Scene.new();
    g_scene.setBackground(.{ .Color = three.Color.new(.{ .number = 0xf7f7f7 }) });
    g_ortho = three.OrthographicCamera.new(.{
        .left = -5.4 * aspect,
        .right = 5.4 * aspect,
        .top = 5.4,
        .bottom = -5.4,
        .near = 0.1,
        .far = 100.0,
    });
    g_persp = three.PerspectiveCamera.new(.{ .fov = 40.0, .aspect = aspect, .near = 0.1, .far = 100.0 });

    addObj(g_scene.asObject3D(), three.AmbientLight.new(.{ .color = .{ .number = 0xffffff }, .intensity = 1.25 }).asObject3D());
    const key = three.DirectionalLight.new(.{ .color = .{ .number = 0xffffff }, .intensity = 2.1 });
    _ = key.asObject3D().getPosition().set(8.0, 12.0, .{ .z = 6.0 });
    addObj(g_scene.asObject3D(), key.asObject3D());
    const rim = three.DirectionalLight.new(.{ .color = .{ .number = 0xffffff }, .intensity = 0.9 });
    _ = rim.asObject3D().getPosition().set(-7.0, 6.0, .{ .z = -8.0 });
    addObj(g_scene.asObject3D(), rim.asObject3D());

    const pmp = three.MeshStandardMaterialParameters.create();
    pmp.setColor(.{ .number = 0xe5e5e5 });
    pmp.setRoughness(0.85);
    const plinth = three.Mesh.new(.{
        .geometry = three.PlaneGeometry.new(.{ .width = 8.6, .height = 8.6 }).asBufferGeometry(),
        .material = .{ .Material = three.MeshStandardMaterial.new(pmp).asMaterial() },
    });
    plinth.asObject3D().getRotation().setX(-std.math.pi / 2.0);
    plinth.asObject3D().getPosition().setY(-0.01);
    addObj(g_scene.asObject3D(), plinth.asObject3D());

    const grid = three.GridHelper.new(.{
        .size = 9.0,
        .divisions = 10.0,
        .color1 = .{ .number = 0x999999 },
        .color2 = .{ .number = 0xd4d4d4 },
    });
    addObj(g_scene.asObject3D(), grid.asLineSegments().asLine().asObject3D());

    const tgeo = three.BoxGeometry.new(.{ .width = 0.54, .height = 1.0, .depth = 0.54 });
    const tmp = three.MeshStandardMaterialParameters.create();
    tmp.setColor(.{ .number = 0xffffff });
    tmp.setRoughness(0.65);
    tmp.setMetalness(0.08);
    g_inst = three.InstancedMesh.new(
        tgeo.asBufferGeometry(),
        .{ .Material = three.MeshStandardMaterial.new(tmp).asMaterial() },
        100.0,
    );
    addObj(g_scene.asObject3D(), g_inst.asMesh().asObject3D());

    g_cam_btn = doc.getElementById("btn-cam").?;
    g_meta = doc.getElementById("meta").?;

    const ids = [5][]const u8{ "btn-cam", "btn-lay0", "btn-lay1", "btn-lay2", "btn-rot" };
    for (ids, 0..) |id, i| {
        const b = doc.getElementById(id).?;
        b.addEventListener(.click, dom.JsListener(dom.PointerEvent).from(onBtn, @constCast(&BTN_TAGS[i])), .{});
    }
    renderCity();
}
