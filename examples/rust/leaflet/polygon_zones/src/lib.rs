#![no_main]

use leaflet::*;

fn poly_opts(
    color: &str,
    weight: f64,
    dash: Option<&str>,
    fill_color: &str,
    fill_opacity: f64,
) -> PolylineOptions {
    let opts = PolylineOptions::create();
    opts.set_smooth_factor(Some(1.0));
    opts.set_color(Some(color));
    opts.set_weight(Some(weight));
    if let Some(d) = dash {
        opts.set_dash_array(StringOrNumberArray::String(d.into()));
    }
    opts.set_fill_color(Some(fill_color));
    opts.set_fill_opacity(Some(fill_opacity));
    opts
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let m_opts = MapOptions::create();
    m_opts.set_attribution_control(Some(false));
    m_opts.set_zoom_animation(Some(false));
    m_opts.set_fade_animation(Some(false));
    m_opts.set_marker_zoom_animation(Some(false));
    let m = map("map", Some(&m_opts));

    let mid_pts = JsArray::from(vec![
        PolylineSetLatLngsLatlngs::LatLng(lat_lng(47.2, 7.6, None)),
        PolylineSetLatLngsLatlngs::LatLng(lat_lng(48.0, 8.6, None)),
        PolylineSetLatLngsLatlngs::LatLng(lat_lng(47.5, 9.7, None)),
        PolylineSetLatLngsLatlngs::LatLng(lat_lng(46.8, 9.5, None)),
        PolylineSetLatLngsLatlngs::LatLng(lat_lng(46.6, 8.4, None)),
        PolylineSetLatLngsLatlngs::LatLng(lat_lng(46.9, 7.3, None)),
    ]);

    let core_pts = JsArray::from(vec![
        PolylineSetLatLngsLatlngs::LatLng(lat_lng(47.35, 8.2, None)),
        PolylineSetLatLngsLatlngs::LatLng(lat_lng(47.55, 8.75, None)),
        PolylineSetLatLngsLatlngs::LatLng(lat_lng(47.15, 9.0, None)),
        PolylineSetLatLngsLatlngs::LatLng(lat_lng(46.95, 8.4, None)),
    ]);

    let mid_poly = polygon(
        mid_pts,
        Some(&poly_opts("#404040", 2.0, None, "#d4d4d4", 0.55)),
    );
    let core_poly = polygon(
        core_pts,
        Some(&poly_opts("#111111", 2.5, None, "#404040", 0.75)),
    );

    let mid_bounds = mid_poly.get_bounds();
    let core_bounds = core_poly.get_bounds();
    let padded = mid_bounds.pad(0.18);
    let hull_rect = rectangle(
        &padded,
        Some(&poly_opts("#737373", 1.5, Some("4 4"), "#efefef", 0.05)),
    );

    hull_rect.add_to(&m);
    mid_poly.add_to(&m);
    core_poly.add_to(&m);

    let probes = [
        lat_lng(47.25, 8.6, None),
        lat_lng(47.15, 8.55, None),
        lat_lng(47.7, 8.6, None),
        lat_lng(47.0, 9.2, None),
        lat_lng(46.8, 7.9, None),
        lat_lng(47.85, 7.5, None),
        lat_lng(47.9, 9.7, None),
        lat_lng(46.55, 9.6, None),
    ];

    let mut in_core = 0usize;
    let mut in_mid = 0usize;
    let mut exterior = 0usize;

    for pt in &probes {
        let fill_hex = if core_bounds.contains(pt) {
            in_core += 1;
            "#111111"
        } else if mid_bounds.contains(pt) {
            in_mid += 1;
            "#737373"
        } else {
            exterior += 1;
            "#ffffff"
        };
        let cm_opts = CircleMarkerOptions::create();
        cm_opts.set_radius(Some(6.0));
        cm_opts.set_color(Some("#111111"));
        cm_opts.set_weight(Some(2.0));
        cm_opts.set_fill_color(Some(fill_hex));
        cm_opts.set_fill_opacity(Some(1.0));
        circle_marker(pt, Some(&cm_opts)).add_to(&m);
    }

    m.fit_bounds(&padded, None);

    if let Some(meta) = dom::document().get_element_by_id("meta") {
        let text = format!(
            "2 nested polygons · 1 padded hull · probes: {in_core} core · {in_mid} sector · {exterior} exterior"
        );
        meta.set_text_content(Some(&text));
    }
}
