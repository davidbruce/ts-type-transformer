#include "dom/dom.h"
#include "leaflet/leaflet.h"
#include <stdio.h>

static Leaflet_LatLngExpression E(Leaflet_LatLng ll) { return Leaflet_LatLngExpression_of_Leaflet_LatLng(ll); }

static int g_clicks = 0;

static void on_click(void* ctx, LeafletMouseEvent e) {
    (void)ctx;
    js_scope_open();
    if (Leaflet_LatLng_get_lat(LeafletMouseEvent_get_latlng(e)) > 0.0) {
        g_clicks++;
    }
    js_scope_close();
}

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();

    Leaflet_MapOptions m_opts = Leaflet_MapOptions_create();
    Leaflet_MapOptions_set_attributionControl(m_opts, false);
    Leaflet_MapOptions_set_zoomAnimation(m_opts, false);
    Leaflet_MapOptions_set_fadeAnimation(m_opts, false);
    Leaflet_MapOptions_set_markerZoomAnimation(m_opts, false);
    Leaflet_Map m = leaflet_map("map", m_opts);

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

    JsArray_Leaflet_LatLngExpression line_pts = JsArray_Leaflet_LatLngExpression_new();
    for (int i = 0; i < 5; i++) JsArray_Leaflet_LatLngExpression_push(line_pts, E(pts[i]));

    Leaflet_PolylineOptions p_opts = Leaflet_PolylineOptions_create();
    Leaflet_PolylineOptions_set_smoothFactor(p_opts, 1.0);
    Leaflet_PolylineOptions_set_color(p_opts, "#404040");
    Leaflet_PolylineOptions_set_weight(p_opts, 2.5);
    Leaflet_Polyline corridor = leaflet_polyline(line_pts, p_opts);
    Leaflet_Polyline_addTo(corridor, m);

    Leaflet_CircleMarkerOptions dot_opts = Leaflet_CircleMarkerOptions_create();
    Leaflet_CircleMarkerOptions_set_radius(dot_opts, 6.0);
    Leaflet_CircleMarkerOptions_set_color(dot_opts, "#111111");
    Leaflet_CircleMarkerOptions_set_weight(dot_opts, 2.0);
    Leaflet_CircleMarkerOptions_set_fillColor(dot_opts, "#ffffff");
    Leaflet_CircleMarkerOptions_set_fillOpacity(dot_opts, 1.0);

    Leaflet_PointExpression pin_size = Leaflet_PointExpression_of(Leaflet_Point_new(22.0, 22.0));
    Leaflet_PointExpression pin_anchor = Leaflet_PointExpression_of(Leaflet_Point_new(11.0, 11.0));

    for (int i = 0; i < 5; i++) {
        Leaflet_CircleMarker cm = leaflet_circleMarker(E(pts[i]), dot_opts);
        Leaflet_CircleMarker_bindTooltip(cm, Leaflet_Content_of(tips[i]));
        Leaflet_CircleMarker_addTo(cm, m);

        Leaflet_DivIconOptions d_opts = Leaflet_DivIconOptions_create();
        Leaflet_DivIconOptions_set_className(d_opts, "mono-pin");
        Leaflet_DivIconOptions_set_html(d_opts, Leaflet_string_or_Element_or_boolean_of(htmls[i]));
        Leaflet_DivIconOptions_set_iconSize(d_opts, pin_size);
        Leaflet_DivIconOptions_set_iconAnchor(d_opts, pin_anchor);

        Leaflet_MarkerOptions mk_opts = Leaflet_MarkerOptions_create();
        Leaflet_MarkerOptions_set_icon(mk_opts, Leaflet_Icon_or_DivIcon_of(leaflet_divIcon(d_opts)));
        Leaflet_Marker_addTo(leaflet_marker(E(pts[i]), mk_opts), m);
    }

    Leaflet_Map_fitBounds(m, Leaflet_LatLngBoundsExpression_of_Leaflet_LatLngBounds(Leaflet_LatLngBounds_pad(Leaflet_Polyline_getBounds(corridor), 0.15)));

    Leaflet_PopupOptions pop_opts = Leaflet_PopupOptions_create();
    Leaflet_PopupOptions_set_maxWidth(pop_opts, 220.0);
    Leaflet_Popup note = leaflet_popup(E(pts[2]), pop_opts);
    Leaflet_Popup_setContent(note, Leaflet_Content_of("03 Bern · 46.95N, 7.45E"));
    Leaflet_Popup_openOn(note, m);

    LeafletMouseEventHandlerFn handler = LeafletMouseEventHandlerFn_from(on_click, 0);
    Leaflet_Map_on_with_Evented_addEventListener_type_click(m, Leaflet_Evented_addEventListener_type_click_click, handler, (JsValue){ 0 });
    LeafletMouseEvent data = LeafletMouseEvent_create();
    LeafletMouseEvent_set_latlng(data, pts[2]);
    Leaflet_Map_fire(m, "click", js_value(data));
    Leaflet_Map_fire(m, "click", js_value(data));

    char buf[160];
    snprintf(buf, sizeof buf, "5 DivIcon stations · popup open (Bern) · %d synthetic clicks dispatched", g_clicks);
    HTMLElement meta = Document_getElementById(get_document(), "meta");
    if (meta.h != 0) HTMLElement_set_textContent(meta, buf);
    js_scope_close();
}
