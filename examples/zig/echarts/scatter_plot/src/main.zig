const std = @import("std");
const dom = @import("dom");
const ec = @import("echarts");

var meta_buf: [128]u8 = undefined;

fn makeValueAxis() ec.JsValue {
    const ls = ec.JsValue.object();
    defer ec.jsRelease(ls);
    ls.set("color", "#e5e5e5");
    const split = ec.JsValue.object();
    defer ec.jsRelease(split);
    split.set("lineStyle", ls);
    const lbl = ec.JsValue.object();
    defer ec.jsRelease(lbl);
    lbl.set("color", "#555555");
    const ax = ec.JsValue.object();
    ax.set("type", "value");
    ax.set("min", @as(f64, 0));
    ax.set("max", @as(f64, 100));
    ax.set("splitLine", split);
    ax.set("axisLabel", lbl);
    return ax;
}

fn makePts(pts: []const [2]f64) ec.JsArray {
    const arr = ec.JsArray.new();
    for (pts) |pt| {
        const pair = ec.JsArray.new();
        defer ec.jsRelease(pair);
        pair.push(ec.JsValue.number(pt[0]));
        pair.push(ec.JsValue.number(pt[1]));
        arr.push(.{ .h = pair.h });
    }
    return arr;
}

export fn start() void {
    const doc = dom.getDocument();
    defer dom.jsRelease(doc);
    const box = doc.getElementById("chart").?;
    defer dom.jsRelease(box);

    const chart = ec.init(box, .{});
    defer ec.jsRelease(chart);
    const c = chart.asEChartsType();

    const leg_ts = ec.JsValue.object();
    defer ec.jsRelease(leg_ts);
    leg_ts.set("color", "#111111");
    const legend = ec.JsValue.object();
    defer ec.jsRelease(legend);
    legend.set("top", @as(f64, 8));
    legend.set("textStyle", leg_ts);

    const grid = ec.JsValue.object();
    defer ec.jsRelease(grid);
    grid.set("left", @as(f64, 48));
    grid.set("right", @as(f64, 24));
    grid.set("top", @as(f64, 48));
    grid.set("bottom", @as(f64, 36));

    const x_axis = makeValueAxis();
    defer ec.jsRelease(x_axis);
    const y_axis = makeValueAxis();
    defer ec.jsRelease(y_axis);

    const ca_pts = [_][2]f64{ .{ 14, 22 }, .{ 21, 31 }, .{ 28, 27 }, .{ 33, 42 }, .{ 40, 48 }, .{ 46, 53 }, .{ 54, 61 }, .{ 62, 59 }, .{ 71, 74 }, .{ 82, 85 } };
    const cb_pts = [_][2]f64{ .{ 18, 64 }, .{ 25, 72 }, .{ 31, 68 }, .{ 39, 79 }, .{ 47, 65 }, .{ 55, 81 }, .{ 64, 77 }, .{ 72, 88 }, .{ 79, 82 }, .{ 88, 92 } };
    const data_a = makePts(&ca_pts);
    defer ec.jsRelease(data_a);
    const data_b = makePts(&cb_pts);
    defer ec.jsRelease(data_b);

    const is_a = ec.JsValue.object();
    defer ec.jsRelease(is_a);
    is_a.set("color", "#111111");
    const s0 = ec.JsValue.object();
    defer ec.jsRelease(s0);
    s0.set("type", "scatter");
    s0.set("name", "Cluster A");
    s0.set("symbol", "circle");
    s0.set("symbolSize", @as(f64, 10));
    s0.set("itemStyle", is_a);
    s0.set("data", data_a);

    const is_b = ec.JsValue.object();
    defer ec.jsRelease(is_b);
    is_b.set("color", "#ffffff");
    is_b.set("borderColor", "#111111");
    is_b.set("borderWidth", @as(f64, 2));
    const s1 = ec.JsValue.object();
    defer ec.jsRelease(s1);
    s1.set("type", "scatter");
    s1.set("name", "Cluster B");
    s1.set("symbol", "rect");
    s1.set("symbolSize", @as(f64, 10));
    s1.set("itemStyle", is_b);
    s1.set("data", data_b);

    const series = ec.JsArray.new();
    defer ec.jsRelease(series);
    series.push(s0);
    series.push(s1);

    const opt = ec.EChartsCoreOption.create();
    defer ec.jsRelease(opt);
    const anim = ec.JsValue.boolean(false);
    defer ec.jsRelease(anim);
    const bg = ec.JsValue.string("#ffffff");
    defer ec.jsRelease(bg);
    opt.setAt("animation", anim);
    opt.setAt("backgroundColor", bg);
    opt.setAt("legend", legend);
    opt.setAt("grid", grid);
    opt.setAt("xAxis", x_axis);
    opt.setAt("yAxis", y_axis);
    opt.setAt("series", .{ .h = series.h });
    c.setOption(opt, .{});

    const w: i32 = @intFromFloat(c.getWidth());
    const h: i32 = @intFromFloat(c.getHeight());
    const meta_el = doc.getElementById("meta").?;
    defer dom.jsRelease(meta_el);
    meta_el.asNode().setTextContent(
        std.fmt.bufPrint(&meta_buf, "Canvas: {d}x{d} \xC2\xB7 20 data points across 2 clusters", .{ w, h }) catch "",
    );
}
