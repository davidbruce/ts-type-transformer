const std = @import("std");
const dom = @import("dom");
const b = @import("babylonjs_lite");

var g_canvas: dom.HTMLCanvasElement = undefined;
var g_engine: b.EngineContext = undefined;
var g_col_pos: [36]b.ObservableVec3 = undefined;
var g_col_scale: [36]b.ObservableVec3 = undefined;
var g_col_x: [36]f64 = undefined;
var g_col_z: [36]f64 = undefined;
var g_col_dist: [36]f64 = undefined;
var g_col_diag: [36]f64 = undefined;
var g_cap_pos: [4]b.ObservableVec3 = undefined;
var g_cap_rot: [4]b.EulerProxy = undefined;
var g_light1_pos: b.ObservableVec3 = undefined;
var g_light2_pos: b.ObservableVec3 = undefined;
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
    var idx: usize = 0;
    while (idx < 36) : (idx += 1) {
        const h = 0.55 +
            1.15 * (0.5 + 0.5 * @sin(g_elapsed * 2.2 - g_col_dist[idx] * 1.15)) +
            0.30 * @cos(g_elapsed * 1.3 + g_col_diag[idx] * 0.7);
        g_col_scale[idx].set(1.0, h, 1.0);
        g_col_pos[idx].set(g_col_x[idx], h * 0.5, g_col_z[idx]);
    }
    var k: usize = 0;
    while (k < 4) : (k += 1) {
        const fk: f64 = @floatFromInt(k);
        const theta = g_elapsed * 0.7 + fk * (std.math.pi * 0.5);
        g_cap_pos[k].set(@cos(theta) * 4.9, 2.6 + @sin(g_elapsed * 1.6 + fk * 1.57) * 0.55, @sin(theta) * 4.9);
        g_cap_rot[k].setZ(@sin(g_elapsed + fk) * 0.35);
        g_cap_rot[k].setY(-theta);
    }
    g_light1_pos.set(@cos(g_elapsed * 0.65) * 5.2, 4.2, @sin(g_elapsed * 0.65) * 5.2);
    g_light2_pos.set(@cos(-g_elapsed * 0.52) * 5.2, 3.4, @sin(-g_elapsed * 0.52) * 5.2);
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
    defer b.jsRelease(clear);
    clear.setR(0.05);
    clear.setG(0.05);
    clear.setB(0.05);
    clear.setA(1.0);
    scene.setClearColor(clear);

    const cam_target = b.vec3(0.0, 1.4, 0.0);
    defer b.jsRelease(cam_target);
    const camera = b.createArcRotateCamera(std.math.pi / 4.0, std.math.pi / 3.1, 12.8, cam_target);
    defer b.jsRelease(camera);
    b.addToScene(scene, .{ .Camera = camera.asCamera() });
    const ctrl = b.attachControl(camera, g_canvas, .{ .scene = scene });
    defer b.jsRelease(ctrl);

    const hemi_dir = b.JsTuple(&.{ f64, f64, f64 }).of(.{ 0.0, 1.0, 0.15 });
    defer b.jsRelease(hemi_dir);
    const hemi = b.createHemisphericLight(.{ .direction = hemi_dir, .intensity = 0.55 });
    defer b.jsRelease(hemi);
    const hdc = b.JsTuple(&.{ f64, f64, f64 }).of(.{ 0.76, 0.76, 0.76 });
    defer b.jsRelease(hdc);
    const hgc = b.JsTuple(&.{ f64, f64, f64 }).of(.{ 0.10, 0.10, 0.10 });
    defer b.jsRelease(hgc);
    hemi.setDiffuseColor(hdc);
    hemi.setGroundColor(hgc);
    b.addToScene(scene, .{ .LightBase = hemi.asLightBase() });

    const dir_v = b.JsTuple(&.{ f64, f64, f64 }).of(.{ -1.0, -2.2, -0.8 });
    defer b.jsRelease(dir_v);
    const dir = b.createDirectionalLight(dir_v, .{ .intensity = 1.35 });
    defer b.jsRelease(dir);
    const dd = b.JsTuple(&.{ f64, f64, f64 }).of(.{ 0.90, 0.90, 0.90 });
    defer b.jsRelease(dd);
    dir.setDiffuse(dd);
    b.addToScene(scene, .{ .LightBase = dir.asLightBase() });

    const p1 = b.JsTuple(&.{ f64, f64, f64 }).of(.{ 5.2, 4.2, 0.0 });
    defer b.jsRelease(p1);
    const light1 = b.createPointLight(p1, .{ .intensity = 10.0 });
    defer b.jsRelease(light1);
    const d1 = b.JsTuple(&.{ f64, f64, f64 }).of(.{ 1.0, 1.0, 1.0 });
    defer b.jsRelease(d1);
    light1.setDiffuse(d1);
    light1.setRange(16.0);
    b.addToScene(scene, .{ .LightBase = light1.asLightBase() });
    g_light1_pos = light1.getPosition();

    const p2 = b.JsTuple(&.{ f64, f64, f64 }).of(.{ -5.2, 3.4, 0.0 });
    defer b.jsRelease(p2);
    const light2 = b.createPointLight(p2, .{ .intensity = 7.5 });
    defer b.jsRelease(light2);
    const d2 = b.JsTuple(&.{ f64, f64, f64 }).of(.{ 0.78, 0.78, 0.78 });
    defer b.jsRelease(d2);
    light2.setDiffuse(d2);
    light2.setRange(16.0);
    b.addToScene(scene, .{ .LightBase = light2.asLightBase() });
    g_light2_pos = light2.getPosition();

    const floor_mat = mkPbr(0.09, 0.10, 0.88);
    defer b.jsRelease(floor_mat);
    const gopts = b.GroundOptions.create();
    defer b.jsRelease(gopts);
    gopts.setWidth(18.0);
    gopts.setHeight(18.0);
    const floor_m = b.createGround(g_engine, .{ .options = gopts });
    defer b.jsRelease(floor_m);
    floor_m.setMaterial(floor_mat.asMaterial());
    b.addToScene(scene, .{ .Mesh = floor_m });

    const shades = [_]f64{ 0.22, 0.44, 0.68, 0.88 };
    const col_mats = [_]b.PbrMaterialProps{
        mkPbr(shades[0], 0.35, 0.36),
        mkPbr(shades[1], 0.35, 0.36),
        mkPbr(shades[2], 0.35, 0.36),
        mkPbr(shades[3], 0.35, 0.36),
    };
    defer for (col_mats) |cm| b.jsRelease(cm);

    const bopts = b.BoxOptions.create();
    defer b.jsRelease(bopts);
    bopts.setWidth(0.72);
    bopts.setHeight(1.0);
    bopts.setDepth(0.72);

    var row: usize = 0;
    while (row < 6) : (row += 1) {
        var col: usize = 0;
        while (col < 6) : (col += 1) {
            const idx = row * 6 + col;
            const fx = @as(f64, @floatFromInt(col)) - 2.5;
            const fz = @as(f64, @floatFromInt(row)) - 2.5;
            const x = fx * 1.15;
            const z = fz * 1.15;
            g_col_x[idx] = x;
            g_col_z[idx] = z;
            g_col_dist[idx] = @sqrt(fx * fx + fz * fz);
            g_col_diag[idx] = fx - fz;

            const m = b.createBox(g_engine, .{ .options = .{ .BoxOptions = bopts } });
            defer b.jsRelease(m);
            g_col_pos[idx] = m.getPosition();
            g_col_scale[idx] = m.getScaling();
            g_col_pos[idx].set(x, 0.5, z);
            m.setMaterial(col_mats[(row + col) % 4].asMaterial());
            b.addToScene(scene, .{ .Mesh = m });
        }
    }

    const cprops = b.createPbrMaterial_props.create();
    defer b.jsRelease(cprops);
    const cbcf = b.JsTuple(&.{ f64, f64, f64, f64 }).of(.{ 0.92, 0.92, 0.92, 1.0 });
    defer b.jsRelease(cbcf);
    const cem = b.JsTuple(&.{ f64, f64, f64 }).of(.{ 0.16, 0.16, 0.16 });
    defer b.jsRelease(cem);
    cprops.setBaseColorFactor(cbcf);
    cprops.setMetallicFactor(0.50);
    cprops.setRoughnessFactor(0.15);
    const cc = b.ClearCoatProps.create();
    defer b.jsRelease(cc);
    cc.setIsEnabled(true);
    cc.setIntensity(1.0);
    cc.setRoughness(0.08);
    b.setPbrClearCoat(cprops, cc);
    b.setPbrEmissive(cprops, cem);
    const cap_mat = b.createPbrMaterial(cprops);
    defer b.jsRelease(cap_mat);

    const copts = b.CapsuleOptions.create();
    defer b.jsRelease(copts);
    copts.setHeight(1.15);
    copts.setRadius(0.22);
    copts.setTessellation(20.0);
    var k: usize = 0;
    while (k < 4) : (k += 1) {
        const cap = b.createCapsule(g_engine, .{ .options = copts });
        defer b.jsRelease(cap);
        g_cap_pos[k] = cap.getPosition();
        g_cap_rot[k] = cap.getRotation();
        cap.setMaterial(cap_mat.asMaterial());
        b.addToScene(scene, .{ .Mesh = cap });
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
