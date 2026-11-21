module Program

open System
open Fable.Core
open Fable.Core.JsInterop
open Dom
open type Dom.Globals
open Dimforge.Rapier3dCompat
open type Dimforge.Rapier3dCompat.Globals
open Babylonjs.Lite
open type Babylonjs.Lite.Globals

let private rackCount = 10
let private ballCount = 11
let private ballRadius = 0.36

let private rackPositions =
    [| for row in 0..3 do
           for col in 0..row do
               let x = (float col - float row * 0.5) * 0.74
               let z = -1.6 - float row * 0.645
               yield (x, ballRadius, z) |]

let main () =
    async {
        do! Async.AwaitPromise (init ())

        let metaEl = document.getElementById "meta"
        let statusEl = document.getElementById "status"
        let pauseBtn = document.getElementById "btn-pause"

        let mutable paused = false
        let mutable stepCount = 0

        let buildWorld (shootCue: bool) =
            let w = World.Create (Vector.Create (0.0, -9.81, 0.0))

            let bed = w.createRigidBody (RigidBodyDesc.``fixed``().setTranslation (0.0, -0.25, 0.0))
            w.createCollider (ColliderDesc.cuboid(4.6, 0.25, 7.6).setFriction(0.14).setRestitution(0.35), bed)
            |> ignore

            let rails =
                [| (-4.35, 0.45, 0.0, 0.25, 0.45, 7.6)
                   (4.35, 0.45, 0.0, 0.25, 0.45, 7.6)
                   (0.0, 0.45, -7.35, 4.6, 0.45, 0.25)
                   (0.0, 0.45, 7.35, 4.6, 0.45, 0.25) |]
            for rx, ry, rz, hx, hy, hz in rails do
                let rBody = w.createRigidBody (RigidBodyDesc.``fixed``().setTranslation (rx, ry, rz))
                w.createCollider (ColliderDesc.cuboid(hx, hy, hz).setFriction(0.12).setRestitution(0.88), rBody)
                |> ignore

            let balls =
                Array.init ballCount (fun i ->
                    if i < rackCount then
                        let x, y, z = rackPositions.[i]
                        let rb =
                            w.createRigidBody (
                                RigidBodyDesc.dynamic().setTranslation(x, y, z).setLinearDamping(0.30).setAngularDamping(0.30)
                            )
                        w.createCollider (
                            ColliderDesc.ball(ballRadius).setRestitution(0.92).setFriction(0.10).setDensity(1.0),
                            rb
                        )
                        |> ignore
                        rb
                    else
                        let desc = RigidBodyDesc.dynamic().setTranslation(0.04, ballRadius, 4.4).setLinearDamping(0.22).setAngularDamping(0.22)
                        if shootCue then
                            desc.setLinvel(-0.18, 0.0, -16.0) |> ignore
                        let rb = w.createRigidBody desc
                        w.createCollider (
                            ColliderDesc.ball(ballRadius).setRestitution(0.94).setFriction(0.10).setDensity(1.25),
                            rb
                        )
                        |> ignore
                        rb
                )

            stepCount <- 0
            (w, balls)

        let mutable (world, balls) = buildWorld true
        let mutable ballMeshes: Mesh array = [||]

        let syncMeshes () =
            if ballMeshes.Length = ballCount then
                for i in 0 .. ballCount - 1 do
                    let p = balls.[i].translation ()
                    let q = balls.[i].rotation ()
                    ballMeshes.[i].position.set (p.x, p.y, p.z)
                    ballMeshes.[i].rotationQuaternion.set (q.x, q.y, q.z, q.w)

        let updateMeta () =
            let mutable moving = 0
            let mutable ke = 0.0
            for i in 0 .. ballCount - 1 do
                let v = balls.[i].linvel ()
                let s2 = v.x * v.x + v.y * v.y + v.z * v.z
                if s2 > 0.05 then moving <- moving + 1
                ke <- ke + 0.5 * s2
            let stateText = if paused then "paused" else "running"
            metaEl.textContent <-
                sprintf "11 spheres · moving=%d · ke=%.1f · step=%d (%s)" moving ke stepCount stateText

        let stepSim () =
            world.step ()
            stepCount <- stepCount + 1
            syncMeshes ()
            updateMeta ()

        for _ in 1..26 do
            world.step ()
            stepCount <- stepCount + 1
        updateMeta ()

        let wireClick (id: string) (fn: unit -> unit) =
            (document.getElementById id).addEventListener (HTMLElementEventMapKeys.click, fun _ -> fn ())

        let resetAll (shoot: bool) (warmup: int) =
            paused <- false
            pauseBtn.textContent <- "Pause"
            world.free ()
            let (w, bs) = buildWorld shoot
            world <- w
            balls <- bs
            for _ in 1..warmup do
                world.step ()
                stepCount <- stepCount + 1
            syncMeshes ()
            updateMeta ()

        wireClick "btn-break" (fun () -> resetAll true 0)
        wireClick "btn-reset" (fun () -> resetAll false 0)
        wireClick "btn-scatter" (fun () ->
            for i in 0 .. ballCount - 1 do
                let ang = float i * 1.15
                balls.[i].applyImpulse (Vector.Create (cos ang * 1.4, 0.25, sin ang * 1.4), true)
            updateMeta ()
        )
        wireClick "btn-pause" (fun () ->
            paused <- not paused
            pauseBtn.textContent <- if paused then "Resume" else "Pause"
            updateMeta ()
        )

        try
            let canvas =
                match document.querySelector HTMLElementTagNameMapKeys.canvas with
                | Some c -> c
                | None -> failwith "Canvas not found"
            let! engine = createEngine canvas |> Async.AwaitPromise
            let scene = createSceneContext engine
            scene.clearColor <- GPUColorDict.Create(r = 0.05, g = 0.07, b = 0.09, a = 1.0)

            let camera = createArcRotateCamera (1.18, 0.92, 13.5, vec3 (0.0, 0.3, -0.4))
            addToScene (scene, camera)
            attachControl (camera, canvas, scene) |> ignore

            let hemi = createHemisphericLight ((0.0, 1.0, 0.2), 0.72)
            hemi.diffuseColor <- (0.85, 0.9, 0.95)
            hemi.groundColor <- (0.12, 0.14, 0.16)
            addToScene (scene, hemi)

            let dir = createDirectionalLight ((-1.0, -2.0, -1.0), 2.1)
            dir.diffuse <- (1.0, 0.97, 0.92)
            addToScene (scene, dir)

            let tableMat =
                createPbrMaterial (
                    CreatePbrMaterialProps.Create(
                        baseColorFactor = (0.08, 0.24, 0.18, 1.0),
                        metallicFactor = 0.05,
                        roughnessFactor = 0.78
                    )
                )
            let tableMesh = createBox (engine, BoxOptions.Create(width = 9.2, height = 0.5, depth = 15.2))
            tableMesh.position.set (0.0, -0.25, 0.0)
            tableMesh.material <- tableMat
            addToScene (scene, tableMesh)

            let railMat =
                createPbrMaterial (
                    CreatePbrMaterialProps.Create(
                        baseColorFactor = (0.18, 0.18, 0.20, 1.0),
                        metallicFactor = 0.35,
                        roughnessFactor = 0.42
                    )
                )
            for rx, ry, rz, wR, hR, dR in
                [| (-4.35, 0.45, 0.0, 0.5, 0.9, 15.2)
                   (4.35, 0.45, 0.0, 0.5, 0.9, 15.2)
                   (0.0, 0.45, -7.35, 9.2, 0.9, 0.5)
                   (0.0, 0.45, 7.35, 9.2, 0.9, 0.5) |] do
                let r = createBox (engine, BoxOptions.Create(width = wR, height = hR, depth = dR))
                r.position.set (rx, ry, rz)
                r.material <- railMat
                addToScene (scene, r)

            let cueMat =
                createPbrMaterial (
                    CreatePbrMaterialProps.Create(
                        baseColorFactor = (0.96, 0.96, 0.95, 1.0),
                        metallicFactor = 0.15,
                        roughnessFactor = 0.18
                    )
                )
            let rackMats =
                [| createPbrMaterial (CreatePbrMaterialProps.Create(baseColorFactor = (0.85, 0.24, 0.22, 1.0), metallicFactor = 0.2, roughnessFactor = 0.22))
                   createPbrMaterial (CreatePbrMaterialProps.Create(baseColorFactor = (0.22, 0.48, 0.85, 1.0), metallicFactor = 0.2, roughnessFactor = 0.22))
                   createPbrMaterial (CreatePbrMaterialProps.Create(baseColorFactor = (0.92, 0.68, 0.18, 1.0), metallicFactor = 0.2, roughnessFactor = 0.22))
                   createPbrMaterial (CreatePbrMaterialProps.Create(baseColorFactor = (0.20, 0.20, 0.22, 1.0), metallicFactor = 0.25, roughnessFactor = 0.2)) |]

            ballMeshes <-
                Array.init ballCount (fun i ->
                    let m = createSphere (engine, SphereOptions.Create(diameter = ballRadius * 2.0, segments = 24.0))
                    m.material <- if i = rackCount then cueMat else rackMats.[i % rackMats.Length]
                    addToScene (scene, m)
                    m
                )

            syncMeshes ()

            onBeforeRender (
                scene,
                fun _deltaMs ->
                    if not paused then
                        stepSim ()
            )

            do! registerScene scene |> Async.AwaitPromise
            do! startEngine engine |> Async.AwaitPromise
        with ex ->
            statusEl.textContent <- $"WebGPU / Babylon Lite error: {ex.Message}"
    }
    |> Async.StartImmediate

main ()
