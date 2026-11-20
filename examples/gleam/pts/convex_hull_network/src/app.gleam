import gleam/float
import gleam/int
import gleam/list
import gleam/option.{None, Some}
import pts/areas/main/canvas_form
import pts/areas/main/canvas_space
import pts/areas/main/canvas_space_options
import pts/areas/main/create
import pts/areas/main/geom
import pts/areas/main/group
import pts/areas/main/i_player
import pts/areas/main/multi_touch_space
import pts/areas/main/polygon
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
  let opts =
    canvas_space_options.create()
    |> canvas_space_options.with_bgcolor("#ffffff")
    |> canvas_space_options.with_resize(True)
    |> canvas_space_options.with_retina(True)

  let cs =
    canvas_space.new(Some(t.StringOrElementString("#pt")), None)
    |> canvas_space.setup(opts)
  let form = canvas_space.get_form(cs)

  let player =
    i_player.create()
    |> i_player.with_animate(fn(time_ms, _ftime, _space) {
      let c = space.get_center(cs)
      let size = space.get_size(cs)
      let pointer = multi_touch_space.get_pointer(cs)
      let cx = pt.get_x(c)
      let cy = pt.get_y(c)
      let px = pt.get_x(pointer)
      let py = pt.get_y(pointer)
      let base_r = float.max(70.0, float.min(pt.get_x(size), pt.get_y(size)) *. 0.34)
      let radial =
        create.radial_pts(t.PtLikePt(c), base_r, 20.0, Some(time_ms *. 0.00025))

      let scaled =
        list.index_map(group.to_list(radial), fn(p, i) {
          let fi = int.to_float(i)
          let scale =
            0.48
            +. 0.52
            *. {
              0.5
              +. 0.5
              *. sin(time_ms *. 0.0011 +. fi *. 0.9)
              *. cos(time_ms *. 0.0007 -. fi *. 0.5)
            }
          let nx = cx +. { pt.get_x(p) -. cx } *. scale
          let ny = cy +. { pt.get_y(p) -. cy } *. scale
          mk_pt(nx, ny)
        })
      let cloud_pts = case px >. 1.0 || py >. 1.0 {
        True -> list.append(scaled, [mk_pt(px, py)])
        False -> scaled
      }

      let cloud = group.new(cloud_pts)
      let hull = polygon.convex_hull(t.PtLikeIterableGroup(cloud), Some(False))
      let bbox = geom.bounding_box(t.PtIterableGroup(hull))
      let cent = polygon.centroid(t.PtLikeIterableGroup(hull))
      let hull_pts = group.to_list(hull)
      let hull_len = list.length(hull_pts)

      let _ =
        canvas_form.stroke_only(
          form,
          t.CanvasFormFillCString("#b5b5b5"),
          Some(1.0),
          None,
          None,
        )
        |> canvas_form.rect_2(t.PtLikeIterableGroup(bbox))
        |> canvas_form.fill(t.CanvasFormFillCString("#f7f7f7"))
        |> canvas_form.stroke(
          t.CanvasFormFillCString("#111111"),
          Some(2.0),
          None,
          None,
        )
        |> canvas_form.polygon_2(t.PtLikeIterableGroup(hull))

      list.each(polygon.network(t.PtIterableGroup(hull), Some(0.0)), fn(seg) {
        let _ =
          canvas_form.stroke_only(
            form,
            t.CanvasFormFillCString("#d4d4d4"),
            Some(1.0),
            None,
            None,
          )
          |> canvas_form.line_2(t.PtLikeIterableGroup(seg))
        Nil
      })
      case hull_len > 3 {
        True ->
          list.each(
            polygon.network(
              t.PtIterableGroup(hull),
              Some(int.to_float(hull_len / 2)),
            ),
            fn(seg) {
              let _ =
                canvas_form.stroke_only(
                  form,
                  t.CanvasFormFillCString("#d4d4d4"),
                  Some(1.0),
                  None,
                  None,
                )
                |> canvas_form.line_2(t.PtLikeIterableGroup(seg))
              Nil
            },
          )
        False -> Nil
      }

      list.each(cloud_pts, fn(p) {
        let _ =
          canvas_form.fill_only(form, t.CanvasFormFillCString("#737373"))
          |> canvas_form.point_2(t.PtLikePt(p), Some(3.0), Some("circle"))
        Nil
      })
      list.each(hull_pts, fn(h) {
        let _ =
          canvas_form.fill(form, t.CanvasFormFillCString("#111111"))
          |> canvas_form.stroke(
            t.CanvasFormFillCString("#ffffff"),
            Some(1.5),
            None,
            None,
          )
          |> canvas_form.point_2(t.PtLikePt(h), Some(5.0), Some("circle"))
        Nil
      })

      let x0 = pt.get_x(cent)
      let y0 = pt.get_y(cent)
      let h_bar = group.new([mk_pt(x0 -. 10.0, y0), mk_pt(x0 +. 10.0, y0)])
      let v_bar = group.new([mk_pt(x0, y0 -. 10.0), mk_pt(x0, y0 +. 10.0)])
      let _ =
        canvas_form.stroke_only(
          form,
          t.CanvasFormFillCString("#111111"),
          Some(1.4),
          None,
          None,
        )
        |> canvas_form.line_2(t.PtLikeIterableGroup(h_bar))
        |> canvas_form.line_2(t.PtLikeIterableGroup(v_bar))
        |> canvas_form.fill(t.CanvasFormFillCString("#ffffff"))
        |> canvas_form.stroke(
          t.CanvasFormFillCString("#111111"),
          Some(1.6),
          None,
          None,
        )
        |> canvas_form.point_2(t.PtLikePt(cent), Some(5.5), Some("circle"))
      Nil
    })

  let _ = space.add(cs, r.dynamic(player))
  let _ = multi_touch_space.bind_mouse(cs, Some(True), None)
  let _ = multi_touch_space.bind_touch(cs, Some(True), None, None)
  let _ = space.play(cs, None)
  Nil
}
