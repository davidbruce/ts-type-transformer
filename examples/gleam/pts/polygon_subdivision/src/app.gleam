import gleam/float
import gleam/list
import gleam/option.{None, Some}
import pts/canvas_form
import pts/canvas_space
import pts/canvas_space_options
import pts/circle
import pts/group
import pts/i_player
import pts/num
import pts/polygon
import pts/pt
import pts/runtime as r
import pts/types as t

@external(javascript, "./app_ffi.mjs", "sin")
fn sin(x: Float) -> Float

fn mk_pt(x: Float, y: Float) -> t.Pt(Nil) {
  let p = pt.make(2.0, None, None)
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
    canvas_space.new(Some(t.StringOrElementString("#pt")), None)
    |> canvas_space.setup(opts)
  let form = canvas_space.get_form(cs)

  let shades = [
    "#f7f7f7", "#efefef", "#e5e5e5", "#d4d4d4", "#b5b5b5", "#999999", "#737373",
    "#404040", "#262626",
  ]

  let player =
    i_player.create()
    |> i_player.with_animate(fn(time_ms, _ftime, _space) {
      let c = canvas_space.get_center(cs)
      let size = canvas_space.get_size(cs)
      let pointer = canvas_space.get_pointer(cs)
      let min_dim = float.min(pt.get_x(size), pt.get_y(size))
      let radius = float.max(80.0, min_dim *. 0.40)

      let px_norm =
        pt.get_x(pointer) /. float.max(1.0, pt.get_x(size)) -. 0.5
      let t_val =
        num.clamp(
          0.22 +. 0.16 *. sin(time_ms *. 0.0012) +. px_norm *. 0.24,
          0.10,
          0.48,
        )

      let poly0 =
        polygon.from_center(t.PtLikePt(c), radius, 6.0)
        |> group.rotate2d(time_ms *. 0.00035, Some(t.PtLikePt(c)), None)
      let centroid = polygon.centroid(t.PtLikeIterableGroup(poly0))

      list.index_fold(shades, poly0, fn(poly, shade, depth) {
        let sw = case depth == 0 {
          True -> 1.6
          False -> 1.0
        }
        canvas_form.fill(form, t.CanvasFormFillCString(shade))
        |> canvas_form.stroke(
          t.CanvasFormFillCString("#111111"),
          Some(sw),
          None,
          None,
        )
        |> canvas_form.polygon_2(t.PtLikeIterableGroup(poly))

          case depth == 0 {
            True ->
              list.each(group.to_list(poly), fn(p) {
                let ray = group.new([centroid, p])
                canvas_form.stroke_only(
                  form,
                  t.CanvasFormFillCString("#d4d4d4"),
                  Some(1.0),
                  None,
                  None,
                )
                |> canvas_form.line_2(t.PtLikeIterableGroup(ray))
                |> canvas_form.fill_only(t.CanvasFormFillCString("#111111"))
                |> canvas_form.point_2(
                  t.PtLikePt(p),
                  Some(3.5),
                  Some("circle"),
                )
                Nil
              })
            False -> Nil
          }

          polygon.midpoints(t.PtIterableGroup(poly), Some(True), Some(t_val))
        })

      let cx = pt.get_x(c)
      let cy = pt.get_y(c)
      let h_bar = group.new([mk_pt(cx -. 10.0, cy), mk_pt(cx +. 10.0, cy)])
      let v_bar = group.new([mk_pt(cx, cy -. 10.0), mk_pt(cx, cy +. 10.0)])
      canvas_form.stroke_only(
        form,
        t.CanvasFormFillCString("#ffffff"),
        Some(1.5),
        None,
        None,
      )
      |> canvas_form.line_2(t.PtLikeIterableGroup(h_bar))
      |> canvas_form.line_2(t.PtLikeIterableGroup(v_bar))
      |> canvas_form.fill(t.CanvasFormFillCString("#ffffff"))
      |> canvas_form.stroke(
        t.CanvasFormFillCString("#111111"),
        Some(1.5),
        None,
        None,
      )
      |> canvas_form.point_2(t.PtLikePt(centroid), Some(5.0), Some("circle"))

      let ring = circle.from_center(t.PtLikePt(pointer), 10.0)
      canvas_form.stroke_only(
        form,
        t.CanvasFormFillCString("#737373"),
        Some(1.5),
        None,
        None,
      )
      |> canvas_form.circle_2(t.PtLikeIterableGroup(ring))
      Nil
    })

  canvas_space.add(cs, r.dynamic(player))
  canvas_space.bind_mouse(cs, Some(True), None)
  canvas_space.bind_touch(cs, Some(True), None, None)
  canvas_space.play(cs, None)
  Nil
}
