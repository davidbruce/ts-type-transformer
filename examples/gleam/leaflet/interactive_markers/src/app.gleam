import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node as dom_node
import gleam/list
import gleam/option.{None, Some}
import leaflet/areas/main as leaflet
import leaflet/areas/main/circle_marker_options
import leaflet/areas/main/div_icon_options
import leaflet/areas/main/div_overlay
import leaflet/areas/main/evented
import leaflet/areas/main/lat_lng
import leaflet/areas/main/lat_lng_bounds
import leaflet/areas/main/layer
import leaflet/areas/main/leaflet_mouse_event
import leaflet/areas/main/map
import leaflet/areas/main/map_options
import leaflet/areas/main/marker_options
import leaflet/areas/main/path_options
import leaflet/areas/main/polyline
import leaflet/areas/main/polyline_options
import leaflet/areas/main/popup
import leaflet/areas/main/popup_options
import leaflet/runtime
import leaflet/types as t

pub fn main() {
  let m_opts = map_options.create()
  map_options.set_attribution_control(m_opts, Some(False))
  map_options.set_zoom_animation(m_opts, Some(False))
  map_options.set_fade_animation(m_opts, Some(False))
  map_options.set_marker_zoom_animation(m_opts, Some(False))
  let m = leaflet.map(t.ContentString("map"), Some(m_opts))

  let bern = leaflet.lat_lng(46.9480, 7.4474, None)
  let stations = [
    #("01", "Geneva", leaflet.lat_lng(46.2044, 6.1432, None)),
    #("02", "Lausanne", leaflet.lat_lng(46.5197, 6.6323, None)),
    #("03", "Bern", bern),
    #("04", "Zurich", leaflet.lat_lng(47.3769, 8.5417, None)),
    #("05", "Basel", leaflet.lat_lng(47.5596, 7.5886, None)),
  ]

  let line_pts = list.map(stations, fn(s) { runtime.dynamic(s.2) })
  let p_opts = polyline_options.create()
  polyline_options.set_smooth_factor(p_opts, Some(1.0))
  path_options.set_color(p_opts, Some("#404040"))
  path_options.set_weight(p_opts, Some(2.5))
  let corridor = leaflet.polyline(line_pts, Some(p_opts))
  let _ = layer.add_to(corridor, t.MapOrLayerGroupMap(m))

  let dot_opts = circle_marker_options.create()
  circle_marker_options.set_radius(dot_opts, Some(6.0))
  path_options.set_color(dot_opts, Some("#111111"))
  path_options.set_weight(dot_opts, Some(2.0))
  path_options.set_fill_color(dot_opts, Some("#ffffff"))
  path_options.set_fill_opacity(dot_opts, Some(1.0))

  let pin_size = t.PointExpressionPoint(leaflet.point(22.0, 22.0, None))
  let pin_anchor = t.PointExpressionPoint(leaflet.point(11.0, 11.0, None))

  list.each(stations, fn(s) {
    let #(code, name, pt) = s
    let cm = leaflet.circle_marker(t.LatLngExpressionLatLng(pt), Some(dot_opts))
    let _ = layer.bind_tooltip(cm, t.LayerBindTooltipContentString(code <> " · " <> name), None)
    let _ = layer.add_to(cm, t.MapOrLayerGroupMap(m))

    let d_opts = div_icon_options.create()
    div_icon_options.set_class_name(d_opts, Some("mono-pin"))
    div_icon_options.set_html(
      d_opts,
      Some(t.StringOrElementOrBooleanString("<span>" <> code <> "</span>")),
    )
    div_icon_options.set_icon_size(d_opts, Some(pin_size))
    div_icon_options.set_icon_anchor(d_opts, Some(pin_anchor))

    let mk_opts = marker_options.create()
    marker_options.set_icon(mk_opts, Some(t.IconOrDivIconDivIcon(leaflet.div_icon(Some(d_opts)))))
    let _ =
      layer.add_to(
        leaflet.marker(t.LatLngExpressionLatLng(pt), Some(mk_opts)),
        t.MapOrLayerGroupMap(m),
      )
    Nil
  })

  let _ =
    map.fit_bounds(
      m,
      t.LatLngBoundsExpressionLatLngBounds(lat_lng_bounds.pad(polyline.get_bounds(corridor), 0.15)),
      None,
    )

  let pop_opts = popup_options.create()
  popup_options.set_max_width(pop_opts, Some(220.0))
  let note = leaflet.popup(t.LatLngExpressionLatLng(bern), Some(pop_opts))
  let _ =
    div_overlay.set_content(
      note,
      t.StringOrHTMLElementOrDivOverlayGetContentResultString("03 Bern · 46.95N, 7.45E"),
    )
  let _ = popup.open_on(note, m)

  let _ =
    evented.on_with_evented_add_event_listener_type_click(
      m,
      t.EventedAddEventListenerTypeClickClick,
      fn(ev) {
        case lat_lng.get_lat(leaflet_mouse_event.get_latlng(ev)) >. 0.0 {
          True ->
            case document.get_element_by_id(core.get_document(), "meta") {
              Some(meta) ->
                dom_node.set_text_content(
                  meta,
                  Some("5 DivIcon stations · popup open (Bern) · synthetic click listener verified"),
                )
              None -> Nil
            }
          False -> Nil
        }
      },
      None,
    )

  let payload = leaflet_mouse_event.create()
  leaflet_mouse_event.set_latlng(payload, bern)
  let _ = evented.fire(m, "click", Some(runtime.dynamic(payload)), None)
  Nil
}
