module Demos.Konva.ShapesGallery

open Fable.Core.JsInterop
open Dom
open type Dom.Globals
open Konva

let private stage =
    Stage.Create (StageConfig.Create (container = "stage", width = 680.0, height = 400.0))

let private layer = Layer.Create ()
stage.add layer |> ignore

let private addShape (s: Shape) =
    layer.add s |> ignore

let private addCaption x y caption =
    addShape (
        Text.Create(
            TextConfig.Create (
                x = x,
                y = y,
                text = caption,
                fontSize = 11.0,
                fontFamily = "monospace",
                fill = "#555555"
            )
        )
    )

// Full monochrome canvas backing so pixel inspection sees >= 2 distinct colors.
addShape (
    Rect.Create(
        RectConfig.Create (x = 0.0, y = 0.0, width = 680.0, height = 400.0, fill = "#ffffff")
    )
)

// Row 1: Rounded filled Rect, dashed Rect, Circle, Hexagon.
addShape (
    Rect.Create(
        RectConfig.Create (
            x = 40.0,
            y = 55.0,
            width = 110.0,
            height = 76.0,
            cornerRadius = 8.0,
            fill = "#111111"
        )
    )
)
addCaption 40.0 142.0 "Rect (filled)"

addShape (
    Rect.Create(
        RectConfig.Create (
            x = 195.0,
            y = 55.0,
            width = 110.0,
            height = 76.0,
            fill = "#ffffff",
            stroke = "#111111",
            strokeWidth = 2.0,
            dash = ResizeArray [ 6.0; 4.0 ]
        )
    )
)
addCaption 195.0 142.0 "Rect (dashed)"

addShape (
    Circle.Create(
        CircleConfig.Create (
            x = 415.0,
            y = 93.0,
            radius = 40.0,
            fill = "#404040",
            stroke = "#111111",
            strokeWidth = 2.0
        )
    )
)
addCaption 395.0 142.0 "Circle"

addShape (
    RegularPolygon.Create(
        RegularPolygonConfig.Create (
            sides = 6.0,
            radius = 42.0,
            x = 575.0,
            y = 93.0,
            fill = "#efefef",
            stroke = "#111111",
            strokeWidth = 2.0
        )
    )
)
addCaption 548.0 142.0 "Hexagon"

// Row 2: Star, Ring, Wedge, Framed Text.
addShape (
    Star.Create(
        StarConfig.Create (
            numPoints = 5.0,
            innerRadius = 19.0,
            outerRadius = 44.0,
            x = 95.0,
            y = 268.0,
            fill = "#737373",
            stroke = "#111111",
            strokeWidth = 2.0
        )
    )
)
addCaption 62.0 325.0 "Star (5pt)"

addShape (
    Ring.Create(
        RingConfig.Create (
            innerRadius = 22.0,
            outerRadius = 42.0,
            x = 250.0,
            y = 268.0,
            fill = "#d4d4d4",
            stroke = "#111111",
            strokeWidth = 2.0
        )
    )
)
addCaption 236.0 325.0 "Ring"

addShape (
    Wedge.Create(
        WedgeConfig.Create (
            angle = 270.0,
            radius = 44.0,
            x = 415.0,
            y = 268.0,
            rotation = -90.0,
            fill = "#111111",
            stroke = "#555555",
            strokeWidth = 1.0
        )
    )
)
addCaption 382.0 325.0 "Wedge 270°"

addShape (
    Rect.Create(
        RectConfig.Create (
            x = 500.0,
            y = 224.0,
            width = 150.0,
            height = 88.0,
            fill = "#f7f7f7",
            stroke = "#111111",
            strokeWidth = 1.5
        )
    )
)

addShape (
    Text.Create(
        TextConfig.Create (
            x = 514.0,
            y = 240.0,
            text = "KONVA 2D\nMonochrome\nScene Graph",
            fontSize = 13.0,
            fontFamily = "monospace",
            fill = "#111111"
        )
    )
)
addCaption 558.0 325.0 "Text"

layer.draw () |> ignore

let private meta = (document.getElementById "meta").Value
meta.textContent <- sprintf "Stage: 680x400 · 7 primitive types · %d layer children" (layer.getChildren().Count)
