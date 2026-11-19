const std = @import("std");
const dom = @import("dom");
const lf = @import("leaflet");

fn boxOpts(color: []const u8, weight: f64, dash: ?[]const u8, fill_color: []const u8, fill_opacity: f64) lf.PolylineOptions {
    const opts = lf.PolylineOptions.create();
    opts.setSmoothFactor(1.0);
    opts.setColor(color);
    opts.setWeight(weight);
    if (dash) |d| {
        opts.setDashArray(.{ .string = lf.JsString.from(d) });
    }
    opts.setFillColor(fill_color);
    opts.setFillOpacity(fill_opacity);
    return opts;
}

export fn start() void {
    const m_opts = lf.MapOptions.create();
    m_opts.setCrs(lf.crs.getSimple());
    m_opts.setMinZoom(-1.0);
    m_opts.setMaxZoom(3.0);
    m_opts.setAttributionControl(false);
    m_opts.setZoomAnimation(false);
    m_opts.setFadeAnimation(false);
    m_opts.setMarkerZoomAnimation(false);
    const m = lf.map(.{ .string = lf.JsString.from("map") }, .{ .options = m_opts });

    const grid_opts = boxOpts("#e5e5e5", 1.0, null, "#ffffff", 0.0);
    var r: f64 = 0.0;
    while (r <= 100.0) : (r += 20.0) {
        const line = lf.JsArray.new();
        line.push(.{ .h = lf.latLng(r, 0.0, .{}).h });
        line.push(.{ .h = lf.latLng(r, 160.0, .{}).h });
        _ = lf.polyline(line, .{ .options = grid_opts }).addTo(.{ .Map = m });
    }
    var c: f64 = 0.0;
    while (c <= 160.0) : (c += 20.0) {
        const line = lf.JsArray.new();
        line.push(.{ .h = lf.latLng(0.0, c, .{}).h });
        line.push(.{ .h = lf.latLng(100.0, c, .{}).h });
        _ = lf.polyline(line, .{ .options = grid_opts }).addTo(.{ .Map = m });
    }

    const env = lf.latLngBounds(.{ .LatLng = lf.latLng(8.0, 12.0, .{}) }, .{ .LatLng = lf.latLng(92.0, 148.0, .{}) });
    _ = lf.rectangle(.{ .LatLngBounds = env }, .{ .options = boxOpts("#111111", 2.5, null, "#ffffff", 0.9) }).addTo(.{ .Map = m });

    const room_coords = [_][4]f64{
        .{ 12.0, 16.0, 46.0, 62.0 },
        .{ 54.0, 16.0, 88.0, 62.0 },
        .{ 12.0, 98.0, 46.0, 144.0 },
        .{ 54.0, 98.0, 88.0, 144.0 },
    };
    const room_fills = [_][]const u8{ "#f7f7f7", "#efefef", "#e5e5e5", "#d4d4d4" };
    for (room_coords, 0..) |rc, i| {
        const b = lf.latLngBounds(.{ .LatLng = lf.latLng(rc[0], rc[1], .{}) }, .{ .LatLng = lf.latLng(rc[2], rc[3], .{}) });
        _ = lf.rectangle(.{ .LatLngBounds = b }, .{ .options = boxOpts("#404040", 1.5, null, room_fills[i], 0.85) }).addTo(.{ .Map = m });
    }

    const atrium = lf.JsArrayOf(lf.Polygon_new_latlngs).of(&.{
        .{ .LatLng = lf.latLng(50.0, 64.0, .{}) },
        .{ .LatLng = lf.latLng(74.0, 80.0, .{}) },
        .{ .LatLng = lf.latLng(50.0, 96.0, .{}) },
        .{ .LatLng = lf.latLng(26.0, 80.0, .{}) },
    });
    _ = lf.polygon(atrium, .{ .options = boxOpts("#111111", 2.0, null, "#555555", 0.35) }).addTo(.{ .Map = m });

    const spine = lf.JsArray.new();
    spine.push(.{ .h = lf.latLng(50.0, 12.0, .{}).h });
    spine.push(.{ .h = lf.latLng(50.0, 148.0, .{}).h });
    _ = lf.polyline(spine, .{ .options = boxOpts("#737373", 1.5, "4 4", "#ffffff", 0.0) }).addTo(.{ .Map = m });

    const col_opts = lf.CircleMarkerOptions.create();
    col_opts.setRadius(5.0);
    col_opts.setColor("#111111");
    col_opts.setWeight(1.5);
    col_opts.setFillColor("#111111");
    col_opts.setFillOpacity(1.0);
    const cols = [_][2]f64{
        .{ 12.0, 16.0 }, .{ 88.0, 16.0 }, .{ 12.0, 80.0 },
        .{ 88.0, 80.0 }, .{ 12.0, 144.0 }, .{ 88.0, 144.0 },
    };
    for (cols) |col| {
        _ = lf.circleMarker(.{ .LatLng = lf.latLng(col[0], col[1], .{}) }, .{ .options = col_opts }).addTo(.{ .Map = m });
    }

    const plan_bounds = lf.latLngBounds(.{ .LatLng = lf.latLng(0.0, 0.0, .{}) }, .{ .LatLng = lf.latLng(100.0, 160.0, .{}) });
    _ = m.fitBounds(.{ .LatLngBounds = plan_bounds }, .{});

    const px = m.latLngToContainerPoint(.{ .LatLng = lf.latLng(50.0, 80.0, .{}) });
    var buf: [160]u8 = undefined;
    const msg = std.fmt.bufPrint(
        &buf,
        "CRS.Simple 100x160 plan · 4 studio rooms + atrium · center @ ({d:.0}, {d:.0}) px",
        .{ px.getX(), px.getY() },
    ) catch "CRS.Simple 100x160 plan";

    const doc = dom.getDocument();
    if (doc.getElementById("meta")) |meta| {
        meta.asNode().setTextContent(msg);
    }
}
