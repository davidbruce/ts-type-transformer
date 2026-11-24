import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node as dom_node
import gleam/list
import gleam/option.{None, Some}
import leaflet
import leaflet/circle
import leaflet/circle_marker_options
import leaflet/lat_lng
import leaflet/layer
import leaflet/map
import leaflet/map_options
import leaflet/path_options
import leaflet/runtime
import leaflet/types as t

pub fn main() {
  let m_opts =
    map_options.create()
    |> map_options.with_attribution_control(False)
    |> map_options.with_zoom_animation(False)
    |> map_options.with_fade_animation(False)
    |> map_options.with_marker_zoom_animation(False)
  let m = leaflet.map_with(t.StringOrHTMLElementString("map"), Some(m_opts))

  let zurich = leaflet.lat_lng(47.3769, 8.5417)
  map.set_view_with(m, t.LatLngExpressionLatLng(zurich), Some(8.0), None)
  let outer_opts =
    circle_marker_options.create()
    |> path_options.with_color("#404040")
    |> path_options.with_weight(1.5)
    |> path_options.with_fill_color("#d4d4d4")
    |> path_options.with_fill_opacity(0.25)
  let outer_circle =
    leaflet.circle_with_radius_with(
      t.LatLngExpressionLatLng(zurich),
      130_000.0,
      Some(outer_opts),
    )
    |> layer.add_to(t.MapOrLayerGroupMap(m))

  list.each(
    [
      #(90_000.0, "#404040", "#a3a3a3", 0.3),
      #(55_000.0, "#111111", "#737373", 0.4),
      #(25_000.0, "#111111", "#262626", 0.55),
    ],
    fn(spec) {
      let #(r, stroke, fill, op) = spec
      let opts =
        circle_marker_options.create()
        |> path_options.with_color(stroke)
        |> path_options.with_weight(1.5)
        |> path_options.with_fill_color(fill)
        |> path_options.with_fill_opacity(op)
      leaflet.circle_with_radius_with(
        t.LatLngExpressionLatLng(zurich),
        r,
        Some(opts),
      )
      |> layer.add_to(t.MapOrLayerGroupMap(m))
      Nil
    },
  )

  let beacons = [
    leaflet.lat_lng(47.5596, 7.5886),
    leaflet.lat_lng(46.948, 7.4474),
    leaflet.lat_lng(47.0502, 8.3093),
    leaflet.lat_lng(47.4245, 9.3767),
    leaflet.lat_lng(46.8508, 9.532),
  ]

  let total_dist_km =
    list.fold(beacons, 0.0, fn(acc, pt) {
      let d_km =
        lat_lng.distance_to(zurich, t.LatLngExpressionLatLng(pt)) /. 1000.0

      let b_opts =
        circle_marker_options.create()
        |> path_options.with_color("#555555")
        |> path_options.with_weight(1.2)
        |> path_options.with_dash_array(t.StringOrNumberArrayString("3 3"))
        |> path_options.with_fill_color("#737373")
        |> path_options.with_fill_opacity(0.18)
      leaflet.circle_with_radius_with(
        t.LatLngExpressionLatLng(pt),
        18_000.0,
        Some(b_opts),
      )
      |> layer.add_to(t.MapOrLayerGroupMap(m))

      let d_opts =
        circle_marker_options.create()
        |> circle_marker_options.with_radius(4.5)
        |> path_options.with_color("#111111")
        |> path_options.with_weight(1.5)
        |> path_options.with_fill_color("#111111")
        |> path_options.with_fill_opacity(1.0)
      leaflet.circle_marker_with(t.LatLngExpressionLatLng(pt), Some(d_opts))
      |> layer.add_to(t.MapOrLayerGroupMap(m))

      acc +. d_km
    })

  map.fit_bounds(
    m,
    t.LatLngBoundsExpressionLatLngBounds(circle.get_bounds(outer_circle)),
  )

  let avg_str = runtime.to_fixed(total_dist_km /. 5.0, 1)

  case document.get_element_by_id(core.get_document(), "meta") {
    Some(meta) ->
      dom_node.set_text_content(
        meta,
        Some(
          "4 concentric rings (25–130 km) · 5 regional beacons (avg "
          <> avg_str
          <> " km)",
        ),
      )
    None -> Nil
  }
}
