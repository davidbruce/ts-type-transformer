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
        | Some d -> Some (StringOrNumberArray.String d)
        | None -> None
    PolylineOptions.Create(
        smoothFactor = 1.0,
        color = color,
        weight = weight,
        ?dashArray = dashOpt,
        fillColor = fillColor,
        fillOpacity = fillOp
    )

let main () =
    let m = map (document.getElementById "map", mapOpts ())

    let gridOpts = boxOpts "#e5e5e5" 1.0 None "#ffffff" 0.0
    for r in [ 0.0; 20.0; 40.0; 60.0; 80.0; 100.0 ] do
        polyline<obj, obj>(
            ResizeArray [ PolylineNewLatlngs.LatLng (latLng (r, 0.0)); PolylineNewLatlngs.LatLng (latLng (r, 160.0)) ],
            gridOpts
        ).addTo (MapOrLayerGroup.Map m)
        |> ignore
    for c in [ 0.0; 20.0; 40.0; 60.0; 80.0; 100.0; 120.0; 140.0; 160.0 ] do
        polyline<obj, obj>(
            ResizeArray [ PolylineNewLatlngs.LatLng (latLng (0.0, c)); PolylineNewLatlngs.LatLng (latLng (100.0, c)) ],
            gridOpts
        ).addTo (MapOrLayerGroup.Map m)
        |> ignore

    let envelope = latLngBounds (latLng (8.0, 12.0), latLng (92.0, 148.0))
    rectangle<obj>(envelope, boxOpts "#111111" 2.5 None "#ffffff" 0.9).addTo (MapOrLayerGroup.Map m) |> ignore

    let rooms =
        [|
            12.0, 16.0, 46.0, 62.0, "#f7f7f7"
            54.0, 16.0, 88.0, 62.0, "#efefef"
            12.0, 98.0, 46.0, 144.0, "#e5e5e5"
            54.0, 98.0, 88.0, 144.0, "#d4d4d4"
        |]
    for (r0, c0, r1, c1, fill) in rooms do
        let b = latLngBounds (latLng (r0, c0), latLng (r1, c1))
        rectangle<obj>(b, boxOpts "#404040" 1.5 None fill 0.85).addTo (MapOrLayerGroup.Map m) |> ignore

    let atrium =
        [
            latLng (50.0, 64.0)
            latLng (74.0, 80.0)
            latLng (50.0, 96.0)
            latLng (26.0, 80.0)
        ] |> List.map PolylineSetLatLngsLatlngs.LatLng
    polygon<obj>(ResizeArray atrium, boxOpts "#111111" 2.0 None "#555555" 0.35).addTo (MapOrLayerGroup.Map m)
    |> ignore

    let spine =
        [
            latLng (50.0, 12.0)
            latLng (50.0, 148.0)
        ] |> List.map PolylineNewLatlngs.LatLng
    polyline<obj, obj>(ResizeArray spine, boxOpts "#737373" 1.5 (Some "4 4") "#ffffff" 0.0)
        .addTo (MapOrLayerGroup.Map m)
    |> ignore

    let colOpts =
        CircleMarkerOptions.Create(
            radius = 5.0,
            color = "#111111",
            weight = 1.5,
            fillColor = "#111111",
            fillOpacity = 1.0
        )
    for (r, c) in [ (12.0, 16.0); (88.0, 16.0); (12.0, 80.0); (88.0, 80.0); (12.0, 144.0); (88.0, 144.0) ] do
        circleMarker<obj>(latLng (r, c), colOpts).addTo (MapOrLayerGroup.Map m) |> ignore

    m.fitBounds (LatLngBoundsExpression.LatLngBounds (latLngBounds (latLng (0.0, 0.0), latLng (100.0, 160.0))))
    |> ignore

    let px = m.latLngToContainerPoint (latLng (50.0, 80.0))
    let meta = document.getElementById "meta"
    meta.textContent <- sprintf "CRS.Simple 100x160 plan · 4 studio rooms + atrium · center @ (%.0f, %.0f) px" px.x px.y

main ()
