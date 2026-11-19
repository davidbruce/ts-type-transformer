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

let private addShapeToLayer (s: Shape<'C>) =
    layer.add (GroupOrShape.Shape (unbox<Shape<ShapeConfig>> s)) |> ignore

let private bgCfg = createEmpty<RectConfig>
bgCfg.x <- Some 0.0
bgCfg.y <- Some 0.0
bgCfg.width <- Some 680.0
bgCfg.height <- Some 400.0
bgCfg.fill <- Some (StringOrCanvasGradient.String "#ffffff")
addShapeToLayer (Rect.Create bgCfg)

for x in 40 .. 40 .. 640 do
    let lc = createEmpty<LineConfig>
    lc.points <- Some (LinePoints.Array (ResizeArray [ float x; 0.0; float x; 400.0 ]))
    lc.stroke <- Some (StringOrCanvasGradient.String "#e5e5e5")
    lc.strokeWidth <- Some 1.0
    addShapeToLayer (Line.Create lc)

for y in 40 .. 40 .. 360 do
    let lc = createEmpty<LineConfig>
    lc.points <- Some (LinePoints.Array (ResizeArray [ 0.0; float y; 680.0; float y ]))
    lc.stroke <- Some (StringOrCanvasGradient.String "#e5e5e5")
    lc.strokeWidth <- Some 1.0
    addShapeToLayer (Line.Create lc)

// Group A: unrotated base frame.
let private gaCfg = createEmpty<GroupConfig>
gaCfg.x <- Some 40.0
gaCfg.y <- Some 55.0
let private groupA = Group.Create gaCfg

let private gaBox = createEmpty<RectConfig>
gaBox.width <- Some 160.0
gaBox.height <- Some 120.0
gaBox.fill <- Some (StringOrCanvasGradient.String "#f7f7f7")
gaBox.stroke <- Some (StringOrCanvasGradient.String "#111111")
gaBox.strokeWidth <- Some 1.5
groupA.add (GroupOrShape.Shape (unbox<Shape<ShapeConfig>> (Rect.Create gaBox))) |> ignore

let private gaDot = createEmpty<CircleConfig>
gaDot.x <- Some 80.0
gaDot.y <- Some 60.0
gaDot.radius <- Some 24.0
gaDot.fill <- Some (StringOrCanvasGradient.String "#111111")
groupA.add (GroupOrShape.Shape (unbox<Shape<ShapeConfig>> (Circle.Create gaDot))) |> ignore

let private gaLbl = createEmpty<TextConfig>
gaLbl.x <- Some 12.0
gaLbl.y <- Some 10.0
gaLbl.text <- Some "Group A · 0°"
gaLbl.fontSize <- Some 11.0
gaLbl.fontFamily <- Some "monospace"
gaLbl.fill <- Some (StringOrCanvasGradient.String "#111111")
groupA.add (GroupOrShape.Shape (unbox<Shape<ShapeConfig>> (Text.Create gaLbl))) |> ignore
layer.add (GroupOrShape.Group groupA) |> ignore

// Group B: rotated 24 degrees.
let private gbCfg = createEmpty<GroupConfig>
gbCfg.x <- Some 340.0
gbCfg.y <- Some 75.0
gbCfg.rotation <- Some 24.0
let private groupB = Group.Create gbCfg

let private gbOuter = createEmpty<RectConfig>
gbOuter.width <- Some 150.0
gbOuter.height <- Some 110.0
gbOuter.fill <- Some (StringOrCanvasGradient.String "#efefef")
gbOuter.stroke <- Some (StringOrCanvasGradient.String "#111111")
gbOuter.strokeWidth <- Some 2.0
groupB.add (GroupOrShape.Shape (unbox<Shape<ShapeConfig>> (Rect.Create gbOuter))) |> ignore

let private gbInner = createEmpty<RectConfig>
gbInner.x <- Some 22.0
gbInner.y <- Some 22.0
gbInner.width <- Some 106.0
gbInner.height <- Some 66.0
gbInner.fill <- Some (StringOrCanvasGradient.String "#ffffff")
gbInner.stroke <- Some (StringOrCanvasGradient.String "#555555")
gbInner.strokeWidth <- Some 1.5
gbInner.dash <- Some (ResizeArray [ 4.0; 3.0 ])
groupB.add (GroupOrShape.Shape (unbox<Shape<ShapeConfig>> (Rect.Create gbInner))) |> ignore

let private gbDot = createEmpty<CircleConfig>
gbDot.x <- Some 75.0
gbDot.y <- Some 55.0
gbDot.radius <- Some 14.0
gbDot.fill <- Some (StringOrCanvasGradient.String "#404040")
groupB.add (GroupOrShape.Shape (unbox<Shape<ShapeConfig>> (Circle.Create gbDot))) |> ignore

let private gbLbl = createEmpty<TextConfig>
gbLbl.x <- Some 10.0
gbLbl.y <- Some 8.0
gbLbl.text <- Some "Group B · 24°"
gbLbl.fontSize <- Some 11.0
gbLbl.fontFamily <- Some "monospace"
gbLbl.fill <- Some (StringOrCanvasGradient.String "#111111")
groupB.add (GroupOrShape.Shape (unbox<Shape<ShapeConfig>> (Text.Create gbLbl))) |> ignore
layer.add (GroupOrShape.Group groupB) |> ignore

// Group C: scaled container with nested rotated child group containing marker.
let private gcCfg = createEmpty<GroupConfig>
gcCfg.x <- Some 495.0
gcCfg.y <- Some 55.0
gcCfg.scaleX <- Some 1.15
gcCfg.scaleY <- Some 1.15
let private groupC = Group.Create gcCfg

let private gcOuter = createEmpty<RectConfig>
gcOuter.width <- Some 130.0
gcOuter.height <- Some 110.0
gcOuter.fill <- Some (StringOrCanvasGradient.String "#ffffff")
gcOuter.stroke <- Some (StringOrCanvasGradient.String "#111111")
gcOuter.strokeWidth <- Some 2.0
groupC.add (GroupOrShape.Shape (unbox<Shape<ShapeConfig>> (Rect.Create gcOuter))) |> ignore

let private gcLbl = createEmpty<TextConfig>
gcLbl.x <- Some 8.0
gcLbl.y <- Some 8.0
gcLbl.text <- Some "Group C · 1.15x"
gcLbl.fontSize <- Some 10.0
gcLbl.fontFamily <- Some "monospace"
gcLbl.fill <- Some (StringOrCanvasGradient.String "#111111")
groupC.add (GroupOrShape.Shape (unbox<Shape<ShapeConfig>> (Text.Create gcLbl))) |> ignore

let private innerCfg = createEmpty<GroupConfig>
innerCfg.x <- Some 25.0
innerCfg.y <- Some 25.0
innerCfg.rotation <- Some 15.0
let private innerGroup = Group.Create innerCfg

let private markerCfg = createEmpty<CircleConfig>
markerCfg.x <- Some 40.0
markerCfg.y <- Some 30.0
markerCfg.radius <- Some 16.0
markerCfg.fill <- Some (StringOrCanvasGradient.String "#111111")
let private marker = Circle.Create markerCfg
innerGroup.add (GroupOrShape.Shape (unbox<Shape<ShapeConfig>> marker)) |> ignore
groupC.add (GroupOrShape.Group innerGroup) |> ignore
layer.add (GroupOrShape.Group groupC) |> ignore

let private absPos = marker.getAbsolutePosition ()
let private bbox = groupB.getClientRect ()
let private stripTxt =
    sprintf "Marker abs=(%d, %d) · Group B bbox=%dx%d" (int (round absPos.x)) (int (round absPos.y)) (int (round bbox.width)) (int (round bbox.height))

let private stripCfg = createEmpty<RectConfig>
stripCfg.x <- Some 28.0
stripCfg.y <- Some 308.0
stripCfg.width <- Some 624.0
stripCfg.height <- Some 60.0
stripCfg.fill <- Some (StringOrCanvasGradient.String "#f7f7f7")
stripCfg.stroke <- Some (StringOrCanvasGradient.String "#111111")
stripCfg.strokeWidth <- Some 1.5
addShapeToLayer (Rect.Create stripCfg)

let private summaryCfg = createEmpty<TextConfig>
summaryCfg.x <- Some 44.0
summaryCfg.y <- Some 331.0
summaryCfg.text <- Some stripTxt
summaryCfg.fontSize <- Some 12.0
summaryCfg.fontFamily <- Some "monospace"
summaryCfg.fill <- Some (StringOrCanvasGradient.String "#111111")
addShapeToLayer (Text.Create summaryCfg)

layer.draw () |> ignore

let private meta = document.getElementById "meta"
meta.textContent <- sprintf "Stage: 680x400 · Nested marker abs=(%d, %d) · Rotated Group B rect=%dx%d" (int (round absPos.x)) (int (round absPos.y)) (int (round bbox.width)) (int (round bbox.height))
