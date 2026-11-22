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

    let stations =
        [|
            "01", "Geneva", latLng (46.2044, 6.1432)
            "02", "Lausanne", latLng (46.5197, 6.6323)
            "03", "Bern", latLng (46.9480, 7.4474)
            "04", "Zurich", latLng (47.3769, 8.5417)
            "05", "Basel", latLng (47.5596, 7.5886)
        |]

    let corridor =
        polyline(
            ResizeArray<PolylineNewLatlngs> [ for (_, _, p) in stations -> !^p ],
            PolylineOptions.Create(smoothFactor = 1.0, color = "#404040", weight = 2.5)
        )
    corridor.addTo m |> ignore

    let dotOpts =
        CircleMarkerOptions.Create(
            radius = 6.0,
            color = "#111111",
            weight = 2.0,
            fillColor = "#ffffff",
            fillOpacity = 1.0
        )

    for (code, name, pt) in stations do
        let cm = circleMarker (pt, dotOpts)
        cm.bindTooltip (!^ (sprintf "%s · %s" code name)) |> ignore
        cm.addTo m |> ignore

        let icon =
            divIcon (
                DivIconOptions.Create(
                    className = "mono-pin",
                    html = !^ (sprintf "<span>%s</span>" code),
                    iconSize = point (22.0, 22.0),
                    iconAnchor = point (11.0, 11.0)
                )
            )
        let mk = marker(pt, MarkerOptions.Create(icon = icon))
        mk.addTo m |> ignore

    m.fitBounds (corridor.getBounds().pad 0.15) |> ignore

    let (_, _, bernPt) = stations[2]
    let note = popup (bernPt, PopupOptions.Create(maxWidth = 220.0))
    note.setContent "03 Bern · 46.95N, 7.45E" |> ignore
    note.openOn m |> ignore

    let mutable clicks = 0
    let onClick: LeafletMouseEventHandlerFn =
        fun (ev: LeafletMouseEvent) ->
            let ll = ev.latlng
            if ll.lat > 0.0 then
                clicks <- clicks + 1
                if clicks > 2 then
                    (note :> DivOverlay).setLatLng ll |> ignore
                    note.setContent (sprintf "Click #%d · %.2fN, %.2fE" clicks ll.lat ll.lng) |> ignore
                    note.openOn m |> ignore
                let meta = (document.getElementById "meta").Value
                meta.textContent <- sprintf "%d DivIcon stations · clicks=%d · last click (%.2fN, %.2fE)" stations.Length clicks ll.lat ll.lng
    m.on_click onClick |> ignore
    let payload = {| latlng = bernPt |}
    m.fire ("click", payload) |> ignore
    m.fire ("click", payload) |> ignore

main ()
