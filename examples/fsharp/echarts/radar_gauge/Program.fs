module Program

open Fable.Core.JsInterop
open Dom
open type Dom.Globals
open Echarts
open type Echarts.Globals

let private radar = init (document.getElementById "chart-radar")
let private gauge = init (document.getElementById "chart-gauge")

let private radarOpt = createEmpty<EChartsCoreOption>
radarOpt?animation <- false
radarOpt?backgroundColor <- "#ffffff"
radarOpt?legend <- createObj [ "bottom" ==> 6; "textStyle" ==> createObj [ "color" ==> "#111111" ] ]
radarOpt?radar <- createObj [
    "radius" ==> "62%"
    "axisName" ==> createObj [ "color" ==> "#111111" ]
    "splitLine" ==> createObj [ "lineStyle" ==> createObj [ "color" ==> "#d4d4d4" ] ]
    "splitArea" ==> createObj [ "areaStyle" ==> createObj [ "color" ==> [| "#ffffff"; "#f7f7f7" |] ] ]
    "axisLine" ==> createObj [ "lineStyle" ==> createObj [ "color" ==> "#b5b5b5" ] ]
    "indicator" ==> [|
        createObj [ "name" ==> "Throughput"; "max" ==> 100 ]
        createObj [ "name" ==> "Memory"; "max" ==> 100 ]
        createObj [ "name" ==> "Startup"; "max" ==> 100 ]
        createObj [ "name" ==> "Type Safety"; "max" ==> 100 ]
        createObj [ "name" ==> "Bundle"; "max" ==> 100 ]
    |]
]
radarOpt?series <- [|
    createObj [
        "type" ==> "radar"
        "data" ==> [|
            createObj [
                "name" ==> "WASM Target"
                "value" ==> [| 92; 86; 88; 96; 84 |]
                "lineStyle" ==> createObj [ "color" ==> "#111111"; "width" ==> 2 ]
                "itemStyle" ==> createObj [ "color" ==> "#111111" ]
                "areaStyle" ==> createObj [ "color" ==> "rgba(17,17,17,0.12)" ]
            ]
            createObj [
                "name" ==> "Baseline"
                "value" ==> [| 70; 68; 74; 72; 66 |]
                "lineStyle" ==> createObj [ "color" ==> "#737373"; "width" ==> 1.5; "type" ==> "dashed" ]
                "itemStyle" ==> createObj [ "color" ==> "#737373" ]
            ]
        |]
    ]
|]
radar.setOption (radarOpt, false)

let private gaugeOpt = createEmpty<EChartsCoreOption>
gaugeOpt?animation <- false
gaugeOpt?backgroundColor <- "#ffffff"
gaugeOpt?series <- [|
    createObj [
        "type" ==> "gauge"
        "min" ==> 0
        "max" ==> 100
        "progress" ==> createObj [ "show" ==> true; "width" ==> 14; "itemStyle" ==> createObj [ "color" ==> "#111111" ] ]
        "axisLine" ==> createObj [ "lineStyle" ==> createObj [ "width" ==> 14; "color" ==> [| [| box 1; box "#e5e5e5" |] |] ] ]
        "axisTick" ==> createObj [ "lineStyle" ==> createObj [ "color" ==> "#737373" ] ]
        "splitLine" ==> createObj [ "lineStyle" ==> createObj [ "color" ==> "#111111" ] ]
        "axisLabel" ==> createObj [ "color" ==> "#555555" ]
        "pointer" ==> createObj [ "itemStyle" ==> createObj [ "color" ==> "#111111" ] ]
        "detail" ==> createObj [ "valueAnimation" ==> false; "formatter" ==> "{value}%"; "color" ==> "#111111"; "fontSize" ==> 22 ]
        "title" ==> createObj [ "color" ==> "#555555"; "fontSize" ==> 12 ]
        "data" ==> [| createObj [ "value" ==> 84; "name" ==> "Score" ] |]
    ]
|]
gauge.setOption (gaugeOpt, false)

let private metaEl = document.getElementById "meta"
metaEl.textContent <- sprintf "Radar: %dx%d · Gauge: %dx%d · Score 84%%" (int (radar.getWidth ())) (int (radar.getHeight ())) (int (gauge.getWidth ())) (int (gauge.getHeight ()))
