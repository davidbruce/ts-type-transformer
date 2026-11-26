#include "dom/dom.h"
#include "leaflet/leaflet.h"
#include <stdio.h>

static Leaflet_PolylineOptions box_opts(const char* color, double weight, const char* dash, const char* fill_color, double fill_opacity) {
    Leaflet_PolylineOptions opts = Leaflet_PolylineOptions_create(
        .smoothFactor = 1.0, .color = color, .weight = weight, .fillColor = fill_color, .fillOpacity = fill_opacity);
    if (dash != 0) Leaflet_PolylineOptions_set_dashArray(opts, dash);
    return opts;
}

__attribute__((export_name("start"))) void start(void) {
    js_scope();

    Leaflet_Map m = leaflet_map("map", Leaflet_MapOptions_create(
        .crs = leaflet_get_CRS_Simple(), .minZoom = -1.0, .maxZoom = 3.0,
        .attributionControl = false, .zoomAnimation = false, .fadeAnimation = false, .markerZoomAnimation = false));

    Leaflet_PolylineOptions grid_opts = box_opts("#e5e5e5", 1.0, 0, "#ffffff", 0.0);
    for (int r = 0; r <= 100; r += 20) {
        Leaflet_Polyline_addTo(leaflet_polyline(Leaflet_LatLng_list(
            leaflet_latLng((double)r, 0.0), leaflet_latLng((double)r, 160.0)), grid_opts), m);
    }
    for (int c = 0; c <= 160; c += 20) {
        Leaflet_Polyline_addTo(leaflet_polyline(Leaflet_LatLng_list(
            leaflet_latLng(0.0, (double)c), leaflet_latLng(100.0, (double)c)), grid_opts), m);
    }

    Leaflet_LatLngBounds env = leaflet_latLngBounds(leaflet_latLng(8.0, 12.0), leaflet_latLng(92.0, 148.0));
    Leaflet_Rectangle_addTo(leaflet_rectangle(env, box_opts("#111111", 2.5, 0, "#ffffff", 0.9)), m);

    static const double room_coords[4][4] = {
        { 12.0, 16.0, 46.0, 62.0 },
        { 54.0, 16.0, 88.0, 62.0 },
        { 12.0, 98.0, 46.0, 144.0 },
        { 54.0, 98.0, 88.0, 144.0 },
    };
    static const char* room_fills[4] = { "#f7f7f7", "#efefef", "#e5e5e5", "#d4d4d4" };
    for (int i = 0; i < 4; i++) {
        Leaflet_LatLngBounds b = leaflet_latLngBounds(
            leaflet_latLng(room_coords[i][0], room_coords[i][1]),
            leaflet_latLng(room_coords[i][2], room_coords[i][3])
        );
        Leaflet_Rectangle_addTo(leaflet_rectangle(b, box_opts("#404040", 1.5, 0, room_fills[i], 0.85)), m);
    }

    Leaflet_Polygon_addTo(leaflet_polygon(Leaflet_LatLng_list(
        leaflet_latLng(50.0, 64.0), leaflet_latLng(74.0, 80.0),
        leaflet_latLng(50.0, 96.0), leaflet_latLng(26.0, 80.0)),
        box_opts("#111111", 2.0, 0, "#555555", 0.35)), m);

    Leaflet_Polyline_addTo(leaflet_polyline(Leaflet_LatLng_list(
        leaflet_latLng(50.0, 12.0), leaflet_latLng(50.0, 148.0)),
        box_opts("#737373", 1.5, "4 4", "#ffffff", 0.0)), m);

    Leaflet_CircleMarkerOptions col_opts = Leaflet_CircleMarkerOptions_create(
        .radius = 5.0, .color = "#111111", .weight = 1.5, .fillColor = "#111111", .fillOpacity = 1.0);
    static const double cols[6][2] = {
        { 12.0, 16.0 }, { 88.0, 16.0 }, { 12.0, 80.0 },
        { 88.0, 80.0 }, { 12.0, 144.0 }, { 88.0, 144.0 },
    };
    for (int i = 0; i < 6; i++) {
        Leaflet_CircleMarker_addTo(leaflet_circleMarker(leaflet_latLng(cols[i][0], cols[i][1]), col_opts), m);
    }

    Leaflet_LatLngBounds plan_bounds = leaflet_latLngBounds(leaflet_latLng(0.0, 0.0), leaflet_latLng(100.0, 160.0));
    Leaflet_Map_fitBounds(m, plan_bounds);

    Leaflet_Point px = Leaflet_Map_latLngToContainerPoint(m, leaflet_latLng(50.0, 80.0));
    char buf[160];
    snprintf(buf, sizeof buf, "CRS.Simple 100x160 plan · 4 studio rooms + atrium · center @ (%d, %d) px",
        (int)(Leaflet_Point_get_x(px) + 0.5), (int)(Leaflet_Point_get_y(px) + 0.5));
    HTMLElement meta = Document_getElementById(get_document(), "meta");
    if (meta.h != 0) HTMLElement_set_textContent(meta, buf);
}
