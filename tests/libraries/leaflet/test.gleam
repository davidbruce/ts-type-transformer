// Leaflet from Gleam, in a page: a map on the page's element, layers on it,
// geometry, and events. No tiles: nothing is fetched. Every check must hold.
import bindings/areas/leaflet as leaflet
import bindings/areas/leaflet/bounds
import bindings/areas/leaflet/circle
import bindings/areas/leaflet/circle_marker
import bindings/areas/leaflet/evented
import bindings/areas/leaflet/lat_lng
import bindings/areas/leaflet/lat_lng_bounds
import bindings/areas/leaflet/fit_bounds_options
import bindings/areas/leaflet/layer
import bindings/areas/leaflet/layer_group
import bindings/areas/leaflet/leaflet_mouse_event
import bindings/areas/leaflet/map
import bindings/areas/leaflet/map_options
import bindings/areas/leaflet/marker
import bindings/areas/leaflet/marker_options
import bindings/areas/leaflet/pan_options
import bindings/areas/leaflet/path_options
import bindings/areas/leaflet/point
import bindings/areas/leaflet/polyline
import bindings/areas/leaflet/popup
import bindings/areas/leaflet/popup_options
import bindings/areas/leaflet/zoom_pan_options
import bindings/areas/test_page
import bindings/runtime
import bindings/types as t
import cell
import gleam/float
import gleam/int
import gleam/io
import gleam/list
import gleam/option.{None, Some}
import gleam/string

fn check(ok: Bool, what: String) -> Int {
  case ok {
    True -> 1
    False -> panic as what
  }
}

fn near(a: Float, b: Float, within: Float) -> Bool {
  float.absolute_value(a -. b) <. within
}

pub fn main() {
  let ll = t.LatLngExpressionLatLng
  let pt = t.PointExpressionPoint

  // A map in the page's #map, centred on London.
  let london = leaflet.lat_lng(51.505, -0.09, None)
  let m = leaflet.map(t.StringOrHTMLElementHTMLElement(test_page.element("map")), None)
  let _ = map.set_view(m, ll(london), Some(13.0), None)
  let passed =
    check(map.get_zoom(m) == 13.0, "zoom")
    + check(lat_lng.equals(map.get_center(m), ll(london), Some(0.000000001)), "centre")
    + check(point.get_x(map.get_size(m)) == 400.0 && point.get_y(map.get_size(m)) == 300.0, "size of the container")
    + check(string.starts_with(leaflet.get_version(), "1."), "version")

  // The centre is the middle of the container, and the projections agree.
  let middle = map.lat_lng_to_container_point(m, ll(london))
  let back = map.container_point_to_lat_lng(m, pt(leaflet.point(200.0, 150.0, None)))
  let passed =
    passed
    + check(point.get_x(middle) == 200.0 && point.get_y(middle) == 150.0, "centre point")
    + check(near(lat_lng.get_lat(back), 51.505, 0.00001), "container point to latlng")
    + check(point.get_x(map.project(m, ll(london), Some(0.0))) >. 0.0, "project")
    + check(map.get_min_zoom(m) == 0.0 && map.get_max_zoom(m) >. 0.0, "zoom limits")

  // Geometry: distances in metres, bounds that grow.
  let paris = leaflet.lat_lng(48.8566, 2.3522, None)
  let km = lat_lng.distance_to(london, ll(paris)) /. 1000.0
  let box = leaflet.lat_lng_bounds(ll(london), ll(london))
  let _ = lat_lng_bounds.extend(box, t.LatLngBoundsExtendLatlngOrBoundsLatLng(paris))
  let passed =
    passed
    + check(km >. 340.0 && km <. 345.0, "distance London to Paris")
    + check(lat_lng_bounds.contains(box, t.LatLngBoundsExtendLatlngOrBoundsLatLng(leaflet.lat_lng(50.0, 1.0, None))), "bounds contain a point between")
    + check(lat_lng.get_lat(lat_lng_bounds.get_north_east(box)) == 51.505, "north east")
    + check(lat_lng_bounds.get_south(box) == 48.8566 && lat_lng_bounds.get_west(box) == -0.09, "south and west")
    + check(lat_lng_bounds.is_valid(box), "valid bounds")
    + check(near(lat_lng.get_lat(lat_lng_bounds.get_center(box)), 50.1808, 0.0001), "bounds centre")
  let p = leaflet.point(3.0, 4.0, None)
  let _ = point.add(p, pt(leaflet.point(1.0, 1.0, None)))
  let passed =
    passed
    + check(point.get_x(p) == 3.0, "add returns a new point")
    + check(point.distance_to(leaflet.point(0.0, 0.0, None), pt(p)) == 5.0, "point distance")
    + check(point.get_x(point.multiply_by(p, 2.0)) == 6.0, "multiply_by")
    + check(point.equals(p, t.PointExpressionArray(#(3.0, 4.0))), "a point equals a tuple")
    + check(point.to_string(p) == "Point(3, 4)", "point to_string")
  let b = leaflet.bounds(pt(leaflet.point(0.0, 0.0, None)), pt(leaflet.point(10.0, 10.0, None)))
  let passed = passed + check(bounds.get_size(b) |> fn(s) { point.get_x(s) == 10.0 }, "bounds size")

  // Layers: a marker added to the map and found there, then removed.
  let pin = leaflet.marker(ll(paris), None)
  let _ = layer.add_to(pin, t.MapOrLayerGroupMap(m))
  let passed =
    passed
    + check(map.has_layer(m, pin), "marker is on the map")
    + check(lat_lng.equals(marker.get_lat_lng(pin), ll(paris), None), "marker position")
  let group = leaflet.layer_group(None, None)
  let ring = leaflet.circle_with_radius(ll(london), 500.0, None)
  let _ = layer_group.add_layer(group, ring)
  let _ = layer.add_to(group, t.MapOrLayerGroupMap(m))
  let _ = map.remove_layer(m, pin)
  let passed =
    passed
    + check(!map.has_layer(m, pin) && map.has_layer(m, group), "pin removed, group kept")
    + check(layer_group.has_layer(group, ring), "the group holds the circle")
    + check(list.length(layer_group.get_layers(group)) == 1, "one layer in the group")
    + check(circle.get_radius(ring) == 500.0, "circle radius")
  let _ = circle.set_radius(ring, 750.0)
  let passed =
    passed
    + check(circle.get_radius(ring) == 750.0, "set_radius")
    + check(lat_lng_bounds.contains(circle.get_bounds(ring), t.LatLngBoundsExtendLatlngOrBoundsLatLng(london)), "circle bounds contain its centre")

  // Options made here: a draggable marker with a title, a styled circle.
  let marker_opts = marker_options.create()
  marker_options.set_title(marker_opts, Some("Big Ben"))
  marker_options.set_draggable(marker_opts, Some(True))
  marker_options.set_opacity(marker_opts, Some(0.5))
  let ben = leaflet.marker(ll(leaflet.lat_lng(51.5007, -0.1246, None)), Some(marker_opts))
  let passed =
    passed
    + check(marker_options.get_title(marker.get_options(ben)) == Some("Big Ben"), "marker title option")
    + check(marker_options.get_draggable(marker.get_options(ben)) == Some(True), "draggable option")
  let style = path_options.create()
  path_options.set_color(style, Some("#ff0000"))
  path_options.set_weight(style, Some(5.0))
  let styled = leaflet.circle_marker(ll(london), None)
  let _ = circle.set_style(circle.upcast(ring), style)
  let passed =
    passed
    + check(path_options.get_color(circle_marker.get_options(ring)) == Some("#ff0000"), "style colour")
    + check(path_options.get_weight(circle_marker.get_options(ring)) == Some(5.0), "style weight")
  let _ = styled

  // A polyline and its bounds.
  let line =
    leaflet.polyline(
      [
        runtime.dynamic(london),
        runtime.dynamic(paris),
      ],
      None,
    )
  let _ = layer.add_to(line, t.MapOrLayerGroupMap(m))
  let passed =
    passed
    + check(!polyline.is_empty(line), "polyline is not empty")
    + check(lat_lng_bounds.contains(polyline.get_bounds(line), t.LatLngBoundsExtendLatlngOrBoundsLatLng(leaflet.lat_lng(50.0, 1.0, None))), "polyline bounds")

  // Fitting the bounds zooms out to show both cities.
  let fit = fit_bounds_options.create()
  fit_bounds_options.set_animate(fit, Some(False))
  let _ = map.fit_bounds(m, t.LatLngBoundsExpressionLatLngBounds(box), Some(fit))
  let passed =
    passed
    + check(map.get_zoom(m) <. 13.0, "fit_bounds zooms out")
    + check(lat_lng_bounds.contains(map.get_bounds(m), t.LatLngBoundsExtendLatlngOrBoundsLatLng(paris)) && lat_lng_bounds.contains(map.get_bounds(m), t.LatLngBoundsExtendLatlngOrBoundsLatLng(london)), "both cities are in view")
  let still = zoom_pan_options.create()
  zoom_pan_options.set_animate(still, Some(False))
  let _ = map.set_zoom(m, 10.0, Some(still))
  let passed = passed + check(map.get_zoom(m) == 10.0, "set_zoom")
  let no_pan = pan_options.create()
  pan_options.set_animate(no_pan, Some(False))
  let _ = map.pan_to(m, ll(paris), Some(no_pan))
  let passed = passed + check(near(lat_lng.get_lat(map.get_center(m)), 48.8566, 0.01), "pan_to")

  // An event fired on the map reaches the listener, with its position.
  let clicks = cell.new(0)
  let on_click = fn(e) {
    case lat_lng.get_lat(leaflet_mouse_event.get_latlng(e)) >. 0.0 {
      True -> cell.set(clicks, cell.get(clicks) + 1)
      False -> Nil
    }
  }
  let _ =
    evented.on_click(
      m,
      t.EventedAddEventListenerTypeClickClick,
      on_click,
      None,
    )
  let data = leaflet_mouse_event.create()
  leaflet_mouse_event.set_latlng(data, london)
  let _ = evented.fire(m, "click", Some(runtime.dynamic(data)), None)
  let _ = evented.fire(m, "click", Some(runtime.dynamic(data)), None)
  let passed =
    passed
    + check(cell.get(clicks) == 2, "the click listener ran twice")
    + check(evented.listens(m, t.EventedListensTypeClick, None), "listens")
  let _ =
    evented.off_click(
      m,
      t.EventedAddEventListenerTypeClickClick,
      Some(on_click),
      None,
    )
  let _ = evented.fire(m, "click", Some(runtime.dynamic(data)), None)
  let passed = passed + check(cell.get(clicks) == 2, "the listener was removed")

  // A popup opened on the map, and a map event by name.
  let opened = cell.new(0)
  let _ =
    evented.on_with_string(m, "popupopen", fn(_) { cell.set(opened, cell.get(opened) + 1) }, None)
  let popup_opts = popup_options.create()
  popup_options.set_max_width(popup_opts, Some(200.0))
  let note = leaflet.popup(ll(london), Some(popup_opts))
  let _ = popup.open_on(note, m)
  let passed =
    passed
    + check(cell.get(opened) == 1, "popupopen fired when the popup opened")
    + check(popup_options.get_max_width(popup.get_options(note)) == Some(200.0), "popup option")
  let _ = map.close_popup(m, Some(note))

  // Map options made here, with a zoom level and no zoom control.
  let opts = map_options.create()
  map_options.set_zoom_control(opts, Some(False))
  map_options.set_zoom(opts, Some(4.0))
  map_options.set_center(opts, Some(t.LatLngExpressionLatLng(paris)))
  let second = leaflet.map(t.StringOrHTMLElementHTMLElement(test_page.element("stage")), Some(opts))
  let passed =
    passed
    + check(map.get_zoom(second) == 4.0, "a second map from options")
    + check(near(lat_lng.get_lng(map.get_center(second)), 2.3522, 0.0001), "its centre")
  let _ = map.remove(second)

  io.println(int.to_string(passed) <> " checks passed")
}
