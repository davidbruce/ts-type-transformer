#include <cstdio>
#include <string>

#include "dom/dom.hpp"
#include "leaflet/leaflet.hpp"

using namespace leaflet;

static PolylineOptions box_opts(const char* color, double weight, const char* dash, const char* fill_color, double fill_opacity) {
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
    m_opts.crs(CRS::get_Simple());
    m_opts.minZoom(-1.0);
    m_opts.maxZoom(3.0);
    m_opts.attributionControl(false);
    m_opts.zoomAnimation(false);
    m_opts.fadeAnimation(false);
    m_opts.markerZoomAnimation(false);
    Map m = leaflet::map("map", m_opts);

    PolylineOptions grid_opts = box_opts("#e5e5e5", 1.0, nullptr, "#ffffff", 0.0);
    for (int r = 0; r <= 100; r += 20) {
        leaflet::polyline(ts::Array<>::of(leaflet::latLng(r, 0.0), leaflet::latLng(r, 160.0)), grid_opts).addTo(m);
    }
    for (int c = 0; c <= 160; c += 20) {
        leaflet::polyline(ts::Array<>::of(leaflet::latLng(0.0, c), leaflet::latLng(100.0, c)), grid_opts).addTo(m);
    }

    LatLngBounds env = leaflet::latLngBounds(leaflet::latLng(8.0, 12.0), leaflet::latLng(92.0, 148.0));
    leaflet::rectangle(env, box_opts("#111111", 2.5, nullptr, "#ffffff", 0.9)).addTo(m);

    struct Room {
        double r0, c0, r1, c1;
        const char* fill;
    };
    Room rooms[4] = {
        { 12.0, 16.0, 46.0, 62.0, "#f7f7f7" },
        { 54.0, 16.0, 88.0, 62.0, "#efefef" },
        { 12.0, 98.0, 46.0, 144.0, "#e5e5e5" },
        { 54.0, 98.0, 88.0, 144.0, "#d4d4d4" },
    };
    for (const Room& rm : rooms) {
        LatLngBounds b = leaflet::latLngBounds(leaflet::latLng(rm.r0, rm.c0), leaflet::latLng(rm.r1, rm.c1));
        leaflet::rectangle(b, box_opts("#404040", 1.5, nullptr, rm.fill, 0.85)).addTo(m);
    }

    auto atrium = ts::Array<Polyline_setLatLngs_latlngs>::of(
        leaflet::latLng(50.0, 64.0),
        leaflet::latLng(74.0, 80.0),
        leaflet::latLng(50.0, 96.0),
        leaflet::latLng(26.0, 80.0)
    );
    leaflet::polygon(atrium, box_opts("#111111", 2.0, nullptr, "#555555", 0.35)).addTo(m);

    leaflet::polyline(
        ts::Array<>::of(leaflet::latLng(50.0, 12.0), leaflet::latLng(50.0, 148.0)),
        box_opts("#737373", 1.5, "4 4", "#ffffff", 0.0)
    ).addTo(m);

    CircleMarkerOptions col_opts = CircleMarkerOptions::create();
    col_opts.radius(5.0);
    col_opts.color("#111111");
    col_opts.weight(1.5);
    col_opts.fillColor("#111111");
    col_opts.fillOpacity(1.0);
    double cols[6][2] = {
        { 12.0, 16.0 }, { 88.0, 16.0 }, { 12.0, 80.0 },
        { 88.0, 80.0 }, { 12.0, 144.0 }, { 88.0, 144.0 },
    };
    for (const auto& col : cols) {
        leaflet::circleMarker(leaflet::latLng(col[0], col[1]), col_opts).addTo(m);
    }

    m.fitBounds(leaflet::latLngBounds(leaflet::latLng(0.0, 0.0), leaflet::latLng(100.0, 160.0)));

    leaflet::Point px = m.latLngToContainerPoint(leaflet::latLng(50.0, 80.0));
    char buf[160];
    std::snprintf(
        buf, sizeof(buf),
        "CRS.Simple 100x160 plan · 4 studio rooms + atrium · center @ (%.0f, %.0f) px",
        px.x(), px.y()
    );
    if (auto meta = dom::get_document().getElementById("meta")) {
        meta->textContent(buf);
    }
}
