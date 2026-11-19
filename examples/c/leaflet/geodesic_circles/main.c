#include "dom/dom.h"
#include "leaflet/leaflet.h"

static LatLngExpression E(LatLng ll) { return LatLngExpression_of_LatLng(ll); }

static int append_str(char* buf, int pos, const char* s) {
    while (*s) buf[pos++] = *s++;
    return pos;
}

static int append_tenths(char* buf, int pos, double val) {
    int scaled = (int)(val * 10.0 + 0.5);
    int whole = scaled / 10;
    int frac = scaled % 10;
    char tmp[16];
    int n = 0;
    if (whole == 0) tmp[n++] = '0';
    else {
        while (whole > 0) {
            tmp[n++] = (char)('0' + (whole % 10));
            whole /= 10;
        }
    }
    for (int i = n - 1; i >= 0; i--) buf[pos++] = tmp[i];
    buf[pos++] = '.';
    buf[pos++] = (char)('0' + frac);
    return pos;
}

__attribute__((export_name("start"))) void start(void) {
    MapOptions m_opts = MapOptions_create();
    MapOptions_set_attributionControl(m_opts, js_some_bool(false));
    MapOptions_set_zoomAnimation(m_opts, js_some_bool(false));
    MapOptions_set_fadeAnimation(m_opts, js_some_bool(false));
    MapOptions_set_markerZoomAnimation(m_opts, js_some_bool(false));
    Map_2 m = map("map", m_opts);

    LatLng zurich = latLng(47.3769, 8.5417);
    Map_2_setView(m, E(zurich), js_some_double(8.0), (ZoomPanOptions){ 0 });
    static const double radii[4] = { 130000.0, 90000.0, 55000.0, 25000.0 };
    static const char* strokes[4] = { "#404040", "#404040", "#111111", "#111111" };
    static const char* fills[4] = { "#d4d4d4", "#a3a3a3", "#737373", "#262626" };
    static const double opacities[4] = { 0.25, 0.30, 0.40, 0.55 };

    Circle outer = { 0 };
    for (int i = 0; i < 4; i++) {
        CircleMarkerOptions c_opts = CircleMarkerOptions_create();
        PathOptions_set_color(c_opts, strokes[i]);
        PathOptions_set_weight(c_opts, js_some_double(1.5));
        PathOptions_set_fillColor(c_opts, fills[i]);
        PathOptions_set_fillOpacity(c_opts, js_some_double(opacities[i]));
        Circle c = circle_with_radius(E(zurich), radii[i], c_opts);
        Layer_addTo(c, m);
        if (i == 0) outer = c;
    }

    LatLng beacons[5] = {
        latLng(47.5596, 7.5886),
        latLng(46.9480, 7.4474),
        latLng(47.0502, 8.3093),
        latLng(47.4245, 9.3767),
        latLng(46.8508, 9.5320),
    };

    double total_dist_km = 0.0;
    for (int i = 0; i < 5; i++) {
        LatLng pt = beacons[i];
        total_dist_km += LatLng_distanceTo(zurich, E(pt)) / 1000.0;

        CircleMarkerOptions b_opts = CircleMarkerOptions_create();
        PathOptions_set_color(b_opts, "#555555");
        PathOptions_set_weight(b_opts, js_some_double(1.2));
        PathOptions_set_dashArray(b_opts, string_or_number_array_of_string(JsString_from("3 3")));
        PathOptions_set_fillColor(b_opts, "#737373");
        PathOptions_set_fillOpacity(b_opts, js_some_double(0.18));
        Layer_addTo(circle_with_radius(E(pt), 18000.0, b_opts), m);

        CircleMarkerOptions d_opts = CircleMarkerOptions_create();
        CircleMarkerOptions_set_radius(d_opts, js_some_double(4.5));
        PathOptions_set_color(d_opts, "#111111");
        PathOptions_set_weight(d_opts, js_some_double(1.5));
        PathOptions_set_fillColor(d_opts, "#111111");
        PathOptions_set_fillOpacity(d_opts, js_some_double(1.0));
        Layer_addTo(circleMarker(E(pt), d_opts), m);
    }

    Map_2_fitBounds(m, LatLngBoundsExpression_of_LatLngBounds(Circle_getBounds(outer)));

    char buf[160];
    int p = 0;
    p = append_str(buf, p, "4 concentric rings (25-130 km) · 5 regional beacons (avg ");
    p = append_tenths(buf, p, total_dist_km / 5.0);
    p = append_str(buf, p, " km)");
    buf[p] = '\0';

    Document doc = get_document();
    HTMLElement meta = Document_getElementById(doc, "meta");
    if (meta.h != 0) Node_set_textContent(meta, buf);
}
