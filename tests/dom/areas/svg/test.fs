// SVG from F#: elements made in the SVG namespace, attributes set, geometry
// read back (bounding boxes, lengths, a path's length). Every check must hold.
module Test

open Fable.Core
open Fable.Core.JsInterop
open Bindings

let mutable passed = 0

let check (ok: bool) (what: string) =
    if ok then passed <- passed + 1 else failwithf "check failed: %s" what

let same (a: obj) (b: obj) = obj.ReferenceEquals(a, b)

let near (a: float) (b: float) = abs (a - b) < 0.01

let SVGNS = "http://www.w3.org/2000/svg"

// An <svg> in the body, made with createElementNS and a typed key.
let svg = document.createElementNS_svg SVGElementTagNameMapKeys.svg
svg.setAttribute ("width", "200")
svg.setAttribute ("height", "100")
svg.setAttribute ("viewBox", "0 0 400 200")
document.body.appendChild svg |> ignore
check (svg.namespaceURI = Some SVGNS) "namespaceURI"
check (svg.localName = "svg") "localName"
check (svg.tagName = "svg") "tagName is lower case in SVG"
check (svg.isConnected) "svg is in the page"

// The generic createElementNS agrees.
// FINDING 8: createElementNS (ns, name) is ambiguous: two overloads of the same shape (Bindings.10.core.fs:4744-4745,
// `namespaceURI` + ElementCreationOptions and `namespace` + StringOrElementCreationOptions); named arguments pick one.
let viaNS = document.createElementNS (namespaceURI = SVGNS, qualifiedName = "g")
check (viaNS.namespaceURI = Some SVGNS && viaNS.localName = "g") "createElementNS"

// Lengths and the viewBox.
check (svg.width.baseVal.value = 200.0) "width baseVal"
check (svg.height.baseVal.value = 100.0) "height baseVal"
check (svg.width.baseVal.unitType = 1.0) "unitType is a plain number"
let vb = svg.viewBox.baseVal
check (vb.x = 0.0 && vb.y = 0.0 && vb.width = 400.0 && vb.height = 200.0) "viewBox baseVal"
svg.setAttribute ("viewBox", "10 20 300 150")
check (vb.x = 10.0 && vb.y = 20.0 && vb.width = 300.0 && vb.height = 150.0) "viewBox is live"
svg.setAttribute ("viewBox", "0 0 400 200")
svg.width.baseVal.valueAsString <- "300px"
check (svg.getAttribute "width" = Some "300px") "setting a length writes the attribute"
check (svg.width.baseVal.value = 300.0) "and reads back"
svg.setAttribute ("width", "200")

// A rect.
let rect = document.createElementNS_svg SVGElementTagNameMapKeys.rect
rect.setAttribute ("x", "20")
rect.setAttribute ("y", "30")
rect.setAttribute ("width", "100")
rect.setAttribute ("height", "50")
rect.setAttribute ("fill", "tomato")
rect.setAttribute ("id", "r")
svg.appendChild rect |> ignore
check (rect.x.baseVal.value = 20.0 && rect.y.baseVal.value = 30.0) "rect x, y"
check (rect.width.baseVal.value = 100.0 && rect.height.baseVal.value = 50.0) "rect size"
check (rect.getAttribute "fill" = Some "tomato") "rect fill"
let bbox = rect.getBBox ()
check (bbox.x = 20.0 && bbox.y = 30.0 && bbox.width = 100.0 && bbox.height = 50.0) "rect getBBox"
check (same (document.getElementById "r").Value rect) "found by id from the document"
check (same rect.parentNode.Value svg) "parent is the svg"
check (same (svg.querySelector "rect").Value rect) "querySelector in the svg"

// A length set through the bindings changes the attribute and the box.
rect.width.baseVal.value <- 150.0
check (rect.getAttribute "width" = Some "150") "length value writes the attribute"
check ((rect.getBBox ()).width = 150.0) "bbox follows"
rect.width.baseVal.valueAsString <- "10%"
check (rect.width.baseVal.unitType = 2.0) "a percentage length"
check (rect.width.baseVal.valueInSpecifiedUnits = 10.0) "valueInSpecifiedUnits"
rect.setAttribute ("width", "100")

// A circle.
let circle = document.createElementNS_svg SVGElementTagNameMapKeys.circle
circle.setAttribute ("cx", "200")
circle.setAttribute ("cy", "100")
circle.setAttribute ("r", "40")
svg.appendChild circle |> ignore
check (circle.cx.baseVal.value = 200.0 && circle.cy.baseVal.value = 100.0 && circle.r.baseVal.value = 40.0) "circle attributes"
let cb = circle.getBBox ()
check (cb.x = 160.0 && cb.y = 60.0 && cb.width = 80.0 && cb.height = 80.0) "circle bbox"
check (abs (circle.getTotalLength () - 2.0 * System.Math.PI * 40.0) < 0.5) "circle total length"
check (circle.isPointInFill (DOMPointInit.Create (x = 200.0, y = 100.0))) "point in the circle"
check (not (circle.isPointInFill (DOMPointInit.Create (x = 10.0, y = 10.0)))) "point outside the circle"

// A line.
let line = document.createElementNS_svg SVGElementTagNameMapKeys.line
for (k, v) in [ "x1", "0"; "y1", "0"; "x2", "30"; "y2", "40"; "stroke", "black" ] do
    line.setAttribute (k, v)
svg.appendChild line |> ignore
check (line.x2.baseVal.value = 30.0 && line.y2.baseVal.value = 40.0) "line end"
check (near (line.getTotalLength ()) 50.0) "line length is 50 (3-4-5)"
let mid = line.getPointAtLength 25.0
check (near mid.x 15.0 && near mid.y 20.0) "getPointAtLength midpoint"

// A path and its length.
let path = document.createElementNS_svg SVGElementTagNameMapKeys.path
path.setAttribute ("d", "M 10 10 L 110 10 L 110 60 Z")
path.setAttribute ("fill", "none")
path.setAttribute ("stroke", "blue")
svg.appendChild path |> ignore
let expected = 100.0 + 50.0 + sqrt (100.0 * 100.0 + 50.0 * 50.0)
check (near (path.getTotalLength ()) expected) "path total length"
let pb = path.getBBox ()
check (pb.x = 10.0 && pb.y = 10.0 && pb.width = 100.0 && pb.height = 50.0) "path bbox"
let end1 = path.getPointAtLength 100.0
check (near end1.x 110.0 && near end1.y 10.0) "path corner by length"
check (path.isPointInFill (DOMPointInit.Create (x = 100.0, y = 15.0))) "inside the triangle"
path.setAttribute ("d", "M 0 0 H 50")
check (near (path.getTotalLength ()) 50.0) "changing d changes the length"
path.pathLength.baseVal <- 10.0
check (path.getAttribute "pathLength" = Some "10") "pathLength animated number"

// A group, with a transform, and text.
let g = document.createElementNS_svg SVGElementTagNameMapKeys.g
g.setAttribute ("transform", "translate(100 50)")
let inner = document.createElementNS_svg SVGElementTagNameMapKeys.rect
inner.setAttribute ("width", "20")
inner.setAttribute ("height", "10")
g.appendChild inner |> ignore
svg.appendChild g |> ignore
check (g.transform.baseVal.length = 1.0) "one transform in the list"
let t = g.transform.baseVal.getItem 0.0
check (t.``type`` = 2.0) "a translate transform"
check (t.matrix.e = 100.0 && t.matrix.f = 50.0) "translate matrix"
check ((g.getBBox ()).width = 20.0) "group bbox in its own coordinates"
let ctm = (inner.getCTM ()).Value
check (near ctm.a 0.5 && near ctm.d 0.5) "CTM carries the viewBox scale: 200px for 400 units"
check (near ctm.e 50.0 && near ctm.f 25.0) "CTM carries the translate, scaled"
let screen = (inner.getScreenCTM ()).Value
check (near screen.a 0.5 && near screen.d 0.5) "screen CTM has the same scale"
let origin = (svg.getBoundingClientRect ())
check (near screen.e (origin.left + 50.0) && near screen.f (origin.top + 25.0)) "screen CTM places the translate on the page"

let text = document.createElementNS_svg SVGElementTagNameMapKeys.text
text.setAttribute ("x", "10")
text.setAttribute ("y", "90")
text.textContent <- "label"
svg.appendChild text |> ignore
check (text.textContent = "label") "text content"
check ((text.getBBox ()).width > 10.0) "text has a width once laid out"
check (text.getNumberOfChars () = 5.0) "getNumberOfChars"
check (text.getComputedTextLength () > 10.0) "getComputedTextLength"

// The svg's own bookkeeping.
check (svg.children.length = 6.0) "six children"
check ((svg.getBoundingClientRect ()).width = 200.0) "rendered at 200px wide"
check (same (svg.getElementById "r").Value rect) "SVGSVGElement.getElementById"
let pt = svg.createSVGPoint ()
pt.x <- 5.0
pt.y <- 6.0
check (pt.x = 5.0 && pt.y = 6.0) "createSVGPoint"
let len = svg.createSVGLength ()
len.valueAsString <- "2in"
check (near len.value 192.0) "an inch is 96px"
// FINDING 9: SVGElement.className is typed obj (Bindings.23.svg.fs:309; in the DOM an SVGAnimatedString), so
// `.baseVal` does not compile; the animated string is read dynamically.
check (svg.className?baseVal = "") "className.baseVal"
svg.setAttribute ("class", "chart")
check (svg.className?baseVal = "chart") "className reflects the attribute"
check (svg.classList.contains "chart") "classList works on SVG"
svg.style.setProperty ("background", "white")
check (svg.style.getPropertyValue "background" <> "") "style on SVG"

// Removal.
svg.removeChild circle |> ignore
check (svg.children.length = 5.0) "removeChild"
check circle.parentNode.IsNone "the circle is detached"

printfn "%d checks passed" passed
