#![no_main]

use leaflet::*;

fn box_opts(
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
        .with_crs(&crs::simple())
        .with_min_zoom(-1.0)
        .with_max_zoom(3.0)
        .with_attribution_control(false)
        .with_zoom_animation(false)
        .with_fade_animation(false)
        .with_marker_zoom_animation(false);
    let m = map_with("map", &m_opts);

    let grid_opts = box_opts("#e5e5e5", 1.0, None, "#ffffff", 0.0);
    for i in 0..=5 {
        let r = (i * 20) as f64;
        let line = [lat_lng(r, 0.0), lat_lng(r, 160.0)];
        polyline_with(line, &grid_opts).add_to(&m);
    }
    for i in 0..=8 {
        let c = (i * 20) as f64;
        let line = [lat_lng(0.0, c), lat_lng(100.0, c)];
        polyline_with(line, &grid_opts).add_to(&m);
    }

    let env = lat_lng_bounds(&lat_lng(8.0, 12.0), &lat_lng(92.0, 148.0));
    rectangle_with(&env, &box_opts("#111111", 2.5, None, "#ffffff", 0.9)).add_to(&m);

    let rooms = [
        (12.0, 16.0, 46.0, 62.0, "#f7f7f7"),
        (54.0, 16.0, 88.0, 62.0, "#efefef"),
        (12.0, 98.0, 46.0, 144.0, "#e5e5e5"),
        (54.0, 98.0, 88.0, 144.0, "#d4d4d4"),
    ];
    for (r0, c0, r1, c1, fill) in rooms {
        let b = lat_lng_bounds(&lat_lng(r0, c0), &lat_lng(r1, c1));
        rectangle_with(&b, &box_opts("#404040", 1.5, None, fill, 0.85)).add_to(&m);
    }

    let atrium = JsArray::<PolylineSetLatLngsLatlngs>::from([
        lat_lng(50.0, 64.0),
        lat_lng(74.0, 80.0),
        lat_lng(50.0, 96.0),
        lat_lng(26.0, 80.0),
    ]);
    polygon_with(atrium, &box_opts("#111111", 2.0, None, "#555555", 0.35)).add_to(&m);

    let spine = [lat_lng(50.0, 12.0), lat_lng(50.0, 148.0)];
    polyline_with(
        spine,
        &box_opts("#737373", 1.5, Some("4 4"), "#ffffff", 0.0),
    )
    .add_to(&m);

    let col_opts = CircleMarkerOptions::default()
        .with_radius(5.0)
        .with_color("#111111")
        .with_weight(1.5)
        .with_fill_color("#111111")
        .with_fill_opacity(1.0);
    for (r, c) in [
        (12.0, 16.0),
        (88.0, 16.0),
        (12.0, 80.0),
        (88.0, 80.0),
        (12.0, 144.0),
        (88.0, 144.0),
    ] {
        circle_marker_with(&lat_lng(r, c), &col_opts).add_to(&m);
    }

    let plan_bounds = lat_lng_bounds(&lat_lng(0.0, 0.0), &lat_lng(100.0, 160.0));
    m.fit_bounds(&plan_bounds);

    let px = m.lat_lng_to_container_point(&lat_lng(50.0, 80.0));
    if let Some(meta) = dom::document().get_element_by_id("meta") {
        let text = format!(
            "CRS.Simple 100x160 plan · 4 studio rooms + atrium · center @ ({:.0}, {:.0}) px",
            px.x(),
            px.y()
        );
        meta.set_text_content(&text);
    }
}
