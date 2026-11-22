import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node as dom_node
import gleam/int
import gleam/list
import gleam/option.{type Option, None, Some}
import leaflet as leaflet
import leaflet/circle_marker_options
import leaflet/lat_lng_bounds
import leaflet/layer
import leaflet/map
import leaflet/map_options
import leaflet/path_options
import leaflet/polyline
import leaflet/polyline_options
import leaflet/types as t

fn poly_opts(
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
    |> map_options.with_attribution_control(False)
    |> map_options.with_zoom_animation(False)
    |> map_options.with_fade_animation(False)
    |> map_options.with_marker_zoom_animation(False)
  let m = leaflet.map(t.StringOrHTMLElementString("map"), Some(m_opts))

  let mid_pts = [
    t.PolylineSetLatLngsLatlngsLatLng(leaflet.lat_lng(47.2, 7.6, None)),
    t.PolylineSetLatLngsLatlngsLatLng(leaflet.lat_lng(48.0, 8.6, None)),
    t.PolylineSetLatLngsLatlngsLatLng(leaflet.lat_lng(47.5, 9.7, None)),
    t.PolylineSetLatLngsLatlngsLatLng(leaflet.lat_lng(46.8, 9.5, None)),
    t.PolylineSetLatLngsLatlngsLatLng(leaflet.lat_lng(46.6, 8.4, None)),
    t.PolylineSetLatLngsLatlngsLatLng(leaflet.lat_lng(46.9, 7.3, None)),
  ]
  let core_pts = [
    t.PolylineSetLatLngsLatlngsLatLng(leaflet.lat_lng(47.35, 8.2, None)),
    t.PolylineSetLatLngsLatlngsLatLng(leaflet.lat_lng(47.55, 8.75, None)),
    t.PolylineSetLatLngsLatlngsLatLng(leaflet.lat_lng(47.15, 9.0, None)),
    t.PolylineSetLatLngsLatlngsLatLng(leaflet.lat_lng(46.95, 8.4, None)),
  ]

  let mid_poly =
    leaflet.polygon(mid_pts, Some(poly_opts("#404040", 2.0, None, "#d4d4d4", 0.55)))
  let core_poly =
    leaflet.polygon(core_pts, Some(poly_opts("#111111", 2.5, None, "#404040", 0.75)))

  let mid_bounds = polyline.get_bounds(mid_poly)
  let core_bounds = polyline.get_bounds(core_poly)
  let padded = lat_lng_bounds.pad(mid_bounds, 0.18)
  let hull_rect =
    leaflet.rectangle(
      t.LatLngBoundsExpressionLatLngBounds(padded),
      Some(poly_opts("#737373", 1.5, Some("4 4"), "#efefef", 0.05)),
    )

  layer.add_to(hull_rect, t.MapOrLayerGroupMap(m))
  layer.add_to(mid_poly, t.MapOrLayerGroupMap(m))
  layer.add_to(core_poly, t.MapOrLayerGroupMap(m))

  let probes = [
    leaflet.lat_lng(47.25, 8.6, None),
    leaflet.lat_lng(47.15, 8.55, None),
    leaflet.lat_lng(47.7, 8.6, None),
    leaflet.lat_lng(47.0, 9.2, None),
    leaflet.lat_lng(46.8, 7.9, None),
    leaflet.lat_lng(47.85, 7.5, None),
    leaflet.lat_lng(47.9, 9.7, None),
    leaflet.lat_lng(46.55, 9.6, None),
  ]

  let #(in_core, in_mid, exterior) =
    list.fold(probes, #(0, 0, 0), fn(acc, pt) {
      let #(c_cnt, m_cnt, e_cnt) = acc
      let #(fill_hex, next_acc) = case
        lat_lng_bounds.contains(core_bounds, t.LatLngBoundsExtendLatlngOrBoundsLatLng(pt))
      {
        True -> #("#111111", #(c_cnt + 1, m_cnt, e_cnt))
        False ->
          case
            lat_lng_bounds.contains(mid_bounds, t.LatLngBoundsExtendLatlngOrBoundsLatLng(pt))
          {
            True -> #("#737373", #(c_cnt, m_cnt + 1, e_cnt))
            False -> #("#ffffff", #(c_cnt, m_cnt, e_cnt + 1))
          }
      }
      let cm_opts =
        circle_marker_options.create()
        |> circle_marker_options.with_radius(6.0)
        |> path_options.with_color("#111111")
        |> path_options.with_weight(2.0)
        |> path_options.with_fill_color(fill_hex)
        |> path_options.with_fill_opacity(1.0)
      leaflet.circle_marker(t.LatLngExpressionLatLng(pt), Some(cm_opts))
      |> layer.add_to(t.MapOrLayerGroupMap(m))
      next_acc
    })

  map.fit_bounds(m, t.LatLngBoundsExpressionLatLngBounds(padded), None)

  case document.get_element_by_id(core.get_document(), "meta") {
    Some(meta) ->
      dom_node.set_text_content(
        meta,
        Some(
          "2 nested polygons · 1 padded hull · probes: "
          <> int.to_string(in_core)
          <> " core · "
          <> int.to_string(in_mid)
          <> " sector · "
          <> int.to_string(exterior)
          <> " exterior",
        ),
      )
    None -> Nil
  }
}
