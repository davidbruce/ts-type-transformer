module Program

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

        let grid = Graphics.Create ()
        for x in 40.0 .. 40.0 .. 640.0 do
            grid.moveTo(x, 0.0).lineTo(x, 400.0) |> ignore
        for y in 40.0 .. 40.0 .. 360.0 do
            grid.moveTo(0.0, y).lineTo(680.0, y) |> ignore
        grid.stroke (strokeStyle 1.0 0xe5e5e5) |> ignore
        app.stage.addChild grid |> ignore

        let rectG = Graphics.Create ()
        rectG.rect(40.0, 46.0, 140.0, 88.0).fill (color = !^ (float 0x111111)) |> ignore
        app.stage.addChild rectG |> ignore

        let roundG = Graphics.Create ()
        roundG
            .roundRect(216.0, 46.0, 156.0, 88.0, 16.0)
            .fill(color = !^ (float 0xf7f7f7))
            .stroke(strokeStyle 2.0 0x111111)
        |> ignore
        app.stage.addChild roundG |> ignore

        let circleG = Graphics.Create ()
        circleG.circle(458.0, 90.0, 44.0).fill (color = !^ (float 0x404040)) |> ignore
        circleG.circle(458.0, 90.0, 22.0).stroke (strokeStyle 2.0 0xffffff) |> ignore
        circleG.circle(458.0, 90.0, 52.0).stroke (strokeStyle 1.5 0x111111) |> ignore
        app.stage.addChild circleG |> ignore

        let ellipseG = Graphics.Create ()
        ellipseG
            .ellipse(588.0, 90.0, 56.0, 36.0)
            .fill(color = !^ (float 0xd4d4d4))
            .stroke(strokeStyle 1.5 0x111111)
        |> ignore
        app.stage.addChild ellipseG |> ignore

        let polyG = Graphics.Create ()
        polyG
            .regularPoly(130.0, 274.0, 58.0, 6.0, 0.25)
            .fill(color = !^ (float 0x737373))
            .stroke(strokeStyle 2.0 0x111111)
        |> ignore
        app.stage.addChild polyG |> ignore

        let starG = Graphics.Create ()
        starG.star(340.0, 274.0, 5.0, 62.0, 26.0, -0.314).fill (color = !^ (float 0x111111)) |> ignore
        app.stage.addChild starG |> ignore

        let diamondG = Graphics.Create ()
        diamondG
            .moveTo(548.0, 212.0)
            .lineTo(618.0, 274.0)
            .lineTo(548.0, 336.0)
            .lineTo(478.0, 274.0)
            .closePath()
            .fill(color = !^ (float 0xb5b5b5))
            .stroke(strokeStyle 1.5 0x111111)
        |> ignore
        app.stage.addChild diamondG |> ignore

        app.render ()
        let meta = document.getElementById "meta"
        meta.textContent <- "7 vector primitives · rect · roundRect · circle · ellipse · regularPoly · star · path · 680x400 WebGL"
    }

main |> Async.StartImmediate
