module Demos.Leaflet.VectorRoute

open Fable.Core
open Fable.Core.JsInterop
open Dom
open type Dom.Globals
open Leaflet
open type Leaflet.Globals

let private mapOpts () =
    MapOptions.Create(
        attributionControl = false,
        zoomAnimation = false,
        fadeAnimation = false,
        markerZoomAnimation = false
    )

let private polyOpts color weight dash fillOpacity =
    let dashOpt =
        match dash with
        | Some d -> Some !^d
        | None -> None
    PolylineOptions.Create(
        smoothFactor = 1.0,
        color = color,
        weight = weight,
        ?dashArray = dashOpt,
        fillOpacity = fillOpacity
    )

let private toPolyPts (pts: LatLng list) : ResizeArray<PolylineNewLatlngs> =
    ResizeArray [ for p in pts -> !^p ]

let main () =
    let m = map ((document.getElementById "map").Value, mapOpts ())

    let hubs =
        [|
            "London", latLng (51.505, -0.09)
            "Paris", latLng (48.8566, 2.3522)
            "Zurich", latLng (47.3769, 8.5417)
            "Munich", latLng (48.1351, 11.5820)
            "Prague", latLng (50.0755, 14.4378)
            "Berlin", latLng (52.5200, 13.4050)
        |]

    let gratOpts = polyOpts "#e5e5e5" 1.0 None 0.0
    for lat in [ 46.0; 48.0; 50.0; 52.0; 54.0 ] do
        polyline(toPolyPts [ latLng (lat, -2.0); latLng (lat, 17.0) ], gratOpts).addTo m |> ignore
    for lng in [ 0.0; 4.0; 8.0; 12.0; 16.0 ] do
        polyline(toPolyPts [ latLng (45.0, lng); latLng (55.0, lng) ], gratOpts).addTo m |> ignore

    let primaryPts = hubs |> Array.map snd |> Array.toList
    let primaryLine = polyline(toPolyPts primaryPts, polyOpts "#111111" 3.0 None 0.0)
    primaryLine.addTo m |> ignore

    let altPts =
        [
            snd hubs[0]
            latLng (50.8503, 4.3517)
            latLng (52.3676, 4.9041)
            snd hubs[5]
        ]
    polyline(toPolyPts altPts, polyOpts "#737373" 1.5 (Some "5 5") 0.0).addTo m |> ignore

    let nodes = layerGroup ()
    let nodeOpts =
        CircleMarkerOptions.Create(
            radius = 6.0,
            color = "#111111",
            weight = 2.0,
            fillColor = "#ffffff",
            fillOpacity = 1.0
        )
    for (name, pt) in hubs do
        let cm = circleMarker (pt, nodeOpts)
        cm.bindTooltip name |> ignore
        nodes.addLayer cm |> ignore
    nodes.addTo m |> ignore

    m.fitBounds (primaryLine.getBounds ()) |> ignore

    let mutable totalKm = 0.0
    for i in 0 .. hubs.Length - 2 do
        totalKm <- totalKm + (snd hubs[i]).distanceTo (snd hubs[i + 1]) / 1000.0

    let meta = (document.getElementById "meta").Value
    meta.textContent <- sprintf "6 hubs · 2 routes · %.0f km total · EPSG:3857 vector SVG" totalKm

main ()
