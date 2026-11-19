#![no_main]

use leaflet::*;

fn box_opts(
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
    m_opts.set_crs(Some(&crs::simple()));
    m_opts.set_min_zoom(Some(-1.0));
    m_opts.set_max_zoom(Some(3.0));
    m_opts.set_attribution_control(Some(false));
    m_opts.set_zoom_animation(Some(false));
    m_opts.set_fade_animation(Some(false));
    m_opts.set_marker_zoom_animation(Some(false));
    let m = map("map", Some(&m_opts));

    let grid_opts = box_opts("#e5e5e5", 1.0, None, "#ffffff", 0.0);
    for i in 0..=5 {
        let r = (i * 20) as f64;
        let line = JsArray::from(vec![
            JsValue::from(&lat_lng(r, 0.0, None)),
            JsValue::from(&lat_lng(r, 160.0, None)),
        ]);
        polyline(line, Some(&grid_opts)).add_to(&m);
    }
    for i in 0..=8 {
        let c = (i * 20) as f64;
        let line = JsArray::from(vec![
            JsValue::from(&lat_lng(0.0, c, None)),
            JsValue::from(&lat_lng(100.0, c, None)),
        ]);
        polyline(line, Some(&grid_opts)).add_to(&m);
    }

    let env = lat_lng_bounds(&lat_lng(8.0, 12.0, None), &lat_lng(92.0, 148.0, None));
    rectangle(&env, Some(&box_opts("#111111", 2.5, None, "#ffffff", 0.9))).add_to(&m);

    let rooms = [
        (12.0, 16.0, 46.0, 62.0, "#f7f7f7"),
        (54.0, 16.0, 88.0, 62.0, "#efefef"),
        (12.0, 98.0, 46.0, 144.0, "#e5e5e5"),
        (54.0, 98.0, 88.0, 144.0, "#d4d4d4"),
    ];
    for (r0, c0, r1, c1, fill) in rooms {
        let b = lat_lng_bounds(&lat_lng(r0, c0, None), &lat_lng(r1, c1, None));
        rectangle(&b, Some(&box_opts("#404040", 1.5, None, fill, 0.85))).add_to(&m);
    }

    let atrium = JsArray::from(vec![
        PolylineSetLatLngsLatlngs::LatLng(lat_lng(50.0, 64.0, None)),
        PolylineSetLatLngsLatlngs::LatLng(lat_lng(74.0, 80.0, None)),
        PolylineSetLatLngsLatlngs::LatLng(lat_lng(50.0, 96.0, None)),
        PolylineSetLatLngsLatlngs::LatLng(lat_lng(26.0, 80.0, None)),
    ]);
    polygon(atrium, Some(&box_opts("#111111", 2.0, None, "#555555", 0.35))).add_to(&m);

    let spine = JsArray::from(vec![
        JsValue::from(&lat_lng(50.0, 12.0, None)),
        JsValue::from(&lat_lng(50.0, 148.0, None)),
    ]);
    polyline(spine, Some(&box_opts("#737373", 1.5, Some("4 4"), "#ffffff", 0.0))).add_to(&m);

    let col_opts = CircleMarkerOptions::create();
    col_opts.set_radius(Some(5.0));
    col_opts.set_color(Some("#111111"));
    col_opts.set_weight(Some(1.5));
    col_opts.set_fill_color(Some("#111111"));
    col_opts.set_fill_opacity(Some(1.0));
    for (r, c) in [
        (12.0, 16.0),
        (88.0, 16.0),
        (12.0, 80.0),
        (88.0, 80.0),
        (12.0, 144.0),
        (88.0, 144.0),
    ] {
        circle_marker(&lat_lng(r, c, None), Some(&col_opts)).add_to(&m);
    }

    let plan_bounds = lat_lng_bounds(&lat_lng(0.0, 0.0, None), &lat_lng(100.0, 160.0, None));
    m.fit_bounds(&plan_bounds, None);

    let px = m.lat_lng_to_container_point(&lat_lng(50.0, 80.0, None));
    if let Some(meta) = dom::document().get_element_by_id("meta") {
        let text = format!(
            "CRS.Simple 100x160 plan · 4 studio rooms + atrium · center @ ({:.0}, {:.0}) px",
            px.x(),
            px.y()
        );
        meta.set_text_content(Some(&text));
    }
}
