#include "dom/dom.h"
#include "leaflet/leaflet.h"
#include <stdio.h>

static Leaflet_MapOptions make_map_opts(void) {
    return Leaflet_MapOptions_create(
        .attributionControl = false, .zoomAnimation = false, .fadeAnimation = false, .markerZoomAnimation = false);
}

static Leaflet_PolylineOptions make_poly_opts(const char* color, double weight, const char* dash, double fill_opacity) {
    Leaflet_PolylineOptions opts = Leaflet_PolylineOptions_create(
        .smoothFactor = 1.0, .color = color, .weight = weight, .fillOpacity = fill_opacity);
    if (dash != 0) Leaflet_PolylineOptions_set_dashArray(opts, dash);
    return opts;
}

__attribute__((export_name("start"))) void start(void) {
    js_scope();

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
        Leaflet_Polyline_addTo(leaflet_polyline(Leaflet_LatLng_list(
            leaflet_latLng(lats[i], -2.0), leaflet_latLng(lats[i], 17.0)), grat_opts), m);
    }
    static const double lngs[5] = { 0.0, 4.0, 8.0, 12.0, 16.0 };
    for (int i = 0; i < 5; i++) {
        Leaflet_Polyline_addTo(leaflet_polyline(Leaflet_LatLng_list(
            leaflet_latLng(45.0, lngs[i]), leaflet_latLng(55.0, lngs[i])), grat_opts), m);
    }

    JsArray_Leaflet_LatLng primary_pts = JsArray_Leaflet_LatLng_new();
    for (int i = 0; i < 6; i++) JsArray_Leaflet_LatLng_push(primary_pts, hubs[i]);
    Leaflet_Polyline primary_line = leaflet_polyline(primary_pts, make_poly_opts("#111111", 3.0, 0, 0.0));
    Leaflet_Polyline_addTo(primary_line, m);

    JsArray_Leaflet_LatLng alt_pts = Leaflet_LatLng_list(
        hubs[0], leaflet_latLng(50.8503, 4.3517), leaflet_latLng(52.3676, 4.9041), hubs[5]);
    Leaflet_Polyline_addTo(leaflet_polyline(alt_pts, make_poly_opts("#737373", 1.5, "5 5", 0.0)), m);

    Leaflet_LayerGroup nodes = leaflet_layerGroup();
    Leaflet_CircleMarkerOptions node_opts = Leaflet_CircleMarkerOptions_create(
        .radius = 6.0, .color = "#111111", .weight = 2.0, .fillColor = "#ffffff", .fillOpacity = 1.0);

    for (int i = 0; i < 6; i++) {
        Leaflet_CircleMarker cm = leaflet_circleMarker(hubs[i], node_opts);
        Leaflet_CircleMarker_bindTooltip(cm, hub_names[i]);
        Leaflet_LayerGroup_addLayer(nodes, cm);
    }
    Leaflet_LayerGroup_addTo(nodes, m);

    Leaflet_Map_fitBounds(m, Leaflet_Polyline_getBounds(primary_line));

    double total_km = 0.0;
    for (int i = 0; i < 5; i++) {
        total_km += Leaflet_LatLng_distanceTo(hubs[i], hubs[i + 1]) / 1000.0;
    }

    char buf[160];
    snprintf(buf, sizeof buf, "6 hubs · 2 routes · %d km total · EPSG:3857 vector SVG", (int)(total_km + 0.5));
    HTMLElement meta = Document_getElementById(get_document(), "meta");
    if (meta.h != 0) HTMLElement_set_textContent(meta, buf);
}
