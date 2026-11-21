module Program

open System
open Fable.Core.JsInterop
open Dom
open type Dom.Globals
open Konva

let private stage =
    Stage.Create (StageConfig.Create (container = !^"stage", width = 680.0, height = 400.0))

let private layer = Layer.Create ()
stage.add layer |> ignore

let private addShape (s: Shape<ShapeConfig>) =
    layer.add (!^s) |> ignore

let private addLine (l: Line<LineConfig>) =
    layer.add (!^(l.asShape ())) |> ignore

addShape (
    Rect.Create(
        RectConfig.Create (x = 0.0, y = 0.0, width = 680.0, height = 400.0, fill = !^"#ffffff")
    ).asShape ()
)

let private cx = 340.0
let private cy = 200.0

for r, strokeCol, fillOpt, sw in [
    (160.0, "#d4d4d4", None, 1.0)
    (125.0, "#a3a3a3", None, 1.0)
    (90.0, "#555555", None, 1.0)
    (52.0, "#111111", Some "#f7f7f7", 1.5)
] do
    let fillVal =
        match fillOpt with
        | Some f -> Some !^f
        | None -> None
    addShape (
        Circle.Create(
            CircleConfig.Create (
                x = cx,
                y = cy,
                radius = r,
                stroke = !^strokeCol,
                strokeWidth = sw,
                ?fill = fillVal
            )
        ).asShape ()
    )

for i in 0 .. 11 do
    let a = float i * Math.PI / 6.0
    addLine (
        Line.Create (
            LineConfig.Create (
                points = !^(ResizeArray [ cx + 36.0 * cos a; cy + 36.0 * sin a; cx + 160.0 * cos a; cy + 160.0 * sin a ]),
                stroke = !^"#d4d4d4",
                strokeWidth = 1.0
            )
        )
    )

for i in 0 .. 23 do
    let a = float i * Math.PI / 12.0
    let r0 = if i % 2 = 0 then 148.0 else 154.0
    addLine (
        Line.Create (
            LineConfig.Create (
                points = !^(ResizeArray [ cx + r0 * cos a; cy + r0 * sin a; cx + 166.0 * cos a; cy + 166.0 * sin a ]),
                stroke = !^"#111111",
                strokeWidth = (if i % 2 = 0 then 2.0 else 1.0)
            )
        )
    )

for i in 0 .. 7 do
    addShape (
        Rect.Create(
            RectConfig.Create (
                x = cx,
                y = cy,
                width = 18.0,
                height = 52.0,
                offsetX = 9.0,
                offsetY = 108.0,
                rotation = float i * 45.0,
                fill = !^(if i % 2 = 0 then "#111111" else "#efefef"),
                stroke = !^"#111111",
                strokeWidth = 1.5
            )
        ).asShape ()
    )

for i in 0 .. 11 do
    let a = float i * Math.PI / 6.0
    addShape (
        Circle.Create(
            CircleConfig.Create (
                x = cx + 125.0 * cos a,
                y = cy + 125.0 * sin a,
                radius = 5.0,
                fill = !^(if i % 2 = 0 then "#111111" else "#ffffff"),
                stroke = !^"#111111",
                strokeWidth = 1.5
            )
        ).asShape ()
    )

addShape (
    RegularPolygon.Create(
        RegularPolygonConfig.Create (sides = 8.0, radius = 26.0, x = cx, y = cy, fill = !^"#111111")
    ).asShape ()
)

addShape (
    Circle.Create(
        CircleConfig.Create (x = cx, y = cy, radius = 9.0, fill = !^"#ffffff")
    ).asShape ()
)

for lx, ly, label in [ (516.0, 194.0, "0°"); (332.0, 376.0, "90°"); (140.0, 194.0, "180°"); (328.0, 14.0, "270°") ] do
    addShape (
        Text.Create(
            TextConfig.Create (
                x = lx,
                y = ly,
                text = label,
                fontSize = 11.0,
                fontFamily = "monospace",
                fill = !^"#555555"
            )
        ).asShape ()
    )

layer.draw () |> ignore

let private meta = (document.getElementById "meta").Value
meta.textContent <- sprintf "Stage: 680x400 · Procedural rosette · %d shapes on layer" (layer.getChildren().Count)
