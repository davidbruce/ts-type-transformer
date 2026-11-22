import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node as dom_node
import gleam/float
import gleam/int
import gleam/list
import gleam/option.{type Option, None, Some}
import leaflet as leaflet
import leaflet/circle_marker_options
import leaflet/layer
import leaflet/map
import leaflet/map_options
import leaflet/path_options
import leaflet/point
import leaflet/polyline_options
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
  let opts =
    polyline_options.create()
    |> polyline_options.with_smooth_factor(1.0)
    |> path_options.with_color(color)
    |> path_options.with_weight(weight)
    |> path_options.with_fill_color(fill_color)
    |> path_options.with_fill_opacity(fill_opacity)
  case dash {
    Some(d) -> path_options.with_dash_array(opts, t.StringOrNumberArrayString(d))
    None -> opts
  }
}

pub fn main() {
  let m_opts =
    map_options.create()
    |> map_options.with_crs(crs.get_simple())
    |> map_options.with_min_zoom(-1.0)
    |> map_options.with_max_zoom(3.0)
    |> map_options.with_attribution_control(False)
    |> map_options.with_zoom_animation(False)
    |> map_options.with_fade_animation(False)
    |> map_options.with_marker_zoom_animation(False)
  let m = leaflet.map(t.ContentString("map"), Some(m_opts))

  let grid_opts = box_opts("#e5e5e5", 1.0, None, "#ffffff", 0.0)
  list.each([0.0, 20.0, 40.0, 60.0, 80.0, 100.0], fn(r) {
    leaflet.polyline(
      [
        runtime.dynamic(leaflet.lat_lng(r, 0.0, None)),
        runtime.dynamic(leaflet.lat_lng(r, 160.0, None)),
      ],
      Some(grid_opts),
    )
    |> layer.add_to(t.MapOrLayerGroupMap(m))
    Nil
  })
  list.each([0.0, 20.0, 40.0, 60.0, 80.0, 100.0, 120.0, 140.0, 160.0], fn(c) {
    leaflet.polyline(
      [
        runtime.dynamic(leaflet.lat_lng(0.0, c, None)),
        runtime.dynamic(leaflet.lat_lng(100.0, c, None)),
      ],
      Some(grid_opts),
    )
    |> layer.add_to(t.MapOrLayerGroupMap(m))
    Nil
  })

  let env =
    leaflet.lat_lng_bounds(
      t.LatLngExpressionLatLng(leaflet.lat_lng(8.0, 12.0, None)),
      t.LatLngExpressionLatLng(leaflet.lat_lng(92.0, 148.0, None)),
    )
  leaflet.rectangle(
    t.LatLngBoundsExpressionLatLngBounds(env),
    Some(box_opts("#111111", 2.5, None, "#ffffff", 0.9)),
  )
  |> layer.add_to(t.MapOrLayerGroupMap(m))

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
      leaflet.rectangle(
        t.LatLngBoundsExpressionLatLngBounds(b),
        Some(box_opts("#404040", 1.5, None, fill, 0.85)),
      )
      |> layer.add_to(t.MapOrLayerGroupMap(m))
      Nil
    },
  )

  let atrium = [
    t.PolylineSetLatLngsLatlngsLatLng(leaflet.lat_lng(50.0, 64.0, None)),
    t.PolylineSetLatLngsLatlngsLatLng(leaflet.lat_lng(74.0, 80.0, None)),
    t.PolylineSetLatLngsLatlngsLatLng(leaflet.lat_lng(50.0, 96.0, None)),
    t.PolylineSetLatLngsLatlngsLatLng(leaflet.lat_lng(26.0, 80.0, None)),
  ]
  leaflet.polygon(atrium, Some(box_opts("#111111", 2.0, None, "#555555", 0.35)))
  |> layer.add_to(t.MapOrLayerGroupMap(m))

  leaflet.polyline(
    [
      runtime.dynamic(leaflet.lat_lng(50.0, 12.0, None)),
      runtime.dynamic(leaflet.lat_lng(50.0, 148.0, None)),
    ],
    Some(box_opts("#737373", 1.5, Some("4 4"), "#ffffff", 0.0)),
  )
  |> layer.add_to(t.MapOrLayerGroupMap(m))

  let col_opts =
    circle_marker_options.create()
    |> circle_marker_options.with_radius(5.0)
    |> path_options.with_color("#111111")
    |> path_options.with_weight(1.5)
    |> path_options.with_fill_color("#111111")
    |> path_options.with_fill_opacity(1.0)

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
      leaflet.circle_marker(t.LatLngExpressionLatLng(leaflet.lat_lng(r, c, None)), Some(col_opts))
      |> layer.add_to(t.MapOrLayerGroupMap(m))
      Nil
    },
  )

  let plan_bounds =
    leaflet.lat_lng_bounds(
      t.LatLngExpressionLatLng(leaflet.lat_lng(0.0, 0.0, None)),
      t.LatLngExpressionLatLng(leaflet.lat_lng(100.0, 160.0, None)),
    )
  map.fit_bounds(m, t.LatLngBoundsExpressionLatLngBounds(plan_bounds), None)

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
