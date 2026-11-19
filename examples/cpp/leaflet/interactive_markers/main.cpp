#include <cstdio>
#include <string>

#include "dom/dom.hpp"
#include "leaflet/leaflet.hpp"

using namespace leaflet;

static ts::Value val_of(const ts::Handle& h) {
    ts::Value v;
    static_cast<ts::Handle&>(v) = h;
    return v;
}

extern "C" __attribute__((export_name("start"))) void start() {
    MapOptions m_opts = MapOptions::create();
    m_opts.attributionControl(false);
    m_opts.zoomAnimation(false);
    m_opts.fadeAnimation(false);
    m_opts.markerZoomAnimation(false);
    Map m = leaflet::map(Content(std::string("map")), m_opts);

    struct Station {
        const char* code;
        const char* name;
        LatLng pt;
    };
    Station stations[5] = {
        { "01", "Geneva", leaflet::latLng(46.2044, 6.1432) },
        { "02", "Lausanne", leaflet::latLng(46.5197, 6.6323) },
        { "03", "Bern", leaflet::latLng(46.9480, 7.4474) },
        { "04", "Zurich", leaflet::latLng(47.3769, 8.5417) },
        { "05", "Basel", leaflet::latLng(47.5596, 7.5886) },
    };

    ts::Array<> line_pts = ts::Array<>::create();
    for (const Station& s : stations) line_pts.push(LatLngExpression(s.pt));
    PolylineOptions p_opts = PolylineOptions::create();
    p_opts.smoothFactor(1.0);
    p_opts.color(std::string("#404040"));
    p_opts.weight(2.5);
    Polyline<> corridor = leaflet::polyline(line_pts, p_opts);
    corridor.addTo(m);

    CircleMarkerOptions dot_opts = CircleMarkerOptions::create();
    dot_opts.radius(6.0);
    dot_opts.color(std::string("#111111"));
    dot_opts.weight(2.0);
    dot_opts.fillColor(std::string("#ffffff"));
    dot_opts.fillOpacity(1.0);

    PointExpression pin_size(leaflet::point(22.0, 22.0));
    PointExpression pin_anchor(leaflet::point(11.0, 11.0));

    for (const Station& s : stations) {
        CircleMarker<> cm = leaflet::circleMarker(s.pt, dot_opts);
        cm.bindTooltip(Layer_bindTooltip_content(std::string(s.code) + " · " + s.name));
        cm.addTo(m);

        DivIconOptions d_opts = DivIconOptions::create();
        d_opts.className(std::string("mono-pin"));
        d_opts.html(string_or_Element_or_boolean(std::string("<span>") + s.code + "</span>"));
        d_opts.iconSize(pin_size);
        d_opts.iconAnchor(pin_anchor);

        MarkerOptions mk_opts = MarkerOptions::create();
        mk_opts.icon(Icon_or_DivIcon(leaflet::divIcon(d_opts)));
        leaflet::marker(s.pt, mk_opts).addTo(m);
    }

    m.fitBounds(LatLngBoundsExpression(corridor.getBounds().pad(0.15)));

    PopupOptions pop_opts = PopupOptions::create();
    pop_opts.maxWidth(220.0);
    Popup note = leaflet::popup(stations[2].pt, pop_opts);
    note.setContent(string_or_HTMLElement_or_DivOverlay_getContent_result(std::string("03 Bern · 46.95N, 7.45E")));
    note.openOn(m);

    int clicks = 0;
    LeafletMouseEventHandlerFn on_click = [&](LeafletMouseEvent ev) {
        if (ev.latlng().lat() > 0.0) clicks++;
    };
    m.on(Evented_addEventListener_type_click::click, on_click);
    LeafletMouseEvent payload = LeafletMouseEvent::create();
    payload.latlng(stations[2].pt);
    m.fire("click", val_of(payload));
    m.fire("click", val_of(payload));
    m.off(Evented_addEventListener_type_click::click, on_click);
    on_click.release();

    char buf[160];
    std::snprintf(
        buf, sizeof(buf),
        "5 DivIcon stations · popup open (Bern) · %d synthetic clicks dispatched",
        clicks
    );
    if (auto meta = dom::get_document().getElementById("meta")) {
        meta->textContent(buf);
    }
}
