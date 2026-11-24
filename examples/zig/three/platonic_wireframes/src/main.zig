const std = @import("std");
const dom = @import("dom");
const three = @import("three");

fn v3(x: f64, y: f64, z: f64) three.Vector3 {
    return three.Vector3.new(.{ .x = x, .y = y, .z = z });
}

const App = struct {
    renderer: three.WebGLRenderer = undefined,
    scene: three.Scene = undefined,
    camera: three.PerspectiveCamera = undefined,
    wire_group: three.Group = undefined,
    wire_btn: dom.HTMLElement = undefined,
    meta: dom.HTMLElement = undefined,
    btns: [4]BtnCtx = undefined,
    wire_visible: bool = true,
    azimuth: f64 = 0.35,
};

const BtnCtx = struct { app: *App, id: usize };

var g_app: App = .{};

fn addSolid(
    app: *App,
    geo: three.BufferGeometry,
    hex: f64,
    rough: f64,
    metal: f64,
    flat: bool,
    x: f64,
    y: f64,
    z: f64,
    thresh: f64,
    line_mat: three.LineBasicMaterial,
) void {
    const mesh = three.Mesh.new(.{
        .geometry = geo,
        .material = three.MeshStandardMaterial.new(.{
            .color = hex,
            .roughness = rough,
            .metalness = metal,
            .flatShading = flat,
        }),
    });
    _ = mesh.getPosition().set(x, y, z);
    _ = mesh.getRotation().setX(0.25);
    _ = mesh.getRotation().setY(0.45);
    _ = app.scene.add(.{mesh});

    const segs = three.LineSegments.new(.{
        .geometry = three.EdgesGeometry.new(geo, .{ .thresholdAngle = thresh }),
        .material = line_mat,
    });
    _ = segs.getPosition().set(x, y, z);
    _ = segs.getRotation().setX(0.25);
    _ = segs.getRotation().setY(0.45);
    _ = app.wire_group.add(.{segs});
}

fn updateView(app: *App) void {
    const scope = three.JsScope.open();
    defer scope.deinit();

    const r = 8.6;
    _ = app.camera.getPosition().set(r * @sin(app.azimuth), 3.6, r * @cos(app.azimuth));
    app.camera.lookAt(v3(0.0, 0.0, 0.0));

    app.wire_group.setVisible(app.wire_visible);
    app.wire_btn.setTextContent(if (app.wire_visible) "Wireframes: On" else "Wireframes: Off");
    app.renderer.render(app.scene, app.camera);

    const deg: i32 = @intFromFloat(@round(app.azimuth * 180.0 / std.math.pi));
    var buf: [256]u8 = undefined;
    const msg = std.fmt.bufPrint(
        &buf,
        "5 monochrome solids · EdgesGeometry wireframes: {s} · azimuth: {d} deg · 680x400 WebGL",
        .{ if (app.wire_visible) "on" else "off", deg },
    ) catch return;
    app.meta.setTextContent(msg);
}

fn onBtn(ctx: *const BtnCtx, _: dom.PointerEvent) void {
    const app = ctx.app;
    if (ctx.id == 0) {
        app.wire_visible = !app.wire_visible;
    } else if (ctx.id == 1) {
        app.azimuth -= std.math.pi / 6.0;
    } else if (ctx.id == 2) {
        app.azimuth += std.math.pi / 6.0;
    } else {
        app.azimuth = 0.35;
        app.wire_visible = true;
    }
    updateView(app);
}

export fn start() void {
    const scope = three.JsScope.open();
    defer scope.deinit();

    const doc = dom.getDocument();
    const width: f64 = 680.0;
    const height: f64 = 400.0;

    g_app.renderer = three.WebGLRenderer.new(.{ .antialias = true, .preserveDrawingBuffer = true }).retain();
    g_app.renderer.setPixelRatio(1.0);
    g_app.renderer.setSize(width, height, null);
    _ = doc.getElementById("stage").?.appendChild(g_app.renderer.getDomElement());

    g_app.scene = three.Scene.new().retain();
    g_app.scene.setBackground(three.Color.new(0xf7f7f7));
    g_app.camera = three.PerspectiveCamera.new(.{ .fov = 42.0, .aspect = width / height, .near = 0.1, .far = 100.0 }).retain();

    const key = three.DirectionalLight.new(.{ .color = 0xffffff, .intensity = 2.2 });
    _ = key.getPosition().set(6.0, 10.0, 8.0);
    const rim = three.DirectionalLight.new(.{ .color = 0xffffff, .intensity = 0.9 });
    _ = rim.getPosition().set(-7.0, 5.0, -6.0);

    const grid = three.GridHelper.new(.{ .size = 12.0, .divisions = 12.0, .color1 = 0x737373, .color2 = 0xd4d4d4 });
    _ = grid.getPosition().setY(-1.2);

    g_app.wire_group = three.Group.new().retain();
    _ = g_app.scene.add(.{
        three.AmbientLight.new(.{ .color = 0xffffff, .intensity = 1.2 }),
        key,
        rim,
        grid,
        g_app.wire_group,
    });

    const line_mat = three.LineBasicMaterial.new(.{ .color = 0x111111 });
    addSolid(&g_app, three.BoxGeometry.new(.{ .width = 1.45, .height = 1.45, .depth = 1.45 }).as(three.BufferGeometry), 0x262626, 0.65, 0.1, false, -3.2, 0.0, -0.6, 1.0, line_mat);
    addSolid(&g_app, three.IcosahedronGeometry.new(.{ .radius = 0.95, .detail = 0.0 }).as(three.BufferGeometry), 0xd4d4d4, 0.55, 0.08, true, -1.55, 0.1, 0.55, 1.0, line_mat);
    addSolid(&g_app, three.TorusKnotGeometry.new(.{ .radius = 0.72, .tube = 0.24, .tubularSegments = 96.0, .radialSegments = 16.0 }).as(three.BufferGeometry), 0x737373, 0.45, 0.15, false, 0.0, 0.25, -0.2, 24.0, line_mat);
    addSolid(&g_app, three.OctahedronGeometry.new(.{ .radius = 0.95, .detail = 0.0 }).as(three.BufferGeometry), 0xefefef, 0.5, 0.05, true, 1.6, 0.1, 0.55, 1.0, line_mat);
    addSolid(&g_app, three.TorusGeometry.new(.{ .radius = 0.72, .tube = 0.23, .radialSegments = 16.0, .tubularSegments = 48.0 }).as(three.BufferGeometry), 0x404040, 0.55, 0.12, false, 3.2, 0.05, -0.5, 22.0, line_mat);

    g_app.wire_btn = doc.getElementById("btn-wire").?.retain();
    g_app.meta = doc.getElementById("meta").?.retain();

    const ids = [4][]const u8{ "btn-wire", "btn-left", "btn-right", "btn-reset" };
    for (ids, 0..) |id, i| {
        g_app.btns[i] = .{ .app = &g_app, .id = i };
        doc.getElementById(id).?.addEventListener(.click, .{ onBtn, &g_app.btns[i] }, null);
    }
    updateView(&g_app);
}
