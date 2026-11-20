module Program

open System
open Pts

let private mkPt (x: float) (y: float) =
    let p = Pt.make 2.0
    p.x <- x
    p.y <- y
    p

let private shades =
    [| "#f7f7f7"
       "#efefef"
       "#e5e5e5"
       "#d4d4d4"
       "#b5b5b5"
       "#999999"
       "#737373"
       "#404040"
       "#262626" |]

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
            let radius = Math.Max(80.0, Math.Min(size.x, size.y) * 0.40)

            let pxNorm = (pointer.x / Math.Max(1.0, size.x)) - 0.5
            let t = Num.clamp (0.22 + 0.16 * Math.Sin(time * 0.0012) + pxNorm * 0.24, 0.10, 0.48)

            let mutable poly = Polygon.fromCenter (c, radius, 6.0)
            poly.rotate2D (time * 0.00035, c) |> ignore
            let centroid = Polygon.centroid poly

            for depth in 0..8 do
                let sw = if depth = 0 then 1.6 else 1.0
                form.fill(shades.[depth]).stroke("#111111", sw).polygon(poly) |> ignore
                if depth = 0 then
                    for pt in poly do
                        let ray = Group.fromPtArray (ResizeArray [ centroid; pt ])
                        form.strokeOnly("#d4d4d4", 1.0).line(ray) |> ignore
                        form.fillOnly("#111111").point(pt, 3.5, "circle") |> ignore
                poly <- Polygon.midpoints (poly, true, t)

            let hBar = Group.fromPtArray (ResizeArray [ mkPt (c.x - 10.0) c.y; mkPt (c.x + 10.0) c.y ])
            let vBar = Group.fromPtArray (ResizeArray [ mkPt c.x (c.y - 10.0); mkPt c.x (c.y + 10.0) ])
            form.strokeOnly("#ffffff", 1.5).line(hBar) |> ignore
            form.strokeOnly("#ffffff", 1.5).line(vBar) |> ignore
            form.fill("#ffffff").stroke("#111111", 1.5).point(centroid, 5.0, "circle") |> ignore

            let ring = Circle.fromCenter (pointer, 10.0)
            form.strokeOnly("#737373", 1.5).circle(ring) |> ignore
    )

space.add player |> ignore
space.bindMouse().bindTouch().play () |> ignore
