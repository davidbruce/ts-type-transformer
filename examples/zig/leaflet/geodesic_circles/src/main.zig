const std = @import("std");
const dom = @import("dom");
const lf = @import("leaflet");

export fn start() void {
    const m_opts = lf.MapOptions.create();
    m_opts.setAttributionControl(false);
    m_opts.setZoomAnimation(false);
    m_opts.setFadeAnimation(false);
    m_opts.setMarkerZoomAnimation(false);
    const m = lf.map(.{ .string = lf.JsString.from("map") }, .{ .options = m_opts });

    const zurich = lf.latLng(47.3769, 8.5417, .{});
    _ = m.setView(.{ .LatLng = zurich }, .{ .zoom = 8.0 });
    const radii = [_]f64{ 130000.0, 90000.0, 55000.0, 25000.0 };
    const strokes = [_][]const u8{ "#404040", "#404040", "#111111", "#111111" };
    const fills = [_][]const u8{ "#d4d4d4", "#a3a3a3", "#737373", "#262626" };
    const opacities = [_]f64{ 0.25, 0.30, 0.40, 0.55 };

    var outer_bounds: lf.LatLngBounds = undefined;
    for (radii, 0..) |r, i| {
        const c_opts = lf.CircleMarkerOptions.create();
        c_opts.setColor(strokes[i]);
        c_opts.setWeight(1.5);
        c_opts.setFillColor(fills[i]);
        c_opts.setFillOpacity(opacities[i]);
        const c = lf.circleWithRadius(.{ .LatLng = zurich }, r, .{ .options = c_opts });
        _ = c.addTo(.{ .Map = m });
        if (i == 0) outer_bounds = c.getBounds();
    }

    const beacons = [_]lf.LatLng{
        lf.latLng(47.5596, 7.5886, .{}),
        lf.latLng(46.9480, 7.4474, .{}),
        lf.latLng(47.0502, 8.3093, .{}),
        lf.latLng(47.4245, 9.3767, .{}),
        lf.latLng(46.8508, 9.5320, .{}),
    };

    var total_dist_km: f64 = 0.0;
    for (beacons) |pt| {
        total_dist_km += zurich.distanceTo(.{ .LatLng = pt }) / 1000.0;

        const b_opts = lf.CircleMarkerOptions.create();
        b_opts.setColor("#555555");
        b_opts.setWeight(1.2);
        b_opts.setDashArray(.{ .string = lf.JsString.from("3 3") });
        b_opts.setFillColor("#737373");
        b_opts.setFillOpacity(0.18);
        _ = lf.circleWithRadius(.{ .LatLng = pt }, 18000.0, .{ .options = b_opts }).addTo(.{ .Map = m });

        const d_opts = lf.CircleMarkerOptions.create();
        d_opts.setRadius(4.5);
        d_opts.setColor("#111111");
        d_opts.setWeight(1.5);
        d_opts.setFillColor("#111111");
        d_opts.setFillOpacity(1.0);
        _ = lf.circleMarker(.{ .LatLng = pt }, .{ .options = d_opts }).addTo(.{ .Map = m });
    }

    _ = m.fitBounds(.{ .LatLngBounds = outer_bounds }, .{});

    var buf: [160]u8 = undefined;
    const msg = std.fmt.bufPrint(
        &buf,
        "4 concentric rings (25-130 km) · 5 regional beacons (avg {d:.1} km)",
        .{total_dist_km / 5.0},
    ) catch "4 concentric rings · 5 beacons";

    const doc = dom.getDocument();
    if (doc.getElementById("meta")) |meta| {
        meta.asNode().setTextContent(msg);
    }
}
