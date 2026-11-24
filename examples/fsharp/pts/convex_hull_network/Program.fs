module Demos.Pts.ConvexHullNetwork

open System
open Pts

let private mkPt (x: float) (y: float) =
    let p = Pt.make 2.0
    p.x <- x
    p.y <- y
    p

let space =
    CanvasSpace
        .Create("#pt")
        .setup(CanvasSpaceOptions.Create(bgcolor = "#ffffff", resize = true, retina = true))

let form = space.getForm ()

let player =
    IPlayer.Create(
        animate = fun time _ _ ->
            let c = space.center
            let size = space.size
            let pointer = space.pointer
            let baseR = Math.Max(70.0, Math.Min(size.x, size.y) * 0.34)
            let radial = Create.radialPts (c, baseR, 20.0, time * 0.00025)

            let cloudPts =
                ResizeArray [
                    for i, p in Seq.indexed radial do
                        let fi = float i
                        let scale = 0.48 + 0.52 * (0.5 + 0.5 * Math.Sin(time * 0.0011 + fi * 0.9) * Math.Cos(time * 0.0007 - fi * 0.5))
                        mkPt (c.x + (p.x - c.x) * scale) (c.y + (p.y - c.y) * scale)
                    if pointer.x > 1.0 || pointer.y > 1.0 then
                        mkPt pointer.x pointer.y
                ]

            let cloud = Group.fromPtArray cloudPts
            let hull = Polygon.convexHull (cloud, false)
            let bbox = Geom.boundingBox hull
            let cent = Polygon.centroid hull

            let hullArr = Array.ofSeq hull
            let midIdx = float (hullArr.Length / 2)

            form.strokeOnly("#b5b5b5", 1.0).rect(bbox) |> ignore
            form.fill("#f7f7f7").stroke("#111111", 2.0).polygon(hull) |> ignore

            for seg in Polygon.network (hull, 0.0) do
                form.strokeOnly("#d4d4d4", 1.0).line(seg) |> ignore
            if hullArr.Length > 3 then
                for seg in Polygon.network (hull, midIdx) do
                    form.strokeOnly("#d4d4d4", 1.0).line(seg) |> ignore

            for p in cloudPts do
                form.fillOnly("#737373").point(p, 3.0, "circle") |> ignore
            for h in hullArr do
                form.fill("#111111").stroke("#ffffff", 1.5).point(h, 5.0, "circle") |> ignore

            let hBar = Group.fromPtArray (ResizeArray [ mkPt (cent.x - 10.0) cent.y; mkPt (cent.x + 10.0) cent.y ])
            let vBar = Group.fromPtArray (ResizeArray [ mkPt cent.x (cent.y - 10.0); mkPt cent.x (cent.y + 10.0) ])
            form.strokeOnly("#111111", 1.4).line(hBar) |> ignore
            form.strokeOnly("#111111", 1.4).line(vBar) |> ignore
            form.fill("#ffffff").stroke("#111111", 1.6).point(cent, 5.5, "circle") |> ignore
    )

space.add player |> ignore
space.bindMouse().bindTouch().play () |> ignore
