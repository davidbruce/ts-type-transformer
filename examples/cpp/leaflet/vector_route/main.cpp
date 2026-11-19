#include <cstdio>
#include <string>

#include "dom/dom.hpp"
#include "leaflet/leaflet.hpp"

using namespace leaflet;

static MapOptions make_map_opts() {
    MapOptions opts = MapOptions::create();
    opts.attributionControl(false);
    opts.zoomAnimation(false);
    opts.fadeAnimation(false);
    opts.markerZoomAnimation(false);
    return opts;
}

static PolylineOptions poly_opts(const char* color, double weight, const char* dash, double fill_opacity) {
    PolylineOptions opts = PolylineOptions::create();
    opts.smoothFactor(1.0);
    opts.color(std::string(color));
    opts.weight(weight);
    if (dash != nullptr) {
        opts.dashArray(string_or_number_array(std::string(dash)));
    }
    opts.fillOpacity(fill_opacity);
    return opts;
}

extern "C" __attribute__((export_name("start"))) void start() {
    Map m = leaflet::map(Content(std::string("map")), make_map_opts());

    struct Hub {
        const char* name;
        LatLng pt;
    };
    Hub hubs[6] = {
        { "London", leaflet::latLng(51.505, -0.09) },
        { "Paris", leaflet::latLng(48.8566, 2.3522) },
        { "Zurich", leaflet::latLng(47.3769, 8.5417) },
        { "Munich", leaflet::latLng(48.1351, 11.5820) },
        { "Prague", leaflet::latLng(50.0755, 14.4378) },
        { "Berlin", leaflet::latLng(52.5200, 13.4050) },
    };

    PolylineOptions grat_opts = poly_opts("#e5e5e5", 1.0, nullptr, 0.0);
    for (double lat : { 46.0, 48.0, 50.0, 52.0, 54.0 }) {
        ts::Array<> line = ts::Array<>::create();
        line.push(LatLngExpression(leaflet::latLng(lat, -2.0)));
        line.push(LatLngExpression(leaflet::latLng(lat, 17.0)));
        leaflet::polyline(line, grat_opts).addTo(m);
    }
    for (double lng : { 0.0, 4.0, 8.0, 12.0, 16.0 }) {
        ts::Array<> line = ts::Array<>::create();
        line.push(LatLngExpression(leaflet::latLng(45.0, lng)));
        line.push(LatLngExpression(leaflet::latLng(55.0, lng)));
        leaflet::polyline(line, grat_opts).addTo(m);
    }

    ts::Array<> primary_pts = ts::Array<>::create();
    for (const Hub& h : hubs) primary_pts.push(LatLngExpression(h.pt));
    Polyline<> primary_line = leaflet::polyline(primary_pts, poly_opts("#111111", 3.0, nullptr, 0.0));
    primary_line.addTo(m);

    ts::Array<> alt_pts = ts::Array<>::create();
    alt_pts.push(LatLngExpression(hubs[0].pt));
    alt_pts.push(LatLngExpression(leaflet::latLng(50.8503, 4.3517)));
    alt_pts.push(LatLngExpression(leaflet::latLng(52.3676, 4.9041)));
    alt_pts.push(LatLngExpression(hubs[5].pt));
    leaflet::polyline(alt_pts, poly_opts("#737373", 1.5, "5 5", 0.0)).addTo(m);

    LayerGroup<> nodes = leaflet::layerGroup();
    CircleMarkerOptions node_opts = CircleMarkerOptions::create();
    node_opts.radius(6.0);
    node_opts.color(std::string("#111111"));
    node_opts.weight(2.0);
    node_opts.fillColor(std::string("#ffffff"));
    node_opts.fillOpacity(1.0);

    for (const Hub& h : hubs) {
        CircleMarker<> cm = leaflet::circleMarker(h.pt, node_opts);
        cm.bindTooltip(Layer_bindTooltip_content(std::string(h.name)));
        nodes.addLayer(cm);
    }
    nodes.addTo(m);

    m.fitBounds(primary_line.getBounds());

    double total_km = 0.0;
    for (int i = 0; i < 5; i++) {
        total_km += hubs[i].pt.distanceTo(hubs[i + 1].pt) / 1000.0;
    }

    char buf[160];
    std::snprintf(buf, sizeof(buf), "6 hubs · 2 routes · %.0f km total · EPSG:3857 vector SVG", total_km);
    if (auto meta = dom::get_document().getElementById("meta")) {
        meta->textContent(buf);
    }
}
