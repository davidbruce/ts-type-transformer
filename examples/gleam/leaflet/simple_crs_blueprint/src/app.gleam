import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node as dom_node
import gleam/float
import gleam/int
import gleam/list
import gleam/option.{type Option, None, Some}
import leaflet/areas/main as leaflet
import leaflet/areas/main/circle_marker_options
import leaflet/areas/main/layer
import leaflet/areas/main/map
import leaflet/areas/main/map_options
import leaflet/areas/main/path_options
import leaflet/areas/main/point
import leaflet/areas/main/polyline_options
import leaflet/namespaces/crs
import leaflet/runtime
import leaflet/types as t

fn box_opts(
  color: String,
  weight: Float,
  dash: Option(String),
  fill_color: String,
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
  path_options.set_fill_color(opts, Some(fill_color))
  path_options.set_fill_opacity(opts, Some(fill_opacity))
  opts
}

pub fn main() {
  let m_opts = map_options.create()
  map_options.set_crs(m_opts, Some(crs.get_simple()))
  map_options.set_min_zoom(m_opts, Some(-1.0))
  map_options.set_max_zoom(m_opts, Some(3.0))
  map_options.set_attribution_control(m_opts, Some(False))
  map_options.set_zoom_animation(m_opts, Some(False))
  map_options.set_fade_animation(m_opts, Some(False))
  map_options.set_marker_zoom_animation(m_opts, Some(False))
  let m = leaflet.map(t.ContentString("map"), Some(m_opts))

  let grid_opts = box_opts("#e5e5e5", 1.0, None, "#ffffff", 0.0)
  list.each([0.0, 20.0, 40.0, 60.0, 80.0, 100.0], fn(r) {
    let line =
      leaflet.polyline(
        [
          runtime.dynamic(leaflet.lat_lng(r, 0.0, None)),
          runtime.dynamic(leaflet.lat_lng(r, 160.0, None)),
        ],
        Some(grid_opts),
      )
    let _ = layer.add_to(line, t.MapOrLayerGroupMap(m))
    Nil
  })
  list.each([0.0, 20.0, 40.0, 60.0, 80.0, 100.0, 120.0, 140.0, 160.0], fn(c) {
    let line =
      leaflet.polyline(
        [
          runtime.dynamic(leaflet.lat_lng(0.0, c, None)),
          runtime.dynamic(leaflet.lat_lng(100.0, c, None)),
        ],
        Some(grid_opts),
      )
    let _ = layer.add_to(line, t.MapOrLayerGroupMap(m))
    Nil
  })

  let env =
    leaflet.lat_lng_bounds(
      t.LatLngExpressionLatLng(leaflet.lat_lng(8.0, 12.0, None)),
      t.LatLngExpressionLatLng(leaflet.lat_lng(92.0, 148.0, None)),
    )
  let _ =
    layer.add_to(
      leaflet.rectangle(
        t.LatLngBoundsExpressionLatLngBounds(env),
        Some(box_opts("#111111", 2.5, None, "#ffffff", 0.9)),
      ),
      t.MapOrLayerGroupMap(m),
    )

  list.each(
    [
      #(12.0, 16.0, 46.0, 62.0, "#f7f7f7"),
      #(54.0, 16.0, 88.0, 62.0, "#efefef"),
      #(12.0, 98.0, 46.0, 144.0, "#e5e5e5"),
      #(54.0, 98.0, 88.0, 144.0, "#d4d4d4"),
    ],
    fn(room) {
      let #(r0, c0, r1, c1, fill) = room
      let b =
        leaflet.lat_lng_bounds(
          t.LatLngExpressionLatLng(leaflet.lat_lng(r0, c0, None)),
          t.LatLngExpressionLatLng(leaflet.lat_lng(r1, c1, None)),
        )
      let _ =
        layer.add_to(
          leaflet.rectangle(
            t.LatLngBoundsExpressionLatLngBounds(b),
            Some(box_opts("#404040", 1.5, None, fill, 0.85)),
          ),
          t.MapOrLayerGroupMap(m),
        )
      Nil
    },
  )

  let atrium = [
    t.PolylineSetLatLngsLatlngsLatLng(leaflet.lat_lng(50.0, 64.0, None)),
    t.PolylineSetLatLngsLatlngsLatLng(leaflet.lat_lng(74.0, 80.0, None)),
    t.PolylineSetLatLngsLatlngsLatLng(leaflet.lat_lng(50.0, 96.0, None)),
    t.PolylineSetLatLngsLatlngsLatLng(leaflet.lat_lng(26.0, 80.0, None)),
  ]
  let _ =
    layer.add_to(
      leaflet.polygon(atrium, Some(box_opts("#111111", 2.0, None, "#555555", 0.35))),
      t.MapOrLayerGroupMap(m),
    )

  let spine =
    leaflet.polyline(
      [
        runtime.dynamic(leaflet.lat_lng(50.0, 12.0, None)),
        runtime.dynamic(leaflet.lat_lng(50.0, 148.0, None)),
      ],
      Some(box_opts("#737373", 1.5, Some("4 4"), "#ffffff", 0.0)),
    )
  let _ = layer.add_to(spine, t.MapOrLayerGroupMap(m))

  let col_opts = circle_marker_options.create()
  circle_marker_options.set_radius(col_opts, Some(5.0))
  path_options.set_color(col_opts, Some("#111111"))
  path_options.set_weight(col_opts, Some(1.5))
  path_options.set_fill_color(col_opts, Some("#111111"))
  path_options.set_fill_opacity(col_opts, Some(1.0))

  list.each(
    [
      #(12.0, 16.0),
      #(88.0, 16.0),
      #(12.0, 80.0),
      #(88.0, 80.0),
      #(12.0, 144.0),
      #(88.0, 144.0),
    ],
    fn(col) {
      let #(r, c) = col
      let _ =
        layer.add_to(
          leaflet.circle_marker(t.LatLngExpressionLatLng(leaflet.lat_lng(r, c, None)), Some(col_opts)),
          t.MapOrLayerGroupMap(m),
        )
      Nil
    },
  )

  let plan_bounds =
    leaflet.lat_lng_bounds(
      t.LatLngExpressionLatLng(leaflet.lat_lng(0.0, 0.0, None)),
      t.LatLngExpressionLatLng(leaflet.lat_lng(100.0, 160.0, None)),
    )
  let _ = map.fit_bounds(m, t.LatLngBoundsExpressionLatLngBounds(plan_bounds), None)

  let px = map.lat_lng_to_container_point(m, t.LatLngExpressionLatLng(leaflet.lat_lng(50.0, 80.0, None)))
  let px_x = int.to_string(float.round(point.get_x(px)))
  let px_y = int.to_string(float.round(point.get_y(px)))

  case document.get_element_by_id(core.get_document(), "meta") {
    Some(meta) ->
      dom_node.set_text_content(
        meta,
        Some(
          "CRS.Simple 100x160 plan · 4 studio rooms + atrium · center @ ("
          <> px_x
          <> ", "
          <> px_y
          <> ") px",
        ),
      )
    None -> Nil
  }
}
