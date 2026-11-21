module Program

open Fable.Core.JsInterop
open Dom
open type Dom.Globals
open Konva

let private stage =
    Stage.Create (StageConfig.Create (container = !^"stage", width = 680.0, height = 400.0))

let private bgLayer = Layer.Create ()
let private fgLayer = Layer.Create ()
stage.add bgLayer |> ignore
stage.add fgLayer |> ignore

let private addTo (l: Layer) (s: Shape<ShapeConfig>) =
    l.add (!^s) |> ignore

let private addLineTo (l: Layer) (line: Line<LineConfig>) =
    l.add (!^(line.asShape ())) |> ignore

// Layer 1: Blueprint Grid (distinct colors: #ffffff, #e5e5e5, #b5b5b5, #737373).
addTo bgLayer (
    Rect.Create(
        RectConfig.Create (x = 0.0, y = 0.0, width = 680.0, height = 400.0, fill = !^"#ffffff")
    ).asShape ()
)

for x in 40 .. 40 .. 640 do
    addLineTo bgLayer (
        Line.Create (
            LineConfig.Create (
                points = !^(ResizeArray [ float x; 20.0; float x; 380.0 ]),
                stroke = !^"#e5e5e5",
                strokeWidth = 1.0
            )
        )
    )

for y in 40 .. 40 .. 360 do
    addLineTo bgLayer (
        Line.Create (
            LineConfig.Create (
                points = !^(ResizeArray [ 20.0; float y; 660.0; float y ]),
                stroke = !^"#e5e5e5",
                strokeWidth = 1.0
            )
        )
    )

addTo bgLayer (
    Rect.Create(
        RectConfig.Create (
            x = 20.0,
            y = 20.0,
            width = 640.0,
            height = 360.0,
            stroke = !^"#b5b5b5",
            strokeWidth = 1.5,
            dash = ResizeArray [ 6.0; 4.0 ]
        )
    ).asShape ()
)

addTo bgLayer (
    Text.Create(
        TextConfig.Create (
            x = 32.0,
            y = 32.0,
            text = "Layer 1: Blueprint Grid (z=0)",
            fontSize = 12.0,
            fontFamily = "monospace",
            fill = !^"#737373"
        )
    ).asShape ()
)

// Layer 2: Overlapping interactive cards & badge.
let private makeCard idVal x y fillHex =
    let r =
        Rect.Create (
            RectConfig.Create (
                id = idVal,
                name = "card",
                x = x,
                y = y,
                width = 200.0,
                height = 130.0,
                fill = !^fillHex,
                stroke = !^"#111111",
                strokeWidth = 2.0
            )
        )
    addTo fgLayer (r.asShape ())
    r

let private cardA = makeCard "card-a" 110.0 80.0 "#efefef"
let private _cardB = makeCard "card-b" 210.0 125.0 "#404040"
let private _cardC = makeCard "card-c" 310.0 170.0 "#ffffff"

addTo fgLayer (
    Circle.Create(
        CircleConfig.Create (id = "badge", x = 555.0, y = 135.0, radius = 38.0, fill = !^"#111111")
    ).asShape ()
)

cardA.moveToTop () |> ignore
let private cardAZ = int (cardA.getZIndex ())

let mutable clicks = 0
cardA.on_click (fun _ -> clicks <- clicks + 1) |> ignore
cardA.fire "click" |> ignore
cardA.fire "click" |> ignore

bgLayer.draw () |> ignore
fgLayer.draw () |> ignore

let private pt = Vector2d.Create (250.0, 160.0)
let private hit = stage.getIntersection pt
let private hitId = if isNull hit then "none" else hit.id.Invoke ()

// Crosshair marker at (250, 160) & foreground label.
addTo fgLayer (
    Circle.Create(
        CircleConfig.Create (
            x = 250.0,
            y = 160.0,
            radius = 7.0,
            fill = !^"#ffffff",
            stroke = !^"#111111",
            strokeWidth = 2.0
        )
    ).asShape ()
)

for (pts: float list) in [ [ 238.0; 160.0; 262.0; 160.0 ]; [ 250.0; 148.0; 250.0; 172.0 ] ] do
    addLineTo fgLayer (
        Line.Create (
            LineConfig.Create (
                points = !^(ResizeArray pts),
                stroke = !^"#111111",
                strokeWidth = 1.5
            )
        )
    )

addTo fgLayer (
    Text.Create(
        TextConfig.Create (
            x = 32.0,
            y = 354.0,
            text = "Layer 2: Elevated Stack (z=1) · Probe (250,160)",
            fontSize = 12.0,
            fontFamily = "monospace",
            fill = !^"#111111"
        )
    ).asShape ()
)

fgLayer.draw () |> ignore

let private meta = document.getElementById "meta"
meta.textContent <- sprintf "Layers: %d · card-a zIndex=%d · hit@(250,160)=%s · clicks=%d" (stage.getLayers().Count) cardAZ hitId clicks
