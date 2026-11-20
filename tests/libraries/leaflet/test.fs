// Leaflet from F#, in a page: a map on the page's element, layers on it,
// geometry, and events. No tiles: nothing is fetched. Every check must hold.
module Test

open Fable.Core
open Fable.Core.JsInterop
open Bindings
open Bindings.Leaflet

let mutable passed = 0

let check (ok: bool) (what: string) =
    if ok then passed <- passed + 1 else failwithf "check failed: %s" what

// A map in the page's #map, centred on London.
let london = Leaflet.latLng (51.505, -0.09)
let m = Leaflet.map (Content.HTMLElement (TestPage.element "map"))
m.setView (london, 13.0) |> ignore
check (m.getZoom () = 13.0) "getZoom"
check (m.getCenter().equals (LatLngExpression.LatLng london, 1e-9)) "getCenter"
check (m.getSize().x = 400.0 && m.getSize().y = 300.0) "the map is as wide as its element"

// The centre is the middle of the container.
let middle = m.latLngToContainerPoint (LatLngExpression.LatLng london)
check (middle.x = 200.0 && middle.y = 150.0) "latLngToContainerPoint"
let back = m.containerPointToLatLng (PointExpression.Point middle)
check (abs (back.lat - 51.505) < 1e-3 && abs (back.lng + 0.09) < 1e-3) "containerPointToLatLng"

// Geometry: distances in metres, bounds that grow.
let paris = Leaflet.latLng (48.8566, 2.3522)
let km = london.distanceTo (LatLngExpression.LatLng paris) / 1000.0
check (km > 340.0 && km < 345.0) "distanceTo"
let bounds = Leaflet.latLngBounds (LatLngExpression.LatLng london, LatLngExpression.LatLng london)
bounds.extend (U5.Case1 paris) |> ignore
check (bounds.contains (U5.Case1 (Leaflet.latLng (50.0, 1.0)))) "bounds contain"
check (bounds.getNorthEast().lat = 51.505) "getNorthEast"
check (bounds.getSouthWest().lat = 48.8566) "getSouthWest"
let pt = Leaflet.point (3.0, 4.0)
check (pt.distanceTo (PointExpression.Point (Leaflet.point (0.0, 0.0))) = 5.0) "Point.distanceTo"
check (pt.add(PointExpression.Point (Leaflet.point (1.0, 1.0))).x = 4.0) "Point.add"
let box = Leaflet.bounds (PointExpression.Point (Leaflet.point (0.0, 0.0)), PointExpression.Point (Leaflet.point (10.0, 20.0)))
check (box.getSize().y = 20.0) "Bounds.getSize"

// Layers: a marker added to the map and found there, then removed.
let pin = Leaflet.marker<obj> (LatLngExpression.LatLng paris)
pin.addTo (MapOrLayerGroup.Map m) |> ignore
check (m.hasLayer pin) "hasLayer"
check (pin.getLatLng().equals (LatLngExpression.LatLng paris)) "getLatLng"
let group = Leaflet.layerGroup<obj> ()
group.addLayer (Leaflet.circle<obj> (LatLngExpression.LatLng london, 500.0)) |> ignore
check (group.getLayers().Count = 1) "layerGroup holds a circle"
group.addTo (MapOrLayerGroup.Map m) |> ignore
m.removeLayer pin |> ignore
check (not (m.hasLayer pin) && m.hasLayer group) "removeLayer"

// Fitting the bounds zooms out to show both cities.
m.fitBounds (LatLngBoundsExpression.LatLngBounds bounds) |> ignore
check (m.getZoom () < 13.0) "fitBounds zooms out"
check (m.getBounds().contains (U5.Case1 paris)) "the map shows Paris"
check (m.getBounds().contains (U5.Case1 london)) "and London"

// An event fired on the map reaches the listener, with its position.
let mutable clicks = 0
let onClick: LeafletMouseEventHandlerFn =
    let f =
        fun (e: LeafletMouseEvent) -> if e.latlng.lat > 0.0 then clicks <- clicks + 1
    f
m.on (EventedAddEventListenerTypeClick.Click, onClick) |> ignore
let data = createObj [ "latlng" ==> london ]
m.fire ("click", data) |> ignore
m.fire ("click", data) |> ignore
check (clicks = 2) "fire reaches the listener"
m.off (EventedAddEventListenerTypeClick.Click, onClick) |> ignore
m.fire ("click", data) |> ignore
check (clicks = 2) "off removes it"

// Paths with options: a polyline's bounds are those of its points.
// FINDING 20 (still open): `Create (color = ..., weight = ...)` alone is ambiguous with PathOptions.Create; naming
// a field only PolylineOptions has (smoothFactor) selects it.
let style = PolylineOptions.Create (smoothFactor = 1.0, color = "red", weight = 5.0)
let line =
    Leaflet.polyline<obj, obj> (ResizeArray [ PolylineNewLatlngs.LatLng london; PolylineNewLatlngs.LatLng paris ], style)
line.addTo (MapOrLayerGroup.Map m) |> ignore
check (m.hasLayer line) "a polyline on the map"
check (line.getBounds().contains (U5.Case1 paris)) "polyline bounds contain its points"
check (line.options.color = Some "red") "the polyline's options"
check (line.getLatLngs().Count = 2) "getLatLngs"
let ring = Leaflet.rectangle<obj> (LatLngBoundsExpression.LatLngBounds bounds)
check (ring.getBounds().contains (U5.Case1 london)) "a rectangle's bounds"

// Zoom and pan.
m.setView (london, 10.0) |> ignore
m.setZoom 12.0 |> ignore
check (m.getZoom () = 12.0) "setZoom"
m.panTo (LatLngExpression.LatLng paris) |> ignore
check (abs (m.getCenter().lat - 48.8566) < 1e-3) "panTo"
check (m.getMinZoom () <= m.getZoom () && m.getZoom () <= m.getMaxZoom ()) "zoom limits"
check (m.getBoundsZoom (LatLngBoundsExpression.LatLngBounds bounds) < 12.0) "getBoundsZoom"

// Projections: Web Mercator maps the origin to the origin.
let origin = CRS.EPSG3857.project (LatLngOrLatLngLiteral.LatLng (Leaflet.latLng (0.0, 0.0)))
check (origin.x = 0.0 && abs origin.y < 1e-6) "EPSG3857 project"
check (CRS.EPSG3857.scale 0.0 = 256.0) "EPSG3857 scale"

printfn "%d checks passed" passed
