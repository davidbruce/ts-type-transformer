const std = @import("std");
const dom = @import("dom");
const ec = @import("echarts");

var meta_buf: [128]u8 = undefined;

fn objLineStyle(hex: []const u8) ec.JsValue {
    const ls = ec.JsValue.object();
    defer ec.jsRelease(ls);
    ls.set("color", hex);
    const wrap = ec.JsValue.object();
    wrap.set("lineStyle", ls);
    return wrap;
}

export fn start() void {
    const doc = dom.getDocument();
    defer dom.jsRelease(doc);
    const box = doc.getElementById("chart").?;
    defer dom.jsRelease(box);

    const chart = ec.init(box, .{});
    defer ec.jsRelease(chart);
    const c = chart.asEChartsType();

    const title_style = ec.JsValue.object();
    defer ec.jsRelease(title_style);
    title_style.set("color", "#111111");
    title_style.set("fontSize", @as(f64, 13));

    const title = ec.JsValue.object();
    defer ec.jsRelease(title);
    title.set("text", "Monthly Output");
    title.set("left", @as(f64, 16));
    title.set("top", @as(f64, 12));
    title.set("textStyle", title_style);

    const grid = ec.JsValue.object();
    defer ec.jsRelease(grid);
    grid.set("left", @as(f64, 48));
    grid.set("right", @as(f64, 24));
    grid.set("top", @as(f64, 48));
    grid.set("bottom", @as(f64, 36));

    const cats = ec.JsArray.new();
    defer ec.jsRelease(cats);
    for ([_][]const u8{ "Jan", "Feb", "Mar", "Apr", "May", "Jun" }) |m| {
        cats.push(ec.JsValue.string(m));
    }
    const x_line = objLineStyle("#111111");
    defer ec.jsRelease(x_line);
    const x_lbl = ec.JsValue.object();
    defer ec.jsRelease(x_lbl);
    x_lbl.set("color", "#555555");
    const x_axis = ec.JsValue.object();
    defer ec.jsRelease(x_axis);
    x_axis.set("type", "category");
    x_axis.set("data", cats);
    x_axis.set("axisLine", x_line);
    x_axis.set("axisLabel", x_lbl);

    const y_split = objLineStyle("#e5e5e5");
    defer ec.jsRelease(y_split);
    const y_lbl = ec.JsValue.object();
    defer ec.jsRelease(y_lbl);
    y_lbl.set("color", "#555555");
    const y_axis = ec.JsValue.object();
    defer ec.jsRelease(y_axis);
    y_axis.set("type", "value");
    y_axis.set("min", @as(f64, 0));
    y_axis.set("max", @as(f64, 100));
    y_axis.set("splitLine", y_split);
    y_axis.set("axisLabel", y_lbl);

    const vals = ec.JsArray.new();
    defer ec.jsRelease(vals);
    for ([_]f64{ 42, 68, 55, 84, 71, 93 }) |v| {
        vals.push(ec.JsValue.number(v));
    }
    const item_style = ec.JsValue.object();
    defer ec.jsRelease(item_style);
    item_style.set("color", "#111111");

    const s0 = ec.JsValue.object();
    defer ec.jsRelease(s0);
    s0.set("type", "bar");
    s0.set("name", "Units");
    s0.set("barWidth", "46%");
    s0.set("data", vals);
    s0.set("itemStyle", item_style);

    const series = ec.JsArray.new();
    defer ec.jsRelease(series);
    series.push(.{ .h = s0.h });

    const opt = ec.EChartsCoreOption.create();
    defer ec.jsRelease(opt);
    const bg = ec.JsValue.string("#ffffff");
    defer ec.jsRelease(bg);
    const anim = ec.JsValue.boolean(false);
    defer ec.jsRelease(anim);
    opt.setAt("animation", anim);
    opt.setAt("backgroundColor", bg);
    opt.setAt("title", title);
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
        std.fmt.bufPrint(&meta_buf, "Canvas: {d}x{d} \xC2\xB7 6 bars \xC2\xB7 Monochrome #111111", .{ w, h }) catch "",
    );
}
