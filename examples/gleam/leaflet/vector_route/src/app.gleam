import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node as dom_node
import gleam/float
import gleam/int
import gleam/list
import gleam/option.{type Option, None, Some}
import leaflet
import leaflet/circle_marker_options
import leaflet/lat_lng
import leaflet/layer
import leaflet/layer_group
import leaflet/map
import leaflet/map_options
import leaflet/path_options
import leaflet/polyline
import leaflet/polyline_options
import leaflet/runtime
import leaflet/types as t

fn make_map_opts() -> t.MapOptions {
  map_options.create()
  |> map_options.with_attribution_control(False)
  |> map_options.with_zoom_animation(False)
  |> map_options.with_fade_animation(False)
  |> map_options.with_marker_zoom_animation(False)
}

fn poly_opts(
  color: String,
  weight: Float,
  dash: Option(String),
  fill_opacity: Float,
) -> t.PolylineOptions {
  let opts =
    polyline_options.create()
    |> polyline_options.with_smooth_factor(1.0)
    |> path_options.with_color(color)
    |> path_options.with_weight(weight)
    |> path_options.with_fill_opacity(fill_opacity)
  case dash {
    Some(d) -> path_options.with_dash_array(opts, d)
    None -> opts
  }
}

fn pair_distances_km(pts: List(t.LatLng)) -> Float {
  case pts {
    [a, b, ..rest] ->
      lat_lng.distance_to(a, b) /. 1000.0 +. pair_distances_km([b, ..rest])
    _ -> 0.0
  }
}

pub fn main() {
  let m = leaflet.map_with("map", make_map_opts())
  let london = leaflet.lat_lng(51.505, -0.09)
  let paris = leaflet.lat_lng(48.8566, 2.3522)
  let zurich = leaflet.lat_lng(47.3769, 8.5417)
  let munich = leaflet.lat_lng(48.1351, 11.582)
  let prague = leaflet.lat_lng(50.0755, 14.4378)
  let berlin = leaflet.lat_lng(52.52, 13.405)

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
    leaflet.polyline_with(
      [
        runtime.dynamic(leaflet.lat_lng(lat, -2.0)),
        runtime.dynamic(leaflet.lat_lng(lat, 17.0)),
      ],
      grat_opts,
    )
    |> layer.add_to_map(m)
    Nil
  })
  list.each([0.0, 4.0, 8.0, 12.0, 16.0], fn(lng) {
    leaflet.polyline_with(
      [
        runtime.dynamic(leaflet.lat_lng(45.0, lng)),
        runtime.dynamic(leaflet.lat_lng(55.0, lng)),
      ],
      grat_opts,
    )
    |> layer.add_to_map(m)
    Nil
  })

  let primary_pts = list.map(hubs, fn(h) { runtime.dynamic(h.1) })
  let primary_line =
    leaflet.polyline_with(primary_pts, poly_opts("#111111", 3.0, None, 0.0))
    |> layer.add_to_map(m)

  leaflet.polyline_with(
    [
      runtime.dynamic(london),
      runtime.dynamic(leaflet.lat_lng(50.8503, 4.3517)),
      runtime.dynamic(leaflet.lat_lng(52.3676, 4.9041)),
      runtime.dynamic(berlin),
    ],
    poly_opts("#737373", 1.5, Some("5 5"), 0.0),
  )
  |> layer.add_to_map(m)

  let nodes = leaflet.layer_group()
  let node_opts =
    circle_marker_options.create()
    |> circle_marker_options.with_radius(6.0)
    |> path_options.with_color("#111111")
    |> path_options.with_weight(2.0)
    |> path_options.with_fill_color("#ffffff")
    |> path_options.with_fill_opacity(1.0)

  list.each(hubs, fn(h) {
    let cm =
      leaflet.circle_marker_with(h.1, node_opts)
      |> layer.bind_tooltip(h.0)
    layer_group.add_layer(nodes, cm)
    Nil
  })
  layer.add_to_map(nodes, m)

  map.fit_bounds_lat_lng_bounds(m, polyline.get_bounds(primary_line))

  let total_km = pair_distances_km(list.map(hubs, fn(h) { h.1 }))
  let km_str = int.to_string(float.round(total_km))
  case document.get_element_by_id(core.get_document(), "meta") {
    Some(meta) ->
      dom_node.set_text_content(
        meta,
        Some(
          "6 hubs · 2 routes · " <> km_str <> " km total · EPSG:3857 vector SVG",
        ),
      )
    None -> Nil
  }
}
