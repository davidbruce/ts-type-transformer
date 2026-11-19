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

let private bgLayer = Layer.Create ()
let private fgLayer = Layer.Create ()
stage.add (bgLayer, box fgLayer) |> ignore

let private addTo (l: Layer) (s: Shape<'C>) =
    l.add (GroupOrShape.Shape (unbox<Shape<ShapeConfig>> s)) |> ignore

// Layer 1: Blueprint Grid (distinct colors: #ffffff, #e5e5e5, #b5b5b5, #737373).
let private bgRect = createEmpty<RectConfig>
bgRect.x <- Some 0.0
bgRect.y <- Some 0.0
bgRect.width <- Some 680.0
bgRect.height <- Some 400.0
bgRect.fill <- Some (StringOrCanvasGradient.String "#ffffff")
addTo bgLayer (Rect.Create bgRect)

for x in 40 .. 40 .. 640 do
    let lc = createEmpty<LineConfig>
    lc.points <- Some (LinePoints.Array (ResizeArray [ float x; 20.0; float x; 380.0 ]))
    lc.stroke <- Some (StringOrCanvasGradient.String "#e5e5e5")
    lc.strokeWidth <- Some 1.0
    addTo bgLayer (Line.Create lc)

for y in 40 .. 40 .. 360 do
    let lc = createEmpty<LineConfig>
    lc.points <- Some (LinePoints.Array (ResizeArray [ 20.0; float y; 660.0; float y ]))
    lc.stroke <- Some (StringOrCanvasGradient.String "#e5e5e5")
    lc.strokeWidth <- Some 1.0
    addTo bgLayer (Line.Create lc)

let private frameCfg = createEmpty<RectConfig>
frameCfg.x <- Some 20.0
frameCfg.y <- Some 20.0
frameCfg.width <- Some 640.0
frameCfg.height <- Some 360.0
frameCfg.stroke <- Some (StringOrCanvasGradient.String "#b5b5b5")
frameCfg.strokeWidth <- Some 1.5
frameCfg.dash <- Some (ResizeArray [ 6.0; 4.0 ])
addTo bgLayer (Rect.Create frameCfg)

let private l1Txt = createEmpty<TextConfig>
l1Txt.x <- Some 32.0
l1Txt.y <- Some 32.0
l1Txt.text <- Some "Layer 1: Blueprint Grid (z=0)"
l1Txt.fontSize <- Some 12.0
l1Txt.fontFamily <- Some "monospace"
l1Txt.fill <- Some (StringOrCanvasGradient.String "#737373")
addTo bgLayer (Text.Create l1Txt)

// Layer 2: Overlapping interactive cards & badge.
let private makeCard idVal x y fillHex =
    let rc = createEmpty<RectConfig>
    rc.id <- Some idVal
    rc.name <- Some "card"
    rc.x <- Some x
    rc.y <- Some y
    rc.width <- Some 200.0
    rc.height <- Some 130.0
    rc.fill <- Some (StringOrCanvasGradient.String fillHex)
    rc.stroke <- Some (StringOrCanvasGradient.String "#111111")
    rc.strokeWidth <- Some 2.0
    let r = Rect.Create rc
    addTo fgLayer r
    r

let private cardA = makeCard "card-a" 110.0 80.0 "#efefef"
let private _cardB = makeCard "card-b" 210.0 125.0 "#404040"
let private _cardC = makeCard "card-c" 310.0 170.0 "#ffffff"

let private badgeCfg = createEmpty<CircleConfig>
badgeCfg.id <- Some "badge"
badgeCfg.x <- Some 555.0
badgeCfg.y <- Some 135.0
badgeCfg.radius <- Some 38.0
badgeCfg.fill <- Some (StringOrCanvasGradient.String "#111111")
addTo fgLayer (Circle.Create badgeCfg)

cardA.moveToTop () |> ignore
let private cardAZ = int (cardA.getZIndex ())

let mutable clicks = 0
cardA.on_click (fun _ -> clicks <- clicks + 1) |> ignore
cardA.fire "click" |> ignore
cardA.fire "click" |> ignore

bgLayer.draw () |> ignore
fgLayer.draw () |> ignore

let private pt = createEmpty<Vector2d>
pt.x <- 250.0
pt.y <- 160.0
let private hit = stage.getIntersection pt
let private hitId = if isNull hit then "none" else hit.id.Invoke ()

// Crosshair marker at (250, 160) & foreground label.
let private dotCfg = createEmpty<CircleConfig>
dotCfg.x <- Some 250.0
dotCfg.y <- Some 160.0
dotCfg.radius <- Some 7.0
dotCfg.fill <- Some (StringOrCanvasGradient.String "#ffffff")
dotCfg.stroke <- Some (StringOrCanvasGradient.String "#111111")
dotCfg.strokeWidth <- Some 2.0
addTo fgLayer (Circle.Create dotCfg)

for (pts: float list) in [ [ 238.0; 160.0; 262.0; 160.0 ]; [ 250.0; 148.0; 250.0; 172.0 ] ] do
    let lc = createEmpty<LineConfig>
    lc.points <- Some (LinePoints.Array (ResizeArray pts))
    lc.stroke <- Some (StringOrCanvasGradient.String "#111111")
    lc.strokeWidth <- Some 1.5
    addTo fgLayer (Line.Create lc)

let private l2Txt = createEmpty<TextConfig>
l2Txt.x <- Some 32.0
l2Txt.y <- Some 354.0
l2Txt.text <- Some "Layer 2: Elevated Stack (z=1) · Probe (250,160)"
l2Txt.fontSize <- Some 12.0
l2Txt.fontFamily <- Some "monospace"
l2Txt.fill <- Some (StringOrCanvasGradient.String "#111111")
addTo fgLayer (Text.Create l2Txt)

fgLayer.draw () |> ignore

let private meta = document.getElementById "meta"
meta.textContent <- sprintf "Layers: %d · card-a zIndex=%d · hit@(250,160)=%s · clicks=%d" (stage.getLayers().Count) cardAZ hitId clicks
