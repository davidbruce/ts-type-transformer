module Program

open Fable.Core
open Fable.Core.JsInterop
open Dom
open type Dom.Globals
open Pixi.Js

let private initOpts () =
    InitOptions.Create(
        width = 680.0,
        height = 400.0,
        backgroundColor = U9.Case2 16777215.0,
        preference = StringOrRendererPreferenceArray.String "webgl",
        preserveDrawingBuffer = true,
        antialias = true,
        autoStart = false
    )

let private mkRect x y w h =
    Rectangle.Create(NumberOrString.Number x, NumberOrString.Number y, NumberOrString.Number w, NumberOrString.Number h)

let main =
    async {
        let app = Application.Create<obj> ()
        do! Async.AwaitPromise (app.init (initOpts ()))

        let stageEl = document.getElementById "stage"
        stageEl.appendChild (app.canvas :> Node) |> ignore

        let query = mkRect 190.0 105.0 260.0 170.0
        let padded = query.clone().pad(10.0, 10.0)
        let hull = query.clone()

        let candidates =
            [|
                mkRect 210.0 125.0 90.0 60.0
                mkRect 330.0 165.0 95.0 80.0
                mkRect 110.0 80.0 130.0 85.0
                mkRect 390.0 75.0 130.0 90.0
                mkRect 135.0 225.0 110.0 90.0
                mkRect 40.0 40.0 100.0 110.0
                mkRect 510.0 48.0 125.0 115.0
                mkRect 505.0 245.0 130.0 110.0
            |]

        let mutable contained = 0
        let mutable intersecting = 0
        let mutable disjoint = 0

        let g = Graphics.Create ()
        for c in candidates do
            hull.enlarge c |> ignore
            if query.containsRect c then
                contained <- contained + 1
                g.rect(c.x, c.y, c.width, c.height).fill (U9.Case2 (float 0x111111)) |> ignore
            elif query.intersects c then
                intersecting <- intersecting + 1
                g
                    .rect(c.x, c.y, c.width, c.height)
                    .fill(U9.Case2 (float 0x737373))
                    .stroke(createObj [ "width" ==> 2.0; "color" ==> 0x111111 ])
                |> ignore
            else
                disjoint <- disjoint + 1
                g
                    .rect(c.x, c.y, c.width, c.height)
                    .fill(U9.Case2 (float 0xe5e5e5))
                    .stroke(createObj [ "width" ==> 1.0; "color" ==> 0xa3a3a3 ])
                |> ignore

        g.rect(hull.x, hull.y, hull.width, hull.height).stroke (createObj [ "width" ==> 1.5; "color" ==> 0xa3a3a3 ]) |> ignore
        g.rect(padded.x, padded.y, padded.width, padded.height).stroke (createObj [ "width" ==> 1.0; "color" ==> 0x737373 ]) |> ignore
        g.rect(query.x, query.y, query.width, query.height).stroke (createObj [ "width" ==> 2.5; "color" ==> 0x111111 ]) |> ignore

        app.stage.addChild g |> ignore
        app.render ()

        let meta = document.getElementById "meta"
        meta.textContent <-
            sprintf
                "query 260x170 · %d contained · %d intersecting · %d disjoint · enlarged hull %.0fx%.0f"
                contained
                intersecting
                disjoint
                hull.width
                hull.height
    }

main |> Async.StartImmediate
