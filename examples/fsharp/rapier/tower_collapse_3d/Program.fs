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

let private layers = 6
let private perLayer = 3
let private plankCount = layers * perLayer
let private halfLong = 0.85
let private halfHeight = 0.22
let private halfThin = 0.26
let private sphereRadius = 0.65

let main () =
    async {
        do! Async.AwaitPromise (init ())

        let metaEl = (document.getElementById "meta").Value
        let statusEl = (document.getElementById "status").Value
        let pauseBtn = (document.getElementById "btn-pause").Value

        let mutable paused = false
        let mutable stepCount = 0
        let mutable shotIdx = 0

        let buildWorld () =
            let w = World.Create (Vector.Create (0.0, -9.81, 0.0))

            let floorBody = w.createRigidBody (RigidBodyDesc.``fixed``().setTranslation (0.0, -0.3, 0.0))
            w.createCollider (ColliderDesc.cuboid(10.0, 0.3, 10.0).setFriction(0.75).setRestitution(0.1), floorBody)
            |> ignore

            let planks =
                Array.init plankCount (fun idx ->
                    let layer = idx / perLayer
                    let slot = idx % perLayer
                    let offset = float (slot - 1) * 0.57
                    let y = halfHeight + float layer * 0.445
                    let x, z, hx, hz =
                        if layer % 2 = 0 then (0.0, offset, halfLong, halfThin)
                        else (offset, 0.0, halfThin, halfLong)

                    let rb = w.createRigidBody (RigidBodyDesc.dynamic().setTranslation (x, y, z))
                    let cd = ColliderDesc.cuboid(hx, halfHeight, hz).setFriction(0.65).setRestitution(0.12).setDensity(1.0)
                    w.createCollider (cd, rb) |> ignore
                    rb
                )

            let striker =
                w.createRigidBody (
                    RigidBodyDesc.dynamic().setTranslation(-6.2, 1.55, 0.12).setLinvel(13.5, 1.1, -0.25)
                )
            w.createCollider (
                ColliderDesc.ball(sphereRadius).setDensity(4.5).setRestitution(0.35).setFriction(0.3),
                striker
            )
            |> ignore

            stepCount <- 0
            (w, planks, striker)

        let mutable (world, planks, striker) = buildWorld ()
        let mutable plankMeshes: Mesh array = [||]
        let mutable strikerMesh: Mesh option = None

        let syncMeshes () =
            if plankMeshes.Length = plankCount then
                for i in 0 .. plankCount - 1 do
                    let p = planks.[i].translation ()
                    let q = planks.[i].rotation ()
                    plankMeshes.[i].position.set (p.x, p.y, p.z)
                    plankMeshes.[i].rotationQuaternion.set (q.x, q.y, q.z, q.w)
            match strikerMesh with
            | Some m ->
                let sp = striker.translation ()
                let sq = striker.rotation ()
                m.position.set (sp.x, sp.y, sp.z)
                m.rotationQuaternion.set (sq.x, sq.y, sq.z, sq.w)
            | None -> ()

        let updateMeta () =
            let mutable maxY = 0.0
            let mutable ke = 0.0
            for i in 0 .. plankCount - 1 do
                let p = planks.[i].translation ()
                let v = planks.[i].linvel ()
                if p.y > maxY then maxY <- p.y
                ke <- ke + 0.5 * (v.x * v.x + v.y * v.y + v.z * v.z)
            let sv = striker.linvel ()
            ke <- ke + 2.25 * (sv.x * sv.x + sv.y * sv.y + sv.z * sv.z)
            let stateText = if paused then "paused" else "running"
            metaEl.textContent <-
                sprintf "19 bodies (18 planks + sphere) · max_y=%.2f m · ke=%.1f · step=%d (%s)" maxY ke stepCount stateText

        let stepSim () =
            world.step ()
            stepCount <- stepCount + 1
            syncMeshes ()
            updateMeta ()

        for _ in 1..18 do
            world.step ()
            stepCount <- stepCount + 1
        updateMeta ()

        let wireClick (id: string) (fn: unit -> unit) =
            (document.getElementById id).Value.addEventListener (HTMLElementEventMapKeys.click, fun _ -> fn ())

        wireClick "btn-launch" (fun () ->
            shotIdx <- shotIdx + 1
            let angle = float shotIdx * 0.9
            let dist = 6.2
            let sx = -dist * cos angle
            let sz = dist * sin angle
            let sy = 1.2 + float (shotIdx % 3) * 0.45
            striker.setTranslation (Vector.Create (sx, sy, sz), true)
            striker.setLinvel (Vector.Create (2.4 * cos angle * 6.0, 0.9, -2.4 * sin angle * 6.0), true)
            striker.setAngvel (Vector.Create (0.0, 0.0, 0.0), true)
            syncMeshes ()
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
            let (w, ps, st) = buildWorld ()
            world <- w
            planks <- ps
            striker <- st
            for _ in 1..12 do
                world.step ()
                stepCount <- stepCount + 1
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

            let camera = createArcRotateCamera (0.85, 1.12, 11.5, vec3 (0.0, 1.5, 0.0))
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

            let palette =
                [| createPbrMaterial (CreatePbrMaterialProps.Create(baseColorFactor = (0.86, 0.86, 0.86, 1.0), metallicFactor = 0.15, roughnessFactor = 0.45))
                   createPbrMaterial (CreatePbrMaterialProps.Create(baseColorFactor = (0.58, 0.58, 0.60, 1.0), metallicFactor = 0.25, roughnessFactor = 0.38))
                   createPbrMaterial (CreatePbrMaterialProps.Create(baseColorFactor = (0.30, 0.31, 0.33, 1.0), metallicFactor = 0.35, roughnessFactor = 0.35)) |]

            plankMeshes <-
                Array.init plankCount (fun idx ->
                    let layer = idx / perLayer
                    let wBox, dBox =
                        if layer % 2 = 0 then (halfLong * 2.0, halfThin * 2.0)
                        else (halfThin * 2.0, halfLong * 2.0)
                    let m = createBox (engine, BoxOptions.Create(width = wBox, height = halfHeight * 2.0, depth = dBox))
                    m.material <- palette.[layer % 3]
                    addToScene (scene, m)
                    m
                )

            let strikerMat =
                createPbrMaterial (
                    CreatePbrMaterialProps.Create(
                        baseColorFactor = (0.95, 0.42, 0.28, 1.0),
                        metallicFactor = 0.55,
                        roughnessFactor = 0.22
                    )
                )
            let sm = createSphere (engine, SphereOptions.Create(diameter = sphereRadius * 2.0, segments = 24.0))
            sm.material <- strikerMat
            addToScene (scene, sm)
            strikerMesh <- Some sm

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
