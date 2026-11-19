#include "dom/dom.h"
#include "leaflet/leaflet.h"

static LatLngExpression E(LatLng ll) { return LatLngExpression_of_LatLng(ll); }

static PolylineOptions poly_opts(const char* color, double weight, const char* dash, const char* fill_color, double fill_opacity) {
    PolylineOptions opts = PolylineOptions_create();
    PolylineOptions_set_smoothFactor(opts, js_some_double(1.0));
    PathOptions_set_color(opts, color);
    PathOptions_set_weight(opts, js_some_double(weight));
    if (dash != 0) {
        PathOptions_set_dashArray(opts, string_or_number_array_of_string(JsString_from(dash)));
    }
    PathOptions_set_fillColor(opts, fill_color);
    PathOptions_set_fillOpacity(opts, js_some_double(fill_opacity));
    return opts;
}

static int append_str(char* buf, int pos, const char* s) {
    while (*s) buf[pos++] = *s++;
    return pos;
}

static int append_int(char* buf, int pos, int val) {
    if (val == 0) {
        buf[pos++] = '0';
        return pos;
    }
    char tmp[16];
    int n = 0;
    while (val > 0) {
        tmp[n++] = (char)('0' + (val % 10));
        val /= 10;
    }
    for (int i = n - 1; i >= 0; i--) buf[pos++] = tmp[i];
    return pos;
}

__attribute__((export_name("start"))) void start(void) {
    MapOptions m_opts = MapOptions_create();
    MapOptions_set_attributionControl(m_opts, js_some_bool(false));
    MapOptions_set_zoomAnimation(m_opts, js_some_bool(false));
    MapOptions_set_fadeAnimation(m_opts, js_some_bool(false));
    MapOptions_set_markerZoomAnimation(m_opts, js_some_bool(false));
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

    LatLngBounds mid_bounds = Polyline_getBounds(mid_poly);
    LatLngBounds core_bounds = Polyline_getBounds(core_poly);
    LatLngBounds padded = LatLngBounds_pad(mid_bounds, 0.18);
    Rectangle hull_rect = rectangle(LatLngBoundsExpression_of_LatLngBounds(padded), poly_opts("#737373", 1.5, "4 4", "#efefef", 0.05));

    Layer_addTo(hull_rect, m);
    Layer_addTo(mid_poly, m);
    Layer_addTo(core_poly, m);

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
        CircleMarkerOptions_set_radius(cm_opts, js_some_double(6.0));
        PathOptions_set_color(cm_opts, "#111111");
        PathOptions_set_weight(cm_opts, js_some_double(2.0));
        PathOptions_set_fillColor(cm_opts, fill_hex);
        PathOptions_set_fillOpacity(cm_opts, js_some_double(1.0));
        Layer_addTo(circleMarker(E(pt), cm_opts), m);
    }

    Map_2_fitBounds(m, LatLngBoundsExpression_of_LatLngBounds(padded));

    char buf[160];
    int p = 0;
    p = append_str(buf, p, "2 nested polygons · 1 padded hull · probes: ");
    p = append_int(buf, p, in_core);
    p = append_str(buf, p, " core · ");
    p = append_int(buf, p, in_mid);
    p = append_str(buf, p, " sector · ");
    p = append_int(buf, p, exterior);
    p = append_str(buf, p, " exterior");
    buf[p] = '\0';

    Document doc = get_document();
    HTMLElement meta = Document_getElementById(doc, "meta");
    if (meta.h != 0) Node_set_textContent(meta, buf);
}
