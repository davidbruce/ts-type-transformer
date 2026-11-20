#include "dom/dom.h"
#include "leaflet/leaflet.h"
#include <stdio.h>

static LatLngExpression E(LatLng ll) { return LatLngExpression_of_LatLng(ll); }

static int g_clicks = 0;

static void on_click(void* ctx, LeafletMouseEvent e) {
    (void)ctx;
    js_scope_open();
    if (LatLng_get_lat(LeafletMouseEvent_get_latlng(e)) > 0.0) {
        g_clicks++;
    }
    js_scope_close();
}

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();

    MapOptions m_opts = MapOptions_create();
    MapOptions_set_attributionControl(m_opts, false);
    MapOptions_set_zoomAnimation(m_opts, false);
    MapOptions_set_fadeAnimation(m_opts, false);
    MapOptions_set_markerZoomAnimation(m_opts, false);
    Map_2 m = map("map", m_opts);

    static const char* tips[5] = {
        "01 · Geneva", "02 · Lausanne", "03 · Bern", "04 · Zurich", "05 · Basel"
    };
    static const char* htmls[5] = {
        "<span>01</span>", "<span>02</span>", "<span>03</span>", "<span>04</span>", "<span>05</span>"
    };
    LatLng pts[5] = {
        latLng(46.2044, 6.1432),
        latLng(46.5197, 6.6323),
        latLng(46.9480, 7.4474),
        latLng(47.3769, 8.5417),
        latLng(47.5596, 7.5886),
    };

    JsArray_LatLngExpression line_pts = JsArray_LatLngExpression_new();
    for (int i = 0; i < 5; i++) JsArray_LatLngExpression_push(line_pts, E(pts[i]));

    PolylineOptions p_opts = PolylineOptions_create();
    PolylineOptions_set_smoothFactor(p_opts, 1.0);
    PolylineOptions_set_color(p_opts, "#404040");
    PolylineOptions_set_weight(p_opts, 2.5);
    Polyline corridor = polyline(line_pts, p_opts);
    Polyline_addTo(corridor, m);

    CircleMarkerOptions dot_opts = CircleMarkerOptions_create();
    CircleMarkerOptions_set_radius(dot_opts, 6.0);
    CircleMarkerOptions_set_color(dot_opts, "#111111");
    CircleMarkerOptions_set_weight(dot_opts, 2.0);
    CircleMarkerOptions_set_fillColor(dot_opts, "#ffffff");
    CircleMarkerOptions_set_fillOpacity(dot_opts, 1.0);

    PointExpression pin_size = PointExpression_of(Point_new(22.0, 22.0));
    PointExpression pin_anchor = PointExpression_of(Point_new(11.0, 11.0));

    for (int i = 0; i < 5; i++) {
        CircleMarker cm = circleMarker(E(pts[i]), dot_opts);
        CircleMarker_bindTooltip(cm, Content_of(tips[i]));
        CircleMarker_addTo(cm, m);

        DivIconOptions d_opts = DivIconOptions_create();
        DivIconOptions_set_className(d_opts, "mono-pin");
        DivIconOptions_set_html(d_opts, string_or_Element_or_boolean_of(htmls[i]));
        DivIconOptions_set_iconSize(d_opts, pin_size);
        DivIconOptions_set_iconAnchor(d_opts, pin_anchor);

        MarkerOptions mk_opts = MarkerOptions_create();
        MarkerOptions_set_icon(mk_opts, Icon_or_DivIcon_of(divIcon(d_opts)));
        Marker_addTo(marker(E(pts[i]), mk_opts), m);
    }

    Map_2_fitBounds(m, LatLngBoundsExpression_of_LatLngBounds(LatLngBounds_pad(Polyline_getBounds(corridor), 0.15)));

    PopupOptions pop_opts = PopupOptions_create();
    PopupOptions_set_maxWidth(pop_opts, 220.0);
    Popup note = popup(E(pts[2]), pop_opts);
    Popup_setContent(note, Content_of("03 Bern · 46.95N, 7.45E"));
    Popup_openOn(note, m);

    LeafletMouseEventHandlerFn handler = LeafletMouseEventHandlerFn_from(on_click, 0);
    Map_2_on_with_Evented_addEventListener_type_click(m, Evented_addEventListener_type_click_click, handler, (JsValue){ 0 });
    LeafletMouseEvent data = LeafletMouseEvent_create();
    LeafletMouseEvent_set_latlng(data, pts[2]);
    Map_2_fire(m, "click", js_value(data));
    Map_2_fire(m, "click", js_value(data));

    char buf[160];
    snprintf(buf, sizeof buf, "5 DivIcon stations · popup open (Bern) · %d synthetic clicks dispatched", g_clicks);
    HTMLElement meta = Document_getElementById(get_document(), "meta");
    if (meta.h != 0) HTMLElement_set_textContent(meta, buf);
    js_scope_close();
}
