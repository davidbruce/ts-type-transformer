// ECharts from Gleam, in a page: a chart on the page's element, a bar series set
// as an option and read back, its size and pixel conversions, events, resizing,
// disposal. Every check must hold.
import bindings/areas/echarts
import bindings/areas/echarts/bar_series_option
import bindings/areas/echarts/e_charts_core_option as raw
import bindings/areas/echarts/e_charts_type
import bindings/areas/echarts/ec_unit_option as unit
import bindings/areas/echarts/eventful
import bindings/areas/echarts/model_finder_object
import bindings/areas/echarts/payload
import bindings/areas/echarts/resize_opts
import bindings/areas/test_page
import bindings/runtime
import bindings/types as t
import cell
import gleam/dynamic
import gleam/dynamic/decode
import gleam/float
import gleam/int
import gleam/io
import gleam/list
import gleam/option.{None, Some}
import gleam/string

fn check(ok: Bool, what: String) -> Int {
  case ok {
    True -> 1
    False -> panic as what
  }
}

// A plain JavaScript object from keys and values.
fn object(entries: List(#(String, dynamic.Dynamic))) -> dynamic.Dynamic {
  runtime.js_object(entries)
}

fn array(items: List(dynamic.Dynamic)) -> dynamic.Dynamic {
  dynamic.array(items)
}

pub fn main() {
  // A chart on the page's #chart.
  let chart = echarts.init_with(Some(test_page.element("chart")), None, None)
  let passed =
    check(!e_charts_type.is_disposed(chart), "not disposed")
    + check(
      e_charts_type.get_width(chart) == 400.0
        && e_charts_type.get_height(chart) == 300.0,
      "size of the element",
    )
    + check(string.length(e_charts_type.get_id(chart)) > 0, "an id")
    + check(
      option.is_some(echarts.get_instance_by_dom(test_page.element("chart"))),
      "found by its element",
    )
    + check(
      option.is_some(echarts.get_instance_by_id(e_charts_type.get_id(chart))),
      "found by its id",
    )

  // A bar series with a category axis and a value axis.
  let series = bar_series_option.create()
  bar_series_option.set_name(series, "sales")
  bar_series_option.set_data(
    series,
    Some(list.map([5.0, 20.0, 36.0, 10.0], dynamic.float)),
  )
  let option_object = raw.create()
  unit.set_animation(option_object, Some(False))
  unit.set_at(option_object, "series", array([runtime.dynamic(series)]))
  unit.set_at(
    option_object,
    "xAxis",
    object([
      #("type", dynamic.string("category")),
      #("data", array(list.map(["A", "B", "C", "D"], dynamic.string))),
    ]),
  )
  unit.set_at(
    option_object,
    "yAxis",
    object([#("type", dynamic.string("value"))]),
  )
  unit.set_at(
    option_object,
    "title",
    object([#("text", dynamic.string("Sales"))]),
  )
  e_charts_type.set_option(chart, option_object)

  // Read back.
  let read = e_charts_type.get_option(chart)
  let first_series = fn() {
    let assert Ok([s]) =
      decode.run(unit.get_at(read, "series"), decode.list(decode.dynamic))
    s
  }
  let assert Ok(data) =
    decode.run(first_series(), decode.at(["data"], decode.list(decode.float)))
  let assert Ok(kind) =
    decode.run(first_series(), decode.at(["type"], decode.string))
  let assert Ok(name) =
    decode.run(first_series(), decode.at(["name"], decode.string))
  let assert Ok([categories]) =
    decode.run(
      unit.get_at(read, "xAxis"),
      decode.list(decode.at(["data"], decode.list(decode.string))),
    )
  let assert Ok([title]) =
    decode.run(
      unit.get_at(read, "title"),
      decode.list(decode.at(["text"], decode.string)),
    )
  let passed =
    passed
    + check(kind == "bar" && name == "sales", "the series reads back")
    + check(data == [5.0, 20.0, 36.0, 10.0], "its data")
    + check(categories == ["A", "B", "C", "D"], "the categories")
    + check(title == "Sales", "the title")
    + check(unit.get_animation(read) == Some(False), "animation is off")

  // Pixel conversions: along the x axis in order, and the taller bar higher up.
  // A finder object with a category, neither its union's first member: `_of`.
  let on_x = model_finder_object.create()
  model_finder_object.set_x_axis_index(on_x, runtime.dynamic(0.0))
  let x_of = fn(c) {
    e_charts_type.convert_to_pixel_of(
      chart,
      t.ModelFinderModelFinderObject(on_x),
      t.ScaleDataValueString(c),
    )
  }
  let on_y = model_finder_object.create()
  model_finder_object.set_y_axis_index(on_y, runtime.dynamic(0.0))
  let y_of = fn(v) {
    e_charts_type.convert_to_pixel_model_finder_object(chart, on_y, v)
  }
  let passed =
    passed
    + check(
      x_of("A") <. x_of("B") && x_of("B") <. x_of("C") && x_of("C") <. x_of("D"),
      "categories go left to right",
    )
    + check(x_of("A") >. 0.0 && x_of("D") <. 400.0, "inside the element")
    + check(y_of(36.0) <. y_of(5.0), "a bigger value is higher (smaller y)")
    + check(y_of(0.0) <. 300.0 && y_of(0.0) >. 0.0, "zero is inside")
    + check(
      float.absolute_value(
        { x_of("B") -. x_of("A") } -. { x_of("C") -. x_of("B") },
      )
        <. 0.001,
      "categories are evenly spaced",
    )

  // An event, fired by an action: highlight, on the series.
  let seen = cell.new(0)
  let _ =
    eventful.on(
      e_charts_type.upcast(chart) |> chart_as_eventful,
      runtime.dynamic("highlight"),
      fn(_) {
        cell.set(seen, cell.get(seen) + 1)
        None
      },
    )
  let action = payload_of("highlight")
  e_charts_type.dispatch_action(chart, action)
  let passed =
    passed
    + check(cell.get(seen) >= 1, "the highlight event reached the listener")

  // A data URL of the rendered chart.
  let url = e_charts_type.get_data_url(chart)
  let passed =
    passed
    + check(string.starts_with(url, "data:image/png;base64,"), "get_data_url")

  // Resizing.
  let size = resize_opts.create()
  resize_opts.set_width(size, 200.0)
  resize_opts.set_height(size, 150.0)
  e_charts_type.resize_with(chart, Some(size))
  let passed =
    passed
    + check(
      e_charts_type.get_width(chart) == 200.0
        && e_charts_type.get_height(chart) == 150.0,
      "resized",
    )

  // Cleared, then disposed.
  e_charts_type.clear(chart)
  let after_clear = e_charts_type.get_option(chart)
  let passed =
    passed
    + check(
      decode.run(
        unit.get_at(after_clear, "series"),
        decode.list(decode.dynamic),
      )
        == Ok([]),
      "clear removes the series",
    )
  e_charts_type.dispose(chart)
  let passed =
    passed
    + check(e_charts_type.is_disposed(chart), "disposed")
    + check(
      option.is_none(echarts.get_instance_by_dom(test_page.element("chart"))),
      "no instance on the element any more",
    )

  io.println(int.to_string(passed) <> " checks passed")
}

fn chart_as_eventful(c) {
  c
}

fn payload_of(kind: String) {
  let p = payload.create()
  payload.set_type(p, kind)
  p
}
