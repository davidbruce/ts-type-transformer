#include "dom/dom.h"
#include "leaflet/leaflet.h"

static LatLngExpression E(LatLng ll) { return LatLngExpression_of_LatLng(ll); }

static int g_clicks = 0;

static void on_click(void* ctx, LeafletMouseEvent e) {
    (void)ctx;
    if (LatLng_get_lat(LeafletMouseEvent_get_latlng(e)) > 0.0) {
        g_clicks++;
    }
}

static int append_str(char* buf, int pos, const char* s) {
    while (*s) buf[pos++] = *s++;
    return pos;
}

static int append_int(char* buf, int pos, int val) {
    if (val == 0) {
        buf[pos++] = '0';
        return pos;
    }
    char tmp[16];
    int n = 0;
    while (val > 0) {
        tmp[n++] = (char)('0' + (val % 10));
        val /= 10;
    }
    for (int i = n - 1; i >= 0; i--) buf[pos++] = tmp[i];
    return pos;
}

__attribute__((export_name("start"))) void start(void) {
    MapOptions m_opts = MapOptions_create();
    MapOptions_set_attributionControl(m_opts, js_some_bool(false));
    MapOptions_set_zoomAnimation(m_opts, js_some_bool(false));
    MapOptions_set_fadeAnimation(m_opts, js_some_bool(false));
    MapOptions_set_markerZoomAnimation(m_opts, js_some_bool(false));
    Map_2 m = map("map", m_opts);

    static const char* codes[5] = { "01", "02", "03", "04", "05" };
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
    PolylineOptions_set_smoothFactor(p_opts, js_some_double(1.0));
    PathOptions_set_color(p_opts, "#404040");
    PathOptions_set_weight(p_opts, js_some_double(2.5));
    Polyline corridor = polyline(line_pts, p_opts);
    Layer_addTo(corridor, m);

    CircleMarkerOptions dot_opts = CircleMarkerOptions_create();
    CircleMarkerOptions_set_radius(dot_opts, js_some_double(6.0));
    PathOptions_set_color(dot_opts, "#111111");
    PathOptions_set_weight(dot_opts, js_some_double(2.0));
    PathOptions_set_fillColor(dot_opts, "#ffffff");
    PathOptions_set_fillOpacity(dot_opts, js_some_double(1.0));

    PointExpression pin_size = PointExpression_of_Point(Point_new(22.0, 22.0, js_none_bool()));
    PointExpression pin_anchor = PointExpression_of_Point(Point_new(11.0, 11.0, js_none_bool()));

    for (int i = 0; i < 5; i++) {
        (void)codes[i];
        CircleMarker cm = circleMarker(E(pts[i]), dot_opts);
        Layer_bindTooltip(cm, Content_of_string(JsString_from(tips[i])));
        Layer_addTo(cm, m);

        DivIconOptions d_opts = DivIconOptions_create();
        DivIconOptions_set_className(d_opts, "mono-pin");
        DivIconOptions_set_html(d_opts, string_or_Element_or_boolean_of_string(JsString_from(htmls[i])));
        DivIconOptions_set_iconSize(d_opts, pin_size);
        DivIconOptions_set_iconAnchor(d_opts, pin_anchor);

        MarkerOptions mk_opts = MarkerOptions_create();
        MarkerOptions_set_icon(mk_opts, Icon_or_DivIcon_of_DivIcon(divIcon(d_opts)));
        Layer_addTo(marker(E(pts[i]), mk_opts), m);
    }

    Map_2_fitBounds(m, LatLngBoundsExpression_of_LatLngBounds(LatLngBounds_pad(Polyline_getBounds(corridor), 0.15)));

    PopupOptions pop_opts = PopupOptions_create();
    PopupOptions_set_maxWidth(pop_opts, js_some_double(220.0));
    Popup note = popup(E(pts[2]), pop_opts);
    DivOverlay_setContent(note, Content_of_string(JsString_from("03 Bern · 46.95N, 7.45E")));
    DivOverlay_openOn(note, m);

    LeafletMouseEventHandlerFn handler = LeafletMouseEventHandlerFn_from(on_click, 0);
    Evented_on_with_Evented_addEventListener_type_click(m, Evented_addEventListener_type_click_click, handler, (JsValue){ 0 });
    LeafletMouseEvent data = LeafletMouseEvent_create();
    LeafletMouseEvent_set_latlng(data, pts[2]);
    Evented_fire(m, "click", (JsValue){ data.h }, js_none_bool());
    Evented_fire(m, "click", (JsValue){ data.h }, js_none_bool());

    char buf[160];
    int p = 0;
    p = append_str(buf, p, "5 DivIcon stations · popup open (Bern) · ");
    p = append_int(buf, p, g_clicks);
    p = append_str(buf, p, " synthetic clicks dispatched");
    buf[p] = '\0';

    Document doc = get_document();
    HTMLElement meta = Document_getElementById(doc, "meta");
    if (meta.h != 0) Node_set_textContent(meta, buf);
}
