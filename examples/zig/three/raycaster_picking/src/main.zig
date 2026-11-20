const std = @import("std");
const dom = @import("dom");
const three = @import("three");

fn v3(x: f64, y: f64, z: f64) three.Vector3 {
    return three.Vector3.new(.{ .x = x, .y = y, .z = z });
}

const NAMES = [9][]const u8{
    "Box",        "Cone",  "Cylinder",    "Sphere",      "Octahedron",
    "Dodecahedron", "Torus", "Icosahedron", "Tetrahedron",
};

const App = struct {
    renderer: three.WebGLRenderer = undefined,
    canvas: dom.HTMLCanvasElement = undefined,
    scene: three.Scene = undefined,
    camera: three.PerspectiveCamera = undefined,
    mats: [9]three.MeshStandardMaterial = undefined,
    meshes: [9]three.Mesh = undefined,
    mesh_arr: three.JsArrayOf(three.Object3D) = undefined,
    box: three.Box3 = undefined,
    helper: three.Box3Helper = undefined,
    raycaster: three.Raycaster = undefined,
    meta: dom.HTMLElement = undefined,
    btns: [5]BtnCtx = undefined,
    yaw: f64 = 0.45,
    selected: i32 = -1,
    hit_buf: [160]u8 = undefined,
    hit_len: usize = 4,
};

const BtnCtx = struct { app: *App, id: usize };

var g_app: App = .{};

fn setHitNone(app: *App) void {
    @memcpy(app.hit_buf[0..4], "none");
    app.hit_len = 4;
}

fn updateCamera(app: *App) void {
    const r = 8.4;
    _ = app.camera.getPosition().set(r * @sin(app.yaw), 5.1, r * @cos(app.yaw));
    app.camera.lookAt(v3(0.0, 0.0, 0.0));
    app.camera.updateMatrixWorld(true);
    app.scene.updateMatrixWorld(true);
}

fn renderScene(app: *App) void {
    const scope = three.JsScope.open();
    defer scope.deinit();

    for (0..9) |i| {
        const hex: f64 = if (@as(i32, @intCast(i)) == app.selected) 0x111111 else 0xd4d4d4;
        _ = app.mats[i].getColor().setHex(hex, null);
    }
    if (app.selected >= 0 and app.selected < 9) {
        _ = app.box.setFromObject(app.meshes[@intCast(app.selected)], .{ .precise = true });
        app.helper.setVisible(true);
    } else {
        app.helper.setVisible(false);
    }
    app.renderer.render(app.scene, app.camera);

    var buf: [256]u8 = undefined;
    const msg = std.fmt.bufPrint(
        &buf,
        "picked: {s} · 9 specimens · click canvas or buttons",
        .{app.hit_buf[0..app.hit_len]},
    ) catch return;
    app.meta.setTextContent(msg);
}

fn castAtNdc(app: *App, nx: f64, ny: f64) void {
    const scope = three.JsScope.open();
    defer scope.deinit();

    app.raycaster.setFromCamera(three.Vector2.new(.{ .x = nx, .y = ny }), app.camera);
    const hits = app.raycaster.intersectObjects(app.mesh_arr, .{ .recursive = false });
    if (hits.len() > 0) {
        const hit = hits.get(0);
        const hobj = hit.getObject();
        var found: i32 = -1;
        for (0..9) |i| {
            if (three.jsSame(app.meshes[i], hobj)) {
                found = @intCast(i);
                break;
            }
        }
        app.selected = found;
        if (found >= 0) {
            const idx: usize = @intCast(found);
            _ = app.box.setFromObject(app.meshes[idx], .{ .precise = true });
            const sz = app.box.getSize(v3(0.0, 0.0, 0.0));
            const pt = hit.getPoint();
            const written = std.fmt.bufPrint(
                &app.hit_buf,
                "{s} at d={d:.2} ({d:.2}, {d:.2}, {d:.2}) box={d:.2}x{d:.2}x{d:.2}",
                .{
                    NAMES[idx],
                    hit.getDistance(),
                    pt.getX(),
                    pt.getY(),
                    pt.getZ(),
                    sz.getX(),
                    sz.getY(),
                    sz.getZ(),
                },
            ) catch "";
            app.hit_len = written.len;
        }
    } else {
        app.selected = -1;
        const written = std.fmt.bufPrint(&app.hit_buf, "miss at ndc ({d:.2}, {d:.2})", .{ nx, ny }) catch "";
        app.hit_len = written.len;
    }
    renderScene(app);
}

fn pickSpecimen(app: *App, idx: usize) void {
    const scope = three.JsScope.open();
    defer scope.deinit();

    updateCamera(app);
    const proj = app.meshes[idx].getWorldPosition(v3(0.0, 0.0, 0.0)).project(app.camera);
    castAtNdc(app, proj.getX(), proj.getY());
}

fn onCanvasClick(app: *App, ev: dom.PointerEvent) void {
    const scope = three.JsScope.open();
    defer scope.deinit();

    const rect = app.canvas.getBoundingClientRect();
    const nx = ((ev.getClientX() - rect.getLeft()) / rect.getWidth()) * 2.0 - 1.0;
    const ny = -((ev.getClientY() - rect.getTop()) / rect.getHeight()) * 2.0 + 1.0;
    updateCamera(app);
    castAtNdc(app, nx, ny);
}

fn onBtn(ctx: *const BtnCtx, _: dom.PointerEvent) void {
    const app = ctx.app;
    if (ctx.id == 0) {
        pickSpecimen(app, 4);
    } else if (ctx.id == 1) {
        pickSpecimen(app, 0);
    } else if (ctx.id == 2) {
        pickSpecimen(app, 8);
    } else if (ctx.id == 3) {
        const scope = three.JsScope.open();
        defer scope.deinit();
        app.yaw += std.math.pi / 4.0;
        updateCamera(app);
        renderScene(app);
    } else {
        const scope = three.JsScope.open();
        defer scope.deinit();
        app.selected = -1;
        setHitNone(app);
        updateCamera(app);
        renderScene(app);
    }
}

export fn start() void {
    const scope = three.JsScope.open();
    defer scope.deinit();

    setHitNone(&g_app);
    const doc = dom.getDocument();
    const width: f64 = 680.0;
    const height: f64 = 400.0;

    g_app.renderer = three.WebGLRenderer.new(.{ .antialias = true, .preserveDrawingBuffer = true }).retain();
    g_app.renderer.setPixelRatio(1.0);
    g_app.renderer.setSize(width, height, null);
    g_app.canvas = g_app.renderer.getDomElement().retain();
    _ = doc.getElementById("stage").?.appendChild(g_app.canvas);

    g_app.scene = three.Scene.new().retain();
    g_app.scene.setBackground(three.Color.new(0xf7f7f7));
    g_app.camera = three.PerspectiveCamera.new(.{ .fov = 42.0, .aspect = width / height, .near = 0.1, .far = 100.0 }).retain();

    const key = three.DirectionalLight.new(.{ .color = 0xffffff, .intensity = 2.2 });
    _ = key.getPosition().set(6.0, 10.0, 8.0);
    const rim = three.DirectionalLight.new(.{ .color = 0xffffff, .intensity = 0.9 });
    _ = rim.getPosition().set(-7.0, 5.0, -6.0);

    const grid = three.GridHelper.new(.{ .size = 9.0, .divisions = 9.0, .color1 = 0x999999, .color2 = 0xd4d4d4 });
    grid.getPosition().setY(-0.85);
    _ = g_app.scene.add(.{
        three.AmbientLight.new(.{ .color = 0xffffff, .intensity = 1.2 }),
        key,
        rim,
        grid,
    });

    const geos = [9]three.BufferGeometry{
        three.BoxGeometry.new(.{ .width = 1.05, .height = 1.05, .depth = 1.05 }).as(three.BufferGeometry),
        three.ConeGeometry.new(.{ .radius = 0.62, .height = 1.25, .radialSegments = 20.0 }).as(three.BufferGeometry),
        three.CylinderGeometry.new(.{ .radiusTop = 0.48, .radiusBottom = 0.48, .height = 1.2, .radialSegments = 24.0 }).as(three.BufferGeometry),
        three.SphereGeometry.new(.{ .radius = 0.62, .widthSegments = 24.0, .heightSegments = 18.0 }).as(three.BufferGeometry),
        three.OctahedronGeometry.new(.{ .radius = 0.68, .detail = 0.0 }).as(three.BufferGeometry),
        three.DodecahedronGeometry.new(.{ .radius = 0.62, .detail = 0.0 }).as(three.BufferGeometry),
        three.TorusGeometry.new(.{ .radius = 0.5, .tube = 0.18, .radialSegments = 14.0, .tubularSegments = 36.0 }).as(three.BufferGeometry),
        three.IcosahedronGeometry.new(.{ .radius = 0.64, .detail = 0.0 }).as(three.BufferGeometry),
        three.TetrahedronGeometry.new(.{ .radius = 0.72, .detail = 0.0 }).as(three.BufferGeometry),
    };

    g_app.mesh_arr = three.JsArrayOf(three.Object3D).new().retain();
    for (0..9) |i| {
        const gx: i32 = @as(i32, @intCast(i % 3)) - 1;
        const gz: i32 = @as(i32, @intCast(i / 3)) - 1;
        g_app.mats[i] = three.MeshStandardMaterial.new(.{
            .color = 0xd4d4d4,
            .roughness = 0.55,
            .metalness = 0.1,
            .flatShading = i >= 4,
        }).retain();

        const m = three.Mesh.new(.{ .geometry = geos[i], .material = g_app.mats[i] }).retain();
        m.setName(NAMES[i]);
        _ = m.getPosition().set(@as(f64, @floatFromInt(gx)) * 2.1, 0.0, @as(f64, @floatFromInt(gz)) * 2.1);
        m.getRotation().setX(0.2);
        m.getRotation().setY(0.35 * @as(f64, @floatFromInt(i)));
        _ = g_app.scene.add(.{m});
        g_app.meshes[i] = m;
        g_app.mesh_arr.push(m);
    }

    g_app.box = three.Box3.new(.{ .min = v3(-0.5, -0.5, -0.5), .max = v3(0.5, 0.5, 0.5) }).retain();
    g_app.helper = three.Box3Helper.new(g_app.box, .{ .color = 0x111111 }).retain();
    g_app.helper.setVisible(false);
    _ = g_app.scene.add(.{g_app.helper});

    g_app.raycaster = three.Raycaster.new(.{ .origin = v3(0.0, 0.0, 0.0), .direction = v3(0.0, 0.0, -1.0) }).retain();
    g_app.meta = doc.getElementById("meta").?.retain();

    g_app.canvas.addEventListener(.click, .{ onCanvasClick, &g_app }, null);

    const ids = [5][]const u8{ "btn-p0", "btn-p1", "btn-p2", "btn-orbit", "btn-clear" };
    for (ids, 0..) |id, i| {
        g_app.btns[i] = .{ .app = &g_app, .id = i };
        doc.getElementById(id).?.addEventListener(.click, .{ onBtn, &g_app.btns[i] }, null);
    }
    pickSpecimen(&g_app, 4);
}
