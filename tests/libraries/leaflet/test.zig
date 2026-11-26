// Leaflet from Zig, in a page: a map on the page's element, layers on it, geometry and events.
// No tiles: nothing is fetched. Every expect must hold.
const std = @import("std");
const b = @import("bindings.zig");

extern "env" fn check(ok: i32, line: u32) void;

fn expect(ok: bool, src: std.builtin.SourceLocation) void {
    check(@intFromBool(ok), src.line);
}

fn ll(lat: f64, lng: f64) b.leaflet.LatLngExpression {
    return .{ .LatLng = b.leaflet.latLng(lat, lng) };
}

var clicks: u32 = 0;
var last_lat: f64 = 0;
fn onClick(_: ?*anyopaque, e: b.leaflet.LeafletMouseEvent) void {
    clicks += 1;
    last_lat = e.getLatlng().getLat();
}

var zooms: u32 = 0;
fn onZoom(ctx: ?*anyopaque, e: b.leaflet.LeafletEvent) void {
    const n: *u32 = @ptrCast(@alignCast(ctx));
    n.* += 1;
    _ = e;
}

var adds: u32 = 0;
var layer_seen: u32 = 0;
fn visit(ctx: ?*anyopaque, l: b.leaflet.Layer) void {
    const n: *u32 = @ptrCast(@alignCast(ctx));
    _ = l;
    n.* += 1;
}

export fn run() void {
    // A map in the page's #map, centred on London.
    const london = b.leaflet.latLng(51.505, -0.09);
    const m = b.leaflet.map(.{ .HTMLElement = b.test_page.element("map") });
    _ = m.setViewWith(.{ .LatLng = london }, .{ .zoom = 13 });
    expect(m.getZoom() == 13, @src());
    expect(m.getCenter().equalsWith(.{ .LatLng = london }, .{ .maxMargin = 1e-9 }), @src());
    expect(m.getSize().getX() == 400 and m.getSize().getY() == 300, @src());
    expect(b.jsSame(m.getContainer(), b.test_page.element("map")), @src());

    // The centre is the middle of the container.
    const middle = m.latLngToContainerPoint(.{ .LatLng = london });
    expect(middle.getX() == 200 and middle.getY() == 150, @src());
    // And back again: the top-left corner is north-west of the centre.
    const corner = m.containerPointToLatLng(.{ .Point = b.leaflet.point(0, 0) });
    expect(corner.getLat() > london.getLat() and corner.getLng() < london.getLng(), @src());

    // Geometry: distances in metres, bounds that grow.
    const paris = b.leaflet.latLng(48.8566, 2.3522);
    const km = london.distanceTo(.{ .LatLng = paris }) / 1000.0;
    expect(km > 340 and km < 345, @src());
    const bounds = b.leaflet.latLngBounds(.{ .LatLng = london }, .{ .LatLng = london });
    _ = bounds.extend(.{ .LatLng = paris });
    expect(bounds.contains(.{ .LatLng = b.leaflet.latLng(50, 1) }), @src());
    expect(!bounds.contains(.{ .LatLng = b.leaflet.latLng(40, 1) }), @src());
    expect(bounds.getNorthEast().getLat() == 51.505, @src());
    expect(bounds.getSouthWest().getLng() == -0.09, @src());
    expect(bounds.getSouth() == 48.8566 and bounds.getEast() == 2.3522, @src());
    expect(bounds.isValid(), @src());
    expect(bounds.getCenter().getLat() > 50.1 and bounds.getCenter().getLat() < 50.2, @src());
    expect(london.toString().len() > 0 and london.clone().getLat() == 51.505, @src());
    expect(london.getAlt() == null, @src());

    // Points.
    const p = b.leaflet.point(3, 4);
    expect(p.getX() == 3 and p.distanceTo(.{ .Point = b.leaflet.point(0, 0) }) == 5, @src());
    expect(p.add(.{ .Point = b.leaflet.point(1, 1) }).getX() == 4, @src());
    expect(p.multiplyBy(2).getY() == 8, @src());
    expect(b.leaflet.point(1.6, 2.4).round().getX() == 2, @src());

    // Layers: a marker added to the map and found there, then removed.
    const pin = b.leaflet.marker(.{ .LatLng = paris });
    _ = pin.addTo(.{ .Map = m });
    expect(m.hasLayer(pin.asLayer()), @src());
    expect(pin.getLatLng().equals(.{ .LatLng = paris }), @src());
    _ = pin.setLatLng(ll(48.0, 2.0));
    expect(pin.getLatLng().getLat() == 48.0, @src());
    _ = pin.setLatLng(.{ .LatLng = paris });

    const group = b.leaflet.layerGroup(.{});
    const ring = b.leaflet.circleWithRadius(.{ .LatLng = london }, 500);
    _ = group.addLayer(ring.asCircleMarker().asLayer());
    _ = group.addTo(.{ .Map = m });
    expect(group.getLayers().len() == 1, @src());
    expect(group.hasLayer(ring.asCircleMarker().asLayer()), @src());
    _ = m.removeLayer(pin.asLayer());
    expect(!m.hasLayer(pin.asLayer()) and m.hasLayer(group.asLayer()), @src());

    // A circle's radius and bounds around its centre.
    expect(ring.getRadius() == 500, @src());
    _ = ring.setRadius(1000);
    expect(ring.getRadius() == 1000, @src());
    expect(ring.getBounds().contains(.{ .LatLng = london }), @src());
    expect(ring.getLatLng().equals(.{ .LatLng = london }), @src());

    // A polyline through points.
    const path = b.JsArray.new();
    path.push(.{ .h = london.h });
    path.push(.{ .h = paris.h });
    const line = b.leaflet.polyline(path);
    expect(line.getLatLngs().len() == 2 and !line.isEmpty(), @src());
    expect(line.getBounds().contains(.{ .LatLng = london }), @src());
    _ = line.addTo(.{ .Map = m });

    // Layers can be walked.
    var seen: u32 = 0;
    _ = m.eachLayer(b.EachLayerFn.from(visit, &seen));
    expect(seen >= 2, @src());

    // Fitting the bounds zooms out to show both cities.
    _ = m.fitBounds(.{ .LatLngBounds = bounds });
    expect(m.getZoom() < 13, @src());
    expect(m.getBounds().contains(.{ .LatLng = paris }) and m.getBounds().contains(.{ .LatLng = london }), @src());
    expect(m.getBoundsZoom(.{ .LatLngBounds = bounds }) == m.getZoom(), @src());

    // Zoom and pan with no animation.
    const still = b.leaflet.ZoomPanOptions.create();
    still.setAnimate(false);
    _ = m.setViewWith(.{ .LatLng = london }, .{ .zoom = 10, .options = still });
    const no_anim = b.leaflet.ZoomOptions.create();
    no_anim.setAnimate(false);
    _ = m.zoomIn(.{ .options = no_anim });
    expect(m.getZoom() == 11, @src());
    _ = m.setZoomWith(8, .{ .options = still });
    expect(m.getZoom() == 8, @src());
    _ = m.panTo(.{ .LatLng = paris });
    expect(m.getMinZoom() >= 0, @src());
    // The projection and the distance between two points on the map.
    expect(m.projectWith(.{ .LatLng = london }, .{ .zoom = 0 }).getX() > 0, @src());
    expect(@abs(m.distance(.{ .LatLng = london }, .{ .LatLng = paris }) - london.distanceTo(.{ .LatLng = paris })) < 1, @src());

    // An event fired on the map reaches the listener, with its position.
    const on_click = b.leaflet.LeafletMouseEventHandlerFn.from(onClick, null);
    _ = m.onClick(on_click);
    const data = b.leaflet.LeafletMouseEvent.create();
    data.setLatlng(london);
    _ = m.fireWith("click", .{ .data = .{ .h = data.h } });
    _ = m.fireWith("click", .{ .data = .{ .h = data.h } });
    expect(clicks == 2 and last_lat == 51.505, @src());
    // Removed, it is not called again. FINDING 21: an option named `fn` is a Zig keyword, so it is written
    // .@"fn" = ...; and each event has its own overload (offClick).
    _ = m.offClick(.{ .@"fn" = on_click });
    _ = m.fireWith("click", .{ .data = .{ .h = data.h } });
    expect(clicks == 2, @src());

    // Events by name, and a listener that runs once.
    var named: u32 = 0;
    _ = m.on("custom", b.leaflet.LeafletEventHandlerFn.from(onZoom, &named));
    _ = m.fire("custom");
    _ = m.fire("custom");
    expect(named == 2, @src());
    var once: u32 = 0;
    _ = m.once("shot", b.leaflet.LeafletEventHandlerFn.from(onZoom, &once));
    _ = m.fire("shot");
    _ = m.fire("shot");
    expect(once == 1, @src());

    // The map's own events: zooming fires zoomend.
    var ends: u32 = 0;
    _ = m.on("zoomend", b.leaflet.LeafletEventHandlerFn.from(onZoom, &ends));
    _ = m.setZoomWith(9, .{ .options = still });
    expect(ends >= 1, @src());

    // Removing the map frees the container; a removed map no longer has its layers.
    _ = m.remove();
    expect(!m.hasLayer(group.asLayer()), @src());
}
