#include "dom/dom.h"
#include "leaflet/leaflet.h"
#include <stdio.h>

static Leaflet_PolylineOptions poly_opts(const char* color, double weight, const char* dash, const char* fill_color, double fill_opacity) {
    Leaflet_PolylineOptions opts = Leaflet_PolylineOptions_create(
        .smoothFactor = 1.0, .color = color, .weight = weight, .fillColor = fill_color, .fillOpacity = fill_opacity);
    if (dash != 0) Leaflet_PolylineOptions_set_dashArray(opts, dash);
    return opts;
}

__attribute__((export_name("start"))) void start(void) {
    js_scope();

    Leaflet_Map m = leaflet_map("map", Leaflet_MapOptions_create(
        .attributionControl = false, .zoomAnimation = false, .fadeAnimation = false, .markerZoomAnimation = false));

    JsArray_Leaflet_LatLng mid_pts = Leaflet_LatLng_list(
        leaflet_latLng(47.2, 7.6),
        leaflet_latLng(48.0, 8.6),
        leaflet_latLng(47.5, 9.7),
        leaflet_latLng(46.8, 9.5),
        leaflet_latLng(46.6, 8.4),
        leaflet_latLng(46.9, 7.3));

    JsArray_Leaflet_LatLng core_pts = Leaflet_LatLng_list(
        leaflet_latLng(47.35, 8.2),
        leaflet_latLng(47.55, 8.75),
        leaflet_latLng(47.15, 9.0),
        leaflet_latLng(46.95, 8.4));

    Leaflet_Polygon mid_poly = leaflet_polygon(mid_pts, poly_opts("#404040", 2.0, 0, "#d4d4d4", 0.55));
    Leaflet_Polygon core_poly = leaflet_polygon(core_pts, poly_opts("#111111", 2.5, 0, "#404040", 0.75));

    Leaflet_LatLngBounds mid_bounds = Leaflet_Polygon_getBounds(mid_poly);
    Leaflet_LatLngBounds core_bounds = Leaflet_Polygon_getBounds(core_poly);
    Leaflet_LatLngBounds padded = Leaflet_LatLngBounds_pad(mid_bounds, 0.18);
    Leaflet_Rectangle hull_rect = leaflet_rectangle(padded, poly_opts("#737373", 1.5, "4 4", "#efefef", 0.05));

    Leaflet_Rectangle_addTo(hull_rect, m);
    Leaflet_Polygon_addTo(mid_poly, m);
    Leaflet_Polygon_addTo(core_poly, m);

    Leaflet_LatLng probes[8] = {
        leaflet_latLng(47.25, 8.6),
        leaflet_latLng(47.15, 8.55),
        leaflet_latLng(47.7, 8.6),
        leaflet_latLng(47.0, 9.2),
        leaflet_latLng(46.8, 7.9),
        leaflet_latLng(47.85, 7.5),
        leaflet_latLng(47.9, 9.7),
        leaflet_latLng(46.55, 9.6),
    };

    int in_core = 0;
    int in_mid = 0;
    int exterior = 0;

    for (int i = 0; i < 8; i++) {
        Leaflet_LatLng pt = probes[i];
        const char* fill_hex = "#ffffff";
        if (Leaflet_LatLngBounds_contains(core_bounds, pt)) {
            in_core++;
            fill_hex = "#111111";
        } else if (Leaflet_LatLngBounds_contains(mid_bounds, pt)) {
            in_mid++;
            fill_hex = "#737373";
        } else {
            exterior++;
        }
        Leaflet_CircleMarker_addTo(leaflet_circleMarker(pt, Leaflet_CircleMarkerOptions_create(
            .radius = 6.0, .color = "#111111", .weight = 2.0, .fillColor = fill_hex, .fillOpacity = 1.0)), m);
    }

    Leaflet_Map_fitBounds(m, padded);

    char buf[160];
    snprintf(buf, sizeof buf, "2 nested polygons · 1 padded hull · probes: %d core · %d sector · %d exterior",
        in_core, in_mid, exterior);
    HTMLElement meta = Document_getElementById(get_document(), "meta");
    if (meta.h != 0) HTMLElement_set_textContent(meta, buf);
}
