import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node as dom_node
import gleam/float
import gleam/int
import gleam/list
import gleam/option.{None, Some}
import leaflet/areas/main as leaflet
import leaflet/areas/main/circle
import leaflet/areas/main/circle_marker_options
import leaflet/areas/main/lat_lng
import leaflet/areas/main/layer
import leaflet/areas/main/map
import leaflet/areas/main/map_options
import leaflet/areas/main/path_options
import leaflet/types as t

pub fn main() {
  let m_opts = map_options.create()
  map_options.set_attribution_control(m_opts, Some(False))
  map_options.set_zoom_animation(m_opts, Some(False))
  map_options.set_fade_animation(m_opts, Some(False))
  map_options.set_marker_zoom_animation(m_opts, Some(False))
  let m = leaflet.map(t.ContentString("map"), Some(m_opts))

  let zurich = leaflet.lat_lng(47.3769, 8.5417, None)
  let _ = map.set_view(m, t.LatLngExpressionLatLng(zurich), Some(8.0), None)
  let outer_opts = circle_marker_options.create()
  path_options.set_color(outer_opts, Some("#404040"))
  path_options.set_weight(outer_opts, Some(1.5))
  path_options.set_fill_color(outer_opts, Some("#d4d4d4"))
  path_options.set_fill_opacity(outer_opts, Some(0.25))
  let outer_circle =
    leaflet.circle_with_radius(t.LatLngExpressionLatLng(zurich), 130_000.0, Some(outer_opts))
  let _ = layer.add_to(outer_circle, t.MapOrLayerGroupMap(m))

  list.each(
    [
      #(90_000.0, "#404040", "#a3a3a3", 0.30),
      #(55_000.0, "#111111", "#737373", 0.40),
      #(25_000.0, "#111111", "#262626", 0.55),
    ],
    fn(spec) {
      let #(r, stroke, fill, op) = spec
      let opts = circle_marker_options.create()
      path_options.set_color(opts, Some(stroke))
      path_options.set_weight(opts, Some(1.5))
      path_options.set_fill_color(opts, Some(fill))
      path_options.set_fill_opacity(opts, Some(op))
      let _ =
        layer.add_to(
          leaflet.circle_with_radius(t.LatLngExpressionLatLng(zurich), r, Some(opts)),
          t.MapOrLayerGroupMap(m),
        )
      Nil
    },
  )

  let beacons = [
    leaflet.lat_lng(47.5596, 7.5886, None),
    leaflet.lat_lng(46.9480, 7.4474, None),
    leaflet.lat_lng(47.0502, 8.3093, None),
    leaflet.lat_lng(47.4245, 9.3767, None),
    leaflet.lat_lng(46.8508, 9.5320, None),
  ]

  let total_dist_km =
    list.fold(beacons, 0.0, fn(acc, pt) {
      let d_km = lat_lng.distance_to(zurich, t.LatLngExpressionLatLng(pt)) /. 1000.0

      let b_opts = circle_marker_options.create()
      path_options.set_color(b_opts, Some("#555555"))
      path_options.set_weight(b_opts, Some(1.2))
      path_options.set_dash_array(b_opts, Some(t.StringOrNumberArrayString("3 3")))
      path_options.set_fill_color(b_opts, Some("#737373"))
      path_options.set_fill_opacity(b_opts, Some(0.18))
      let _ =
        layer.add_to(
          leaflet.circle_with_radius(t.LatLngExpressionLatLng(pt), 18_000.0, Some(b_opts)),
          t.MapOrLayerGroupMap(m),
        )

      let d_opts = circle_marker_options.create()
      circle_marker_options.set_radius(d_opts, Some(4.5))
      path_options.set_color(d_opts, Some("#111111"))
      path_options.set_weight(d_opts, Some(1.5))
      path_options.set_fill_color(d_opts, Some("#111111"))
      path_options.set_fill_opacity(d_opts, Some(1.0))
      let _ =
        layer.add_to(
          leaflet.circle_marker(t.LatLngExpressionLatLng(pt), Some(d_opts)),
          t.MapOrLayerGroupMap(m),
        )

      acc +. d_km
    })

  let _ =
    map.fit_bounds(
      m,
      t.LatLngBoundsExpressionLatLngBounds(circle.get_bounds(outer_circle)),
      None,
    )

  let avg_tenths = float.round(total_dist_km /. 5.0 *. 10.0)
  let avg_str =
    int.to_string(avg_tenths / 10) <> "." <> int.to_string(avg_tenths % 10)

  case document.get_element_by_id(core.get_document(), "meta") {
    Some(meta) ->
      dom_node.set_text_content(
        meta,
        Some("4 concentric rings (25–130 km) · 5 regional beacons (avg " <> avg_str <> " km)"),
      )
    None -> Nil
  }
}
