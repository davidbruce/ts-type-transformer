import gleam/float
import gleam/int
import gleam/list
import gleam/option.{None, Some}
import pts/areas/main/canvas_form
import pts/areas/main/canvas_space
import pts/areas/main/canvas_space_options
import pts/areas/main/circle
import pts/areas/main/create
import pts/areas/main/group
import pts/areas/main/i_player
import pts/areas/main/line
import pts/areas/main/multi_touch_space
import pts/areas/main/num
import pts/areas/main/pt
import pts/areas/main/space
import pts/runtime as r
import pts/types as t

@external(javascript, "./app_ffi.mjs", "sin")
fn sin(x: Float) -> Float

pub fn main() -> Nil {
  let opts = canvas_space_options.create()
  canvas_space_options.set_bgcolor(opts, Some("#ffffff"))
  canvas_space_options.set_resize(opts, Some(True))
  canvas_space_options.set_retina(opts, Some(True))

  let cs =
    canvas_space.new(Some(t.StringOrElementString("#pt")), None)
    |> canvas_space.setup(opts)
  let form = canvas_space.get_form(cs)

  let player = i_player.create()
  i_player.set_animate(
    player,
    Some(fn(time, _ftime, _space) {
      let pointer = multi_touch_space.get_pointer(cs)
      let bound = space.get_inner_bound(cs)
      let size = space.get_size(cs)
      let max_dist = float.max(pt.get_x(size), pt.get_y(size)) *. 0.55
      let denom = float.max(1.0, max_dist)

      let grid = create.grid_pts(bound, 18.0, 11.0, None)
      list.each(group.to_list(grid), fn(p) {
        let delta = pt.subtract_new(p, [r.dynamic(pointer)])
        let dist = pt.magnitude(delta)
        let weight = 1.0 -. num.clamp(dist /. denom, 0.0, 1.0)
        let angle =
          pt.angle(delta, None) +. sin(time *. 0.0015 +. dist *. 0.01) *. 0.35
        let len = 8.0 +. 34.0 *. weight
        let light = int.to_string(float.round(65.0 -. 57.0 *. weight))
        let alpha = 0.25 +. 0.75 *. weight

        let _ =
          canvas_form.stroke_only(
            form,
            t.CanvasFormFillCString(
              "hsla(0, 0%, "
              <> light
              <> "%, "
              <> float.to_string(alpha)
              <> ")",
            ),
            Some(1.5 +. 2.5 *. weight),
            None,
            None,
          )
          |> canvas_form.line_2(
            t.PtLikeIterableGroup(line.from_angle(t.PtLikePt(p), angle, len)),
          )
          |> canvas_form.fill_only(
            t.CanvasFormFillCString("hsla(0, 0%, " <> light <> "%, 0.9)"),
          )
          |> canvas_form.circle_2(
            t.PtLikeIterableGroup(circle.from_center(
              t.PtLikePt(p),
              1.5 +. 4.5 *. weight *. weight,
            )),
          )
        Nil
      })

      let ring_radius = 18.0 +. 6.0 *. sin(time *. 0.004)
      let _ =
        canvas_form.stroke_only(
          form,
          t.CanvasFormFillCString("#11111199"),
          Some(2.0),
          None,
          None,
        )
        |> canvas_form.circle_2(
          t.PtLikeIterableGroup(circle.from_center(
            t.PtLikePt(pointer),
            ring_radius,
          )),
        )
      Nil
    }),
  )

  let _ = space.add(cs, r.dynamic(player))
  let _ = multi_touch_space.bind_mouse(cs, Some(True), None)
  let _ = multi_touch_space.bind_touch(cs, Some(True), None, None)
  let _ = space.play(cs, None)
  Nil
}
