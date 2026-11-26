// three.js from F#, in Node: vector, matrix and rotation math, a scene graph,
// a mesh and a ray cast against it. Every check must hold.
module Test

open Fable.Core
open Fable.Core.JsInterop
open Bindings

let mutable passed = 0

let check (ok: bool) (what: string) =
    if ok then passed <- passed + 1 else failwithf "check failed: %s" what

let near (a: float) (b: float) = abs (a - b) < 1e-6

// Vectors.
let v = Vector3.Create (1.0, 2.0, 3.0)
check (v.x = 1.0 && v.y = 2.0 && v.z = 3.0) "Vector3 fields"
check (near (v.length ()) (sqrt 14.0)) "length"
let w = Vector3.Create (4.0, 5.0, 6.0)
check (v.dot w = 32.0) "dot"
let c = Vector3.Create(1.0, 0.0, 0.0).cross (Vector3.Create (0.0, 1.0, 0.0))
check (c.x = 0.0 && c.y = 0.0 && c.z = 1.0) "cross"
let sum = v.clone().add w
check (sum.x = 5.0 && sum.y = 7.0 && sum.z = 9.0) "add"
check (v.x = 1.0) "clone leaves the original"
check (v.distanceTo w = sqrt 27.0) "distanceTo"
let unit' = Vector3.Create(3.0, 0.0, 4.0).normalize ()
check (near unit'.x 0.6 && near unit'.z 0.8) "normalize"
check (v.clone().multiplyScalar(2.0).z = 6.0) "multiplyScalar"
check (Vector3.Create(0.0, 0.0, 0.0).lerp (Vector3.Create (10.0, 0.0, 0.0), 0.25) |> fun l -> l.x = 2.5) "lerp"
check (v.equals (Vector3.Create (1.0, 2.0, 3.0))) "equals"
check ((Seq.toList v) = [ 1.0; 2.0; 3.0 ]) "a vector is a sequence"
check (v.toArray () |> Seq.toList = [ 1.0; 2.0; 3.0 ]) "toArray"

// Quaternions and Euler angles: a quarter turn about Z takes x to y.
let q = Quaternion.Create ()
q.setFromAxisAngle (Vector3.Create (0.0, 0.0, 1.0), System.Math.PI / 2.0) |> ignore
check (near q.z (sqrt 0.5) && near q.w (sqrt 0.5)) "setFromAxisAngle"
let turned = Vector3.Create(1.0, 0.0, 0.0).applyQuaternion q
check (near turned.x 0.0 && near turned.y 1.0) "applyQuaternion"
check (near (q.length ()) 1.0) "a unit quaternion"
let e = Euler.Create (0.0, 0.0, System.Math.PI / 2.0)
let fromEuler = Quaternion.Create().setFromEuler e
check (near fromEuler.z q.z && near fromEuler.w q.w) "setFromEuler"
let back = Euler.Create().setFromQuaternion q
check (near back.z (System.Math.PI / 2.0)) "setFromQuaternion"
check (e.order = EulerOrder.XYZ) "the default order"
let inverse = q.clone().invert ()
check (near (q.clone().multiply(inverse).w) 1.0) "invert"

// Matrices.
let m = Matrix4.Create ()
check (m.determinant () = 1.0) "identity determinant"
m.makeTranslation (1.0, 2.0, 3.0) |> ignore
let moved = Vector3.Create(1.0, 1.0, 1.0).applyMatrix4 m
check (moved.x = 2.0 && moved.y = 3.0 && moved.z = 4.0) "makeTranslation"
let s = Matrix4.Create().makeScale (2.0, 3.0, 4.0)
check (s.determinant () = 24.0) "makeScale determinant"
let both = Matrix4.Create().multiplyMatrices (m, s)
let pos = Vector3.Create ()
let rot = Quaternion.Create ()
let scl = Vector3.Create ()
both.decompose (pos, rot, scl) |> ignore
check (pos.x = 1.0 && pos.y = 2.0 && pos.z = 3.0) "decompose: position"
check (scl.x = 2.0 && scl.y = 3.0 && scl.z = 4.0) "decompose: scale"
check (near rot.w 1.0) "decompose: no rotation"
let composed = Matrix4.Create().compose (pos, rot, scl)
check (composed.equals both) "compose"
let inv = both.clone().invert ()
let ident = Matrix4.Create().multiplyMatrices (both, inv)
check (near (ident.determinant ()) 1.0) "invert"
check (let (a, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _) = both.elements in a = 2.0) "elements"

// Colours.
let red = Color.Create (ColorRepresentation.String "red")
check (red.r = 1.0 && red.g = 0.0 && red.b = 0.0) "a named colour"
check (red.getHexString () = "ff0000") "getHexString"
let teal = Color.Create (ColorRepresentation.Number 0x008080)
check (teal.getHex () = float 0x008080) "a hex colour"
check (near (Color.Create(0.0, 0.0, 0.0).lerp (Color.Create (1.0, 1.0, 1.0), 0.5)).g 0.5) "lerp"

// Boxes and spheres.
let bx = Box3.Create (Vector3.Create (-1.0, -1.0, -1.0), Vector3.Create (1.0, 1.0, 1.0))
check (bx.containsPoint (Vector3.Create (0.5, 0.5, 0.5))) "Box3 contains"
check (not (bx.containsPoint (Vector3.Create (2.0, 0.0, 0.0)))) "Box3 does not contain"
let size = bx.getSize (Vector3.Create ())
check (size.x = 2.0 && size.y = 2.0 && size.z = 2.0) "getSize"
bx.expandByPoint (Vector3.Create (3.0, 0.0, 0.0)) |> ignore
check (bx.max.x = 3.0) "expandByPoint"
check (not (bx.isEmpty ())) "not empty"
let ball = Sphere.Create (Vector3.Create (0.0, 0.0, 0.0), 2.0)
check (ball.containsPoint (Vector3.Create (1.0, 1.0, 1.0))) "Sphere contains"
check (ball.distanceToPoint (Vector3.Create (5.0, 0.0, 0.0)) = 3.0) "Sphere distanceToPoint"
check (ball.intersectsBox (Box3.Create (Vector3.Create (1.0, 1.0, 1.0), Vector3.Create (4.0, 4.0, 4.0)))) "Sphere intersectsBox"

// A scene graph: positions are relative to the parent.
let parent = Group.Create ()
parent.position.set (10.0, 0.0, 0.0) |> ignore
let child = Object3D.Create ()
child.position.set (0.0, 5.0, 0.0) |> ignore
child.name <- "child"
parent.add child |> ignore
check (parent.children.Count = 1) "add"
check (obj.ReferenceEquals (child.parent, parent)) "parent"
parent.updateMatrixWorld true
let world = child.getWorldPosition (Vector3.Create ())
check (world.x = 10.0 && world.y = 5.0 && world.z = 0.0) "getWorldPosition"
let local = child.worldToLocal (Vector3.Create (10.0, 5.0, 0.0))
check (near local.x 0.0 && near local.y 0.0) "worldToLocal"
parent.rotation.z <- System.Math.PI / 2.0
parent.updateMatrixWorld true
let rotated = child.getWorldPosition (Vector3.Create ())
check (near rotated.x 5.0 && near rotated.y 0.0) "rotating the parent moves the child"
check (parent.isGroup && child.isObject3D) "type flags"
check (parent.getObjectByName("child").IsSome) "getObjectByName"
check (parent.getObjectByName("nobody").IsNone) "getObjectByName misses"
let mutable visited = 0
parent.traverse (fun _ -> visited <- visited + 1)
check (visited = 2) "traverse"
parent.remove child |> ignore
check (parent.children.Count = 0 && isNull (box child.parent)) "remove"

// A mesh: a geometry and a material.
let geometry = BoxGeometry.Create (2.0, 2.0, 2.0)
check (geometry.parameters.width = 2.0) "BoxGeometry parameters"
let material = MeshBasicMaterial.Create ()
let mesh = Mesh.Create (geometry, LineMaterial.Material material)
check (mesh.isMesh && mesh.``type`` = "Mesh") "Mesh"
check (obj.ReferenceEquals (mesh.geometry, geometry)) "mesh.geometry"
check (material.isMeshBasicMaterial) "material"

// Geometry attributes: 24 vertices of a box (4 per face).
let position =
    match geometry.getAttribute "position" with
    | BufferGeometryGetAttributeResult.BufferAttribute a -> a
    | _ -> failwith "a position attribute is a BufferAttribute"
check (position.count = 24.0 && position.itemSize = 3.0) "position attribute"
check (geometry.hasAttribute "normal" && geometry.hasAttribute "uv") "normal and uv attributes"
check (abs (position.getX 0.0) = 1.0) "a vertex of the box"
geometry.computeBoundingBox ()
check (geometry.boundingBox.Value.max.x = 1.0 && geometry.boundingBox.Value.min.z = -1.0) "computeBoundingBox"
geometry.computeBoundingSphere ()
check (near geometry.boundingSphere.Value.radius (sqrt 3.0)) "computeBoundingSphere"
check (geometry.index.Value.count = 36.0) "index: 12 triangles"

// A ray cast: from in front of the box toward it.
mesh.position.set (0.0, 0.0, -5.0) |> ignore
mesh.updateMatrixWorld true
let caster = Raycaster.Create (Vector3.Create (0.0, 0.0, 5.0), Vector3.Create (0.0, 0.0, -1.0))
let hits = caster.intersectObject (mesh, false)
check (hits.Count >= 1) "the ray hits the mesh"
check (near hits.[0].distance 9.0) "distance to the near face"
check (near hits.[0].point.z -4.0) "the point of the hit"
check (obj.ReferenceEquals (hits.[0].object, mesh)) "the object hit"
let missing = Raycaster.Create (Vector3.Create (50.0, 0.0, 5.0), Vector3.Create (0.0, 0.0, -1.0))
check (missing.intersectObject(mesh, false).Count = 0) "a ray that misses"

printfn "%d checks passed" passed
