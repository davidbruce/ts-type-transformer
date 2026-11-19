const std = @import("std");
const dom = @import("dom");
const b = @import("babylonjs_lite");

var g_canvas: dom.HTMLCanvasElement = undefined;
var g_engine: b.EngineContext = undefined;
var g_spot_pos: b.ObservableVec3 = undefined;
var g_spot_dir: b.ObservableVec3 = undefined;
var g_poly_pos: [5]b.ObservableVec3 = undefined;
var g_poly_rot: [5]b.EulerProxy = undefined;
var g_poly_x: [5]f64 = undefined;
var g_poly_z: [5]f64 = undefined;
var g_elapsed: f64 = 0.0;

fn setStatusError(err: b.JsValue) void {
    const doc = dom.getDocument();
    defer dom.jsRelease(doc);
    if (doc.getElementById("status")) |status| {
        defer dom.jsRelease(status);
        const msg = b.jsErrorMessage(err);
        defer b.jsRelease(msg);
        var msg_buf: [256]u8 = undefined;
        var out_buf: [320]u8 = undefined;
        const msg_slice = msg.copy(&msg_buf);
        const text = std.fmt.bufPrint(&out_buf, "WebGPU / Babylon Lite error: {s}", .{msg_slice}) catch return;
        status.asNode().setTextContent(text);
    }
}

fn onBeforeRender(_: ?*anyopaque, delta_ms: f64) void {
    g_elapsed += delta_ms * 0.001;
    const sx = @sin(g_elapsed * 0.65) * 5.2;
    g_spot_pos.set(sx, 6.2, 3.0);
    g_spot_dir.set(-sx * 0.15, -1.0, -0.55);
    var i: usize = 0;
    while (i < 5) : (i += 1) {
        const fi: f64 = @floatFromInt(i);
        g_poly_pos[i].set(g_poly_x[i], 2.05 + @sin(g_elapsed * 1.3 + fi * 1.1) * 0.18, g_poly_z[i]);
        g_poly_rot[i].setY(g_elapsed * (0.45 + 0.12 * fi));
        g_poly_rot[i].setX(@sin(g_elapsed * 0.7 + fi) * 0.35);
    }
}

fn onStarted(_: ?*anyopaque, ok: bool, val: b.JsValue) void {
    if (!ok) setStatusError(val);
}

fn onRegistered(_: ?*anyopaque, ok: bool, val: b.JsValue) void {
    if (!ok) {
        setStatusError(val);
        return;
    }
    const p = b.startEngine(g_engine);
    defer b.jsRelease(p);
    p.then(onStarted, null);
}

fn mkStd(diff: f64, spec: f64, power: f64) b.StandardMaterialProps {
    const mat = b.createStandardMaterial();
    const dc = b.JsTuple(&.{ f64, f64, f64 }).of(.{ diff, diff, diff });
    defer b.jsRelease(dc);
    const sc = b.JsTuple(&.{ f64, f64, f64 }).of(.{ spec, spec, spec });
    defer b.jsRelease(sc);
    mat.setDiffuseColor(dc);
    mat.setSpecularColor(sc);
    mat.setSpecularPower(power);
    return mat;
}

fn onEngine(_: ?*anyopaque, ok: bool, val: b.JsValue) void {
    if (!ok) {
        setStatusError(val);
        return;
    }
    g_engine = .{ .h = val.h };
    _ = b.jsRetain(g_engine);

    const scene = b.createSceneContext(g_engine.asSurfaceContext(), .{});
    defer b.jsRelease(scene);
    const clear = dom.GPUColorDict.create();
    defer dom.jsRelease(clear);
    clear.setR(0.05);
    clear.setG(0.05);
    clear.setB(0.05);
    clear.setA(1.0);
    scene.setClearColor(clear);

    const cam_target = b.vec3(0.0, 1.6, 0.0);
    defer b.jsRelease(cam_target);
    const camera = b.createArcRotateCamera(std.math.pi / 2.0, std.math.pi / 2.8, 11.5, cam_target);
    defer b.jsRelease(camera);
    b.addToScene(scene, .{ .Camera = camera.asCamera() });
    const ctrl = b.attachControl(camera, g_canvas, .{ .scene = scene });
    defer b.jsRelease(ctrl);

    const hemi_dir = b.JsTuple(&.{ f64, f64, f64 }).of(.{ 0.0, 1.0, 0.0 });
    defer b.jsRelease(hemi_dir);
    const hemi = b.createHemisphericLight(.{ .direction = hemi_dir, .intensity = 0.42 });
    defer b.jsRelease(hemi);
    const hdc = b.JsTuple(&.{ f64, f64, f64 }).of(.{ 0.70, 0.70, 0.70 });
    defer b.jsRelease(hdc);
    const hgc = b.JsTuple(&.{ f64, f64, f64 }).of(.{ 0.08, 0.08, 0.08 });
    defer b.jsRelease(hgc);
    hemi.setDiffuseColor(hdc);
    hemi.setGroundColor(hgc);
    b.addToScene(scene, .{ .LightBase = hemi.asLightBase() });

    const spot_p = b.JsTuple(&.{ f64, f64, f64 }).of(.{ 0.0, 6.2, 3.0 });
    defer b.jsRelease(spot_p);
    const spot_d = b.JsTuple(&.{ f64, f64, f64 }).of(.{ 0.0, -1.0, -0.55 });
    defer b.jsRelease(spot_d);
    const spot = b.createSpotLight(spot_p, spot_d, std.math.pi / 3.2, 3.0, .{ .intensity = 26.0 });
    defer b.jsRelease(spot);
    const sd = b.JsTuple(&.{ f64, f64, f64 }).of(.{ 1.0, 1.0, 1.0 });
    defer b.jsRelease(sd);
    spot.setDiffuse(sd);
    spot.setRange(22.0);
    b.addToScene(scene, .{ .LightBase = spot.asLightBase() });
    g_spot_pos = spot.getPosition();
    g_spot_dir = spot.getDirection();

    const fill_v = b.JsTuple(&.{ f64, f64, f64 }).of(.{ 1.2, -1.5, -1.0 });
    defer b.jsRelease(fill_v);
    const fill = b.createDirectionalLight(fill_v, .{ .intensity = 0.65 });
    defer b.jsRelease(fill);
    const fd = b.JsTuple(&.{ f64, f64, f64 }).of(.{ 0.80, 0.80, 0.80 });
    defer b.jsRelease(fd);
    fill.setDiffuse(fd);
    b.addToScene(scene, .{ .LightBase = fill.asLightBase() });

    const floor_mat = mkStd(0.10, 0.25, 24.0);
    defer b.jsRelease(floor_mat);
    const gopts = b.GroundOptions.create();
    defer b.jsRelease(gopts);
    gopts.setWidth(18.0);
    gopts.setHeight(18.0);
    const floor_m = b.createGround(g_engine, .{ .options = gopts });
    defer b.jsRelease(floor_m);
    floor_m.setMaterial(floor_mat.asMaterial());
    b.addToScene(scene, .{ .Mesh = floor_m });

    const plinth_mat = mkStd(0.18, 0.30, 32.0);
    defer b.jsRelease(plinth_mat);

    var i: usize = 0;
    while (i < 5) : (i += 1) {
        const fi: f64 = @floatFromInt(i);
        const x = (fi - 2.0) * 2.25;
        const z = -@abs(fi - 2.0) * 0.55;
        g_poly_x[i] = x;
        g_poly_z[i] = z;

        const copts = b.CylinderOptions.create();
        defer b.jsRelease(copts);
        copts.setHeight(1.1);
        copts.setDiameter(1.15);
        copts.setTessellation(28.0);
        const plinth = b.createCylinder(g_engine, .{ .options = copts });
        defer b.jsRelease(plinth);
        const ppos = plinth.getPosition();
        defer b.jsRelease(ppos);
        ppos.set(x, 0.55, z);
        plinth.setMaterial(plinth_mat.asMaterial());
        b.addToScene(scene, .{ .Mesh = plinth });

        const v = 0.90 - fi * 0.13;
        const poly_mat = mkStd(v, 0.95, 18.0 + fi * 24.0);
        defer b.jsRelease(poly_mat);
        const popts = b.PolyhedronOptions.create();
        defer b.jsRelease(popts);
        popts.setType(fi);
        popts.setSize(0.62);
        popts.setFlat(true);
        const mesh = b.createPolyhedron(g_engine, .{ .options = popts });
        defer b.jsRelease(mesh);
        g_poly_pos[i] = mesh.getPosition();
        g_poly_pos[i].set(x, 2.05, z);
        g_poly_rot[i] = mesh.getRotation();
        mesh.setMaterial(poly_mat.asMaterial());
        b.addToScene(scene, .{ .Mesh = mesh });
    }

    const render_cb = b.number_fn.from(onBeforeRender, null);
    defer b.jsRelease(render_cb);
    b.onBeforeRender(scene, render_cb);

    const reg = b.registerScene(scene);
    defer b.jsRelease(reg);
    reg.then(onRegistered, null);
}

export fn start() void {
    const doc = dom.getDocument();
    defer dom.jsRelease(doc);
    g_canvas = doc.querySelector(.canvas) orelse return;
    const p = b.createEngineWithEngineOptions(.{ .HTMLCanvasElement = g_canvas }, .{});
    defer b.jsRelease(p);
    p.then(onEngine, null);
}
