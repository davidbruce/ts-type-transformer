import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node as dom_node
import gleam/float
import gleam/int
import gleam/list
import gleam/option.{type Option, None, Some}
import leaflet/areas/main as leaflet
import leaflet/areas/main/circle_marker_options
import leaflet/areas/main/lat_lng
import leaflet/areas/main/layer
import leaflet/areas/main/layer_group
import leaflet/areas/main/map
import leaflet/areas/main/map_options
import leaflet/areas/main/path_options
import leaflet/areas/main/polyline
import leaflet/areas/main/polyline_options
import leaflet/runtime
import leaflet/types as t

fn make_map_opts() -> t.MapOptions(Nil) {
  let opts = map_options.create()
  map_options.set_attribution_control(opts, Some(False))
  map_options.set_zoom_animation(opts, Some(False))
  map_options.set_fade_animation(opts, Some(False))
  map_options.set_marker_zoom_animation(opts, Some(False))
  opts
}

fn poly_opts(
  color: String,
  weight: Float,
  dash: Option(String),
  fill_opacity: Float,
) -> t.PolylineOptions(Nil) {
  let opts = polyline_options.create()
  polyline_options.set_smooth_factor(opts, Some(1.0))
  path_options.set_color(opts, Some(color))
  path_options.set_weight(opts, Some(weight))
  case dash {
    Some(d) -> path_options.set_dash_array(opts, Some(t.StringOrNumberArrayString(d)))
    None -> Nil
  }
  path_options.set_fill_opacity(opts, Some(fill_opacity))
  opts
}

fn pair_distances_km(pts: List(t.LatLng(Nil))) -> Float {
  case pts {
    [a, b, ..rest] ->
      lat_lng.distance_to(a, t.LatLngExpressionLatLng(b)) /. 1000.0
      +. pair_distances_km([b, ..rest])
    _ -> 0.0
  }
}

pub fn main() {
  let m = leaflet.map(t.ContentString("map"), Some(make_map_opts()))
  let london = leaflet.lat_lng(51.505, -0.09, None)
  let paris = leaflet.lat_lng(48.8566, 2.3522, None)
  let zurich = leaflet.lat_lng(47.3769, 8.5417, None)
  let munich = leaflet.lat_lng(48.1351, 11.5820, None)
  let prague = leaflet.lat_lng(50.0755, 14.4378, None)
  let berlin = leaflet.lat_lng(52.5200, 13.4050, None)

  let hubs = [
    #("London", london),
    #("Paris", paris),
    #("Zurich", zurich),
    #("Munich", munich),
    #("Prague", prague),
    #("Berlin", berlin),
  ]

  let grat_opts = poly_opts("#e5e5e5", 1.0, None, 0.0)
  list.each([46.0, 48.0, 50.0, 52.0, 54.0], fn(lat) {
    let line =
      leaflet.polyline(
        [
          runtime.dynamic(leaflet.lat_lng(lat, -2.0, None)),
          runtime.dynamic(leaflet.lat_lng(lat, 17.0, None)),
        ],
        Some(grat_opts),
      )
    let _ = layer.add_to(line, t.MapOrLayerGroupMap(m))
    Nil
  })
  list.each([0.0, 4.0, 8.0, 12.0, 16.0], fn(lng) {
    let line =
      leaflet.polyline(
        [
          runtime.dynamic(leaflet.lat_lng(45.0, lng, None)),
          runtime.dynamic(leaflet.lat_lng(55.0, lng, None)),
        ],
        Some(grat_opts),
      )
    let _ = layer.add_to(line, t.MapOrLayerGroupMap(m))
    Nil
  })

  let primary_pts = list.map(hubs, fn(h) { runtime.dynamic(h.1) })
  let primary_line =
    leaflet.polyline(primary_pts, Some(poly_opts("#111111", 3.0, None, 0.0)))
  let _ = layer.add_to(primary_line, t.MapOrLayerGroupMap(m))

  let alt_line =
    leaflet.polyline(
      [
        runtime.dynamic(london),
        runtime.dynamic(leaflet.lat_lng(50.8503, 4.3517, None)),
        runtime.dynamic(leaflet.lat_lng(52.3676, 4.9041, None)),
        runtime.dynamic(berlin),
      ],
      Some(poly_opts("#737373", 1.5, Some("5 5"), 0.0)),
    )
  let _ = layer.add_to(alt_line, t.MapOrLayerGroupMap(m))

  let nodes = leaflet.layer_group(None, None)
  let node_opts = circle_marker_options.create()
  circle_marker_options.set_radius(node_opts, Some(6.0))
  path_options.set_color(node_opts, Some("#111111"))
  path_options.set_weight(node_opts, Some(2.0))
  path_options.set_fill_color(node_opts, Some("#ffffff"))
  path_options.set_fill_opacity(node_opts, Some(1.0))

  list.each(hubs, fn(h) {
    let cm = leaflet.circle_marker(t.LatLngExpressionLatLng(h.1), Some(node_opts))
    let _ = layer.bind_tooltip(cm, t.LayerBindTooltipContentString(h.0), None)
    let _ = layer_group.add_layer(nodes, cm)
    Nil
  })
  let _ = layer.add_to(nodes, t.MapOrLayerGroupMap(m))

  let _ =
    map.fit_bounds(
      m,
      t.LatLngBoundsExpressionLatLngBounds(polyline.get_bounds(primary_line)),
      None,
    )

  let total_km = pair_distances_km(list.map(hubs, fn(h) { h.1 }))
  let km_str = int.to_string(float.round(total_km))
  case document.get_element_by_id(core.get_document(), "meta") {
    Some(meta) ->
      dom_node.set_text_content(
        meta,
        Some("6 hubs · 2 routes · " <> km_str <> " km total · EPSG:3857 vector SVG"),
      )
    None -> Nil
  }
}
