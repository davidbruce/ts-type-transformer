module Program

open System
open Fable.Core
open Fable.Core.JsInterop
open Dom
open type Dom.Globals
open Pixi.Js

let private strokeStyle w (c: int) : StrokeInput =
    U9.Case9 (!^ (StrokeStyle.Create (width = w, color = !^ (float c))))

let private initOpts () =
    InitOptions.Create(
        width = 680.0,
        height = 400.0,
        backgroundColor = !^ 16777215.0,
        preference = !^ "webgl",
        preserveDrawingBuffer = true,
        antialias = true,
        autoStart = false
    )

let main =
    async {
        let app = Application.Create ()
        do! Async.AwaitPromise (app.init (initOpts ()))

        let stageEl = document.getElementById "stage"
        stageEl.appendChild app.canvas |> ignore

        let guides = Graphics.Create ()
        let rings = [| 42.0; 82.0; 122.0; 162.0 |]
        let ringColors = [| 0x111111; 0x555555; 0xa3a3a3; 0xe5e5e5 |]

        for r in rings do
            guides.circle(340.0, 200.0, r).stroke (strokeStyle 1.0 0xe5e5e5) |> ignore

        for step in 0 .. 11 do
            let theta = float step * (2.0 * Math.PI / 12.0)
            let spokeM = Matrix.Create().rotate(theta)
            let dir = spokeM.apply<Point> (Point.Create (1.0, 0.0))
            guides.moveTo(340.0, 200.0).lineTo(340.0 + dir.x * 182.0, 200.0 + dir.y * 182.0) |> ignore
        guides.stroke (strokeStyle 1.0 0xd4d4d4) |> ignore
        app.stage.addChild guides |> ignore

        let tiles = Graphics.Create ()
        for rIdx in 0 .. 3 do
            let radius = rings[rIdx]
            let s = 8.0 + float rIdx * 2.5
            let corners = [| (0.0, -s); (s * 0.72, 0.0); (0.0, s); (-s * 0.72, 0.0) |]
            for step in 0 .. 11 do
                let theta = float step * (2.0 * Math.PI / 12.0) + float rIdx * 0.13
                let dir = Matrix.Create().rotate(theta).apply<Point> (Point.Create (1.0, 0.0))
                let tileM =
                    Matrix
                        .Create()
                        .rotate(theta + float rIdx * 0.2)
                        .translate(340.0 + radius * dir.x, 200.0 + radius * dir.y)
                let (x0, y0) = corners[0]
                let p0 = tileM.apply<Point> (Point.Create (x0, y0))
                tiles.moveTo (p0.x, p0.y) |> ignore
                for k in 1 .. 3 do
                    let (xk, yk) = corners[k]
                    let pk = tileM.apply<Point> (Point.Create (xk, yk))
                    tiles.lineTo (pk.x, pk.y) |> ignore
                tiles
                    .closePath()
                    .fill(color = !^ (float ringColors[rIdx]))
                    .stroke(strokeStyle 1.2 0x111111)
                |> ignore
        app.stage.addChild tiles |> ignore

        let centerDot = Graphics.Create ()
        centerDot.circle(340.0, 200.0, 8.0).fill (color = !^ (float 0x111111)) |> ignore
        app.stage.addChild centerDot |> ignore

        app.render ()
        let meta = document.getElementById "meta"
        meta.textContent <- "48 matrix-transformed diamond tiles · 4 rings · 12 radial spokes · 680x400 WebGL"
    }

main |> Async.StartImmediate
