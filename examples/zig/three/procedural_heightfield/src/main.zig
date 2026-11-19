const std = @import("std");
const dom = @import("dom");
const three = @import("three");

const GRID_N: usize = 28;

fn v3(x: f64, y: f64, z: f64) three.Vector3 {
    return three.Vector3.new(.{ .x = x, .y = y, .z = z });
}

fn addObj(parent: three.Object3D, child: three.Object3D) void {
    _ = parent.add(three.JsArrayOf(three.Object3D).of(&.{child}));
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

var g_renderer: three.WebGLRenderer = undefined;
var g_scene: three.Scene = undefined;
var g_camera: three.PerspectiveCamera = undefined;
var g_geo: three.BufferGeometry = undefined;
var g_pos_attr: three.Float32BufferAttribute = undefined;
var g_pts_attr: three.Float32BufferAttribute = undefined;
var g_wire_mesh: three.Mesh = undefined;
var g_wire_btn: dom.HTMLElement = undefined;
var g_meta: dom.HTMLElement = undefined;
var g_tri_count: usize = 0;
var g_wave_mode: usize = 0;
var g_overlay_on: bool = true;
var g_azimuth: f64 = 0.62;

fn rebuildField() void {
    const span: f64 = 7.6;
    const half: f64 = span / 2.0;
    const denom: f64 = @floatFromInt(GRID_N - 1);

    var iz: usize = 0;
    while (iz < GRID_N) : (iz += 1) {
        var ix: usize = 0;
        while (ix < GRID_N) : (ix += 1) {
            const idx = iz * GRID_N + ix;
            const x = -half + span * @as(f64, @floatFromInt(ix)) / denom;
            const z = -half + span * @as(f64, @floatFromInt(iz)) / denom;
            const y = waveHeight(g_wave_mode, x, z);
            _ = g_pos_attr.setXYZ(@floatFromInt(idx), x, y, z);
        }
    }

    var pi: usize = 0;
    iz = 0;
    while (iz < GRID_N) : (iz += 2) {
        var ix: usize = 0;
        while (ix < GRID_N) : (ix += 2) {
            const x = -half + span * @as(f64, @floatFromInt(ix)) / denom;
            const z = -half + span * @as(f64, @floatFromInt(iz)) / denom;
            const y = waveHeight(g_wave_mode, x, z) + 0.06;
            _ = g_pts_attr.setXYZ(@floatFromInt(pi), x, y, z);
            pi += 1;
        }
    }

    g_pos_attr.asBufferAttribute().setNeedsUpdate(true);
    g_pts_attr.asBufferAttribute().setNeedsUpdate(true);
    g_geo.computeVertexNormals();
    g_geo.computeBoundingBox();

    const r = 9.2;
    _ = g_camera.asObject3D().getPosition().set(r * @sin(g_azimuth), 5.4, .{ .z = r * @cos(g_azimuth) });
    const orig = v3(0.0, 0.0, 0.0);
    g_camera.asObject3D().lookAt(orig);

    g_wire_mesh.asObject3D().setVisible(g_overlay_on);
    g_wire_btn.asNode().setTextContent(if (g_overlay_on) "Wireframe: On" else "Wireframe: Off");
    g_renderer.render(g_scene.asObject3D(), g_camera.asCamera());

    const bb = g_geo.getBoundingBox().?;
    var buf: [256]u8 = undefined;
    const msg = std.fmt.bufPrint(
        &buf,
        "profile: {s} · {d} vertices · {d} triangles · y=[{d:.2}, {d:.2}] · wireframe: {s}",
        .{
            waveName(g_wave_mode),
            GRID_N * GRID_N,
            g_tri_count,
            bb.getMin().getY(),
            bb.getMax().getY(),
            if (g_overlay_on) "on" else "off",
        },
    ) catch return;
    g_meta.asNode().setTextContent(msg);
}

const BTN_TAGS = [5]usize{ 0, 1, 2, 3, 4 };
fn onBtn(ctx: ?*anyopaque, _: dom.PointerEvent) void {
    const id = @as(*const usize, @ptrCast(@alignCast(ctx.?))).*;
    if (id < 3) {
        g_wave_mode = id;
    } else if (id == 3) {
        g_overlay_on = !g_overlay_on;
    } else {
        g_azimuth += std.math.pi / 4.0;
    }
    rebuildField();
}

export fn start() void {
    const doc = dom.getDocument();
    const width: f64 = 680.0;
    const height: f64 = 400.0;

    const rp = three.WebGLRendererParameters.create();
    rp.setAntialias(true);
    rp.setPreserveDrawingBuffer(true);
    g_renderer = three.WebGLRenderer.new(rp);
    g_renderer.setPixelRatio(1.0);
    g_renderer.setSize(width, height, .{});
    _ = doc.getElementById("stage").?.asNode().appendChild(.{ .h = g_renderer.getDomElement().h });

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
        .divisions = 12.0,
        .color1 = .{ .number = 0x999999 },
        .color2 = .{ .number = 0xe5e5e5 },
    });
    grid.asLineSegments().asLine().asObject3D().getPosition().setY(-1.1);
    addObj(g_scene.asObject3D(), grid.asLineSegments().asLine().asObject3D());

    g_geo = three.BufferGeometry.new();
    g_pos_attr = three.Float32BufferAttribute.new(.{ .number = @floatFromInt(GRID_N * GRID_N * 3) }, 3.0, .{});

    const indices = three.JsArrayOf(f64).new();
    var iz: usize = 0;
    while (iz < GRID_N - 1) : (iz += 1) {
        var ix: usize = 0;
        while (ix < GRID_N - 1) : (ix += 1) {
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
    g_tri_count = (GRID_N - 1) * (GRID_N - 1) * 2;
    _ = g_geo.setAttribute(three.JsValue.string("position"), .{ .BufferAttribute = g_pos_attr.asBufferAttribute() });
    _ = g_geo.setIndex2(.{ .array = indices });

    const smp = three.MeshStandardMaterialParameters.create();
    smp.setColor(.{ .number = 0xb5b5b5 });
    smp.setRoughness(0.72);
    smp.setMetalness(0.05);
    smp.setFlatShading(true);
    const surf = three.Mesh.new(.{ .geometry = g_geo, .material = .{ .Material = three.MeshStandardMaterial.new(smp).asMaterial() } });
    addObj(g_scene.asObject3D(), surf.asObject3D());

    const wmp = three.MeshBasicMaterialParameters.create();
    wmp.setColor(.{ .number = 0x262626 });
    wmp.setWireframe(true);
    g_wire_mesh = three.Mesh.new(.{ .geometry = g_geo, .material = .{ .Material = three.MeshBasicMaterial.new(wmp).asMaterial() } });
    g_wire_mesh.asObject3D().getPosition().setY(0.015);
    addObj(g_scene.asObject3D(), g_wire_mesh.asObject3D());

    const pt_count = (GRID_N / 2) * (GRID_N / 2);
    const pts_geo = three.BufferGeometry.new();
    g_pts_attr = three.Float32BufferAttribute.new(.{ .number = @floatFromInt(pt_count * 3) }, 3.0, .{});
    _ = pts_geo.setAttribute(three.JsValue.string("position"), .{ .BufferAttribute = g_pts_attr.asBufferAttribute() });
    const pmp = three.PointsMaterialParameters.create();
    pmp.setColor(.{ .number = 0x111111 });
    pmp.setSize(0.14);
    const pts = three.Points.new(.{ .geometry = pts_geo, .material = .{ .Material = three.PointsMaterial.new(pmp).asMaterial() } });
    addObj(g_scene.asObject3D(), pts.asObject3D());

    g_wire_btn = doc.getElementById("btn-wire").?;
    g_meta = doc.getElementById("meta").?;

    const ids = [5][]const u8{ "btn-p0", "btn-p1", "btn-p2", "btn-wire", "btn-orbit" };
    for (ids, 0..) |id, i| {
        const b = doc.getElementById(id).?;
        b.addEventListener(.click, dom.JsListener(dom.PointerEvent).from(onBtn, @constCast(&BTN_TAGS[i])), .{});
    }
    rebuildField();
}
