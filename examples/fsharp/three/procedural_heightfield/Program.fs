module Program

open System
open Fable.Core
open Fable.Core.JsInterop
open Dom
open type Dom.Globals
open Three

let mutable private waveMode = 0
let mutable private overlayOn = true
let mutable private azimuth = 0.62

let private waveHeight (mode: int) (x: float) (z: float) =
    if mode = 1 then
        let r = sqrt (x * x + z * z)
        0.78 * cos (r * 2.2) / (1.0 + 0.18 * r * r)
    elif mode = 2 then
        0.065 * (x * x - z * z) + 0.18 * sin (x * 1.6)
    else
        0.55 * cos (x * 1.4) * sin (z * 1.4) + 0.25 * cos ((x + z) * 1.1)

let private waveName (mode: int) =
    if mode = 1 then "Radial Ripple"
    elif mode = 2 then "Saddle Ridge"
    else "Interference"

let main () =
    let width = 680.0
    let height = 400.0
    let gridN = 28

    let renderer =
        WebGLRenderer.Create(
            WebGLRendererParameters.Create(antialias = true, preserveDrawingBuffer = true)
        )
    renderer.setPixelRatio 1.0
    renderer.setSize (width, height)
    (document.getElementById "stage").Value.appendChild renderer.domElement |> ignore

    let scene = Scene.Create()
    scene.background <- Some (ColorOrTexture.Color (Color.Create (float 0xf7f7f7)))

    let camera = PerspectiveCamera.Create(42.0, width / height, 0.1, 100.0)

    scene.add (AmbientLight.Create(float 0xffffff, 1.2)) |> ignore
    let keyLight = DirectionalLight.Create(float 0xffffff, 2.2)
    keyLight.position.set (6.0, 10.0, 8.0) |> ignore
    scene.add (keyLight) |> ignore
    let rimLight = DirectionalLight.Create(float 0xffffff, 0.9)
    rimLight.position.set (-7.0, 5.0, -6.0) |> ignore
    scene.add (rimLight) |> ignore

    let grid =
        GridHelper.Create(
            9.0,
            12.0,
            !^ (float 0x999999),
            !^ (float 0xe5e5e5)
        )
    grid.position.y <- -1.1
    scene.add (grid) |> ignore

    let geo = BufferGeometry.Create()
    let vertCount = gridN * gridN
    let posAttr = Float32BufferAttribute.Create(float (vertCount * 3), 3.0)

    let indices = ResizeArray<float>()
    for iz in 0 .. gridN - 2 do
        for ix in 0 .. gridN - 2 do
            let a = float (iz * gridN + ix)
            let b = a + 1.0
            let c = a + float gridN
            let d = c + 1.0
            indices.Add a
            indices.Add c
            indices.Add b
            indices.Add b
            indices.Add c
            indices.Add d

    geo.setAttribute ("position", posAttr :> BufferAttribute<BufferAttributeEventMap>) |> ignore
    geo.setIndex (BufferAttributeOrNumberArray.Array indices) |> ignore

    let surfMat =
        MeshStandardMaterial.Create(
            MeshStandardMaterialParameters.Create(
                color = !^ (float 0xb5b5b5),
                roughness = 0.72,
                metalness = 0.05,
                flatShading = true
            )
        )
    let surfaceMesh = Mesh.Create(geo, surfMat)
    scene.add (surfaceMesh) |> ignore

    let wireMat =
        MeshBasicMaterial.Create(
            MeshBasicMaterialParameters.Create(
                color = !^ (float 0x262626),
                wireframe = true
            )
        )
    let wireMesh = Mesh.Create(geo, wireMat)
    wireMesh.position.y <- 0.015
    scene.add (wireMesh) |> ignore

    let ptCount = (gridN / 2) * (gridN / 2)
    let ptsGeo = BufferGeometry.Create()
    let ptsAttr = Float32BufferAttribute.Create(float (ptCount * 3), 3.0)
    ptsGeo.setAttribute ("position", ptsAttr :> BufferAttribute<BufferAttributeEventMap>) |> ignore
    let ptsMat =
        PointsMaterial.Create(
            PointsMaterialParameters.Create(
                color = !^ (float 0x111111),
                size = 0.14
            )
        )
    let ptsObj = Points.Create(ptsGeo, ptsMat)
    scene.add (ptsObj) |> ignore

    let wireBtn = (document.getElementById "btn-wire").Value
    let metaEl = (document.getElementById "meta").Value

    let rebuildField () =
        let span = 7.6
        let half = span / 2.0
        for iz in 0 .. gridN - 1 do
            for ix in 0 .. gridN - 1 do
                let idx = iz * gridN + ix
                let x = -half + span * float ix / float (gridN - 1)
                let z = -half + span * float iz / float (gridN - 1)
                let y = waveHeight waveMode x z
                posAttr.setXYZ (float idx, x, y, z) |> ignore

        let mutable pi = 0
        for iz in 0 .. 2 .. gridN - 1 do
            for ix in 0 .. 2 .. gridN - 1 do
                let x = -half + span * float ix / float (gridN - 1)
                let z = -half + span * float iz / float (gridN - 1)
                let y = waveHeight waveMode x z + 0.06
                ptsAttr.setXYZ (float pi, x, y, z) |> ignore
                pi <- pi + 1

        posAttr.needsUpdate <- true
        ptsAttr.needsUpdate <- true
        geo.computeVertexNormals ()
        geo.computeBoundingBox ()

        let radius = 9.2
        camera.position.set (radius * sin azimuth, 5.4, radius * cos azimuth) |> ignore
        camera.lookAt (Vector3.Create(0.0, 0.0, 0.0))
        wireMesh.visible <- overlayOn
        wireBtn.textContent <- if overlayOn then "Wireframe: On" else "Wireframe: Off"
        renderer.render (scene, camera)

        let bb = geo.boundingBox.Value
        metaEl.textContent <-
            sprintf
                "profile: %s · %d vertices · %d triangles · y=[%.2f, %.2f] · wireframe: %s"
                (waveName waveMode)
                vertCount
                (indices.Count / 3)
                bb.min.y
                bb.max.y
                (if overlayOn then "on" else "off")

    let wireClick (id: string) (fn: unit -> unit) =
        let b = (document.getElementById id).Value
        b.addEventListener (HTMLElementEventMapKeys.click, fun _ -> fn ())

    wireClick "btn-p0" (fun () ->
        waveMode <- 0
        rebuildField ()
    )
    wireClick "btn-p1" (fun () ->
        waveMode <- 1
        rebuildField ()
    )
    wireClick "btn-p2" (fun () ->
        waveMode <- 2
        rebuildField ()
    )
    wireClick "btn-wire" (fun () ->
        overlayOn <- not overlayOn
        rebuildField ()
    )
    wireClick "btn-orbit" (fun () ->
        azimuth <- azimuth + Math.PI / 4.0
        rebuildField ()
    )

    rebuildField ()

main ()
