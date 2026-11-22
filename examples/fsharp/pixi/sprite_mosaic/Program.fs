module Program

open Fable.Core
open Fable.Core.JsInterop
open Dom
open type Dom.Globals
open Pixi.Js

let private strokeStyle w (c: int) : StrokeInput =
    U9.Case9 (!^ (StrokeStyle.Create (width = w, color = float c)))

let private initOpts () =
    InitOptions.Create(
        width = 680.0,
        height = 400.0,
        backgroundColor = 16777215.0,
        preference = "webgl",
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

        let frame = Graphics.Create ()
        frame
            .rect(32.0, 20.0, 616.0, 360.0)
            .stroke(strokeStyle 1.5 0xd4d4d4)
        |> ignore
        app.stage.addChild frame |> ignore

        let shades = [| 0x111111; 0x262626; 0x404040; 0x555555; 0x737373; 0xa3a3a3; 0xd4d4d4; 0xe5e5e5 |]
        let whiteTex : Texture<TextureSource<obj>> = unbox Texture.WHITE

        for row in 0 .. 5 do
            for col in 0 .. 9 do
                let sprite = Sprite.Create whiteTex
                let idx = (row * 3 + col * 5) % shades.Length
                sprite.width <- 52.0
                sprite.height <- 48.0
                sprite.anchor.set (0.5, 0.5) |> ignore
                sprite.position.set (70.0 + float col * 60.0, 56.0 + float row * 56.0) |> ignore
                sprite.rotation <- (float col - 4.5) * 0.04 + (float row - 2.5) * 0.03
                sprite.alpha <- 0.65 + 0.35 * float ((col + row) % 3) / 2.0
                sprite.tint <- float shades[idx]
                app.stage.addChild sprite |> ignore

        app.render ()
        let meta = (document.getElementById "meta").Value
        meta.textContent <- "60 Texture.WHITE sprites · 10x6 mosaic · 8 monochrome tints · 680x400 WebGL"
    }

main |> Async.StartImmediate
