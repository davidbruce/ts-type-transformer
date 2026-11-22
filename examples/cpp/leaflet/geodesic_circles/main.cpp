#include <cstdio>
#include <string>

#include "dom/dom.hpp"
#include "leaflet/leaflet.hpp"

using namespace leaflet;

TS_EXPORT(start) void start() {
    Map m = leaflet::map("map", {{.attributionControl = false, .fadeAnimation = false, .markerZoomAnimation = false, .zoomAnimation = false}});

    LatLng zurich = leaflet::latLng(47.3769, 8.5417);
    m.setView(zurich, 8.0);

    struct RingSpec {
        double radius;
        const char* stroke;
        const char* fill;
        double opacity;
    };
    RingSpec rings[4] = {
        { 130000.0, "#404040", "#d4d4d4", 0.25 },
        { 90000.0, "#404040", "#a3a3a3", 0.30 },
        { 55000.0, "#111111", "#737373", 0.40 },
        { 25000.0, "#111111", "#262626", 0.55 },
    };

    LatLngBounds outer_bounds;
    for (const RingSpec& r : rings) {
        Circle c = leaflet::circle(zurich, r.radius, {{.color = r.stroke, .weight = 1.5, .fillColor = r.fill, .fillOpacity = r.opacity}});
        c.addTo(m);
        if (!outer_bounds) outer_bounds = c.getBounds();
    }

    LatLng beacons[5] = {
        leaflet::latLng(47.5596, 7.5886),
        leaflet::latLng(46.9480, 7.4474),
        leaflet::latLng(47.0502, 8.3093),
        leaflet::latLng(47.4245, 9.3767),
        leaflet::latLng(46.8508, 9.5320),
    };

    double total_dist_km = 0.0;
    for (const LatLng& pt : beacons) {
        total_dist_km += zurich.distanceTo(pt) / 1000.0;

        leaflet::circle(pt, 18000.0, {{.color = "#555555", .weight = 1.2, .dashArray = "3 3", .fillColor = "#737373", .fillOpacity = 0.18}}).addTo(m);
        leaflet::circleMarker(pt, {{.color = "#111111", .weight = 1.5, .fillColor = "#111111", .fillOpacity = 1.0, .radius = 4.5}}).addTo(m);
    }

    m.fitBounds(outer_bounds);

    char buf[160];
    std::snprintf(
        buf, sizeof(buf),
        "4 concentric rings (25–130 km) · 5 regional beacons (avg %.1f km)",
        total_dist_km / 5.0
    );
    if (auto meta = dom::get_document().getElementById("meta")) {
        meta->textContent(buf);
    }
}
