#include "dom/dom.h"
#include "leaflet/leaflet.h"

static LatLngExpression E(LatLng ll) { return LatLngExpression_of_LatLng(ll); }

static PolylineOptions box_opts(const char* color, double weight, const char* dash, const char* fill_color, double fill_opacity) {
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
    MapOptions_set_crs(m_opts, get_CRS_Simple());
    MapOptions_set_minZoom(m_opts, js_some_double(-1.0));
    MapOptions_set_maxZoom(m_opts, js_some_double(3.0));
    MapOptions_set_attributionControl(m_opts, js_some_bool(false));
    MapOptions_set_zoomAnimation(m_opts, js_some_bool(false));
    MapOptions_set_fadeAnimation(m_opts, js_some_bool(false));
    MapOptions_set_markerZoomAnimation(m_opts, js_some_bool(false));
    Map_2 m = map("map", m_opts);

    PolylineOptions grid_opts = box_opts("#e5e5e5", 1.0, 0, "#ffffff", 0.0);
    for (int r = 0; r <= 100; r += 20) {
        JsArray_LatLngExpression line = JsArray_LatLngExpression_new();
        JsArray_LatLngExpression_push(line, E(latLng((double)r, 0.0)));
        JsArray_LatLngExpression_push(line, E(latLng((double)r, 160.0)));
        Layer_addTo(polyline(line, grid_opts), m);
    }
    for (int c = 0; c <= 160; c += 20) {
        JsArray_LatLngExpression line = JsArray_LatLngExpression_new();
        JsArray_LatLngExpression_push(line, E(latLng(0.0, (double)c)));
        JsArray_LatLngExpression_push(line, E(latLng(100.0, (double)c)));
        Layer_addTo(polyline(line, grid_opts), m);
    }

    LatLngBounds env = latLngBounds(E(latLng(8.0, 12.0)), E(latLng(92.0, 148.0)));
    Layer_addTo(rectangle(LatLngBoundsExpression_of_LatLngBounds(env), box_opts("#111111", 2.5, 0, "#ffffff", 0.9)), m);

    static const double room_coords[4][4] = {
        { 12.0, 16.0, 46.0, 62.0 },
        { 54.0, 16.0, 88.0, 62.0 },
        { 12.0, 98.0, 46.0, 144.0 },
        { 54.0, 98.0, 88.0, 144.0 },
    };
    static const char* room_fills[4] = { "#f7f7f7", "#efefef", "#e5e5e5", "#d4d4d4" };
    for (int i = 0; i < 4; i++) {
        LatLngBounds b = latLngBounds(
            E(latLng(room_coords[i][0], room_coords[i][1])),
            E(latLng(room_coords[i][2], room_coords[i][3]))
        );
        Layer_addTo(rectangle(LatLngBoundsExpression_of_LatLngBounds(b), box_opts("#404040", 1.5, 0, room_fills[i], 0.85)), m);
    }

    JsArray_LatLngExpression atrium = JsArray_LatLngExpression_new();
    JsArray_LatLngExpression_push(atrium, E(latLng(50.0, 64.0)));
    JsArray_LatLngExpression_push(atrium, E(latLng(74.0, 80.0)));
    JsArray_LatLngExpression_push(atrium, E(latLng(50.0, 96.0)));
    JsArray_LatLngExpression_push(atrium, E(latLng(26.0, 80.0)));
    Layer_addTo(polygon(atrium, box_opts("#111111", 2.0, 0, "#555555", 0.35)), m);

    JsArray_LatLngExpression spine = JsArray_LatLngExpression_new();
    JsArray_LatLngExpression_push(spine, E(latLng(50.0, 12.0)));
    JsArray_LatLngExpression_push(spine, E(latLng(50.0, 148.0)));
    Layer_addTo(polyline(spine, box_opts("#737373", 1.5, "4 4", "#ffffff", 0.0)), m);

    CircleMarkerOptions col_opts = CircleMarkerOptions_create();
    CircleMarkerOptions_set_radius(col_opts, js_some_double(5.0));
    PathOptions_set_color(col_opts, "#111111");
    PathOptions_set_weight(col_opts, js_some_double(1.5));
    PathOptions_set_fillColor(col_opts, "#111111");
    PathOptions_set_fillOpacity(col_opts, js_some_double(1.0));
    static const double cols[6][2] = {
        { 12.0, 16.0 }, { 88.0, 16.0 }, { 12.0, 80.0 },
        { 88.0, 80.0 }, { 12.0, 144.0 }, { 88.0, 144.0 },
    };
    for (int i = 0; i < 6; i++) {
        Layer_addTo(circleMarker(E(latLng(cols[i][0], cols[i][1])), col_opts), m);
    }

    LatLngBounds plan_bounds = latLngBounds(E(latLng(0.0, 0.0)), E(latLng(100.0, 160.0)));
    Map_2_fitBounds(m, LatLngBoundsExpression_of_LatLngBounds(plan_bounds));

    Point px = Map_2_latLngToContainerPoint(m, E(latLng(50.0, 80.0)));
    char buf[160];
    int p = 0;
    p = append_str(buf, p, "CRS.Simple 100x160 plan · 4 studio rooms + atrium · center @ (");
    p = append_int(buf, p, (int)(Point_get_x(px) + 0.5));
    p = append_str(buf, p, ", ");
    p = append_int(buf, p, (int)(Point_get_y(px) + 0.5));
    p = append_str(buf, p, ") px");
    buf[p] = '\0';

    Document doc = get_document();
    HTMLElement meta = Document_getElementById(doc, "meta");
    if (meta.h != 0) Node_set_textContent(meta, buf);
}
