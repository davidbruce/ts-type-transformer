const std = @import("std");
const dom = @import("dom");
const lf = @import("leaflet");

fn polyOpts(color: []const u8, weight: f64, dash: ?[]const u8, fill_color: []const u8, fill_opacity: f64) lf.PolylineOptions {
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
    m_opts.setAttributionControl(false);
    m_opts.setZoomAnimation(false);
    m_opts.setFadeAnimation(false);
    m_opts.setMarkerZoomAnimation(false);
    const m = lf.map(.{ .string = lf.JsString.from("map") }, .{ .options = m_opts });

    const mid_ring = lf.JsArrayOf(lf.Polygon_new_latlngs).of(&.{
        .{ .LatLng = lf.latLng(47.2, 7.6, .{}) },
        .{ .LatLng = lf.latLng(48.0, 8.6, .{}) },
        .{ .LatLng = lf.latLng(47.5, 9.7, .{}) },
        .{ .LatLng = lf.latLng(46.8, 9.5, .{}) },
        .{ .LatLng = lf.latLng(46.6, 8.4, .{}) },
        .{ .LatLng = lf.latLng(46.9, 7.3, .{}) },
    });

    const core_ring = lf.JsArrayOf(lf.Polygon_new_latlngs).of(&.{
        .{ .LatLng = lf.latLng(47.35, 8.2, .{}) },
        .{ .LatLng = lf.latLng(47.55, 8.75, .{}) },
        .{ .LatLng = lf.latLng(47.15, 9.0, .{}) },
        .{ .LatLng = lf.latLng(46.95, 8.4, .{}) },
    });

    const mid_poly = lf.polygon(mid_ring, .{ .options = polyOpts("#404040", 2.0, null, "#d4d4d4", 0.55) });
    const core_poly = lf.polygon(core_ring, .{ .options = polyOpts("#111111", 2.5, null, "#404040", 0.75) });

    const mid_bounds = mid_poly.asPolyline().getBounds();
    const core_bounds = core_poly.asPolyline().getBounds();
    const padded = mid_bounds.pad(0.18);
    const hull_rect = lf.rectangle(.{ .LatLngBounds = padded }, .{ .options = polyOpts("#737373", 1.5, "4 4", "#efefef", 0.05) });

    _ = hull_rect.addTo(.{ .Map = m });
    _ = mid_poly.addTo(.{ .Map = m });
    _ = core_poly.addTo(.{ .Map = m });

    const probes = [_]lf.LatLng{
        lf.latLng(47.25, 8.6, .{}),
        lf.latLng(47.15, 8.55, .{}),
        lf.latLng(47.7, 8.6, .{}),
        lf.latLng(47.0, 9.2, .{}),
        lf.latLng(46.8, 7.9, .{}),
        lf.latLng(47.85, 7.5, .{}),
        lf.latLng(47.9, 9.7, .{}),
        lf.latLng(46.55, 9.6, .{}),
    };

    var in_core: usize = 0;
    var in_mid: usize = 0;
    var exterior: usize = 0;

    for (probes) |pt| {
        const fill_hex: []const u8 = if (core_bounds.contains(.{ .LatLng = pt })) blk: {
            in_core += 1;
            break :blk "#111111";
        } else if (mid_bounds.contains(.{ .LatLng = pt })) blk: {
            in_mid += 1;
            break :blk "#737373";
        } else blk: {
            exterior += 1;
            break :blk "#ffffff";
        };
        const cm_opts = lf.CircleMarkerOptions.create();
        cm_opts.setRadius(6.0);
        cm_opts.setColor("#111111");
        cm_opts.setWeight(2.0);
        cm_opts.setFillColor(fill_hex);
        cm_opts.setFillOpacity(1.0);
        _ = lf.circleMarker(.{ .LatLng = pt }, .{ .options = cm_opts }).addTo(.{ .Map = m });
    }

    _ = m.fitBounds(.{ .LatLngBounds = padded }, .{});

    var buf: [160]u8 = undefined;
    const msg = std.fmt.bufPrint(
        &buf,
        "2 nested polygons · 1 padded hull · probes: {d} core · {d} sector · {d} exterior",
        .{ in_core, in_mid, exterior },
    ) catch "2 nested polygons · 1 padded hull";

    const doc = dom.getDocument();
    if (doc.getElementById("meta")) |meta| {
        meta.asNode().setTextContent(msg);
    }
}
