module Program

open Dom
open type Dom.Globals
open Echarts
open type Echarts.Globals

let private setChartOption (chart: ECharts) (opt: obj) =
    chart.setOption (unbox<EChartsCoreOption> opt, false)

let private chart = init ((document.getElementById "chart").Value)

setChartOption chart {|
    animation = false
    backgroundColor = "#ffffff"
    title = {| text = "Monthly Output"; left = 16; top = 12; textStyle = {| color = "#111111"; fontSize = 13 |} |}
    grid = {| left = 48; right = 24; top = 48; bottom = 36 |}
    xAxis = {|
        ``type`` = "category"
        data = [| "Jan"; "Feb"; "Mar"; "Apr"; "May"; "Jun" |]
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
        {|
            ``type`` = "bar"
            name = "Units"
            barWidth = "46%"
            data = ResizeArray [ 42; 68; 55; 84; 71; 93 ]
            itemStyle = {| color = "#111111" |}
        |}
    |]
|}

let private metaEl = (document.getElementById "meta").Value
metaEl.textContent <- sprintf "Canvas: %dx%d · 6 bars · Monochrome #111111" (int (chart.getWidth ())) (int (chart.getHeight ()))
