import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node
import echarts as ec
import echarts/e_charts_core_option as raw
import echarts/e_charts_type
import echarts/ec_unit_option as unit
import echarts/runtime
import gleam/dynamic.{type Dynamic}
import gleam/float
import gleam/int
import gleam/list
import gleam/option.{None, Some}

fn obj(entries: List(#(String, Dynamic))) -> Dynamic {
  runtime.js_object(entries)
}

fn value_axis() -> Dynamic {
  obj([
    #("type", dynamic.string("value")),
    #("min", dynamic.float(0.0)),
    #("max", dynamic.float(100.0)),
    #(
      "splitLine",
      obj([#("lineStyle", obj([#("color", dynamic.string("#e5e5e5"))]))]),
    ),
    #("axisLabel", obj([#("color", dynamic.string("#555555"))])),
  ])
}

fn pts(coords: List(#(Float, Float))) -> Dynamic {
  dynamic.array(
    list.map(coords, fn(p) {
      dynamic.array([dynamic.float(p.0), dynamic.float(p.1)])
    }),
  )
}

pub fn main() -> Nil {
  let doc = core.get_document()
  case document.get_element_by_id(doc, "chart") {
    None -> Nil
    Some(box) -> {
      let chart = ec.init_with(Some(box), None, None)
      let opt =
        raw.create()
        |> unit.with_animation(False)
        |> unit.with_at("backgroundColor", dynamic.string("#ffffff"))
        |> unit.with_at(
          "legend",
          obj([
            #("top", dynamic.float(8.0)),
            #("textStyle", obj([#("color", dynamic.string("#111111"))])),
          ]),
        )
        |> unit.with_at(
          "grid",
          obj([
            #("left", dynamic.float(48.0)),
            #("right", dynamic.float(24.0)),
            #("top", dynamic.float(48.0)),
            #("bottom", dynamic.float(36.0)),
          ]),
        )
        |> unit.with_at("xAxis", value_axis())
        |> unit.with_at("yAxis", value_axis())
        |> unit.with_at(
          "series",
          dynamic.array([
            obj([
              #("type", dynamic.string("scatter")),
              #("name", dynamic.string("Cluster A")),
              #("symbol", dynamic.string("circle")),
              #("symbolSize", dynamic.float(10.0)),
              #("itemStyle", obj([#("color", dynamic.string("#111111"))])),
              #(
                "data",
                pts([
                  #(14.0, 22.0),
                  #(21.0, 31.0),
                  #(28.0, 27.0),
                  #(33.0, 42.0),
                  #(40.0, 48.0),
                  #(46.0, 53.0),
                  #(54.0, 61.0),
                  #(62.0, 59.0),
                  #(71.0, 74.0),
                  #(82.0, 85.0),
                ]),
              ),
            ]),
            obj([
              #("type", dynamic.string("scatter")),
              #("name", dynamic.string("Cluster B")),
              #("symbol", dynamic.string("rect")),
              #("symbolSize", dynamic.float(10.0)),
              #(
                "itemStyle",
                obj([
                  #("color", dynamic.string("#ffffff")),
                  #("borderColor", dynamic.string("#111111")),
                  #("borderWidth", dynamic.float(2.0)),
                ]),
              ),
              #(
                "data",
                pts([
                  #(18.0, 64.0),
                  #(25.0, 72.0),
                  #(31.0, 68.0),
                  #(39.0, 79.0),
                  #(47.0, 65.0),
                  #(55.0, 81.0),
                  #(64.0, 77.0),
                  #(72.0, 88.0),
                  #(79.0, 82.0),
                  #(88.0, 92.0),
                ]),
              ),
            ]),
          ]),
        )
      e_charts_type.set_option(chart, opt)

      case document.get_element_by_id(doc, "meta") {
        None -> Nil
        Some(meta) -> {
          let w = int.to_string(float.round(e_charts_type.get_width(chart)))
          let h = int.to_string(float.round(e_charts_type.get_height(chart)))
          node.set_text_content(
            meta,
            Some(
              "Canvas: "
              <> w
              <> "x"
              <> h
              <> " · 20 data points across 2 clusters",
            ),
          )
        }
      }
      Nil
    }
  }
}
