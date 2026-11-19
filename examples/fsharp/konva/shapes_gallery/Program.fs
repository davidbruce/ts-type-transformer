module Program

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

let private addCaption x y caption =
    let tc = createEmpty<TextConfig>
    tc.x <- Some x
    tc.y <- Some y
    tc.text <- Some caption
    tc.fontSize <- Some 11.0
    tc.fontFamily <- Some "monospace"
    tc.fill <- Some (StringOrCanvasGradient.String "#555555")
    addShape (Text.Create tc)

// Full monochrome canvas backing so pixel inspection sees >= 2 distinct colors.
let private bgCfg = createEmpty<RectConfig>
bgCfg.x <- Some 0.0
bgCfg.y <- Some 0.0
bgCfg.width <- Some 680.0
bgCfg.height <- Some 400.0
bgCfg.fill <- Some (StringOrCanvasGradient.String "#ffffff")
addShape (Rect.Create bgCfg)

// Row 1: Rounded filled Rect, dashed Rect, Circle, Hexagon.
let private r1 = createEmpty<RectConfig>
r1.x <- Some 40.0
r1.y <- Some 55.0
r1.width <- Some 110.0
r1.height <- Some 76.0
r1.cornerRadius <- Some (NumberOrNumberArray.Number 8.0)
r1.fill <- Some (StringOrCanvasGradient.String "#111111")
addShape (Rect.Create r1)
addCaption 40.0 142.0 "Rect (filled)"

let private r2 = createEmpty<RectConfig>
r2.x <- Some 195.0
r2.y <- Some 55.0
r2.width <- Some 110.0
r2.height <- Some 76.0
r2.fill <- Some (StringOrCanvasGradient.String "#ffffff")
r2.stroke <- Some (StringOrCanvasGradient.String "#111111")
r2.strokeWidth <- Some 2.0
r2.dash <- Some (ResizeArray [ 6.0; 4.0 ])
addShape (Rect.Create r2)
addCaption 195.0 142.0 "Rect (dashed)"

let private c1 = createEmpty<CircleConfig>
c1.x <- Some 415.0
c1.y <- Some 93.0
c1.radius <- Some 40.0
c1.fill <- Some (StringOrCanvasGradient.String "#404040")
c1.stroke <- Some (StringOrCanvasGradient.String "#111111")
c1.strokeWidth <- Some 2.0
addShape (Circle.Create c1)
addCaption 395.0 142.0 "Circle"

let private hexCfg = createEmpty<RegularPolygonConfig>
hexCfg.x <- Some 575.0
hexCfg.y <- Some 93.0
hexCfg.sides <- 6.0
hexCfg.radius <- 42.0
hexCfg.fill <- Some (StringOrCanvasGradient.String "#efefef")
hexCfg.stroke <- Some (StringOrCanvasGradient.String "#111111")
hexCfg.strokeWidth <- Some 2.0
addShape (RegularPolygon.Create hexCfg)
addCaption 548.0 142.0 "Hexagon"

// Row 2: Star, Ring, Wedge, Framed Text.
let private starCfg = createEmpty<StarConfig>
starCfg.x <- Some 95.0
starCfg.y <- Some 268.0
starCfg.numPoints <- 5.0
starCfg.innerRadius <- 19.0
starCfg.outerRadius <- 44.0
starCfg.fill <- Some (StringOrCanvasGradient.String "#737373")
starCfg.stroke <- Some (StringOrCanvasGradient.String "#111111")
starCfg.strokeWidth <- Some 2.0
addShape (Star.Create starCfg)
addCaption 62.0 325.0 "Star (5pt)"

let private ringCfg = createEmpty<RingConfig>
ringCfg.x <- Some 250.0
ringCfg.y <- Some 268.0
ringCfg.innerRadius <- 22.0
ringCfg.outerRadius <- 42.0
ringCfg.fill <- Some (StringOrCanvasGradient.String "#d4d4d4")
ringCfg.stroke <- Some (StringOrCanvasGradient.String "#111111")
ringCfg.strokeWidth <- Some 2.0
addShape (Ring.Create ringCfg)
addCaption 236.0 325.0 "Ring"

let private wedgeCfg = createEmpty<WedgeConfig>
wedgeCfg.x <- Some 415.0
wedgeCfg.y <- Some 268.0
wedgeCfg.radius <- 44.0
wedgeCfg.angle <- 270.0
wedgeCfg.rotation <- Some -90.0
wedgeCfg.fill <- Some (StringOrCanvasGradient.String "#111111")
wedgeCfg.stroke <- Some (StringOrCanvasGradient.String "#555555")
wedgeCfg.strokeWidth <- Some 1.0
addShape (Wedge.Create wedgeCfg)
addCaption 382.0 325.0 "Wedge 270°"

let private tfCfg = createEmpty<RectConfig>
tfCfg.x <- Some 500.0
tfCfg.y <- Some 224.0
tfCfg.width <- Some 150.0
tfCfg.height <- Some 88.0
tfCfg.fill <- Some (StringOrCanvasGradient.String "#f7f7f7")
tfCfg.stroke <- Some (StringOrCanvasGradient.String "#111111")
tfCfg.strokeWidth <- Some 1.5
addShape (Rect.Create tfCfg)

let private txtCfg = createEmpty<TextConfig>
txtCfg.x <- Some 514.0
txtCfg.y <- Some 240.0
txtCfg.text <- Some "KONVA 2D\nMonochrome\nScene Graph"
txtCfg.fontSize <- Some 13.0
txtCfg.fontFamily <- Some "monospace"
txtCfg.fill <- Some (StringOrCanvasGradient.String "#111111")
addShape (Text.Create txtCfg)
addCaption 558.0 325.0 "Text"

layer.draw () |> ignore

let private meta = document.getElementById "meta"
meta.textContent <- sprintf "Stage: 680x400 · 7 primitive types · %d layer children" (layer.getChildren().Count)
