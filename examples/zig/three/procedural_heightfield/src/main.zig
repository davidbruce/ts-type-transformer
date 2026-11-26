const std = @import("std");
const dom = @import("dom");
const three = @import("three");

const GRID_N: usize = 28;

fn v3(x: f64, y: f64, z: f64) three.Vector3 {
    return three.Vector3.new(.{ .x = x, .y = y, .z = z });
}

fn waveHeight(mode: usize, x: f64, z: f64) f64 {
    if (mode == 1) {
        const r = @sqrt(x * x + z * z);
        return 0.78 * @cos(r * 2.2) / (1.0 + 0.18 * r * r);
    } else if (mode == 2) {
        return 0.065 * (x * x - z * z) + 0.18 * @sin(x * 1.6);
    }
    return 0.55 * @cos(x * 1.4) * @sin(z * 1.4) + 0.25 * @cos((x + z) * 1.1);
}

fn waveName(mode: usize) []const u8 {
    if (mode == 1) return "Radial Ripple";
    if (mode == 2) return "Saddle Ridge";
    return "Interference";
}

const App = struct {
    renderer: three.WebGLRenderer = undefined,
    scene: three.Scene = undefined,
    camera: three.PerspectiveCamera = undefined,
    geo: three.BufferGeometry = undefined,
    pos_attr: three.Float32BufferAttribute = undefined,
    pts_attr: three.Float32BufferAttribute = undefined,
    wire_mesh: three.Mesh = undefined,
    wire_btn: dom.HTMLElement = undefined,
    meta: dom.HTMLElement = undefined,
    btns: [5]BtnCtx = undefined,
    tri_count: usize = 0,
    wave_mode: usize = 0,
    overlay_on: bool = true,
    azimuth: f64 = 0.62,
};

const BtnCtx = struct { app: *App, id: usize };

var g_app: App = .{};

fn rebuildField(app: *App) void {
    const scope = three.JsScope.open();
    defer scope.deinit();

    const span: f64 = 7.6;
    const half: f64 = span / 2.0;
    const denom: f64 = @floatFromInt(GRID_N - 1);

    for (0..GRID_N) |iz| {
        for (0..GRID_N) |ix| {
            const idx = iz * GRID_N + ix;
            const x = -half + span * @as(f64, @floatFromInt(ix)) / denom;
            const z = -half + span * @as(f64, @floatFromInt(iz)) / denom;
            _ = app.pos_attr.setXYZ(idx, x, waveHeight(app.wave_mode, x, z), z);
        }
    }

    var pi: usize = 0;
    var iz: usize = 0;
    while (iz < GRID_N) : (iz += 2) {
        var ix: usize = 0;
        while (ix < GRID_N) : (ix += 2) {
            const x = -half + span * @as(f64, @floatFromInt(ix)) / denom;
            const z = -half + span * @as(f64, @floatFromInt(iz)) / denom;
            _ = app.pts_attr.setXYZ(pi, x, waveHeight(app.wave_mode, x, z) + 0.06, z);
            pi += 1;
        }
    }

    app.pos_attr.setNeedsUpdate(true);
    app.pts_attr.setNeedsUpdate(true);
    app.geo.computeVertexNormals();
    app.geo.computeBoundingBox();

    const r = 9.2;
    _ = app.camera.getPosition().setWith(r * @sin(app.azimuth), 5.4, r * @cos(app.azimuth));
    app.camera.lookAt(v3(0.0, 0.0, 0.0));

    app.wire_mesh.setVisible(app.overlay_on);
    app.wire_btn.setTextContent(if (app.overlay_on) "Wireframe: On" else "Wireframe: Off");
    app.renderer.render(app.scene, app.camera);

    const bb = app.geo.getBoundingBox().?;
    var buf: [256]u8 = undefined;
    const msg = std.fmt.bufPrint(
        &buf,
        "profile: {s} · {d} vertices · {d} triangles · y=[{d:.2}, {d:.2}] · wireframe: {s}",
        .{
            waveName(app.wave_mode),
            GRID_N * GRID_N,
            app.tri_count,
            bb.getMin().getY(),
            bb.getMax().getY(),
            if (app.overlay_on) "on" else "off",
        },
    ) catch return;
    app.meta.setTextContent(msg);
}

fn onBtn(ctx: *const BtnCtx, _: dom.PointerEvent) void {
    const app = ctx.app;
    if (ctx.id < 3) {
        app.wave_mode = ctx.id;
    } else if (ctx.id == 3) {
        app.overlay_on = !app.overlay_on;
    } else {
        app.azimuth += std.math.pi / 4.0;
    }
    rebuildField(app);
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

    const key = three.DirectionalLight.new(.{ .color = 0xffffff, .intensity = 2.2 });
    _ = key.getPosition().setWith(6.0, 10.0, 8.0);
    const rim = three.DirectionalLight.new(.{ .color = 0xffffff, .intensity = 0.9 });
    _ = rim.getPosition().setWith(-7.0, 5.0, -6.0);

    const grid = three.GridHelper.new(.{ .size = 9.0, .divisions = 12.0, .color1 = 0x999999, .color2 = 0xe5e5e5 });
    _ = grid.getPosition().setY(-1.1);

    g_app.geo = three.BufferGeometry.new().retain();
    g_app.pos_attr = three.Float32BufferAttribute.new(GRID_N * GRID_N * 3, 3.0).retain();

    const indices = three.JsArrayOf(f64).new();
    for (0..GRID_N - 1) |iz| {
        for (0..GRID_N - 1) |ix| {
            const a: f64 = @floatFromInt(iz * GRID_N + ix);
            const b = a + 1.0;
            const c = a + @as(f64, @floatFromInt(GRID_N));
            const d = c + 1.0;
            indices.push(a);
            indices.push(c);
            indices.push(b);
            indices.push(b);
            indices.push(c);
            indices.push(d);
        }
    }
    g_app.tri_count = (GRID_N - 1) * (GRID_N - 1) * 2;
    _ = g_app.geo.setAttribute("position", g_app.pos_attr);
    _ = g_app.geo.setIndex(indices);

    const surf = three.Mesh.new(.{
        .geometry = g_app.geo,
        .material = three.MeshStandardMaterial.new(.{ .color = 0xb5b5b5, .roughness = 0.72, .metalness = 0.05, .flatShading = true }),
    });

    g_app.wire_mesh = three.Mesh.new(.{
        .geometry = g_app.geo,
        .material = three.MeshBasicMaterial.new(.{ .color = 0x262626, .wireframe = true }),
    }).retain();
    _ = g_app.wire_mesh.getPosition().setY(0.015);

    const pt_count = (GRID_N / 2) * (GRID_N / 2);
    const pts_geo = three.BufferGeometry.new();
    g_app.pts_attr = three.Float32BufferAttribute.new(pt_count * 3, 3.0).retain();
    _ = pts_geo.setAttribute("position", g_app.pts_attr);
    const pts = three.Points.new(.{
        .geometry = pts_geo,
        .material = three.PointsMaterial.new(.{ .color = 0x111111, .size = 0.14 }),
    });

    _ = g_app.scene.add(.{
        three.AmbientLight.new(.{ .color = 0xffffff, .intensity = 1.2 }),
        key,
        rim,
        grid,
        surf,
        g_app.wire_mesh,
        pts,
    });

    g_app.wire_btn = doc.getElementById("btn-wire").?.retain();
    g_app.meta = doc.getElementById("meta").?.retain();

    const ids = [5][]const u8{ "btn-p0", "btn-p1", "btn-p2", "btn-wire", "btn-orbit" };
    for (ids, 0..) |id, i| {
        g_app.btns[i] = .{ .app = &g_app, .id = i };
        doc.getElementById(id).?.addEventListener(.click, .{ onBtn, &g_app.btns[i] });
    }
    rebuildField(&g_app);
}
