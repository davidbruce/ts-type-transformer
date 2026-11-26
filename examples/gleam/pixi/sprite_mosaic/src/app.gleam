import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node as dom_node
import gleam/int
import gleam/option.{None, Some}
import pixi_js/application
import pixi_js/application_init_options
import pixi_js/container
import pixi_js/graphics
import pixi_js/observable_point
import pixi_js/runtime
import pixi_js/sprite
import pixi_js/stroke_style
import pixi_js/texture

fn shade_at(idx: Int) -> Int {
  case idx {
    0 -> 0x111111
    1 -> 0x262626
    2 -> 0x404040
    3 -> 0x555555
    4 -> 0x737373
    5 -> 0xa3a3a3
    6 -> 0xd4d4d4
    _ -> 0xe5e5e5
  }
}

pub fn main() {
  let app = application.new()
  let opts =
    application_init_options.create()
    |> application_init_options.with_width(680.0)
    |> application_init_options.with_height(400.0)
    |> application_init_options.with_background_color(int.to_float(0xffffff))
    |> application_init_options.with_preference("webgl")
    |> application_init_options.with_preserve_drawing_buffer(True)
    |> application_init_options.with_antialias(True)
    |> application_init_options.with_auto_start(False)

  use ready <- runtime.promise_then(application.init_with(app, opts))
  let assert Ok(_) = ready
  let doc = core.get_document()
  case document.get_element_by_id(doc, "stage") {
    Some(stage_el) -> {
      dom_node.append_child(stage_el, application.get_canvas(app))
      Nil
    }
    None -> Nil
  }

  let stage = application.get_stage(app)

  let frame =
    graphics.new()
    |> graphics.rect(32.0, 20.0, 616.0, 360.0)
    |> graphics.stroke_style_with(
      stroke_style.create()
      |> stroke_style.with_width(1.5)
      |> stroke_style.with_color(int.to_float(0xd4d4d4)),
    )
  container.add_child(stage, [frame])

  let white = texture.get_white()
  int.range(from: 0, to: 6, with: Nil, run: fn(_, row) {
    int.range(from: 0, to: 10, with: Nil, run: fn(_, col) {
      let s = sprite.from_texture(white)
      let idx = { row * 3 + col * 5 } % 8
      let c_f = int.to_float(col)
      let r_f = int.to_float(row)
      let alpha_mod = int.to_float({ col + row } % 3)
      container.set_width(s, 52.0)
      container.set_height(s, 48.0)
      sprite.get_anchor(s)
      |> observable_point.set_with(0.5, 0.5)
      container.set_x(s, 70.0 +. c_f *. 60.0)
      container.set_y(s, 56.0 +. r_f *. 56.0)
      container.set_rotation(
        s,
        { c_f -. 4.5 } *. 0.04 +. { r_f -. 2.5 } *. 0.03,
      )
      container.set_alpha(s, 0.65 +. 0.35 *. alpha_mod /. 2.0)
      container.set_tint(s, int.to_float(shade_at(idx)))
      container.add_child(stage, [s])
      Nil
    })
  })

  application.render(app)

  case document.get_element_by_id(doc, "meta") {
    Some(meta) ->
      dom_node.set_text_content(
        meta,
        Some(
          "60 Texture.WHITE sprites · 10x6 mosaic · 8 monochrome tints · 680x400 WebGL",
        ),
      )
    None -> Nil
  }
}
