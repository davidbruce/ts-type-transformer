import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node as dom_node
import gleam/int
import gleam/list
import gleam/option.{None, Some}
import leaflet
import leaflet/circle_marker_options
import leaflet/div_icon_options
import leaflet/div_overlay
import leaflet/evented
import leaflet/lat_lng
import leaflet/lat_lng_bounds
import leaflet/layer
import leaflet/leaflet_mouse_event
import leaflet/map
import leaflet/map_options
import leaflet/marker_options
import leaflet/path_options
import leaflet/polyline
import leaflet/polyline_options
import leaflet/popup
import leaflet/popup_options
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

  let bern = leaflet.lat_lng(46.948, 7.4474)
  let stations = [
    #("01", "Geneva", leaflet.lat_lng(46.2044, 6.1432)),
    #("02", "Lausanne", leaflet.lat_lng(46.5197, 6.6323)),
    #("03", "Bern", bern),
    #("04", "Zurich", leaflet.lat_lng(47.3769, 8.5417)),
    #("05", "Basel", leaflet.lat_lng(47.5596, 7.5886)),
  ]

  let line_pts = list.map(stations, fn(s) { runtime.dynamic(s.2) })
  let p_opts =
    polyline_options.create()
    |> polyline_options.with_smooth_factor(1.0)
    |> path_options.with_color("#404040")
    |> path_options.with_weight(2.5)
  let corridor =
    leaflet.polyline_with(line_pts, Some(p_opts))
    |> layer.add_to(t.MapOrLayerGroupMap(m))

  let dot_opts =
    circle_marker_options.create()
    |> circle_marker_options.with_radius(6.0)
    |> path_options.with_color("#111111")
    |> path_options.with_weight(2.0)
    |> path_options.with_fill_color("#ffffff")
    |> path_options.with_fill_opacity(1.0)

  let pin_size = t.PointExpressionPoint(leaflet.point(22.0, 22.0))
  let pin_anchor = t.PointExpressionPoint(leaflet.point(11.0, 11.0))

  list.each(stations, fn(s) {
    let #(code, name, pt) = s
    leaflet.circle_marker_with(t.LatLngExpressionLatLng(pt), Some(dot_opts))
    |> layer.bind_tooltip_with(
      t.LayerBindTooltipContentString(code <> " · " <> name),
      None,
    )
    |> layer.add_to(t.MapOrLayerGroupMap(m))

    let d_opts =
      div_icon_options.create()
      |> div_icon_options.with_class_name("mono-pin")
      |> div_icon_options.with_html(t.StringOrElementOrBooleanString(
        "<span>" <> code <> "</span>",
      ))
      |> div_icon_options.with_icon_size(pin_size)
      |> div_icon_options.with_icon_anchor(pin_anchor)

    let mk_opts =
      marker_options.create()
      |> marker_options.with_icon(
        t.IconOrDivIconDivIcon(leaflet.div_icon_with(Some(d_opts))),
      )
    leaflet.marker_with(t.LatLngExpressionLatLng(pt), Some(mk_opts))
    |> layer.add_to(t.MapOrLayerGroupMap(m))
    Nil
  })

  map.fit_bounds(
    m,
    t.LatLngBoundsExpressionLatLngBounds(lat_lng_bounds.pad(
      polyline.get_bounds(corridor),
      0.15,
    )),
  )

  let pop_opts =
    popup_options.create()
    |> popup_options.with_max_width(220.0)
  let note = leaflet.popup_with(t.LatLngExpressionLatLng(bern), Some(pop_opts))
  div_overlay.set_content(
    note,
    t.DivOverlayResultOrStringOrHTMLElementString("03 Bern · 46.95N, 7.45E"),
  )
  popup.open_on(note, m)

  let clicks = runtime.ref_new(0)
  evented.on_click(m, fn(ev) {
    let ll = leaflet_mouse_event.get_latlng(ev)
    let lat = lat_lng.get_lat(ll)
    let lng = lat_lng.get_lng(ll)
    case lat >. 0.0 {
      True -> {
        let n = runtime.ref_update(clicks, fn(c) { c + 1 })
        let coords =
          runtime.to_fixed(lat, 2) <> "N, " <> runtime.to_fixed(lng, 2) <> "E"
        case n > 2 {
          True -> {
            div_overlay.set_lat_lng(note, t.LatLngExpressionLatLng(ll))
            div_overlay.set_content(
              note,
              t.DivOverlayResultOrStringOrHTMLElementString(
                "Click #" <> int.to_string(n) <> " · " <> coords,
              ),
            )
            popup.open_on(note, m)
            Nil
          }
          False -> Nil
        }
        case document.get_element_by_id(core.get_document(), "meta") {
          Some(meta) ->
            dom_node.set_text_content(
              meta,
              Some(
                "5 DivIcon stations · clicks="
                <> int.to_string(n)
                <> " · last click ("
                <> coords
                <> ")",
              ),
            )
          None -> Nil
        }
      }
      False -> Nil
    }
  })

  let payload = leaflet_mouse_event.create()
  leaflet_mouse_event.set_latlng(payload, bern)
  evented.fire_with(m, "click", Some(runtime.dynamic(payload)), None)
  evented.fire_with(m, "click", Some(runtime.dynamic(payload)), None)
  Nil
}
