module Demos.Three.RaycasterPicking

open System
open Fable.Core
open Fable.Core.JsInterop
open Dom
open type Dom.Globals
open Three

let mutable private yaw = 0.45
let mutable private selectedIndex = -1
let mutable private lastHitInfo = "none"

let main () =
    let width = 680.0
    let height = 400.0

    let renderer =
        WebGLRenderer.Create(
            WebGLRendererParameters.Create(antialias = true, preserveDrawingBuffer = true)
        )
    renderer.setPixelRatio 1.0
    renderer.setSize (width, height)
    let canvas = renderer.domElement
    (document.getElementById "stage").Value.appendChild canvas |> ignore

    let scene = Scene.Create()
    scene.background <- Some (ColorOrTexture.Color (Color.Create (float 0xf7f7f7)))

    let camera = PerspectiveCamera.Create(42.0, width / height, 0.1, 100.0)

    scene.add (AmbientLight.Create(float 0xffffff, 1.2)) |> ignore
    let keyLight = DirectionalLight.Create(float 0xffffff, 2.2)
    keyLight.position.set (6.0, 10.0, 8.0) |> ignore
    scene.add keyLight |> ignore
    let rimLight = DirectionalLight.Create(float 0xffffff, 0.9)
    rimLight.position.set (-7.0, 5.0, -6.0) |> ignore
    scene.add rimLight |> ignore

    let grid =
        GridHelper.Create(
            9.0,
            9.0,
            (float 0x999999),
            (float 0xd4d4d4)
        )
    grid.position.y <- -0.85
    scene.add grid |> ignore

    let geos: BufferGeometry<NormalBufferAttributes, BufferGeometryEventMap>[] =
        [|
            BoxGeometry.Create(1.05, 1.05, 1.05) :> BufferGeometry<NormalBufferAttributes, BufferGeometryEventMap>
            ConeGeometry.Create(0.62, 1.25, 20.0) :> BufferGeometry<NormalBufferAttributes, BufferGeometryEventMap>
            CylinderGeometry.Create(0.48, 0.48, 1.2, 24.0) :> BufferGeometry<NormalBufferAttributes, BufferGeometryEventMap>
            SphereGeometry.Create(0.62, 24.0, 18.0) :> BufferGeometry<NormalBufferAttributes, BufferGeometryEventMap>
            OctahedronGeometry.Create(0.68, 0.0) :> BufferGeometry<NormalBufferAttributes, BufferGeometryEventMap>
            DodecahedronGeometry.Create(0.62, 0.0) :> BufferGeometry<NormalBufferAttributes, BufferGeometryEventMap>
            TorusGeometry.Create(0.5, 0.18, 14.0, 36.0) :> BufferGeometry<NormalBufferAttributes, BufferGeometryEventMap>
            IcosahedronGeometry.Create(0.64, 0.0) :> BufferGeometry<NormalBufferAttributes, BufferGeometryEventMap>
            TetrahedronGeometry.Create(0.72, 0.0) :> BufferGeometry<NormalBufferAttributes, BufferGeometryEventMap>
        |]
    let names =
        [| "Box"; "Cone"; "Cylinder"; "Sphere"; "Octahedron"; "Dodecahedron"; "Torus"; "Icosahedron"; "Tetrahedron" |]

    let mats = ResizeArray<MeshStandardMaterial>()
    let meshes = ResizeArray<Object3D<Object3DEventMap>>()
    for i in 0 .. 8 do
        let gx = i % 3 - 1
        let gz = i / 3 - 1
        let mat =
            MeshStandardMaterial.Create(
                MeshStandardMaterialParameters.Create(
                    color = float 0xd4d4d4,
                    roughness = 0.55,
                    metalness = 0.1,
                    flatShading = (i >= 4)
                )
            )
        let m =
            Mesh.Create(
                geos.[i],
                mat
            )
        m.name <- names.[i]
        m.position.set (float gx * 2.1, 0.0, float gz * 2.1) |> ignore
        m.rotation.set (0.2, 0.35 * float i, 0.0) |> ignore
        scene.add m |> ignore
        mats.Add mat
        meshes.Add (m :> Object3D<Object3DEventMap>)

    let box = Box3.Create(Vector3.Create(-0.5, -0.5, -0.5), Vector3.Create(0.5, 0.5, 0.5))
    let boxHelper = Box3Helper.Create(box, float 0x111111)
    boxHelper.visible <- false
    scene.add boxHelper |> ignore

    let raycaster = Raycaster.Create ()
    let metaEl = (document.getElementById "meta").Value

    let updateCamera () =
        let radius = 8.4
        camera.position.set (radius * sin yaw, 5.1, radius * cos yaw) |> ignore
        camera.lookAt (Vector3.Create(0.0, 0.0, 0.0))
        camera.updateMatrixWorld true
        scene.updateMatrixWorld true

    let renderScene () =
        for i in 0 .. 8 do
            if i = selectedIndex then
                mats.[i].color.setHex (float 0x111111) |> ignore
            else
                mats.[i].color.setHex (float 0xd4d4d4) |> ignore

        if selectedIndex >= 0 && selectedIndex < meshes.Count then
            box.setFromObject (meshes.[selectedIndex], true) |> ignore
            boxHelper.visible <- true
        else
            boxHelper.visible <- false

        renderer.render (scene, camera)
        metaEl.textContent <- sprintf "picked: %s · 9 specimens · click canvas or buttons" lastHitInfo

    let castAtNdc (nx: float) (ny: float) =
        raycaster.setFromCamera (Vector2.Create(nx, ny), camera)
        let hits = raycaster.intersectObjects (meshes, false)
        if hits.Count > 0 then
            let hitObj = hits.[0].object
            let mutable found = -1
            for i in 0 .. meshes.Count - 1 do
                if obj.ReferenceEquals (meshes.[i], hitObj) then
                    found <- i
            selectedIndex <- found
            let sz = box.getSize (Vector3.Create ())
            if found >= 0 then
                box.setFromObject (meshes.[found], true) |> ignore
                box.getSize sz |> ignore
                lastHitInfo <-
                    sprintf "%s at d=%.2f (%.2f, %.2f, %.2f) box=%.2fx%.2fx%.2f"
                        names.[found]
                        hits.[0].distance
                        hits.[0].point.x
                        hits.[0].point.y
                        hits.[0].point.z
                        sz.x
                        sz.y
                        sz.z
            else
                lastHitInfo <- "unknown"
        else
            selectedIndex <- -1
            lastHitInfo <- sprintf "miss at ndc (%.2f, %.2f)" nx ny
        renderScene ()

    let pickSpecimen (idx: int) =
        updateCamera ()
        let wp = meshes.[idx].getWorldPosition (Vector3.Create ())
        let proj = wp.project (camera :> Camera)
        castAtNdc proj.x proj.y

    let wireClick (id: string) (fn: unit -> unit) =
        let b = (document.getElementById id).Value
        b.addEventListener (HTMLElementEventMapKeys.click, fun _ -> fn ())

    canvas.addEventListener (
        HTMLElementEventMapKeys.click,
        fun ev ->
            let me: Dom.MouseEvent = ev :> Dom.MouseEvent
            let rect = canvas.getBoundingClientRect ()
            let nx = ((me.clientX - rect.left) / rect.width) * 2.0 - 1.0
            let ny = -((me.clientY - rect.top) / rect.height) * 2.0 + 1.0
            updateCamera ()
            castAtNdc nx ny
    )

    wireClick "btn-p0" (fun () -> pickSpecimen 4)
    wireClick "btn-p1" (fun () -> pickSpecimen 0)
    wireClick "btn-p2" (fun () -> pickSpecimen 8)
    wireClick "btn-orbit" (fun () ->
        yaw <- yaw + Math.PI / 4.0
        updateCamera ()
        renderScene ()
    )
    wireClick "btn-clear" (fun () ->
        selectedIndex <- -1
        lastHitInfo <- "none"
        updateCamera ()
        renderScene ()
    )

    pickSpecimen 4

main ()
