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

let main () =
    let m = map (unbox<HTMLElement> (document.getElementById "map"), mapOpts ())
    let zurich = latLng (47.3769, 8.5417)
    m.setView (LatLngExpression.LatLng zurich, 8.0) |> ignore

    let rings =
        [|
            130000.0, "#404040", "#d4d4d4", 0.25
            90000.0, "#404040", "#a3a3a3", 0.30
            55000.0, "#111111", "#737373", 0.40
            25000.0, "#111111", "#262626", 0.55
        |]

    let mutable outerCircle : Circle<obj> = null
    for (r, stroke, fill, op) in rings do
        let cOpts =
            CircleMarkerOptions.Create(
                radius = Some r,
                color = Some stroke,
                weight = Some 1.5,
                fillColor = Some fill,
                fillOpacity = Some op
            )
        let c = circle<obj>(zurich, r, cOpts)
        c.addTo (MapOrLayerGroup.Map m) |> ignore
        if obj.ReferenceEquals (outerCircle, null) then outerCircle <- c

    let beacons =
        [|
            latLng (47.5596, 7.5886)
            latLng (46.9480, 7.4474)
            latLng (47.0502, 8.3093)
            latLng (47.4245, 9.3767)
            latLng (46.8508, 9.5320)
        |]

    let mutable totalDistKm = 0.0
    for pt in beacons do
        totalDistKm <- totalDistKm + zurich.distanceTo (LatLngExpression.LatLng pt) / 1000.0
        let bRingOpts =
            CircleMarkerOptions.Create(
                radius = Some 18000.0,
                color = Some "#555555",
                weight = Some 1.2,
                dashArray = Some (StringOrNumberArray.String "3 3"),
                fillColor = Some "#737373",
                fillOpacity = Some 0.18
            )
        circle<obj>(pt, 18000.0, bRingOpts).addTo (MapOrLayerGroup.Map m)
        |> ignore
        circleMarker<obj>(
            pt,
            CircleMarkerOptions.Create(
                radius = Some 4.5,
                color = Some "#111111",
                weight = Some 1.5,
                fillColor = Some "#111111",
                fillOpacity = Some 1.0
            )
        ).addTo (MapOrLayerGroup.Map m)
        |> ignore

    m.fitBounds (LatLngBoundsExpression.LatLngBounds (outerCircle.getBounds ())) |> ignore

    let avgKm = totalDistKm / float beacons.Length
    let meta = document.getElementById "meta"
    meta.textContent <- sprintf "4 concentric rings (25–130 km) · 5 regional beacons (avg %.1f km)" avgKm

main ()
