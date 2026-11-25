import gleam/float
import gleam/int
import gleam/list
import gleam/option.{None, Some}
import pts/canvas_form
import pts/canvas_space
import pts/canvas_space_options
import pts/curve
import pts/group
import pts/i_player
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
      let size = canvas_space.get_size(cs)
      let c = canvas_space.get_center(cs)
      let pointer = canvas_space.get_pointer(cs)
      let w = float.max(120.0, pt.get_x(size))
      let h = float.max(120.0, pt.get_y(size))
      let cy = pt.get_y(c)
      let amp = h *. 0.24
      let count = 11
      let px = pt.get_x(pointer)
      let py = pt.get_y(pointer)

      let base_line = group.new([mk_pt(w *. 0.06, cy), mk_pt(w *. 0.94, cy)])
      canvas_form.stroke_only_with(form, "#e5e5e5", 1.0, None, None)
      |> canvas_form.line_group(base_line)

      let pts_list =
        int.range(from: 0, to: count, with: [], run: fn(acc, i) {
          let fi = int.to_float(i)
          let u = fi /. int.to_float(count - 1)
          let x = w *. { 0.08 +. 0.84 *. u }
          let y0 =
            cy
            +. sin(time_ms *. 0.0014 +. fi *. 0.72)
            *. amp
            +. cos(time_ms *. 0.0009 -. fi *. 0.45)
            *. { amp *. 0.42 }
          let dx = float.absolute_value(x -. px)
          let y = case dx <. w *. 0.25 && { px >. 1.0 || py >. 1.0 } {
            True -> {
              let pull = 1.0 -. dx /. { w *. 0.25 }
              y0 +. { py -. y0 } *. pull *. 0.35
            }
            False -> y0
          }
          [mk_pt(x, y), ..acc]
        })
        |> list.reverse

      let ctrls = group.new(pts_list)
      let bs = curve.bspline_group_with(ctrls, 12.0, 1.0)
      let card = curve.cardinal_group_with(ctrls, 12.0, 0.35)
      let cr = curve.catmull_rom_group_with(ctrls, 12.0)

      list.index_fold(group.to_list(cr), Nil, fn(_, p, i) {
        case i % 3 == 0 {
          True -> {
            let stem = group.new([mk_pt(pt.get_x(p), cy), p])
            canvas_form.stroke_only_with(form, "#d4d4d4", 1.0, None, None)
            |> canvas_form.line_group(stem)
            Nil
          }
          False -> Nil
        }
      })

      canvas_form.stroke_only_with(form, "#d4d4d4", 1.0, None, None)
      |> canvas_form.line_group(ctrls)
      |> canvas_form.stroke_only_with("#b5b5b5", 2.0, None, None)
      |> canvas_form.line_group(bs)
      |> canvas_form.stroke_only_with("#555555", 1.8, None, None)
      |> canvas_form.line_group(card)
      |> canvas_form.stroke_only_with("#111111", 2.6, None, None)
      |> canvas_form.line_group(cr)

      list.each(pts_list, fn(p) {
        canvas_form.fill(form, "#ffffff")
        |> canvas_form.stroke_with("#111111", 1.5, None, None)
        |> canvas_form.point_pt_with(p, 4.5, "circle")
        Nil
      })
    })

  canvas_space.add_i_player(cs, player)
  canvas_space.bind_mouse(cs)
  canvas_space.bind_touch(cs)
  canvas_space.play(cs)
  Nil
}
