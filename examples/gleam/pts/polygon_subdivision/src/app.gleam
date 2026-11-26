import gleam/float
import gleam/list
import gleam/option.{None, Some}
import pts/canvas_form
import pts/canvas_space
import pts/canvas_space_options
import pts/circle
import pts/group
import pts/iplayer
import pts/num
import pts/polygon
import pts/pt
import pts/types as t

@external(javascript, "./app_ffi.mjs", "sin")
fn sin(x: Float) -> Float

fn mk_pt(x: Float, y: Float) -> t.Pt(Nil) {
  let p = pt.make(2.0)
  pt.set_x(p, x)
  pt.set_y(p, y)
  p
}

pub fn main() -> Nil {
  let opts =
    canvas_space_options.create()
    |> canvas_space_options.with_bgcolor("#ffffff")
    |> canvas_space_options.with_resize(True)
    |> canvas_space_options.with_retina(True)

  let cs =
    canvas_space.new_with(Some("#pt"), None)
    |> canvas_space.setup(opts)
  let form = canvas_space.get_form(cs)

  let shades = [
    "#f7f7f7", "#efefef", "#e5e5e5", "#d4d4d4", "#b5b5b5", "#999999", "#737373",
    "#404040", "#262626",
  ]

  let player =
    iplayer.create()
    |> iplayer.with_animate(fn(time_ms, _ftime, _space) {
      let c = canvas_space.get_center(cs)
      let size = canvas_space.get_size(cs)
      let pointer = canvas_space.get_pointer(cs)
      let min_dim = float.min(pt.get_x(size), pt.get_y(size))
      let radius = float.max(80.0, min_dim *. 0.4)

      let px_norm = pt.get_x(pointer) /. float.max(1.0, pt.get_x(size)) -. 0.5
      let t_val =
        num.clamp(
          0.22 +. 0.16 *. sin(time_ms *. 0.0012) +. px_norm *. 0.24,
          0.1,
          0.48,
        )

      let poly0 =
        polygon.from_center_pt(c, radius, 6.0)
        |> group.rotate2d_pt_with(time_ms *. 0.00035, c, "")
      let centroid = polygon.centroid_group(poly0)

      list.index_fold(shades, poly0, fn(poly, shade, depth) {
        let sw = case depth == 0 {
          True -> 1.6
          False -> 1.0
        }
        canvas_form.fill(form, shade)
        |> canvas_form.stroke_with("#111111", sw, None, None)
        |> canvas_form.polygon_group(poly)

        case depth == 0 {
          True ->
            list.each(group.to_list(poly), fn(p) {
              let ray = group.new([centroid, p])
              canvas_form.stroke_only_with(form, "#d4d4d4", 1.0, None, None)
              |> canvas_form.line_group(ray)
              |> canvas_form.fill_only("#111111")
              |> canvas_form.point_pt_with(p, 3.5, "circle")
              Nil
            })
          False -> Nil
        }

        polygon.midpoints_group_with(poly, True, t_val)
      })

      let cx = pt.get_x(c)
      let cy = pt.get_y(c)
      let h_bar = group.new([mk_pt(cx -. 10.0, cy), mk_pt(cx +. 10.0, cy)])
      let v_bar = group.new([mk_pt(cx, cy -. 10.0), mk_pt(cx, cy +. 10.0)])
      canvas_form.stroke_only_with(form, "#ffffff", 1.5, None, None)
      |> canvas_form.line_group(h_bar)
      |> canvas_form.line_group(v_bar)
      |> canvas_form.fill("#ffffff")
      |> canvas_form.stroke_with("#111111", 1.5, None, None)
      |> canvas_form.point_pt_with(centroid, 5.0, "circle")

      let ring = circle.from_center_pt(pointer, 10.0)
      canvas_form.stroke_only_with(form, "#737373", 1.5, None, None)
      |> canvas_form.circle_group(ring)
      Nil
    })

  canvas_space.add_i_player(cs, player)
  canvas_space.bind_mouse(cs)
  canvas_space.bind_touch(cs)
  canvas_space.play(cs)
  Nil
}
