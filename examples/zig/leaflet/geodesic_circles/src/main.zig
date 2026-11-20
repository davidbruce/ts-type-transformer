const std = @import("std");
const dom = @import("dom");
const lf = @import("leaflet");

export fn start() void {
    const scope = lf.JsScope.open();
    defer scope.deinit();

    const m = lf.map("map", .{
        .attributionControl = false,
        .zoomAnimation = false,
        .fadeAnimation = false,
        .markerZoomAnimation = false,
    });

    const zurich = lf.latLng(47.3769, 8.5417, null);
    _ = m.setView(zurich, .{ .zoom = 8.0 });

    const radii = [_]f64{ 130000.0, 90000.0, 55000.0, 25000.0 };
    const strokes = [_][]const u8{ "#404040", "#404040", "#111111", "#111111" };
    const fills = [_][]const u8{ "#d4d4d4", "#a3a3a3", "#737373", "#262626" };
    const opacities = [_]f64{ 0.25, 0.30, 0.40, 0.55 };

    var outer_bounds: lf.LatLngBounds = undefined;
    for (radii, 0..) |r, i| {
        const c = lf.circleWithRadius(zurich, r, .{
            .color = strokes[i],
            .weight = 1.5,
            .fillColor = fills[i],
            .fillOpacity = opacities[i],
        });
        _ = c.addTo(m);
        if (i == 0) outer_bounds = c.getBounds();
    }

    const beacons = [_]lf.LatLng{
        lf.latLng(47.5596, 7.5886, null),
        lf.latLng(46.9480, 7.4474, null),
        lf.latLng(47.0502, 8.3093, null),
        lf.latLng(47.4245, 9.3767, null),
        lf.latLng(46.8508, 9.5320, null),
    };

    const beacon_opts = lf.CircleMarkerOptions.init(.{
        .color = "#555555",
        .weight = 1.2,
        .dashArray = "3 3",
        .fillColor = "#737373",
        .fillOpacity = 0.18,
    });
    const dot_opts = lf.CircleMarkerOptions.init(.{
        .radius = 4.5,
        .color = "#111111",
        .weight = 1.5,
        .fillColor = "#111111",
        .fillOpacity = 1.0,
    });

    var total_dist_km: f64 = 0.0;
    for (beacons) |pt| {
        total_dist_km += zurich.distanceTo(pt) / 1000.0;
        _ = lf.circleWithRadius(pt, 18000.0, beacon_opts).addTo(m);
        _ = lf.circleMarker(pt, dot_opts).addTo(m);
    }

    _ = m.fitBounds(outer_bounds, null);

    var buf: [160]u8 = undefined;
    const msg = std.fmt.bufPrint(
        &buf,
        "4 concentric rings (25-130 km) · 5 regional beacons (avg {d:.1} km)",
        .{total_dist_km / 5.0},
    ) catch "4 concentric rings · 5 beacons";

    if (dom.getDocument().getElementById("meta")) |meta| {
        meta.setTextContent(msg);
    }
}
