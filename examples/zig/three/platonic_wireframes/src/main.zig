const std = @import("std");
const dom = @import("dom");
const three = @import("three");

fn v3(x: f64, y: f64, z: f64) three.Vector3 {
    return three.Vector3.new(.{ .x = x, .y = y, .z = z });
}

fn addObj(parent: three.Object3D, child: three.Object3D) void {
    _ = parent.add(three.JsArrayOf(three.Object3D).of(&.{child}));
}

var g_renderer: three.WebGLRenderer = undefined;
var g_scene: three.Scene = undefined;
var g_camera: three.PerspectiveCamera = undefined;
var g_wire_group: three.Group = undefined;
var g_wire_btn: dom.HTMLElement = undefined;
var g_meta: dom.HTMLElement = undefined;
var g_wire_visible: bool = true;
var g_azimuth: f64 = 0.35;

fn addSolid(
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
    const mp = three.MeshStandardMaterialParameters.create();
    mp.setColor(.{ .number = hex });
    mp.setRoughness(rough);
    mp.setMetalness(metal);
    mp.setFlatShading(flat);
    const mat = three.MeshStandardMaterial.new(mp);

    const mesh = three.Mesh.new(.{ .geometry = geo, .material = .{ .Material = mat.asMaterial() } });
    _ = mesh.asObject3D().getPosition().set(x, y, .{ .z = z });
    mesh.asObject3D().getRotation().setX(0.25);
    mesh.asObject3D().getRotation().setY(0.45);
    addObj(g_scene.asObject3D(), mesh.asObject3D());

    const egeo = three.EdgesGeometry.new(geo, .{ .thresholdAngle = thresh });
    const segs = three.LineSegments.new(.{ .geometry = egeo.asBufferGeometry(), .material = .{ .Material = line_mat.asMaterial() } });
    _ = segs.asLine().asObject3D().getPosition().set(x, y, .{ .z = z });
    segs.asLine().asObject3D().getRotation().setX(0.25);
    segs.asLine().asObject3D().getRotation().setY(0.45);
    addObj(g_wire_group.asObject3D(), segs.asLine().asObject3D());
}

fn updateView() void {
    const r = 8.6;
    _ = g_camera.asObject3D().getPosition().set(r * @sin(g_azimuth), 3.6, .{ .z = r * @cos(g_azimuth) });
    const orig = v3(0.0, 0.0, 0.0);
    g_camera.asObject3D().lookAt(orig);

    g_wire_group.asObject3D().setVisible(g_wire_visible);
    g_wire_btn.asNode().setTextContent(if (g_wire_visible) "Wireframes: On" else "Wireframes: Off");
    g_renderer.render(g_scene.asObject3D(), g_camera.asCamera());

    const deg: i32 = @intFromFloat(@round(g_azimuth * 180.0 / std.math.pi));
    var buf: [256]u8 = undefined;
    const msg = std.fmt.bufPrint(
        &buf,
        "5 monochrome solids · EdgesGeometry wireframes: {s} · azimuth: {d} deg · 680x400 WebGL",
        .{ if (g_wire_visible) "on" else "off", deg },
    ) catch return;
    g_meta.asNode().setTextContent(msg);
}

const BTN_TAGS = [4]usize{ 0, 1, 2, 3 };
fn onBtn(ctx: ?*anyopaque, _: dom.PointerEvent) void {
    const id = @as(*const usize, @ptrCast(@alignCast(ctx.?))).*;
    if (id == 0) {
        g_wire_visible = !g_wire_visible;
    } else if (id == 1) {
        g_azimuth -= std.math.pi / 6.0;
    } else if (id == 2) {
        g_azimuth += std.math.pi / 6.0;
    } else {
        g_azimuth = 0.35;
        g_wire_visible = true;
    }
    updateView();
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

    const stage = doc.getElementById("stage").?;
    _ = stage.asNode().appendChild(.{ .h = g_renderer.getDomElement().h });

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
        .size = 12.0,
        .divisions = 12.0,
        .color1 = .{ .number = 0x737373 },
        .color2 = .{ .number = 0xd4d4d4 },
    });
    grid.asLineSegments().asLine().asObject3D().getPosition().setY(-1.2);
    addObj(g_scene.asObject3D(), grid.asLineSegments().asLine().asObject3D());

    g_wire_group = three.Group.new();
    addObj(g_scene.asObject3D(), g_wire_group.asObject3D());

    const lmp = three.LineBasicMaterialParameters.create();
    lmp.setColor(.{ .number = 0x111111 });
    const line_mat = three.LineBasicMaterial.new(lmp);

    addSolid(three.BoxGeometry.new(.{ .width = 1.45, .height = 1.45, .depth = 1.45 }).asBufferGeometry(), 0x262626, 0.65, 0.1, false, -3.2, 0.0, -0.6, 1.0, line_mat);
    addSolid(three.IcosahedronGeometry.new(.{ .radius = 0.95, .detail = 0.0 }).asBufferGeometry(), 0xd4d4d4, 0.55, 0.08, true, -1.55, 0.1, 0.55, 1.0, line_mat);
    addSolid(three.TorusKnotGeometry.new(.{ .radius = 0.72, .tube = 0.24, .tubularSegments = 96.0, .radialSegments = 16.0 }).asBufferGeometry(), 0x737373, 0.45, 0.15, false, 0.0, 0.25, -0.2, 24.0, line_mat);
    addSolid(three.OctahedronGeometry.new(.{ .radius = 0.95, .detail = 0.0 }).asBufferGeometry(), 0xefefef, 0.5, 0.05, true, 1.6, 0.1, 0.55, 1.0, line_mat);
    addSolid(three.TorusGeometry.new(.{ .radius = 0.72, .tube = 0.23, .radialSegments = 16.0, .tubularSegments = 48.0 }).asBufferGeometry(), 0x404040, 0.55, 0.12, false, 3.2, 0.05, -0.5, 22.0, line_mat);

    g_wire_btn = doc.getElementById("btn-wire").?;
    g_meta = doc.getElementById("meta").?;

    const ids = [4][]const u8{ "btn-wire", "btn-left", "btn-right", "btn-reset" };
    for (ids, 0..) |id, i| {
        const b = doc.getElementById(id).?;
        b.addEventListener(.click, dom.JsListener(dom.PointerEvent).from(onBtn, @constCast(&BTN_TAGS[i])), .{});
    }
    updateView();
}
