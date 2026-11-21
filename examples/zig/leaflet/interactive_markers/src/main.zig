const std = @import("std");
const dom = @import("dom");
const lf = @import("leaflet");

const State = struct {
    clicks: u32 = 0,
    map: lf.Map = .{ .h = 0 },
    note: lf.Popup = .{ .h = 0 },
};

var g_state: State = .{};

fn onClick(st: *State, e: lf.LeafletMouseEvent) void {
    const scope = lf.JsScope.open();
    defer scope.deinit();
    const ll = e.getLatlng();
    const lat = ll.getLat();
    const lng = ll.getLng();
    if (lat <= 0.0) return;
    st.clicks += 1;
    var buf: [160]u8 = undefined;
    if (st.clicks > 2) {
        const pop_msg = std.fmt.bufPrint(&buf, "Click #{d} \xC2\xB7 {d:.2}N, {d:.2}E", .{ st.clicks, lat, lng }) catch "Click";
        _ = st.note.setLatLng(ll);
        _ = st.note.setContent(pop_msg);
        _ = st.note.openOn(st.map);
    }
    const msg = std.fmt.bufPrint(
        &buf,
        "5 DivIcon stations \xC2\xB7 clicks={d} \xC2\xB7 last click ({d:.2}N, {d:.2}E)",
        .{ st.clicks, lat, lng },
    ) catch "5 DivIcon stations";
    if (dom.getDocument().getElementById("meta")) |meta| {
        meta.setTextContent(msg);
    }
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

    const tips = [_][]const u8{
        "01 · Geneva", "02 · Lausanne", "03 · Bern", "04 · Zurich", "05 · Basel",
    };
    const htmls = [_][]const u8{
        "<span>01</span>", "<span>02</span>", "<span>03</span>", "<span>04</span>", "<span>05</span>",
    };
    const pts = [_]lf.LatLng{
        lf.latLng(46.2044, 6.1432, null),
        lf.latLng(46.5197, 6.6323, null),
        lf.latLng(46.9480, 7.4474, null),
        lf.latLng(47.3769, 8.5417, null),
        lf.latLng(47.5596, 7.5886, null),
    };

    const corridor = lf.polyline(&pts, .{ .smoothFactor = 1.0, .color = "#404040", .weight = 2.5 });
    _ = corridor.addTo(m);

    const dot_opts = lf.CircleMarkerOptions.init(.{
        .radius = 6.0,
        .color = "#111111",
        .weight = 2.0,
        .fillColor = "#ffffff",
        .fillOpacity = 1.0,
    });
    const pin_size = lf.point(22.0, 22.0, null);
    const pin_anchor = lf.point(11.0, 11.0, null);

    for (pts, 0..) |pt, i| {
        const cm = lf.circleMarker(pt, dot_opts);
        _ = cm.bindTooltip(tips[i], null);
        _ = cm.addTo(m);

        _ = lf.marker(pt, .{
            .icon = lf.divIcon(.{
                .className = "mono-pin",
                .html = htmls[i],
                .iconSize = pin_size,
                .iconAnchor = pin_anchor,
            }),
        }).addTo(m);
    }

    _ = m.fitBounds(corridor.getBounds().pad(0.15), null);

    const note = lf.popup(pts[2], .{ .maxWidth = 220.0 });
    _ = note.setContent("03 Bern · 46.95N, 7.45E");
    _ = note.openOn(m);

    g_state.map = m.retain();
    g_state.note = note.retain();

    _ = m.onWithCanvasAddEventListenerTypeClick(.click, .{ onClick, &g_state }, null);
    const payload = lf.LeafletMouseEvent.init(.{ .latlng = pts[2] });
    _ = m.fire("click", .{ .data = payload });
    _ = m.fire("click", .{ .data = payload });
}
