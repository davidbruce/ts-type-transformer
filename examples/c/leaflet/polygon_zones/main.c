#include "dom/dom.h"
#include "leaflet/leaflet.h"
#include <stdio.h>

static Leaflet_LatLngExpression E(Leaflet_LatLng ll) { return Leaflet_LatLngExpression_of_Leaflet_LatLng(ll); }

static Leaflet_PolylineOptions poly_opts(const char* color, double weight, const char* dash, const char* fill_color, double fill_opacity) {
    Leaflet_PolylineOptions opts = Leaflet_PolylineOptions_create();
    Leaflet_PolylineOptions_set_smoothFactor(opts, 1.0);
    Leaflet_PolylineOptions_set_color(opts, color);
    Leaflet_PolylineOptions_set_weight(opts, weight);
    if (dash != 0) {
        Leaflet_PolylineOptions_set_dashArray(opts, Leaflet_string_or_number_array_of(dash));
    }
    Leaflet_PolylineOptions_set_fillColor(opts, fill_color);
    Leaflet_PolylineOptions_set_fillOpacity(opts, fill_opacity);
    return opts;
}

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();

    Leaflet_MapOptions m_opts = Leaflet_MapOptions_create();
    Leaflet_MapOptions_set_attributionControl(m_opts, false);
    Leaflet_MapOptions_set_zoomAnimation(m_opts, false);
    Leaflet_MapOptions_set_fadeAnimation(m_opts, false);
    Leaflet_MapOptions_set_markerZoomAnimation(m_opts, false);
    Leaflet_Map m = leaflet_map("map", m_opts);

    JsArray_Leaflet_LatLngExpression mid_pts = JsArray_Leaflet_LatLngExpression_new();
    JsArray_Leaflet_LatLngExpression_push(mid_pts, E(leaflet_latLng(47.2, 7.6)));
    JsArray_Leaflet_LatLngExpression_push(mid_pts, E(leaflet_latLng(48.0, 8.6)));
    JsArray_Leaflet_LatLngExpression_push(mid_pts, E(leaflet_latLng(47.5, 9.7)));
    JsArray_Leaflet_LatLngExpression_push(mid_pts, E(leaflet_latLng(46.8, 9.5)));
    JsArray_Leaflet_LatLngExpression_push(mid_pts, E(leaflet_latLng(46.6, 8.4)));
    JsArray_Leaflet_LatLngExpression_push(mid_pts, E(leaflet_latLng(46.9, 7.3)));

    JsArray_Leaflet_LatLngExpression core_pts = JsArray_Leaflet_LatLngExpression_new();
    JsArray_Leaflet_LatLngExpression_push(core_pts, E(leaflet_latLng(47.35, 8.2)));
    JsArray_Leaflet_LatLngExpression_push(core_pts, E(leaflet_latLng(47.55, 8.75)));
    JsArray_Leaflet_LatLngExpression_push(core_pts, E(leaflet_latLng(47.15, 9.0)));
    JsArray_Leaflet_LatLngExpression_push(core_pts, E(leaflet_latLng(46.95, 8.4)));

    Leaflet_Polygon mid_poly = leaflet_polygon(mid_pts, poly_opts("#404040", 2.0, 0, "#d4d4d4", 0.55));
    Leaflet_Polygon core_poly = leaflet_polygon(core_pts, poly_opts("#111111", 2.5, 0, "#404040", 0.75));

    Leaflet_LatLngBounds mid_bounds = Leaflet_Polygon_getBounds(mid_poly);
    Leaflet_LatLngBounds core_bounds = Leaflet_Polygon_getBounds(core_poly);
    Leaflet_LatLngBounds padded = Leaflet_LatLngBounds_pad(mid_bounds, 0.18);
    Leaflet_Rectangle hull_rect = leaflet_rectangle(Leaflet_LatLngBoundsExpression_of_Leaflet_LatLngBounds(padded), poly_opts("#737373", 1.5, "4 4", "#efefef", 0.05));

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
        if (Leaflet_LatLngBounds_contains_with_LatLngExpression(core_bounds, E(pt))) {
            in_core++;
            fill_hex = "#111111";
        } else if (Leaflet_LatLngBounds_contains_with_LatLngExpression(mid_bounds, E(pt))) {
            in_mid++;
            fill_hex = "#737373";
        } else {
            exterior++;
        }
        Leaflet_CircleMarkerOptions cm_opts = Leaflet_CircleMarkerOptions_create();
        Leaflet_CircleMarkerOptions_set_radius(cm_opts, 6.0);
        Leaflet_CircleMarkerOptions_set_color(cm_opts, "#111111");
        Leaflet_CircleMarkerOptions_set_weight(cm_opts, 2.0);
        Leaflet_CircleMarkerOptions_set_fillColor(cm_opts, fill_hex);
        Leaflet_CircleMarkerOptions_set_fillOpacity(cm_opts, 1.0);
        Leaflet_CircleMarker_addTo(leaflet_circleMarker(E(pt), cm_opts), m);
    }

    Leaflet_Map_fitBounds(m, Leaflet_LatLngBoundsExpression_of_Leaflet_LatLngBounds(padded));

    char buf[160];
    snprintf(buf, sizeof buf, "2 nested polygons · 1 padded hull · probes: %d core · %d sector · %d exterior",
        in_core, in_mid, exterior);
    HTMLElement meta = Document_getElementById(get_document(), "meta");
    if (meta.h != 0) HTMLElement_set_textContent(meta, buf);
    js_scope_close();
}
