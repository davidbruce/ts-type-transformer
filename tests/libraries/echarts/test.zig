// ECharts from Zig, in a page: a bar chart on the page's #chart, read back, converted, with events. Every expect must hold.
const std = @import("std");
const b = @import("bindings.zig");
const ec = b.echarts;

extern "env" fn check(ok: i32, line: u32) void;

fn expect(ok: bool, src: std.builtin.SourceLocation) void {
    check(@intFromBool(ok), src.line);
}

fn num(x: f64) b.JsValue {
    return b.JsValue.number(x);
}

var fired: u32 = 0;
fn onPing(ctx: ?*anyopaque, _: b.JsArray) ?bool {
    const n: *u32 = @ptrCast(@alignCast(ctx));
    n.* += 1;
    return null;
}

// FINDING 16: BarSeriesOption.create() makes `{}`; the discriminant `type: "bar"` is only a Zig constant
// (BarSeriesOption.type), never written to the object, and there is no setType. ECharts throws
// "Unknown series undefined" without it. The property is written here through EChartsCoreOption.setAt,
// an index-signature type whose handle is reinterpreted.
fn stampType(h: u32, name: []const u8) void {
    const any: ec.EChartsCoreOption = .{ .h = h };
    any.setAt("type", b.JsValue.string(name));
}

// FINDING 22: which namespace a name is in is not predictable: echarts.EChartsOption and
// echarts.BarSeriesOption are in the package's struct, but XAXisComponentOption_xAxis (a nested/synthetic
// type) is at the top of bindings, so this program mixes `ec.` and `b.`.
export fn run() void {
    // A chart on the page's #chart, 400 by 300.
    const chart = ec.init(b.test_page.element("chart"), .{});
    const c = chart.asEChartsType();
    expect(c.getWidth() == 400 and c.getHeight() == 300, @src());
    expect(!c.isDisposed(), @src());
    expect(b.jsSame(c.getDom(), b.test_page.element("chart")), @src());
    expect(c.getId().len() > 0, @src());
    expect(!c.isSSR(), @src());
    expect(c.getDevicePixelRatio() > 0, @src());

    // The same element has one chart.
    const again = ec.getInstanceByDom(b.test_page.element("chart"));
    expect(again != null and b.jsSame(again.?, chart), @src());

    // A bar chart: a title, a category axis, a value axis and one series.
    const option = ec.EChartsOption.create();
    const title = ec.TitleComponentOption.create();
    title.setText("Sales");
    option.setTitle(.{ .TitleComponentOption = title });
    const x_axis = b.XAXisComponentOption_xAxis.create();
    const labels = b.JsArrayOf(b.string_or_number_or_CategoryAxisBaseOption_data).new();
    labels.push(.{ .string = b.JsString.from("A") });
    labels.push(.{ .string = b.JsString.from("B") });
    labels.push(.{ .string = b.JsString.from("C") });
    x_axis.setData(labels);
    option.setXAxis(.{ .XAXisComponentOption_xAxis = x_axis });
    option.setYAxis(.{ .YAXisComponentOption_yAxis_value = b.YAXisComponentOption_yAxis_value.create() });
    const bars = ec.BarSeriesOption.create();
    stampType(bars.h, "bar");
    bars.setName(.{ .string = b.JsString.from("sales") });
    const values = b.JsArray.new();
    values.push(num(5));
    values.push(num(20));
    values.push(num(36));
    bars.setData(values);
    bars.setAnimation(false);
    option.setSeries(.{ .BarSeriesOption = bars });
    c.setOption(option.asEChartsCoreOption(), .{});

    // Read back: the option the chart holds is merged and normalised.
    const held = c.getOption();
    const series: b.JsArray = .{ .h = held.getAt("series").h };
    expect(series.len() == 1, @src());
    const first: ec.BarSeriesOption = .{ .h = series.get(0).h };
    expect(first.getName().?.string.eql("sales"), @src());
    const data: b.JsArray = first.getData().?;
    expect(data.len() == 3 and data.get(1).toNumber() == 20 and data.get(2).toNumber() == 36, @src());
    expect(first.getAnimation() == false, @src());
    const titles: b.JsArray = .{ .h = held.getAt("title").h };
    expect(titles.len() == 1, @src());
    const held_title: ec.TitleComponentOption = .{ .h = titles.get(0).h };
    expect(held_title.getText().?.eql("Sales"), @src());

    // Pixel conversion: the category axis spreads A, B, C across the grid, so B is in the middle third,
    // and the value axis maps 36 higher on the screen (a smaller y) than 5.
    const x_finder = ec.ModelFinderObject.create();
    x_finder.setXAxisIndex(num(0));
    const y_finder = ec.ModelFinderObject.create();
    y_finder.setYAxisIndex(num(0));
    const xa = c.convertToPixel(.{ .ModelFinderObject = x_finder }, .{ .string = b.JsString.from("A") });
    const xb = c.convertToPixel(.{ .ModelFinderObject = x_finder }, .{ .string = b.JsString.from("B") });
    const xc = c.convertToPixel(.{ .ModelFinderObject = x_finder }, .{ .string = b.JsString.from("C") });
    expect(xa < xb and xb < xc, @src());
    expect(xa > 0 and xc < 400, @src());
    expect(@abs((xb - xa) - (xc - xb)) < 1e-6, @src());
    const y5 = c.convertToPixel(.{ .ModelFinderObject = y_finder }, .{ .number = 5 });
    const y36 = c.convertToPixel(.{ .ModelFinderObject = y_finder }, .{ .number = 36 });
    expect(y36 < y5 and y5 < 300 and y36 > 0, @src());
    // And back: a pixel position converts to the value.
    const back = c.convertFromPixel(.{ .ModelFinderObject = y_finder }, y5);
    expect(@abs(back - 5) < 1e-6, @src());

    // Updating the option merges: the same series gets new data.
    const next = ec.EChartsOption.create();
    const next_bars = ec.BarSeriesOption.create();
    stampType(next_bars.h, "bar");
    next_bars.setName(.{ .string = b.JsString.from("sales") });
    const more = b.JsArray.new();
    more.push(num(1));
    more.push(num(2));
    more.push(num(3));
    more.push(num(4));
    next_bars.setData(more);
    next.setSeries(.{ .BarSeriesOption = next_bars });
    c.setOption(next.asEChartsCoreOption(), .{});
    const merged: b.JsArray = .{ .h = c.getOption().getAt("series").h };
    const merged_first: ec.BarSeriesOption = .{ .h = merged.get(0).h };
    expect(merged_first.getData().?.len() == 4, @src());
    // The title survived the merge.
    const kept: b.JsArray = .{ .h = c.getOption().getAt("title").h };
    expect(kept.len() == 1, @src());

    // Resizing the chart: the container stays 400 by 300 but resize takes explicit sizes.
    const size = ec.ResizeOpts.create();
    size.setWidth(.{ .number = 200 });
    size.setHeight(.{ .number = 150 });
    c.resize(size);
    expect(c.getWidth() == 200 and c.getHeight() == 150, @src());

    // Events: a listener on a custom event is called by trigger, and not after off.
    const handler = b.any_unknown_array_to_void_or_boolean_fn.from(onPing, &fired);
    _ = c.on(.{ .string = b.JsString.from("ping") }, handler, .{});
    _ = c.trigger(.{ .string = b.JsString.from("ping") }, b.JsArray.new());
    _ = c.trigger(.{ .string = b.JsString.from("ping") }, b.JsArray.new());
    expect(fired == 2, @src());
    expect(!c.isSilent(.{ .string = b.JsString.from("ping") }), @src());
    _ = c.off(.{ .eventType = .{ .string = b.JsString.from("ping") } });
    _ = c.trigger(.{ .string = b.JsString.from("ping") }, b.JsArray.new());
    expect(fired == 2, @src());

    // Rendering: an SVG string of the chart, and a data URL.
    expect(c.getDataURL(null).len() > 100, @src());

    // Clear empties the option; dispose ends the chart.
    c.clear();
    expect(!c.isDisposed(), @src());
    c.dispose();
    expect(c.isDisposed(), @src());
    expect(ec.getInstanceByDom(b.test_page.element("chart")) == null, @src());
}
