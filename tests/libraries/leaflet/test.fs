// Leaflet from F#, in a page: a map on the page's element. Every check must hold.
module Test

open Fable.Core
open Bindings
open Bindings.Leaflet

let mutable passed = 0

let check (ok: bool) (what: string) =
    if ok then passed <- passed + 1 else failwithf "check failed: %s" what

let london = Leaflet.latLng (51.505, -0.09)
let m = Leaflet.map (Content.HTMLElement (TestPage.element "map"))
m.setView (london, 13.0) |> ignore
check (m.getZoom () = 13.0) "getZoom"
check (m.getSize().x = 400.0) "the map is as wide as its element"

printfn "%d checks passed" passed
