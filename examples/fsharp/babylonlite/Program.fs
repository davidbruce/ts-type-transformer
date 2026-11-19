module Program

open System
open Fable.Core
open Dom
open Babylonjs.Lite
open type Babylonjs.Lite.Globals

let main =
    async {
        try
            let canvas = (document.querySelector HTMLElementTagNameMapKeys.canvas).Value
            let! engine = createEngine canvas |> Async.AwaitPromise
            let scene = createSceneContext engine
            scene.clearColor <- GPUColorDict.Create(r = 0.05, g = 0.07, b = 0.11, a = 1.0)

            // Camera + orbit controls.
            let camera = createArcRotateCamera (Math.PI / 4.0, Math.PI / 3.0, 8.5, vec3 (0.0, 1.6, 0.0))
            addToScene (scene, camera)
            attachControl (camera, canvas, scene) |> ignore

            // Lights (subtypes of LightBase pass directly to addToScene).
            let hemi = createHemisphericLight ((0.0, 1.0, 0.2), 0.75)
            hemi.diffuseColor <- (0.75, 0.85, 1.0)
            hemi.groundColor <- (0.12, 0.14, 0.22)
            addToScene (scene, hemi)

            let dir = createDirectionalLight ((-1.0, -2.0, -1.2), 2.2)
            dir.diffuse <- (1.0, 0.96, 0.9)
            addToScene (scene, dir)

            let orbLight = createPointLight ((2.8, 1.6, 0.0), 8.0)
            orbLight.diffuse <- (1.0, 0.45, 0.35)
            orbLight.range <- 12.0
            addToScene (scene, orbLight)

            // Center metallic torus knot.
            let knotMat =
                createPbrMaterial (
                    CreatePbrMaterialProps.Create(
                        baseColorFactor = (0.18, 0.78, 0.95, 1.0),
                        metallicFactor = 0.75,
                        roughnessFactor = 0.22
                    )
                )

            let knot =
                createTorusKnot (
                    engine,
                    TorusKnotOptions.Create(
                        radius = 1.15,
                        tube = 0.34,
                        radialSegments = 48.0,
                        tubularSegments = 160.0
                    )
                )

            knot.position.set (0.0, 2.0, 0.0)
            knot.material <- knotMat
            addToScene (scene, knot)

            // Orbiting coral sphere synced with the point light.
            let orbMat =
                createPbrMaterial (
                    CreatePbrMaterialProps.Create(
                        baseColorFactor = (1.0, 0.42, 0.32, 1.0),
                        metallicFactor = 0.2,
                        roughnessFactor = 0.35
                    )
                )

            let orb = createSphere (engine, SphereOptions.Create(diameter = 0.55, segments = 32.0))
            orb.material <- orbMat
            addToScene (scene, orb)

            // Subtle procedural grid ground.
            let groundMat =
                createGridMaterial (
                    GridMaterialOptions.Create(
                        mainColor = (0.07, 0.09, 0.14),
                        lineColor = (0.22, 0.48, 0.75),
                        gridRatio = 0.5,
                        majorUnitFrequency = 4.0,
                        minorUnitVisibility = 0.35
                    )
                )

            let ground = createGround (engine, GroundOptions.Create(width = 16.0, height = 16.0))
            ground.material <- groundMat
            addToScene (scene, ground)

            // Animate each frame.
            let mutable elapsed = 0.0

            onBeforeRender (
                scene,
                fun deltaMs ->
                    elapsed <- elapsed + deltaMs * 0.001
                    knot.rotation.y <- elapsed * 0.55
                    knot.rotation.x <- Math.Sin(elapsed * 0.4) * 0.25
                    let x = Math.Cos(elapsed * 0.9) * 2.8
                    let y = 2.0 + Math.Sin(elapsed * 1.8) * 0.45
                    let z = Math.Sin(elapsed * 0.9) * 2.8
                    orb.position.set (x, y, z)
                    orbLight.position.set (x, y, z)
            )

            do! registerScene scene |> Async.AwaitPromise
            do! startEngine engine |> Async.AwaitPromise
        with ex ->
            (document.getElementById "status").textContent <- $"WebGPU / Babylon Lite error: {ex.Message}"
    }

Async.StartImmediate main
