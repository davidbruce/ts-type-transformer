module Program

open System
open Fable.Core
open Dom
open Babylonjs.Lite
open type Babylonjs.Lite.Globals

let main =
    async {
        try
            let canvas =
                match document.querySelector HTMLElementTagNameMapKeys.canvas with
                | Some c -> c
                | None -> failwith "Canvas not found"
            let! engine = createEngine canvas |> Async.AwaitPromise
            let scene = createSceneContext engine
            scene.clearColor <- GPUColorDict.Create(r = 0.06, g = 0.06, b = 0.06, a = 1.0)

            let camera = createArcRotateCamera (Math.PI / 4.2, Math.PI / 2.9, 8.8, vec3 (0.0, 2.1, 0.0))
            addToScene (scene, camera)
            attachControl (camera, canvas, scene) |> ignore

            let hemi = createHemisphericLight ((0.0, 1.0, 0.2), 0.72)
            hemi.diffuseColor <- (0.85, 0.85, 0.85)
            hemi.groundColor <- (0.14, 0.14, 0.14)
            addToScene (scene, hemi)

            let dir = createDirectionalLight ((-1.2, -2.0, -1.1), 2.1)
            dir.diffuse <- (1.0, 1.0, 1.0)
            addToScene (scene, dir)

            let keyLight = createPointLight ((3.2, 3.6, 2.4), 8.0)
            keyLight.diffuse <- (0.92, 0.92, 0.92)
            keyLight.range <- 14.0
            addToScene (scene, keyLight)

            let floorMat =
                createPbrMaterial (
                    CreatePbrMaterialProps.Create(
                        baseColorFactor = (0.09, 0.09, 0.09, 1.0),
                        metallicFactor = 0.10,
                        roughnessFactor = 0.88
                    )
                )
            let floor = createGround (engine, GroundOptions.Create(width = 16.0, height = 16.0))
            floor.material <- floorMat
            addToScene (scene, floor)

            let discMat =
                createPbrMaterial (
                    CreatePbrMaterialProps.Create(
                        baseColorFactor = (0.22, 0.22, 0.22, 1.0),
                        metallicFactor = 0.25,
                        roughnessFactor = 0.65
                    )
                )
            let disc = createDisc (engine, DiscOptions.Create(radius = 2.6, tessellation = 48.0))
            disc.rotation.x <- Math.PI / 2.0
            disc.position.set (0.0, 0.03, 0.0)
            disc.material <- discMat
            addToScene (scene, disc)

            let srcBox = createBox (engine, BoxOptions.Create(size = 2.1))
            let cylY = createCylinder (engine, CylinderOptions.Create(height = 3.0, diameter = 1.18, tessellation = 16.0))
            let cylX = createCylinder (engine, CylinderOptions.Create(height = 3.0, diameter = 1.18, tessellation = 16.0))
            cylX.rotation.z <- Math.PI / 2.0
            let cylZ = createCylinder (engine, CylinderOptions.Create(height = 3.0, diameter = 1.18, tessellation = 16.0))
            cylZ.rotation.x <- Math.PI / 2.0

            let s1 = csgSubtract (createCsgFromMesh srcBox, createCsgFromMesh cylY)
            let s2 = csgSubtract (s1, createCsgFromMesh cylX)
            let s3 = csgSubtract (s2, createCsgFromMesh cylZ)
            let monolith = createMeshFromCsg (engine, s3, "monolith")
            monolith.position.set (0.0, 2.15, 0.0)

            let monoProps =
                CreatePbrMaterialProps.Create(
                    baseColorFactor = (0.86, 0.86, 0.86, 1.0),
                    metallicFactor = 0.65,
                    roughnessFactor = 0.24
                )
            setPbrClearCoat (monoProps, ClearCoatProps.Create(isEnabled = true, intensity = 0.90, roughness = 0.10))
            monolith.material <- createPbrMaterial monoProps
            addToScene (scene, monolith)

            let coreBox = createBox (engine, BoxOptions.Create(size = 0.82))
            let coreSphere = createSphere (engine, SphereOptions.Create(diameter = 1.02, segments = 14.0))
            let coreSolid = csgIntersect (createCsgFromMesh coreBox, createCsgFromMesh coreSphere)
            let core = createMeshFromCsg (engine, coreSolid, "core")
            core.position.set (0.0, 2.15, 0.0)
            core.material <-
                createPbrMaterial (
                    CreatePbrMaterialProps.Create(
                        baseColorFactor = (0.18, 0.18, 0.18, 1.0),
                        metallicFactor = 0.85,
                        roughnessFactor = 0.16
                    )
                )
            addToScene (scene, core)

            let outerRing = createTorus (engine, TorusOptions.Create(diameter = 4.2, thickness = 0.09, tessellation = 48.0))
            outerRing.position.set (0.0, 2.15, 0.0)
            outerRing.material <-
                createPbrMaterial (
                    CreatePbrMaterialProps.Create(
                        baseColorFactor = (0.55, 0.55, 0.55, 1.0),
                        metallicFactor = 0.80,
                        roughnessFactor = 0.25
                    )
                )
            addToScene (scene, outerRing)

            let innerRing = createTorus (engine, TorusOptions.Create(diameter = 3.55, thickness = 0.07, tessellation = 48.0))
            innerRing.position.set (0.0, 2.15, 0.0)
            innerRing.material <-
                createPbrMaterial (
                    CreatePbrMaterialProps.Create(
                        baseColorFactor = (0.88, 0.88, 0.88, 1.0),
                        metallicFactor = 0.40,
                        roughnessFactor = 0.30
                    )
                )
            addToScene (scene, innerRing)

            let mutable elapsed = 0.0

            onBeforeRender (
                scene,
                fun deltaMs ->
                    elapsed <- elapsed + deltaMs * 0.001
                    monolith.rotation.y <- elapsed * 0.35
                    monolith.rotation.x <- Math.Sin(elapsed * 0.3) * 0.20
                    core.rotation.y <- -elapsed * 0.90
                    core.rotation.z <- elapsed * 0.50
                    outerRing.rotation.x <- elapsed * 0.42
                    outerRing.rotation.y <- elapsed * 0.25
                    innerRing.rotation.z <- -elapsed * 0.58
                    innerRing.rotation.x <- elapsed * 0.31
            )

            do! registerScene scene |> Async.AwaitPromise
            do! startEngine engine |> Async.AwaitPromise
        with ex ->
            (document.getElementById "status").textContent <- $"WebGPU / Babylon Lite error: {ex.Message}"
    }

Async.StartImmediate main
