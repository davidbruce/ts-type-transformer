module Program

open Fable.Core.JsInterop
open Dom
open type Dom.Globals
open Echarts
open type Echarts.Globals

let private chart = init (unbox<HTMLElement> (document.getElementById "chart"))

let private opt = createEmpty<EChartsCoreOption>
opt?animation <- false
opt?backgroundColor <- "#ffffff"
opt?color <- [| "#111111"; "#404040"; "#737373"; "#a3a3a3"; "#d4d4d4" |]
opt?legend <- createObj [ "bottom" ==> 8; "textStyle" ==> createObj [ "color" ==> "#111111" ] ]
opt?tooltip <- createObj [ "trigger" ==> "item" ]
opt?series <- [|
    createObj [
        "type" ==> "pie"
        "name" ==> "Subsystems"
        "radius" ==> [| "44%"; "72%" |]
        "center" ==> [| "50%"; "46%" |]
        "itemStyle" ==> createObj [ "borderColor" ==> "#ffffff"; "borderWidth" ==> 2 ]
        "label" ==> createObj [ "color" ==> "#111111" ]
        "data" ==> [|
            createObj [ "name" ==> "Runtime"; "value" ==> 34 ]
            createObj [ "name" ==> "Bindings"; "value" ==> 26 ]
            createObj [ "name" ==> "Glue"; "value" ==> 18 ]
            createObj [ "name" ==> "Parser"; "value" ==> 14 ]
            createObj [ "name" ==> "Tests"; "value" ==> 8 ]
        |]
    ]
|]

chart.setOption (opt, false)

let private metaEl = document.getElementById "meta"
metaEl.textContent <- sprintf "Canvas: %dx%d · 5 monochrome segments · Total 100" (int (chart.getWidth ())) (int (chart.getHeight ()))
