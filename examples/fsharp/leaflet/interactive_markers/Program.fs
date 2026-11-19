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

    let stations =
        [|
            "01", "Geneva", latLng (46.2044, 6.1432)
            "02", "Lausanne", latLng (46.5197, 6.6323)
            "03", "Bern", latLng (46.9480, 7.4474)
            "04", "Zurich", latLng (47.3769, 8.5417)
            "05", "Basel", latLng (47.5596, 7.5886)
        |]

    let corridor =
        polyline<obj, obj>(
            ResizeArray (stations |> Array.map (fun (_, _, p) -> PolylineNewLatlngs.LatLng p) |> Array.toList),
            PolylineOptions.Create(smoothFactor = Some 1.0, color = Some "#404040", weight = Some 2.5)
        )
    corridor.addTo (MapOrLayerGroup.Map m) |> ignore

    let dotOpts =
        CircleMarkerOptions.Create(
            radius = Some 6.0,
            color = Some "#111111",
            weight = Some 2.0,
            fillColor = Some "#ffffff",
            fillOpacity = Some 1.0
        )

    for (code, name, pt) in stations do
        let cm = circleMarker<obj> (pt, dotOpts)
        cm.bindTooltip (LayerBindTooltipContent.String (sprintf "%s · %s" code name)) |> ignore
        cm.addTo (MapOrLayerGroup.Map m) |> ignore

        let icon =
            divIcon (
                DivIconOptions.Create(
                    className = Some "mono-pin",
                    html = Some (StringOrElementOrBoolean.String (sprintf "<span>%s</span>" code)),
                    iconSize = Some (PointExpression.Point (point (22.0, 22.0))),
                    iconAnchor = Some (PointExpression.Point (point (11.0, 11.0)))
                )
            )
        let mk = marker<obj>(pt, MarkerOptions.Create(icon = Some (IconOrDivIcon.DivIcon icon)))
        mk.addTo (MapOrLayerGroup.Map m) |> ignore

    m.fitBounds (LatLngBoundsExpression.LatLngBounds (corridor.getBounds().pad 0.15)) |> ignore

    let (_, _, bernPt) = stations[2]
    let note = popup (bernPt, PopupOptions.Create(maxWidth = Some 220.0))
    note.setContent (StringOrHTMLElementOrFn.String "03 Bern · 46.95N, 7.45E") |> ignore
    note.openOn m |> ignore

    let mutable clicks = 0
    let onClick: LeafletMouseEventHandlerFn =
        fun (ev: LeafletMouseEvent) ->
            if ev.latlng.lat > 0.0 then
                clicks <- clicks + 1
    m.on (EventedAddEventListenerTypeClick.Click, onClick) |> ignore
    let payload = createObj [ "latlng" ==> bernPt ]
    m.fire ("click", payload) |> ignore
    m.fire ("click", payload) |> ignore

    let meta = document.getElementById "meta"
    meta.textContent <- sprintf "%d DivIcon stations · popup open (Bern) · %d synthetic clicks dispatched" stations.Length clicks

main ()
