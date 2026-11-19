const std = @import("std");
const dom = @import("dom");
const ec = @import("echarts");

var meta_buf: [128]u8 = undefined;

fn pushSlice(arr: ec.JsArray, name: []const u8, value: f64) void {
    const item = ec.JsValue.object();
    defer ec.jsRelease(item);
    item.set("name", name);
    item.set("value", value);
    arr.push(item);
}

export fn start() void {
    const doc = dom.getDocument();
    defer dom.jsRelease(doc);
    const box = doc.getElementById("chart").?;
    defer dom.jsRelease(box);

    const chart = ec.init(box, .{});
    defer ec.jsRelease(chart);
    const c = chart.asEChartsType();

    const palette = ec.JsArray.new();
    defer ec.jsRelease(palette);
    for ([_][]const u8{ "#111111", "#404040", "#737373", "#a3a3a3", "#d4d4d4" }) |hex| {
        palette.push(ec.JsValue.string(hex));
    }

    const leg_ts = ec.JsValue.object();
    defer ec.jsRelease(leg_ts);
    leg_ts.set("color", "#111111");
    const legend = ec.JsValue.object();
    defer ec.jsRelease(legend);
    legend.set("bottom", @as(f64, 8));
    legend.set("textStyle", leg_ts);

    const tooltip = ec.JsValue.object();
    defer ec.jsRelease(tooltip);
    tooltip.set("trigger", "item");

    const radius = ec.JsArray.new();
    defer ec.jsRelease(radius);
    radius.push(ec.JsValue.string("44%"));
    radius.push(ec.JsValue.string("72%"));

    const center = ec.JsArray.new();
    defer ec.jsRelease(center);
    center.push(ec.JsValue.string("50%"));
    center.push(ec.JsValue.string("46%"));

    const item_style = ec.JsValue.object();
    defer ec.jsRelease(item_style);
    item_style.set("borderColor", "#ffffff");
    item_style.set("borderWidth", @as(f64, 2));

    const lbl = ec.JsValue.object();
    defer ec.jsRelease(lbl);
    lbl.set("color", "#111111");

    const data = ec.JsArray.new();
    defer ec.jsRelease(data);
    pushSlice(data, "Runtime", 34);
    pushSlice(data, "Bindings", 26);
    pushSlice(data, "Glue", 18);
    pushSlice(data, "Parser", 14);
    pushSlice(data, "Tests", 8);

    const s0 = ec.JsValue.object();
    defer ec.jsRelease(s0);
    s0.set("type", "pie");
    s0.set("name", "Subsystems");
    s0.set("radius", radius);
    s0.set("center", center);
    s0.set("itemStyle", item_style);
    s0.set("label", lbl);
    s0.set("data", data);

    const series = ec.JsArray.new();
    defer ec.jsRelease(series);
    series.push(s0);

    const opt = ec.EChartsCoreOption.create();
    defer ec.jsRelease(opt);
    const anim = ec.JsValue.boolean(false);
    defer ec.jsRelease(anim);
    const bg = ec.JsValue.string("#ffffff");
    defer ec.jsRelease(bg);
    opt.setAt("animation", anim);
    opt.setAt("backgroundColor", bg);
    opt.setAt("color", .{ .h = palette.h });
    opt.setAt("legend", legend);
    opt.setAt("tooltip", tooltip);
    opt.setAt("series", .{ .h = series.h });
    c.setOption(opt, .{});

    const w: i32 = @intFromFloat(c.getWidth());
    const h: i32 = @intFromFloat(c.getHeight());
    const meta_el = doc.getElementById("meta").?;
    defer dom.jsRelease(meta_el);
    meta_el.asNode().setTextContent(
        std.fmt.bufPrint(&meta_buf, "Canvas: {d}x{d} \xC2\xB7 5 monochrome segments \xC2\xB7 Total 100", .{ w, h }) catch "",
    );
}
