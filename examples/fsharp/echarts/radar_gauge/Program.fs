module Program

open Dom
open type Dom.Globals
open Echarts
open type Echarts.Globals

let private setChartOption (chart: ECharts) (opt: obj) =
    chart.setOption (unbox<EChartsCoreOption> opt, false)

let private radar = init ((document.getElementById "chart-radar").Value)
let private gauge = init ((document.getElementById "chart-gauge").Value)

setChartOption radar {|
    animation = false
    backgroundColor = "#ffffff"
    legend = {| bottom = 6; textStyle = {| color = "#111111" |} |}
    radar = {|
        radius = "62%"
        axisName = {| color = "#111111" |}
        splitLine = {| lineStyle = {| color = "#d4d4d4" |} |}
        splitArea = {| areaStyle = {| color = [| "#ffffff"; "#f7f7f7" |] |} |}
        axisLine = {| lineStyle = {| color = "#b5b5b5" |} |}
        indicator = [|
            {| name = "Throughput"; max = 100 |}
            {| name = "Memory"; max = 100 |}
            {| name = "Startup"; max = 100 |}
            {| name = "Type Safety"; max = 100 |}
            {| name = "Bundle"; max = 100 |}
        |]
    |}
    series = [|
        {|
            ``type`` = "radar"
            data = [|
                box {|
                    name = "WASM Target"
                    value = [| 92; 86; 88; 96; 84 |]
                    lineStyle = {| color = "#111111"; width = 2 |}
                    itemStyle = {| color = "#111111" |}
                    areaStyle = {| color = "rgba(17,17,17,0.12)" |}
                |}
                box {|
                    name = "Baseline"
                    value = [| 70; 68; 74; 72; 66 |]
                    lineStyle = {| color = "#737373"; width = 1.5; ``type`` = "dashed" |}
                    itemStyle = {| color = "#737373" |}
                |}
            |]
        |}
    |]
|}

setChartOption gauge {|
    animation = false
    backgroundColor = "#ffffff"
    series = [|
        {|
            ``type`` = "gauge"
            min = 0
            max = 100
            progress = {| show = true; width = 14; itemStyle = {| color = "#111111" |} |}
            axisLine = {| lineStyle = {| width = 14; color = [| [| box 1; box "#e5e5e5" |] |] |} |}
            axisTick = {| lineStyle = {| color = "#737373" |} |}
            splitLine = {| lineStyle = {| color = "#111111" |} |}
            axisLabel = {| color = "#555555" |}
            pointer = {| itemStyle = {| color = "#111111" |} |}
            detail = {| valueAnimation = false; formatter = "{value}%"; color = "#111111"; fontSize = 22 |}
            title = {| color = "#555555"; fontSize = 12 |}
            data = [| {| value = 84; name = "Score" |} |]
        |}
    |]
|}

let private metaEl = (document.getElementById "meta").Value
metaEl.textContent <- sprintf "Radar: %dx%d · Gauge: %dx%d · Score 84%%" (int (radar.getWidth ())) (int (radar.getHeight ())) (int (gauge.getWidth ())) (int (gauge.getHeight ()))
