module Program

open System
open Fable.Core
open Fable.Core.JsInterop
open Pts

let space =
    CanvasSpace
        .Create("#pt")
        .setup(CanvasSpaceOptions.Create(bgcolor = "#101622", resize = true, retina = true))

let form = space.getForm ()

let player = createEmpty<IPlayer>

player.animate <-
    Some(fun time _ _ ->
        let pointer = space.pointer
        let grid = Create.gridPts (space.innerBound, 18.0, 11.0)
        let maxDist = Math.Max(space.size.x, space.size.y) * 0.55

        for pt in grid do
            let delta = pt.``$subtract`` (pointer)
            let dist = delta.magnitude ()
            let t = 1.0 - Num.clamp (dist / Math.Max(1.0, maxDist), 0.0, 1.0)
            let angle = delta.angle () + Math.Sin(time * 0.0015 + dist * 0.01) * 0.35
            let len = 8.0 + 34.0 * t
            let seg = Line.fromAngle (pt, angle, len)

            let hue = 155.0 + 185.0 * t
            let alpha = 0.25 + 0.75 * t
            let color = $"hsla({hue:F0}, 85%%, 62%%, {alpha:F2})"

            form
                .strokeOnly(color, 1.5 + 2.5 * t)
                .line(seg)
            |> ignore

            let dot = Circle.fromCenter (pt, 1.5 + 4.5 * t * t)

            form
                .fillOnly(color)
                .circle(dot)
            |> ignore

        let ringRadius = 18.0 + 6.0 * Math.Sin(time * 0.004)
        let ring = Circle.fromCenter (pointer, ringRadius)

        form
            .strokeOnly("#ffffffaa", 2.0)
            .circle(ring)
        |> ignore)

space.add player |> ignore

space.bindMouse().bindTouch().play () |> ignore
