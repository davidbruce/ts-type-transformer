module Program

open Dom
open type Dom.Globals
open Echarts
open type Echarts.Globals

let private setChartOption (chart: ECharts) (opt: obj) =
    chart.setOption (unbox<EChartsCoreOption> opt, false)

let private chart = init (document.getElementById "chart")

let private makeAxis () =
    {|
        ``type`` = "value"
        min = 0
        max = 100
        splitLine = {| lineStyle = {| color = "#e5e5e5" |} |}
        axisLabel = {| color = "#555555" |}
    |}

setChartOption chart {|
    animation = false
    backgroundColor = "#ffffff"
    legend = {| top = 8; textStyle = {| color = "#111111" |} |}
    grid = {| left = 48; right = 24; top = 48; bottom = 36 |}
    xAxis = makeAxis ()
    yAxis = makeAxis ()
    series = [|
        box {|
            ``type`` = "scatter"
            name = "Cluster A"
            symbol = "circle"
            symbolSize = 10
            itemStyle = {| color = "#111111" |}
            data = [| [|14;22|]; [|21;31|]; [|28;27|]; [|33;42|]; [|40;48|]; [|46;53|]; [|54;61|]; [|62;59|]; [|71;74|]; [|82;85|] |]
        |}
        box {|
            ``type`` = "scatter"
            name = "Cluster B"
            symbol = "rect"
            symbolSize = 10
            itemStyle = {| color = "#ffffff"; borderColor = "#111111"; borderWidth = 2 |}
            data = [| [|18;64|]; [|25;72|]; [|31;68|]; [|39;79|]; [|47;65|]; [|55;81|]; [|64;77|]; [|72;88|]; [|79;82|]; [|88;92|] |]
        |}
    |]
|}

let private metaEl = document.getElementById "meta"
metaEl.textContent <- sprintf "Canvas: %dx%d · 20 data points across 2 clusters" (int (chart.getWidth ())) (int (chart.getHeight ()))
