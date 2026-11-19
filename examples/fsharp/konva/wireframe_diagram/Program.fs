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

let private bgCfg = createEmpty<RectConfig>
bgCfg.x <- Some 0.0
bgCfg.y <- Some 0.0
bgCfg.width <- Some 680.0
bgCfg.height <- Some 400.0
bgCfg.fill <- Some (StringOrCanvasGradient.String "#ffffff")
addShape (Rect.Create bgCfg)

let private addStageBox x title subtitle fillHex inkHex =
    let gc = createEmpty<GroupConfig>
    gc.x <- Some x
    gc.y <- Some 95.0
    let g = Group.Create gc

    let rc = createEmpty<RectConfig>
    rc.width <- Some 126.0
    rc.height <- Some 68.0
    rc.cornerRadius <- Some (NumberOrNumberArray.Number 3.0)
    rc.fill <- Some (StringOrCanvasGradient.String fillHex)
    rc.stroke <- Some (StringOrCanvasGradient.String "#111111")
    rc.strokeWidth <- Some 2.0
    g.add (GroupOrShape.Shape (unbox<Shape<ShapeConfig>> (Rect.Create rc))) |> ignore

    let t1 = createEmpty<TextConfig>
    t1.x <- Some 12.0
    t1.y <- Some 16.0
    t1.text <- Some title
    t1.fontSize <- Some 12.0
    t1.fontStyle <- Some "bold"
    t1.fontFamily <- Some "monospace"
    t1.fill <- Some (StringOrCanvasGradient.String inkHex)
    g.add (GroupOrShape.Shape (unbox<Shape<ShapeConfig>> (Text.Create t1))) |> ignore

    let t2 = createEmpty<TextConfig>
    t2.x <- Some 12.0
    t2.y <- Some 38.0
    t2.text <- Some subtitle
    t2.fontSize <- Some 11.0
    t2.fontFamily <- Some "monospace"
    t2.fill <- Some (StringOrCanvasGradient.String inkHex)
    g.add (GroupOrShape.Shape (unbox<Shape<ShapeConfig>> (Text.Create t2))) |> ignore

    layer.add (GroupOrShape.Group g) |> ignore

addStageBox 28.0 "01 PARSE" ".d.ts AST" "#ffffff" "#111111"
addStageBox 194.0 "02 LOWER" "Typed IR" "#f7f7f7" "#111111"
addStageBox 360.0 "03 EMIT" "6 Targets" "#efefef" "#111111"
addStageBox 526.0 "04 RUN" "WASM / JS" "#111111" "#ffffff"

let private addArrow (pts: float list) =
    let ac = createEmpty<ArrowConfig>
    ac.points <- Some (LinePoints.Array (ResizeArray pts))
    ac.pointerLength <- Some 8.0
    ac.pointerWidth <- Some 8.0
    ac.fill <- Some (StringOrCanvasGradient.String "#111111")
    ac.stroke <- Some (StringOrCanvasGradient.String "#111111")
    ac.strokeWidth <- Some 2.0
    addShape (Arrow.Create ac)

addArrow [ 154.0; 129.0; 190.0; 129.0 ]
addArrow [ 320.0; 129.0; 356.0; 129.0 ]
addArrow [ 486.0; 129.0; 522.0; 129.0 ]
addArrow [ 257.0; 163.0; 257.0; 251.0 ]
addArrow [ 423.0; 163.0; 423.0; 251.0 ]

let private fbCfg = createEmpty<LineConfig>
fbCfg.points <- Some (LinePoints.Array (ResizeArray [ 589.0; 163.0; 589.0; 212.0; 91.0; 212.0; 91.0; 168.0 ]))
fbCfg.stroke <- Some (StringOrCanvasGradient.String "#737373")
fbCfg.strokeWidth <- Some 1.5
fbCfg.dash <- Some (ResizeArray [ 6.0; 4.0 ])
addShape (Line.Create fbCfg)

let private barCfg = createEmpty<RectConfig>
barCfg.x <- Some 28.0
barCfg.y <- Some 255.0
barCfg.width <- Some 624.0
barCfg.height <- Some 58.0
barCfg.cornerRadius <- Some (NumberOrNumberArray.Number 3.0)
barCfg.fill <- Some (StringOrCanvasGradient.String "#f7f7f7")
barCfg.stroke <- Some (StringOrCanvasGradient.String "#111111")
barCfg.strokeWidth <- Some 1.5
addShape (Rect.Create barCfg)

let private barTxt = createEmpty<TextConfig>
barTxt.x <- Some 180.0
barTxt.y <- Some 278.0
barTxt.text <- Some "Shared DOM & Package Handle Runtime"
barTxt.fontSize <- Some 12.0
barTxt.fontFamily <- Some "monospace"
barTxt.fill <- Some (StringOrCanvasGradient.String "#111111")
addShape (Text.Create barTxt)

layer.draw () |> ignore

let private rects = layer.find<Node<NodeConfig>> (box "Rect")
let private arrows = layer.find<Node<NodeConfig>> (box "Arrow")
let private jsonLen = layer.toJSON().Length

let private meta = document.getElementById "meta"
meta.textContent <- sprintf "Pipeline diagram · %d rects · %d arrows · JSON %d bytes" rects.Count arrows.Count jsonLen
