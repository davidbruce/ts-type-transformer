const std = @import("std");
const dom = @import("dom");
const lf = @import("leaflet");

fn makeMapOpts() lf.MapOptions {
    const opts = lf.MapOptions.create();
    opts.setAttributionControl(false);
    opts.setZoomAnimation(false);
    opts.setFadeAnimation(false);
    opts.setMarkerZoomAnimation(false);
    return opts;
}

fn polyOpts(color: []const u8, weight: f64, dash: ?[]const u8, fill_opacity: f64) lf.PolylineOptions {
    const opts = lf.PolylineOptions.create();
    opts.setSmoothFactor(1.0);
    opts.setColor(color);
    opts.setWeight(weight);
    if (dash) |d| {
        opts.setDashArray(.{ .string = lf.JsString.from(d) });
    }
    opts.setFillOpacity(fill_opacity);
    return opts;
}

export fn start() void {
    const m = lf.map(.{ .string = lf.JsString.from("map") }, .{ .options = makeMapOpts() });

    const hub_names = [_][]const u8{ "London", "Paris", "Zurich", "Munich", "Prague", "Berlin" };
    const hubs = [_]lf.LatLng{
        lf.latLng(51.505, -0.09, .{}),
        lf.latLng(48.8566, 2.3522, .{}),
        lf.latLng(47.3769, 8.5417, .{}),
        lf.latLng(48.1351, 11.5820, .{}),
        lf.latLng(50.0755, 14.4378, .{}),
        lf.latLng(52.5200, 13.4050, .{}),
    };

    const grat_opts = polyOpts("#e5e5e5", 1.0, null, 0.0);
    const lats = [_]f64{ 46.0, 48.0, 50.0, 52.0, 54.0 };
    for (lats) |lat| {
        const pts = lf.JsArray.new();
        pts.push(.{ .h = lf.latLng(lat, -2.0, .{}).h });
        pts.push(.{ .h = lf.latLng(lat, 17.0, .{}).h });
        _ = lf.polyline(pts, .{ .options = grat_opts }).addTo(.{ .Map = m });
    }
    const lngs = [_]f64{ 0.0, 4.0, 8.0, 12.0, 16.0 };
    for (lngs) |lng| {
        const pts = lf.JsArray.new();
        pts.push(.{ .h = lf.latLng(45.0, lng, .{}).h });
        pts.push(.{ .h = lf.latLng(55.0, lng, .{}).h });
        _ = lf.polyline(pts, .{ .options = grat_opts }).addTo(.{ .Map = m });
    }

    const primary_pts = lf.JsArray.new();
    for (hubs) |pt| primary_pts.push(.{ .h = pt.h });
    const primary_line = lf.polyline(primary_pts, .{ .options = polyOpts("#111111", 3.0, null, 0.0) });
    _ = primary_line.addTo(.{ .Map = m });

    const alt_pts = lf.JsArray.new();
    alt_pts.push(.{ .h = hubs[0].h });
    alt_pts.push(.{ .h = lf.latLng(50.8503, 4.3517, .{}).h });
    alt_pts.push(.{ .h = lf.latLng(52.3676, 4.9041, .{}).h });
    alt_pts.push(.{ .h = hubs[5].h });
    _ = lf.polyline(alt_pts, .{ .options = polyOpts("#737373", 1.5, "5 5", 0.0) }).addTo(.{ .Map = m });

    const nodes = lf.layerGroup(.{});
    const node_opts = lf.CircleMarkerOptions.create();
    node_opts.setRadius(6.0);
    node_opts.setColor("#111111");
    node_opts.setWeight(2.0);
    node_opts.setFillColor("#ffffff");
    node_opts.setFillOpacity(1.0);

    for (hubs, 0..) |pt, i| {
        const cm = lf.circleMarker(.{ .LatLng = pt }, .{ .options = node_opts });
        _ = cm.bindTooltip(.{ .string = lf.JsString.from(hub_names[i]) }, .{});
        _ = nodes.addLayer(.{ .h = cm.h });
    }
    _ = nodes.addTo(.{ .Map = m });

    _ = m.fitBounds(.{ .LatLngBounds = primary_line.getBounds() }, .{});

    var total_km: f64 = 0.0;
    var i: usize = 0;
    while (i + 1 < hubs.len) : (i += 1) {
        total_km += hubs[i].distanceTo(.{ .LatLng = hubs[i + 1] }) / 1000.0;
    }

    var buf: [160]u8 = undefined;
    const msg = std.fmt.bufPrint(
        &buf,
        "6 hubs · 2 routes · {d:.0} km total · EPSG:3857 vector SVG",
        .{total_km},
    ) catch "6 hubs · 2 routes · EPSG:3857 vector SVG";

    const doc = dom.getDocument();
    if (doc.getElementById("meta")) |meta| {
        meta.asNode().setTextContent(msg);
    }
}
