const std = @import("std");
const dom = @import("dom");
const three = @import("three");

fn v3(x: f64, y: f64, z: f64) three.Vector3 {
    return three.Vector3.new(.{ .x = x, .y = y, .z = z });
}

fn addObj(parent: three.Object3D, child: three.Object3D) void {
    _ = parent.add(three.JsArrayOf(three.Object3D).of(&.{child}));
}

fn stdMat(hex: f64, rough: f64, metal: f64, flat: bool) three.Material {
    const mp = three.MeshStandardMaterialParameters.create();
    mp.setColor(.{ .number = hex });
    mp.setRoughness(rough);
    mp.setMetalness(metal);
    mp.setFlatShading(flat);
    return three.MeshStandardMaterial.new(mp).asMaterial();
}

var g_renderer: three.WebGLRenderer = undefined;
var g_scene: three.Scene = undefined;
var g_camera: three.PerspectiveCamera = undefined;
var g_inner_pivot: three.Group = undefined;
var g_moon_pivot: three.Group = undefined;
var g_outer_pivot: three.Group = undefined;
var g_outer_station: three.Group = undefined;
var g_moon: three.Mesh = undefined;
var g_station_body: three.Mesh = undefined;
var g_tilt_btn: dom.HTMLElement = undefined;
var g_meta: dom.HTMLElement = undefined;
var g_epoch: f64 = 0.6;
var g_tilted: bool = false;

fn updateOrrery() void {
    g_inner_pivot.asObject3D().getRotation().setY(g_epoch);
    g_moon_pivot.asObject3D().getRotation().setY(g_epoch * 2.5);
    g_outer_pivot.asObject3D().getRotation().setY(-g_epoch * 0.55);
    g_outer_pivot.asObject3D().getRotation().setZ(if (g_tilted) 0.38 else 0.0);
    g_station_body.asObject3D().getRotation().setY(g_epoch * 1.4);

    g_scene.asObject3D().updateMatrixWorld(true);
    const mpos = g_moon.asObject3D().getWorldPosition(v3(0.0, 0.0, 0.0));
    const spos = g_outer_station.asObject3D().getWorldPosition(v3(0.0, 0.0, 0.0));

    g_tilt_btn.asNode().setTextContent(if (g_tilted) "Outer Tilt: On" else "Outer Tilt: Off");
    g_renderer.render(g_scene.asObject3D(), g_camera.asCamera());

    const deg: i32 = @intFromFloat(@round(g_epoch * 180.0 / std.math.pi));
    var buf: [256]u8 = undefined;
    const msg = std.fmt.bufPrint(
        &buf,
        "epoch: {d} deg · moon world: ({d:.2}, {d:.2}, {d:.2}) · station world: ({d:.2}, {d:.2}, {d:.2})",
        .{ deg, mpos.getX(), mpos.getY(), mpos.getZ(), spos.getX(), spos.getY(), spos.getZ() },
    ) catch return;
    g_meta.asNode().setTextContent(msg);
}

const BTN_TAGS = [3]usize{ 0, 1, 2 };
fn onBtn(ctx: ?*anyopaque, _: dom.PointerEvent) void {
    const id = @as(*const usize, @ptrCast(@alignCast(ctx.?))).*;
    if (id == 0) {
        g_epoch += std.math.pi / 6.0;
    } else if (id == 1) {
        g_tilted = !g_tilted;
    } else {
        g_epoch = 0.6;
        g_tilted = false;
    }
    updateOrrery();
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
    _ = g_camera.asObject3D().getPosition().set(5.6, 5.2, .{ .z = 7.6 });
    const target = v3(0.0, -0.1, 0.0);
    g_camera.asObject3D().lookAt(target);

    addObj(g_scene.asObject3D(), three.AmbientLight.new(.{ .color = .{ .number = 0xffffff }, .intensity = 1.2 }).asObject3D());
    const key = three.DirectionalLight.new(.{ .color = .{ .number = 0xffffff }, .intensity = 2.2 });
    _ = key.asObject3D().getPosition().set(6.0, 10.0, .{ .z = 8.0 });
    addObj(g_scene.asObject3D(), key.asObject3D());
    const rim = three.DirectionalLight.new(.{ .color = .{ .number = 0xffffff }, .intensity = 0.9 });
    _ = rim.asObject3D().getPosition().set(-7.0, 5.0, .{ .z = -6.0 });
    addObj(g_scene.asObject3D(), rim.asObject3D());

    const grid = three.GridHelper.new(.{
        .size = 12.0,
        .divisions = 12.0,
        .color1 = .{ .number = 0x999999 },
        .color2 = .{ .number = 0xe5e5e5 },
    });
    grid.asLineSegments().asLine().asObject3D().getPosition().setY(-1.5);
    addObj(g_scene.asObject3D(), grid.asLineSegments().asLine().asObject3D());

    const pedestal = three.Mesh.new(.{
        .geometry = three.CylinderGeometry.new(.{ .radiusTop = 0.55, .radiusBottom = 0.75, .height = 0.35, .radialSegments = 24.0 }).asBufferGeometry(),
        .material = .{ .Material = stdMat(0x262626, 0.7, 0.05, false) },
    });
    pedestal.asObject3D().getPosition().setY(-1.3);
    addObj(g_scene.asObject3D(), pedestal.asObject3D());

    const core = three.Mesh.new(.{
        .geometry = three.SphereGeometry.new(.{ .radius = 0.65, .widthSegments = 24.0, .heightSegments = 18.0 }).asBufferGeometry(),
        .material = .{ .Material = stdMat(0x111111, 0.45, 0.2, false) },
    });
    addObj(g_scene.asObject3D(), core.asObject3D());

    g_inner_pivot = three.Group.new();
    addObj(g_scene.asObject3D(), g_inner_pivot.asObject3D());
    const tp1 = three.MeshBasicMaterialParameters.create();
    tp1.setColor(.{ .number = 0x999999 });
    const inner_track = three.Mesh.new(.{
        .geometry = three.TorusGeometry.new(.{ .radius = 2.1, .tube = 0.025, .radialSegments = 8.0, .tubularSegments = 64.0 }).asBufferGeometry(),
        .material = .{ .Material = three.MeshBasicMaterial.new(tp1).asMaterial() },
    });
    inner_track.asObject3D().getRotation().setX(std.math.pi / 2.0);
    addObj(g_inner_pivot.asObject3D(), inner_track.asObject3D());

    const inner_planet = three.Group.new();
    _ = inner_planet.asObject3D().getPosition().set(2.1, 0.0, .{ .z = 0.0 });
    addObj(g_inner_pivot.asObject3D(), inner_planet.asObject3D());

    const planet = three.Mesh.new(.{
        .geometry = three.SphereGeometry.new(.{ .radius = 0.36, .widthSegments = 20.0, .heightSegments = 16.0 }).asBufferGeometry(),
        .material = .{ .Material = stdMat(0x555555, 0.55, 0.1, false) },
    });
    addObj(inner_planet.asObject3D(), planet.asObject3D());

    g_moon_pivot = three.Group.new();
    addObj(inner_planet.asObject3D(), g_moon_pivot.asObject3D());
    g_moon = three.Mesh.new(.{
        .geometry = three.OctahedronGeometry.new(.{ .radius = 0.18, .detail = 0.0 }).asBufferGeometry(),
        .material = .{ .Material = stdMat(0xd4d4d4, 0.4, 0.1, true) },
    });
    _ = g_moon.asObject3D().getPosition().set(0.75, 0.0, .{ .z = 0.0 });
    addObj(g_moon_pivot.asObject3D(), g_moon.asObject3D());

    g_outer_pivot = three.Group.new();
    addObj(g_scene.asObject3D(), g_outer_pivot.asObject3D());
    const tp2 = three.MeshBasicMaterialParameters.create();
    tp2.setColor(.{ .number = 0xb5b5b5 });
    const outer_track = three.Mesh.new(.{
        .geometry = three.TorusGeometry.new(.{ .radius = 3.8, .tube = 0.025, .radialSegments = 8.0, .tubularSegments = 80.0 }).asBufferGeometry(),
        .material = .{ .Material = three.MeshBasicMaterial.new(tp2).asMaterial() },
    });
    outer_track.asObject3D().getRotation().setX(std.math.pi / 2.0);
    addObj(g_outer_pivot.asObject3D(), outer_track.asObject3D());

    g_outer_station = three.Group.new();
    _ = g_outer_station.asObject3D().getPosition().set(3.8, 0.0, .{ .z = 0.0 });
    addObj(g_outer_pivot.asObject3D(), g_outer_station.asObject3D());

    g_station_body = three.Mesh.new(.{
        .geometry = three.IcosahedronGeometry.new(.{ .radius = 0.42, .detail = 0.0 }).asBufferGeometry(),
        .material = .{ .Material = stdMat(0x404040, 0.5, 0.1, true) },
    });
    addObj(g_outer_station.asObject3D(), g_station_body.asObject3D());

    const halo = three.Mesh.new(.{
        .geometry = three.TorusGeometry.new(.{ .radius = 0.68, .tube = 0.06, .radialSegments = 10.0, .tubularSegments = 32.0 }).asBufferGeometry(),
        .material = .{ .Material = stdMat(0x111111, 0.4, 0.1, false) },
    });
    halo.asObject3D().getRotation().setX(0.8);
    addObj(g_outer_station.asObject3D(), halo.asObject3D());

    g_tilt_btn = doc.getElementById("btn-tilt").?;
    g_meta = doc.getElementById("meta").?;

    const ids = [3][]const u8{ "btn-step", "btn-tilt", "btn-reset" };
    for (ids, 0..) |id, i| {
        const b = doc.getElementById(id).?;
        b.addEventListener(.click, dom.JsListener(dom.PointerEvent).from(onBtn, @constCast(&BTN_TAGS[i])), .{});
    }
    updateOrrery();
}
