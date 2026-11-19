const std = @import("std");
const dom = @import("dom");
const ec = @import("echarts");

var meta_buf: [128]u8 = undefined;

fn colorObj(hex: []const u8) ec.JsValue {
    const o = ec.JsValue.object();
    o.set("color", hex);
    return o;
}

fn lineWrap(hex: []const u8) ec.JsValue {
    const ls = colorObj(hex);
    defer ec.jsRelease(ls);
    const w = ec.JsValue.object();
    w.set("lineStyle", ls);
    return w;
}

fn numArr(vals: []const f64) ec.JsArray {
    const a = ec.JsArray.new();
    for (vals) |v| a.push(ec.JsValue.number(v));
    return a;
}

export fn start() void {
    const doc = dom.getDocument();
    defer dom.jsRelease(doc);
    const radar_box = doc.getElementById("chart-radar").?;
    defer dom.jsRelease(radar_box);
    const gauge_box = doc.getElementById("chart-gauge").?;
    defer dom.jsRelease(gauge_box);

    const radar = ec.init(radar_box, .{});
    defer ec.jsRelease(radar);
    const gauge = ec.init(gauge_box, .{});
    defer ec.jsRelease(gauge);
    const rc = radar.asEChartsType();
    const gc = gauge.asEChartsType();

    // ---- Radar ----
    const r_leg_ts = colorObj("#111111");
    defer ec.jsRelease(r_leg_ts);
    const r_leg = ec.JsValue.object();
    defer ec.jsRelease(r_leg);
    r_leg.set("bottom", @as(f64, 6));
    r_leg.set("textStyle", r_leg_ts);

    const r_an = colorObj("#111111");
    defer ec.jsRelease(r_an);
    const r_sl = lineWrap("#d4d4d4");
    defer ec.jsRelease(r_sl);
    const r_sa_cols = ec.JsArray.new();
    defer ec.jsRelease(r_sa_cols);
    r_sa_cols.push(ec.JsValue.string("#ffffff"));
    r_sa_cols.push(ec.JsValue.string("#f7f7f7"));
    const r_sa_as = ec.JsValue.object();
    defer ec.jsRelease(r_sa_as);
    r_sa_as.set("color", r_sa_cols);
    const r_sa = ec.JsValue.object();
    defer ec.jsRelease(r_sa);
    r_sa.set("areaStyle", r_sa_as);
    const r_al = lineWrap("#b5b5b5");
    defer ec.jsRelease(r_al);

    const inds = ec.JsArray.new();
    defer ec.jsRelease(inds);
    for ([_][]const u8{ "Throughput", "Memory", "Startup", "Type Safety", "Bundle" }) |name| {
        const ind = ec.JsValue.object();
        defer ec.jsRelease(ind);
        ind.set("name", name);
        ind.set("max", @as(f64, 100));
        inds.push(ind);
    }

    const r_cfg = ec.JsValue.object();
    defer ec.jsRelease(r_cfg);
    r_cfg.set("radius", "62%");
    r_cfg.set("axisName", r_an);
    r_cfg.set("splitLine", r_sl);
    r_cfg.set("splitArea", r_sa);
    r_cfg.set("axisLine", r_al);
    r_cfg.set("indicator", inds);

    const va = numArr(&.{ 92, 86, 88, 96, 84 });
    defer ec.jsRelease(va);
    const ls0 = colorObj("#111111");
    defer ec.jsRelease(ls0);
    ls0.set("width", @as(f64, 2));
    const is0 = colorObj("#111111");
    defer ec.jsRelease(is0);
    const as0 = colorObj("rgba(17,17,17,0.12)");
    defer ec.jsRelease(as0);
    const item0 = ec.JsValue.object();
    defer ec.jsRelease(item0);
    item0.set("name", "WASM Target");
    item0.set("value", va);
    item0.set("lineStyle", ls0);
    item0.set("itemStyle", is0);
    item0.set("areaStyle", as0);

    const vb = numArr(&.{ 70, 68, 74, 72, 66 });
    defer ec.jsRelease(vb);
    const ls1 = colorObj("#737373");
    defer ec.jsRelease(ls1);
    ls1.set("width", @as(f64, 1.5));
    ls1.set("type", "dashed");
    const is1 = colorObj("#737373");
    defer ec.jsRelease(is1);
    const item1 = ec.JsValue.object();
    defer ec.jsRelease(item1);
    item1.set("name", "Baseline");
    item1.set("value", vb);
    item1.set("lineStyle", ls1);
    item1.set("itemStyle", is1);

    const r_data = ec.JsArray.new();
    defer ec.jsRelease(r_data);
    r_data.push(item0);
    r_data.push(item1);
    const rs0 = ec.JsValue.object();
    defer ec.jsRelease(rs0);
    rs0.set("type", "radar");
    rs0.set("data", r_data);
    const r_series = ec.JsArray.new();
    defer ec.jsRelease(r_series);
    r_series.push(rs0);

    const anim = ec.JsValue.boolean(false);
    defer ec.jsRelease(anim);
    const bg = ec.JsValue.string("#ffffff");
    defer ec.jsRelease(bg);

    const radar_opt = ec.EChartsCoreOption.create();
    defer ec.jsRelease(radar_opt);
    radar_opt.setAt("animation", anim);
    radar_opt.setAt("backgroundColor", bg);
    radar_opt.setAt("legend", r_leg);
    radar_opt.setAt("radar", r_cfg);
    radar_opt.setAt("series", .{ .h = r_series.h });
    rc.setOption(radar_opt, .{});

    // ---- Gauge ----
    const g_prog_is = colorObj("#111111");
    defer ec.jsRelease(g_prog_is);
    const g_prog = ec.JsValue.object();
    defer ec.jsRelease(g_prog);
    g_prog.set("show", true);
    g_prog.set("width", @as(f64, 14));
    g_prog.set("itemStyle", g_prog_is);

    const stop = ec.JsArray.new();
    defer ec.jsRelease(stop);
    stop.push(ec.JsValue.number(1));
    stop.push(ec.JsValue.string("#e5e5e5"));
    const al_stops = ec.JsArray.new();
    defer ec.jsRelease(al_stops);
    al_stops.push(.{ .h = stop.h });
    const g_al_ls = ec.JsValue.object();
    defer ec.jsRelease(g_al_ls);
    g_al_ls.set("width", @as(f64, 14));
    g_al_ls.set("color", al_stops);
    const g_al = ec.JsValue.object();
    defer ec.jsRelease(g_al);
    g_al.set("lineStyle", g_al_ls);

    const g_tick = lineWrap("#737373");
    defer ec.jsRelease(g_tick);
    const g_split = lineWrap("#111111");
    defer ec.jsRelease(g_split);
    const g_lbl = colorObj("#555555");
    defer ec.jsRelease(g_lbl);
    const g_ptr_is = colorObj("#111111");
    defer ec.jsRelease(g_ptr_is);
    const g_ptr = ec.JsValue.object();
    defer ec.jsRelease(g_ptr);
    g_ptr.set("itemStyle", g_ptr_is);

    const g_det = colorObj("#111111");
    defer ec.jsRelease(g_det);
    g_det.set("valueAnimation", false);
    g_det.set("formatter", "{value}%");
    g_det.set("fontSize", @as(f64, 22));

    const g_title = colorObj("#555555");
    defer ec.jsRelease(g_title);
    g_title.set("fontSize", @as(f64, 12));

    const g_datum = ec.JsValue.object();
    defer ec.jsRelease(g_datum);
    g_datum.set("value", @as(f64, 84));
    g_datum.set("name", "Score");
    const g_data = ec.JsArray.new();
    defer ec.jsRelease(g_data);
    g_data.push(g_datum);

    const gs0 = ec.JsValue.object();
    defer ec.jsRelease(gs0);
    gs0.set("type", "gauge");
    gs0.set("min", @as(f64, 0));
    gs0.set("max", @as(f64, 100));
    gs0.set("progress", g_prog);
    gs0.set("axisLine", g_al);
    gs0.set("axisTick", g_tick);
    gs0.set("splitLine", g_split);
    gs0.set("axisLabel", g_lbl);
    gs0.set("pointer", g_ptr);
    gs0.set("detail", g_det);
    gs0.set("title", g_title);
    gs0.set("data", g_data);
    const g_series = ec.JsArray.new();
    defer ec.jsRelease(g_series);
    g_series.push(gs0);

    const gauge_opt = ec.EChartsCoreOption.create();
    defer ec.jsRelease(gauge_opt);
    gauge_opt.setAt("animation", anim);
    gauge_opt.setAt("backgroundColor", bg);
    gauge_opt.setAt("series", .{ .h = g_series.h });
    gc.setOption(gauge_opt, .{});

    const w1: i32 = @intFromFloat(rc.getWidth());
    const h1: i32 = @intFromFloat(rc.getHeight());
    const w2: i32 = @intFromFloat(gc.getWidth());
    const h2: i32 = @intFromFloat(gc.getHeight());
    const meta_el = doc.getElementById("meta").?;
    defer dom.jsRelease(meta_el);
    meta_el.asNode().setTextContent(
        std.fmt.bufPrint(&meta_buf, "Radar: {d}x{d} \xC2\xB7 Gauge: {d}x{d} \xC2\xB7 Score 84%", .{ w1, h1, w2, h2 }) catch "",
    );
}
