#include "dom/dom.h"
#include "leaflet/leaflet.h"
#include <stdio.h>

static Leaflet_LatLngExpression E(Leaflet_LatLng ll) { return Leaflet_LatLngExpression_of_Leaflet_LatLng(ll); }

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();

    Leaflet_MapOptions m_opts = Leaflet_MapOptions_create();
    Leaflet_MapOptions_set_attributionControl(m_opts, false);
    Leaflet_MapOptions_set_zoomAnimation(m_opts, false);
    Leaflet_MapOptions_set_fadeAnimation(m_opts, false);
    Leaflet_MapOptions_set_markerZoomAnimation(m_opts, false);
    Leaflet_Map m = leaflet_map("map", m_opts);

    Leaflet_LatLng zurich = leaflet_latLng(47.3769, 8.5417);
    Leaflet_Map_setView(m, E(zurich), 8.0);
    static const double radii[4] = { 130000.0, 90000.0, 55000.0, 25000.0 };
    static const char* strokes[4] = { "#404040", "#404040", "#111111", "#111111" };
    static const char* fills[4] = { "#d4d4d4", "#a3a3a3", "#737373", "#262626" };
    static const double opacities[4] = { 0.25, 0.30, 0.40, 0.55 };

    Leaflet_Circle outer = { 0 };
    for (int i = 0; i < 4; i++) {
        Leaflet_CircleMarkerOptions c_opts = Leaflet_CircleMarkerOptions_create();
        Leaflet_CircleMarkerOptions_set_color(c_opts, strokes[i]);
        Leaflet_CircleMarkerOptions_set_weight(c_opts, 1.5);
        Leaflet_CircleMarkerOptions_set_fillColor(c_opts, fills[i]);
        Leaflet_CircleMarkerOptions_set_fillOpacity(c_opts, opacities[i]);
        Leaflet_Circle c = leaflet_circle_with_radius(E(zurich), radii[i], c_opts);
        Leaflet_Circle_addTo(c, m);
        if (i == 0) outer = c;
    }

    Leaflet_LatLng beacons[5] = {
        leaflet_latLng(47.5596, 7.5886),
        leaflet_latLng(46.9480, 7.4474),
        leaflet_latLng(47.0502, 8.3093),
        leaflet_latLng(47.4245, 9.3767),
        leaflet_latLng(46.8508, 9.5320),
    };

    double total_dist_km = 0.0;
    for (int i = 0; i < 5; i++) {
        Leaflet_LatLng pt = beacons[i];
        total_dist_km += Leaflet_LatLng_distanceTo(zurich, E(pt)) / 1000.0;

        Leaflet_CircleMarkerOptions b_opts = Leaflet_CircleMarkerOptions_create();
        Leaflet_CircleMarkerOptions_set_color(b_opts, "#555555");
        Leaflet_CircleMarkerOptions_set_weight(b_opts, 1.2);
        Leaflet_CircleMarkerOptions_set_dashArray(b_opts, Leaflet_string_or_number_array_of("3 3"));
        Leaflet_CircleMarkerOptions_set_fillColor(b_opts, "#737373");
        Leaflet_CircleMarkerOptions_set_fillOpacity(b_opts, 0.18);
        Leaflet_Circle_addTo(leaflet_circle_with_radius(E(pt), 18000.0, b_opts), m);

        Leaflet_CircleMarkerOptions d_opts = Leaflet_CircleMarkerOptions_create();
        Leaflet_CircleMarkerOptions_set_radius(d_opts, 4.5);
        Leaflet_CircleMarkerOptions_set_color(d_opts, "#111111");
        Leaflet_CircleMarkerOptions_set_weight(d_opts, 1.5);
        Leaflet_CircleMarkerOptions_set_fillColor(d_opts, "#111111");
        Leaflet_CircleMarkerOptions_set_fillOpacity(d_opts, 1.0);
        Leaflet_CircleMarker_addTo(leaflet_circleMarker(E(pt), d_opts), m);
    }

    Leaflet_Map_fitBounds(m, Leaflet_LatLngBoundsExpression_of_Leaflet_LatLngBounds(Leaflet_Circle_getBounds(outer)));

    char buf[160];
    snprintf(buf, sizeof buf, "4 concentric rings (25-130 km) · 5 regional beacons (avg %.1f km)", total_dist_km / 5.0);
    HTMLElement meta = Document_getElementById(get_document(), "meta");
    if (meta.h != 0) HTMLElement_set_textContent(meta, buf);
    js_scope_close();
}
