import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node as dom_node
import gleam/int
import gleam/list
import gleam/option.{None, Some}
import pixi_js/areas/main/application
import pixi_js/areas/main/application_init_options
import pixi_js/areas/main/container
import pixi_js/areas/main/graphics
import pixi_js/areas/main/observable_point
import pixi_js/areas/main/sprite
import pixi_js/areas/main/texture
import pixi_js/runtime
import pixi_js/types as t

fn range(from: Int, to: Int) -> List(Int) {
  case from > to {
    True -> []
    False -> [from, ..range(from + 1, to)]
  }
}

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
  let opts = application_init_options.create()
  application_init_options.set_width(opts, Some(680.0))
  application_init_options.set_height(opts, Some(400.0))
  application_init_options.set_background_color(
    opts,
    Some(runtime.dynamic(int.to_float(0xffffff))),
  )
  application_init_options.set_preference(
    opts,
    Some(t.StringOrRendererPreferenceArrayString("webgl")),
  )
  application_init_options.set_preserve_drawing_buffer(opts, Some(True))
  application_init_options.set_antialias(opts, Some(True))
  application_init_options.set_auto_start(opts, Some(False))

  runtime.promise_then(application.init(app, Some(opts)), fn(ready) {
    let assert Ok(_) = ready
    let doc = core.get_document()
    case document.get_element_by_id(doc, "stage") {
      Some(stage_el) -> {
        let _ = dom_node.append_child(stage_el, application.get_canvas(app))
        Nil
      }
      None -> Nil
    }

    let stage = application.get_stage(app)

    let frame = graphics.new(None)
    let _ = graphics.rect(frame, 32.0, 20.0, 616.0, 360.0)
    let _ =
      graphics.stroke(frame, [
        runtime.js_object([
          #("width", runtime.dynamic(1.5)),
          #("color", runtime.dynamic(int.to_float(0xd4d4d4))),
        ]),
      ])
    let _ = container.add_child(stage, [frame], runtime.codec_list(t.graphics_codec()))

    let white = texture.get_white()
    list.each(range(0, 5), fn(row) {
      list.each(range(0, 9), fn(col) {
        let s = sprite.from_2(runtime.dynamic(white), None)
        let idx = { row * 3 + col * 5 } % 8
        let c_f = int.to_float(col)
        let r_f = int.to_float(row)
        let alpha_mod = int.to_float({ col + row } % 3)
        container.set_width(s, 52.0)
        container.set_height(s, 48.0)
        let _ = observable_point.set(sprite.get_anchor(s), Some(0.5), Some(0.5))
        container.set_x(s, 70.0 +. c_f *. 60.0)
        container.set_y(s, 56.0 +. r_f *. 56.0)
        container.set_rotation(s, { c_f -. 4.5 } *. 0.04 +. { r_f -. 2.5 } *. 0.03)
        container.set_alpha(s, 0.65 +. 0.35 *. alpha_mod /. 2.0)
        container.set_tint(s, runtime.dynamic(int.to_float(shade_at(idx))))
        let _ = container.add_child(stage, [s], runtime.codec_list(t.sprite_codec()))
        Nil
      })
    })

    application.render(app)

    case document.get_element_by_id(doc, "meta") {
      Some(meta) ->
        dom_node.set_text_content(
          meta,
          Some("60 Texture.WHITE sprites · 10x6 mosaic · 8 monochrome tints · 680x400 WebGL"),
        )
      None -> Nil
    }
  })
}
