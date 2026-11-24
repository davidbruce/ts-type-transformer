module Demos.Konva.GroupsTransforms

open Fable.Core.JsInterop
open Dom
open type Dom.Globals
open Konva

let private stage =
    Stage.Create (StageConfig.Create (container = "stage", width = 680.0, height = 400.0))

let private layer = Layer.Create ()
stage.add layer |> ignore

let private addShape (target: Container) (s: Shape) =
    target.add s |> ignore

let private addLine (target: Container) (line: Line) =
    target.add line |> ignore

addShape layer (
    Rect.Create(
        RectConfig.Create (x = 0.0, y = 0.0, width = 680.0, height = 400.0, fill = "#ffffff")
    )
)

for x in 40 .. 40 .. 640 do
    addLine layer (
        Line.Create (
            LineConfig.Create (
                points = !^(ResizeArray [ float x; 0.0; float x; 400.0 ]),
                stroke = "#e5e5e5",
                strokeWidth = 1.0
            )
        )
    )

for y in 40 .. 40 .. 360 do
    addLine layer (
        Line.Create (
            LineConfig.Create (
                points = !^(ResizeArray [ 0.0; float y; 680.0; float y ]),
                stroke = "#e5e5e5",
                strokeWidth = 1.0
            )
        )
    )

// Group A: unrotated base frame.
let private groupA = Group.Create (GroupConfig.Create (x = 40.0, y = 55.0))

addShape groupA (
    Rect.Create(
        RectConfig.Create (width = 160.0, height = 120.0, fill = "#f7f7f7", stroke = "#111111", strokeWidth = 1.5)
    )
)
addShape groupA (
    Circle.Create(
        CircleConfig.Create (x = 80.0, y = 60.0, radius = 24.0, fill = "#111111")
    )
)
addShape groupA (
    Text.Create(
        TextConfig.Create (x = 12.0, y = 10.0, text = "Group A · 0°", fontSize = 11.0, fontFamily = "monospace", fill = "#111111")
    )
)
layer.add groupA |> ignore

// Group B: rotated 24 degrees.
let private groupB = Group.Create (GroupConfig.Create (x = 340.0, y = 75.0, rotation = 24.0))

addShape groupB (
    Rect.Create(
        RectConfig.Create (width = 150.0, height = 110.0, fill = "#efefef", stroke = "#111111", strokeWidth = 2.0)
    )
)
addShape groupB (
    Rect.Create(
        RectConfig.Create (
            x = 22.0,
            y = 22.0,
            width = 106.0,
            height = 66.0,
            fill = "#ffffff",
            stroke = "#555555",
            strokeWidth = 1.5,
            dash = ResizeArray [ 4.0; 3.0 ]
        )
    )
)
addShape groupB (
    Circle.Create(
        CircleConfig.Create (x = 75.0, y = 55.0, radius = 14.0, fill = "#404040")
    )
)
addShape groupB (
    Text.Create(
        TextConfig.Create (x = 10.0, y = 8.0, text = "Group B · 24°", fontSize = 11.0, fontFamily = "monospace", fill = "#111111")
    )
)
layer.add groupB |> ignore

// Group C: scaled container with nested rotated child group containing marker.
let private groupC = Group.Create (GroupConfig.Create (x = 495.0, y = 55.0, scaleX = 1.15, scaleY = 1.15))

addShape groupC (
    Rect.Create(
        RectConfig.Create (width = 130.0, height = 110.0, fill = "#ffffff", stroke = "#111111", strokeWidth = 2.0)
    )
)
addShape groupC (
    Text.Create(
        TextConfig.Create (x = 8.0, y = 8.0, text = "Group C · 1.15x", fontSize = 10.0, fontFamily = "monospace", fill = "#111111")
    )
)

let private innerGroup = Group.Create (GroupConfig.Create (x = 25.0, y = 25.0, rotation = 15.0))
let private marker = Circle.Create (CircleConfig.Create (x = 40.0, y = 30.0, radius = 16.0, fill = "#111111"))
addShape innerGroup marker
groupC.add innerGroup |> ignore
layer.add groupC |> ignore

let private absPos = marker.getAbsolutePosition ()
let private bbox = groupB.getClientRect ()
let private stripTxt =
    sprintf "Marker abs=(%d, %d) · Group B bbox=%dx%d" (int (round absPos.x)) (int (round absPos.y)) (int (round bbox.width)) (int (round bbox.height))

addShape layer (
    Rect.Create(
        RectConfig.Create (x = 28.0, y = 308.0, width = 624.0, height = 60.0, fill = "#f7f7f7", stroke = "#111111", strokeWidth = 1.5)
    )
)
addShape layer (
    Text.Create(
        TextConfig.Create (x = 44.0, y = 331.0, text = stripTxt, fontSize = 12.0, fontFamily = "monospace", fill = "#111111")
    )
)

layer.draw () |> ignore

let private meta = (document.getElementById "meta").Value
meta.textContent <- sprintf "Stage: 680x400 · Nested marker abs=(%d, %d) · Rotated Group B rect=%dx%d" (int (round absPos.x)) (int (round absPos.y)) (int (round bbox.width)) (int (round bbox.height))
