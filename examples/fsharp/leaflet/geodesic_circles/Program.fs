module Program

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

let main () =
    let m = map ((document.getElementById "map").Value, mapOpts ())
    let zurich = latLng (47.3769, 8.5417)
    m.setView (zurich, 8.0) |> ignore

    let rings =
        [|
            130000.0, "#404040", "#d4d4d4", 0.25
            90000.0, "#404040", "#a3a3a3", 0.30
            55000.0, "#111111", "#737373", 0.40
            25000.0, "#111111", "#262626", 0.55
        |]

    let mutable outerCircle : Circle<obj> option = None
    for (r, stroke, fill, op) in rings do
        let cOpts =
            CircleMarkerOptions.Create(
                radius = r,
                color = stroke,
                weight = 1.5,
                fillColor = fill,
                fillOpacity = op
            )
        let c = circle (zurich, r, cOpts)
        c.addTo (!^m) |> ignore
        if outerCircle.IsNone then outerCircle <- Some c

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
        totalDistKm <- totalDistKm + zurich.distanceTo pt / 1000.0
        let bRingOpts =
            CircleMarkerOptions.Create(
                radius = 18000.0,
                color = "#555555",
                weight = 1.2,
                dashArray = !^"3 3",
                fillColor = "#737373",
                fillOpacity = 0.18
            )
        circle(pt, 18000.0, bRingOpts).addTo (!^m) |> ignore
        circleMarker(
            pt,
            CircleMarkerOptions.Create(
                radius = 4.5,
                color = "#111111",
                weight = 1.5,
                fillColor = "#111111",
                fillOpacity = 1.0
            )
        ).addTo (!^m)
        |> ignore

    match outerCircle with
    | Some c -> m.fitBounds (c.getBounds ()) |> ignore
    | None -> ()

    let avgKm = totalDistKm / float beacons.Length
    let meta = (document.getElementById "meta").Value
    meta.textContent <- sprintf "4 concentric rings (25–130 km) · 5 regional beacons (avg %.1f km)" avgKm

main ()
