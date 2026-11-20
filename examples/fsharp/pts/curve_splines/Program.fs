module Program

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
            let size = space.size
            let c = space.center
            let pointer = space.pointer
            let w = Math.Max(120.0, size.x)
            let h = Math.Max(120.0, size.y)
            let amp = h * 0.24
            let count = 11

            let baseLine = Group.fromPtArray (ResizeArray [ mkPt (w * 0.06) c.y; mkPt (w * 0.94) c.y ])
            form.strokeOnly("#e5e5e5", 1.0).line(baseLine) |> ignore

            let pts =
                ResizeArray [
                    for i in 0 .. count - 1 do
                        let u = float i / float (count - 1)
                        let x = w * (0.08 + 0.84 * u)
                        let fi = float i
                        let baseY =
                            c.y
                            + Math.Sin(time * 0.0014 + fi * 0.72) * amp
                            + Math.Cos(time * 0.0009 - fi * 0.45) * (amp * 0.42)
                        let dx = Math.Abs(x - pointer.x)
                        let y =
                            if dx < w * 0.25 && (pointer.x > 1.0 || pointer.y > 1.0) then
                                let pull = 1.0 - dx / (w * 0.25)
                                baseY + (pointer.y - baseY) * pull * 0.35
                            else
                                baseY
                        mkPt x y
                ]

            let ctrls = Group.fromPtArray pts
            let bs = Curve.bspline (ctrls, 12.0)
            let card = Curve.cardinal (ctrls, 12.0, 0.35)
            let cr = Curve.catmullRom (ctrls, 12.0)

            cr
            |> Seq.iteri (fun i p ->
                if i % 3 = 0 then
                    let stem = Group.fromPtArray (ResizeArray [ mkPt p.x c.y; p ])
                    form.strokeOnly("#d4d4d4", 1.0).line(stem) |> ignore)

            form.strokeOnly("#d4d4d4", 1.0).line(ctrls) |> ignore
            form.strokeOnly("#b5b5b5", 2.0).line(bs) |> ignore
            form.strokeOnly("#555555", 1.8).line(card) |> ignore
            form.strokeOnly("#111111", 2.6).line(cr) |> ignore

            for p in pts do
                form.fill("#ffffff").stroke("#111111", 1.5).point(p, 4.5, "circle") |> ignore
    )

space.add player |> ignore
space.bindMouse().bindTouch().play () |> ignore
