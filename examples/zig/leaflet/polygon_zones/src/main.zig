const std = @import("std");
const dom = @import("dom");
const lf = @import("leaflet");

fn polyOpts(color: []const u8, weight: f64, dash: ?[]const u8, fill_color: []const u8, fill_opacity: f64) lf.PolylineOptions {
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

    const m = lf.mapWith("map", .{
        .attributionControl = false,
        .zoomAnimation = false,
        .fadeAnimation = false,
        .markerZoomAnimation = false,
    });

    const mid_poly = lf.polygonWith(&.{
        lf.latLng(47.2, 7.6),
        lf.latLng(48.0, 8.6),
        lf.latLng(47.5, 9.7),
        lf.latLng(46.8, 9.5),
        lf.latLng(46.6, 8.4),
        lf.latLng(46.9, 7.3),
    }, polyOpts("#404040", 2.0, null, "#d4d4d4", 0.55));

    const core_poly = lf.polygonWith(&.{
        lf.latLng(47.35, 8.2),
        lf.latLng(47.55, 8.75),
        lf.latLng(47.15, 9.0),
        lf.latLng(46.95, 8.4),
    }, polyOpts("#111111", 2.5, null, "#404040", 0.75));

    const mid_bounds = mid_poly.getBounds();
    const core_bounds = core_poly.getBounds();
    const padded = mid_bounds.pad(0.18);
    const hull_rect = lf.rectangleWith(padded, polyOpts("#737373", 1.5, "4 4", "#efefef", 0.05));

    _ = hull_rect.addTo(m);
    _ = mid_poly.addTo(m);
    _ = core_poly.addTo(m);

    const probes = [_]lf.LatLng{
        lf.latLng(47.25, 8.6),
        lf.latLng(47.15, 8.55),
        lf.latLng(47.7, 8.6),
        lf.latLng(47.0, 9.2),
        lf.latLng(46.8, 7.9),
        lf.latLng(47.85, 7.5),
        lf.latLng(47.9, 9.7),
        lf.latLng(46.55, 9.6),
    };

    var in_core: usize = 0;
    var in_mid: usize = 0;
    var exterior: usize = 0;

    for (probes) |pt| {
        const fill_hex: []const u8 = if (core_bounds.contains(pt)) blk: {
            in_core += 1;
            break :blk "#111111";
        } else if (mid_bounds.contains(pt)) blk: {
            in_mid += 1;
            break :blk "#737373";
        } else blk: {
            exterior += 1;
            break :blk "#ffffff";
        };
        _ = lf.circleMarkerWith(pt, .{
            .radius = 6.0,
            .color = "#111111",
            .weight = 2.0,
            .fillColor = fill_hex,
            .fillOpacity = 1.0,
        }).addTo(m);
    }

    _ = m.fitBounds(padded);

    var buf: [160]u8 = undefined;
    const msg = std.fmt.bufPrint(
        &buf,
        "2 nested polygons · 1 padded hull · probes: {d} core · {d} sector · {d} exterior",
        .{ in_core, in_mid, exterior },
    ) catch "2 nested polygons · 1 padded hull";

    if (dom.getDocument().getElementById("meta")) |meta| {
        meta.setTextContent(msg);
    }
}
