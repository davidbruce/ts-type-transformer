const std = @import("std");
const dom = @import("dom");
const b = @import("babylonjs_lite");

var g_canvas: dom.HTMLCanvasElement = undefined;
var g_engine: b.EngineContext = undefined;
var g_knot_rot: b.EulerProxy = undefined;
var g_orb_pos: b.ObservableVec3 = undefined;
var g_orb_light_pos: b.ObservableVec3 = undefined;
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
    g_knot_rot.setY(g_elapsed * 0.55);
    g_knot_rot.setX(@sin(g_elapsed * 0.4) * 0.25);
    const x = @cos(g_elapsed * 0.9) * 2.8;
    const y = 2.0 + @sin(g_elapsed * 1.8) * 0.45;
    const z = @sin(g_elapsed * 0.9) * 2.8;
    g_orb_pos.set(x, y, z);
    g_orb_light_pos.set(x, y, z);
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
    clear.setG(0.07);
    clear.setB(0.11);
    clear.setA(1.0);
    scene.setClearColor(clear);

    const cam_target = b.vec3(0.0, 1.6, 0.0);
    defer b.jsRelease(cam_target);
    const camera = b.createArcRotateCamera(std.math.pi / 4.0, std.math.pi / 3.0, 8.5, cam_target);
    defer b.jsRelease(camera);
    b.addToScene(scene, .{ .Camera = camera.asCamera() });
    const ctrl = b.attachControl(camera, g_canvas, .{ .scene = scene });
    defer b.jsRelease(ctrl);

    const hemi_dir = b.JsTuple(&.{ f64, f64, f64 }).of(.{ 0.0, 1.0, 0.2 });
    defer b.jsRelease(hemi_dir);
    const hemi = b.createHemisphericLight(.{ .direction = hemi_dir, .intensity = 0.75 });
    defer b.jsRelease(hemi);
    const hdc = b.JsTuple(&.{ f64, f64, f64 }).of(.{ 0.75, 0.85, 1.0 });
    defer b.jsRelease(hdc);
    const hgc = b.JsTuple(&.{ f64, f64, f64 }).of(.{ 0.12, 0.14, 0.22 });
    defer b.jsRelease(hgc);
    hemi.setDiffuseColor(hdc);
    hemi.setGroundColor(hgc);
    b.addToScene(scene, .{ .LightBase = hemi.asLightBase() });

    const dir_v = b.JsTuple(&.{ f64, f64, f64 }).of(.{ -1.0, -2.0, -1.2 });
    defer b.jsRelease(dir_v);
    const dir = b.createDirectionalLight(dir_v, .{ .intensity = 2.2 });
    defer b.jsRelease(dir);
    const dd = b.JsTuple(&.{ f64, f64, f64 }).of(.{ 1.0, 0.96, 0.9 });
    defer b.jsRelease(dd);
    dir.setDiffuse(dd);
    b.addToScene(scene, .{ .LightBase = dir.asLightBase() });

    const orb_lpos = b.JsTuple(&.{ f64, f64, f64 }).of(.{ 2.8, 1.6, 0.0 });
    defer b.jsRelease(orb_lpos);
    const orb_light = b.createPointLight(orb_lpos, .{ .intensity = 8.0 });
    defer b.jsRelease(orb_light);
    const old = b.JsTuple(&.{ f64, f64, f64 }).of(.{ 1.0, 0.45, 0.35 });
    defer b.jsRelease(old);
    orb_light.setDiffuse(old);
    orb_light.setRange(12.0);
    b.addToScene(scene, .{ .LightBase = orb_light.asLightBase() });
    g_orb_light_pos = orb_light.getPosition();

    const kprops = b.createPbrMaterial_props.create();
    defer b.jsRelease(kprops);
    const kbcf = b.JsTuple(&.{ f64, f64, f64, f64 }).of(.{ 0.18, 0.78, 0.95, 1.0 });
    defer b.jsRelease(kbcf);
    kprops.setBaseColorFactor(kbcf);
    kprops.setMetallicFactor(0.75);
    kprops.setRoughnessFactor(0.22);
    const knot_mat = b.createPbrMaterial(kprops);
    defer b.jsRelease(knot_mat);

    const kopts = b.TorusKnotOptions.create();
    defer b.jsRelease(kopts);
    kopts.setRadius(1.15);
    kopts.setTube(0.34);
    kopts.setRadialSegments(48.0);
    kopts.setTubularSegments(160.0);
    const knot = b.createTorusKnot(g_engine, .{ .options = kopts });
    defer b.jsRelease(knot);
    const knot_pos = knot.getPosition();
    defer b.jsRelease(knot_pos);
    knot_pos.set(0.0, 2.0, 0.0);
    knot.setMaterial(knot_mat.asMaterial());
    b.addToScene(scene, .{ .Mesh = knot });
    g_knot_rot = knot.getRotation();

    const oprops = b.createPbrMaterial_props.create();
    defer b.jsRelease(oprops);
    const obcf = b.JsTuple(&.{ f64, f64, f64, f64 }).of(.{ 1.0, 0.42, 0.32, 1.0 });
    defer b.jsRelease(obcf);
    oprops.setBaseColorFactor(obcf);
    oprops.setMetallicFactor(0.2);
    oprops.setRoughnessFactor(0.35);
    const orb_mat = b.createPbrMaterial(oprops);
    defer b.jsRelease(orb_mat);

    const sopts = b.SphereOptions.create();
    defer b.jsRelease(sopts);
    sopts.setDiameter(0.55);
    sopts.setSegments(32.0);
    const orb = b.createSphere(g_engine, .{ .options = sopts });
    defer b.jsRelease(orb);
    orb.setMaterial(orb_mat.asMaterial());
    b.addToScene(scene, .{ .Mesh = orb });
    g_orb_pos = orb.getPosition();

    const ground_mat = b.createGridMaterial(null);
    defer b.jsRelease(ground_mat);
    const gopts = b.GroundOptions.create();
    defer b.jsRelease(gopts);
    gopts.setWidth(16.0);
    gopts.setHeight(16.0);
    const ground = b.createGround(g_engine, .{ .options = gopts });
    defer b.jsRelease(ground);
    ground.setMaterial(ground_mat.asMaterial());
    b.addToScene(scene, .{ .Mesh = ground });

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
