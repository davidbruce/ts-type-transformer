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

        let stageEl = (document.getElementById "stage").Value
        stageEl.appendChild app.canvas |> ignore

        let guide = Graphics.Create ()
        guide.moveTo(40.0, 200.0).lineTo(640.0, 200.0).moveTo(340.0, 24.0).lineTo(340.0, 376.0) |> ignore
        guide.stroke (strokeStyle 1.0 0xe5e5e5) |> ignore
        guide.circle(340.0, 200.0, 110.0).stroke (strokeStyle 1.0 0xd4d4d4) |> ignore
        app.stage.addChild guide |> ignore

        let root = Container.Create ()
        root.position.set (340.0, 200.0) |> ignore

        for i in 0 .. 2 do
            let arm = Container.Create ()
            arm.rotation <- float i * 2.0 * Math.PI / 3.0

            let armBar = Graphics.Create ()
            armBar.rect(0.0, -4.0, 110.0, 8.0).fill (color = !^ (float 0x737373)) |> ignore
            armBar.circle(110.0, 0.0, 10.0).fill (color = !^ (float 0x111111)) |> ignore
            arm.addChild armBar |> ignore

            let sub = Container.Create ()
            sub.x <- 110.0
            sub.rotation <- -0.45

            let subBar = Graphics.Create ()
            subBar.moveTo(-32.0, 0.0).lineTo(48.0, 0.0).stroke (strokeStyle 2.0 0x404040) |> ignore
            sub.addChild subBar |> ignore

            let satBox = Graphics.Create ()
            satBox.rect(-12.0, -12.0, 24.0, 24.0).fill (color = !^ (float 0x262626)) |> ignore
            satBox.x <- 48.0
            sub.addChild satBox |> ignore

            let satOrb = Graphics.Create ()
            satOrb
                .circle(0.0, 0.0, 14.0)
                .fill(color = !^ (float 0xd4d4d4))
                .stroke(strokeStyle 2.0 0x111111)
            |> ignore
            satOrb.x <- -32.0
            sub.addChild satOrb |> ignore

            arm.addChild sub |> ignore
            root.addChild arm |> ignore

        let hub = Graphics.Create ()
        hub.circle(0.0, 0.0, 18.0).fill (color = !^ (float 0x111111)) |> ignore
        hub.circle(0.0, 0.0, 7.0).fill (color = !^ (float 0xffffff)) |> ignore
        root.addChild hub |> ignore

        app.stage.addChild root |> ignore
        app.render ()

        let rb = root.getBounds ()
        let meta = (document.getElementById "meta").Value
        meta.textContent <- sprintf "3 primary arms · 6 nested satellites · bounds %.1fx%.1f · 680x400 WebGL" rb.width rb.height
    }

main |> Async.StartImmediate
