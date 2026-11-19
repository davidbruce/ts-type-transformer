const std = @import("std");
const dom = @import("dom");
const b = @import("babylonjs_lite");

var g_canvas: dom.HTMLCanvasElement = undefined;
var g_engine: b.EngineContext = undefined;
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
    const x = @cos(g_elapsed * 0.85) * 4.5;
    const y = 3.0 + @sin(g_elapsed * 1.4) * 0.65;
    const z = @sin(g_elapsed * 0.85) * 4.5;
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

fn mkPbr(v: f64, metallic: f64, roughness: f64) b.PbrMaterialProps {
    const props = b.createPbrMaterial_props.create();
    defer b.jsRelease(props);
    const col = b.JsTuple(&.{ f64, f64, f64, f64 }).of(.{ v, v, v, 1.0 });
    defer b.jsRelease(col);
    props.setBaseColorFactor(col);
    props.setMetallicFactor(metallic);
    props.setRoughnessFactor(roughness);
    return b.createPbrMaterial(props);
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
    clear.setR(0.06);
    clear.setG(0.06);
    clear.setB(0.06);
    clear.setA(1.0);
    scene.setClearColor(clear);

    const cam_target = b.vec3(0.0, 1.1, 0.0);
    defer b.jsRelease(cam_target);
    const camera = b.createArcRotateCamera(std.math.pi / 4.0, std.math.pi / 3.1, 12.0, cam_target);
    defer b.jsRelease(camera);
    b.addToScene(scene, .{ .Camera = camera.asCamera() });
    const ctrl = b.attachControl(camera, g_canvas, .{ .scene = scene });
    defer b.jsRelease(ctrl);

    const hemi_dir = b.JsTuple(&.{ f64, f64, f64 }).of(.{ 0.0, 1.0, 0.2 });
    defer b.jsRelease(hemi_dir);
    const hemi = b.createHemisphericLight(.{ .direction = hemi_dir, .intensity = 0.65 });
    defer b.jsRelease(hemi);
    const hdc = b.JsTuple(&.{ f64, f64, f64 }).of(.{ 0.82, 0.82, 0.82 });
    defer b.jsRelease(hdc);
    const hgc = b.JsTuple(&.{ f64, f64, f64 }).of(.{ 0.12, 0.12, 0.12 });
    defer b.jsRelease(hgc);
    hemi.setDiffuseColor(hdc);
    hemi.setGroundColor(hgc);
    b.addToScene(scene, .{ .LightBase = hemi.asLightBase() });

    const dir_v = b.JsTuple(&.{ f64, f64, f64 }).of(.{ -1.0, -2.0, -1.0 });
    defer b.jsRelease(dir_v);
    const dir = b.createDirectionalLight(dir_v, .{ .intensity = 1.8 });
    defer b.jsRelease(dir);
    const dd = b.JsTuple(&.{ f64, f64, f64 }).of(.{ 0.95, 0.95, 0.95 });
    defer b.jsRelease(dd);
    dir.setDiffuse(dd);
    b.addToScene(scene, .{ .LightBase = dir.asLightBase() });

    const orb_lpos = b.JsTuple(&.{ f64, f64, f64 }).of(.{ 4.5, 3.0, 0.0 });
    defer b.jsRelease(orb_lpos);
    const orb_light = b.createPointLight(orb_lpos, .{ .intensity = 12.0 });
    defer b.jsRelease(orb_light);
    const old = b.JsTuple(&.{ f64, f64, f64 }).of(.{ 1.0, 1.0, 1.0 });
    defer b.jsRelease(old);
    orb_light.setDiffuse(old);
    orb_light.setRange(18.0);
    b.addToScene(scene, .{ .LightBase = orb_light.asLightBase() });
    g_orb_light_pos = orb_light.getPosition();

    const floor_mat = mkPbr(0.11, 0.08, 0.88);
    defer b.jsRelease(floor_mat);
    const gopts = b.GroundOptions.create();
    defer b.jsRelease(gopts);
    gopts.setWidth(14.0);
    gopts.setHeight(14.0);
    const floor_m = b.createGround(g_engine, .{ .options = gopts });
    defer b.jsRelease(floor_m);
    floor_m.setMaterial(floor_mat.asMaterial());
    b.addToScene(scene, .{ .Mesh = floor_m });

    const plinth_mat = mkPbr(0.17, 0.12, 0.78);
    defer b.jsRelease(plinth_mat);
    const popts = b.BoxOptions.create();
    defer b.jsRelease(popts);
    popts.setWidth(8.4);
    popts.setHeight(0.08);
    popts.setDepth(8.4);
    const plinth = b.createBox(g_engine, .{ .options = .{ .BoxOptions = popts } });
    defer b.jsRelease(plinth);
    const plinth_pos = plinth.getPosition();
    defer b.jsRelease(plinth_pos);
    plinth_pos.set(0.0, 0.04, 0.0);
    plinth.setMaterial(plinth_mat.asMaterial());
    b.addToScene(scene, .{ .Mesh = plinth });

    var row: i32 = 0;
    while (row < 5) : (row += 1) {
        var col: i32 = 0;
        while (col < 5) : (col += 1) {
            const frow: f64 = @floatFromInt(row);
            const fcol: f64 = @floatFromInt(col);
            const mat = mkPbr(0.85, 0.25 * fcol, 0.08 + 0.22 * frow);
            defer b.jsRelease(mat);
            const sopts = b.SphereOptions.create();
            defer b.jsRelease(sopts);
            sopts.setDiameter(1.05);
            sopts.setSegments(24.0);
            const sphere = b.createSphere(g_engine, .{ .options = sopts });
            defer b.jsRelease(sphere);
            const spos = sphere.getPosition();
            defer b.jsRelease(spos);
            spos.set((fcol - 2.0) * 1.55, 0.75, (frow - 2.0) * 1.55);
            sphere.setMaterial(mat.asMaterial());
            b.addToScene(scene, .{ .Mesh = sphere });
        }
    }

    const oprops = b.createPbrMaterial_props.create();
    defer b.jsRelease(oprops);
    const obcf = b.JsTuple(&.{ f64, f64, f64, f64 }).of(.{ 0.95, 0.95, 0.95, 1.0 });
    defer b.jsRelease(obcf);
    const oem = b.JsTuple(&.{ f64, f64, f64 }).of(.{ 0.95, 0.95, 0.95 });
    defer b.jsRelease(oem);
    oprops.setBaseColorFactor(obcf);
    oprops.setMetallicFactor(0.0);
    oprops.setRoughnessFactor(0.2);
    b.setPbrEmissive(oprops, oem);
    const orb_mat = b.createPbrMaterial(oprops);
    defer b.jsRelease(orb_mat);

    const mopts = b.SphereOptions.create();
    defer b.jsRelease(mopts);
    mopts.setDiameter(0.36);
    mopts.setSegments(16.0);
    const orb = b.createSphere(g_engine, .{ .options = mopts });
    defer b.jsRelease(orb);
    g_orb_pos = orb.getPosition();
    g_orb_pos.set(4.5, 3.0, 0.0);
    orb.setMaterial(orb_mat.asMaterial());
    b.addToScene(scene, .{ .Mesh = orb });

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
