const std = @import("std");
const dom = @import("dom");
const b = @import("babylonjs_lite");

var g_canvas: dom.HTMLCanvasElement = undefined;
var g_engine: b.EngineContext = undefined;
var g_mono_rot: b.EulerProxy = undefined;
var g_core_rot: b.EulerProxy = undefined;
var g_outer_rot: b.EulerProxy = undefined;
var g_inner_rot: b.EulerProxy = undefined;
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
    g_mono_rot.setY(g_elapsed * 0.35);
    g_mono_rot.setX(@sin(g_elapsed * 0.3) * 0.20);
    g_core_rot.setY(-g_elapsed * 0.90);
    g_core_rot.setZ(g_elapsed * 0.50);
    g_outer_rot.setX(g_elapsed * 0.42);
    g_outer_rot.setY(g_elapsed * 0.25);
    g_inner_rot.setZ(-g_elapsed * 0.58);
    g_inner_rot.setX(g_elapsed * 0.31);
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
    clear.setR(0.06);
    clear.setG(0.06);
    clear.setB(0.06);
    clear.setA(1.0);
    scene.setClearColor(clear);

    const cam_target = b.vec3(0.0, 2.1, 0.0);
    defer b.jsRelease(cam_target);
    const camera = b.createArcRotateCamera(std.math.pi / 4.2, std.math.pi / 2.9, 8.8, cam_target);
    defer b.jsRelease(camera);
    b.addToScene(scene, .{ .Camera = camera.asCamera() });
    const ctrl = b.attachControl(camera, g_canvas, .{ .scene = scene });
    defer b.jsRelease(ctrl);

    const hemi_dir = b.JsTuple(&.{ f64, f64, f64 }).of(.{ 0.0, 1.0, 0.2 });
    defer b.jsRelease(hemi_dir);
    const hemi = b.createHemisphericLight(.{ .direction = hemi_dir, .intensity = 0.72 });
    defer b.jsRelease(hemi);
    const hdc = b.JsTuple(&.{ f64, f64, f64 }).of(.{ 0.85, 0.85, 0.85 });
    defer b.jsRelease(hdc);
    const hgc = b.JsTuple(&.{ f64, f64, f64 }).of(.{ 0.14, 0.14, 0.14 });
    defer b.jsRelease(hgc);
    hemi.setDiffuseColor(hdc);
    hemi.setGroundColor(hgc);
    b.addToScene(scene, .{ .LightBase = hemi.asLightBase() });

    const dir_v = b.JsTuple(&.{ f64, f64, f64 }).of(.{ -1.2, -2.0, -1.1 });
    defer b.jsRelease(dir_v);
    const dir = b.createDirectionalLight(dir_v, .{ .intensity = 2.1 });
    defer b.jsRelease(dir);
    const dd = b.JsTuple(&.{ f64, f64, f64 }).of(.{ 1.0, 1.0, 1.0 });
    defer b.jsRelease(dd);
    dir.setDiffuse(dd);
    b.addToScene(scene, .{ .LightBase = dir.asLightBase() });

    const kl_p = b.JsTuple(&.{ f64, f64, f64 }).of(.{ 3.2, 3.6, 2.4 });
    defer b.jsRelease(kl_p);
    const key_light = b.createPointLight(kl_p, .{ .intensity = 8.0 });
    defer b.jsRelease(key_light);
    const kld = b.JsTuple(&.{ f64, f64, f64 }).of(.{ 0.92, 0.92, 0.92 });
    defer b.jsRelease(kld);
    key_light.setDiffuse(kld);
    key_light.setRange(14.0);
    b.addToScene(scene, .{ .LightBase = key_light.asLightBase() });

    const floor_mat = mkPbr(0.09, 0.10, 0.88);
    defer b.jsRelease(floor_mat);
    const gopts = b.GroundOptions.create();
    defer b.jsRelease(gopts);
    gopts.setWidth(16.0);
    gopts.setHeight(16.0);
    const floor_m = b.createGround(g_engine, .{ .options = gopts });
    defer b.jsRelease(floor_m);
    floor_m.setMaterial(floor_mat.asMaterial());
    b.addToScene(scene, .{ .Mesh = floor_m });

    const disc_mat = mkPbr(0.22, 0.25, 0.65);
    defer b.jsRelease(disc_mat);
    const dopts = b.DiscOptions.create();
    defer b.jsRelease(dopts);
    dopts.setRadius(2.6);
    dopts.setTessellation(48.0);
    const disc = b.createDisc(g_engine, .{ .options = dopts });
    defer b.jsRelease(disc);
    const disc_rot = disc.getRotation();
    defer b.jsRelease(disc_rot);
    disc_rot.setX(std.math.pi / 2.0);
    const disc_pos = disc.getPosition();
    defer b.jsRelease(disc_pos);
    disc_pos.set(0.0, 0.03, 0.0);
    disc.setMaterial(disc_mat.asMaterial());
    b.addToScene(scene, .{ .Mesh = disc });

    const sbopts = b.BoxOptions.create();
    defer b.jsRelease(sbopts);
    sbopts.setSize(2.1);
    const src_box = b.createBox(g_engine, .{ .options = .{ .BoxOptions = sbopts } });
    defer b.jsRelease(src_box);
    const copts = b.CylinderOptions.create();
    defer b.jsRelease(copts);
    copts.setHeight(3.0);
    copts.setDiameter(1.18);
    copts.setTessellation(16.0);
    const cyl_y = b.createCylinder(g_engine, .{ .options = copts });
    defer b.jsRelease(cyl_y);
    const cyl_x = b.createCylinder(g_engine, .{ .options = copts });
    defer b.jsRelease(cyl_x);
    const rx = cyl_x.getRotation();
    defer b.jsRelease(rx);
    rx.setZ(std.math.pi / 2.0);
    const cyl_z = b.createCylinder(g_engine, .{ .options = copts });
    defer b.jsRelease(cyl_z);
    const rz = cyl_z.getRotation();
    defer b.jsRelease(rz);
    rz.setX(std.math.pi / 2.0);

    const cb = b.createCsgFromMesh(src_box, .{});
    defer b.jsRelease(cb);
    const cy = b.createCsgFromMesh(cyl_y, .{});
    defer b.jsRelease(cy);
    const cx = b.createCsgFromMesh(cyl_x, .{});
    defer b.jsRelease(cx);
    const cz = b.createCsgFromMesh(cyl_z, .{});
    defer b.jsRelease(cz);
    const s1 = b.csgSubtract(cb, cy);
    defer b.jsRelease(s1);
    const s2 = b.csgSubtract(s1, cx);
    defer b.jsRelease(s2);
    const s3 = b.csgSubtract(s2, cz);
    defer b.jsRelease(s3);
    const monolith = b.createMeshFromCsg(g_engine, s3, .{ .name = "monolith" });
    defer b.jsRelease(monolith);
    const mpos = monolith.getPosition();
    defer b.jsRelease(mpos);
    mpos.set(0.0, 2.15, 0.0);
    g_mono_rot = monolith.getRotation();

    const mprops = b.createPbrMaterial_props.create();
    defer b.jsRelease(mprops);
    const mbcf = b.JsTuple(&.{ f64, f64, f64, f64 }).of(.{ 0.86, 0.86, 0.86, 1.0 });
    defer b.jsRelease(mbcf);
    mprops.setBaseColorFactor(mbcf);
    mprops.setMetallicFactor(0.65);
    mprops.setRoughnessFactor(0.24);
    const cc = b.ClearCoatProps.create();
    defer b.jsRelease(cc);
    cc.setIsEnabled(true);
    cc.setIntensity(0.90);
    cc.setRoughness(0.10);
    b.setPbrClearCoat(mprops, cc);
    const mono_mat = b.createPbrMaterial(mprops);
    defer b.jsRelease(mono_mat);
    monolith.setMaterial(mono_mat.asMaterial());
    b.addToScene(scene, .{ .Mesh = monolith });

    const cbopts = b.BoxOptions.create();
    defer b.jsRelease(cbopts);
    cbopts.setSize(0.82);
    const core_box = b.createBox(g_engine, .{ .options = .{ .BoxOptions = cbopts } });
    defer b.jsRelease(core_box);
    const csopts = b.SphereOptions.create();
    defer b.jsRelease(csopts);
    csopts.setDiameter(1.02);
    csopts.setSegments(14.0);
    const core_sphere = b.createSphere(g_engine, .{ .options = csopts });
    defer b.jsRelease(core_sphere);
    const cbox_s = b.createCsgFromMesh(core_box, .{});
    defer b.jsRelease(cbox_s);
    const csph_s = b.createCsgFromMesh(core_sphere, .{});
    defer b.jsRelease(csph_s);
    const core_s = b.csgIntersect(cbox_s, csph_s);
    defer b.jsRelease(core_s);
    const core = b.createMeshFromCsg(g_engine, core_s, .{ .name = "core" });
    defer b.jsRelease(core);
    const core_pos = core.getPosition();
    defer b.jsRelease(core_pos);
    core_pos.set(0.0, 2.15, 0.0);
    g_core_rot = core.getRotation();
    const core_mat = mkPbr(0.18, 0.85, 0.16);
    defer b.jsRelease(core_mat);
    core.setMaterial(core_mat.asMaterial());
    b.addToScene(scene, .{ .Mesh = core });

    const otopts = b.TorusOptions.create();
    defer b.jsRelease(otopts);
    otopts.setDiameter(4.2);
    otopts.setThickness(0.09);
    otopts.setTessellation(48.0);
    const outer_ring = b.createTorus(g_engine, .{ .options = otopts });
    defer b.jsRelease(outer_ring);
    const opos = outer_ring.getPosition();
    defer b.jsRelease(opos);
    opos.set(0.0, 2.15, 0.0);
    g_outer_rot = outer_ring.getRotation();
    const outer_mat = mkPbr(0.55, 0.80, 0.25);
    defer b.jsRelease(outer_mat);
    outer_ring.setMaterial(outer_mat.asMaterial());
    b.addToScene(scene, .{ .Mesh = outer_ring });

    const itopts = b.TorusOptions.create();
    defer b.jsRelease(itopts);
    itopts.setDiameter(3.55);
    itopts.setThickness(0.07);
    itopts.setTessellation(48.0);
    const inner_ring = b.createTorus(g_engine, .{ .options = itopts });
    defer b.jsRelease(inner_ring);
    const ipos = inner_ring.getPosition();
    defer b.jsRelease(ipos);
    ipos.set(0.0, 2.15, 0.0);
    g_inner_rot = inner_ring.getRotation();
    const inner_mat = mkPbr(0.88, 0.40, 0.30);
    defer b.jsRelease(inner_mat);
    inner_ring.setMaterial(inner_mat.asMaterial());
    b.addToScene(scene, .{ .Mesh = inner_ring });

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
