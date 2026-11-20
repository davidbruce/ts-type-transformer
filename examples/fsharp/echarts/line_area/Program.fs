module Program

open Dom
open type Dom.Globals
open Echarts
open type Echarts.Globals

let private setChartOption (chart: ECharts) (opt: obj) =
    chart.setOption (unbox<EChartsCoreOption> opt, false)

let private chart = init (document.getElementById "chart")

setChartOption chart {|
    animation = false
    backgroundColor = "#ffffff"
    legend = {| top = 8; textStyle = {| color = "#111111" |} |}
    tooltip = {| trigger = "axis" |}
    grid = {| left = 48; right = 24; top = 48; bottom = 36 |}
    xAxis = {|
        ``type`` = "category"
        boundaryGap = false
        data = [| "W1"; "W2"; "W3"; "W4"; "W5"; "W6"; "W7"; "W8" |]
        axisLine = {| lineStyle = {| color = "#111111" |} |}
        axisLabel = {| color = "#555555" |}
    |}
    yAxis = {|
        ``type`` = "value"
        min = 0
        max = 100
        splitLine = {| lineStyle = {| color = "#e5e5e5" |} |}
        axisLabel = {| color = "#555555" |}
    |}
    series = [|
        box {|
            ``type`` = "line"
            name = "Throughput"
            smooth = true
            data = ResizeArray [ 28; 44; 39; 62; 58; 76; 81; 89 ]
            lineStyle = {| color = "#111111"; width = 2 |}
            itemStyle = {| color = "#111111" |}
            areaStyle = {| color = "rgba(17,17,17,0.12)" |}
        |}
        box {|
            ``type`` = "line"
            name = "Baseline"
            smooth = false
            data = ResizeArray [ 35; 38; 42; 46; 50; 54; 58; 62 ]
            lineStyle = {| color = "#737373"; width = 1.5; ``type`` = "dashed" |}
            itemStyle = {| color = "#737373" |}
        |}
    |]
|}

let private metaEl = document.getElementById "meta"
metaEl.textContent <- sprintf "Canvas: %dx%d · 2 series (Throughput + Baseline)" (int (chart.getWidth ())) (int (chart.getHeight ()))
