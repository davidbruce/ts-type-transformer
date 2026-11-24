// Konva from F#, in a page: a Stage on the page's element, a Layer with
// shapes, groups, positions, sizes, lookups, events and serialization.
// Every check must hold.
module Test

open Fable.Core
open Fable.Core.JsInterop
open Bindings
open Bindings.Konva

let mutable passed = 0

let check (ok: bool) (what: string) =
    if ok then passed <- passed + 1 else failwithf "check failed: %s" what

let near (a: float) (b: float) = abs (a - b) < 1e-6

// A stage in the page's #stage, 400 by 300.
let stageConfig = createEmpty<StageConfig_2>
stageConfig.container <- Some(HTMLDivElementOrString.String "stage")
stageConfig.width <- Some 400.0
stageConfig.height <- Some 300.0
let stage = Stage_2.Create stageConfig
check (stage.width.Invoke () = 400.0 && stage.height.Invoke () = 300.0) "stage size"
check (not (isNull (box (stage.getContent ())))) "stage content"

// A layer on the stage, and shapes on the layer.
let layer = Layer_2.Create ()
stage.add (layer) |> ignore
check (stage.getLayers().Count = 1) "stage has one layer"
check (obj.ReferenceEquals(layer.getStage (), stage)) "layer stage"

let rc = createEmpty<RectConfig_2>
rc.x <- Some 20.0
rc.y <- Some 30.0
rc.width <- Some 100.0
rc.height <- Some 50.0
rc.name <- Some "box"
rc.id <- Some "r1"
let rect = Rect_2.Create rc
check (rect.x.Invoke () = 20.0 && rect.y.Invoke () = 30.0) "rect position"
check (rect.width.Invoke () = 100.0 && rect.height.Invoke () = 50.0) "rect size"
check (rect.id.Invoke () = "r1" && rect.name.Invoke () = "box") "rect id and name"
check (rect.getClassName () = "Rect") "rect class name"

let cc = createEmpty<CircleConfig_2>
cc.x <- Some 200.0
cc.y <- Some 100.0
cc.radius <- Some 25.0
let circle = Circle_2.Create cc
check (circle.radius.Invoke () = 25.0 && circle.getWidth () = 50.0 && circle.getHeight () = 50.0) "circle size"

let tc = createEmpty<TextConfig_2>
tc.text <- Some "hello konva"
tc.fontSize <- Some 20.0
tc.x <- Some 10.0
tc.y <- Some 200.0
let text = Text_2.Create tc
check (text.text.Invoke () = "hello konva" && text.fontSize.Invoke () = 20.0) "text content and fontSize"
check (text.getTextWidth () > 0.0 && text.getTextHeight () > 0.0) "text dimensions"
text.setText "hi" |> ignore
check (text.text.Invoke () = "hi") "setText"

// Shapes added to the layer, in order; a group holding one of them.
layer.add<Node_2> (rect, circle) |> ignore
check (layer.getChildren().Count = 2) "layer has two shapes"
let group = Group_2.Create ()
group.setPosition (Vector2d_2.Create (50.0, 10.0)) |> ignore
group.add text |> ignore
layer.add group |> ignore
check (layer.getChildren().Count = 3) "layer has three children"
check (group.getChildren().Count = 1) "group has one child"
check (obj.ReferenceEquals(text.getParent (), group)) "text parent is group"
check (obj.ReferenceEquals(text.getLayer (), layer)) "text layer"
check (rect.getZIndex () = 0.0 && circle.getZIndex () = 1.0 && group.getZIndex () = 2.0) "zIndex order"
layer.draw () |> ignore

// Positions: local, absolute (inside the group), and moved.
let absPos = text.getAbsolutePosition ()
check (absPos.x = 60.0 && absPos.y = 210.0) "absolute position"
check (text.getPosition().x = 10.0) "local position"
rect.setPosition (Vector2d_2.Create (40.0, 30.0)) |> ignore
check (rect.x.Invoke () = 40.0 && rect.getPosition().x = 40.0) "setPosition"
rect.move (Vector2d_2.Create (5.0, 5.0)) |> ignore
check (rect.x.Invoke () = 45.0 && rect.y.Invoke () = 35.0) "move"
rect.x.Invoke (box 60.0) |> ignore
check (rect.x.Invoke () = 60.0) "GetSet setter"
rect.x.Invoke (box 45.0) |> ignore
rect.rotate 90.0 |> ignore
check (near (rect.rotation.Invoke ()) 90.0) "rotate"

// The bounding box on screen, of the circle (centre 200,100, radius 25).
let bbox = circle.getClientRect ()
check (near bbox.x 175.0 && near bbox.y 75.0 && near bbox.width 50.0 && near bbox.height 50.0) "getClientRect"

// Finding nodes: by id, by name, by class.
let found = layer.findOne (StringOrFunction.String "#r1")
check (found.IsSome && found.Value.getClassName () = "Rect") "findOne by id"
check (layer.find(box ".box").Count = 1) "find by name"
check (layer.find(box "Circle").Count = 1) "find by class"
check (layer.find(box "Shape").Count = 3) "find all shapes"
check (layer.findOne(StringOrFunction.String "#nothing").IsNone) "findOne missing"

// Events: a listener on the circle, fired, and removed.
let mutable clicks = 0
circle.on_click (fun _ -> clicks <- clicks + 1) |> ignore
circle.fire "click" |> ignore
circle.fire "click" |> ignore
check (clicks = 2) "on_click fires"
circle.off "click" |> ignore
circle.fire "click" |> ignore
check (clicks = 2) "off removes listener"

// Serialization: a node as JSON, with its class and attributes.
let json = rect.toJSON ()
check (json.Contains "\"className\":\"Rect\"" && json.Contains "\"id\":\"r1\"") "rect toJSON"
check (layer.toJSON().Contains "\"children\"") "layer toJSON"
let objRep = layer.toObject ()
check (objRep.children.IsSome && objRep.children.Value.Count = 3) "layer toObject"

// Hiding, and removal: a hidden node is not visible; destroy takes it from the layer.
circle.hide () |> ignore
check (not (unbox<bool> (circle.isVisible ()))) "hide"
circle.show () |> ignore
check (unbox<bool> (circle.isVisible ())) "show"
circle.destroy () |> ignore
check (layer.getChildren().Count = 2) "destroy removes from layer"

printfn "%d checks passed" passed
