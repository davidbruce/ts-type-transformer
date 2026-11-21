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
            scene.clearColor <- GPUColorDict.Create(r = 0.05, g = 0.05, b = 0.05, a = 1.0)

            let camera = createArcRotateCamera (Math.PI / 4.0, Math.PI / 3.1, 12.8, vec3 (0.0, 1.4, 0.0))
            addToScene (scene, camera)
            attachControl (camera, canvas, scene) |> ignore

            let hemi = createHemisphericLight ((0.0, 1.0, 0.15), 0.55)
            hemi.diffuseColor <- (0.76, 0.76, 0.76)
            hemi.groundColor <- (0.10, 0.10, 0.10)
            addToScene (scene, hemi)

            let dir = createDirectionalLight ((-1.0, -2.2, -0.8), 1.35)
            dir.diffuse <- (0.90, 0.90, 0.90)
            addToScene (scene, dir)

            let light1 = createPointLight ((5.2, 4.2, 0.0), 10.0)
            light1.diffuse <- (1.0, 1.0, 1.0)
            light1.range <- 16.0
            addToScene (scene, light1)

            let light2 = createPointLight ((-5.2, 3.4, 0.0), 7.5)
            light2.diffuse <- (0.78, 0.78, 0.78)
            light2.range <- 16.0
            addToScene (scene, light2)

            let floorMat =
                createPbrMaterial (
                    CreatePbrMaterialProps.Create(
                        baseColorFactor = (0.09, 0.09, 0.09, 1.0),
                        metallicFactor = 0.10,
                        roughnessFactor = 0.88
                    )
                )
            let floor = createGround (engine, GroundOptions.Create(width = 18.0, height = 18.0))
            floor.material <- floorMat
            addToScene (scene, floor)

            let shades = [| 0.22; 0.44; 0.68; 0.88 |]
            let colMats =
                shades
                |> Array.map (fun v ->
                    createPbrMaterial (
                        CreatePbrMaterialProps.Create(
                            baseColorFactor = (v, v, v, 1.0),
                            metallicFactor = 0.35,
                            roughnessFactor = 0.36
                        )
                    ))

            let columns =
                [ for row in 0..5 do
                      for col in 0..5 do
                          let fx = float col - 2.5
                          let fz = float row - 2.5
                          let x = fx * 1.15
                          let z = fz * 1.15
                          let dist = Math.Sqrt(fx * fx + fz * fz)
                          let m = createBox (engine, BoxOptions.Create(width = 0.72, height = 1.0, depth = 0.72))
                          m.position.set (x, 0.5, z)
                          m.material <- colMats.[(row + col) % 4]
                          addToScene (scene, m)
                          yield (m, x, z, dist, fx - fz) ]

            let capProps =
                CreatePbrMaterialProps.Create(
                    baseColorFactor = (0.92, 0.92, 0.92, 1.0),
                    metallicFactor = 0.50,
                    roughnessFactor = 0.15
                )
            setPbrClearCoat (capProps, ClearCoatProps.Create(isEnabled = true, intensity = 1.0, roughness = 0.08))
            setPbrEmissive (capProps, (0.16, 0.16, 0.16))
            let capMat = createPbrMaterial capProps

            let capsules =
                [| for _ in 0..3 ->
                       let cap = createCapsule (engine, CapsuleOptions.Create(height = 1.15, radius = 0.22, tessellation = 20.0))
                       cap.material <- capMat
                       addToScene (scene, cap)
                       cap |]

            let mutable elapsed = 0.0

            onBeforeRender (
                scene,
                fun deltaMs ->
                    elapsed <- elapsed + deltaMs * 0.001
                    for (m, x, z, dist, diag) in columns do
                        let h =
                            0.55
                            + 1.15 * (0.5 + 0.5 * Math.Sin(elapsed * 2.2 - dist * 1.15))
                            + 0.30 * Math.Cos(elapsed * 1.3 + diag * 0.7)
                        m.scaling.set (1.0, h, 1.0)
                        m.position.set (x, h * 0.5, z)

                    capsules
                    |> Array.iteri (fun k cap ->
                        let fk = float k
                        let theta = elapsed * 0.7 + fk * (Math.PI / 2.0)
                        cap.position.set (Math.Cos(theta) * 4.9, 2.6 + Math.Sin(elapsed * 1.6 + fk * 1.57) * 0.55, Math.Sin(theta) * 4.9)
                        cap.rotation.z <- Math.Sin(elapsed + fk) * 0.35
                        cap.rotation.y <- -theta)

                    light1.position.set (Math.Cos(elapsed * 0.65) * 5.2, 4.2, Math.Sin(elapsed * 0.65) * 5.2)
                    light2.position.set (Math.Cos(-elapsed * 0.52) * 5.2, 3.4, Math.Sin(-elapsed * 0.52) * 5.2)
            )

            do! registerScene scene |> Async.AwaitPromise
            do! startEngine engine |> Async.AwaitPromise
        with ex ->
            (document.getElementById "status").Value.textContent <- $"WebGPU / Babylon Lite error: {ex.Message}"
    }

Async.StartImmediate main
