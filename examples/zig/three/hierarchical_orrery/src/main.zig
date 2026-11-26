const std = @import("std");
const dom = @import("dom");
const three = @import("three");

fn v3(x: f64, y: f64, z: f64) three.Vector3 {
    return three.Vector3.new(.{ .x = x, .y = y, .z = z });
}

fn stdMat(hex: f64, rough: f64, metal: f64, flat: bool) three.MeshStandardMaterial {
    return three.MeshStandardMaterial.new(.{
        .color = hex,
        .roughness = rough,
        .metalness = metal,
        .flatShading = flat,
    });
}

const App = struct {
    renderer: three.WebGLRenderer = undefined,
    scene: three.Scene = undefined,
    camera: three.PerspectiveCamera = undefined,
    inner_pivot: three.Group = undefined,
    moon_pivot: three.Group = undefined,
    outer_pivot: three.Group = undefined,
    outer_station: three.Group = undefined,
    moon: three.Mesh = undefined,
    station_body: three.Mesh = undefined,
    tilt_btn: dom.HTMLElement = undefined,
    meta: dom.HTMLElement = undefined,
    btns: [3]BtnCtx = undefined,
    epoch: f64 = 0.6,
    tilted: bool = false,
};

const BtnCtx = struct { app: *App, id: usize };

var g_app: App = .{};

fn updateOrrery(app: *App) void {
    const scope = three.JsScope.open();
    defer scope.deinit();

    _ = app.inner_pivot.getRotation().setY(app.epoch);
    _ = app.moon_pivot.getRotation().setY(app.epoch * 2.5);
    _ = app.outer_pivot.getRotation().setY(-app.epoch * 0.55);
    _ = app.outer_pivot.getRotation().setZ(@as(f64, if (app.tilted) 0.38 else 0.0));
    _ = app.station_body.getRotation().setY(app.epoch * 1.4);

    app.scene.updateMatrixWorld(true);
    const mpos = app.moon.getWorldPosition(v3(0.0, 0.0, 0.0));
    const spos = app.outer_station.getWorldPosition(v3(0.0, 0.0, 0.0));

    app.tilt_btn.setTextContent(if (app.tilted) "Outer Tilt: On" else "Outer Tilt: Off");
    app.renderer.render(app.scene, app.camera);

    const deg: i32 = @intFromFloat(@round(app.epoch * 180.0 / std.math.pi));
    var buf: [256]u8 = undefined;
    const msg = std.fmt.bufPrint(
        &buf,
        "epoch: {d} deg · moon world: ({d:.2}, {d:.2}, {d:.2}) · station world: ({d:.2}, {d:.2}, {d:.2})",
        .{ deg, mpos.getX(), mpos.getY(), mpos.getZ(), spos.getX(), spos.getY(), spos.getZ() },
    ) catch return;
    app.meta.setTextContent(msg);
}

fn onBtn(ctx: *const BtnCtx, _: dom.PointerEvent) void {
    const app = ctx.app;
    if (ctx.id == 0) {
        app.epoch += std.math.pi / 6.0;
    } else if (ctx.id == 1) {
        app.tilted = !app.tilted;
    } else {
        app.epoch = 0.6;
        app.tilted = false;
    }
    updateOrrery(app);
}

export fn start() void {
    const scope = three.JsScope.open();
    defer scope.deinit();

    const doc = dom.getDocument();
    const width: f64 = 680.0;
    const height: f64 = 400.0;

    g_app.renderer = three.WebGLRenderer.new(.{ .antialias = true, .preserveDrawingBuffer = true }).retain();
    g_app.renderer.setPixelRatio(1.0);
    g_app.renderer.setSize(width, height);
    _ = doc.getElementById("stage").?.appendChild(g_app.renderer.getDomElement());

    g_app.scene = three.Scene.new().retain();
    g_app.scene.setBackground(three.Color.new(0xf7f7f7));
    g_app.camera = three.PerspectiveCamera.new(.{ .fov = 42.0, .aspect = width / height, .near = 0.1, .far = 100.0 }).retain();
    _ = g_app.camera.getPosition().setWith(5.6, 5.2, 7.6);
    g_app.camera.lookAt(v3(0.0, -0.1, 0.0));

    const key = three.DirectionalLight.new(.{ .color = 0xffffff, .intensity = 2.2 });
    _ = key.getPosition().setWith(6.0, 10.0, 8.0);
    const rim = three.DirectionalLight.new(.{ .color = 0xffffff, .intensity = 0.9 });
    _ = rim.getPosition().setWith(-7.0, 5.0, -6.0);

    const grid = three.GridHelper.new(.{ .size = 12.0, .divisions = 12.0, .color1 = 0x999999, .color2 = 0xe5e5e5 });
    _ = grid.getPosition().setY(-1.5);

    const pedestal = three.Mesh.new(.{
        .geometry = three.CylinderGeometry.new(.{ .radiusTop = 0.55, .radiusBottom = 0.75, .height = 0.35, .radialSegments = 24.0 }),
        .material = stdMat(0x262626, 0.7, 0.05, false),
    });
    _ = pedestal.getPosition().setY(-1.3);

    const core = three.Mesh.new(.{
        .geometry = three.SphereGeometry.new(.{ .radius = 0.65, .widthSegments = 24.0, .heightSegments = 18.0 }),
        .material = stdMat(0x111111, 0.45, 0.2, false),
    });

    g_app.inner_pivot = three.Group.new().retain();
    const inner_track = three.Mesh.new(.{
        .geometry = three.TorusGeometry.new(.{ .radius = 2.1, .tube = 0.025, .radialSegments = 8.0, .tubularSegments = 64.0 }),
        .material = three.MeshBasicMaterial.new(.{ .color = 0x999999 }),
    });
    _ = inner_track.getRotation().setX(std.math.pi / 2.0);

    const inner_planet = three.Group.new();
    _ = inner_planet.getPosition().setWith(2.1, 0.0, 0.0);

    const planet = three.Mesh.new(.{
        .geometry = three.SphereGeometry.new(.{ .radius = 0.36, .widthSegments = 20.0, .heightSegments = 16.0 }),
        .material = stdMat(0x555555, 0.55, 0.1, false),
    });
    g_app.moon_pivot = three.Group.new().retain();
    g_app.moon = three.Mesh.new(.{
        .geometry = three.OctahedronGeometry.new(.{ .radius = 0.18, .detail = 0.0 }),
        .material = stdMat(0xd4d4d4, 0.4, 0.1, true),
    }).retain();
    _ = g_app.moon.getPosition().setWith(0.75, 0.0, 0.0);
    _ = g_app.moon_pivot.add(.{g_app.moon});
    _ = inner_planet.add(.{ planet, g_app.moon_pivot });
    _ = g_app.inner_pivot.add(.{ inner_track, inner_planet });

    g_app.outer_pivot = three.Group.new().retain();
    const outer_track = three.Mesh.new(.{
        .geometry = three.TorusGeometry.new(.{ .radius = 3.8, .tube = 0.025, .radialSegments = 8.0, .tubularSegments = 80.0 }),
        .material = three.MeshBasicMaterial.new(.{ .color = 0xb5b5b5 }),
    });
    _ = outer_track.getRotation().setX(std.math.pi / 2.0);

    g_app.outer_station = three.Group.new().retain();
    _ = g_app.outer_station.getPosition().setWith(3.8, 0.0, 0.0);

    g_app.station_body = three.Mesh.new(.{
        .geometry = three.IcosahedronGeometry.new(.{ .radius = 0.42, .detail = 0.0 }),
        .material = stdMat(0x404040, 0.5, 0.1, true),
    }).retain();
    const halo = three.Mesh.new(.{
        .geometry = three.TorusGeometry.new(.{ .radius = 0.68, .tube = 0.06, .radialSegments = 10.0, .tubularSegments = 32.0 }),
        .material = stdMat(0x111111, 0.4, 0.1, false),
    });
    _ = halo.getRotation().setX(0.8);
    _ = g_app.outer_station.add(.{ g_app.station_body, halo });
    _ = g_app.outer_pivot.add(.{ outer_track, g_app.outer_station });

    _ = g_app.scene.add(.{
        three.AmbientLight.new(.{ .color = 0xffffff, .intensity = 1.2 }),
        key,
        rim,
        grid,
        pedestal,
        core,
        g_app.inner_pivot,
        g_app.outer_pivot,
    });

    g_app.tilt_btn = doc.getElementById("btn-tilt").?.retain();
    g_app.meta = doc.getElementById("meta").?.retain();

    const ids = [3][]const u8{ "btn-step", "btn-tilt", "btn-reset" };
    for (ids, 0..) |id, i| {
        g_app.btns[i] = .{ .app = &g_app, .id = i };
        doc.getElementById(id).?.addEventListener(.click, .{ onBtn, &g_app.btns[i] });
    }
    updateOrrery(&g_app);
}
