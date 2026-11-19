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
opt?legend <- createObj [ "top" ==> 8; "textStyle" ==> createObj [ "color" ==> "#111111" ] ]
opt?tooltip <- createObj [ "trigger" ==> "axis" ]
opt?grid <- createObj [ "left" ==> 48; "right" ==> 24; "top" ==> 48; "bottom" ==> 36 ]
opt?xAxis <- createObj [
    "type" ==> "category"
    "boundaryGap" ==> false
    "data" ==> [| "W1"; "W2"; "W3"; "W4"; "W5"; "W6"; "W7"; "W8" |]
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
        "type" ==> "line"
        "name" ==> "Throughput"
        "smooth" ==> true
        "data" ==> [| box 28; box 44; box 39; box 62; box 58; box 76; box 81; box 89 |]
        "lineStyle" ==> createObj [ "color" ==> "#111111"; "width" ==> 2 ]
        "itemStyle" ==> createObj [ "color" ==> "#111111" ]
        "areaStyle" ==> createObj [ "color" ==> "rgba(17,17,17,0.12)" ]
    ]
    createObj [
        "type" ==> "line"
        "name" ==> "Baseline"
        "smooth" ==> false
        "data" ==> [| box 35; box 38; box 42; box 46; box 50; box 54; box 58; box 62 |]
        "lineStyle" ==> createObj [ "color" ==> "#737373"; "width" ==> 1.5; "type" ==> "dashed" ]
        "itemStyle" ==> createObj [ "color" ==> "#737373" ]
    ]
|]

chart.setOption (opt, false)

let private metaEl = document.getElementById "meta"
metaEl.textContent <- sprintf "Canvas: %dx%d · 2 series (Throughput + Baseline)" (int (chart.getWidth ())) (int (chart.getHeight ()))
