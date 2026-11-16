// Apache ECharts from F#, in a page: a bar chart on the page's element, its
// option read back, coordinates converted, events, and disposal. Every check
// must hold.
module Test

open Fable.Core
open Fable.Core.JsInterop
open Bindings
open Bindings.Echarts

let mutable passed = 0

let check (ok: bool) (what: string) =
    if ok then passed <- passed + 1 else failwithf "check failed: %s" what

// A chart on the page's #chart (400 by 300).
let chart = Echarts.init (TestPage.element "chart")
check (chart.getWidth () = 400.0 && chart.getHeight () = 300.0) "the chart is as big as its element"
check (not (chart.isDisposed ())) "not disposed"
check (obj.ReferenceEquals (chart.getDom (), TestPage.element "chart")) "getDom"
check (chart.getId () <> "") "getId"
check (obj.ReferenceEquals (Echarts.getInstanceByDom (TestPage.element "chart") |> Option.get, chart)) "getInstanceByDom"

// FINDING 25: a series option (`BarSeriesOption`, `EChartsOptionXAxis`...) is an anonymous record
// of ~100 required fields (Bindings.01.echarts_and_1_more.fs:92, 11454, 11544): an option cannot be written
// but as an object made by hand.
let option: EChartsOption =
    unbox (
        createObj [
            "animation" ==> false
            "xAxis" ==> createObj [ "type" ==> "category"; "data" ==> [| "mon"; "tue"; "wed" |] ]
            "yAxis" ==> createObj [ "type" ==> "value"; "min" ==> 0; "max" ==> 40 ]
            "series" ==> [| createObj [ "type" ==> "bar"; "name" ==> "sales"; "data" ==> ResizeArray<float> [ 5.0; 20.0; 36.0 ] ] |]
            "grid" ==> createObj [ "left" ==> 50; "right" ==> 50; "top" ==> 50; "bottom" ==> 50 ]
        ]
    )
// FINDING 27: `chart.setOption option` is ambiguous between the two overloads (notMerge/lazyUpdate or opts): a flag picks one.
chart.setOption (option, false)

// Read back: the series are normalised to an array with their data.
let got = chart.getOption ()
let series = got.["series"] |> unbox<ResizeArray<obj>>
check (series.Count = 1) "one series"
check ((series.[0]?``type``: string) = "bar") "its type"
check ((series.[0]?name: string) = "sales") "its name"
check ((series.[0]?data: ResizeArray<float>) |> Seq.toList = [ 5.0; 20.0; 36.0 ]) "its data"
check (got.["animation"] |> unbox<bool> = false) "animation is as set"
let xAxis = got.["xAxis"] |> unbox<ResizeArray<obj>>
check ((xAxis.[0]?data: ResizeArray<string>) |> Seq.toList = [ "mon"; "tue"; "wed" ]) "the x axis categories"

// Coordinates: the category axis at the middle of the plot, and values on the y axis.
let top = chart.convertToPixel (ModelFinder.ModelFinderObject (unbox (createObj [ "yAxisIndex" ==> 0 ])), ScaleDataValue.Number 40.0)
let bottom = chart.convertToPixel (ModelFinder.ModelFinderObject (unbox (createObj [ "yAxisIndex" ==> 0 ])), ScaleDataValue.Number 0.0)
check (abs (top - 50.0) < 1.0) "the top of the y axis"
check (abs (bottom - 250.0) < 1.0) "the bottom of the y axis"
let mid = chart.convertToPixel (ModelFinder.ModelFinderObject (unbox (createObj [ "yAxisIndex" ==> 0 ])), ScaleDataValue.Number 20.0)
check (abs (mid - 150.0) < 1.0) "a value in the middle"
let back = chart.convertFromPixel (ModelFinder.ModelFinderObject (unbox (createObj [ "yAxisIndex" ==> 0 ])), 150.0)
check (abs (back - 20.0) < 0.5) "convertFromPixel"
check (chart.containPixel (ModelFinder.ModelFinderObject (unbox (createObj [ "gridIndex" ==> 0 ])), ResizeArray [ 200.0; 150.0 ])) "containPixel inside"
check (not (chart.containPixel (ModelFinder.ModelFinderObject (unbox (createObj [ "gridIndex" ==> 0 ])), ResizeArray [ 5.0; 5.0 ]))) "containPixel outside"

// Setting again merges: a new series data keeps the rest.
chart.setOption (unbox<EChartsOption> (createObj [ "series" ==> [| createObj [ "data" ==> ResizeArray<float> [ 10.0; 10.0; 10.0 ] ] |] ]), false)
let merged = chart.getOption().["series"] |> unbox<ResizeArray<obj>>
check ((merged.[0]?data: ResizeArray<float>) |> Seq.toList = [ 10.0; 10.0; 10.0 ] && (merged.[0]?``type``: string) = "bar") "setOption merges"

// The rendered chart as an image.
check (chart.getDataURL () |> fun u -> u.StartsWith "data:image/png") "getDataURL"

// Events: `on` and `trigger`. FINDING 26: the handler is typed `ResizeArray<obj> -> BooleanOrVoid` (rest
// arguments as an array) but is called with the arguments one by one.
let mutable fired = 0
let handler: ResizeArray<obj> -> BooleanOrVoid =
    let f = fun (_: ResizeArray<obj>) -> fired <- fired + 1; U2.Case2 ()
    f
chart.on<obj, string> ("ping", handler) |> ignore
chart.trigger<string> ("ping", box 1) |> ignore
chart.trigger<string> ("ping", box 2) |> ignore
check (fired = 2) "an event reaches its handler"
chart.off (U3.Case1 "ping") |> ignore
chart.trigger<string> ("ping", box 3) |> ignore
check (fired = 2) "off"

// Resize and clear.
chart.resize (unbox (createObj [ "width" ==> 200; "height" ==> 100 ]))
check (chart.getWidth () = 200.0 && chart.getHeight () = 100.0) "resize"
chart.clear ()
check ((chart.getOption().["series"] |> unbox<ResizeArray<obj>>).Count = 0) "clear"

// Disposal.
chart.dispose ()
check (chart.isDisposed ()) "dispose"
check (Echarts.getInstanceByDom (TestPage.element "chart") |> Option.isNone) "no instance after dispose"

printfn "%d checks passed" passed
