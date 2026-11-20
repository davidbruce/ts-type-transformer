#include "dom/dom.h"
#include "leaflet/leaflet.h"
#include <stdio.h>

static LatLngExpression E(LatLng ll) { return LatLngExpression_of_LatLng(ll); }

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();

    MapOptions m_opts = MapOptions_create();
    MapOptions_set_attributionControl(m_opts, false);
    MapOptions_set_zoomAnimation(m_opts, false);
    MapOptions_set_fadeAnimation(m_opts, false);
    MapOptions_set_markerZoomAnimation(m_opts, false);
    Map_2 m = map("map", m_opts);

    LatLng zurich = latLng(47.3769, 8.5417);
    Map_2_setView(m, E(zurich), 8.0);
    static const double radii[4] = { 130000.0, 90000.0, 55000.0, 25000.0 };
    static const char* strokes[4] = { "#404040", "#404040", "#111111", "#111111" };
    static const char* fills[4] = { "#d4d4d4", "#a3a3a3", "#737373", "#262626" };
    static const double opacities[4] = { 0.25, 0.30, 0.40, 0.55 };

    Circle outer = { 0 };
    for (int i = 0; i < 4; i++) {
        CircleMarkerOptions c_opts = CircleMarkerOptions_create();
        CircleMarkerOptions_set_color(c_opts, strokes[i]);
        CircleMarkerOptions_set_weight(c_opts, 1.5);
        CircleMarkerOptions_set_fillColor(c_opts, fills[i]);
        CircleMarkerOptions_set_fillOpacity(c_opts, opacities[i]);
        Circle c = circle_with_radius(E(zurich), radii[i], c_opts);
        Circle_addTo(c, m);
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
        CircleMarkerOptions_set_color(b_opts, "#555555");
        CircleMarkerOptions_set_weight(b_opts, 1.2);
        CircleMarkerOptions_set_dashArray(b_opts, string_or_number_array_of("3 3"));
        CircleMarkerOptions_set_fillColor(b_opts, "#737373");
        CircleMarkerOptions_set_fillOpacity(b_opts, 0.18);
        Circle_addTo(circle_with_radius(E(pt), 18000.0, b_opts), m);

        CircleMarkerOptions d_opts = CircleMarkerOptions_create();
        CircleMarkerOptions_set_radius(d_opts, 4.5);
        CircleMarkerOptions_set_color(d_opts, "#111111");
        CircleMarkerOptions_set_weight(d_opts, 1.5);
        CircleMarkerOptions_set_fillColor(d_opts, "#111111");
        CircleMarkerOptions_set_fillOpacity(d_opts, 1.0);
        CircleMarker_addTo(circleMarker(E(pt), d_opts), m);
    }

    Map_2_fitBounds(m, LatLngBoundsExpression_of_LatLngBounds(Circle_getBounds(outer)));

    char buf[160];
    snprintf(buf, sizeof buf, "4 concentric rings (25-130 km) · 5 regional beacons (avg %.1f km)", total_dist_km / 5.0);
    HTMLElement meta = Document_getElementById(get_document(), "meta");
    if (meta.h != 0) HTMLElement_set_textContent(meta, buf);
    js_scope_close();
}
