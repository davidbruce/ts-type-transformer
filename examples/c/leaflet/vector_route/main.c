#include "dom/dom.h"
#include "leaflet/leaflet.h"
#include <stdio.h>

static LatLngExpression E(LatLng ll) { return LatLngExpression_of_LatLng(ll); }

static MapOptions make_map_opts(void) {
    MapOptions opts = MapOptions_create();
    MapOptions_set_attributionControl(opts, false);
    MapOptions_set_zoomAnimation(opts, false);
    MapOptions_set_fadeAnimation(opts, false);
    MapOptions_set_markerZoomAnimation(opts, false);
    return opts;
}

static PolylineOptions make_poly_opts(const char* color, double weight, const char* dash, double fill_opacity) {
    PolylineOptions opts = PolylineOptions_create();
    PolylineOptions_set_smoothFactor(opts, 1.0);
    PolylineOptions_set_color(opts, color);
    PolylineOptions_set_weight(opts, weight);
    if (dash != 0) {
        PolylineOptions_set_dashArray(opts, string_or_number_array_of(dash));
    }
    PolylineOptions_set_fillOpacity(opts, fill_opacity);
    return opts;
}

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();

    Map_2 m = map("map", make_map_opts());

    const char* hub_names[6] = { "London", "Paris", "Zurich", "Munich", "Prague", "Berlin" };
    LatLng hubs[6] = {
        latLng(51.505, -0.09),
        latLng(48.8566, 2.3522),
        latLng(47.3769, 8.5417),
        latLng(48.1351, 11.5820),
        latLng(50.0755, 14.4378),
        latLng(52.5200, 13.4050),
    };

    PolylineOptions grat_opts = make_poly_opts("#e5e5e5", 1.0, 0, 0.0);
    static const double lats[5] = { 46.0, 48.0, 50.0, 52.0, 54.0 };
    for (int i = 0; i < 5; i++) {
        JsArray_LatLngExpression pts = JsArray_LatLngExpression_new();
        JsArray_LatLngExpression_push(pts, E(latLng(lats[i], -2.0)));
        JsArray_LatLngExpression_push(pts, E(latLng(lats[i], 17.0)));
        Polyline_addTo(polyline(pts, grat_opts), m);
    }
    static const double lngs[5] = { 0.0, 4.0, 8.0, 12.0, 16.0 };
    for (int i = 0; i < 5; i++) {
        JsArray_LatLngExpression pts = JsArray_LatLngExpression_new();
        JsArray_LatLngExpression_push(pts, E(latLng(45.0, lngs[i])));
        JsArray_LatLngExpression_push(pts, E(latLng(55.0, lngs[i])));
        Polyline_addTo(polyline(pts, grat_opts), m);
    }

    JsArray_LatLngExpression primary_pts = JsArray_LatLngExpression_new();
    for (int i = 0; i < 6; i++) JsArray_LatLngExpression_push(primary_pts, E(hubs[i]));
    Polyline primary_line = polyline(primary_pts, make_poly_opts("#111111", 3.0, 0, 0.0));
    Polyline_addTo(primary_line, m);

    JsArray_LatLngExpression alt_pts = JsArray_LatLngExpression_new();
    JsArray_LatLngExpression_push(alt_pts, E(hubs[0]));
    JsArray_LatLngExpression_push(alt_pts, E(latLng(50.8503, 4.3517)));
    JsArray_LatLngExpression_push(alt_pts, E(latLng(52.3676, 4.9041)));
    JsArray_LatLngExpression_push(alt_pts, E(hubs[5]));
    Polyline_addTo(polyline(alt_pts, make_poly_opts("#737373", 1.5, "5 5", 0.0)), m);

    LayerGroup nodes = layerGroup();
    CircleMarkerOptions node_opts = CircleMarkerOptions_create();
    CircleMarkerOptions_set_radius(node_opts, 6.0);
    CircleMarkerOptions_set_color(node_opts, "#111111");
    CircleMarkerOptions_set_weight(node_opts, 2.0);
    CircleMarkerOptions_set_fillColor(node_opts, "#ffffff");
    CircleMarkerOptions_set_fillOpacity(node_opts, 1.0);

    for (int i = 0; i < 6; i++) {
        CircleMarker cm = circleMarker(E(hubs[i]), node_opts);
        CircleMarker_bindTooltip(cm, Content_of(hub_names[i]));
        LayerGroup_addLayer(nodes, cm);
    }
    LayerGroup_addTo(nodes, m);

    Map_2_fitBounds(m, LatLngBoundsExpression_of_LatLngBounds(Polyline_getBounds(primary_line)));

    double total_km = 0.0;
    for (int i = 0; i < 5; i++) {
        total_km += LatLng_distanceTo(hubs[i], E(hubs[i + 1])) / 1000.0;
    }

    char buf[160];
    snprintf(buf, sizeof buf, "6 hubs · 2 routes · %d km total · EPSG:3857 vector SVG", (int)(total_km + 0.5));
    HTMLElement meta = Document_getElementById(get_document(), "meta");
    if (meta.h != 0) HTMLElement_set_textContent(meta, buf);
    js_scope_close();
}
