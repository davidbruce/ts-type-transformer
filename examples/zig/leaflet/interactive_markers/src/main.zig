const std = @import("std");
const dom = @import("dom");
const lf = @import("leaflet");

var g_clicks: u32 = 0;

fn onClick(_: ?*anyopaque, e: lf.LeafletMouseEvent) void {
    if (e.getLatlng().getLat() > 0.0) {
        g_clicks += 1;
    }
}

export fn start() void {
    const m_opts = lf.MapOptions.create();
    m_opts.setAttributionControl(false);
    m_opts.setZoomAnimation(false);
    m_opts.setFadeAnimation(false);
    m_opts.setMarkerZoomAnimation(false);
    const m = lf.map(.{ .string = lf.JsString.from("map") }, .{ .options = m_opts });

    const tips = [_][]const u8{
        "01 · Geneva", "02 · Lausanne", "03 · Bern", "04 · Zurich", "05 · Basel",
    };
    const htmls = [_][]const u8{
        "<span>01</span>", "<span>02</span>", "<span>03</span>", "<span>04</span>", "<span>05</span>",
    };
    const pts = [_]lf.LatLng{
        lf.latLng(46.2044, 6.1432, .{}),
        lf.latLng(46.5197, 6.6323, .{}),
        lf.latLng(46.9480, 7.4474, .{}),
        lf.latLng(47.3769, 8.5417, .{}),
        lf.latLng(47.5596, 7.5886, .{}),
    };

    const line_pts = lf.JsArray.new();
    for (pts) |pt| line_pts.push(.{ .h = pt.h });

    const p_opts = lf.PolylineOptions.create();
    p_opts.setSmoothFactor(1.0);
    p_opts.setColor("#404040");
    p_opts.setWeight(2.5);
    const corridor = lf.polyline(line_pts, .{ .options = p_opts });
    _ = corridor.addTo(.{ .Map = m });

    const dot_opts = lf.CircleMarkerOptions.create();
    dot_opts.setRadius(6.0);
    dot_opts.setColor("#111111");
    dot_opts.setWeight(2.0);
    dot_opts.setFillColor("#ffffff");
    dot_opts.setFillOpacity(1.0);

    const pin_size = lf.PointExpression{ .Point = lf.point(22.0, 22.0, .{}) };
    const pin_anchor = lf.PointExpression{ .Point = lf.point(11.0, 11.0, .{}) };

    for (pts, 0..) |pt, i| {
        const cm = lf.circleMarker(.{ .LatLng = pt }, .{ .options = dot_opts });
        _ = cm.bindTooltip(.{ .string = lf.JsString.from(tips[i]) }, .{});
        _ = cm.addTo(.{ .Map = m });

        const d_opts = lf.DivIconOptions.create();
        d_opts.setClassName("mono-pin");
        d_opts.setHtml(.{ .string = lf.JsString.from(htmls[i]) });
        d_opts.setIconSize(pin_size);
        d_opts.setIconAnchor(pin_anchor);

        const mk_opts = lf.MarkerOptions.create();
        mk_opts.setIcon(.{ .DivIcon = lf.divIcon(d_opts) });
        _ = lf.marker(.{ .LatLng = pt }, .{ .options = mk_opts }).addTo(.{ .Map = m });
    }

    _ = m.fitBounds(.{ .LatLngBounds = corridor.getBounds().pad(0.15) }, .{});

    const pop_opts = lf.PopupOptions.create();
    pop_opts.setMaxWidth(220.0);
    const note = lf.popup(.{ .LatLng = pts[2] }, .{ .options = pop_opts });
    _ = note.setContent(.{ .string = lf.JsString.from("03 Bern · 46.95N, 7.45E") });
    _ = note.openOn(m);

    const handler = lf.LeafletMouseEventHandlerFn.from(onClick, null);
    _ = m.onWithCanvasAddEventListenerTypeClick(.click, handler, .{});
    const payload = lf.LeafletMouseEvent.create();
    payload.setLatlng(pts[2]);
    _ = m.fire("click", .{ .data = .{ .h = payload.h } });
    _ = m.fire("click", .{ .data = .{ .h = payload.h } });

    var buf: [160]u8 = undefined;
    const msg = std.fmt.bufPrint(
        &buf,
        "5 DivIcon stations · popup open (Bern) · {d} synthetic clicks dispatched",
        .{g_clicks},
    ) catch "5 DivIcon stations";

    const doc = dom.getDocument();
    if (doc.getElementById("meta")) |meta| {
        meta.asNode().setTextContent(msg);
    }
}
