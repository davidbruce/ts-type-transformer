module Demos.Konva.WireframeDiagram

open Fable.Core.JsInterop
open Dom
open type Dom.Globals
open Konva

let private stage =
    Stage.Create (StageConfig.Create (container = "stage", width = 680.0, height = 400.0))

let private layer = Layer.Create ()
stage.add layer |> ignore

let private addShape (target: Container<GroupOrShape, _>) (s: Shape<ShapeConfig>) =
    target.add s |> ignore

let private addLine (target: Container<GroupOrShape, _>) (l: Line<'C>) =
    target.add (l.asShape ()) |> ignore

addShape layer (
    Rect.Create(
        RectConfig.Create (x = 0.0, y = 0.0, width = 680.0, height = 400.0, fill = "#ffffff")
    ).asShape ()
)

let private addStageBox x title subtitle fillHex inkHex =
    let g = Group.Create (GroupConfig.Create (x = x, y = 95.0))

    addShape g (
        Rect.Create(
            RectConfig.Create (
                width = 126.0,
                height = 68.0,
                cornerRadius = 3.0,
                fill = !^fillHex,
                stroke = "#111111",
                strokeWidth = 2.0
            )
        ).asShape ()
    )

    addShape g (
        Text.Create(
            TextConfig.Create (
                x = 12.0,
                y = 16.0,
                text = title,
                fontSize = 12.0,
                fontStyle = "bold",
                fontFamily = "monospace",
                fill = !^inkHex
            )
        ).asShape ()
    )

    addShape g (
        Text.Create(
            TextConfig.Create (
                x = 12.0,
                y = 38.0,
                text = subtitle,
                fontSize = 11.0,
                fontFamily = "monospace",
                fill = !^inkHex
            )
        ).asShape ()
    )

    layer.add g |> ignore

addStageBox 28.0 "01 PARSE" ".d.ts AST" "#ffffff" "#111111"
addStageBox 194.0 "02 LOWER" "Typed IR" "#f7f7f7" "#111111"
addStageBox 360.0 "03 EMIT" "6 Targets" "#efefef" "#111111"
addStageBox 526.0 "04 RUN" "WASM / JS" "#111111" "#ffffff"

let private addArrow (pts: float list) =
    addLine layer (
        Arrow.Create (
            ArrowConfig.Create (
                points = !^(ResizeArray pts),
                pointerLength = 8.0,
                pointerWidth = 8.0,
                fill = "#111111",
                stroke = "#111111",
                strokeWidth = 2.0
            )
        )
    )

addArrow [ 154.0; 129.0; 190.0; 129.0 ]
addArrow [ 320.0; 129.0; 356.0; 129.0 ]
addArrow [ 486.0; 129.0; 522.0; 129.0 ]
addArrow [ 257.0; 163.0; 257.0; 251.0 ]
addArrow [ 423.0; 163.0; 423.0; 251.0 ]

addLine layer (
    Line.Create (
        LineConfig.Create (
            points = !^(ResizeArray [ 589.0; 163.0; 589.0; 212.0; 91.0; 212.0; 91.0; 168.0 ]),
            stroke = "#737373",
            strokeWidth = 1.5,
            dash = ResizeArray [ 6.0; 4.0 ]
        )
    )
)

addShape layer (
    Rect.Create(
        RectConfig.Create (
            x = 28.0,
            y = 255.0,
            width = 624.0,
            height = 58.0,
            cornerRadius = 3.0,
            fill = "#f7f7f7",
            stroke = "#111111",
            strokeWidth = 1.5
        )
    ).asShape ()
)

addShape layer (
    Text.Create(
        TextConfig.Create (
            x = 180.0,
            y = 278.0,
            text = "Shared DOM & Package Handle Runtime",
            fontSize = 12.0,
            fontFamily = "monospace",
            fill = "#111111"
        )
    ).asShape ()
)

layer.draw () |> ignore

let private rects = layer.find<Node<NodeConfig>> "Rect"
let private arrows = layer.find<Node<NodeConfig>> "Arrow"
let private jsonLen = layer.toJSON().Length

let private meta = (document.getElementById "meta").Value
meta.textContent <- sprintf "Pipeline diagram · %d rects · %d arrows · JSON %d bytes" rects.Count arrows.Count jsonLen
