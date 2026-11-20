module Program

open System
open Fable.Core
open Dom
open type Dom.Globals
open Three

let mutable private epoch = 0.6
let mutable private outerTilted = false

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
    stage.appendChild (renderer.domElement :> Dom.Node) |> ignore

    let scene = Scene.Create<Object3DEventMap>()
    scene.background <- ColorOrTexture.Color (Color.Create (ColorRepresentation.Number (float 0xf7f7f7)))

    let camera = PerspectiveCamera.Create(42.0, width / height, 0.1, 100.0)
    camera.position.set (5.6, 5.2, 7.6) |> ignore
    camera.lookAt (Vector3.Create(0.0, -0.1, 0.0))

    scene.add (AmbientLight.Create(ColorRepresentation.Number (float 0xffffff), 1.2) :> Object3D<Object3DEventMap>) |> ignore
    let keyLight = DirectionalLight.Create(ColorRepresentation.Number (float 0xffffff), 2.2)
    keyLight.position.set (6.0, 10.0, 8.0) |> ignore
    scene.add (keyLight :> Object3D<Object3DEventMap>) |> ignore
    let rimLight = DirectionalLight.Create(ColorRepresentation.Number (float 0xffffff), 0.9)
    rimLight.position.set (-7.0, 5.0, -6.0) |> ignore
    scene.add (rimLight :> Object3D<Object3DEventMap>) |> ignore

    let grid =
        GridHelper.Create(
            12.0,
            12.0,
            ColorRepresentation.Number (float 0x999999),
            ColorRepresentation.Number (float 0xe5e5e5)
        )
    grid.position.y <- -1.5
    scene.add (grid :> Object3D<Object3DEventMap>) |> ignore

    let pedestal =
        Mesh.Create<CylinderGeometry, MeshStandardMaterial, Object3DEventMap>(
            CylinderGeometry.Create(0.55, 0.75, 0.35, 24.0),
            MeshStandardMaterial.Create(
                MeshStandardMaterialParameters.Create(
                    color = ColorRepresentation.Number (float 0x262626),
                    roughness = 0.7
                )
            )
        )
    pedestal.position.y <- -1.3
    scene.add (pedestal :> Object3D<Object3DEventMap>) |> ignore

    let core =
        Mesh.Create<SphereGeometry, MeshStandardMaterial, Object3DEventMap>(
            SphereGeometry.Create(0.65, 24.0, 18.0),
            MeshStandardMaterial.Create(
                MeshStandardMaterialParameters.Create(
                    color = ColorRepresentation.Number (float 0x111111),
                    roughness = 0.45,
                    metalness = 0.2
                )
            )
        )
    scene.add (core :> Object3D<Object3DEventMap>) |> ignore

    let innerPivot = Group.Create<Object3DEventMap>()
    scene.add (innerPivot :> Object3D<Object3DEventMap>) |> ignore

    let innerTrack =
        Mesh.Create<TorusGeometry, MeshBasicMaterial, Object3DEventMap>(
            TorusGeometry.Create(2.1, 0.025, 8.0, 64.0),
            MeshBasicMaterial.Create(
                MeshBasicMaterialParameters.Create(color = ColorRepresentation.Number (float 0x999999))
            )
        )
    innerTrack.rotation.x <- Math.PI / 2.0
    innerPivot.add (innerTrack :> Object3D<Object3DEventMap>) |> ignore

    let innerPlanet = Group.Create<Object3DEventMap>()
    innerPlanet.position.set (2.1, 0.0, 0.0) |> ignore
    innerPivot.add (innerPlanet :> Object3D<Object3DEventMap>) |> ignore

    let planetMesh =
        Mesh.Create<SphereGeometry, MeshStandardMaterial, Object3DEventMap>(
            SphereGeometry.Create(0.36, 20.0, 16.0),
            MeshStandardMaterial.Create(
                MeshStandardMaterialParameters.Create(
                    color = ColorRepresentation.Number (float 0x555555),
                    roughness = 0.55
                )
            )
        )
    innerPlanet.add (planetMesh :> Object3D<Object3DEventMap>) |> ignore

    let moonPivot = Group.Create<Object3DEventMap>()
    innerPlanet.add (moonPivot :> Object3D<Object3DEventMap>) |> ignore

    let moon =
        Mesh.Create<OctahedronGeometry, MeshStandardMaterial, Object3DEventMap>(
            OctahedronGeometry.Create(0.18, 0.0),
            MeshStandardMaterial.Create(
                MeshStandardMaterialParameters.Create(
                    color = ColorRepresentation.Number (float 0xd4d4d4),
                    roughness = 0.4,
                    flatShading = true
                )
            )
        )
    moon.position.set (0.75, 0.0, 0.0) |> ignore
    moonPivot.add (moon :> Object3D<Object3DEventMap>) |> ignore

    let outerPivot = Group.Create<Object3DEventMap>()
    scene.add (outerPivot :> Object3D<Object3DEventMap>) |> ignore

    let outerTrack =
        Mesh.Create<TorusGeometry, MeshBasicMaterial, Object3DEventMap>(
            TorusGeometry.Create(3.8, 0.025, 8.0, 80.0),
            MeshBasicMaterial.Create(
                MeshBasicMaterialParameters.Create(color = ColorRepresentation.Number (float 0xb5b5b5))
            )
        )
    outerTrack.rotation.x <- Math.PI / 2.0
    outerPivot.add (outerTrack :> Object3D<Object3DEventMap>) |> ignore

    let outerStation = Group.Create<Object3DEventMap>()
    outerStation.position.set (3.8, 0.0, 0.0) |> ignore
    outerPivot.add (outerStation :> Object3D<Object3DEventMap>) |> ignore

    let stationBody =
        Mesh.Create<IcosahedronGeometry, MeshStandardMaterial, Object3DEventMap>(
            IcosahedronGeometry.Create(0.42, 0.0),
            MeshStandardMaterial.Create(
                MeshStandardMaterialParameters.Create(
                    color = ColorRepresentation.Number (float 0x404040),
                    roughness = 0.5,
                    flatShading = true
                )
            )
        )
    outerStation.add (stationBody :> Object3D<Object3DEventMap>) |> ignore

    let halo =
        Mesh.Create<TorusGeometry, MeshStandardMaterial, Object3DEventMap>(
            TorusGeometry.Create(0.68, 0.06, 10.0, 32.0),
            MeshStandardMaterial.Create(
                MeshStandardMaterialParameters.Create(
                    color = ColorRepresentation.Number (float 0x111111),
                    roughness = 0.4
                )
            )
        )
    halo.rotation.x <- 0.8
    outerStation.add (halo :> Object3D<Object3DEventMap>) |> ignore

    let tiltBtn = document.getElementById "btn-tilt"
    let metaEl = document.getElementById "meta"

    let updateOrrery () =
        innerPivot.rotation.y <- epoch
        moonPivot.rotation.y <- epoch * 2.5
        outerPivot.rotation.y <- -epoch * 0.55
        outerPivot.rotation.z <- if outerTilted then 0.38 else 0.0
        stationBody.rotation.y <- epoch * 1.4
        scene.updateMatrixWorld true
        let moonPos = moon.getWorldPosition (Vector3.Create ())
        let stationPos = outerStation.getWorldPosition (Vector3.Create ())
        tiltBtn.textContent <- if outerTilted then "Outer Tilt: On" else "Outer Tilt: Off"
        renderer.render (scene :> Object3D<Object3DEventMap>, camera :> Camera)
        let deg = int (Math.Round(epoch * 180.0 / Math.PI))
        metaEl.textContent <-
            sprintf
                "epoch: %d deg · moon world: (%.2f, %.2f, %.2f) · station world: (%.2f, %.2f, %.2f)"
                deg moonPos.x moonPos.y moonPos.z stationPos.x stationPos.y stationPos.z

    let wireClick (id: string) (fn: unit -> unit) =
        let b = document.getElementById id
        b.addEventListener (HTMLElementEventMapKeys.click, fun _ -> fn ())

    wireClick "btn-step" (fun () ->
        epoch <- epoch + Math.PI / 6.0
        updateOrrery ()
    )
    wireClick "btn-tilt" (fun () ->
        outerTilted <- not outerTilted
        updateOrrery ()
    )
    wireClick "btn-reset" (fun () ->
        epoch <- 0.6
        outerTilted <- false
        updateOrrery ()
    )

    updateOrrery ()

main ()
