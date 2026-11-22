const std = @import("std");
const dom = @import("dom");
const three = @import("three");

fn v3(x: f64, y: f64, z: f64) three.Vector3 {
    return three.Vector3.new(.{ .x = x, .y = y, .z = z });
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

const App = struct {
    renderer: three.WebGLRenderer = undefined,
    scene: three.Scene = undefined,
    ortho: three.OrthographicCamera = undefined,
    persp: three.PerspectiveCamera = undefined,
    inst: three.InstancedMesh = undefined,
    cam_btn: dom.HTMLElement = undefined,
    meta: dom.HTMLElement = undefined,
    btns: [5]BtnCtx = undefined,
    use_ortho: bool = true,
    layout_mode: usize = 0,
    cam_yaw: f64 = 0.785398,
};

const BtnCtx = struct { app: *App, id: usize };

var g_app: App = .{};

fn renderCity(app: *App) void {
    const scope = three.JsScope.open();
    defer scope.deinit();

    const pos = v3(0.0, 0.0, 0.0);
    const rot_q = three.Quaternion.new(null);
    const scale = v3(1.0, 1.0, 1.0);
    const mtx = three.Matrix4.new();
    const axis_y = v3(0.0, 1.0, 0.0);
    const col = three.Color.new(null);

    var max_h: f64 = 0.0;
    var idx: usize = 0;
    for (0..10) |gz| {
        for (0..10) |gx| {
            const x = (@as(f64, @floatFromInt(gx)) - 4.5) * 0.78;
            const z = (@as(f64, @floatFromInt(gz)) - 4.5) * 0.78;
            const h = towerHeight(app.layout_mode, gx, gz);
            if (h > max_h) max_h = h;
            _ = pos.set(x, h * 0.5, z);
            _ = rot_q.setFromAxisAngle(axis_y, 0.08 * @as(f64, @floatFromInt((gx + gz) % 4)));
            _ = scale.set(1.0, h, 1.0);
            _ = mtx.compose(pos, rot_q, scale);
            _ = app.inst.setMatrixAt(idx, mtx);

            var t = (h - 0.3) / 3.2;
            if (t < 0.0) t = 0.0;
            if (t > 1.0) t = 1.0;
            const shade = 0.82 - 0.70 * t;
            _ = col.setRGB(shade, shade, shade, null);
            _ = app.inst.setColorAt(idx, col);
            idx += 1;
        }
    }

    app.inst.getInstanceMatrix().setNeedsUpdate(true);
    if (app.inst.getInstanceColor()) |icol| {
        icol.setNeedsUpdate(true);
    }

    const radius = 11.5;
    const cx = radius * @cos(app.cam_yaw);
    const cz = radius * @sin(app.cam_yaw);
    const target = v3(0.0, 0.9, 0.0);

    _ = app.ortho.getPosition().set(cx, 8.2, cz);
    app.ortho.lookAt(target);
    _ = app.persp.getPosition().set(cx, 7.6, cz);
    app.persp.lookAt(target);

    if (app.use_ortho) {
        app.renderer.render(app.scene, app.ortho);
    } else {
        app.renderer.render(app.scene, app.persp);
    }

    const cname = if (app.use_ortho) "Orthographic" else "Perspective";
    var cbuf: [64]u8 = undefined;
    const cmsg = std.fmt.bufPrint(&cbuf, "Camera: {s}", .{cname}) catch return;
    app.cam_btn.setTextContent(cmsg);

    var buf: [256]u8 = undefined;
    const msg = std.fmt.bufPrint(
        &buf,
        "100 InstancedMesh towers · layout: {s} · peak height: {d:.2} · camera: {s}",
        .{ layoutName(app.layout_mode), max_h, cname },
    ) catch return;
    app.meta.setTextContent(msg);
}

fn onBtn(ctx: *const BtnCtx, _: dom.PointerEvent) void {
    const app = ctx.app;
    if (ctx.id == 0) {
        app.use_ortho = !app.use_ortho;
    } else if (ctx.id >= 1 and ctx.id <= 3) {
        app.layout_mode = ctx.id - 1;
    } else {
        app.cam_yaw += std.math.pi / 4.0;
    }
    renderCity(app);
}

export fn start() void {
    const scope = three.JsScope.open();
    defer scope.deinit();

    const doc = dom.getDocument();
    const width: f64 = 680.0;
    const height: f64 = 400.0;
    const aspect = width / height;

    g_app.renderer = three.WebGLRenderer.new(.{ .antialias = true, .preserveDrawingBuffer = true }).retain();
    g_app.renderer.setPixelRatio(1.0);
    g_app.renderer.setSize(width, height, null);
    _ = doc.getElementById("stage").?.appendChild(g_app.renderer.getDomElement());

    g_app.scene = three.Scene.new().retain();
    g_app.scene.setBackground(three.Color.new(0xf7f7f7));
    g_app.ortho = three.OrthographicCamera.new(.{
        .left = -5.4 * aspect,
        .right = 5.4 * aspect,
        .top = 5.4,
        .bottom = -5.4,
        .near = 0.1,
        .far = 100.0,
    }).retain();
    g_app.persp = three.PerspectiveCamera.new(.{ .fov = 40.0, .aspect = aspect, .near = 0.1, .far = 100.0 }).retain();

    const key = three.DirectionalLight.new(.{ .color = 0xffffff, .intensity = 2.1 });
    _ = key.getPosition().set(8.0, 12.0, 6.0);
    const rim = three.DirectionalLight.new(.{ .color = 0xffffff, .intensity = 0.9 });
    _ = rim.getPosition().set(-7.0, 6.0, -8.0);

    const plinth = three.Mesh.new(.{
        .geometry = three.PlaneGeometry.new(.{ .width = 8.6, .height = 8.6 }),
        .material = three.MeshStandardMaterial.new(.{ .color = 0xe5e5e5, .roughness = 0.85 }),
    });
    plinth.getRotation().setX(-std.math.pi / 2.0);
    plinth.getPosition().setY(-0.01);

    const grid = three.GridHelper.new(.{ .size = 9.0, .divisions = 10.0, .color1 = 0x999999, .color2 = 0xd4d4d4 });

    g_app.inst = three.InstancedMesh.new(
        three.BoxGeometry.new(.{ .width = 0.54, .height = 1.0, .depth = 0.54 }),
        three.MeshStandardMaterial.new(.{ .color = 0xffffff, .roughness = 0.65, .metalness = 0.08 }),
        100.0,
    ).retain();

    _ = g_app.scene.add(.{
        three.AmbientLight.new(.{ .color = 0xffffff, .intensity = 1.25 }),
        key,
        rim,
        plinth,
        grid,
        g_app.inst,
    });

    g_app.cam_btn = doc.getElementById("btn-cam").?.retain();
    g_app.meta = doc.getElementById("meta").?.retain();

    const ids = [5][]const u8{ "btn-cam", "btn-lay0", "btn-lay1", "btn-lay2", "btn-rot" };
    for (ids, 0..) |id, i| {
        g_app.btns[i] = .{ .app = &g_app, .id = i };
        doc.getElementById(id).?.addEventListener(.click, .{ onBtn, &g_app.btns[i] }, null);
    }
    renderCity(&g_app);
}
