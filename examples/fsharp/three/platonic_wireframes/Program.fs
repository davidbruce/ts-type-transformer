module Program

open System
open Fable.Core
open Fable.Core.JsInterop
open Dom
open type Dom.Globals
open Three

let mutable private wireVisible = true
let mutable private azimuth = 0.35

let private wireButtonText () =
    if wireVisible then "Wireframes: On" else "Wireframes: Off"

let main () =
    let width = 680.0
    let height = 400.0

    let rendererParams =
        WebGLRendererParameters.Create(
            antialias = true,
            preserveDrawingBuffer = true
        )
    let renderer = WebGLRenderer.Create rendererParams
    renderer.setPixelRatio 1.0
    renderer.setSize (width, height)

    let stage = document.getElementById "stage"
    stage.appendChild renderer.domElement |> ignore

    let scene = Scene.Create()
    scene.background <- ColorOrTexture.Color (Color.Create (float 0xf7f7f7))

    let camera = PerspectiveCamera.Create(42.0, width / height, 0.1, 100.0)

    let ambient = AmbientLight.Create(float 0xffffff, 1.2)
    scene.add (ambient) |> ignore

    let keyLight = DirectionalLight.Create(float 0xffffff, 2.2)
    keyLight.position.set (6.0, 10.0, 8.0) |> ignore
    scene.add (keyLight) |> ignore

    let rimLight = DirectionalLight.Create(float 0xffffff, 0.9)
    rimLight.position.set (-7.0, 5.0, -6.0) |> ignore
    scene.add (rimLight) |> ignore

    let grid =
        GridHelper.Create(
            12.0,
            12.0,
            !^ (float 0x737373),
            !^ (float 0xd4d4d4)
        )
    grid.position.y <- -1.2
    scene.add (grid) |> ignore

    let wireGroup = Group.Create()
    scene.add (wireGroup) |> ignore

    let lineMat =
        LineBasicMaterial.Create(
            LineBasicMaterialParameters.Create(color = !^ (float 0x111111))
        )

    let addSolid
        (geo: BufferGeometry<NormalBufferAttributes, BufferGeometryEventMap>)
        (hex: int)
        (rough: float)
        (metal: float)
        (flat: bool)
        (x: float)
        (y: float)
        (z: float)
        (edgeThresh: float)
        =
        let mat =
            MeshStandardMaterial.Create(
                MeshStandardMaterialParameters.Create(
                    color = !^ (float hex),
                    roughness = rough,
                    metalness = metal,
                    flatShading = flat
                )
            )
        let mesh = Mesh.Create(geo, mat)
        mesh.position.set (x, y, z) |> ignore
        mesh.rotation.set (0.25, 0.45, 0.0) |> ignore
        scene.add (mesh) |> ignore

        let edgesGeo = EdgesGeometry.Create(geo, edgeThresh)
        let segs =
            LineSegments.Create(
                edgesGeo,
                lineMat
            )
        segs.position.set (x, y, z) |> ignore
        segs.rotation.set (0.25, 0.45, 0.0) |> ignore
        wireGroup.add (segs) |> ignore

    addSolid (BoxGeometry.Create(1.45, 1.45, 1.45)) 0x262626 0.65 0.1 false -3.2 0.0 -0.6 1.0
    addSolid (IcosahedronGeometry.Create(0.95, 0.0)) 0xd4d4d4 0.55 0.08 true -1.55 0.1 0.55 1.0
    addSolid (TorusKnotGeometry.Create(0.72, 0.24, 96.0, 16.0)) 0x737373 0.45 0.15 false 0.0 0.25 -0.2 24.0
    addSolid (OctahedronGeometry.Create(0.95, 0.0)) 0xefefef 0.5 0.05 true 1.6 0.1 0.55 1.0
    addSolid (TorusGeometry.Create(0.72, 0.23, 16.0, 48.0)) 0x404040 0.55 0.12 false 3.2 0.05 -0.5 22.0

    let metaEl = document.getElementById "meta"
    let wireBtn = document.getElementById "btn-wire"
    let wireClick (id: string) (fn: unit -> unit) =
        let b = document.getElementById id
        b.addEventListener (HTMLElementEventMapKeys.click, fun _ -> fn ())

    let updateView () =
        let radius = 8.6
        camera.position.set (radius * sin azimuth, 3.6, radius * cos azimuth) |> ignore
        camera.lookAt (Vector3.Create(0.0, 0.0, 0.0))
        wireGroup.visible <- wireVisible
        wireBtn.textContent <- wireButtonText ()
        renderer.render (scene, camera)
        let deg = int (Math.Round(azimuth * 180.0 / Math.PI))
        let wLabel = if wireVisible then "on" else "off"
        metaEl.textContent <-
            sprintf "5 monochrome solids · EdgesGeometry wireframes: %s · azimuth: %d deg · 680x400 WebGL" wLabel deg

    wireClick "btn-wire" (fun () ->
        wireVisible <- not wireVisible
        updateView ()
    )
    wireClick "btn-left" (fun () ->
        azimuth <- azimuth - Math.PI / 6.0
        updateView ()
    )
    wireClick "btn-right" (fun () ->
        azimuth <- azimuth + Math.PI / 6.0
        updateView ()
    )
    wireClick "btn-reset" (fun () ->
        azimuth <- 0.35
        wireVisible <- true
        updateView ()
    )

    updateView ()

main ()
