#include "dom/dom.h"
#include "leaflet/leaflet.h"
#include <stdio.h>

static int g_clicks = 0;
static Leaflet_Map g_map = { 0 };
static Leaflet_Popup g_note = { 0 };

static void on_click(void* ctx, LeafletMouseEvent e) {
    (void)ctx;
    js_scope();
    Leaflet_LatLng ll = LeafletMouseEvent_get_latlng(e);
    double lat = Leaflet_LatLng_get_lat(ll);
    double lng = Leaflet_LatLng_get_lng(ll);
    if (lat <= 0.0) return;
    g_clicks++;
    char text[160];
    if (g_clicks > 2) {
        snprintf(text, sizeof text, "Click #%d \xC2\xB7 %.2fN, %.2fE", g_clicks, lat, lng);
        Leaflet_Popup_setLatLng(g_note, ll);
        Leaflet_Popup_setContent(g_note, text);
        Leaflet_Popup_openOn(g_note, g_map);
    }
    snprintf(text, sizeof text, "5 DivIcon stations \xC2\xB7 clicks=%d \xC2\xB7 last click (%.2fN, %.2fE)", g_clicks, lat, lng);
    HTMLElement meta = Document_getElementById(get_document(), "meta");
    if (js_not_null(meta)) HTMLElement_set_textContent(meta, text);
}

__attribute__((export_name("start"))) void start(void) {
    js_scope();

    Leaflet_Map m = leaflet_map("map", Leaflet_MapOptions_create(
        .attributionControl = false, .zoomAnimation = false, .fadeAnimation = false, .markerZoomAnimation = false));

    static const char* tips[5] = {
        "01 · Geneva", "02 · Lausanne", "03 · Bern", "04 · Zurich", "05 · Basel"
    };
    static const char* htmls[5] = {
        "<span>01</span>", "<span>02</span>", "<span>03</span>", "<span>04</span>", "<span>05</span>"
    };
    Leaflet_LatLng pts[5] = {
        leaflet_latLng(46.2044, 6.1432),
        leaflet_latLng(46.5197, 6.6323),
        leaflet_latLng(46.9480, 7.4474),
        leaflet_latLng(47.3769, 8.5417),
        leaflet_latLng(47.5596, 7.5886),
    };

    JsArray_Leaflet_LatLng line_pts = JsArray_Leaflet_LatLng_new();
    for (int i = 0; i < 5; i++) JsArray_Leaflet_LatLng_push(line_pts, pts[i]);

    Leaflet_Polyline corridor = leaflet_polyline(line_pts,
        Leaflet_PolylineOptions_create(.smoothFactor = 1.0, .color = "#404040", .weight = 2.5));
    Leaflet_Polyline_addTo(corridor, m);

    Leaflet_CircleMarkerOptions dot_opts = Leaflet_CircleMarkerOptions_create(
        .radius = 6.0, .color = "#111111", .weight = 2.0, .fillColor = "#ffffff", .fillOpacity = 1.0);

    Leaflet_Point pin_size = Leaflet_Point_new(22.0, 22.0);
    Leaflet_Point pin_anchor = Leaflet_Point_new(11.0, 11.0);

    for (int i = 0; i < 5; i++) {
        Leaflet_CircleMarker cm = leaflet_circleMarker(pts[i], dot_opts);
        Leaflet_CircleMarker_bindTooltip(cm, tips[i]);
        Leaflet_CircleMarker_addTo(cm, m);

        Leaflet_DivIcon pin = leaflet_divIcon(Leaflet_DivIconOptions_create(
            .className = "mono-pin", .html = htmls[i],
            .iconSize_point = pin_size, .iconAnchor_point = pin_anchor));
        Leaflet_Marker_addTo(leaflet_marker(pts[i], Leaflet_MarkerOptions_create(.icon_div = pin)), m);
    }

    Leaflet_Map_fitBounds(m, Leaflet_LatLngBounds_pad(Leaflet_Polyline_getBounds(corridor), 0.15));

    Leaflet_Popup note = leaflet_popup(pts[2], Leaflet_PopupOptions_create(.maxWidth = 220.0));
    Leaflet_Popup_setContent(note, "03 Bern · 46.95N, 7.45E");
    Leaflet_Popup_openOn(note, m);

    g_map = m;
    js_retain(g_map);
    g_note = note;
    js_retain(g_note);

    Leaflet_Map_on_click(m, on_click);
    LeafletMouseEvent data = LeafletMouseEvent_create(.latlng = pts[2]);
    Leaflet_Map_fire(m, "click", js_value(data));
    Leaflet_Map_fire(m, "click", js_value(data));
}
