module Demos.Leaflet.PolygonZones

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

let private toPolygonPts (pts: LatLng list) : ResizeArray<PolylineSetLatLngsLatlngs> =
    ResizeArray [ for p in pts -> !^p ]

let main () =
    let m = map ((document.getElementById "map").Value, mapOpts ())

    let midRing =
        [
            latLng (47.2, 7.6)
            latLng (48.0, 8.6)
            latLng (47.5, 9.7)
            latLng (46.8, 9.5)
            latLng (46.6, 8.4)
            latLng (46.9, 7.3)
        ]

    let coreRing =
        [
            latLng (47.35, 8.2)
            latLng (47.55, 8.75)
            latLng (47.15, 9.0)
            latLng (46.95, 8.4)
        ]

    let midPoly =
        polygon(
            toPolygonPts midRing,
            PolylineOptions.Create(
                smoothFactor = 1.0,
                color = "#404040",
                weight = 2.0,
                fillColor = "#d4d4d4",
                fillOpacity = 0.55
            )
        )
    let corePoly =
        polygon(
            toPolygonPts coreRing,
            PolylineOptions.Create(
                smoothFactor = 1.0,
                color = "#111111",
                weight = 2.5,
                fillColor = "#404040",
                fillOpacity = 0.75
            )
        )

    let paddedBounds = midPoly.getBounds().pad 0.18
    let hullRect =
        rectangle(
            paddedBounds,
            PolylineOptions.Create(
                smoothFactor = 1.0,
                color = "#737373",
                weight = 1.5,
                dashArray = "4 4",
                fillColor = "#efefef",
                fillOpacity = 0.05
            )
        )

    hullRect.addTo m |> ignore
    midPoly.addTo m |> ignore
    corePoly.addTo m |> ignore

    let coreBounds = corePoly.getBounds ()
    let midBounds = midPoly.getBounds ()

    let probes =
        [|
            latLng (47.25, 8.6)
            latLng (47.15, 8.55)
            latLng (47.7, 8.6)
            latLng (47.0, 9.2)
            latLng (46.8, 7.9)
            latLng (47.85, 7.5)
            latLng (47.9, 9.7)
            latLng (46.55, 9.6)
        |]

    let mutable inCore = 0
    let mutable inMid = 0
    let mutable exterior = 0

    for pt in probes do
        let fillHex =
            if coreBounds.contains pt then
                inCore <- inCore + 1
                "#111111"
            elif midBounds.contains pt then
                inMid <- inMid + 1
                "#737373"
            else
                exterior <- exterior + 1
                "#ffffff"
        circleMarker(
            pt,
            CircleMarkerOptions.Create(
                radius = 6.0,
                color = "#111111",
                weight = 2.0,
                fillColor = fillHex,
                fillOpacity = 1.0
            )
        ).addTo m
        |> ignore

    m.fitBounds paddedBounds |> ignore

    let meta = (document.getElementById "meta").Value
    meta.textContent <-
        sprintf "2 nested polygons · 1 padded hull · probes: %d core · %d sector · %d exterior" inCore inMid exterior

main ()
