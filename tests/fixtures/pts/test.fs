// Calls Pts, a real library, through its generated F# bindings; every check must hold.
module Test

open Fable.Core
open Bindings

let mutable passed = 0

let check (ok: bool) (what: string) =
    if ok then passed <- passed + 1 else failwithf "check failed: %s" what


// A Pt is an array of coordinates, with names for the first four.
let p = Pt.Create(3.0, 4.0)
check (p.x = 3.0 && p.y = 4.0) "coordinates"
check (p.magnitude () = 5.0) "magnitude"
check (p.toString () = "Pt(3, 4)") "toString"
p.id <- "corner"
check (p.id = "corner") "property"

// A method that changes a Pt returns it; its `$` twin returns another.
let moved = p.``$add``(1.0, 1.0)
check (not (obj.ReferenceEquals(moved, p)) && moved.x = 4.0 && p.x = 3.0) "$add returns another"
check (obj.ReferenceEquals(p.add(1.0, 1.0), p) && p.x = 4.0) "add returns this"
check (p.equals (PtLike.Pt moved)) "union argument"
check (p.clone().equals(PtLike.Pt p, 0.001)) "optional argument"

// Statics.
check (Num.lerp (0.0, 10.0, 0.5) = 5.0) "lerp"
check (Num.clamp (12.0, 0.0, 10.0) = 10.0) "clamp"
check (Geom.toDegree System.Math.PI = 180.0) "toDegree"
check (Pt.make(3.0, 2.0).toString () = "Pt(2, 2, 2)") "optional arguments"

// A Group is an array of Pt: indexed, and a sequence.
let square = Group.Create(Pt.Create(0.0, 0.0), Pt.Create(4.0, 0.0), Pt.Create(4.0, 4.0), Pt.Create(0.0, 4.0))
check (square.length = 4 && square[2].y = 4.0) "indexed"
check ((square |> Seq.sumBy (fun corner -> corner.x)) = 8.0) "a sequence"
check (square.p1.x = 0.0) "getter"
check (square.centroid().toString () = "Pt(2, 2)") "centroid"
check (square.boundingBox().p2.x = 4.0) "bounding box"

// A union parameter takes any of its members: a Group where points are wanted.
check (Geom.centroid(PtLikeIterable.Group square).x = 2.0) "a Group as points"
check (Line.magnitude (PtIterable.Group(Group.Create(Pt.Create(0.0, 0.0), Pt.Create(3.0, 4.0)))) = 5.0) "line magnitude"
let ray = Line.fromAngle (PtLike.Pt(Pt.Create(1.0, 1.0)), 0.0, 5.0)
check (ray.p2.x = 6.0 && ray.p2.y = 1.0) "from angle"


printfn "%d checks passed" passed
