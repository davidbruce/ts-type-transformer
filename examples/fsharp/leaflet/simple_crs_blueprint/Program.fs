module Program

open Fable.Core.JsInterop
open Dom
open type Dom.Globals
open Leaflet
open type Leaflet.Globals

let private mapOpts () =
    MapOptions.Create(
        crs = CRS.Simple,
        minZoom = -1.0,
        maxZoom = 3.0,
        attributionControl = false,
        zoomAnimation = false,
        fadeAnimation = false,
        markerZoomAnimation = false
    )

let private boxOpts color weight dash fillColor fillOp =
    let dashOpt =
        match dash with
        | Some d -> Some !^d
        | None -> None
    PolylineOptions.Create(
        smoothFactor = 1.0,
        color = color,
        weight = weight,
        ?dashArray = dashOpt,
        fillColor = fillColor,
        fillOpacity = fillOp
    )

let private toPolyPts (pts: LatLng list) : ResizeArray<PolylineNewLatlngs> =
    ResizeArray [ for p in pts -> !^p ]

let main () =
    let m = map (document.getElementById "map", mapOpts ())

    let gridOpts = boxOpts "#e5e5e5" 1.0 None "#ffffff" 0.0
    for r in [ 0.0; 20.0; 40.0; 60.0; 80.0; 100.0 ] do
        polyline(toPolyPts [ latLng (r, 0.0); latLng (r, 160.0) ], gridOpts).addTo (!^m) |> ignore
    for c in [ 0.0; 20.0; 40.0; 60.0; 80.0; 100.0; 120.0; 140.0; 160.0 ] do
        polyline(toPolyPts [ latLng (0.0, c); latLng (100.0, c) ], gridOpts).addTo (!^m) |> ignore

    let envelope = latLngBounds (latLng (8.0, 12.0), latLng (92.0, 148.0))
    rectangle(envelope, boxOpts "#111111" 2.5 None "#ffffff" 0.9).addTo (!^m) |> ignore

    let rooms =
        [|
            12.0, 16.0, 46.0, 62.0, "#f7f7f7"
            54.0, 16.0, 88.0, 62.0, "#efefef"
            12.0, 98.0, 46.0, 144.0, "#e5e5e5"
            54.0, 98.0, 88.0, 144.0, "#d4d4d4"
        |]
    for (r0, c0, r1, c1, fill) in rooms do
        let b = latLngBounds (latLng (r0, c0), latLng (r1, c1))
        rectangle(b, boxOpts "#404040" 1.5 None fill 0.85).addTo (!^m) |> ignore

    let atrium : ResizeArray<PolylineSetLatLngsLatlngs> =
        ResizeArray [
            for p in [ latLng (50.0, 64.0); latLng (74.0, 80.0); latLng (50.0, 96.0); latLng (26.0, 80.0) ] -> !^p
        ]
    polygon(atrium, boxOpts "#111111" 2.0 None "#555555" 0.35).addTo (!^m) |> ignore

    let spine = toPolyPts [ latLng (50.0, 12.0); latLng (50.0, 148.0) ]
    polyline(spine, boxOpts "#737373" 1.5 (Some "4 4") "#ffffff" 0.0).addTo (!^m) |> ignore

    let colOpts =
        CircleMarkerOptions.Create(
            radius = 5.0,
            color = "#111111",
            weight = 1.5,
            fillColor = "#111111",
            fillOpacity = 1.0
        )
    for (r, c) in [ (12.0, 16.0); (88.0, 16.0); (12.0, 80.0); (88.0, 80.0); (12.0, 144.0); (88.0, 144.0) ] do
        circleMarker(latLng (r, c), colOpts).addTo (!^m) |> ignore

    m.fitBounds (latLngBounds (latLng (0.0, 0.0), latLng (100.0, 160.0))) |> ignore

    let px = m.latLngToContainerPoint (latLng (50.0, 80.0))
    let meta = document.getElementById "meta"
    meta.textContent <- sprintf "CRS.Simple 100x160 plan · 4 studio rooms + atrium · center @ (%.0f, %.0f) px" px.x px.y

main ()
