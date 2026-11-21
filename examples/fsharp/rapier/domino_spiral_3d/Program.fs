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

let private domCount = 14
let private cylCount = 3
let private domHx = 0.09
let private domHy = 0.65
let private domHz = 0.34
let private marbleRadius = 0.36
let private rampAngle = -0.38
let private rampPos = (-3.55, 1.02, -2.5)

let private spiralPose (i: int) =
    let t = float i * 0.255
    let r = 2.6 - float i * 0.045
    let x = -1.35 + r * sin t
    let z = -2.5 + 2.6 - r * cos t
    let yaw = -t
    (x, domHy, z, yaw)

let main () =
    async {
        do! Async.AwaitPromise (init ())

        let metaEl = document.getElementById "meta"
        let statusEl = document.getElementById "status"
        let pauseBtn = document.getElementById "btn-pause"

        let mutable paused = false
        let mutable stepCount = 0

        let buildWorld () =
            let w = World.Create (Vector.Create (0.0, -9.81, 0.0))

            let floorBody = w.createRigidBody (RigidBodyDesc.``fixed``().setTranslation (0.0, -0.3, 0.0))
            w.createCollider (ColliderDesc.cuboid(10.0, 0.3, 10.0).setFriction(0.75).setRestitution(0.1), floorBody)
            |> ignore

            let rx, ry, rz = rampPos
            let rampRot = Quaternion.Create (0.0, 0.0, sin (rampAngle * 0.5), cos (rampAngle * 0.5))
            let rampBody =
                w.createRigidBody (RigidBodyDesc.``fixed``().setTranslation(rx, ry, rz).setRotation(rampRot))
            w.createCollider (ColliderDesc.cuboid(1.75, 0.10, 0.65).setFriction(0.15).setRestitution(0.2), rampBody)
            |> ignore

            let marble =
                w.createRigidBody (
                    RigidBodyDesc.dynamic().setTranslation(-4.75, 2.02, -2.5).setLinvel(3.0, 0.0, 0.0)
                )
            w.createCollider (
                ColliderDesc.ball(marbleRadius).setDensity(3.2).setRestitution(0.32).setFriction(0.18),
                marble
            )
            |> ignore

            let doms =
                Array.init domCount (fun i ->
                    let x, y, z, yaw = spiralPose i
                    let q = Quaternion.Create (0.0, sin (yaw * 0.5), 0.0, cos (yaw * 0.5))
                    let rb =
                        w.createRigidBody (RigidBodyDesc.dynamic().setTranslation(x, y, z).setRotation(q))
                    w.createCollider (
                        ColliderDesc.cuboid(domHx, domHy, domHz).setFriction(0.55).setRestitution(0.18).setDensity(1.0),
                        rb
                    )
                    |> ignore
                    rb
                )

            let endX, _, endZ, endYaw = spiralPose (domCount - 1)
            let fwdX = cos (-endYaw)
            let fwdZ = sin (-endYaw)
            let perpX = -fwdZ
            let perpZ = fwdX
            let cyls =
                Array.init cylCount (fun j ->
                    let spread = float (j - 1) * 0.52
                    let cx = endX + fwdX * 0.72 + perpX * spread
                    let cz = endZ + fwdZ * 0.72 + perpZ * spread
                    let rb = w.createRigidBody (RigidBodyDesc.dynamic().setTranslation (cx, 0.42, cz))
                    w.createCollider (
                        ColliderDesc.cylinder(0.42, 0.25).setFriction(0.55).setRestitution(0.25).setDensity(1.0),
                        rb
                    )
                    |> ignore
                    rb
                )

            stepCount <- 0
            (w, marble, doms, cyls)

        let mutable (world, marble, doms, cyls) = buildWorld ()
        let mutable marbleMesh: Mesh option = None
        let mutable domMeshes: Mesh array = [||]
        let mutable cylMeshes: Mesh array = [||]

        let syncMeshes () =
            match marbleMesh with
            | Some m ->
                let mp = marble.translation ()
                let mq = marble.rotation ()
                m.position.set (mp.x, mp.y, mp.z)
                m.rotationQuaternion.set (mq.x, mq.y, mq.z, mq.w)
            | None -> ()
            if domMeshes.Length = domCount then
                for i in 0 .. domCount - 1 do
                    let p = doms.[i].translation ()
                    let q = doms.[i].rotation ()
                    domMeshes.[i].position.set (p.x, p.y, p.z)
                    domMeshes.[i].rotationQuaternion.set (q.x, q.y, q.z, q.w)
            if cylMeshes.Length = cylCount then
                for j in 0 .. cylCount - 1 do
                    let p = cyls.[j].translation ()
                    let q = cyls.[j].rotation ()
                    cylMeshes.[j].position.set (p.x, p.y, p.z)
                    cylMeshes.[j].rotationQuaternion.set (q.x, q.y, q.z, q.w)

        let updateMeta () =
            let mutable fallen = 0
            let mutable ke = 0.0
            let mv = marble.linvel ()
            ke <- ke + 0.5 * (mv.x * mv.x + mv.y * mv.y + mv.z * mv.z)
            for i in 0 .. domCount - 1 do
                let p = doms.[i].translation ()
                let v = doms.[i].linvel ()
                if p.y < 0.48 then fallen <- fallen + 1
                ke <- ke + 0.5 * (v.x * v.x + v.y * v.y + v.z * v.z)
            for j in 0 .. cylCount - 1 do
                let v = cyls.[j].linvel ()
                ke <- ke + 0.5 * (v.x * v.x + v.y * v.y + v.z * v.z)
            let stateText = if paused then "paused" else "running"
            metaEl.textContent <-
                sprintf "18 bodies (marble + 14 dominoes + 3 cylinders) · fallen=%d · ke=%.1f · step=%d (%s)" fallen ke stepCount stateText

        let stepSim () =
            world.step ()
            stepCount <- stepCount + 1
            syncMeshes ()
            updateMeta ()

        for _ in 1..20 do
            world.step ()
            stepCount <- stepCount + 1
        updateMeta ()

        let wireClick (id: string) (fn: unit -> unit) =
            (document.getElementById id).addEventListener (HTMLElementEventMapKeys.click, fun _ -> fn ())

        wireClick "btn-launch" (fun () ->
            marble.setTranslation (Vector.Create (-4.75, 2.02, -2.5), true)
            marble.setLinvel (Vector.Create (3.0, 0.0, 0.0), true)
            marble.setAngvel (Vector.Create (0.0, 0.0, 0.0), true)
            syncMeshes ()
            updateMeta ()
        )

        wireClick "btn-nudge" (fun () ->
            doms.[0].applyImpulse (Vector.Create (1.6, 0.0, 0.0), true)
            updateMeta ()
        )

        wireClick "btn-pause" (fun () ->
            paused <- not paused
            pauseBtn.textContent <- if paused then "Resume" else "Pause"
            updateMeta ()
        )

        wireClick "btn-reset" (fun () ->
            paused <- false
            pauseBtn.textContent <- "Pause"
            world.free ()
            let (w, mb, ds, cs) = buildWorld ()
            world <- w
            marble <- mb
            doms <- ds
            cyls <- cs
            syncMeshes ()
            updateMeta ()
        )

        try
            let canvas =
                match document.querySelector HTMLElementTagNameMapKeys.canvas with
                | Some c -> c
                | None -> failwith "Canvas not found"
            let! engine = createEngine canvas |> Async.AwaitPromise
            let scene = createSceneContext engine
            scene.clearColor <- GPUColorDict.Create(r = 0.06, g = 0.07, b = 0.09, a = 1.0)

            let camera = createArcRotateCamera (0.65, 1.05, 12.0, vec3 (-0.4, 0.8, 0.0))
            addToScene (scene, camera)
            attachControl (camera, canvas, scene) |> ignore

            let hemi = createHemisphericLight ((0.0, 1.0, 0.2), 0.72)
            hemi.diffuseColor <- (0.85, 0.88, 0.95)
            hemi.groundColor <- (0.14, 0.14, 0.16)
            addToScene (scene, hemi)

            let dir = createDirectionalLight ((-1.0, -2.0, -1.2), 2.1)
            dir.diffuse <- (1.0, 0.97, 0.92)
            addToScene (scene, dir)

            let groundMat =
                createGridMaterial (
                    GridMaterialOptions.Create(
                        mainColor = (0.09, 0.10, 0.12),
                        lineColor = (0.34, 0.36, 0.40),
                        gridRatio = 0.5,
                        majorUnitFrequency = 4.0,
                        minorUnitVisibility = 0.3
                    )
                )
            let ground = createGround (engine, GroundOptions.Create(width = 20.0, height = 20.0))
            ground.material <- groundMat
            addToScene (scene, ground)

            let rampMat =
                createPbrMaterial (
                    CreatePbrMaterialProps.Create(
                        baseColorFactor = (0.28, 0.30, 0.34, 1.0),
                        metallicFactor = 0.35,
                        roughnessFactor = 0.48
                    )
                )
            let rampMesh = createBox (engine, BoxOptions.Create(width = 3.5, height = 0.2, depth = 1.3))
            let rx, ry, rz = rampPos
            rampMesh.position.set (rx, ry, rz)
            rampMesh.rotationQuaternion.set (0.0, 0.0, sin (rampAngle * 0.5), cos (rampAngle * 0.5))
            rampMesh.material <- rampMat
            addToScene (scene, rampMesh)

            let marbleMat =
                createPbrMaterial (
                    CreatePbrMaterialProps.Create(
                        baseColorFactor = (0.95, 0.42, 0.28, 1.0),
                        metallicFactor = 0.55,
                        roughnessFactor = 0.2
                    )
                )
            let mb = createSphere (engine, SphereOptions.Create(diameter = marbleRadius * 2.0, segments = 24.0))
            mb.material <- marbleMat
            addToScene (scene, mb)
            marbleMesh <- Some mb

            let domMatA =
                createPbrMaterial (CreatePbrMaterialProps.Create(baseColorFactor = (0.88, 0.88, 0.88, 1.0), metallicFactor = 0.15, roughnessFactor = 0.4))
            let domMatB =
                createPbrMaterial (CreatePbrMaterialProps.Create(baseColorFactor = (0.45, 0.46, 0.48, 1.0), metallicFactor = 0.25, roughnessFactor = 0.38))

            domMeshes <-
                Array.init domCount (fun i ->
                    let m = createBox (engine, BoxOptions.Create(width = domHx * 2.0, height = domHy * 2.0, depth = domHz * 2.0))
                    m.material <- if i % 2 = 0 then domMatA else domMatB
                    addToScene (scene, m)
                    m
                )

            let cylMat =
                createPbrMaterial (CreatePbrMaterialProps.Create(baseColorFactor = (0.26, 0.62, 0.88, 1.0), metallicFactor = 0.45, roughnessFactor = 0.25))
            cylMeshes <-
                Array.init cylCount (fun _ ->
                    let m = createCylinder (engine, CylinderOptions.Create(height = 0.84, diameter = 0.5, tessellation = 24.0))
                    m.material <- cylMat
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
