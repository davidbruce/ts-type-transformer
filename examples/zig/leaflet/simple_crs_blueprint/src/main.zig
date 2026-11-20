const std = @import("std");
const dom = @import("dom");
const lf = @import("leaflet");

fn boxOpts(color: []const u8, weight: f64, dash: ?[]const u8, fill_color: []const u8, fill_opacity: f64) lf.PolylineOptions {
    return lf.PolylineOptions.init(.{
        .smoothFactor = 1.0,
        .color = color,
        .weight = weight,
        .dashArray = dash,
        .fillColor = fill_color,
        .fillOpacity = fill_opacity,
    });
}

export fn start() void {
    const scope = lf.JsScope.open();
    defer scope.deinit();

    const m = lf.map("map", .{
        .crs = lf.crs.getSimple(),
        .minZoom = -1.0,
        .maxZoom = 3.0,
        .attributionControl = false,
        .zoomAnimation = false,
        .fadeAnimation = false,
        .markerZoomAnimation = false,
    });

    const grid_opts = boxOpts("#e5e5e5", 1.0, null, "#ffffff", 0.0);
    var r: f64 = 0.0;
    while (r <= 100.0) : (r += 20.0) {
        _ = lf.polyline(.{ lf.latLng(r, 0.0, null), lf.latLng(r, 160.0, null) }, grid_opts).addTo(m);
    }
    var c: f64 = 0.0;
    while (c <= 160.0) : (c += 20.0) {
        _ = lf.polyline(.{ lf.latLng(0.0, c, null), lf.latLng(100.0, c, null) }, grid_opts).addTo(m);
    }

    const env = lf.latLngBounds(lf.latLng(8.0, 12.0, null), lf.latLng(92.0, 148.0, null));
    _ = lf.rectangle(env, boxOpts("#111111", 2.5, null, "#ffffff", 0.9)).addTo(m);

    const room_coords = [_][4]f64{
        .{ 12.0, 16.0, 46.0, 62.0 },
        .{ 54.0, 16.0, 88.0, 62.0 },
        .{ 12.0, 98.0, 46.0, 144.0 },
        .{ 54.0, 98.0, 88.0, 144.0 },
    };
    const room_fills = [_][]const u8{ "#f7f7f7", "#efefef", "#e5e5e5", "#d4d4d4" };
    for (room_coords, 0..) |rc, i| {
        const b = lf.latLngBounds(lf.latLng(rc[0], rc[1], null), lf.latLng(rc[2], rc[3], null));
        _ = lf.rectangle(b, boxOpts("#404040", 1.5, null, room_fills[i], 0.85)).addTo(m);
    }

    _ = lf.polygon(&.{
        lf.latLng(50.0, 64.0, null),
        lf.latLng(74.0, 80.0, null),
        lf.latLng(50.0, 96.0, null),
        lf.latLng(26.0, 80.0, null),
    }, boxOpts("#111111", 2.0, null, "#555555", 0.35)).addTo(m);

    _ = lf.polyline(.{ lf.latLng(50.0, 12.0, null), lf.latLng(50.0, 148.0, null) }, boxOpts("#737373", 1.5, "4 4", "#ffffff", 0.0)).addTo(m);

    const col_opts = lf.CircleMarkerOptions.init(.{
        .radius = 5.0,
        .color = "#111111",
        .weight = 1.5,
        .fillColor = "#111111",
        .fillOpacity = 1.0,
    });
    const cols = [_][2]f64{
        .{ 12.0, 16.0 }, .{ 88.0, 16.0 }, .{ 12.0, 80.0 },
        .{ 88.0, 80.0 }, .{ 12.0, 144.0 }, .{ 88.0, 144.0 },
    };
    for (cols) |col| {
        _ = lf.circleMarker(lf.latLng(col[0], col[1], null), col_opts).addTo(m);
    }

    const plan_bounds = lf.latLngBounds(lf.latLng(0.0, 0.0, null), lf.latLng(100.0, 160.0, null));
    _ = m.fitBounds(plan_bounds, null);

    const px = m.latLngToContainerPoint(lf.latLng(50.0, 80.0, null));
    var buf: [160]u8 = undefined;
    const msg = std.fmt.bufPrint(
        &buf,
        "CRS.Simple 100x160 plan · 4 studio rooms + atrium · center @ ({d:.0}, {d:.0}) px",
        .{ px.getX(), px.getY() },
    ) catch "CRS.Simple 100x160 plan";

    if (dom.getDocument().getElementById("meta")) |meta| {
        meta.setTextContent(msg);
    }
}
