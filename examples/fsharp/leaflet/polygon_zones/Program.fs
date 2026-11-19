module Program

open Fable.Core.JsInterop
open Dom
open type Dom.Globals
open Leaflet
open type Leaflet.Globals

let private mapOpts () =
    MapOptions.Create(
        attributionControl = Some false,
        zoomAnimation = Some false,
        fadeAnimation = Some false,
        markerZoomAnimation = Some false
    )

let private toPolygonPts (pts: LatLng list) =
    ResizeArray (pts |> List.map PolylineSetLatLngsLatlngs.LatLng)

let main () =
    let m = map (unbox<HTMLElement> (document.getElementById "map"), mapOpts ())

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
        polygon<obj>(
            toPolygonPts midRing,
            PolylineOptions.Create(
                smoothFactor = Some 1.0,
                color = Some "#404040",
                weight = Some 2.0,
                fillColor = Some "#d4d4d4",
                fillOpacity = Some 0.55
            )
        )
    let corePoly =
        polygon<obj>(
            toPolygonPts coreRing,
            PolylineOptions.Create(
                smoothFactor = Some 1.0,
                color = Some "#111111",
                weight = Some 2.5,
                fillColor = Some "#404040",
                fillOpacity = Some 0.75
            )
        )

    let paddedBounds = midPoly.getBounds().pad 0.18
    let hullRect =
        rectangle<obj>(
            paddedBounds,
            PolylineOptions.Create(
                smoothFactor = Some 1.0,
                color = Some "#737373",
                weight = Some 1.5,
                dashArray = Some (StringOrNumberArray.String "4 4"),
                fillColor = Some "#efefef",
                fillOpacity = Some 0.05
            )
        )

    hullRect.addTo (MapOrLayerGroup.Map m) |> ignore
    midPoly.addTo (MapOrLayerGroup.Map m) |> ignore
    corePoly.addTo (MapOrLayerGroup.Map m) |> ignore

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
        circleMarker<obj>(
            pt,
            CircleMarkerOptions.Create(
                radius = Some 6.0,
                color = Some "#111111",
                weight = Some 2.0,
                fillColor = Some fillHex,
                fillOpacity = Some 1.0
            )
        ).addTo (MapOrLayerGroup.Map m)
        |> ignore

    m.fitBounds (LatLngBoundsExpression.LatLngBounds paddedBounds) |> ignore

    let meta = document.getElementById "meta"
    meta.textContent <-
        sprintf "2 nested polygons · 1 padded hull · probes: %d core · %d sector · %d exterior" inCore inMid exterior

main ()
