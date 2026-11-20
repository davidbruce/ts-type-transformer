#include "dom/dom.h"
#include "leaflet/leaflet.h"
#include <stdio.h>

static LatLngExpression E(LatLng ll) { return LatLngExpression_of_LatLng(ll); }

static PolylineOptions poly_opts(const char* color, double weight, const char* dash, const char* fill_color, double fill_opacity) {
    PolylineOptions opts = PolylineOptions_create();
    PolylineOptions_set_smoothFactor(opts, 1.0);
    PolylineOptions_set_color(opts, color);
    PolylineOptions_set_weight(opts, weight);
    if (dash != 0) {
        PolylineOptions_set_dashArray(opts, string_or_number_array_of(dash));
    }
    PolylineOptions_set_fillColor(opts, fill_color);
    PolylineOptions_set_fillOpacity(opts, fill_opacity);
    return opts;
}

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();

    MapOptions m_opts = MapOptions_create();
    MapOptions_set_attributionControl(m_opts, false);
    MapOptions_set_zoomAnimation(m_opts, false);
    MapOptions_set_fadeAnimation(m_opts, false);
    MapOptions_set_markerZoomAnimation(m_opts, false);
    Map_2 m = map("map", m_opts);

    JsArray_LatLngExpression mid_pts = JsArray_LatLngExpression_new();
    JsArray_LatLngExpression_push(mid_pts, E(latLng(47.2, 7.6)));
    JsArray_LatLngExpression_push(mid_pts, E(latLng(48.0, 8.6)));
    JsArray_LatLngExpression_push(mid_pts, E(latLng(47.5, 9.7)));
    JsArray_LatLngExpression_push(mid_pts, E(latLng(46.8, 9.5)));
    JsArray_LatLngExpression_push(mid_pts, E(latLng(46.6, 8.4)));
    JsArray_LatLngExpression_push(mid_pts, E(latLng(46.9, 7.3)));

    JsArray_LatLngExpression core_pts = JsArray_LatLngExpression_new();
    JsArray_LatLngExpression_push(core_pts, E(latLng(47.35, 8.2)));
    JsArray_LatLngExpression_push(core_pts, E(latLng(47.55, 8.75)));
    JsArray_LatLngExpression_push(core_pts, E(latLng(47.15, 9.0)));
    JsArray_LatLngExpression_push(core_pts, E(latLng(46.95, 8.4)));

    Polygon mid_poly = polygon(mid_pts, poly_opts("#404040", 2.0, 0, "#d4d4d4", 0.55));
    Polygon core_poly = polygon(core_pts, poly_opts("#111111", 2.5, 0, "#404040", 0.75));

    LatLngBounds mid_bounds = Polygon_getBounds(mid_poly);
    LatLngBounds core_bounds = Polygon_getBounds(core_poly);
    LatLngBounds padded = LatLngBounds_pad(mid_bounds, 0.18);
    Rectangle hull_rect = rectangle(LatLngBoundsExpression_of_LatLngBounds(padded), poly_opts("#737373", 1.5, "4 4", "#efefef", 0.05));

    Rectangle_addTo(hull_rect, m);
    Polygon_addTo(mid_poly, m);
    Polygon_addTo(core_poly, m);

    LatLng probes[8] = {
        latLng(47.25, 8.6),
        latLng(47.15, 8.55),
        latLng(47.7, 8.6),
        latLng(47.0, 9.2),
        latLng(46.8, 7.9),
        latLng(47.85, 7.5),
        latLng(47.9, 9.7),
        latLng(46.55, 9.6),
    };

    int in_core = 0;
    int in_mid = 0;
    int exterior = 0;

    for (int i = 0; i < 8; i++) {
        LatLng pt = probes[i];
        const char* fill_hex = "#ffffff";
        if (LatLngBounds_contains_with_LatLngExpression(core_bounds, E(pt))) {
            in_core++;
            fill_hex = "#111111";
        } else if (LatLngBounds_contains_with_LatLngExpression(mid_bounds, E(pt))) {
            in_mid++;
            fill_hex = "#737373";
        } else {
            exterior++;
        }
        CircleMarkerOptions cm_opts = CircleMarkerOptions_create();
        CircleMarkerOptions_set_radius(cm_opts, 6.0);
        CircleMarkerOptions_set_color(cm_opts, "#111111");
        CircleMarkerOptions_set_weight(cm_opts, 2.0);
        CircleMarkerOptions_set_fillColor(cm_opts, fill_hex);
        CircleMarkerOptions_set_fillOpacity(cm_opts, 1.0);
        CircleMarker_addTo(circleMarker(E(pt), cm_opts), m);
    }

    Map_2_fitBounds(m, LatLngBoundsExpression_of_LatLngBounds(padded));

    char buf[160];
    snprintf(buf, sizeof buf, "2 nested polygons · 1 padded hull · probes: %d core · %d sector · %d exterior",
        in_core, in_mid, exterior);
    HTMLElement meta = Document_getElementById(get_document(), "meta");
    if (meta.h != 0) HTMLElement_set_textContent(meta, buf);
    js_scope_close();
}
