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

fn ind(name: String) -> Dynamic {
  obj([#("name", dynamic.string(name)), #("max", dynamic.float(100.0))])
}

pub fn main() -> Nil {
  let doc = core.get_document()
  case document.get_element_by_id(doc, "chart-radar"), document.get_element_by_id(doc, "chart-gauge") {
    Some(r_box), Some(g_box) -> {
      let radar = ec.init(Some(r_box), None, None)
      let gauge = ec.init(Some(g_box), None, None)

      let r_opt = raw.create()
      unit.set_animation(r_opt, Some(False))
      unit.set_at(r_opt, "backgroundColor", dynamic.string("#ffffff"))
      unit.set_at(
        r_opt,
        "legend",
        obj([
          #("bottom", dynamic.float(6.0)),
          #("textStyle", obj([#("color", dynamic.string("#111111"))])),
        ]),
      )
      unit.set_at(
        r_opt,
        "radar",
        obj([
          #("radius", dynamic.string("62%")),
          #("axisName", obj([#("color", dynamic.string("#111111"))])),
          #("splitLine", obj([#("lineStyle", obj([#("color", dynamic.string("#d4d4d4"))]))])),
          #(
            "splitArea",
            obj([
              #(
                "areaStyle",
                obj([#("color", dynamic.array([dynamic.string("#ffffff"), dynamic.string("#f7f7f7")]))]),
              ),
            ]),
          ),
          #("axisLine", obj([#("lineStyle", obj([#("color", dynamic.string("#b5b5b5"))]))])),
          #(
            "indicator",
            dynamic.array([
              ind("Throughput"),
              ind("Memory"),
              ind("Startup"),
              ind("Type Safety"),
              ind("Bundle"),
            ]),
          ),
        ]),
      )
      unit.set_at(
        r_opt,
        "series",
        dynamic.array([
          obj([
            #("type", dynamic.string("radar")),
            #(
              "data",
              dynamic.array([
                obj([
                  #("name", dynamic.string("WASM Target")),
                  #("value", dynamic.array(list.map([92.0, 86.0, 88.0, 96.0, 84.0], dynamic.float))),
                  #("lineStyle", obj([#("color", dynamic.string("#111111")), #("width", dynamic.float(2.0))])),
                  #("itemStyle", obj([#("color", dynamic.string("#111111"))])),
                  #("areaStyle", obj([#("color", dynamic.string("rgba(17,17,17,0.12)"))])),
                ]),
                obj([
                  #("name", dynamic.string("Baseline")),
                  #("value", dynamic.array(list.map([70.0, 68.0, 74.0, 72.0, 66.0], dynamic.float))),
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
            ),
          ]),
        ]),
      )
      e_charts_type.set_option(radar, r_opt, None, None)

      let g_opt = raw.create()
      unit.set_animation(g_opt, Some(False))
      unit.set_at(g_opt, "backgroundColor", dynamic.string("#ffffff"))
      unit.set_at(
        g_opt,
        "series",
        dynamic.array([
          obj([
            #("type", dynamic.string("gauge")),
            #("min", dynamic.float(0.0)),
            #("max", dynamic.float(100.0)),
            #(
              "progress",
              obj([
                #("show", dynamic.bool(True)),
                #("width", dynamic.float(14.0)),
                #("itemStyle", obj([#("color", dynamic.string("#111111"))])),
              ]),
            ),
            #(
              "axisLine",
              obj([
                #(
                  "lineStyle",
                  obj([
                    #("width", dynamic.float(14.0)),
                    #("color", dynamic.array([dynamic.array([dynamic.float(1.0), dynamic.string("#e5e5e5")])])),
                  ]),
                ),
              ]),
            ),
            #("axisTick", obj([#("lineStyle", obj([#("color", dynamic.string("#737373"))]))])),
            #("splitLine", obj([#("lineStyle", obj([#("color", dynamic.string("#111111"))]))])),
            #("axisLabel", obj([#("color", dynamic.string("#555555"))])),
            #("pointer", obj([#("itemStyle", obj([#("color", dynamic.string("#111111"))]))])),
            #(
              "detail",
              obj([
                #("valueAnimation", dynamic.bool(False)),
                #("formatter", dynamic.string("{value}%")),
                #("color", dynamic.string("#111111")),
                #("fontSize", dynamic.float(22.0)),
              ]),
            ),
            #("title", obj([#("color", dynamic.string("#555555")), #("fontSize", dynamic.float(12.0))])),
            #(
              "data",
              dynamic.array([
                obj([#("value", dynamic.float(84.0)), #("name", dynamic.string("Score"))]),
              ]),
            ),
          ]),
        ]),
      )
      e_charts_type.set_option(gauge, g_opt, None, None)

      case document.get_element_by_id(doc, "meta") {
        None -> Nil
        Some(meta) -> {
          let w1 = int.to_string(float.round(e_charts_type.get_width(radar)))
          let h1 = int.to_string(float.round(e_charts_type.get_height(radar)))
          let w2 = int.to_string(float.round(e_charts_type.get_width(gauge)))
          let h2 = int.to_string(float.round(e_charts_type.get_height(gauge)))
          node.set_text_content(
            meta,
            Some("Radar: " <> w1 <> "x" <> h1 <> " · Gauge: " <> w2 <> "x" <> h2 <> " · Score 84%"),
          )
        }
      }
      Nil
    }
    _, _ -> Nil
  }
}
