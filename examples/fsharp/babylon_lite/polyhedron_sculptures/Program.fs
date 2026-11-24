module Demos.BabylonLite.PolyhedronSculptures

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

            let camera = createArcRotateCamera (Math.PI / 2.0, Math.PI / 2.8, 11.5, vec3 (0.0, 1.6, 0.0))
            addToScene (scene, camera)
            attachControl (camera, canvas, scene) |> ignore

            let hemi = createHemisphericLight ((0.0, 1.0, 0.0), 0.42)
            hemi.diffuseColor <- (0.70, 0.70, 0.70)
            hemi.groundColor <- (0.08, 0.08, 0.08)
            addToScene (scene, hemi)

            let spot = createSpotLight ((0.0, 6.2, 3.0), (0.0, -1.0, -0.55), Math.PI / 3.2, 3.0, 26.0)
            spot.diffuse <- (1.0, 1.0, 1.0)
            spot.range <- 22.0
            addToScene (scene, spot)

            let fill = createDirectionalLight ((1.2, -1.5, -1.0), 0.65)
            fill.diffuse <- (0.80, 0.80, 0.80)
            addToScene (scene, fill)

            let floorMat = createStandardMaterial ()
            floorMat.diffuseColor <- (0.10, 0.10, 0.10)
            floorMat.specularColor <- (0.25, 0.25, 0.25)
            floorMat.specularPower <- 24.0
            let floor = createGround (engine, GroundOptions.Create(width = 18.0, height = 18.0))
            floor.material <- floorMat
            addToScene (scene, floor)

            let plinthMat = createStandardMaterial ()
            plinthMat.diffuseColor <- (0.18, 0.18, 0.18)
            plinthMat.specularColor <- (0.30, 0.30, 0.30)
            plinthMat.specularPower <- 32.0

            let sculptures =
                [| for i in 0..4 ->
                       let fi = float i
                       let x = (fi - 2.0) * 2.25
                       let z = -Math.Abs(fi - 2.0) * 0.55

                       let plinth = createCylinder (engine, CylinderOptions.Create(height = 1.1, diameter = 1.15, tessellation = 28.0))
                       plinth.position.set (x, 0.55, z)
                       plinth.material <- plinthMat
                       addToScene (scene, plinth)

                       let v = 0.90 - fi * 0.13
                       let polyMat = createStandardMaterial ()
                       polyMat.diffuseColor <- (v, v, v)
                       polyMat.specularColor <- (0.95, 0.95, 0.95)
                       polyMat.specularPower <- 18.0 + fi * 24.0

                       let mesh = createPolyhedron (engine, PolyhedronOptions.Create(``type`` = fi, size = 0.62, flat = true))
                       mesh.position.set (x, 2.05, z)
                       mesh.material <- polyMat
                       addToScene (scene, mesh)
                       mesh |]

            let mutable elapsed = 0.0

            onBeforeRender (
                scene,
                fun deltaMs ->
                    elapsed <- elapsed + deltaMs * 0.001
                    let sx = Math.Sin(elapsed * 0.65) * 5.2
                    spot.position.set (sx, 6.2, 3.0)
                    spot.direction.set (-sx * 0.15, -1.0, -0.55)
                    sculptures
                    |> Array.iteri (fun i m ->
                        let fi = float i
                        m.position.y <- 2.05 + Math.Sin(elapsed * 1.3 + fi * 1.1) * 0.18
                        m.rotation.y <- elapsed * (0.45 + 0.12 * fi)
                        m.rotation.x <- Math.Sin(elapsed * 0.7 + fi) * 0.35)
            )

            do! registerScene scene |> Async.AwaitPromise
            do! startEngine engine |> Async.AwaitPromise
        with ex ->
            (document.getElementById "status").Value.textContent <- $"WebGPU / Babylon Lite error: {ex.Message}"
    }

Async.StartImmediate main
