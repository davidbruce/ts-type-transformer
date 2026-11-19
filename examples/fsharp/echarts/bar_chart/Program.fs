module Program

open Fable.Core.JsInterop
open Dom
open type Dom.Globals
open Echarts
open type Echarts.Globals

let private boxEl = unbox<HTMLElement> (document.getElementById "chart")
let private chart = init boxEl

let private opt = createEmpty<EChartsCoreOption>
opt?animation <- false
opt?backgroundColor <- "#ffffff"
opt?title <- createObj [ "text" ==> "Monthly Output"; "left" ==> 16; "top" ==> 12; "textStyle" ==> createObj [ "color" ==> "#111111"; "fontSize" ==> 13 ] ]
opt?grid <- createObj [ "left" ==> 48; "right" ==> 24; "top" ==> 48; "bottom" ==> 36 ]
opt?xAxis <- createObj [
    "type" ==> "category"
    "data" ==> [| "Jan"; "Feb"; "Mar"; "Apr"; "May"; "Jun" |]
    "axisLine" ==> createObj [ "lineStyle" ==> createObj [ "color" ==> "#111111" ] ]
    "axisLabel" ==> createObj [ "color" ==> "#555555" ]
]
opt?yAxis <- createObj [
    "type" ==> "value"
    "min" ==> 0
    "max" ==> 100
    "splitLine" ==> createObj [ "lineStyle" ==> createObj [ "color" ==> "#e5e5e5" ] ]
    "axisLabel" ==> createObj [ "color" ==> "#555555" ]
]
opt?series <- [|
    createObj [
        "type" ==> "bar"
        "name" ==> "Units"
        "barWidth" ==> "46%"
        "data" ==> [| box 42; box 68; box 55; box 84; box 71; box 93 |]
        "itemStyle" ==> createObj [ "color" ==> "#111111" ]
    ]
|]

chart.setOption (opt, false)

let private metaEl = document.getElementById "meta"
metaEl.textContent <- sprintf "Canvas: %dx%d · 6 bars · Monochrome #111111" (int (chart.getWidth ())) (int (chart.getHeight ()))
