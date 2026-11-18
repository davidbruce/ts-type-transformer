// PixiJS (v8) from F#, in a page: points, rectangles and matrices, a scene
// graph of containers with bounds, a Graphics shape and a Sprite. Nothing is
// rendered. Every check must hold.
module Test

open Fable.Core
open Fable.Core.JsInterop
open Bindings
open Bindings.PixiJs

let mutable passed = 0

let check (ok: bool) (what: string) =
    if ok then passed <- passed + 1 else failwithf "check failed: %s" what

let near (a: float) (b: float) = abs (a - b) < 1e-6

// Points.
let p = Point.Create (3.0, 4.0)
check (p.x = 3.0 && p.y = 4.0) "Point"
let q = p.clone ()
q.set (10.0, 20.0) |> ignore
check (p.x = 3.0 && q.x = 10.0 && q.y = 20.0) "clone and set"
check (p.equals (Point.Create (3.0, 4.0))) "equals"
check (Point.Create().x = 0.0) "the default point"

// Rectangles.
let r = Rectangle.Create (NumberOrString.Number 10.0, NumberOrString.Number 20.0, NumberOrString.Number 100.0, NumberOrString.Number 50.0)
check (r.x = 10.0 && r.y = 20.0 && r.width = 100.0 && r.height = 50.0) "Rectangle"
check (r.left = 10.0 && r.right = 110.0 && r.top = 20.0 && r.bottom = 70.0) "edges"
check (r.contains (50.0, 40.0)) "contains"
check (not (r.contains (5.0, 40.0))) "does not contain"
let other = Rectangle.Create (NumberOrString.Number 100.0, NumberOrString.Number 60.0, NumberOrString.Number 50.0, NumberOrString.Number 50.0)
check (r.intersects other) "intersects"
check (not (r.isEmpty ())) "not empty"
let padded = r.clone().pad (5.0, 10.0)
check (padded.x = 5.0 && padded.width = 110.0 && padded.height = 70.0) "pad"

// Matrices: 2D transforms.
let m = Matrix.Create ()
check (m.isIdentity ()) "identity"
m.translate(10.0, 20.0).scale (2.0, 3.0) |> ignore
check (m.a = 2.0 && m.d = 3.0 && m.tx = 20.0 && m.ty = 60.0) "translate then scale"
let moved = m.apply<Point> (Point.Create (1.0, 1.0))
check (moved.x = 22.0 && moved.y = 63.0) "apply"
let inv = m.clone().invert ()
let back = inv.apply<Point> (moved)
check (near back.x 1.0 && near back.y 1.0) "invert and apply back"
let rot = Matrix.Create().rotate (System.Math.PI / 2.0)
let turned = rot.apply<Point> (Point.Create (1.0, 0.0))
check (near turned.x 0.0 && near turned.y 1.0) "rotate"
check (m.equals (m.clone ())) "matrix equals"

// A scene graph: children, positions, transforms.
let stage = Container.Create<obj> ()
let box = Container.Create<obj> ()
box.position.set (100.0, 50.0) |> ignore
stage.addChild box |> ignore
check (stage.children.Count = 1) "addChild"
check (obj.ReferenceEquals (box.parent, stage)) "parent"
box.x <- 120.0
check (box.position.x = 120.0 && box.y = 50.0) "x and position"
box.scale.set (2.0, 2.0) |> ignore
box.rotation <- 0.5
check (box.rotation = 0.5 && near box.angle (0.5 * 180.0 / System.Math.PI)) "rotation and angle"
box.alpha <- 0.5
box.visible <- false
check (box.alpha = 0.5 && not box.visible) "alpha and visible"
box.visible <- true
check (box.worldTransform <> null) "worldTransform"

// Graphics: a drawn rectangle has bounds.
let g = Graphics.Create ()
g.rect(0.0, 0.0, 100.0, 50.0).fill (U9.Case1 "red") |> ignore
let gb = g.getLocalBounds ()
check (gb.width = 100.0 && gb.height = 50.0) "Graphics bounds"
g.position.set (10.0, 10.0) |> ignore
stage.addChild g |> ignore
check (stage.children.Count = 2) "two children"

// Bounds in the parent's space: the graphics at (10, 10), 100 by 50.
let gp = g.getBounds ()
check (gp.x = 10.0 && gp.y = 10.0) "getBounds"
check (near gp.width 100.0 && near gp.height 50.0) "getBounds size"

// A Sprite with the white texture: 1 by 1, scaled by its size.
let sprite = Sprite.Create (SpriteOptionsOrTexture.Texture (unbox Texture.WHITE)) // FINDING 22: Texture.WHITE is a Texture<BufferImageSource>, not a Texture<TextureSource<obj>>
check (sprite.width = 1.0 && sprite.height = 1.0) "the white texture is 1 by 1"
sprite.width <- 40.0
sprite.height <- 30.0
sprite.position.set (200.0, 100.0) |> ignore
sprite.anchor.set (0.5, 0.5) |> ignore
check (sprite.width = 40.0 && sprite.height = 30.0) "sprite size"
check (sprite.anchor.x = 0.5) "anchor"
stage.addChild sprite |> ignore
let sb = sprite.getBounds ()
check (near sb.x 180.0 && near sb.y 85.0 && near sb.width 40.0 && near sb.height 30.0) "the sprite's bounds follow its anchor"

// The stage's bounds hold all its children.
let all = stage.getBounds ()
check (all.x <= 10.0 && all.right >= 220.0 && all.bottom >= 115.0) "the stage's bounds hold its children"

// Children: indexes and removal.
check (obj.ReferenceEquals (stage.getChildAt 2, sprite)) "getChildAt"
check (stage.getChildIndex (box) = 0.0) "getChildIndex"
stage.removeChild sprite |> ignore
check (stage.children.Count = 2 && isNull sprite.parent) "removeChild"
stage.removeChildren () |> ignore
check (stage.children.Count = 0) "removeChildren"

// FINDING 23: a Container is an EventEmitter in PixiJS, but `EventEmitter<_,_>` is an empty interface (Bindings.Prelude.fs:159):
// no `on`, `emit` or `off` on any display object.

box.destroy ()
check box.destroyed "destroy"

// An Application: its renderer is made asynchronously (`init`) with named options.
async {
    let app = Application.Create<obj> ()
    do! Async.AwaitPromise (app.init (InitOptions.Create (width = 200.0, height = 100.0)))
    check (not (isNull app.canvas)) "the application's canvas"
    check (app.screen.width = 200.0 && app.screen.height = 100.0) "the screen has the requested size"
    app.stage.addChild stage |> ignore
    app.render ()
    check (app.stage.children.Count = 1) "the stage holds a container"
    app.destroy ()
    printfn "%d checks passed" passed
}
|> Async.StartImmediate
