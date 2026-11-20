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

            let camera = createArcRotateCamera (Math.PI / 4.0, Math.PI / 3.1, 12.0, vec3 (0.0, 1.1, 0.0))
            addToScene (scene, camera)
            attachControl (camera, canvas, scene) |> ignore

            let hemi = createHemisphericLight ((0.0, 1.0, 0.2), 0.65)
            hemi.diffuseColor <- (0.82, 0.82, 0.82)
            hemi.groundColor <- (0.12, 0.12, 0.12)
            addToScene (scene, hemi)

            let dir = createDirectionalLight ((-1.0, -2.0, -1.0), 1.8)
            dir.diffuse <- (0.95, 0.95, 0.95)
            addToScene (scene, dir)

            let orbLight = createPointLight ((4.5, 3.0, 0.0), 12.0)
            orbLight.diffuse <- (1.0, 1.0, 1.0)
            orbLight.range <- 18.0
            addToScene (scene, orbLight)

            let floorMat =
                createPbrMaterial (
                    CreatePbrMaterialProps.Create(
                        baseColorFactor = (0.11, 0.11, 0.11, 1.0),
                        metallicFactor = 0.08,
                        roughnessFactor = 0.88
                    )
                )

            let floor = createGround (engine, GroundOptions.Create(width = 14.0, height = 14.0))
            floor.material <- floorMat
            addToScene (scene, floor)

            let plinthMat =
                createPbrMaterial (
                    CreatePbrMaterialProps.Create(
                        baseColorFactor = (0.17, 0.17, 0.17, 1.0),
                        metallicFactor = 0.12,
                        roughnessFactor = 0.78
                    )
                )

            let plinth = createBox (engine, BoxOptions.Create(width = 8.4, height = 0.08, depth = 8.4))
            plinth.position.set (0.0, 0.04, 0.0)
            plinth.material <- plinthMat
            addToScene (scene, plinth)

            for row in 0..4 do
                for col in 0..4 do
                    let roughness = 0.08 + 0.22 * float row
                    let metallic = 0.25 * float col
                    let mat =
                        createPbrMaterial (
                            CreatePbrMaterialProps.Create(
                                baseColorFactor = (0.85, 0.85, 0.85, 1.0),
                                metallicFactor = metallic,
                                roughnessFactor = roughness
                            )
                        )
                    let sphere = createSphere (engine, SphereOptions.Create(diameter = 1.05, segments = 24.0))
                    sphere.position.set (float (col - 2) * 1.55, 0.75, float (row - 2) * 1.55)
                    sphere.material <- mat
                    addToScene (scene, sphere)

            let orbProps =
                CreatePbrMaterialProps.Create(
                    baseColorFactor = (0.95, 0.95, 0.95, 1.0),
                    metallicFactor = 0.0,
                    roughnessFactor = 0.2
                )
            setPbrEmissive (orbProps, (0.95, 0.95, 0.95))
            let orbMat = createPbrMaterial orbProps

            let orb = createSphere (engine, SphereOptions.Create(diameter = 0.36, segments = 16.0))
            orb.position.set (4.5, 3.0, 0.0)
            orb.material <- orbMat
            addToScene (scene, orb)

            let mutable elapsed = 0.0

            onBeforeRender (
                scene,
                fun deltaMs ->
                    elapsed <- elapsed + deltaMs * 0.001
                    let x = Math.Cos(elapsed * 0.85) * 4.5
                    let y = 3.0 + Math.Sin(elapsed * 1.4) * 0.65
                    let z = Math.Sin(elapsed * 0.85) * 4.5
                    orb.position.set (x, y, z)
                    orbLight.position.set (x, y, z)
            )

            do! registerScene scene |> Async.AwaitPromise
            do! startEngine engine |> Async.AwaitPromise
        with ex ->
            (document.getElementById "status").textContent <- $"WebGPU / Babylon Lite error: {ex.Message}"
    }

Async.StartImmediate main
