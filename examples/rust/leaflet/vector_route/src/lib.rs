#![no_main]

use leaflet::*;

fn make_map_opts() -> MapOptions {
    let opts = MapOptions::create();
    opts.set_attribution_control(Some(false));
    opts.set_zoom_animation(Some(false));
    opts.set_fade_animation(Some(false));
    opts.set_marker_zoom_animation(Some(false));
    opts
}

fn poly_opts(color: &str, weight: f64, dash: Option<&str>, fill_opacity: f64) -> PolylineOptions {
    let opts = PolylineOptions::create();
    opts.set_smooth_factor(Some(1.0));
    opts.set_color(Some(color));
    opts.set_weight(Some(weight));
    if let Some(d) = dash {
        opts.set_dash_array(StringOrNumberArray::String(d.into()));
    }
    opts.set_fill_opacity(Some(fill_opacity));
    opts
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let m = map("map", Some(&make_map_opts()));

    let hubs = [
        ("London", lat_lng(51.505, -0.09, None)),
        ("Paris", lat_lng(48.8566, 2.3522, None)),
        ("Zurich", lat_lng(47.3769, 8.5417, None)),
        ("Munich", lat_lng(48.1351, 11.5820, None)),
        ("Prague", lat_lng(50.0755, 14.4378, None)),
        ("Berlin", lat_lng(52.5200, 13.4050, None)),
    ];

    let grat_opts = poly_opts("#e5e5e5", 1.0, None, 0.0);
    for lat in [46.0, 48.0, 50.0, 52.0, 54.0] {
        let pts = JsArray::from(vec![
            JsValue::from(&lat_lng(lat, -2.0, None)),
            JsValue::from(&lat_lng(lat, 17.0, None)),
        ]);
        polyline(pts, Some(&grat_opts)).add_to(&m);
    }
    for lng in [0.0, 4.0, 8.0, 12.0, 16.0] {
        let pts = JsArray::from(vec![
            JsValue::from(&lat_lng(45.0, lng, None)),
            JsValue::from(&lat_lng(55.0, lng, None)),
        ]);
        polyline(pts, Some(&grat_opts)).add_to(&m);
    }

    let primary_pts = JsArray::from(hubs.iter().map(|(_, p)| JsValue::from(p)).collect::<Vec<_>>());
    let primary_line = polyline(primary_pts, Some(&poly_opts("#111111", 3.0, None, 0.0)));
    primary_line.add_to(&m);

    let alt_pts = JsArray::from(vec![
        JsValue::from(&hubs[0].1),
        JsValue::from(&lat_lng(50.8503, 4.3517, None)),
        JsValue::from(&lat_lng(52.3676, 4.9041, None)),
        JsValue::from(&hubs[5].1),
    ]);
    polyline(alt_pts, Some(&poly_opts("#737373", 1.5, Some("5 5"), 0.0))).add_to(&m);

    let nodes = layer_group(None, None);
    let node_opts = CircleMarkerOptions::create();
    node_opts.set_radius(Some(6.0));
    node_opts.set_color(Some("#111111"));
    node_opts.set_weight(Some(2.0));
    node_opts.set_fill_color(Some("#ffffff"));
    node_opts.set_fill_opacity(Some(1.0));

    for (name, pt) in &hubs {
        let cm = circle_marker(pt, Some(&node_opts));
        cm.bind_tooltip(LayerBindTooltipContent::String((*name).into()), None);
        nodes.add_layer(&cm);
    }
    nodes.add_to(&m);

    m.fit_bounds(&primary_line.get_bounds(), None);

    let mut total_km = 0.0;
    for i in 0..hubs.len() - 1 {
        total_km += hubs[i].1.distance_to(&hubs[i + 1].1) / 1000.0;
    }

    if let Some(meta) = dom::document().get_element_by_id("meta") {
        let text = format!("6 hubs · 2 routes · {total_km:.0} km total · EPSG:3857 vector SVG");
        meta.set_text_content(Some(&text));
    }
}
