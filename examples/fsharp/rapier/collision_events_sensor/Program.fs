module Program

open System
open Fable.Core
open Fable.Core.JsInterop
open Dom
open type Dom.Globals
open Dimforge.Rapier3dCompat
open type Dimforge.Rapier3dCompat.Globals

let private bodyCount = 6

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
        (toCx (cx + r00 * lx + r01 * ly), toCy (cy + r10 * lx + r11 * ly))
    let x0, y0 = corner (-hx) (-hy)
    let x1, y1 = corner hx (-hy)
    let x2, y2 = corner hx hy
    let x3, y3 = corner (-hx) hy

    ctx.fillStyle <- !^ fillHex
    ctx.beginPath ()
    ctx.moveTo (x0, y0)
    ctx.lineTo (x1, y1)
    ctx.lineTo (x2, y2)
    ctx.lineTo (x3, y3)
    ctx.closePath ()
    ctx.fill ()

    ctx.strokeStyle <- !^ "#111111"
    ctx.lineWidth <- 1.5
    ctx.beginPath ()
    ctx.moveTo (x0, y0)
    ctx.lineTo (x1, y1)
    ctx.lineTo (x2, y2)
    ctx.lineTo (x3, y3)
    ctx.closePath ()
    ctx.stroke ()

let private drawDashedRect (ctx: CanvasRenderingContext2D) (x: float) (y: float) (w: float) (h: float) =
    let drawSegs x1 y1 x2 y2 n =
        let dx = x2 - x1
        let dy = y2 - y1
        for i in 0 .. n - 1 do
            if i % 2 = 0 then
                let t0 = float i / float n
                let t1 = float (i + 1) / float n
                ctx.beginPath ()
                ctx.moveTo (x1 + dx * t0, y1 + dy * t0)
                ctx.lineTo (x1 + dx * t1, y1 + dy * t1)
                ctx.stroke ()
    drawSegs x y (x + w) y 14
    drawSegs (x + w) y (x + w) (y + h) 4
    drawSegs (x + w) (y + h) x (y + h) 14
    drawSegs x (y + h) x y 4

let main () =
    async {
        do! Async.AwaitPromise (init ())

        let canvas =
            match document.querySelector HTMLElementTagNameMapKeys.canvas with
            | Some c -> c
            | None -> failwith "Canvas not found"
        let ctx = canvas.getContext_2d ()
        let metaEl = document.getElementById "meta"
        let pauseBtn = document.getElementById "btn-pause"

        let mutable paused = false
        let mutable stepCount = 0
        let mutable solidContacts = 0
        let mutable sensorCrossings = 0
        let mutable activeInSensor = 0
        let isBox = [| false; false; true; false; true; false |]

        let addFixedRamp (w: World) (cx: float) (cy: float) (hx: float) (hy: float) (theta: float) =
            let desc = RigidBodyDesc.``fixed``().setTranslation(cx, cy, 0.0).setRotation(Rotation.Create(0.0, 0.0, sin (theta * 0.5), cos (theta * 0.5)))
            let rb = w.createRigidBody desc
            w.createCollider (ColliderDesc.cuboid(hx, hy, 1.0).setFriction(0.15).setRestitution(0.35), rb) |> ignore

        let spawnBodies (w: World) =
            Array.init bodyCount (fun i ->
                let x = -3.7 + float i * 0.52
                let y = 5.4 + float (i % 3) * 0.58
                let rb = w.createRigidBody (RigidBodyDesc.dynamic().setTranslation (x, y, 0.0))
                let cd =
                    if isBox.[i] then ColliderDesc.cuboid (0.28, 0.28, 0.28)
                    else ColliderDesc.ball 0.32
                let configured =
                    cd.setRestitution(0.35).setFriction(0.25).setActiveEvents(ActiveEvents.COLLISION_EVENTS)
                w.createCollider (configured, rb) |> ignore
                rb
            )

        let createScene () =
            let w = World.Create (Vector.Create (0.0, -9.81, 0.0))
            let q = EventQueue.Create true

            addFixedRamp w 0.0 -0.35 6.8 0.35 0.0
            addFixedRamp w -5.4 2.8 0.25 3.2 0.0
            addFixedRamp w 5.4 2.8 0.25 3.2 0.0
            addFixedRamp w -2.1 4.2 2.2 0.18 -0.36
            addFixedRamp w 1.8 2.6 2.2 0.18 0.36

            let sensorBody = w.createRigidBody (RigidBodyDesc.``fixed``().setTranslation (0.0, 1.35, 0.0))
            let sensorDesc =
                ColliderDesc.cuboid(1.85, 0.30, 1.2).setSensor(true).setActiveEvents(ActiveEvents.COLLISION_EVENTS)
            let sensorCollider = w.createCollider (sensorDesc, sensorBody)
            let sHandle = sensorCollider.handle

            let bs = spawnBodies w
            stepCount <- 0
            solidContacts <- 0
            sensorCrossings <- 0
            activeInSensor <- 0
            (w, q, bs, sHandle)

        let mutable (world, queue, bodies, sensorHandle) = createScene ()

        let buildScene () =
            queue.free ()
            world.free ()
            let (w, q, bs, sHandle) = createScene ()
            world <- w
            queue <- q
            bodies <- bs
            sensorHandle <- sHandle

        let stepSim () =
            world.step queue
            stepCount <- stepCount + 1
            queue.drainCollisionEvents (fun h1 h2 started ->
                if h1 = sensorHandle || h2 = sensorHandle then
                    if started then
                        sensorCrossings <- sensorCrossings + 1
                        activeInSensor <- activeInSensor + 1
                    else
                        if activeInSensor > 0 then activeInSensor <- activeInSensor - 1
                elif started then
                    solidContacts <- solidContacts + 1
            )

        let render () =
            ctx.fillStyle <- !^ "#ffffff"
            ctx.fillRect (0.0, 0.0, 680.0, 400.0)

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

            // Sensor gate zone
            let sx = toCx -1.85
            let sy = toCy 1.65
            let sw = 3.70 * 46.0
            let sh = 0.60 * 46.0
            if activeInSensor > 0 then
                ctx.fillStyle <- !^ "#efefef"
                ctx.fillRect (sx, sy, sw, sh)
                ctx.strokeStyle <- !^ "#111111"
                ctx.lineWidth <- 2.0
                ctx.strokeRect (sx, sy, sw, sh)
            else
                ctx.strokeStyle <- !^ "#737373"
                ctx.lineWidth <- 1.4
                drawDashedRect ctx sx sy sw sh

            ctx.fillStyle <- !^ "#555555"
            ctx.font <- "11px ui-monospace, monospace"
            ctx.fillText (sprintf "SENSOR GATE (%d inside)" activeInSensor, sx + 8.0, sy + 17.0)

            // Fixed ramps and catch bin
            drawRotatedBox ctx 0.0 -0.35 0.0 0.0 0.0 1.0 6.8 0.35 "#f7f7f7"
            drawRotatedBox ctx -5.4 2.8 0.0 0.0 0.0 1.0 0.25 3.2 "#efefef"
            drawRotatedBox ctx 5.4 2.8 0.0 0.0 0.0 1.0 0.25 3.2 "#efefef"
            drawRotatedBox ctx -2.1 4.2 0.0 0.0 (sin -0.18) (cos -0.18) 2.2 0.18 "#e5e5e5"
            drawRotatedBox ctx 1.8 2.6 0.0 0.0 (sin 0.18) (cos 0.18) 2.2 0.18 "#e5e5e5"

            // Dynamic bodies
            for i in 0 .. bodyCount - 1 do
                let p = bodies.[i].translation ()
                let q = bodies.[i].rotation ()
                if isBox.[i] then
                    drawRotatedBox ctx p.x p.y q.x q.y q.z q.w 0.28 0.28 "#d4d4d4"
                else
                    let cx = toCx p.x
                    let cy = toCy p.y
                    ctx.fillStyle <- !^ "#262626"
                    ctx.beginPath ()
                    ctx.arc (cx, cy, 0.32 * 46.0, 0.0, 2.0 * Math.PI)
                    ctx.fill ()
                    ctx.strokeStyle <- !^ "#111111"
                    ctx.lineWidth <- 1.5
                    ctx.beginPath ()
                    ctx.arc (cx, cy, 0.32 * 46.0, 0.0, 2.0 * Math.PI)
                    ctx.stroke ()

            let pState = if paused then "paused" else "running"
            metaEl.textContent <-
                sprintf "EventQueue · solid_contacts=%d · sensor_crossings=%d · inside=%d · step=%d (%s)" solidContacts sensorCrossings activeInSensor stepCount pState

        for _ in 1..40 do stepSim ()
        render ()

        let wireClick (id: string) (fn: unit -> unit) =
            let b = document.getElementById id
            b.addEventListener (HTMLElementEventMapKeys.click, fun _ -> fn ())

        wireClick "btn-drop" (fun () ->
            for i in 0 .. bodyCount - 1 do
                let x = -3.7 + float i * 0.52
                let y = 5.4 + float (i % 3) * 0.58
                bodies.[i].setTranslation (Vector.Create (x, y, 0.0), true)
                bodies.[i].setLinvel (Vector.Create (0.4, 0.0, 0.0), true)
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
            for _ in 1..25 do stepSim ()
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
