import gleam/float
import gleam/int
import gleam/list
import gleam/option.{None, Some}
import pts/canvas_form
import pts/canvas_space
import pts/canvas_space_options
import pts/circle
import pts/group
import pts/i_player
import pts/line
import pts/pt
import pts/types as t

@external(javascript, "./app_ffi.mjs", "sin")
fn sin(x: Float) -> Float

@external(javascript, "./app_ffi.mjs", "cos")
fn cos(x: Float) -> Float

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

  let player =
    i_player.create()
    |> i_player.with_animate(fn(time_ms, _ftime, _space) {
      let c = canvas_space.get_center(cs)
      let size = canvas_space.get_size(cs)
      let pointer = canvas_space.get_pointer(cs)
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
      canvas_form.stroke_only_with(form, "#999999", 1.2, None, None)
      |> canvas_form.line_group(chord)

      let probe_center = case
        pt.get_x(pointer) >. 1.0 || pt.get_y(pointer) >. 1.0
      {
        True -> pointer
        False -> c
      }
      let probe = circle.from_center_pt(probe_center, min_dim *. 0.2)

      let targets =
        list.map([0, 1, 2, 3, 4], fn(i) {
          let fi = int.to_float(i)
          let orbit_r = min_dim *. { 0.14 +. 0.04 *. fi }
          let theta = time_ms *. { 0.00045 +. 0.00012 *. fi } +. fi *. 1.256
          let tc =
            mk_pt(cx +. cos(theta) *. orbit_r, cy +. sin(theta) *. orbit_r)
          let tr = min_dim *. { 0.11 +. 0.025 *. int.to_float(i % 3) }
          let target = circle.from_center_pt(tc, tr)

          canvas_form.fill(form, "#f7f7f7")
          |> canvas_form.stroke_with("#737373", 1.4, None, None)
          |> canvas_form.circle_group(target)

          case
            line.perpendicular_from_pt_of_with(
              t.PtIterableGroup(chord),
              t.PtLikePt(tc),
              False,
            )
          {
            Some(foot) -> {
              let stem = group.new([tc, foot])
              canvas_form.stroke_only_with(form, "#b5b5b5", 1.0, None, None)
              |> canvas_form.line_group(stem)
              |> canvas_form.fill_only("#555555")
              |> canvas_form.point_pt_with(foot, 2.5, "circle")
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
      canvas_form.stroke_only_with(form, "#111111", 2.0, None, None)
      |> canvas_form.circle_group(probe)
      |> canvas_form.stroke_only_with("#111111", 1.4, None, None)
      |> canvas_form.line_group(px1)
      |> canvas_form.line_group(py1)

      list.each(targets, fn(target) {
        let hits =
          circle.intersect_circle2d_of(
            t.PtIterableGroup(probe),
            t.PtIterableGroup(target),
          )
        let hit_list = group.to_list(hits)
        case list.length(hit_list) == 2 {
          True -> {
            canvas_form.stroke_only_with(form, "#111111", 1.4, None, None)
            |> canvas_form.line_group(hits)
            Nil
          }
          False -> Nil
        }
        list.each(hit_list, fn(h) {
          canvas_form.fill(form, "#ffffff")
          |> canvas_form.stroke_with("#111111", 2.0, None, None)
          |> canvas_form.point_pt_with(h, 5.0, "circle")
          Nil
        })
      })
    })

  canvas_space.add_i_player(cs, player)
  canvas_space.bind_mouse(cs)
  canvas_space.bind_touch(cs)
  canvas_space.play(cs)
  Nil
}
