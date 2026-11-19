#![no_main]

use leaflet::*;

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let m_opts = MapOptions::create();
    m_opts.set_attribution_control(Some(false));
    m_opts.set_zoom_animation(Some(false));
    m_opts.set_fade_animation(Some(false));
    m_opts.set_marker_zoom_animation(Some(false));
    let m = map("map", Some(&m_opts));

    let zurich = lat_lng(47.3769, 8.5417, None);
    m.set_view(&zurich, Some(8.0), None);
    let rings = [
        (130000.0, "#404040", "#d4d4d4", 0.25),
        (90000.0, "#404040", "#a3a3a3", 0.30),
        (55000.0, "#111111", "#737373", 0.40),
        (25000.0, "#111111", "#262626", 0.55),
    ];

    let mut outer_bounds = None;
    for (r, stroke, fill, op) in rings {
        let c_opts = CircleMarkerOptions::create();
        c_opts.set_color(Some(stroke));
        c_opts.set_weight(Some(1.5));
        c_opts.set_fill_color(Some(fill));
        c_opts.set_fill_opacity(Some(op));
        let c = circle_with_radius(&zurich, r, Some(&c_opts));
        c.add_to(&m);
        if outer_bounds.is_none() {
            outer_bounds = Some(c.get_bounds());
        }
    }

    let beacons = [
        lat_lng(47.5596, 7.5886, None),
        lat_lng(46.9480, 7.4474, None),
        lat_lng(47.0502, 8.3093, None),
        lat_lng(47.4245, 9.3767, None),
        lat_lng(46.8508, 9.5320, None),
    ];

    let mut total_dist_km = 0.0;
    for pt in &beacons {
        total_dist_km += zurich.distance_to(pt) / 1000.0;

        let b_opts = CircleMarkerOptions::create();
        b_opts.set_color(Some("#555555"));
        b_opts.set_weight(Some(1.2));
        b_opts.set_dash_array(StringOrNumberArray::String("3 3".into()));
        b_opts.set_fill_color(Some("#737373"));
        b_opts.set_fill_opacity(Some(0.18));
        circle_with_radius(pt, 18000.0, Some(&b_opts)).add_to(&m);

        let d_opts = CircleMarkerOptions::create();
        d_opts.set_radius(Some(4.5));
        d_opts.set_color(Some("#111111"));
        d_opts.set_weight(Some(1.5));
        d_opts.set_fill_color(Some("#111111"));
        d_opts.set_fill_opacity(Some(1.0));
        circle_marker(pt, Some(&d_opts)).add_to(&m);
    }

    if let Some(bounds) = outer_bounds {
        m.fit_bounds(&bounds, None);
    }

    if let Some(meta) = dom::document().get_element_by_id("meta") {
        let avg_km = total_dist_km / beacons.len() as f64;
        let text = format!("4 concentric rings (25–130 km) · 5 regional beacons (avg {avg_km:.1} km)");
        meta.set_text_content(Some(&text));
    }
}
