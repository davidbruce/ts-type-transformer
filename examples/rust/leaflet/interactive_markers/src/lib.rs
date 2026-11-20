#![no_main]

use leaflet::*;
use std::cell::Cell;
use std::rc::Rc;

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let m_opts = MapOptions::default()
        .with_attribution_control(false)
        .with_zoom_animation(false)
        .with_fade_animation(false)
        .with_marker_zoom_animation(false);
    let m = map("map", Some(&m_opts));

    let stations = [
        ("01", "Geneva", lat_lng(46.2044, 6.1432, None)),
        ("02", "Lausanne", lat_lng(46.5197, 6.6323, None)),
        ("03", "Bern", lat_lng(46.9480, 7.4474, None)),
        ("04", "Zurich", lat_lng(47.3769, 8.5417, None)),
        ("05", "Basel", lat_lng(47.5596, 7.5886, None)),
    ];

    let line_pts: JsArray<JsValue> = stations.iter().map(|(_, _, p)| JsValue::from(p)).collect();
    let p_opts = PolylineOptions::default()
        .with_smooth_factor(1.0)
        .with_color("#404040")
        .with_weight(2.5);
    let corridor = polyline(line_pts, Some(&p_opts));
    corridor.add_to(&m);

    let dot_opts = CircleMarkerOptions::default()
        .with_radius(6.0)
        .with_color("#111111")
        .with_weight(2.0)
        .with_fill_color("#ffffff")
        .with_fill_opacity(1.0);

    let pin_size = point(22.0, 22.0, None);
    let pin_anchor = point(11.0, 11.0, None);

    for (code, name, pt) in &stations {
        let cm = circle_marker(pt, Some(&dot_opts));
        cm.bind_tooltip(format!("{code} · {name}"), None);
        cm.add_to(&m);

        let d_opts = DivIconOptions::default()
            .with_class_name("mono-pin")
            .with_html(format!("<span>{code}</span>"))
            .with_icon_size(pin_size.clone())
            .with_icon_anchor(pin_anchor.clone());

        let mk_opts = MarkerOptions::default().with_icon(div_icon(Some(&d_opts)));
        marker(pt, Some(&mk_opts)).add_to(&m);
    }

    m.fit_bounds(&corridor.get_bounds().pad(0.15), None);

    let pop_opts = PopupOptions::default().with_max_width(220.0);
    let note = popup(&stations[2].2, Some(&pop_opts));
    note.set_content("03 Bern · 46.95N, 7.45E");
    note.open_on(&m);

    let clicks = Rc::new(Cell::new(0usize));
    let on_click = LeafletMouseEventHandlerFn::new({
        let clicks = clicks.clone();
        move |ev: LeafletMouseEvent| {
            if ev.latlng().lat() > 0.0 {
                clicks.set(clicks.get() + 1);
            }
        }
    });
    m.on_with_evented_add_event_listener_type_click(
        EventedAddEventListenerTypeClick::Click,
        &on_click,
        None,
    );
    let data = LeafletMouseEvent::default().with_latlng(&stations[2].2);
    let payload = JsValue::from(&data);
    m.fire("click", Some(&payload), None);
    m.fire("click", Some(&payload), None);

    if let Some(meta) = dom::document().get_element_by_id("meta") {
        let text = format!(
            "{} DivIcon stations · popup open (Bern) · {} synthetic clicks dispatched",
            stations.len(),
            clicks.get()
        );
        meta.set_text_content(Some(&text));
    }
}
