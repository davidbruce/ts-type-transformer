#include "dom/dom.h"
#include "leaflet/leaflet.h"
#include <stdio.h>

static Leaflet_LatLngExpression E(Leaflet_LatLng ll) { return Leaflet_LatLngExpression_of_Leaflet_LatLng(ll); }

static Leaflet_MapOptions make_map_opts(void) {
    Leaflet_MapOptions opts = Leaflet_MapOptions_create();
    Leaflet_MapOptions_set_attributionControl(opts, false);
    Leaflet_MapOptions_set_zoomAnimation(opts, false);
    Leaflet_MapOptions_set_fadeAnimation(opts, false);
    Leaflet_MapOptions_set_markerZoomAnimation(opts, false);
    return opts;
}

static Leaflet_PolylineOptions make_poly_opts(const char* color, double weight, const char* dash, double fill_opacity) {
    Leaflet_PolylineOptions opts = Leaflet_PolylineOptions_create();
    Leaflet_PolylineOptions_set_smoothFactor(opts, 1.0);
    Leaflet_PolylineOptions_set_color(opts, color);
    Leaflet_PolylineOptions_set_weight(opts, weight);
    if (dash != 0) {
        Leaflet_PolylineOptions_set_dashArray(opts, Leaflet_string_or_number_array_of(dash));
    }
    Leaflet_PolylineOptions_set_fillOpacity(opts, fill_opacity);
    return opts;
}

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();

    Leaflet_Map m = leaflet_map("map", make_map_opts());

    const char* hub_names[6] = { "London", "Paris", "Zurich", "Munich", "Prague", "Berlin" };
    Leaflet_LatLng hubs[6] = {
        leaflet_latLng(51.505, -0.09),
        leaflet_latLng(48.8566, 2.3522),
        leaflet_latLng(47.3769, 8.5417),
        leaflet_latLng(48.1351, 11.5820),
        leaflet_latLng(50.0755, 14.4378),
        leaflet_latLng(52.5200, 13.4050),
    };

    Leaflet_PolylineOptions grat_opts = make_poly_opts("#e5e5e5", 1.0, 0, 0.0);
    static const double lats[5] = { 46.0, 48.0, 50.0, 52.0, 54.0 };
    for (int i = 0; i < 5; i++) {
        JsArray_Leaflet_LatLngExpression pts = JsArray_Leaflet_LatLngExpression_new();
        JsArray_Leaflet_LatLngExpression_push(pts, E(leaflet_latLng(lats[i], -2.0)));
        JsArray_Leaflet_LatLngExpression_push(pts, E(leaflet_latLng(lats[i], 17.0)));
        Leaflet_Polyline_addTo(leaflet_polyline(pts, grat_opts), m);
    }
    static const double lngs[5] = { 0.0, 4.0, 8.0, 12.0, 16.0 };
    for (int i = 0; i < 5; i++) {
        JsArray_Leaflet_LatLngExpression pts = JsArray_Leaflet_LatLngExpression_new();
        JsArray_Leaflet_LatLngExpression_push(pts, E(leaflet_latLng(45.0, lngs[i])));
        JsArray_Leaflet_LatLngExpression_push(pts, E(leaflet_latLng(55.0, lngs[i])));
        Leaflet_Polyline_addTo(leaflet_polyline(pts, grat_opts), m);
    }

    JsArray_Leaflet_LatLngExpression primary_pts = JsArray_Leaflet_LatLngExpression_new();
    for (int i = 0; i < 6; i++) JsArray_Leaflet_LatLngExpression_push(primary_pts, E(hubs[i]));
    Leaflet_Polyline primary_line = leaflet_polyline(primary_pts, make_poly_opts("#111111", 3.0, 0, 0.0));
    Leaflet_Polyline_addTo(primary_line, m);

    JsArray_Leaflet_LatLngExpression alt_pts = JsArray_Leaflet_LatLngExpression_new();
    JsArray_Leaflet_LatLngExpression_push(alt_pts, E(hubs[0]));
    JsArray_Leaflet_LatLngExpression_push(alt_pts, E(leaflet_latLng(50.8503, 4.3517)));
    JsArray_Leaflet_LatLngExpression_push(alt_pts, E(leaflet_latLng(52.3676, 4.9041)));
    JsArray_Leaflet_LatLngExpression_push(alt_pts, E(hubs[5]));
    Leaflet_Polyline_addTo(leaflet_polyline(alt_pts, make_poly_opts("#737373", 1.5, "5 5", 0.0)), m);

    Leaflet_LayerGroup nodes = leaflet_layerGroup();
    Leaflet_CircleMarkerOptions node_opts = Leaflet_CircleMarkerOptions_create();
    Leaflet_CircleMarkerOptions_set_radius(node_opts, 6.0);
    Leaflet_CircleMarkerOptions_set_color(node_opts, "#111111");
    Leaflet_CircleMarkerOptions_set_weight(node_opts, 2.0);
    Leaflet_CircleMarkerOptions_set_fillColor(node_opts, "#ffffff");
    Leaflet_CircleMarkerOptions_set_fillOpacity(node_opts, 1.0);

    for (int i = 0; i < 6; i++) {
        Leaflet_CircleMarker cm = leaflet_circleMarker(E(hubs[i]), node_opts);
        Leaflet_CircleMarker_bindTooltip(cm, Leaflet_Content_of(hub_names[i]));
        Leaflet_LayerGroup_addLayer(nodes, cm);
    }
    Leaflet_LayerGroup_addTo(nodes, m);

    Leaflet_Map_fitBounds(m, Leaflet_LatLngBoundsExpression_of_Leaflet_LatLngBounds(Leaflet_Polyline_getBounds(primary_line)));

    double total_km = 0.0;
    for (int i = 0; i < 5; i++) {
        total_km += Leaflet_LatLng_distanceTo(hubs[i], E(hubs[i + 1])) / 1000.0;
    }

    char buf[160];
    snprintf(buf, sizeof buf, "6 hubs · 2 routes · %d km total · EPSG:3857 vector SVG", (int)(total_km + 0.5));
    HTMLElement meta = Document_getElementById(get_document(), "meta");
    if (meta.h != 0) HTMLElement_set_textContent(meta, buf);
    js_scope_close();
}
