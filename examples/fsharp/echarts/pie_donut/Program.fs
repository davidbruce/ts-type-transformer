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
    color = [| "#111111"; "#404040"; "#737373"; "#a3a3a3"; "#d4d4d4" |]
    legend = {| bottom = 8; textStyle = {| color = "#111111" |} |}
    tooltip = {| trigger = "item" |}
    series = [|
        {|
            ``type`` = "pie"
            name = "Subsystems"
            radius = [| "44%"; "72%" |]
            center = [| "50%"; "46%" |]
            itemStyle = {| borderColor = "#ffffff"; borderWidth = 2 |}
            label = {| color = "#111111" |}
            data = [|
                {| name = "Runtime"; value = 34 |}
                {| name = "Bindings"; value = 26 |}
                {| name = "Glue"; value = 18 |}
                {| name = "Parser"; value = 14 |}
                {| name = "Tests"; value = 8 |}
            |]
        |}
    |]
|}

let private metaEl = document.getElementById "meta"
metaEl.textContent <- sprintf "Canvas: %dx%d · 5 monochrome segments · Total 100" (int (chart.getWidth ())) (int (chart.getHeight ()))
