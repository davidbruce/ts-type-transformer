module Demos.Three.InstancedCityscape

open System
open Fable.Core
open Fable.Core.JsInterop
open Dom
open type Dom.Globals
open Three

let mutable private useOrtho = true
let mutable private layoutMode = 0
let mutable private camYaw = 0.785398

let private layoutName (m: int) =
    if m = 1 then "Amphitheater"
    elif m = 2 then "Diagonal"
    else "Plaza"

let private towerHeight (m: int) (gx: int) (gz: int) =
    let fx = float gx - 4.5
    let fz = float gz - 4.5
    if m = 1 then
        let r = sqrt (fx * fx + fz * fz)
        0.35 + 0.42 * r + 0.25 * cos (fx * 0.8)
    elif m = 2 then
        let diag = abs (fx - fz)
        0.3 + 2.8 * exp (-diag * diag / 5.5) + 0.25 * sin (fz * 0.9)
    else
        let r2 = fx * fx + fz * fz
        0.35 + 3.1 * exp (-r2 / 9.5) + 0.2 * cos (fx * 1.4) * cos (fz * 1.4)

let main () =
    let width = 680.0
    let height = 400.0
    let aspect = width / height

    let renderer =
        WebGLRenderer.Create(
            WebGLRendererParameters.Create(antialias = true, preserveDrawingBuffer = true)
        )
    renderer.setPixelRatio 1.0
    renderer.setSize (width, height)
    (document.getElementById "stage").Value.appendChild renderer.domElement |> ignore

    let scene = Scene.Create()
    scene.background <- Some (ColorOrTexture.Color (Color.Create (float 0xf7f7f7)))

    let orthoCam = OrthographicCamera.Create(-5.4 * aspect, 5.4 * aspect, 5.4, -5.4, 0.1, 100.0)
    let perspCam = PerspectiveCamera.Create(40.0, aspect, 0.1, 100.0)

    scene.add (AmbientLight.Create(float 0xffffff, 1.25)) |> ignore
    let keyLight = DirectionalLight.Create(float 0xffffff, 2.1)
    keyLight.position.set (8.0, 12.0, 6.0) |> ignore
    scene.add (keyLight) |> ignore
    let rimLight = DirectionalLight.Create(float 0xffffff, 0.9)
    rimLight.position.set (-7.0, 6.0, -8.0) |> ignore
    scene.add (rimLight) |> ignore

    let plinth =
        Mesh.Create(
            PlaneGeometry.Create(8.6, 8.6),
            MaterialOrMaterialArray.Material (MeshStandardMaterial.Create(
                MeshStandardMaterialParameters.Create(
                    color = float 0xe5e5e5,
                    roughness = 0.85
                )
            ))
        )
    plinth.rotation.x <- -Math.PI / 2.0
    plinth.position.y <- -0.01
    scene.add (plinth) |> ignore

    let grid =
        GridHelper.Create(
            9.0,
            10.0,
            (float 0x999999),
            (float 0xd4d4d4)
        )
    scene.add (grid) |> ignore

    let towerGeo = BoxGeometry.Create(0.54, 1.0, 0.54)
    let towerMat =
        MeshStandardMaterial.Create(
            MeshStandardMaterialParameters.Create(
                color = float 0xffffff,
                roughness = 0.65,
                metalness = 0.08
            )
        )
    let inst =
        InstancedMesh.Create(
            Some towerGeo,
            Some (MaterialOrMaterialArray.Material towerMat),
            100.0
        )
    scene.add (inst) |> ignore

    let camBtn = (document.getElementById "btn-cam").Value
    let metaEl = (document.getElementById "meta").Value

    let pos = Vector3.Create ()
    let rotQ = Quaternion.Create ()
    let scale = Vector3.Create ()
    let mtx = Matrix4.Create ()
    let axisY = Vector3.Create(0.0, 1.0, 0.0)
    let col = Color.Create ()

    let renderCity () =
        let mutable maxH = 0.0
        let mutable idx = 0
        for gz in 0 .. 9 do
            for gx in 0 .. 9 do
                let x = (float gx - 4.5) * 0.78
                let z = (float gz - 4.5) * 0.78
                let h = towerHeight layoutMode gx gz
                if h > maxH then maxH <- h
                pos.set (x, h * 0.5, z) |> ignore
                rotQ.setFromAxisAngle (axisY, 0.08 * float ((gx + gz) % 4)) |> ignore
                scale.set (1.0, h, 1.0) |> ignore
                mtx.compose (pos, rotQ, scale) |> ignore
                inst.setMatrixAt (float idx, mtx) |> ignore
                let t = min 1.0 (max 0.0 ((h - 0.3) / 3.2))
                let shade = 0.82 - 0.70 * t
                col.setRGB (shade, shade, shade) |> ignore
                inst.setColorAt (float idx, col) |> ignore
                idx <- idx + 1

        inst.instanceMatrix.needsUpdate <- true
        match inst.instanceColor with
        | Some color -> color.needsUpdate <- true
        | None -> ()

        let radius = 11.5
        let cx = radius * cos camYaw
        let cz = radius * sin camYaw
        orthoCam.position.set (cx, 8.2, cz) |> ignore
        orthoCam.lookAt (Vector3.Create(0.0, 0.9, 0.0))
        perspCam.position.set (cx, 7.6, cz) |> ignore
        perspCam.lookAt (Vector3.Create(0.0, 0.9, 0.0))

        let activeCam: Camera = if useOrtho then (orthoCam :> Camera) else (perspCam :> Camera)
        renderer.render (scene, activeCam)

        let camName = if useOrtho then "Orthographic" else "Perspective"
        camBtn.textContent <- sprintf "Camera: %s" camName
        metaEl.textContent <-
            sprintf "100 InstancedMesh towers · layout: %s · peak height: %.2f · camera: %s"
                (layoutName layoutMode) maxH camName

    let wireClick (id: string) (fn: unit -> unit) =
        let b = (document.getElementById id).Value
        b.addEventListener (HTMLElementEventMapKeys.click, fun _ -> fn ())

    wireClick "btn-cam" (fun () ->
        useOrtho <- not useOrtho
        renderCity ()
    )
    wireClick "btn-lay0" (fun () ->
        layoutMode <- 0
        renderCity ()
    )
    wireClick "btn-lay1" (fun () ->
        layoutMode <- 1
        renderCity ()
    )
    wireClick "btn-lay2" (fun () ->
        layoutMode <- 2
        renderCity ()
    )
    wireClick "btn-rot" (fun () ->
        camYaw <- camYaw + Math.PI / 4.0
        renderCity ()
    )

    renderCity ()

main ()
