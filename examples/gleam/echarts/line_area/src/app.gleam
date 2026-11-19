import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node
import echarts/areas/main as ec
import echarts/areas/main/e_charts_core_option as raw
import echarts/areas/main/e_charts_type
import echarts/areas/main/ec_unit_option as unit
import echarts/runtime
import gleam/dynamic.{type Dynamic}
import gleam/float
import gleam/int
import gleam/list
import gleam/option.{None, Some}

fn obj(entries: List(#(String, Dynamic))) -> Dynamic {
  runtime.js_object(entries)
}

pub fn main() -> Nil {
  let doc = core.get_document()
  case document.get_element_by_id(doc, "chart") {
    None -> Nil
    Some(box) -> {
      let chart = ec.init(Some(box), None, None)
      let opt = raw.create()
      unit.set_animation(opt, Some(False))
      unit.set_at(opt, "backgroundColor", dynamic.string("#ffffff"))
      unit.set_at(
        opt,
        "legend",
        obj([
          #("top", dynamic.float(8.0)),
          #("textStyle", obj([#("color", dynamic.string("#111111"))])),
        ]),
      )
      unit.set_at(opt, "tooltip", obj([#("trigger", dynamic.string("axis"))]))
      unit.set_at(
        opt,
        "grid",
        obj([
          #("left", dynamic.float(48.0)),
          #("right", dynamic.float(24.0)),
          #("top", dynamic.float(48.0)),
          #("bottom", dynamic.float(36.0)),
        ]),
      )
      unit.set_at(
        opt,
        "xAxis",
        obj([
          #("type", dynamic.string("category")),
          #("boundaryGap", dynamic.bool(False)),
          #(
            "data",
            dynamic.array(list.map(["W1", "W2", "W3", "W4", "W5", "W6", "W7", "W8"], dynamic.string)),
          ),
          #(
            "axisLine",
            obj([#("lineStyle", obj([#("color", dynamic.string("#111111"))]))]),
          ),
          #("axisLabel", obj([#("color", dynamic.string("#555555"))])),
        ]),
      )
      unit.set_at(
        opt,
        "yAxis",
        obj([
          #("type", dynamic.string("value")),
          #("min", dynamic.float(0.0)),
          #("max", dynamic.float(100.0)),
          #(
            "splitLine",
            obj([#("lineStyle", obj([#("color", dynamic.string("#e5e5e5"))]))]),
          ),
          #("axisLabel", obj([#("color", dynamic.string("#555555"))])),
        ]),
      )
      unit.set_at(
        opt,
        "series",
        dynamic.array([
          obj([
            #("type", dynamic.string("line")),
            #("name", dynamic.string("Throughput")),
            #("smooth", dynamic.bool(True)),
            #(
              "data",
              dynamic.array(list.map([28.0, 44.0, 39.0, 62.0, 58.0, 76.0, 81.0, 89.0], dynamic.float)),
            ),
            #(
              "lineStyle",
              obj([#("color", dynamic.string("#111111")), #("width", dynamic.float(2.0))]),
            ),
            #("itemStyle", obj([#("color", dynamic.string("#111111"))])),
            #("areaStyle", obj([#("color", dynamic.string("rgba(17,17,17,0.12)"))])),
          ]),
          obj([
            #("type", dynamic.string("line")),
            #("name", dynamic.string("Baseline")),
            #("smooth", dynamic.bool(False)),
            #(
              "data",
              dynamic.array(list.map([35.0, 38.0, 42.0, 46.0, 50.0, 54.0, 58.0, 62.0], dynamic.float)),
            ),
            #(
              "lineStyle",
              obj([
                #("color", dynamic.string("#737373")),
                #("width", dynamic.float(1.5)),
                #("type", dynamic.string("dashed")),
              ]),
            ),
            #("itemStyle", obj([#("color", dynamic.string("#737373"))])),
          ]),
        ]),
      )
      e_charts_type.set_option(chart, opt, None, None)

      case document.get_element_by_id(doc, "meta") {
        None -> Nil
        Some(meta) -> {
          let w = int.to_string(float.round(e_charts_type.get_width(chart)))
          let h = int.to_string(float.round(e_charts_type.get_height(chart)))
          node.set_text_content(
            meta,
            Some("Canvas: " <> w <> "x" <> h <> " · 2 series (Throughput + Baseline)"),
          )
        }
      }
      Nil
    }
  }
}
