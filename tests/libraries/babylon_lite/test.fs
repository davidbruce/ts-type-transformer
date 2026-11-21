// @babylonjs/lite from F#, in Node: vectors, cameras, lights, materials,
// procedural mesh buffers, accessibility tree, and math tuples. Every check must hold.
module Test

open System
open Bindings
open type Bindings.Globals

let mutable passed = 0

let check (ok: bool) (what: string) =
    if ok then passed <- passed + 1 else failwithf "check failed: %s" what

let near (a: float) (b: float) = abs (a - b) < 1e-6

// Vectors.
let target = vec3 (1.0, 2.0, 3.0)
check (target.x = 1.0 && target.y = 2.0 && target.z = 3.0) "vec3 fields"
target.y <- 1.6
check (near target.y 1.6) "vec3 mutation"

// Camera.
let camera = createArcRotateCamera (Math.PI / 4.0, Math.PI / 3.0, 8.5, target)
check (near camera.alpha (Math.PI / 4.0) && near camera.beta (Math.PI / 3.0)) "camera angles"
check (camera.radius = 8.5 && near camera.target.y 1.6) "camera radius and target"

// Lights.
let hemi = createHemisphericLight ((0.0, 1.0, 0.2), 0.75)
hemi.diffuseColor <- (0.75, 0.85, 1.0)
hemi.groundColor <- (0.12, 0.14, 0.22)
let (dr, dg, db) = hemi.diffuseColor
let (gr, _, _) = hemi.groundColor
check (near hemi.intensity 0.75 && near dr 0.75 && near dg 0.85 && near db 1.0 && near gr 0.12) "hemispheric light"

let dir = createDirectionalLight ((-1.0, -2.0, -1.2), 2.2)
dir.diffuse <- (1.0, 0.96, 0.9)
let (d0, d1, d2) = dir.diffuse
check (near dir.intensity 2.2 && near d0 1.0 && near d1 0.96 && near d2 0.9) "directional light"

let pt = createPointLight ((2.8, 1.6, 0.0), 8.0)
pt.range <- 12.0
check (near pt.intensity 8.0 && near pt.range 12.0) "point light"

// Materials.
let pbr =
    createPbrMaterial (
        CreatePbrMaterialProps.Create(
            baseColorFactor = (0.18, 0.78, 0.95, 1.0),
            metallicFactor = 0.75,
            roughnessFactor = 0.22
        )
    )
check (pbr.metallicFactor = Some 0.75 && pbr.roughnessFactor = Some 0.22) "PBR material"

let grid = createGridMaterial (GridMaterialOptions.Create(gridRatio = 0.5, majorUnitFrequency = 4.0))
check (grid.vertexSource.Length > 0 && grid.fragmentSource.Length > 0) "grid shader material"

// Procedural geometry data (typed arrays on CPU).
let sphere = createSphereData (SphereOptions.Create(diameter = 2.0, segments = 8.0))
check (sphere.vertexCount > 0.0 && sphere.indexCount > 0.0) "sphere counts"
check (sphere.positions.length > 0 && sphere.indices.length > 0) "sphere typed array lengths"

let knot = createTorusKnotData (TorusKnotOptions.Create(radius = 1.0, tube = 0.3))
check (knot.positions.length > 0 && knot.indices.length > 0) "torus knot typed array lengths"

// Accessibility tree.
let tree = createAccessibilityTree ()
let node = addAccessibilityNode (tree, AccessibilityNodeOptions.Create(tag = AccessibilityTag.Create(name = "hero", role = "button")))
check (tree.roots.Count = 1 && not tree.disposed) "accessibility tree roots"
check (node.tag.Value.name = Some "hero" && node.tag.Value.role = Some "button") "accessibility node tag"

// Quaternion -> Euler tuple conversion.
let (ex, ey, ez) = quatToEulerXYZTuple (0.0, 0.0, 0.0, 1.0)
check (near ex 0.0 && near ey 0.0 && near ez 0.0) "identity quaternion euler"

printfn "%d checks passed" passed
