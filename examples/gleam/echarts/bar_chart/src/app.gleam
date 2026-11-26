import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node
import echarts as ec
import echarts/axis_label_option
import echarts/axis_line_option
import echarts/bar_item_style_option
import echarts/bar_series_option
import echarts/category_axis_base_option_data as axis_data
import echarts/echarts_init_opts
import echarts/echarts_option
import echarts/echarts_type
import echarts/grid_component_option
import echarts/line_style_option
import echarts/split_line_option
import echarts/title_component_option
import echarts/title_text_style_option
import echarts/x_axis_component_option_category as category_x_axis_option
import echarts/y_axis_component_option_value as value_y_axis_option
import gleam/dynamic
import gleam/float
import gleam/int
import gleam/list
import gleam/option.{None, Some}

pub fn main() -> Nil {
  let doc = core.get_document()
  case document.get_element_by_id(doc, "chart") {
    None -> Nil
    Some(box) -> {
      let chart = ec.init_with(Some(box), dynamic.nil(), echarts_init_opts.create())
      let title =
        title_component_option.create()
        |> title_component_option.with_text("Monthly Output")
        |> title_component_option.with_left(16.0)
        |> title_component_option.with_top(12.0)
        |> title_component_option.with_text_style(
          title_text_style_option.create()
          |> title_text_style_option.with_color(dynamic.string("#111111"))
          |> title_text_style_option.with_font_size(13.0),
        )
      let grid =
        grid_component_option.create()
        |> grid_component_option.with_left(48.0)
        |> grid_component_option.with_right(24.0)
        |> grid_component_option.with_top(48.0)
        |> grid_component_option.with_bottom(36.0)
      let x_axis =
        category_x_axis_option.create()
        |> category_x_axis_option.with_data(list.map(
          ["Jan", "Feb", "Mar", "Apr", "May", "Jun"],
          axis_data.String,
        ))
        |> category_x_axis_option.with_axis_line(
          axis_line_option.create()
          |> axis_line_option.with_line_style(
            line_style_option.create()
            |> line_style_option.with_color("#111111"),
          ),
        )
        |> category_x_axis_option.with_axis_label(
          axis_label_option.create()
          |> axis_label_option.with_color(dynamic.string("#555555")),
        )
      let y_axis =
        value_y_axis_option.create()
        |> value_y_axis_option.with_min(0.0)
        |> value_y_axis_option.with_max(100.0)
        |> value_y_axis_option.with_split_line(
          split_line_option.create()
          |> split_line_option.with_line_style(
            line_style_option.create()
            |> line_style_option.with_color("#e5e5e5"),
          ),
        )
        |> value_y_axis_option.with_axis_label(
          axis_label_option.create()
          |> axis_label_option.with_color(dynamic.string("#555555")),
        )
      let bars =
        bar_series_option.create()
        |> bar_series_option.with_name("Units")
        |> bar_series_option.with_bar_width_string("46%")
        |> bar_series_option.with_data(list.map(
          [42.0, 68.0, 55.0, 84.0, 71.0, 93.0],
          dynamic.float,
        ))
        |> bar_series_option.with_item_style(
          bar_item_style_option.create()
          |> bar_item_style_option.with_color("#111111"),
        )
      let opt =
        echarts_option.create()
        |> echarts_option.with_animation(False)
        |> echarts_option.with_background_color("#ffffff")
        |> echarts_option.with_title(title)
        |> echarts_option.with_grid(grid)
        |> echarts_option.with_x_axis_category(x_axis)
        |> echarts_option.with_y_axis_value(y_axis)
        |> echarts_option.with_series_bar(bars)
      echarts_type.set_option(chart, opt)

      case document.get_element_by_id(doc, "meta") {
        None -> Nil
        Some(meta) -> {
          let w = int.to_string(float.round(echarts_type.get_width(chart)))
          let h = int.to_string(float.round(echarts_type.get_height(chart)))
          node.set_text_content(
            meta,
            Some(
              "Canvas: " <> w <> "x" <> h <> " · 6 bars · Monochrome #111111",
            ),
          )
        }
      }
      Nil
    }
  }
}
