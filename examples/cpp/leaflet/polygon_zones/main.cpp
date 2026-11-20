#include <cstdio>
#include <string>

#include "dom/dom.hpp"
#include "leaflet/leaflet.hpp"

using namespace leaflet;

static PolylineOptions poly_opts(const char* color, double weight, const char* dash, const char* fill_color, double fill_opacity) {
    PolylineOptions opts = PolylineOptions::create();
    opts.smoothFactor(1.0);
    opts.color(color);
    opts.weight(weight);
    if (dash != nullptr) {
        opts.dashArray(dash);
    }
    opts.fillColor(fill_color);
    opts.fillOpacity(fill_opacity);
    return opts;
}

TS_EXPORT(start) void start() {
    MapOptions m_opts = MapOptions::create();
    m_opts.attributionControl(false);
    m_opts.zoomAnimation(false);
    m_opts.fadeAnimation(false);
    m_opts.markerZoomAnimation(false);
    Map m = leaflet::map("map", m_opts);

    auto mid_pts = ts::Array<Polyline_setLatLngs_latlngs>::of(
        leaflet::latLng(47.2, 7.6),
        leaflet::latLng(48.0, 8.6),
        leaflet::latLng(47.5, 9.7),
        leaflet::latLng(46.8, 9.5),
        leaflet::latLng(46.6, 8.4),
        leaflet::latLng(46.9, 7.3)
    );
    auto core_pts = ts::Array<Polyline_setLatLngs_latlngs>::of(
        leaflet::latLng(47.35, 8.2),
        leaflet::latLng(47.55, 8.75),
        leaflet::latLng(47.15, 9.0),
        leaflet::latLng(46.95, 8.4)
    );

    Polygon<> mid_poly = leaflet::polygon(mid_pts, poly_opts("#404040", 2.0, nullptr, "#d4d4d4", 0.55));
    Polygon<> core_poly = leaflet::polygon(core_pts, poly_opts("#111111", 2.5, nullptr, "#404040", 0.75));

    LatLngBounds mid_bounds = mid_poly.getBounds();
    LatLngBounds core_bounds = core_poly.getBounds();
    LatLngBounds padded = mid_bounds.pad(0.18);
    Rectangle<> hull_rect = leaflet::rectangle(padded, poly_opts("#737373", 1.5, "4 4", "#efefef", 0.05));

    hull_rect.addTo(m);
    mid_poly.addTo(m);
    core_poly.addTo(m);

    LatLng probes[8] = {
        leaflet::latLng(47.25, 8.6),
        leaflet::latLng(47.15, 8.55),
        leaflet::latLng(47.7, 8.6),
        leaflet::latLng(47.0, 9.2),
        leaflet::latLng(46.8, 7.9),
        leaflet::latLng(47.85, 7.5),
        leaflet::latLng(47.9, 9.7),
        leaflet::latLng(46.55, 9.6),
    };

    int in_core = 0;
    int in_mid = 0;
    int exterior = 0;

    for (const LatLng& pt : probes) {
        const char* fill_hex = "#ffffff";
        if (core_bounds.contains(pt)) {
            in_core++;
            fill_hex = "#111111";
        } else if (mid_bounds.contains(pt)) {
            in_mid++;
            fill_hex = "#737373";
        } else {
            exterior++;
        }
        CircleMarkerOptions cm_opts = CircleMarkerOptions::create();
        cm_opts.radius(6.0);
        cm_opts.color("#111111");
        cm_opts.weight(2.0);
        cm_opts.fillColor(fill_hex);
        cm_opts.fillOpacity(1.0);
        leaflet::circleMarker(pt, cm_opts).addTo(m);
    }

    m.fitBounds(padded);

    char buf[160];
    std::snprintf(
        buf, sizeof(buf),
        "2 nested polygons · 1 padded hull · probes: %d core · %d sector · %d exterior",
        in_core, in_mid, exterior
    );
    if (auto meta = dom::get_document().getElementById("meta")) {
        meta->textContent(buf);
    }
}
