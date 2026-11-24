#![no_main]

use leaflet::*;

fn poly_opts(
    color: &str,
    weight: f64,
    dash: Option<&str>,
    fill_color: &str,
    fill_opacity: f64,
) -> PolylineOptions {
    let mut opts = PolylineOptions::default()
        .with_smooth_factor(1.0)
        .with_color(color)
        .with_weight(weight)
        .with_fill_color(fill_color)
        .with_fill_opacity(fill_opacity);
    if let Some(d) = dash {
        opts = opts.with_dash_array(d);
    }
    opts
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let m_opts = MapOptions::default()
        .with_attribution_control(false)
        .with_zoom_animation(false)
        .with_fade_animation(false)
        .with_marker_zoom_animation(false);
    let m = map_with("map", &m_opts);

    let mid_pts = JsArray::<PolylineSetLatLngsLatlngs>::from([
        lat_lng(47.2, 7.6),
        lat_lng(48.0, 8.6),
        lat_lng(47.5, 9.7),
        lat_lng(46.8, 9.5),
        lat_lng(46.6, 8.4),
        lat_lng(46.9, 7.3),
    ]);

    let core_pts = JsArray::<PolylineSetLatLngsLatlngs>::from([
        lat_lng(47.35, 8.2),
        lat_lng(47.55, 8.75),
        lat_lng(47.15, 9.0),
        lat_lng(46.95, 8.4),
    ]);

    let mid_poly = polygon_with(mid_pts, &poly_opts("#404040", 2.0, None, "#d4d4d4", 0.55));
    let core_poly = polygon_with(core_pts, &poly_opts("#111111", 2.5, None, "#404040", 0.75));

    let mid_bounds = mid_poly.get_bounds();
    let core_bounds = core_poly.get_bounds();
    let padded = mid_bounds.pad(0.18);
    let hull_rect = rectangle_with(
        &padded,
        &poly_opts("#737373", 1.5, Some("4 4"), "#efefef", 0.05),
    );

    hull_rect.add_to(&m);
    mid_poly.add_to(&m);
    core_poly.add_to(&m);

    let probes = [
        lat_lng(47.25, 8.6),
        lat_lng(47.15, 8.55),
        lat_lng(47.7, 8.6),
        lat_lng(47.0, 9.2),
        lat_lng(46.8, 7.9),
        lat_lng(47.85, 7.5),
        lat_lng(47.9, 9.7),
        lat_lng(46.55, 9.6),
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
        let cm_opts = CircleMarkerOptions::default()
            .with_radius(6.0)
            .with_color("#111111")
            .with_weight(2.0)
            .with_fill_color(fill_hex)
            .with_fill_opacity(1.0);
        circle_marker_with(pt, &cm_opts).add_to(&m);
    }

    m.fit_bounds(&padded);

    if let Some(meta) = dom::document().get_element_by_id("meta") {
        let text = format!(
            "2 nested polygons · 1 padded hull · probes: {in_core} core · {in_mid} sector · {exterior} exterior"
        );
        meta.set_text_content(&text);
    }
}
