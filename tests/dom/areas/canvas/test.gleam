// A canvas's 2D context from Gleam: rectangles, paths, transforms, state,
// pixels read and written, text, gradients and the data URL.
import bindings/areas/canvas/canvas_compositing
import bindings/areas/canvas/canvas_rendering_context2d as ctx2d
import bindings/areas/canvas/image_data
import bindings/areas/canvas/text_metrics
import bindings/areas/core as dom
import bindings/areas/core/document
import bindings/areas/core/node
import bindings/areas/canvas/canvas_gradient
import bindings/areas/html/html_canvas_element as canvas_el
import bindings/runtime
import bindings/types as t
import cell
import gleam/dynamic/decode
import gleam/float
import gleam/int
import gleam/io
import gleam/list
import gleam/option.{None, Some}
import gleam/string

fn style(s: String) {
  t.StringOrCanvasGradientOrCanvasPatternString(s)
}

// The pixel at (x, y) as [r, g, b, a]. FINDING 2 (awkward): ImageData.data is an opaque
// Uint8ClampedArray with no accessors, so it is read through Dynamic.
fn pixel(ctx, x: Int, y: Int) -> List(Int) {
  let data =
    runtime.dynamic(
      image_data.get_data(ctx2d.get_image_data(
        ctx,
        int.to_float(x),
        int.to_float(y),
        1.0,
        1.0,
        None,
      )),
    )
  list.map([0, 1, 2, 3], fn(i) {
    let assert Ok(v) = decode.run(data, decode.at([i], decode.int))
    v
  })
}

pub fn main() {
  let count = cell.new(0)
  let check = fn(ok: Bool, what: String) {
    case ok {
      True -> cell.set(count, cell.get(count) + 1)
      False -> panic as what
    }
  }
  let doc = dom.get_document()
  let body = document.get_body(doc)

  let canvas = document.create_element_canvas(doc, None)
  canvas_el.set_width(canvas, 100.0)
  canvas_el.set_height(canvas, 60.0)
  node.append_child(body, canvas)
  check(canvas_el.get_width(canvas) == 100.0 && canvas_el.get_height(canvas) == 60.0, "canvas size")
  let assert Some(ctx) = canvas_el.get_context_2d(canvas, None)
  check(canvas_el.get_width(ctx2d.get_canvas(ctx)) == 100.0, "context knows its canvas")
  check(pixel(ctx, 5, 5) == [0, 0, 0, 0], "a new canvas is transparent")

  // Filled and stroked rectangles.
  ctx2d.set_fill_style(ctx, style("#ff0000"))
  ctx2d.fill_rect(ctx, 0.0, 0.0, 10.0, 10.0)
  check(pixel(ctx, 5, 5) == [255, 0, 0, 255], "fillRect red")
  check(pixel(ctx, 15, 5) == [0, 0, 0, 0], "outside the rectangle")
  check(ctx2d.get_fill_style(ctx) == style("#ff0000"), "fillStyle reads back")
  ctx2d.set_fill_style(ctx, style("rgb(0, 128, 255)"))
  check(ctx2d.get_fill_style(ctx) == style("#0080ff"), "fillStyle is normalised")
  ctx2d.fill_rect(ctx, 20.0, 0.0, 10.0, 10.0)
  check(pixel(ctx, 25, 5) == [0, 128, 255, 255], "fillRect blue")
  ctx2d.set_stroke_style(ctx, style("#00ff00"))
  ctx2d.set_line_width(ctx, 2.0)
  ctx2d.stroke_rect(ctx, 41.0, 11.0, 18.0, 18.0)
  check(pixel(ctx, 41, 20) == [0, 255, 0, 255], "strokeRect edge")
  check(pixel(ctx, 50, 20) == [0, 0, 0, 0], "strokeRect leaves the inside")
  check(ctx2d.get_line_width(ctx) == 2.0, "lineWidth")
  ctx2d.clear_rect(ctx, 0.0, 0.0, 10.0, 10.0)
  check(pixel(ctx, 5, 5) == [0, 0, 0, 0], "clearRect")

  // Paths.
  ctx2d.set_fill_style(ctx, style("#ffff00"))
  ctx2d.begin_path(ctx)
  ctx2d.arc(ctx, 80.0, 30.0, 10.0, 0.0, 6.283185307179586, None)
  ctx2d.fill(ctx, None)
  check(pixel(ctx, 80, 30) == [255, 255, 0, 255], "arc filled")
  check(pixel(ctx, 80, 45) == [0, 0, 0, 0], "outside the arc")
  check(ctx2d.is_point_in_path(ctx, 80.0, 30.0, None), "point in path")
  check(!ctx2d.is_point_in_path(ctx, 0.0, 0.0, None), "point not in path")
  ctx2d.begin_path(ctx)
  ctx2d.move_to(ctx, 0.0, 50.0)
  ctx2d.line_to(ctx, 40.0, 50.0)
  ctx2d.set_stroke_style(ctx, style("#ff00ff"))
  ctx2d.set_line_width(ctx, 4.0)
  ctx2d.stroke(ctx)
  check(pixel(ctx, 20, 50) == [255, 0, 255, 255], "lineTo stroked")
  check(ctx2d.is_point_in_stroke(ctx, 20.0, 50.0), "point in stroke")
  check(!ctx2d.is_point_in_stroke(ctx, 20.0, 40.0), "point not in stroke")
  ctx2d.begin_path(ctx)
  ctx2d.rect(ctx, 0.0, 12.0, 8.0, 8.0)
  ctx2d.set_fill_style(ctx, style("#ffffff"))
  ctx2d.fill(ctx, None)
  check(pixel(ctx, 4, 16) == [255, 255, 255, 255], "rect path filled")
  ctx2d.begin_path(ctx)
  ctx2d.move_to(ctx, 60.0, 40.0)
  ctx2d.line_to(ctx, 70.0, 40.0)
  ctx2d.line_to(ctx, 70.0, 55.0)
  ctx2d.close_path(ctx)
  check(ctx2d.is_point_in_path(ctx, 68.0, 45.0, None), "closed triangle contains a point")

  // State: save and restore.
  ctx2d.set_fill_style(ctx, style("#112233"))
  ctx2d.save(ctx)
  ctx2d.set_fill_style(ctx, style("#445566"))
  ctx2d.set_line_width(ctx, 9.0)
  check(ctx2d.get_fill_style(ctx) == style("#445566"), "changed inside save")
  ctx2d.restore(ctx)
  check(ctx2d.get_fill_style(ctx) == style("#112233"), "fillStyle restored")
  check(ctx2d.get_line_width(ctx) == 4.0, "lineWidth restored")

  // Transforms.
  ctx2d.clear_rect(ctx, 0.0, 0.0, 100.0, 60.0)
  ctx2d.set_fill_style(ctx, style("#ff0000"))
  ctx2d.save(ctx)
  ctx2d.translate(ctx, 30.0, 20.0)
  ctx2d.fill_rect(ctx, 0.0, 0.0, 6.0, 6.0)
  check(pixel(ctx, 32, 22) == [255, 0, 0, 255], "translate")
  check(pixel(ctx, 2, 2) == [0, 0, 0, 0], "nothing at the origin")
  ctx2d.scale(ctx, 2.0, 2.0)
  ctx2d.fill_rect(ctx, 5.0, 0.0, 3.0, 3.0)
  check(pixel(ctx, 41, 22) == [255, 0, 0, 255], "scale after translate")
  ctx2d.restore(ctx)
  ctx2d.set_transform(ctx, 1.0, 0.0, 0.0, 1.0, 70.0, 40.0)
  ctx2d.fill_rect(ctx, 0.0, 0.0, 4.0, 4.0)
  check(pixel(ctx, 71, 41) == [255, 0, 0, 255], "setTransform")
  ctx2d.reset_transform(ctx)
  ctx2d.rotate(ctx, 3.141592653589793)
  ctx2d.fill_rect(ctx, 0.0, 0.0, -4.0, -4.0)
  check(pixel(ctx, 2, 2) == [255, 0, 0, 255], "rotate by pi mirrors")
  ctx2d.reset_transform(ctx)

  // Alpha blends.
  ctx2d.clear_rect(ctx, 0.0, 0.0, 100.0, 60.0)
  canvas_compositing.set_global_alpha(ctx, 0.5)
  check(canvas_compositing.get_global_alpha(ctx) == 0.5, "globalAlpha")
  ctx2d.set_fill_style(ctx, style("#ffffff"))
  ctx2d.fill_rect(ctx, 0.0, 0.0, 4.0, 4.0)
  let assert [255, 255, 255, a] = pixel(ctx, 1, 1)
  check(a >= 127 && a <= 129, "half alpha: " <> int.to_string(a))
  canvas_compositing.set_global_alpha(ctx, 1.0)

  // A gradient.
  let gradient = ctx2d.create_linear_gradient(ctx, 0.0, 0.0, 100.0, 0.0)
  canvas_gradient.add_color_stop(gradient, 0.0, "#ff0000")
  canvas_gradient.add_color_stop(gradient, 1.0, "#0000ff")
  ctx2d.set_fill_style(ctx, t.StringOrCanvasGradientOrCanvasPatternCanvasGradient(gradient))
  ctx2d.fill_rect(ctx, 0.0, 30.0, 100.0, 10.0)
  let assert [r0, _, b0, 255] = pixel(ctx, 0, 35)
  let assert [r1, _, b1, 255] = pixel(ctx, 99, 35)
  check(r0 > 240 && b0 < 15 && r1 < 15 && b1 > 240, "gradient ends")

  // Pixels moved with getImageData and putImageData.
  ctx2d.clear_rect(ctx, 0.0, 0.0, 100.0, 60.0)
  ctx2d.set_fill_style(ctx, style("#0000ff"))
  ctx2d.fill_rect(ctx, 0.0, 0.0, 8.0, 8.0)
  let copy = ctx2d.get_image_data(ctx, 0.0, 0.0, 8.0, 8.0, None)
  check(image_data.get_width(copy) == 8.0 && image_data.get_height(copy) == 8.0, "imageData size")
  ctx2d.put_image_data(ctx, copy, 50.0, 40.0)
  check(pixel(ctx, 53, 43) == [0, 0, 255, 255], "putImageData copies pixels")
  check(pixel(ctx, 49, 43) == [0, 0, 0, 0], "putImageData is placed")
  let blank = ctx2d.create_image_data(ctx, 3.0, 2.0, None)
  check(image_data.get_width(blank) == 3.0 && image_data.get_height(blank) == 2.0, "createImageData size")
  ctx2d.put_image_data(ctx, blank, 50.0, 40.0)
  check(pixel(ctx, 51, 41) == [0, 0, 0, 0], "a blank ImageData erases")
  check(image_data.new(4.0, 5.0, None) |> image_data.get_height == 5.0, "new ImageData")

  // FINDING 3 (can't express): ImageData::new_with_data needs a Uint8ClampedArray,
  // which has no constructor or setters, so pixels can only be copied, not made.

  // Text.
  ctx2d.set_font(ctx, "20px monospace")
  check(ctx2d.get_font(ctx) == "20px monospace", "font")
  let short = ctx2d.measure_text(ctx, "ab")
  let long = ctx2d.measure_text(ctx, "abcdefgh")
  check(text_metrics.get_width(short) >. 0.0, "text has width")
  check(float.absolute_value(text_metrics.get_width(long) -. 4.0 *. text_metrics.get_width(short)) <. 0.01, "monospace widths scale")
  ctx2d.clear_rect(ctx, 0.0, 0.0, 100.0, 60.0)
  ctx2d.set_fill_style(ctx, style("#000000"))
  ctx2d.fill_text(ctx, "MMMM", 2.0, 30.0, None)
  let inked =
    list.index_map(list.repeat(0, 41), fn(_, i) { i })
    |> list.filter(fn(x) {
      let assert [_, _, _, a] = pixel(ctx, x, 22)
      a > 0
    })
  check(inked != [], "fillText draws")

  // Line dash, and the context's own settings.
  ctx2d.set_line_dash(ctx, [4.0, 2.0])
  check(ctx2d.get_line_dash(ctx) == [4.0, 2.0], "line dash")
  ctx2d.reset(ctx)
  check(ctx2d.get_line_width(ctx) == 1.0, "reset restores lineWidth")
  check(pixel(ctx, 5, 22) == [0, 0, 0, 0], "reset clears")

  // The data URL.
  check(string.starts_with(canvas_el.to_data_url(canvas, None, None), "data:image/png;base64,"), "png data url")
  check(string.starts_with(canvas_el.to_data_url(canvas, Some("image/jpeg"), Some(0.8)), "data:image/jpeg;base64,"), "jpeg data url")

  // Resizing clears the canvas and its state.
  ctx2d.set_fill_style(ctx, style("#ff0000"))
  ctx2d.fill_rect(ctx, 0.0, 0.0, 100.0, 60.0)
  canvas_el.set_width(canvas, 50.0)
  check(canvas_el.get_width(canvas) == 50.0, "resized")
  check(pixel(ctx, 5, 5) == [0, 0, 0, 0], "resizing clears")

  io.println(int.to_string(cell.get(count)) <> " checks passed")
}
