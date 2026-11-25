#include "dom/dom.h"
#include "leaflet/leaflet.h"
#include <stdio.h>

__attribute__((export_name("start"))) void start(void) {
    js_scope();

    Leaflet_Map m = leaflet_map("map", Leaflet_MapOptions_create(
        .attributionControl = false, .zoomAnimation = false, .fadeAnimation = false, .markerZoomAnimation = false));

    Leaflet_LatLng zurich = leaflet_latLng(47.3769, 8.5417);
    Leaflet_Map_setView(m, zurich, 8.0);
    static const double radii[4] = { 130000.0, 90000.0, 55000.0, 25000.0 };
    static const char* strokes[4] = { "#404040", "#404040", "#111111", "#111111" };
    static const char* fills[4] = { "#d4d4d4", "#a3a3a3", "#737373", "#262626" };
    static const double opacities[4] = { 0.25, 0.30, 0.40, 0.55 };

    Leaflet_Circle outer = { 0 };
    for (int i = 0; i < 4; i++) {
        Leaflet_Circle c = leaflet_circle_with_radius(zurich, radii[i], Leaflet_CircleMarkerOptions_create(
            .color = strokes[i], .weight = 1.5, .fillColor = fills[i], .fillOpacity = opacities[i]));
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
        total_dist_km += Leaflet_LatLng_distanceTo(zurich, pt) / 1000.0;

        Leaflet_Circle_addTo(leaflet_circle_with_radius(pt, 18000.0, Leaflet_CircleMarkerOptions_create(
            .color = "#555555", .weight = 1.2, .dashArray = "3 3",
            .fillColor = "#737373", .fillOpacity = 0.18)), m);

        Leaflet_CircleMarker_addTo(leaflet_circleMarker(pt, Leaflet_CircleMarkerOptions_create(
            .radius = 4.5, .color = "#111111", .weight = 1.5, .fillColor = "#111111", .fillOpacity = 1.0)), m);
    }

    Leaflet_Map_fitBounds(m, Leaflet_Circle_getBounds(outer));

    char buf[160];
    snprintf(buf, sizeof buf, "4 concentric rings (25-130 km) · 5 regional beacons (avg %.1f km)", total_dist_km / 5.0);
    HTMLElement meta = Document_getElementById(get_document(), "meta");
    if (meta.h != 0) HTMLElement_set_textContent(meta, buf);
}
