const std = @import("std");
const dom = @import("dom");
const ec = @import("echarts");

var meta_buf: [128]u8 = undefined;

fn colorObj(hex: []const u8) ec.JsValue {
    const o = ec.JsValue.object();
    o.set("color", hex);
    return o;
}

export fn start() void {
    const doc = dom.getDocument();
    defer dom.jsRelease(doc);
    const box = doc.getElementById("chart").?;
    defer dom.jsRelease(box);

    const chart = ec.init(box, .{});
    defer ec.jsRelease(chart);
    const c = chart.asEChartsType();

    const leg_ts = colorObj("#111111");
    defer ec.jsRelease(leg_ts);
    const legend = ec.JsValue.object();
    defer ec.jsRelease(legend);
    legend.set("top", @as(f64, 8));
    legend.set("textStyle", leg_ts);

    const tooltip = ec.JsValue.object();
    defer ec.jsRelease(tooltip);
    tooltip.set("trigger", "axis");

    const grid = ec.JsValue.object();
    defer ec.jsRelease(grid);
    grid.set("left", @as(f64, 48));
    grid.set("right", @as(f64, 24));
    grid.set("top", @as(f64, 48));
    grid.set("bottom", @as(f64, 36));

    const weeks = ec.JsArray.new();
    defer ec.jsRelease(weeks);
    for ([_][]const u8{ "W1", "W2", "W3", "W4", "W5", "W6", "W7", "W8" }) |w| {
        weeks.push(ec.JsValue.string(w));
    }
    const x_ls = colorObj("#111111");
    defer ec.jsRelease(x_ls);
    const x_line = ec.JsValue.object();
    defer ec.jsRelease(x_line);
    x_line.set("lineStyle", x_ls);
    const x_lbl = colorObj("#555555");
    defer ec.jsRelease(x_lbl);
    const x_axis = ec.JsValue.object();
    defer ec.jsRelease(x_axis);
    x_axis.set("type", "category");
    x_axis.set("boundaryGap", false);
    x_axis.set("data", weeks);
    x_axis.set("axisLine", x_line);
    x_axis.set("axisLabel", x_lbl);

    const y_ls = colorObj("#e5e5e5");
    defer ec.jsRelease(y_ls);
    const y_split = ec.JsValue.object();
    defer ec.jsRelease(y_split);
    y_split.set("lineStyle", y_ls);
    const y_lbl = colorObj("#555555");
    defer ec.jsRelease(y_lbl);
    const y_axis = ec.JsValue.object();
    defer ec.jsRelease(y_axis);
    y_axis.set("type", "value");
    y_axis.set("min", @as(f64, 0));
    y_axis.set("max", @as(f64, 100));
    y_axis.set("splitLine", y_split);
    y_axis.set("axisLabel", y_lbl);

    const d0 = ec.JsArray.new();
    defer ec.jsRelease(d0);
    for ([_]f64{ 28, 44, 39, 62, 58, 76, 81, 89 }) |v| d0.push(ec.JsValue.number(v));
    const ls0 = colorObj("#111111");
    defer ec.jsRelease(ls0);
    ls0.set("width", @as(f64, 2));
    const is0 = colorObj("#111111");
    defer ec.jsRelease(is0);
    const as0 = colorObj("rgba(17,17,17,0.12)");
    defer ec.jsRelease(as0);
    const s0 = ec.JsValue.object();
    defer ec.jsRelease(s0);
    s0.set("type", "line");
    s0.set("name", "Throughput");
    s0.set("smooth", true);
    s0.set("data", d0);
    s0.set("lineStyle", ls0);
    s0.set("itemStyle", is0);
    s0.set("areaStyle", as0);

    const d1 = ec.JsArray.new();
    defer ec.jsRelease(d1);
    for ([_]f64{ 35, 38, 42, 46, 50, 54, 58, 62 }) |v| d1.push(ec.JsValue.number(v));
    const ls1 = colorObj("#737373");
    defer ec.jsRelease(ls1);
    ls1.set("width", @as(f64, 1.5));
    ls1.set("type", "dashed");
    const is1 = colorObj("#737373");
    defer ec.jsRelease(is1);
    const s1 = ec.JsValue.object();
    defer ec.jsRelease(s1);
    s1.set("type", "line");
    s1.set("name", "Baseline");
    s1.set("smooth", false);
    s1.set("data", d1);
    s1.set("lineStyle", ls1);
    s1.set("itemStyle", is1);

    const series = ec.JsArray.new();
    defer ec.jsRelease(series);
    series.push(.{ .h = s0.h });
    series.push(.{ .h = s1.h });

    const opt = ec.EChartsCoreOption.create();
    defer ec.jsRelease(opt);
    const anim = ec.JsValue.boolean(false);
    defer ec.jsRelease(anim);
    const bg = ec.JsValue.string("#ffffff");
    defer ec.jsRelease(bg);
    opt.setAt("animation", anim);
    opt.setAt("backgroundColor", bg);
    opt.setAt("legend", legend);
    opt.setAt("tooltip", tooltip);
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
        std.fmt.bufPrint(&meta_buf, "Canvas: {d}x{d} \xC2\xB7 2 series (Throughput + Baseline)", .{ w, h }) catch "",
    );
}
