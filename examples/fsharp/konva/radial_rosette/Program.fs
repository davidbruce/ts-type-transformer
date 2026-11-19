module Program

open System
open Fable.Core.JsInterop
open Dom
open type Dom.Globals
open Konva

let private stageCfg = createEmpty<StageConfig>
stageCfg.container <- Some (HTMLDivElementOrString.String "stage")
stageCfg.width <- Some 680.0
stageCfg.height <- Some 400.0
let private stage = Stage.Create stageCfg

let private layer = Layer.Create ()
stage.add layer |> ignore

let private addShape (s: Shape<'C>) =
    layer.add (GroupOrShape.Shape (unbox<Shape<ShapeConfig>> s)) |> ignore

let private bgCfg = createEmpty<RectConfig>
bgCfg.x <- Some 0.0
bgCfg.y <- Some 0.0
bgCfg.width <- Some 680.0
bgCfg.height <- Some 400.0
bgCfg.fill <- Some (StringOrCanvasGradient.String "#ffffff")
addShape (Rect.Create bgCfg)

let private cx = 340.0
let private cy = 200.0

for r, strokeCol, fillOpt, sw in [
    (160.0, "#d4d4d4", None, 1.0)
    (125.0, "#a3a3a3", None, 1.0)
    (90.0, "#555555", None, 1.0)
    (52.0, "#111111", Some "#f7f7f7", 1.5)
] do
    let cc = createEmpty<CircleConfig>
    cc.x <- Some cx
    cc.y <- Some cy
    cc.radius <- Some r
    cc.stroke <- Some (StringOrCanvasGradient.String strokeCol)
    cc.strokeWidth <- Some sw
    match fillOpt with
    | Some f -> cc.fill <- Some (StringOrCanvasGradient.String f)
    | None -> ()
    addShape (Circle.Create cc)

for i in 0 .. 11 do
    let a = float i * Math.PI / 6.0
    let lc = createEmpty<LineConfig>
    lc.points <- Some (LinePoints.Array (ResizeArray [ cx + 36.0 * cos a; cy + 36.0 * sin a; cx + 160.0 * cos a; cy + 160.0 * sin a ]))
    lc.stroke <- Some (StringOrCanvasGradient.String "#d4d4d4")
    lc.strokeWidth <- Some 1.0
    addShape (Line.Create lc)

for i in 0 .. 23 do
    let a = float i * Math.PI / 12.0
    let r0 = if i % 2 = 0 then 148.0 else 154.0
    let lc = createEmpty<LineConfig>
    lc.points <- Some (LinePoints.Array (ResizeArray [ cx + r0 * cos a; cy + r0 * sin a; cx + 166.0 * cos a; cy + 166.0 * sin a ]))
    lc.stroke <- Some (StringOrCanvasGradient.String "#111111")
    lc.strokeWidth <- Some (if i % 2 = 0 then 2.0 else 1.0)
    addShape (Line.Create lc)

for i in 0 .. 7 do
    let pc = createEmpty<RectConfig>
    pc.x <- Some cx
    pc.y <- Some cy
    pc.width <- Some 18.0
    pc.height <- Some 52.0
    pc.offsetX <- Some 9.0
    pc.offsetY <- Some 108.0
    pc.rotation <- Some (float i * 45.0)
    pc.fill <- Some (StringOrCanvasGradient.String (if i % 2 = 0 then "#111111" else "#efefef"))
    pc.stroke <- Some (StringOrCanvasGradient.String "#111111")
    pc.strokeWidth <- Some 1.5
    addShape (Rect.Create pc)

for i in 0 .. 11 do
    let a = float i * Math.PI / 6.0
    let oc = createEmpty<CircleConfig>
    oc.x <- Some (cx + 125.0 * cos a)
    oc.y <- Some (cy + 125.0 * sin a)
    oc.radius <- Some 5.0
    oc.fill <- Some (StringOrCanvasGradient.String (if i % 2 = 0 then "#111111" else "#ffffff"))
    oc.stroke <- Some (StringOrCanvasGradient.String "#111111")
    oc.strokeWidth <- Some 1.5
    addShape (Circle.Create oc)

let private corePoly = createEmpty<RegularPolygonConfig>
corePoly.x <- Some cx
corePoly.y <- Some cy
corePoly.sides <- 8.0
corePoly.radius <- 26.0
corePoly.fill <- Some (StringOrCanvasGradient.String "#111111")
addShape (RegularPolygon.Create corePoly)

let private coreDot = createEmpty<CircleConfig>
coreDot.x <- Some cx
coreDot.y <- Some cy
coreDot.radius <- Some 9.0
coreDot.fill <- Some (StringOrCanvasGradient.String "#ffffff")
addShape (Circle.Create coreDot)

for lx, ly, label in [ (516.0, 194.0, "0°"); (332.0, 376.0, "90°"); (140.0, 194.0, "180°"); (328.0, 14.0, "270°") ] do
    let tc = createEmpty<TextConfig>
    tc.x <- Some lx
    tc.y <- Some ly
    tc.text <- Some label
    tc.fontSize <- Some 11.0
    tc.fontFamily <- Some "monospace"
    tc.fill <- Some (StringOrCanvasGradient.String "#555555")
    addShape (Text.Create tc)

layer.draw () |> ignore

let private meta = document.getElementById "meta"
meta.textContent <- sprintf "Stage: 680x400 · Procedural rosette · %d shapes on layer" (layer.getChildren().Count)
