// Leaflet from C, in a page: a map on the page's element, layers on it,
// geometry, and events. No tiles: nothing is fetched. Every CHECK must hold.
#include "bindings.h"

TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

static LatLngExpression E(LatLng l) { return LatLngExpression_of_LatLng(l); }
static bool near(double a, double b, double eps) { return (a > b ? a - b : b - a) < eps; }

static int clicks, positive_clicks, once_clicks, named;
static void on_click(void* ctx, LeafletMouseEvent e) {
    clicks++;
    if (LatLng_get_lat(LeafletMouseEvent_get_latlng(e)) > 0) positive_clicks++;
}
static void on_once(void* ctx, LeafletMouseEvent e) { once_clicks++; }
static void on_named(void* ctx, LeafletEvent e) { named++; }
static int layers_seen;
static void count_layer(void* ctx, Layer l) { layers_seen++; }

__attribute__((export_name("run"))) void run(void) {
    // A map in the page's #map, centred on London.
    LatLng london = latLng(51.505, -0.09);
    Map m = map(element("map"));
    Map_setView(m, E(london), js_some_double(13), (ZoomPanOptions){ 0 });
    CHECK(Map_getZoom(m) == 13);
    CHECK(LatLng_equals(Map_getCenter(m), E(london), js_some_double(1e-9)));
    Point size = Map_getSize(m);
    CHECK(Point_get_x(size) == 400 && Point_get_y(size) == 300);
    CHECK(js_same(Map_getContainer(m), element("map")));

    // The centre is the middle of the container.
    Point middle = Map_latLngToContainerPoint(m, E(london));
    CHECK(Point_get_x(middle) == 200 && Point_get_y(middle) == 150);
    LatLng back = Map_containerPointToLatLng(m, PointExpression_of_Point(Point_new(200, 150, js_none_bool())));
    CHECK(near(LatLng_get_lat(back), 51.505, 1e-4) && near(LatLng_get_lng(back), -0.09, 1e-4));

    // Geometry: distances in metres, bounds that grow.
    LatLng paris = latLng(48.8566, 2.3522);
    double km = LatLng_distanceTo(london, E(paris)) / 1000;
    CHECK(km > 340 && km < 345);
    CHECK(near(LatLng_get_lat(paris), 48.8566, 1e-12) && !LatLng_get_alt(paris).has);
    LatLngBounds bounds = latLngBounds(E(london), E(london));
    LatLngBounds_extend(bounds, paris);
    CHECK(LatLngBounds_contains(bounds, latLng(50, 1)));
    CHECK(LatLng_get_lat(LatLngBounds_getNorthEast(bounds)) == 51.505);
    CHECK(near(LatLngBounds_getWest(bounds), -0.09, 1e-12) && near(LatLngBounds_getEast(bounds), 2.3522, 1e-12));
    CHECK(near(LatLng_get_lat(LatLngBounds_getCenter(bounds)), (51.505 + 48.8566) / 2, 1e-9));
    // Points do arithmetic.
    Point p = Point_new(3, 4, js_none_bool());
    CHECK(Point_distanceTo(p, PointExpression_of_Point(Point_new(0, 0, js_none_bool()))) == 5);
    Point doubled = Point_multiplyBy(Point_clone(p), 2);
    CHECK(Point_get_x(doubled) == 6 && Point_get_y(doubled) == 8 && Point_get_x(p) == 3);

    // Layers: a marker added to the map and found there, then removed.
    Marker pin = marker(E(paris));
    Layer_addTo(pin, m);
    CHECK(Map_hasLayer(m, pin));
    CHECK(LatLng_equals(Marker_getLatLng(pin), E(paris), (JsOptionalDouble){ 0 }));
    Marker_setLatLng(pin, E(london));
    CHECK(LatLng_equals(Marker_getLatLng(pin), E(london), js_none_double()));
    Marker_setLatLng(pin, E(paris));
    LayerGroup group = layerGroup((JsArray_Layer){ 0 });
    LayerGroup_addLayer(group, circle_with_radius(E(london), 500));
    Layer_addTo(group, m);
    Map_removeLayer(m, pin);
    CHECK(!Map_hasLayer(m, pin) && Map_hasLayer(m, group));
    layers_seen = 0;
    Map_eachLayer(m, EachLayerFn_from(count_layer, 0), (JsValue){ 0 });
    CHECK(layers_seen >= 2);

    // A circle's bounds reach about 500 metres out.
    Circle ring = circle_with_radius(E(london), 500);
    Layer_addTo(ring, m);
    LatLngBounds ring_bounds = Circle_getBounds(ring);
    double north_m = LatLng_distanceTo(london, E(LatLngBounds_getNorthEast(ring_bounds)));
    CHECK(north_m > 500 && north_m < 800);

    // Fitting the bounds zooms out to show both cities.
    Map_fitBounds(m, LatLngBoundsExpression_of(bounds), (FitBoundsOptions){ 0 });
    CHECK(Map_getZoom(m) < 13);
    CHECK(LatLngBounds_contains(Map_getBounds(m), paris));
    CHECK(LatLngBounds_contains(Map_getBounds(m), london));

    // An event fired on the map reaches the listener, with its position.
    LeafletMouseEventHandlerFn handler = LeafletMouseEventHandlerFn_from(on_click, 0);
    Evented_on_click(m, handler, (JsValue){ 0 });
    LeafletMouseEvent data = LeafletMouseEvent_create();
    LeafletMouseEvent_set_latlng(data, london);
    Evented_fire(m, "click", (JsValue){ data.h }, js_none_bool());
    Evented_fire(m, "click", (JsValue){ data.h }, js_none_bool());
    CHECK(clicks == 2 && positive_clicks == 2);
    CHECK(Evented_listens_click_with_fn_and_context(m, handler, (JsValue){ 0 }, js_none_bool()));
    // Removed, it is not called again.
    Evented_off_click(m, handler, (JsValue){ 0 });
    Evented_fire(m, "click", (JsValue){ data.h }, js_none_bool());
    CHECK(clicks == 2);
    // `once` is called once.
    Evented_once_click(m, LeafletMouseEventHandlerFn_from(on_once, 0), (JsValue){ 0 });
    Evented_fire(m, "click", (JsValue){ data.h }, js_none_bool());
    Evented_fire(m, "click", (JsValue){ data.h }, js_none_bool());
    CHECK(once_clicks == 1);
    // An event by name.
    Evented_on(m, "custom", LeafletEventHandlerFn_from(on_named, 0), (JsValue){ 0 });
    Evented_fire(m, "custom", (JsValue){ 0 }, js_none_bool());
    CHECK(named == 1);

    // Zoom changes are events too: setZoom moves the zoom.
    // (not animated: an animated zoom lands later.)
    ZoomPanOptions still = ZoomPanOptions_create();
    PanOptions_set_animate(still, js_some_bool(false));
    Map_setZoom(m, 5, still);
    CHECK(Map_getZoom(m) == 5);
    Map_remove(m);
}
