const std = @import("std");
const dom = @import("dom");
const three = @import("three");

fn v3(x: f64, y: f64, z: f64) three.Vector3 {
    return three.Vector3.new(.{ .x = x, .y = y, .z = z });
}

fn addObj(parent: three.Object3D, child: three.Object3D) void {
    _ = parent.add(three.JsArrayOf(three.Object3D).of(&.{child}));
}

const NAMES = [9][]const u8{
    "Box",        "Cone",  "Cylinder",    "Sphere",      "Octahedron",
    "Dodecahedron", "Torus", "Icosahedron", "Tetrahedron",
};

var g_renderer: three.WebGLRenderer = undefined;
var g_canvas: dom.HTMLCanvasElement = undefined;
var g_scene: three.Scene = undefined;
var g_camera: three.PerspectiveCamera = undefined;
var g_mats: [9]three.MeshStandardMaterial = undefined;
var g_meshes: [9]three.Object3D = undefined;
var g_mesh_arr: three.JsArrayOf(three.Object3D) = undefined;
var g_box: three.Box3 = undefined;
var g_helper: three.Box3Helper = undefined;
var g_raycaster: three.Raycaster = undefined;
var g_meta: dom.HTMLElement = undefined;
var g_yaw: f64 = 0.45;
var g_selected: i32 = -1;
var g_hit_buf: [160]u8 = undefined;
var g_hit_len: usize = 4;

fn setHitNone() void {
    @memcpy(g_hit_buf[0..4], "none");
    g_hit_len = 4;
}

fn updateCamera() void {
    const r = 8.4;
    _ = g_camera.asObject3D().getPosition().set(r * @sin(g_yaw), 5.1, .{ .z = r * @cos(g_yaw) });
    const orig = v3(0.0, 0.0, 0.0);
    g_camera.asObject3D().lookAt(orig);
    g_camera.asObject3D().updateMatrixWorld(true);
    g_scene.asObject3D().updateMatrixWorld(true);
}

fn renderScene() void {
    for (0..9) |i| {
        const hex: f64 = if (@as(i32, @intCast(i)) == g_selected) 0x111111 else 0xd4d4d4;
        const c = g_mats[i].asMeshStandardMaterialProperties().getColor();
        _ = c.setHex(hex, .{});
    }
    if (g_selected >= 0 and g_selected < 9) {
        _ = g_box.setFromObject(g_meshes[@intCast(g_selected)], .{ .precise = true });
        g_helper.asLineSegments().asLine().asObject3D().setVisible(true);
    } else {
        g_helper.asLineSegments().asLine().asObject3D().setVisible(false);
    }
    g_renderer.render(g_scene.asObject3D(), g_camera.asCamera());

    var buf: [256]u8 = undefined;
    const msg = std.fmt.bufPrint(
        &buf,
        "picked: {s} · 9 specimens · click canvas or buttons",
        .{g_hit_buf[0..g_hit_len]},
    ) catch return;
    g_meta.asNode().setTextContent(msg);
}

fn castAtNdc(nx: f64, ny: f64) void {
    const ndc = three.Vector2.new(.{ .x = nx, .y = ny });
    g_raycaster.setFromCamera(ndc, g_camera.asCamera());

    const hits = g_raycaster.intersectObjects(g_mesh_arr, .{ .recursive = false });
    if (hits.len() > 0) {
        const hit = hits.get(0);
        const hobj = hit.getObject();
        var found: i32 = -1;
        for (0..9) |i| {
            if (three.jsSame(g_meshes[i], hobj)) {
                found = @intCast(i);
                break;
            }
        }
        g_selected = found;
        if (found >= 0) {
            const idx: usize = @intCast(found);
            _ = g_box.setFromObject(g_meshes[idx], .{ .precise = true });
            const sz = g_box.getSize(v3(0.0, 0.0, 0.0));
            const pt = hit.getPoint();
            const written = std.fmt.bufPrint(
                &g_hit_buf,
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
            g_hit_len = written.len;
        }
    } else {
        g_selected = -1;
        const written = std.fmt.bufPrint(&g_hit_buf, "miss at ndc ({d:.2}, {d:.2})", .{ nx, ny }) catch "";
        g_hit_len = written.len;
    }
    renderScene();
}

fn pickSpecimen(idx: usize) void {
    updateCamera();
    const wp = g_meshes[idx].getWorldPosition(v3(0.0, 0.0, 0.0));
    const proj = wp.project(g_camera.asCamera());
    castAtNdc(proj.getX(), proj.getY());
}

fn onCanvasClick(_: ?*anyopaque, ev: dom.PointerEvent) void {
    const rect = g_canvas.asElement().getBoundingClientRect();
    const nx = ((ev.getClientX() - rect.asDOMRectReadOnly().getLeft()) / rect.asDOMRectReadOnly().getWidth()) * 2.0 - 1.0;
    const ny = -((ev.getClientY() - rect.asDOMRectReadOnly().getTop()) / rect.asDOMRectReadOnly().getHeight()) * 2.0 + 1.0;
    updateCamera();
    castAtNdc(nx, ny);
}

const BTN_TAGS = [5]usize{ 0, 1, 2, 3, 4 };
fn onBtn(ctx: ?*anyopaque, _: dom.PointerEvent) void {
    const id = @as(*const usize, @ptrCast(@alignCast(ctx.?))).*;
    if (id == 0) {
        pickSpecimen(4);
    } else if (id == 1) {
        pickSpecimen(0);
    } else if (id == 2) {
        pickSpecimen(8);
    } else if (id == 3) {
        g_yaw += std.math.pi / 4.0;
        updateCamera();
        renderScene();
    } else {
        g_selected = -1;
        setHitNone();
        updateCamera();
        renderScene();
    }
}

export fn start() void {
    setHitNone();
    const doc = dom.getDocument();
    const width: f64 = 680.0;
    const height: f64 = 400.0;

    const rp = three.WebGLRendererParameters.create();
    rp.setAntialias(true);
    rp.setPreserveDrawingBuffer(true);
    g_renderer = three.WebGLRenderer.new(rp);
    g_renderer.setPixelRatio(1.0);
    g_renderer.setSize(width, height, .{});
    g_canvas = .{ .h = g_renderer.getDomElement().h };
    _ = doc.getElementById("stage").?.asNode().appendChild(g_canvas.asNode());

    g_scene = three.Scene.new();
    g_scene.setBackground(.{ .Color = three.Color.new(.{ .number = 0xf7f7f7 }) });
    g_camera = three.PerspectiveCamera.new(.{ .fov = 42.0, .aspect = width / height, .near = 0.1, .far = 100.0 });

    addObj(g_scene.asObject3D(), three.AmbientLight.new(.{ .color = .{ .number = 0xffffff }, .intensity = 1.2 }).asObject3D());
    const key = three.DirectionalLight.new(.{ .color = .{ .number = 0xffffff }, .intensity = 2.2 });
    _ = key.asObject3D().getPosition().set(6.0, 10.0, .{ .z = 8.0 });
    addObj(g_scene.asObject3D(), key.asObject3D());
    const rim = three.DirectionalLight.new(.{ .color = .{ .number = 0xffffff }, .intensity = 0.9 });
    _ = rim.asObject3D().getPosition().set(-7.0, 5.0, .{ .z = -6.0 });
    addObj(g_scene.asObject3D(), rim.asObject3D());

    const grid = three.GridHelper.new(.{
        .size = 9.0,
        .divisions = 9.0,
        .color1 = .{ .number = 0x999999 },
        .color2 = .{ .number = 0xd4d4d4 },
    });
    grid.asLineSegments().asLine().asObject3D().getPosition().setY(-0.85);
    addObj(g_scene.asObject3D(), grid.asLineSegments().asLine().asObject3D());

    const geos = [9]three.BufferGeometry{
        three.BoxGeometry.new(.{ .width = 1.05, .height = 1.05, .depth = 1.05 }).asBufferGeometry(),
        three.ConeGeometry.new(.{ .radius = 0.62, .height = 1.25, .radialSegments = 20.0 }).asBufferGeometry(),
        three.CylinderGeometry.new(.{ .radiusTop = 0.48, .radiusBottom = 0.48, .height = 1.2, .radialSegments = 24.0 }).asBufferGeometry(),
        three.SphereGeometry.new(.{ .radius = 0.62, .widthSegments = 24.0, .heightSegments = 18.0 }).asBufferGeometry(),
        three.OctahedronGeometry.new(.{ .radius = 0.68, .detail = 0.0 }).asBufferGeometry(),
        three.DodecahedronGeometry.new(.{ .radius = 0.62, .detail = 0.0 }).asBufferGeometry(),
        three.TorusGeometry.new(.{ .radius = 0.5, .tube = 0.18, .radialSegments = 14.0, .tubularSegments = 36.0 }).asBufferGeometry(),
        three.IcosahedronGeometry.new(.{ .radius = 0.64, .detail = 0.0 }).asBufferGeometry(),
        three.TetrahedronGeometry.new(.{ .radius = 0.72, .detail = 0.0 }).asBufferGeometry(),
    };

    g_mesh_arr = three.JsArrayOf(three.Object3D).new();
    for (0..9) |i| {
        const gx: i32 = @as(i32, @intCast(i % 3)) - 1;
        const gz: i32 = @as(i32, @intCast(i / 3)) - 1;
        const mp = three.MeshStandardMaterialParameters.create();
        mp.setColor(.{ .number = 0xd4d4d4 });
        mp.setRoughness(0.55);
        mp.setMetalness(0.1);
        mp.setFlatShading(i >= 4);
        g_mats[i] = three.MeshStandardMaterial.new(mp);

        const m = three.Mesh.new(.{ .geometry = geos[i], .material = .{ .Material = g_mats[i].asMaterial() } });
        const mo = m.asObject3D();
        mo.setName(NAMES[i]);
        _ = mo.getPosition().set(@as(f64, @floatFromInt(gx)) * 2.1, 0.0, .{ .z = @as(f64, @floatFromInt(gz)) * 2.1 });
        mo.getRotation().setX(0.2);
        mo.getRotation().setY(0.35 * @as(f64, @floatFromInt(i)));
        addObj(g_scene.asObject3D(), mo);
        g_meshes[i] = mo;
        g_mesh_arr.push(mo);
    }

    g_box = three.Box3.new(.{ .min = v3(-0.5, -0.5, -0.5), .max = v3(0.5, 0.5, 0.5) });
    g_helper = three.Box3Helper.new(g_box, .{ .color = .{ .number = 0x111111 } });
    g_helper.asLineSegments().asLine().asObject3D().setVisible(false);
    addObj(g_scene.asObject3D(), g_helper.asLineSegments().asLine().asObject3D());

    g_raycaster = three.Raycaster.new(.{ .origin = v3(0.0, 0.0, 0.0), .direction = v3(0.0, 0.0, -1.0) });
    g_meta = doc.getElementById("meta").?;

    g_canvas.addEventListener(.click, dom.JsListener(dom.PointerEvent).from(onCanvasClick, null), .{});

    const ids = [5][]const u8{ "btn-p0", "btn-p1", "btn-p2", "btn-orbit", "btn-clear" };
    for (ids, 0..) |id, i| {
        const b = doc.getElementById(id).?;
        b.addEventListener(.click, dom.JsListener(dom.PointerEvent).from(onBtn, @constCast(&BTN_TAGS[i])), .{});
    }
    pickSpecimen(4);
}
