const std = @import("std");
const dom = @import("dom");
const lf = @import("leaflet");

fn polyOpts(color: []const u8, weight: f64, dash: ?[]const u8, fill_opacity: f64) lf.PolylineOptions {
    return lf.PolylineOptions.init(.{
        .smoothFactor = 1.0,
        .color = color,
        .weight = weight,
        .dashArray = dash,
        .fillOpacity = fill_opacity,
    });
}

export fn start() void {
    const scope = lf.JsScope.open();
    defer scope.deinit();

    const m = lf.map("map", .{
        .attributionControl = false,
        .zoomAnimation = false,
        .fadeAnimation = false,
        .markerZoomAnimation = false,
    });

    const hub_names = [_][]const u8{ "London", "Paris", "Zurich", "Munich", "Prague", "Berlin" };
    const hubs = [_]lf.LatLng{
        lf.latLng(51.505, -0.09, null),
        lf.latLng(48.8566, 2.3522, null),
        lf.latLng(47.3769, 8.5417, null),
        lf.latLng(48.1351, 11.5820, null),
        lf.latLng(50.0755, 14.4378, null),
        lf.latLng(52.5200, 13.4050, null),
    };

    const grat_opts = polyOpts("#e5e5e5", 1.0, null, 0.0);
    for ([_]f64{ 46.0, 48.0, 50.0, 52.0, 54.0 }) |lat| {
        _ = lf.polyline(.{ lf.latLng(lat, -2.0, null), lf.latLng(lat, 17.0, null) }, grat_opts).addTo(m);
    }
    for ([_]f64{ 0.0, 4.0, 8.0, 12.0, 16.0 }) |lng| {
        _ = lf.polyline(.{ lf.latLng(45.0, lng, null), lf.latLng(55.0, lng, null) }, grat_opts).addTo(m);
    }

    const primary_line = lf.polyline(&hubs, polyOpts("#111111", 3.0, null, 0.0));
    _ = primary_line.addTo(m);

    _ = lf.polyline(.{
        hubs[0],
        lf.latLng(50.8503, 4.3517, null),
        lf.latLng(52.3676, 4.9041, null),
        hubs[5],
    }, polyOpts("#737373", 1.5, "5 5", 0.0)).addTo(m);

    const nodes = lf.layerGroup(null);
    const node_opts = lf.CircleMarkerOptions.init(.{
        .radius = 6.0,
        .color = "#111111",
        .weight = 2.0,
        .fillColor = "#ffffff",
        .fillOpacity = 1.0,
    });

    for (hubs, 0..) |pt, i| {
        const cm = lf.circleMarker(pt, node_opts);
        _ = cm.bindTooltip(hub_names[i], null);
        _ = nodes.addLayer(cm);
    }
    _ = nodes.addTo(m);

    _ = m.fitBounds(primary_line.getBounds(), null);

    var total_km: f64 = 0.0;
    for (0..hubs.len - 1) |i| {
        total_km += hubs[i].distanceTo(hubs[i + 1]) / 1000.0;
    }

    var buf: [160]u8 = undefined;
    const msg = std.fmt.bufPrint(
        &buf,
        "6 hubs · 2 routes · {d:.0} km total · EPSG:3857 vector SVG",
        .{total_km},
    ) catch "6 hubs · 2 routes · EPSG:3857 vector SVG";

    if (dom.getDocument().getElementById("meta")) |meta| {
        meta.setTextContent(msg);
    }
}
