#include "dom/dom.h"
#include "leaflet/leaflet.h"
#include <stdio.h>

static Leaflet_LatLngExpression E(Leaflet_LatLng ll) { return Leaflet_LatLngExpression_of_Leaflet_LatLng(ll); }

static Leaflet_PolylineOptions box_opts(const char* color, double weight, const char* dash, const char* fill_color, double fill_opacity) {
    Leaflet_PolylineOptions opts = Leaflet_PolylineOptions_create();
    Leaflet_PolylineOptions_set_smoothFactor(opts, 1.0);
    Leaflet_PolylineOptions_set_color(opts, color);
    Leaflet_PolylineOptions_set_weight(opts, weight);
    if (dash != 0) {
        Leaflet_PolylineOptions_set_dashArray(opts, Leaflet_string_or_number_array_of(dash));
    }
    Leaflet_PolylineOptions_set_fillColor(opts, fill_color);
    Leaflet_PolylineOptions_set_fillOpacity(opts, fill_opacity);
    return opts;
}

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();

    Leaflet_MapOptions m_opts = Leaflet_MapOptions_create();
    Leaflet_MapOptions_set_crs(m_opts, leaflet_get_CRS_Simple());
    Leaflet_MapOptions_set_minZoom(m_opts, -1.0);
    Leaflet_MapOptions_set_maxZoom(m_opts, 3.0);
    Leaflet_MapOptions_set_attributionControl(m_opts, false);
    Leaflet_MapOptions_set_zoomAnimation(m_opts, false);
    Leaflet_MapOptions_set_fadeAnimation(m_opts, false);
    Leaflet_MapOptions_set_markerZoomAnimation(m_opts, false);
    Leaflet_Map m = leaflet_map("map", m_opts);

    Leaflet_PolylineOptions grid_opts = box_opts("#e5e5e5", 1.0, 0, "#ffffff", 0.0);
    for (int r = 0; r <= 100; r += 20) {
        JsArray_Leaflet_LatLngExpression line = JsArray_Leaflet_LatLngExpression_new();
        JsArray_Leaflet_LatLngExpression_push(line, E(leaflet_latLng((double)r, 0.0)));
        JsArray_Leaflet_LatLngExpression_push(line, E(leaflet_latLng((double)r, 160.0)));
        Leaflet_Polyline_addTo(leaflet_polyline(line, grid_opts), m);
    }
    for (int c = 0; c <= 160; c += 20) {
        JsArray_Leaflet_LatLngExpression line = JsArray_Leaflet_LatLngExpression_new();
        JsArray_Leaflet_LatLngExpression_push(line, E(leaflet_latLng(0.0, (double)c)));
        JsArray_Leaflet_LatLngExpression_push(line, E(leaflet_latLng(100.0, (double)c)));
        Leaflet_Polyline_addTo(leaflet_polyline(line, grid_opts), m);
    }

    Leaflet_LatLngBounds env = leaflet_latLngBounds(E(leaflet_latLng(8.0, 12.0)), E(leaflet_latLng(92.0, 148.0)));
    Leaflet_Rectangle_addTo(leaflet_rectangle(Leaflet_LatLngBoundsExpression_of_Leaflet_LatLngBounds(env), box_opts("#111111", 2.5, 0, "#ffffff", 0.9)), m);

    static const double room_coords[4][4] = {
        { 12.0, 16.0, 46.0, 62.0 },
        { 54.0, 16.0, 88.0, 62.0 },
        { 12.0, 98.0, 46.0, 144.0 },
        { 54.0, 98.0, 88.0, 144.0 },
    };
    static const char* room_fills[4] = { "#f7f7f7", "#efefef", "#e5e5e5", "#d4d4d4" };
    for (int i = 0; i < 4; i++) {
        Leaflet_LatLngBounds b = leaflet_latLngBounds(
            E(leaflet_latLng(room_coords[i][0], room_coords[i][1])),
            E(leaflet_latLng(room_coords[i][2], room_coords[i][3]))
        );
        Leaflet_Rectangle_addTo(leaflet_rectangle(Leaflet_LatLngBoundsExpression_of_Leaflet_LatLngBounds(b), box_opts("#404040", 1.5, 0, room_fills[i], 0.85)), m);
    }

    JsArray_Leaflet_LatLngExpression atrium = JsArray_Leaflet_LatLngExpression_new();
    JsArray_Leaflet_LatLngExpression_push(atrium, E(leaflet_latLng(50.0, 64.0)));
    JsArray_Leaflet_LatLngExpression_push(atrium, E(leaflet_latLng(74.0, 80.0)));
    JsArray_Leaflet_LatLngExpression_push(atrium, E(leaflet_latLng(50.0, 96.0)));
    JsArray_Leaflet_LatLngExpression_push(atrium, E(leaflet_latLng(26.0, 80.0)));
    Leaflet_Polygon_addTo(leaflet_polygon(atrium, box_opts("#111111", 2.0, 0, "#555555", 0.35)), m);

    JsArray_Leaflet_LatLngExpression spine = JsArray_Leaflet_LatLngExpression_new();
    JsArray_Leaflet_LatLngExpression_push(spine, E(leaflet_latLng(50.0, 12.0)));
    JsArray_Leaflet_LatLngExpression_push(spine, E(leaflet_latLng(50.0, 148.0)));
    Leaflet_Polyline_addTo(leaflet_polyline(spine, box_opts("#737373", 1.5, "4 4", "#ffffff", 0.0)), m);

    Leaflet_CircleMarkerOptions col_opts = Leaflet_CircleMarkerOptions_create();
    Leaflet_CircleMarkerOptions_set_radius(col_opts, 5.0);
    Leaflet_CircleMarkerOptions_set_color(col_opts, "#111111");
    Leaflet_CircleMarkerOptions_set_weight(col_opts, 1.5);
    Leaflet_CircleMarkerOptions_set_fillColor(col_opts, "#111111");
    Leaflet_CircleMarkerOptions_set_fillOpacity(col_opts, 1.0);
    static const double cols[6][2] = {
        { 12.0, 16.0 }, { 88.0, 16.0 }, { 12.0, 80.0 },
        { 88.0, 80.0 }, { 12.0, 144.0 }, { 88.0, 144.0 },
    };
    for (int i = 0; i < 6; i++) {
        Leaflet_CircleMarker_addTo(leaflet_circleMarker(E(leaflet_latLng(cols[i][0], cols[i][1])), col_opts), m);
    }

    Leaflet_LatLngBounds plan_bounds = leaflet_latLngBounds(E(leaflet_latLng(0.0, 0.0)), E(leaflet_latLng(100.0, 160.0)));
    Leaflet_Map_fitBounds(m, Leaflet_LatLngBoundsExpression_of_Leaflet_LatLngBounds(plan_bounds));

    Leaflet_Point px = Leaflet_Map_latLngToContainerPoint(m, E(leaflet_latLng(50.0, 80.0)));
    char buf[160];
    snprintf(buf, sizeof buf, "CRS.Simple 100x160 plan · 4 studio rooms + atrium · center @ (%d, %d) px",
        (int)(Leaflet_Point_get_x(px) + 0.5), (int)(Leaflet_Point_get_y(px) + 0.5));
    HTMLElement meta = Document_getElementById(get_document(), "meta");
    if (meta.h != 0) HTMLElement_set_textContent(meta, buf);
    js_scope_close();
}
