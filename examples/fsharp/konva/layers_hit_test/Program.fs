module Program

open Fable.Core.JsInterop
open Dom
open type Dom.Globals
open Konva

let private stage =
    Stage.Create (StageConfig.Create (container = "stage", width = 680.0, height = 400.0))

let private bgLayer = Layer.Create ()
let private fgLayer = Layer.Create ()
stage.add bgLayer |> ignore
stage.add fgLayer |> ignore

let private addTo (l: Layer) (s: Shape<ShapeConfig>) =
    l.add s |> ignore

let private addLineTo (l: Layer) (line: Line<LineConfig>) =
    l.add (line.asShape ()) |> ignore

// Layer 1: Blueprint Grid (distinct colors: #ffffff, #e5e5e5, #b5b5b5, #737373).
addTo bgLayer (
    Rect.Create(
        RectConfig.Create (x = 0.0, y = 0.0, width = 680.0, height = 400.0, fill = "#ffffff")
    ).asShape ()
)

for x in 40 .. 40 .. 640 do
    addLineTo bgLayer (
        Line.Create (
            LineConfig.Create (
                points = !^(ResizeArray [ float x; 20.0; float x; 380.0 ]),
                stroke = "#e5e5e5",
                strokeWidth = 1.0
            )
        )
    )

for y in 40 .. 40 .. 360 do
    addLineTo bgLayer (
        Line.Create (
            LineConfig.Create (
                points = !^(ResizeArray [ 20.0; float y; 660.0; float y ]),
                stroke = "#e5e5e5",
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
            stroke = "#b5b5b5",
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
            fill = "#737373"
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
                stroke = "#111111",
                strokeWidth = 2.0
            )
        )
    addTo fgLayer (r.asShape ())
    r

let private cardA = makeCard "card-a" 110.0 80.0 "#efefef"
let private cardB = makeCard "card-b" 210.0 125.0 "#404040"
let private cardC = makeCard "card-c" 310.0 170.0 "#ffffff"

addTo fgLayer (
    Circle.Create(
        CircleConfig.Create (id = "badge", x = 555.0, y = 135.0, radius = 38.0, fill = "#111111")
    ).asShape ()
)

// Crosshair marker at (250, 160) & foreground label in a non-listening overlay group.
let private overlay = Group.Create (GroupConfig.Create (listening = false))

overlay.add (
    (Circle.Create(
        CircleConfig.Create (
            x = 250.0,
            y = 160.0,
            radius = 7.0,
            fill = "#ffffff",
            stroke = "#111111",
            strokeWidth = 2.0
        )
    )).asShape ()
) |> ignore

for (pts: float list) in [ [ 238.0; 160.0; 262.0; 160.0 ]; [ 250.0; 148.0; 250.0; 172.0 ] ] do
    overlay.add (
        (Line.Create (
            LineConfig.Create (
                points = !^(ResizeArray pts),
                stroke = "#111111",
                strokeWidth = 1.5
            )
        )).asShape ()
    ) |> ignore

overlay.add (
    (Text.Create(
        TextConfig.Create (
            x = 32.0,
            y = 354.0,
            text = "Layer 2: Elevated Stack (z=1) · Probe (250,160)",
            fontSize = 12.0,
            fontFamily = "monospace",
            fill = "#111111"
        )
    )).asShape ()
) |> ignore

fgLayer.add overlay |> ignore

cardA.moveToTop () |> ignore
overlay.moveToTop () |> ignore
bgLayer.draw () |> ignore
fgLayer.draw () |> ignore

let mutable clicks = 0
let private handleClick (card: Rect) (cardId: string) =
    card.moveToTop () |> ignore
    overlay.moveToTop () |> ignore
    fgLayer.draw () |> ignore
    clicks <- clicks + 1
    let z = int (card.getZIndex ())
    let pt = Vector2d.Create (250.0, 160.0)
    let hit = stage.getIntersection pt
    let hitId = match hit with Some h -> h.id.Invoke () | None -> "none"
    let meta = (document.getElementById "meta").Value
    meta.textContent <- sprintf "Layers: %d · top=%s (z=%d) · hit@(250,160)=%s · clicks=%d" (stage.getLayers().Count) cardId z hitId clicks

for (card, cardId) in [ (cardA, "card-a"); (cardB, "card-b"); (cardC, "card-c") ] do
    card.on_click (fun _ -> handleClick card cardId) |> ignore

cardA.fire "click" |> ignore
cardA.fire "click" |> ignore
