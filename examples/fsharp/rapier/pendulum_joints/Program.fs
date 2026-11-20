module Program

open System
open Fable.Core
open Dom
open type Dom.Globals
open Dimforge.Rapier3dCompat
open type Dimforge.Rapier3dCompat.Globals

let private trailLen = 36
let private linkHx = 0.14
let private linkHy = 0.62

let private toCx (wx: float) = 340.0 + wx * 46.0
let private toCy (wy: float) = 350.0 - wy * 46.0

let private drawRotatedBox
    (ctx: CanvasRenderingContext2D)
    (cx: float)
    (cy: float)
    (qx: float)
    (qy: float)
    (qz: float)
    (qw: float)
    (hx: float)
    (hy: float)
    (fillHex: string)
    =
    let r00 = 1.0 - 2.0 * (qy * qy + qz * qz)
    let r01 = 2.0 * (qx * qy - qw * qz)
    let r10 = 2.0 * (qx * qy + qw * qz)
    let r11 = 1.0 - 2.0 * (qx * qx + qz * qz)

    let corner lx ly =
        let wx = cx + r00 * lx + r01 * ly
        let wy = cy + r10 * lx + r11 * ly
        (toCx wx, toCy wy)

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

let private drawPivot (ctx: CanvasRenderingContext2D) (wx: float) (wy: float) =
    let cx = toCx wx
    let cy = toCy wy
    ctx.fillStyle <- StringOrCanvasGradientOrCanvasPattern.String "#ffffff"
    ctx.beginPath ()
    ctx.arc (cx, cy, 4.0, 0.0, 2.0 * Math.PI)
    ctx.fill ()
    ctx.strokeStyle <- StringOrCanvasGradientOrCanvasPattern.String "#111111"
    ctx.lineWidth <- 1.6
    ctx.beginPath ()
    ctx.arc (cx, cy, 4.0, 0.0, 2.0 * Math.PI)
    ctx.stroke ()

let main () =
    async {
        do! Async.AwaitPromise (init ())

        let canvas = unbox<HTMLCanvasElement> (document.getElementById "canvas")
        let ctx = canvas.getContext_2d ()
        let metaEl = document.getElementById "meta"
        let pauseBtn = document.getElementById "btn-pause"

        let mutable paused = false
        let mutable stepCount = 0
        let mutable world: World = Unchecked.defaultof<World>
        let mutable chainLinks: RigidBody array = [||]
        let mutable armLink: RigidBody = Unchecked.defaultof<RigidBody>
        let mutable armBob: RigidBody = Unchecked.defaultof<RigidBody>
        let trailAX = Array.zeroCreate<float> trailLen
        let trailAY = Array.zeroCreate<float> trailLen
        let trailBX = Array.zeroCreate<float> trailLen
        let trailBY = Array.zeroCreate<float> trailLen
        let mutable trailCount = 0

        let buildScene () =
            if not (isNull (box world)) then world.free ()
            world <- World.Create (Vector.Create (0.0, -9.81, 0.0))

            let gantry = world.createRigidBody (RigidBodyDesc.``fixed``().setTranslation (0.0, 6.2, 0.0))
            world.createCollider (ColliderDesc.cuboid(6.0, 0.22, 0.6), gantry) |> ignore

            let zAxis = Vector.Create (0.0, 0.0, 1.0)

            // Assembly A: 3-link hanging pendulum at x = -2.2
            let l1 = world.createRigidBody (RigidBodyDesc.dynamic().setTranslation (-2.8, 5.4, 0.0))
            world.createCollider (ColliderDesc.cuboid(linkHx, linkHy, 0.2).setDensity(1.4), l1) |> ignore
            let l2 = world.createRigidBody (RigidBodyDesc.dynamic().setTranslation (-3.3, 4.1, 0.0))
            world.createCollider (ColliderDesc.cuboid(linkHx, linkHy, 0.2).setDensity(1.4), l2) |> ignore
            let l3 = world.createRigidBody (RigidBodyDesc.dynamic().setTranslation (-3.8, 2.8, 0.0))
            world.createCollider (ColliderDesc.cuboid(linkHx, linkHy, 0.2).setDensity(1.6), l3) |> ignore

            world.createImpulseJoint (
                JointData.revolute (Vector.Create (-2.2, -0.22, 0.0), Vector.Create (0.0, linkHy + 0.06, 0.0), zAxis),
                gantry, l1, true
            ) |> ignore
            world.createImpulseJoint (
                JointData.revolute (Vector.Create (0.0, -linkHy - 0.06, 0.0), Vector.Create (0.0, linkHy + 0.06, 0.0), zAxis),
                l1, l2, true
            ) |> ignore
            world.createImpulseJoint (
                JointData.revolute (Vector.Create (0.0, -linkHy - 0.06, 0.0), Vector.Create (0.0, linkHy + 0.06, 0.0), zAxis),
                l2, l3, true
            ) |> ignore
            l3.applyImpulse (Vector.Create (-1.4, 0.0, 0.0), true)
            chainLinks <- [| l1; l2; l3 |]

            // Assembly B: 2-body articulated swinging arm + heavy sphere bob at x = +2.3
            armLink <- world.createRigidBody (RigidBodyDesc.dynamic().setTranslation (3.1, 5.2, 0.0))
            world.createCollider (ColliderDesc.cuboid(0.14, 0.85, 0.2).setDensity(1.2), armLink) |> ignore
            armBob <- world.createRigidBody (RigidBodyDesc.dynamic().setTranslation (3.8, 3.6, 0.0))
            world.createCollider (ColliderDesc.ball(0.42).setDensity(3.0), armBob) |> ignore

            world.createImpulseJoint (
                JointData.revolute (Vector.Create (2.3, -0.22, 0.0), Vector.Create (0.0, 0.88, 0.0), zAxis),
                gantry, armLink, true
            ) |> ignore
            world.createImpulseJoint (
                JointData.revolute (Vector.Create (0.0, -0.88, 0.0), Vector.Create (0.0, 0.72, 0.0), zAxis),
                armLink, armBob, true
            ) |> ignore
            armBob.applyImpulse (Vector.Create (1.8, 0.2, 0.0), true)

            stepCount <- 0
            trailCount <- 0

        let stepSim () =
            world.step ()
            stepCount <- stepCount + 1
            let pA = chainLinks.[2].translation ()
            let pB = armBob.translation ()
            let idx = (stepCount - 1) % trailLen
            trailAX.[idx] <- pA.x
            trailAY.[idx] <- pA.y
            trailBX.[idx] <- pB.x
            trailBY.[idx] <- pB.y
            if trailCount < trailLen then trailCount <- trailCount + 1

        let render () =
            ctx.fillStyle <- StringOrCanvasGradientOrCanvasPattern.String "#ffffff"
            ctx.fillRect (0.0, 0.0, 680.0, 400.0)

            ctx.strokeStyle <- StringOrCanvasGradientOrCanvasPattern.String "#efefef"
            ctx.lineWidth <- 1.0
            for gx in -6 .. 6 do
                let x = toCx (float gx)
                ctx.beginPath ()
                ctx.moveTo (x, 0.0)
                ctx.lineTo (x, 400.0)
                ctx.stroke ()
            for gy in 0 .. 7 do
                let y = toCy (float gy)
                ctx.beginPath ()
                ctx.moveTo (0.0, y)
                ctx.lineTo (680.0, y)
                ctx.stroke ()

            // Overhead gantry
            drawRotatedBox ctx 0.0 6.2 0.0 0.0 0.0 1.0 6.0 0.22 "#efefef"

            // End-bob swing trails
            ctx.fillStyle <- StringOrCanvasGradientOrCanvasPattern.String "#b5b5b5"
            for i in 0 .. trailCount - 1 do
                ctx.beginPath ()
                ctx.arc (toCx trailAX.[i], toCy trailAY.[i], 1.6, 0.0, 2.0 * Math.PI)
                ctx.fill ()
                ctx.beginPath ()
                ctx.arc (toCx trailBX.[i], toCy trailBY.[i], 1.6, 0.0, 2.0 * Math.PI)
                ctx.fill ()

            // Assembly A links
            for i in 0..2 do
                let p = chainLinks.[i].translation ()
                let q = chainLinks.[i].rotation ()
                let fillHex = if i = 2 then "#d4d4d4" else "#f7f7f7"
                drawRotatedBox ctx p.x p.y q.x q.y q.z q.w linkHx linkHy fillHex

            // Assembly B rod + sphere bob
            let ap = armLink.translation ()
            let aq = armLink.rotation ()
            let bp = armBob.translation ()
            drawRotatedBox ctx ap.x ap.y aq.x aq.y aq.z aq.w 0.14 0.85 "#f7f7f7"

            let bCx = toCx bp.x
            let bCy = toCy bp.y
            ctx.fillStyle <- StringOrCanvasGradientOrCanvasPattern.String "#262626"
            ctx.beginPath ()
            ctx.arc (bCx, bCy, 0.42 * 46.0, 0.0, 2.0 * Math.PI)
            ctx.fill ()
            ctx.strokeStyle <- StringOrCanvasGradientOrCanvasPattern.String "#111111"
            ctx.lineWidth <- 1.6
            ctx.beginPath ()
            ctx.arc (bCx, bCy, 0.42 * 46.0, 0.0, 2.0 * Math.PI)
            ctx.stroke ()

            // Pivot pins
            drawPivot ctx -2.2 5.98
            for i in 0..1 do
                let p = chainLinks.[i].translation ()
                let q = chainLinks.[i].rotation ()
                let r01 = 2.0 * (q.x * q.y - q.w * q.z)
                let r11 = 1.0 - 2.0 * (q.x * q.x + q.z * q.z)
                drawPivot ctx (p.x - r01 * (linkHy + 0.06)) (p.y - r11 * (linkHy + 0.06))
            drawPivot ctx 2.3 5.98
            let armR01 = 2.0 * (aq.x * aq.y - aq.w * aq.z)
            let armR11 = 1.0 - 2.0 * (aq.x * aq.x + aq.z * aq.z)
            drawPivot ctx (ap.x - armR01 * 0.88) (ap.y - armR11 * 0.88)

            ctx.fillStyle <- StringOrCanvasGradientOrCanvasPattern.String "#555555"
            ctx.font <- "11px ui-monospace, monospace"
            ctx.fillText ("3-Link Revolute Chain", toCx -3.5, 36.0)
            ctx.fillText ("2-Body Bob Pendulum", toCx 1.1, 36.0)

            let pA = chainLinks.[2].translation ()
            let pState = if paused then "paused" else "running"
            metaEl.textContent <-
                sprintf "5 revolute joints · chain_tip=(%.2f, %.2f) · bob=(%.2f, %.2f) · step=%d (%s)" pA.x pA.y bp.x bp.y stepCount pState

        buildScene ()
        for _ in 1..20 do stepSim ()
        render ()

        let wireClick (id: string) (fn: unit -> unit) =
            let b = document.getElementById id
            b.addEventListener (HTMLElementEventMapKeys.click, fun _ -> fn ())

        wireClick "btn-kick" (fun () ->
            chainLinks.[2].applyImpulse (Vector.Create (1.6, 0.3, 0.0), true)
            armBob.applyImpulse (Vector.Create (-2.0, 0.4, 0.0), true)
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
            for _ in 1..15 do stepSim ()
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
