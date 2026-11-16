// Leaflet from C++, in a page: a map on the page's element, layers on it,
// geometry, and events. No tiles: nothing is fetched. Every CHECK must hold.
#include <cmath>

#include "bindings.hpp"

extern "C" TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

// A generated object where a ts::Value is wanted (`data: object`): same handle.
static ts::Value val_of(const ts::Handle& h) {
    ts::Value v;
    static_cast<ts::Handle&>(v) = h;
    return v;
}

static bool near(double a, double b, double eps = 1e-6) { return std::fabs(a - b) < eps; }

extern "C" __attribute__((export_name("run"))) void run() {
    // A map in the page's #map, centred on London.
    LatLng london = leaflet::latLng(51.505, -0.09);
    // Animations off: a zoom or pan then takes effect at once, not at transition end.
    MapOptions still = MapOptions::create();
    still.zoomAnimation(false);
    still.fadeAnimation(false);
    still.markerZoomAnimation(false);
    Map m = leaflet::map(test_page::element("map"), still);
    m.setView(london, 13);
    CHECK(m.getZoom() == 13);
    CHECK(m.getCenter().equals(london, 1e-9));
    CHECK(m.getSize().x() == 400 && m.getSize().y() == 300);
    CHECK(!leaflet::get_version().empty());

    // The centre is the middle of the container.
    Point middle = m.latLngToContainerPoint(london);
    CHECK(middle.x() == 200 && middle.y() == 150);
    LatLng back = m.containerPointToLatLng(leaflet::point(200, 150));
    CHECK(near(back.lat(), 51.505, 1e-3) && near(back.lng(), -0.09, 1e-3));

    // Geometry: distances in metres, bounds that grow.
    LatLng paris = leaflet::latLng(48.8566, 2.3522);
    double km = london.distanceTo(paris) / 1000;
    CHECK(km > 340 && km < 345);
    LatLngBounds bounds = leaflet::latLngBounds(london, london);
    bounds.extend(paris);
    CHECK(bounds.contains(leaflet::latLng(50, 1)));
    CHECK(!bounds.contains(leaflet::latLng(40, 1)));
    CHECK(bounds.getNorthEast().lat() == 51.505);
    CHECK(bounds.getSouthWest().lat() == 48.8566);
    CHECK(near(bounds.getWest(), -0.09) && near(bounds.getEast(), 2.3522));
    CHECK(bounds.isValid());
    CHECK(bounds.pad(0.5).getNorth() > bounds.getNorth());

    // A LatLng where a LatLngExpression is wanted: itself, a literal object, or an array.
    LatLngLiteral lit = LatLngLiteral::create();
    lit.lat(10);
    lit.lng(20);
    CHECK(leaflet::latLng(1, 2).equals(LatLngExpression(leaflet::latLng(1, 2))));
    CHECK(leaflet::latLng(10, 20).equals(LatLngExpression(lit)));
    CHECK(leaflet::latLng(1, 2).toString() == "LatLng(1, 2)");
    CHECK(leaflet::latLng(5, 190).wrap().lng() == -170);

    // Points.
    Point p = leaflet::point(3, 4);
    CHECK(p.distanceTo(PointExpression(leaflet::point(0, 0))) == 5);
    CHECK(p.add(PointExpression(leaflet::point(1, 1))).x() == 4);
    CHECK(p.multiplyBy(2).y() == 8);
    CHECK(p.x() == 3);

    // Layers: a marker added to the map and found there, then removed.
    Marker<> pin = leaflet::marker(paris);
    pin.addTo(m);
    CHECK(m.hasLayer(pin));
    CHECK(pin.getLatLng().equals(paris));
    pin.setLatLng(london);
    CHECK(pin.getLatLng().equals(london));
    pin.setLatLng(paris);
    LayerGroup<> group = leaflet::layerGroup();
    Circle<> disc = leaflet::circle(london, 500);
    group.addLayer(disc);
    group.addTo(m);
    CHECK(group.hasLayer(disc));
    CHECK(group.getLayers().size() == 1);
    m.removeLayer(pin);
    CHECK(!m.hasLayer(pin) && m.hasLayer(group));
    CHECK(disc.getRadius() == 500);
    CHECK(disc.getBounds().contains(london));

    // Shapes: a polyline, a polygon and a rectangle, their bounds.
    ts::Array<> path = ts::Array<>::create();
    path.push(LatLngExpression(london));
    path.push(LatLngExpression(paris));
    Polyline<> route = leaflet::polyline(path);
    CHECK(route.getLatLngs().size() == 2);
    CHECK(route.getBounds().contains(london) && route.getBounds().contains(paris));
    route.addTo(m);
    CHECK(m.hasLayer(route));
    Rectangle<> box = leaflet::rectangle(LatLngBoundsExpression(bounds));
    CHECK(box.getBounds().equals(LatLngBoundsExpression(bounds)));
    CircleMarker<> dot = leaflet::circleMarker(paris);
    dot.setRadius(12);
    CHECK(dot.getRadius() == 12);

    // eachLayer visits what is on the map.
    int layers = 0;
    m.eachLayer(Layer_fn([&](Layer) { layers++; }));
    CHECK(layers >= 2);

    // Fitting the bounds zooms out to show both cities.
    m.fitBounds(bounds);
    CHECK(m.getZoom() < 13);
    CHECK(m.getBounds().contains(paris) && m.getBounds().contains(london));
    m.setZoom(5);
    CHECK(m.getZoom() == 5);
    m.zoomIn();
    CHECK(m.getZoom() == 6);
    PanOptions instant = PanOptions::create();
    instant.animate(false);
    m.panTo(paris, instant);
    CHECK(m.getCenter().distanceTo(paris) < 1000);

    // Project to pixels and back at a zoom.
    Point world = m.project(london, 10);
    LatLng unprojected = m.unproject(PointExpression(world), 10);
    CHECK(near(unprojected.lat(), 51.505, 1e-6));

    // An event fired on the map reaches the listener, with its position.
    int clicks = 0;
    LeafletMouseEventHandlerFn onClick = [&](LeafletMouseEvent e) {
        if (e.latlng().lat() > 0) clicks++;
    };
    m.on(Evented_addEventListener_type_click::click, onClick);
    LeafletMouseEvent data = LeafletMouseEvent::create();
    data.latlng(london);
    m.fire("click", val_of(data));
    m.fire("click", val_of(data));
    CHECK(clicks == 2);
    // Removed: the listener hears nothing more.
    m.off(Evented_addEventListener_type_click::click, onClick);
    m.fire("click", val_of(data));
    CHECK(clicks == 2);
    onClick.release();

    // A custom event by name, a plain LeafletEvent listener.
    std::string heard;
    LeafletEventHandlerFn onPing = [&](LeafletEvent e) { heard = e.type(); };
    m.on("ping", onPing);
    m.fire("ping");
    CHECK(heard == "ping");
    onPing.release();

    // zoom and view-change events as the map moves.
    int zooms = 0;
    LeafletEventHandlerFn onZoom = [&](LeafletEvent) { zooms++; };
    m.on("zoomend", onZoom);
    m.setZoom(9, ZoomPanOptions::create());
    CHECK(m.getZoom() == 9);
    CHECK(zooms >= 1);
    // Not released: an animated move that is still running fires zoomend later.

    // Options on a map made here: zoom limits.
    MapOptions limited = MapOptions::create();
    limited.minZoom(3);
    limited.maxZoom(8);
    limited.zoomControl(false);
    limited.zoomAnimation(false);
    limited.center(LatLngExpression(london));
    limited.zoom(5);
    Map small = leaflet::map(test_page::element("stage"), limited);
    CHECK(small.getZoom() == 5);
    CHECK(small.getMinZoom() == 3);
    CHECK(near(small.getCenter().lat(), 51.505, 1e-6));
    small.setZoom(20);
    CHECK(small.getZoom() == 8);
    small.remove();
}
