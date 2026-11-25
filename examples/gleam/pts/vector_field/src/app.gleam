import gleam/float
import gleam/int
import gleam/list
import gleam/option.{None, Some}
import pts/canvas_form
import pts/canvas_space
import pts/canvas_space_options
import pts/circle
import pts/create
import pts/group
import pts/i_player
import pts/line
import pts/num
import pts/pt
import pts/runtime as r

@external(javascript, "./app_ffi.mjs", "sin")
fn sin(x: Float) -> Float

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

  let player =
    i_player.create()
    |> i_player.with_animate(fn(time, _ftime, _space) {
      let pointer = canvas_space.get_pointer(cs)
      let bound = canvas_space.get_inner_bound(cs)
      let size = canvas_space.get_size(cs)
      let max_dist = float.max(pt.get_x(size), pt.get_y(size)) *. 0.55
      let denom = float.max(1.0, max_dist)

      let grid = create.grid_pts(bound, 18.0, 11.0)
      list.each(group.to_list(grid), fn(p) {
        let delta = pt.subtract_new(p, [r.dynamic(pointer)])
        let dist = pt.magnitude(delta)
        let weight = 1.0 -. num.clamp(dist /. denom, 0.0, 1.0)
        let angle =
          pt.angle(delta) +. sin(time *. 0.0015 +. dist *. 0.01) *. 0.35
        let len = 8.0 +. 34.0 *. weight
        let light = int.to_string(float.round(65.0 -. 57.0 *. weight))
        let alpha = 0.25 +. 0.75 *. weight

        canvas_form.stroke_only_with(
          form,
          "hsla(0, 0%, " <> light <> "%, " <> float.to_string(alpha) <> ")",
          1.5 +. 2.5 *. weight,
          None,
          None,
        )
        |> canvas_form.line_group(line.from_angle_pt(p, angle, len))
        |> canvas_form.fill_only("hsla(0, 0%, " <> light <> "%, 0.9)")
        |> canvas_form.circle_group(circle.from_center_pt(
          p,
          1.5 +. 4.5 *. weight *. weight,
        ))
        Nil
      })

      let ring_radius = 18.0 +. 6.0 *. sin(time *. 0.004)
      canvas_form.stroke_only_with(form, "#11111199", 2.0, None, None)
      |> canvas_form.circle_group(circle.from_center_pt(pointer, ring_radius))
      Nil
    })

  canvas_space.add_i_player(cs, player)
  canvas_space.bind_mouse(cs)
  canvas_space.bind_touch(cs)
  canvas_space.play(cs)
  Nil
}
