import gleam/float
import gleam/int
import gleam/list
import gleam/option.{None, Some}
import pts/areas/main/canvas_form
import pts/areas/main/canvas_space
import pts/areas/main/canvas_space_options
import pts/areas/main/circle
import pts/areas/main/group
import pts/areas/main/i_player
import pts/areas/main/line
import pts/areas/main/multi_touch_space
import pts/areas/main/pt
import pts/areas/main/space
import pts/runtime as r
import pts/types as t

@external(javascript, "./app_ffi.mjs", "sin")
fn sin(x: Float) -> Float

@external(javascript, "./app_ffi.mjs", "cos")
fn cos(x: Float) -> Float

fn mk_pt(x: Float, y: Float) -> t.Pt(Nil) {
  let p = pt.make(2.0, None, None)
  pt.set_x(p, x)
  pt.set_y(p, y)
  p
}

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
    Some(fn(time_ms, _ftime, _space) {
      let c = space.get_center(cs)
      let size = space.get_size(cs)
      let pointer = multi_touch_space.get_pointer(cs)
      let min_dim = float.max(120.0, float.min(pt.get_x(size), pt.get_y(size)))
      let cx = pt.get_x(c)
      let cy = pt.get_y(c)

      let ang = time_ms *. 0.00035
      let half_len = min_dim *. 0.44
      let ca = cos(ang)
      let sa = sin(ang)
      let chord =
        group.new([
          mk_pt(cx -. ca *. half_len, cy -. sa *. half_len),
          mk_pt(cx +. ca *. half_len, cy +. sa *. half_len),
        ])
      let _ =
        canvas_form.stroke_only(
          form,
          t.CanvasFormFillCString("#999999"),
          Some(1.2),
          None,
          None,
        )
        |> canvas_form.line_2(t.PtLikeIterableGroup(chord))

      let probe_center = case
        pt.get_x(pointer) >. 1.0 || pt.get_y(pointer) >. 1.0
      {
        True -> pointer
        False -> c
      }
      let probe = circle.from_center(t.PtLikePt(probe_center), min_dim *. 0.20)

      let targets =
        list.map([0, 1, 2, 3, 4], fn(i) {
          let fi = int.to_float(i)
          let orbit_r = min_dim *. { 0.14 +. 0.04 *. fi }
          let theta = time_ms *. { 0.00045 +. 0.00012 *. fi } +. fi *. 1.256
          let tc = mk_pt(cx +. cos(theta) *. orbit_r, cy +. sin(theta) *. orbit_r)
          let tr = min_dim *. { 0.11 +. 0.025 *. int.to_float(i % 3) }
          let target = circle.from_center(t.PtLikePt(tc), tr)

          let _ =
            canvas_form.fill(form, t.CanvasFormFillCString("#f7f7f7"))
            |> canvas_form.stroke(
              t.CanvasFormFillCString("#737373"),
              Some(1.4),
              None,
              None,
            )
            |> canvas_form.circle_2(t.PtLikeIterableGroup(target))

          case line.perpendicular_from_pt(
            t.PtIterableGroup(chord),
            t.PtLikePt(tc),
            Some(False),
          ) {
            Some(foot) -> {
              let stem = group.new([tc, foot])
              let _ =
                canvas_form.stroke_only(
                  form,
                  t.CanvasFormFillCString("#b5b5b5"),
                  Some(1.0),
                  None,
                  None,
                )
                |> canvas_form.line_2(t.PtLikeIterableGroup(stem))
                |> canvas_form.fill_only(t.CanvasFormFillCString("#555555"))
                |> canvas_form.point_2(
                  t.PtLikePt(foot),
                  Some(2.5),
                  Some("circle"),
                )
              Nil
            }
            None -> Nil
          }

          target
        })

      let pcx = pt.get_x(probe_center)
      let pcy = pt.get_y(probe_center)
      let px1 = group.new([mk_pt(pcx -. 6.0, pcy), mk_pt(pcx +. 6.0, pcy)])
      let py1 = group.new([mk_pt(pcx, pcy -. 6.0), mk_pt(pcx, pcy +. 6.0)])
      let _ =
        canvas_form.stroke_only(
          form,
          t.CanvasFormFillCString("#111111"),
          Some(2.0),
          None,
          None,
        )
        |> canvas_form.circle_2(t.PtLikeIterableGroup(probe))
        |> canvas_form.stroke_only(
          t.CanvasFormFillCString("#111111"),
          Some(1.4),
          None,
          None,
        )
        |> canvas_form.line_2(t.PtLikeIterableGroup(px1))
        |> canvas_form.line_2(t.PtLikeIterableGroup(py1))

      list.each(targets, fn(target) {
        let hits =
          circle.intersect_circle2d(
            t.PtIterableGroup(probe),
            t.PtIterableGroup(target),
          )
        let hit_list = group.to_list(hits)
        case list.length(hit_list) == 2 {
          True -> {
            let _ =
              canvas_form.stroke_only(
                form,
                t.CanvasFormFillCString("#111111"),
                Some(1.4),
                None,
                None,
              )
              |> canvas_form.line_2(t.PtLikeIterableGroup(hits))
            Nil
          }
          False -> Nil
        }
        list.each(hit_list, fn(h) {
          let _ =
            canvas_form.fill(form, t.CanvasFormFillCString("#ffffff"))
            |> canvas_form.stroke(
              t.CanvasFormFillCString("#111111"),
              Some(2.0),
              None,
              None,
            )
            |> canvas_form.point_2(t.PtLikePt(h), Some(5.0), Some("circle"))
          Nil
        })
      })
    }),
  )

  let _ = space.add(cs, r.dynamic(player))
  let _ = multi_touch_space.bind_mouse(cs, Some(True), None)
  let _ = multi_touch_space.bind_touch(cs, Some(True), None, None)
  let _ = space.play(cs, None)
  Nil
}
