module Demos.Pts.CircleIntersections

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
            let minDim = Math.Max(120.0, Math.Min(size.x, size.y))

            let ang = time * 0.00035
            let halfLen = minDim * 0.44
            let ca = Math.Cos ang
            let sa = Math.Sin ang
            let chord =
                Group.fromPtArray (
                    ResizeArray
                        [ mkPt (c.x - ca * halfLen) (c.y - sa * halfLen)
                          mkPt (c.x + ca * halfLen) (c.y + sa * halfLen) ]
                )
            form.strokeOnly("#999999", 1.2).line(chord) |> ignore

            let probeCenter =
                if pointer.x > 1.0 || pointer.y > 1.0 then pointer else c
            let probe = Circle.fromCenter (probeCenter, minDim * 0.20)

            let targetCircles =
                [ for i in 0..4 do
                      let fi = float i
                      let orbitR = minDim * (0.14 + 0.04 * fi)
                      let theta = time * (0.00045 + 0.00012 * fi) + fi * 1.256
                      let tc = mkPt (c.x + Math.Cos theta * orbitR) (c.y + Math.Sin theta * orbitR)
                      let tr = minDim * (0.11 + 0.025 * float (i % 3))
                      let circle = Circle.fromCenter (tc, tr)

                      form.fill("#f7f7f7").stroke("#737373", 1.4).circle(circle) |> ignore
                      match Line.perpendicularFromPt (chord, tc, false) with
                      | Some foot ->
                          let stem = Group.fromPtArray (ResizeArray [ tc; foot ])
                          form.strokeOnly("#b5b5b5", 1.0).line(stem) |> ignore
                          form.fillOnly("#555555").point(foot, 2.5, "circle") |> ignore
                      | None -> ()
                      yield circle ]

            form.strokeOnly("#111111", 2.0).circle(probe) |> ignore
            let px1 = Group.fromPtArray (ResizeArray [ mkPt (probeCenter.x - 6.0) probeCenter.y; mkPt (probeCenter.x + 6.0) probeCenter.y ])
            let py1 = Group.fromPtArray (ResizeArray [ mkPt probeCenter.x (probeCenter.y - 6.0); mkPt probeCenter.x (probeCenter.y + 6.0) ])
            form.strokeOnly("#111111", 1.4).line(px1) |> ignore
            form.strokeOnly("#111111", 1.4).line(py1) |> ignore

            for circle in targetCircles do
                let hitList = ResizeArray (Circle.intersectCircle2D (probe, circle))
                if hitList.Count = 2 then
                    let secant = Group.fromPtArray hitList
                    form.strokeOnly("#111111", 1.4).line(secant) |> ignore
                for h in hitList do
                    form.fill("#ffffff").stroke("#111111", 2.0).point(h, 5.0, "circle") |> ignore
    )

space.add player |> ignore
space.bindMouse().bindTouch().play () |> ignore
