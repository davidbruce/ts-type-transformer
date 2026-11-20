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

fn slice(name: String, value: Float) -> Dynamic {
  obj([#("name", dynamic.string(name)), #("value", dynamic.float(value))])
}

pub fn main() -> Nil {
  let doc = core.get_document()
  case document.get_element_by_id(doc, "chart") {
    None -> Nil
    Some(box) -> {
      let chart = ec.init(Some(box), None, None)
      let opt =
        raw.create()
        |> unit.with_animation(False)
        |> unit.with_at("backgroundColor", dynamic.string("#ffffff"))
        |> unit.with_at(
          "color",
          dynamic.array(list.map(["#111111", "#404040", "#737373", "#a3a3a3", "#d4d4d4"], dynamic.string)),
        )
        |> unit.with_at(
          "legend",
          obj([
            #("bottom", dynamic.float(8.0)),
            #("textStyle", obj([#("color", dynamic.string("#111111"))])),
          ]),
        )
        |> unit.with_at("tooltip", obj([#("trigger", dynamic.string("item"))]))
        |> unit.with_at(
          "series",
          dynamic.array([
            obj([
              #("type", dynamic.string("pie")),
              #("name", dynamic.string("Subsystems")),
              #("radius", dynamic.array([dynamic.string("44%"), dynamic.string("72%")])),
              #("center", dynamic.array([dynamic.string("50%"), dynamic.string("46%")])),
              #(
                "itemStyle",
                obj([
                  #("borderColor", dynamic.string("#ffffff")),
                  #("borderWidth", dynamic.float(2.0)),
                ]),
              ),
              #("label", obj([#("color", dynamic.string("#111111"))])),
              #(
                "data",
                dynamic.array([
                  slice("Runtime", 34.0),
                  slice("Bindings", 26.0),
                  slice("Glue", 18.0),
                  slice("Parser", 14.0),
                  slice("Tests", 8.0),
                ]),
              ),
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
            Some("Canvas: " <> w <> "x" <> h <> " · 5 monochrome segments · Total 100"),
          )
        }
      }
      Nil
    }
  }
}
