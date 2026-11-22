#include <cstdio>
#include <memory>
#include <string>

#include "dom/dom.hpp"
#include "leaflet/leaflet.hpp"

using namespace leaflet;

TS_EXPORT(start) void start() {
    Map m = leaflet::map("map", {{.attributionControl = false, .fadeAnimation = false, .markerZoomAnimation = false, .zoomAnimation = false}});

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

    auto line_pts = ts::Array<LatLng>::create();
    for (const Station& s : stations) line_pts.push(s.pt);
    Polyline corridor = leaflet::polyline(line_pts, {{.color = "#404040", .weight = 2.5, .smoothFactor = 1}});
    corridor.addTo(m);

    CircleMarkerOptions dot_opts({.color = "#111111", .weight = 2, .fillColor = "#ffffff", .fillOpacity = 1, .radius = 6});

    Point pin_size = leaflet::point(22.0, 22.0);
    Point pin_anchor = leaflet::point(11.0, 11.0);

    for (const Station& s : stations) {
        CircleMarker cm = leaflet::circleMarker(s.pt, dot_opts);
        cm.bindTooltip(std::string(s.code) + " · " + s.name);
        cm.addTo(m);

        DivIcon pin = leaflet::divIcon({{.html = std::string("<span>") + s.code + "</span>", .iconSize = pin_size, .iconAnchor = pin_anchor, .className = "mono-pin"}});
        leaflet::marker(s.pt, {{.icon = pin}}).addTo(m);
    }

    m.fitBounds(corridor.getBounds().pad(0.15));

    Popup note = leaflet::popup(stations[2].pt, {{.maxWidth = 220}});
    note.setContent("03 Bern · 46.95N, 7.45E");
    note.openOn(m);

    struct ClickState {
        int clicks = 0;
        Map m;
        Popup note;
    };
    auto st = std::make_shared<ClickState>(ClickState{ 0, m, note });
    LeafletMouseEventHandlerFn on_click([st](LeafletMouseEvent ev) {
        LatLng ll = ev.latlng();
        double lat = ll.lat();
        double lng = ll.lng();
        if (lat <= 0.0) return;
        st->clicks++;
        char buf[160];
        if (st->clicks > 2) {
            std::snprintf(buf, sizeof(buf), "Click #%d \xC2\xB7 %.2fN, %.2fE", st->clicks, lat, lng);
            st->note.setLatLng(ll);
            st->note.setContent(buf);
            st->note.openOn(st->m);
        }
        std::snprintf(
            buf, sizeof(buf),
            "5 DivIcon stations \xC2\xB7 clicks=%d \xC2\xB7 last click (%.2fN, %.2fE)",
            st->clicks, lat, lng
        );
        if (auto meta = dom::get_document().getElementById("meta")) {
            meta->textContent(buf);
        }
    });
    m.on_click(on_click);
    LeafletMouseEvent payload = LeafletMouseEvent::create();
    payload.latlng(stations[2].pt);
    m.fire("click", payload);
    m.fire("click", payload);
}
