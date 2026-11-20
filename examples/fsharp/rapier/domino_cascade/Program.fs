module Program

open System
open Fable.Core
open Dom
open type Dom.Globals
open Dimforge.Rapier3dCompat
open type Dimforge.Rapier3dCompat.Globals

let private dominoCount = 9
let private domHx = 0.13
let private domHy = 0.85
let private trailLen = 22

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
        let mutable dominoes: RigidBody array = [||]
        let mutable striker: RigidBody = Unchecked.defaultof<RigidBody>
        let trailX = Array.zeroCreate<float> trailLen
        let trailY = Array.zeroCreate<float> trailLen
        let mutable trailCount = 0

        let buildScene () =
            if not (isNull (box world)) then world.free ()
            world <- World.Create (Vector.Create (0.0, -9.81, 0.0))

            let floor = world.createRigidBody (RigidBodyDesc.``fixed``().setTranslation (0.0, -0.35, 0.0))
            world.createCollider (ColliderDesc.cuboid(7.2, 0.35, 2.0).setFriction(0.8), floor) |> ignore

            dominoes <-
                Array.init dominoCount (fun i ->
                    let x = -2.4 + float i * 0.88
                    let rb = world.createRigidBody (RigidBodyDesc.dynamic().setTranslation (x, domHy, 0.0))
                    let cd = ColliderDesc.cuboid(domHx, domHy, 0.35).setFriction(0.55).setRestitution(0.15).setDensity(1.0)
                    world.createCollider (cd, rb) |> ignore
                    rb
                )

            striker <- world.createRigidBody (RigidBodyDesc.dynamic().setTranslation (-5.3, 1.05, 0.0))
            let bcd = ColliderDesc.ball(0.40).setDensity(2.8).setRestitution(0.45).setFriction(0.25)
            world.createCollider (bcd, striker) |> ignore
            striker.applyImpulse (Vector.Create (4.8, 0.2, 0.0), true)

            stepCount <- 0
            trailCount <- 0

        let stepSim () =
            world.step ()
            stepCount <- stepCount + 1
            let sp = striker.translation ()
            let idx = (stepCount - 1) % trailLen
            trailX.[idx] <- sp.x
            trailY.[idx] <- sp.y
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
                ctx.lineTo (x, 350.0)
                ctx.stroke ()
            for gy in 1 .. 7 do
                let y = toCy (float gy)
                ctx.beginPath ()
                ctx.moveTo (0.0, y)
                ctx.lineTo (680.0, y)
                ctx.stroke ()

            ctx.fillStyle <- StringOrCanvasGradientOrCanvasPattern.String "#f7f7f7"
            ctx.fillRect (0.0, 350.0, 680.0, 50.0)
            ctx.strokeStyle <- StringOrCanvasGradientOrCanvasPattern.String "#111111"
            ctx.lineWidth <- 1.5
            ctx.beginPath ()
            ctx.moveTo (0.0, 350.0)
            ctx.lineTo (680.0, 350.0)
            ctx.stroke ()

            // Striker trail
            ctx.fillStyle <- StringOrCanvasGradientOrCanvasPattern.String "#b5b5b5"
            for i in 0 .. trailCount - 1 do
                ctx.beginPath ()
                ctx.arc (toCx trailX.[i], toCy trailY.[i], 2.0, 0.0, 2.0 * Math.PI)
                ctx.fill ()

            // Dominoes
            let mutable toppled = 0
            for i in 0 .. dominoCount - 1 do
                let p = dominoes.[i].translation ()
                let q = dominoes.[i].rotation ()
                let tilted = abs q.z > 0.25
                if tilted then toppled <- toppled + 1
                let fillHex = if tilted then "#d4d4d4" else "#f7f7f7"
                drawRotatedBox ctx p.x p.y q.x q.y q.z q.w domHx domHy fillHex

            // Striker sphere
            let sp = striker.translation ()
            let scx = toCx sp.x
            let scy = toCy sp.y
            ctx.fillStyle <- StringOrCanvasGradientOrCanvasPattern.String "#262626"
            ctx.beginPath ()
            ctx.arc (scx, scy, 0.40 * 46.0, 0.0, 2.0 * Math.PI)
            ctx.fill ()
            ctx.strokeStyle <- StringOrCanvasGradientOrCanvasPattern.String "#111111"
            ctx.lineWidth <- 1.6
            ctx.beginPath ()
            ctx.arc (scx, scy, 0.40 * 46.0, 0.0, 2.0 * Math.PI)
            ctx.stroke ()
            ctx.fillStyle <- StringOrCanvasGradientOrCanvasPattern.String "#e5e5e5"
            ctx.beginPath ()
            ctx.arc (scx - 5.0, scy - 5.0, 2.5, 0.0, 2.0 * Math.PI)
            ctx.fill ()

            ctx.fillStyle <- StringOrCanvasGradientOrCanvasPattern.String "#555555"
            ctx.font <- "11px ui-monospace, monospace"
            ctx.fillText (sprintf "Dominoes toppled: %d / %d" toppled dominoCount, 18.0, 24.0)

            let pState = if paused then "paused" else "running"
            metaEl.textContent <-
                sprintf "9 cuboids + striker sphere · toppled=%d/%d · striker_x=%.2f m · step=%d (%s)" toppled dominoCount sp.x stepCount pState

        buildScene ()
        for _ in 1..22 do stepSim ()
        render ()

        let wireClick (id: string) (fn: unit -> unit) =
            let b = document.getElementById id
            b.addEventListener (HTMLElementEventMapKeys.click, fun _ -> fn ())

        wireClick "btn-push" (fun () ->
            striker.applyImpulse (Vector.Create (3.5, 0.6, 0.0), true)
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
            for _ in 1..12 do stepSim ()
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
