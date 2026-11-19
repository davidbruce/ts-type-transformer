module Program

open System
open Fable.Core
open Dom
open type Dom.Globals
open Dimforge.Rapier3dCompat
open type Dimforge.Rapier3dCompat.Globals

let private rayCount = 37
let private maxToi = 16.0
let private emitterX = -5.6
let private emitterY = 4.6

let private toCx (wx: float) = 340.0 + wx * 46.0
let private toCy (wy: float) = 350.0 - wy * 46.0

let private drawBox
    (ctx: CanvasRenderingContext2D)
    (cx: float)
    (cy: float)
    (theta: float)
    (hx: float)
    (hy: float)
    (fillHex: string)
    =
    let c = cos theta
    let s = sin theta
    let corner lx ly =
        (toCx (cx + c * lx - s * ly), toCy (cy + s * lx + c * ly))
    let x0, y0 = corner (-hx) (-hy)
    let x1, y1 = corner hx (-hy)
    let x2, y2 = corner hx hy
    let x3, y3 = corner (-hx) hy

    ctx.fillStyle <- StringOrCanvasGradientOrCanvasPattern.String fillHex
    ctx.beginPath ()
    ctx.moveTo (x0, y0)
    ctx.lineTo (x1, y1)
    ctx.lineTo (x2, y2)
    ctx.lineTo (x3, y3)
    ctx.closePath ()
    ctx.fill ()

    ctx.strokeStyle <- StringOrCanvasGradientOrCanvasPattern.String "#111111"
    ctx.lineWidth <- 1.5
    ctx.beginPath ()
    ctx.moveTo (x0, y0)
    ctx.lineTo (x1, y1)
    ctx.lineTo (x2, y2)
    ctx.lineTo (x3, y3)
    ctx.closePath ()
    ctx.stroke ()

let private drawCircle (ctx: CanvasRenderingContext2D) (wx: float) (wy: float) (r: float) (fillHex: string) =
    let cx = toCx wx
    let cy = toCy wy
    let rp = r * 46.0
    ctx.fillStyle <- StringOrCanvasGradientOrCanvasPattern.String fillHex
    ctx.beginPath ()
    ctx.arc (cx, cy, rp, 0.0, 2.0 * Math.PI)
    ctx.fill ()
    ctx.strokeStyle <- StringOrCanvasGradientOrCanvasPattern.String "#111111"
    ctx.lineWidth <- 1.5
    ctx.beginPath ()
    ctx.arc (cx, cy, rp, 0.0, 2.0 * Math.PI)
    ctx.stroke ()

let main () =
    async {
        do! Async.AwaitPromise (init ())

        let canvas = unbox<HTMLCanvasElement> (document.getElementById "canvas")
        let ctx = canvas.getContext_2d ()
        let metaEl = document.getElementById "meta"
        let sweepBtn = unbox<HTMLElement> (document.getElementById "btn-sweep")

        let world = World.Create (Vector.Create (0.0, 0.0, 0.0))

        let addBox cx cy theta hx hy =
            let desc = RigidBodyDesc.``fixed``().setTranslation(cx, cy, 0.0).setRotation(Rotation.Create(0.0, 0.0, sin (theta * 0.5), cos (theta * 0.5)))
            let rb = world.createRigidBody desc
            world.createCollider (ColliderDesc.cuboid (hx, hy, 1.0), rb) |> ignore

        let addSphere cx cy r =
            let rb = world.createRigidBody (RigidBodyDesc.``fixed``().setTranslation (cx, cy, 0.0))
            world.createCollider (ColliderDesc.ball r, rb) |> ignore

        addBox 0.0 -0.35 0.0 7.0 0.35
        addBox -1.6 0.85 0.0 0.70 0.85
        addBox 2.2 1.35 0.0 0.65 1.35
        addBox 0.2 1.2 -0.42 1.3 0.18
        addBox 0.4 4.4 0.0 1.9 0.22
        addSphere -2.5 3.6 0.65
        addSphere 4.2 3.2 0.75
        world.step ()

        let mutable autoSweep = true
        let mutable phase = 0.0
        let mutable manualOffset = 0.0

        let render () =
            ctx.fillStyle <- StringOrCanvasGradientOrCanvasPattern.String "#ffffff"
            ctx.fillRect (0.0, 0.0, 680.0, 400.0)

            ctx.strokeStyle <- StringOrCanvasGradientOrCanvasPattern.String "#efefef"
            ctx.lineWidth <- 1.0
            for gx in -6 .. 6 do
                let x = toCx (float gx)
                ctx.beginPath ()
                ctx.moveTo (x, 0.0)
                ctx.lineTo (x, 350.0)
                ctx.stroke ()
            for gy in 1 .. 7 do
                let y = toCy (float gy)
                ctx.beginPath ()
                ctx.moveTo (0.0, y)
                ctx.lineTo (680.0, y)
                ctx.stroke ()

            // Static obstacles
            drawBox ctx 0.0 -0.35 0.0 7.0 0.35 "#f7f7f7"
            drawBox ctx -1.6 0.85 0.0 0.70 0.85 "#efefef"
            drawBox ctx 2.2 1.35 0.0 0.65 1.35 "#efefef"
            drawBox ctx 0.2 1.2 -0.42 1.3 0.18 "#d4d4d4"
            drawBox ctx 0.4 4.4 0.0 1.9 0.22 "#efefef"
            drawCircle ctx -2.5 3.6 0.65 "#d4d4d4"
            drawCircle ctx 4.2 3.2 0.75 "#d4d4d4"

            let baseAngle = -0.35 + manualOffset + 0.28 * sin phase
            let ox = toCx emitterX
            let oy = toCy emitterY
            let origin = Vector.Create (emitterX, emitterY, 0.0)

            let mutable hits = 0
            let mutable minToi = maxToi
            for i in 0 .. rayCount - 1 do
                let frac = float i / float (rayCount - 1) - 0.5
                let angle = baseAngle + frac * 1.35
                let dx = cos angle
                let dy = sin angle
                let ray = Ray.Create (origin, Vector.Create (dx, dy, 0.0))
                let hit = world.castRayAndGetNormal (ray, maxToi, true)
                if not (isNull (box hit)) then
                    hits <- hits + 1
                    let toi = hit.timeOfImpact
                    if toi < minToi then minToi <- toi
                    let pt = ray.pointAt toi
                    let hx = toCx pt.x
                    let hy = toCy pt.y

                    ctx.strokeStyle <- StringOrCanvasGradientOrCanvasPattern.String "#737373"
                    ctx.lineWidth <- 1.0
                    ctx.beginPath ()
                    ctx.moveTo (ox, oy)
                    ctx.lineTo (hx, hy)
                    ctx.stroke ()

                    let n = hit.normal
                    ctx.strokeStyle <- StringOrCanvasGradientOrCanvasPattern.String "#111111"
                    ctx.lineWidth <- 1.4
                    ctx.beginPath ()
                    ctx.moveTo (hx, hy)
                    ctx.lineTo (toCx (pt.x + n.x * 0.45), toCy (pt.y + n.y * 0.45))
                    ctx.stroke ()

                    ctx.fillStyle <- StringOrCanvasGradientOrCanvasPattern.String "#111111"
                    ctx.beginPath ()
                    ctx.arc (hx, hy, 2.8, 0.0, 2.0 * Math.PI)
                    ctx.fill ()
                else
                    ctx.strokeStyle <- StringOrCanvasGradientOrCanvasPattern.String "#e5e5e5"
                    ctx.lineWidth <- 0.9
                    ctx.beginPath ()
                    ctx.moveTo (ox, oy)
                    ctx.lineTo (toCx (emitterX + dx * maxToi), toCy (emitterY + dy * maxToi))
                    ctx.stroke ()

            // Emitter turret
            ctx.fillStyle <- StringOrCanvasGradientOrCanvasPattern.String "#111111"
            ctx.beginPath ()
            ctx.arc (ox, oy, 6.0, 0.0, 2.0 * Math.PI)
            ctx.fill ()
            ctx.strokeStyle <- StringOrCanvasGradientOrCanvasPattern.String "#ffffff"
            ctx.lineWidth <- 1.5
            ctx.beginPath ()
            ctx.arc (ox, oy, 3.0, 0.0, 2.0 * Math.PI)
            ctx.stroke ()

            let deg = int (Math.Round(baseAngle * 180.0 / Math.PI))
            let modeLabel = if autoSweep then "auto" else "manual"
            metaEl.textContent <-
                sprintf "37 rays · hits=%d/%d · nearest_toi=%.2f m · center=%d deg (%s)" hits rayCount minToi deg modeLabel

        render ()

        let wireClick (id: string) (fn: unit -> unit) =
            let b = unbox<HTMLElement> (document.getElementById id)
            b.addEventListener (HTMLElementEventMapKeys.click, fun _ -> fn ())

        wireClick "btn-sweep" (fun () ->
            autoSweep <- not autoSweep
            sweepBtn.textContent <- if autoSweep then "Sweep: Auto" else "Sweep: Manual"
            render ()
        )

        wireClick "btn-angle" (fun () ->
            manualOffset <- manualOffset + Math.PI / 12.0
            if manualOffset > 0.8 then manualOffset <- -0.6
            render ()
        )

        wireClick "btn-reset" (fun () ->
            autoSweep <- true
            phase <- 0.0
            manualOffset <- 0.0
            sweepBtn.textContent <- "Sweep: Auto"
            render ()
        )

        let rec loop (_ts: float) =
            if autoSweep then
                phase <- phase + 0.025
                render ()
            requestAnimationFrame loop |> ignore

        requestAnimationFrame loop |> ignore
    }
    |> Async.StartImmediate

main ()
