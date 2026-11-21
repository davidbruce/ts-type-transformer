// A 2D canvas from F#: rectangles, paths, pixels read back and written,
// text measured, state saved and transforms applied. Every check must hold.
module Test

open Fable.Core
open Fable.Core.JsInterop
open Bindings

let mutable passed = 0

let check (ok: bool) (what: string) =
    if ok then passed <- passed + 1 else failwithf "check failed: %s" what

let same (a: obj) (b: obj) = obj.ReferenceEquals(a, b)

let canvas: HTMLCanvasElement = document.createElement HTMLElementTagNameMapKeys.canvas
canvas.width <- 100.0
canvas.height <- 80.0
document.body.appendChild canvas |> ignore
let ctx = (canvas.getContext_2d ()).Value

// One pixel as (r, g, b, a).
let pixel (x: float) (y: float) =
    let d = (ctx.getImageData (x, y, 1.0, 1.0)).data
    int d.[0], int d.[1], int d.[2], int d.[3]

check (canvas.width = 100.0 && canvas.height = 80.0) "canvas size"
check (same ctx.canvas canvas) "context canvas"
check (pixel 10.0 10.0 = (0, 0, 0, 0)) "a new canvas is transparent"

// fillRect with a colour string.
ctx.fillStyle <- "#ff0000"
ctx.fillRect (0.0, 0.0, 50.0, 40.0)
check (pixel 10.0 10.0 = (255, 0, 0, 255)) "fillRect red"
check (pixel 60.0 10.0 = (0, 0, 0, 0)) "outside the rect is clear"
check (match ctx.fillStyle with
       | StringOrCanvasGradientOrCanvasPattern.String s -> s = "#ff0000"
       | _ -> false) "fillStyle reads back normalised"
ctx.fillStyle <- "rgb(0, 0, 255)"
ctx.fillRect (50.0, 0.0, 50.0, 40.0)
check (pixel 70.0 20.0 = (0, 0, 255, 255)) "fillRect blue"
check (match ctx.fillStyle with
       | StringOrCanvasGradientOrCanvasPattern.String s -> s = "#0000ff"
       | _ -> false) "fillStyle rgb becomes hex"

// clearRect.
ctx.clearRect (0.0, 0.0, 10.0, 10.0)
check (pixel 5.0 5.0 = (0, 0, 0, 0)) "clearRect"
check (pixel 20.0 20.0 = (255, 0, 0, 255)) "clearRect leaves the rest"

// strokeRect: a 4px black line on the border of a rect.
ctx.strokeStyle <- "#000000"
ctx.lineWidth <- 4.0
check (ctx.lineWidth = 4.0) "lineWidth"
ctx.strokeRect (20.0, 50.0, 40.0, 20.0)
check (pixel 40.0 50.0 = (0, 0, 0, 255)) "stroke on the edge"
check (pixel 40.0 60.0 = (0, 0, 0, 0)) "inside a stroked rect is empty"
ctx.lineWidth <- 1.0

// Alpha.
ctx.clearRect (0.0, 0.0, 100.0, 80.0)
ctx.globalAlpha <- 0.5
ctx.fillStyle <- "#ffffff"
ctx.fillRect (0.0, 0.0, 10.0, 10.0)
let (_, _, _, a) = pixel 5.0 5.0
check (a >= 127 && a <= 128) "globalAlpha"
ctx.globalAlpha <- 1.0
check (ctx.globalAlpha = 1.0) "globalAlpha reset"
ctx.clearRect (0.0, 0.0, 100.0, 80.0)

// Paths: a filled triangle and an arc.
ctx.fillStyle <- "#00ff00"
ctx.beginPath ()
ctx.moveTo (10.0, 70.0)
ctx.lineTo (50.0, 70.0)
ctx.lineTo (30.0, 30.0)
ctx.closePath ()
ctx.fill ()
check (pixel 30.0 60.0 = (0, 255, 0, 255)) "triangle filled"
check (pixel 5.0 30.0 = (0, 0, 0, 0)) "outside the triangle"
check (ctx.isPointInPath (30.0, 60.0)) "isPointInPath inside"
check (not (ctx.isPointInPath (5.0, 30.0))) "isPointInPath outside"
ctx.fillStyle <- "#ff00ff"
ctx.beginPath ()
ctx.arc (75.0, 50.0, 15.0, 0.0, 2.0 * System.Math.PI)
ctx.fill ()
check (pixel 75.0 50.0 = (255, 0, 255, 255)) "arc centre"
check (pixel 75.0 38.0 = (255, 0, 255, 255)) "inside the arc"
check (pixel 75.0 30.0 = (0, 0, 0, 0)) "outside the arc"
check (ctx.isPointInPath (75.0, 40.0)) "point in the circle"
ctx.beginPath ()
ctx.rect (0.0, 0.0, 5.0, 5.0)
check (ctx.isPointInPath (2.0, 2.0)) "rect path"

// Gradients.
ctx.clearRect (0.0, 0.0, 100.0, 80.0)
let gradient = ctx.createLinearGradient (0.0, 0.0, 100.0, 0.0)
gradient.addColorStop (0.0, "#000000")
gradient.addColorStop (1.0, "#ffffff")
ctx.fillStyle <- gradient
ctx.fillRect (0.0, 0.0, 100.0, 10.0)
let (r0, _, _, _) = pixel 2.0 5.0
let (r1, _, _, _) = pixel 97.0 5.0
let (rm, _, _, _) = pixel 50.0 5.0
check (r0 < 20 && r1 > 235) "gradient runs dark to light"
check (rm > 100 && rm < 155) "gradient midpoint"
check (match ctx.fillStyle with
       | StringOrCanvasGradientOrCanvasPattern.CanvasGradient _ -> true
       | _ -> false) "fillStyle is a gradient"
ctx.clearRect (0.0, 0.0, 100.0, 80.0)

// save and restore keep drawing state.
ctx.fillStyle <- "#ff0000"
ctx.save ()
ctx.fillStyle <- "#00ff00"
ctx.lineWidth <- 9.0
ctx.fillRect (0.0, 0.0, 10.0, 10.0)
check (pixel 5.0 5.0 = (0, 255, 0, 255)) "inside save: green"
ctx.restore ()
check (ctx.lineWidth = 1.0) "restore brings back lineWidth"
ctx.fillRect (20.0, 0.0, 10.0, 10.0)
check (pixel 25.0 5.0 = (255, 0, 0, 255)) "restore brings back fillStyle"
ctx.clearRect (0.0, 0.0, 100.0, 80.0)

// Transforms.
ctx.save ()
ctx.translate (30.0, 20.0)
ctx.fillStyle <- "#0000ff"
ctx.fillRect (0.0, 0.0, 10.0, 10.0)
check (pixel 35.0 25.0 = (0, 0, 255, 255)) "translate"
check (pixel 5.0 5.0 = (0, 0, 0, 0)) "nothing at the origin"
let m = ctx.getTransform ()
check (m.e = 30.0 && m.f = 20.0) "getTransform translation"
ctx.scale (2.0, 2.0)
ctx.fillRect (20.0, 0.0, 5.0, 5.0)
check (pixel 75.0 25.0 = (0, 0, 255, 255)) "scale doubles the offset"
check ((ctx.getTransform ()).a = 2.0) "getTransform scale"
ctx.resetTransform ()
check ((ctx.getTransform ()).e = 0.0) "resetTransform"
ctx.setTransform (1.0, 0.0, 0.0, 1.0, 10.0, 10.0)
check ((ctx.getTransform ()).e = 10.0) "setTransform"
ctx.restore ()
check ((ctx.getTransform ()).e = 0.0) "restore undoes the transform"
ctx.clearRect (0.0, 0.0, 100.0, 80.0)
ctx.save ()
ctx.translate (50.0, 40.0)
ctx.rotate (System.Math.PI / 2.0)
ctx.fillStyle <- "#ff0000"
ctx.fillRect (0.0, 0.0, 20.0, 10.0)
check (pixel 45.0 50.0 = (255, 0, 0, 255)) "rotate a quarter turn"
ctx.restore ()
ctx.clearRect (0.0, 0.0, 100.0, 80.0)

// putImageData and getImageData round trip.
let image = ctx.createImageData (2.0, 2.0)
check (image.width = 2.0 && image.height = 2.0) "createImageData size"
check (image.data.length = 16) "four bytes a pixel"
for i in 0..3 do
    image.data.[i * 4] <- byte (i * 60)
    image.data.[i * 4 + 1] <- byte 200
    image.data.[i * 4 + 2] <- byte 10
    image.data.[i * 4 + 3] <- byte 255
ctx.putImageData (image, 40.0, 40.0)
check (pixel 40.0 40.0 = (0, 200, 10, 255)) "putImageData first pixel"
check (pixel 41.0 40.0 = (60, 200, 10, 255)) "putImageData second pixel"
check (pixel 41.0 41.0 = (180, 200, 10, 255)) "putImageData last pixel"
let back = ctx.getImageData (40.0, 40.0, 2.0, 2.0)
check (back.data.length = 16 && int back.data.[8] = 120) "getImageData reads it back"
let made = ImageData.Create (3.0, 1.0)
check (made.width = 3.0) "ImageData constructor"
ctx.clearRect (0.0, 0.0, 100.0, 80.0)

// Text.
ctx.font <- "20px sans-serif"
check (ctx.font = "20px sans-serif") "font"
let metrics = ctx.measureText "Hello"
check (metrics.width > 20.0) "measureText width"
check (metrics.width < (ctx.measureText "Hello Hello").width) "a longer text is wider"
check ((ctx.measureText "").width = 0.0) "empty text has no width"
ctx.font <- "40px sans-serif"
check ((ctx.measureText "Hello").width > metrics.width) "a larger font is wider"
ctx.fillStyle <- "#000000"
ctx.fillText ("Hi", 5.0, 40.0)
let ink = ctx.getImageData (0.0, 0.0, 100.0, 80.0)
let mutable inked = 0
for i in 0 .. 4 .. ink.data.length - 4 do
    if ink.data.[i + 3] > 0uy then inked <- inked + 1
check (inked > 50) "fillText drew pixels"

// Encoding.
check (canvas.toDataURL().StartsWith "data:image/png;base64,") "toDataURL png"
check ((canvas.toDataURL ("image/jpeg", 0.8)).StartsWith "data:image/jpeg;base64,") "toDataURL jpeg"

printfn "%d checks passed" passed
