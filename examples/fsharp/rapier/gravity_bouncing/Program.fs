module Program

open System
open Fable.Core
open Fable.Core.JsInterop
open Dom
open type Dom.Globals
open Dimforge.Rapier3dCompat
open type Dimforge.Rapier3dCompat.Globals

let private restitutions = [| 0.00; 0.35; 0.60; 0.80; 0.95 |]
let private xCoords = [| -4.2; -2.1; 0.0; 2.1; 4.2 |]
let private ballFills = [| "#efefef"; "#d4d4d4"; "#b5b5b5"; "#737373"; "#262626" |]
let private trailLen = 20

let private toCx (wx: float) = 340.0 + wx * 46.0
let private toCy (wy: float) = 350.0 - wy * 46.0

let private drawDashed (ctx: CanvasRenderingContext2D) (x1: float) (y1: float) (x2: float) (y2: float) (segments: int) =
    let dx = x2 - x1
    let dy = y2 - y1
    for i in 0 .. segments - 1 do
        if i % 2 = 0 then
            let t0 = float i / float segments
            let t1 = float (i + 1) / float segments
            ctx.beginPath ()
            ctx.moveTo (x1 + dx * t0, y1 + dy * t0)
            ctx.lineTo (x1 + dx * t1, y1 + dy * t1)
            ctx.stroke ()

let main () =
    async {
        do! Async.AwaitPromise (init ())

        let canvas =
            match document.querySelector HTMLElementTagNameMapKeys.canvas with
            | Some c -> c
            | None -> failwith "Canvas not found"
        let ctx = (canvas.getContext_2d ()).Value
        let metaEl = (document.getElementById "meta").Value
        let gravBtn = (document.getElementById "btn-gravity").Value
        let pauseBtn = (document.getElementById "btn-pause").Value

        let mutable earthGravity = true
        let mutable paused = false
        let mutable stepCount = 0
        let trailX = Array.init 5 (fun _ -> Array.zeroCreate<float> trailLen)
        let trailY = Array.init 5 (fun _ -> Array.zeroCreate<float> trailLen)
        let mutable trailCount = 0

        let createScene () =
            let gy = if earthGravity then -9.81 else -1.62
            let w = World.Create (Vector.Create (0.0, gy, 0.0))

            let ground = w.createRigidBody (RigidBodyDesc.``fixed``().setTranslation (0.0, -0.4, 0.0))
            w.createCollider (ColliderDesc.cuboid (7.0, 0.4, 2.0), ground) |> ignore

            let leftWall = w.createRigidBody (RigidBodyDesc.``fixed``().setTranslation (-6.8, 3.5, 0.0))
            w.createCollider (ColliderDesc.cuboid (0.25, 4.0, 2.0), leftWall) |> ignore

            let rightWall = w.createRigidBody (RigidBodyDesc.``fixed``().setTranslation (6.8, 3.5, 0.0))
            w.createCollider (ColliderDesc.cuboid (0.25, 4.0, 2.0), rightWall) |> ignore

            let bs =
                Array.init 5 (fun i ->
                    let rb =
                        w.createRigidBody (
                            RigidBodyDesc.dynamic().setTranslation (xCoords.[i], 5.6, 0.0)
                        )
                    let cdesc =
                        ColliderDesc.ball(0.42).setRestitution(restitutions.[i]).setFriction(0.2).setDensity(1.2)
                    w.createCollider (cdesc, rb) |> ignore
                    rb
                )
            stepCount <- 0
            trailCount <- 0
            (w, bs)

        let mutable (world, balls) = createScene ()

        let buildScene () =
            world.free ()
            let (w, bs) = createScene ()
            world <- w
            balls <- bs

        let stepSim () =
            world.step ()
            stepCount <- stepCount + 1
            let idx = (stepCount - 1) % trailLen
            for i in 0..4 do
                let p = balls.[i].translation ()
                trailX.[i].[idx] <- p.x
                trailY.[i].[idx] <- p.y
            if trailCount < trailLen then trailCount <- trailCount + 1

        let render () =
            ctx.fillStyle <- !^ "#ffffff"
            ctx.fillRect (0.0, 0.0, 680.0, 400.0)

            // Blueprint drafting grid
            ctx.strokeStyle <- !^ "#efefef"
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

            // Ground slab & cross-hatch ticks
            ctx.fillStyle <- !^ "#f7f7f7"
            ctx.fillRect (0.0, 350.0, 680.0, 50.0)
            ctx.strokeStyle <- !^ "#d4d4d4"
            ctx.lineWidth <- 1.0
            let mutable hx = -20.0
            while hx < 700.0 do
                ctx.beginPath ()
                ctx.moveTo (hx, 350.0)
                ctx.lineTo (hx - 24.0, 400.0)
                ctx.stroke ()
                hx <- hx + 18.0
            ctx.strokeStyle <- !^ "#111111"
            ctx.lineWidth <- 1.5
            ctx.beginPath ()
            ctx.moveTo (0.0, 350.0)
            ctx.lineTo (680.0, 350.0)
            ctx.stroke ()

            // Drop reference line & vertical guides
            ctx.strokeStyle <- !^ "#d4d4d4"
            ctx.lineWidth <- 1.0
            drawDashed ctx (toCx -5.0) (toCy 5.6) (toCx 5.0) (toCy 5.6) 40
            for i in 0..4 do
                let cx = toCx xCoords.[i]
                drawDashed ctx cx (toCy 5.6) cx 350.0 20

            // Trails & balls
            let mutable energySum = 0.0
            let mutable highestY = 0.0
            for i in 0..4 do
                ctx.fillStyle <- !^ "#b5b5b5"
                for k in 0 .. trailCount - 1 do
                    let tx = toCx trailX.[i].[k]
                    let ty = toCy trailY.[i].[k]
                    ctx.beginPath ()
                    ctx.arc (tx, ty, 1.6, 0.0, 2.0 * Math.PI)
                    ctx.fill ()

                let p = balls.[i].translation ()
                let v = balls.[i].linvel ()
                let spd2 = v.x * v.x + v.y * v.y
                energySum <- energySum + 0.5 * spd2
                if p.y > highestY then highestY <- p.y

                let cx = toCx p.x
                let cy = toCy p.y
                let rPx = 0.42 * 46.0

                ctx.fillStyle <- !^ ballFills.[i]
                ctx.beginPath ()
                ctx.arc (cx, cy, rPx, 0.0, 2.0 * Math.PI)
                ctx.fill ()

                ctx.strokeStyle <- !^ "#111111"
                ctx.lineWidth <- 1.6
                ctx.beginPath ()
                ctx.arc (cx, cy, rPx, 0.0, 2.0 * Math.PI)
                ctx.stroke ()

                // Crisp inner ring detail
                let hlColor = if i >= 3 then "#d4d4d4" else "#737373"
                ctx.fillStyle <- !^ hlColor
                ctx.beginPath ()
                ctx.arc (cx - 5.0, cy - 5.0, 2.8, 0.0, 2.0 * Math.PI)
                ctx.fill ()

                ctx.fillStyle <- !^ "#111111"
                ctx.font <- "11px ui-monospace, monospace"
                ctx.fillText (sprintf "e=%.2f" restitutions.[i], cx - 22.0, 368.0)

            let gMode = if earthGravity then "Earth (-9.81)" else "Moon (-1.62)"
            let pState = if paused then "paused" else "running"
            metaEl.textContent <-
                sprintf "5 spheres · g=%s m/s² · max_y=%.2f m · ke=%.1f · step=%d (%s)" gMode highestY energySum stepCount pState

        for _ in 1..28 do stepSim ()
        render ()

        let wireClick (id: string) (fn: unit -> unit) =
            let b = (document.getElementById id).Value
            b.addEventListener (HTMLElementEventMapKeys.click, fun _ -> fn ())

        wireClick "btn-gravity" (fun () ->
            earthGravity <- not earthGravity
            let gy = if earthGravity then -9.81 else -1.62
            world.gravity <- Vector.Create (0.0, gy, 0.0)
            for i in 0..4 do
                balls.[i].applyImpulse (Vector.Create (0.0, 0.25, 0.0), true)
            gravBtn.textContent <- if earthGravity then "Gravity: Earth" else "Gravity: Moon"
            render ()
        )

        wireClick "btn-kick" (fun () ->
            for i in 0..4 do
                let dx = (float i - 2.0) * 0.28
                balls.[i].applyImpulse (Vector.Create (dx, 3.4, 0.0), true)
            render ()
        )

        wireClick "btn-pause" (fun () ->
            paused <- not paused
            pauseBtn.textContent <- if paused then "Resume" else "Pause"
            render ()
        )

        wireClick "btn-reset" (fun () ->
            paused <- false
            pauseBtn.textContent <- "Pause"
            buildScene ()
            for _ in 1..18 do stepSim ()
            render ()
        )

        let rec loop (_ts: float) =
            if not paused then
                stepSim ()
                render ()
            requestAnimationFrame loop |> ignore

        requestAnimationFrame loop |> ignore
    }
    |> Async.StartImmediate

main ()
