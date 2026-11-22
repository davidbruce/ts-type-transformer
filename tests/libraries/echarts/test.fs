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

let option =
    EChartsOption.Create (
        animation = false,
        xAxis =
            U5.Case1 (
                EChartsOptionXAxisData.Create (
                    data = ResizeArray [
                        AngleAxisComponentOptionCategoryData.String "mon"
                        AngleAxisComponentOptionCategoryData.String "tue"
                        AngleAxisComponentOptionCategoryData.String "wed"
                    ]
                )
            ),
        yAxis =
            U5.Case4 (
                EChartsOptionYAxisScale.Create (
                    min = AngleAxisComponentOptionCategoryMin.Number 0.0,
                    max = AngleAxisComponentOptionCategoryMin.Number 40.0
                )
            ),
        series =
            U9.Case1 (
                BarSeriesOption.Create (
                    name = OptionName.String "sales",
                    data = ResizeArray [
                        Some (BarSeriesOptionData.Number 5.0)
                        Some (BarSeriesOptionData.Number 20.0)
                        Some (BarSeriesOptionData.Number 36.0)
                    ]
                )
            ),
        grid =
            EChartsOptionGrid.GridComponentOption (
                GridComponentOption.Create (
                    show = false,
                    left = PositionSizeOption.Number 50.0,
                    right = PositionSizeOption.Number 50.0,
                    top = PositionSizeOption.Number 50.0,
                    bottom = PositionSizeOption.Number 50.0
                )
            )
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
let yFinder = ModelFinder.ModelFinderObject (ModelFinderObject.Create (yAxisIndex = ModelFinderIndexQuery.Number 0.0))
let gridFinder = ModelFinder.ModelFinderObject (ModelFinderObject.Create (gridIndex = ModelFinderIndexQuery.Number 0.0))
let top = chart.convertToPixel (yFinder, ScaleDataValue.Number 40.0)
let bottom = chart.convertToPixel (yFinder, ScaleDataValue.Number 0.0)
check (abs (top - 50.0) < 1.0) "the top of the y axis"
check (abs (bottom - 250.0) < 1.0) "the bottom of the y axis"
let mid = chart.convertToPixel (yFinder, ScaleDataValue.Number 20.0)
check (abs (mid - 150.0) < 1.0) "a value in the middle"
let back = chart.convertFromPixel (yFinder, 150.0)
check (abs (back - 20.0) < 0.5) "convertFromPixel"
check (chart.containPixel (gridFinder, ResizeArray [ 200.0; 150.0 ])) "containPixel inside"
check (not (chart.containPixel (gridFinder, ResizeArray [ 5.0; 5.0 ]))) "containPixel outside"

// Setting again merges: a new series data keeps the rest.
let updated =
    EChartsOption.Create (
        series =
            U9.Case1 (
                BarSeriesOption.Create (
                    data = ResizeArray [
                        Some (BarSeriesOptionData.Number 10.0)
                        Some (BarSeriesOptionData.Number 10.0)
                        Some (BarSeriesOptionData.Number 10.0)
                    ]
                )
            )
    )
chart.setOption (updated, false)
let merged = chart.getOption().["series"] |> unbox<ResizeArray<obj>>
check ((merged.[0]?data: ResizeArray<float>) |> Seq.toList = [ 10.0; 10.0; 10.0 ] && (merged.[0]?``type``: string) = "bar") "setOption merges"

// The rendered chart as an image.
check (chart.getDataURL () |> fun u -> u.StartsWith "data:image/png") "getDataURL"

// Events: `on` and `trigger`.
let mutable fired = 0
let mutable seen: (float * string) list = []
let handler =
    Echarts.OnHandler.Create(fun args ->
        fired <- fired + 1
        seen <- seen @ [ (unbox<float> args.[0], unbox<string> args.[1]) ]
        U2.Case2 ())
chart.on<obj, string> ("ping", handler) |> ignore
chart.trigger<string> ("ping", box 1, box "a") |> ignore
chart.trigger<string> ("ping", box 2, box "b") |> ignore
check (fired = 2) "an event reaches its handler"
check (seen = [ (1.0, "a"); (2.0, "b") ]) "it reads all triggered arguments"
chart.off "ping" |> ignore
chart.trigger<string> ("ping", box 3) |> ignore
check (fired = 2) "off"

// Resize and clear.
chart.resize (ResizeOpts.Create (width = StringOrNumber.Number 200.0, height = StringOrNumber.Number 100.0))
check (chart.getWidth () = 200.0 && chart.getHeight () = 100.0) "resize"
chart.clear ()
check ((chart.getOption().["series"] |> unbox<ResizeArray<obj>>).Count = 0) "clear"

// Disposal.
chart.dispose ()
check (chart.isDisposed ()) "dispose"
check (Echarts.getInstanceByDom (TestPage.element "chart") |> Option.isNone) "no instance after dispose"

printfn "%d checks passed" passed
