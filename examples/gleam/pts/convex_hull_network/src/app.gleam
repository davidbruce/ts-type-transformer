import gleam/float
import gleam/int
import gleam/list
import gleam/option.{None}
import pts/canvas_form
import pts/canvas_space
import pts/canvas_space_options
import pts/create
import pts/geom
import pts/group
import pts/i_player
import pts/polygon
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
    canvas_space.new_with("#pt", None)
    |> canvas_space.setup(opts)
  let form = canvas_space.get_form(cs)

  let player =
    i_player.create()
    |> i_player.with_animate(fn(time_ms, _ftime, _space) {
      let c = canvas_space.get_center(cs)
      let size = canvas_space.get_size(cs)
      let pointer = canvas_space.get_pointer(cs)
      let cx = pt.get_x(c)
      let cy = pt.get_y(c)
      let px = pt.get_x(pointer)
      let py = pt.get_y(pointer)
      let base_r =
        float.max(70.0, float.min(pt.get_x(size), pt.get_y(size)) *. 0.34)
      let radial =
        create.radial_pts_pt_with(c, base_r, 20.0, time_ms *. 0.00025)

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
      let hull = polygon.convex_hull_group_with(cloud, False)
      let bbox = geom.bounding_box_group(hull)
      let cent = polygon.centroid_group(hull)
      let hull_pts = group.to_list(hull)
      let hull_len = list.length(hull_pts)

      canvas_form.stroke_only_with(form, "#b5b5b5", 1.0, None, None)
      |> canvas_form.rect_group(bbox)
      |> canvas_form.fill("#f7f7f7")
      |> canvas_form.stroke_with("#111111", 2.0, None, None)
      |> canvas_form.polygon_group(hull)

      list.each(polygon.network_group_with(hull, 0.0), fn(seg) {
        canvas_form.stroke_only_with(form, "#d4d4d4", 1.0, None, None)
        |> canvas_form.line_group(seg)
        Nil
      })
      case hull_len > 3 {
        True ->
          list.each(
            polygon.network_group_with(hull, int.to_float(hull_len / 2)),
            fn(seg) {
              canvas_form.stroke_only_with(form, "#d4d4d4", 1.0, None, None)
              |> canvas_form.line_group(seg)
              Nil
            },
          )
        False -> Nil
      }

      list.each(cloud_pts, fn(p) {
        canvas_form.fill_only(form, "#737373")
        |> canvas_form.point_pt_with(p, 3.0, "circle")
        Nil
      })
      list.each(hull_pts, fn(h) {
        canvas_form.fill(form, "#111111")
        |> canvas_form.stroke_with("#ffffff", 1.5, None, None)
        |> canvas_form.point_pt_with(h, 5.0, "circle")
        Nil
      })

      let x0 = pt.get_x(cent)
      let y0 = pt.get_y(cent)
      let h_bar = group.new([mk_pt(x0 -. 10.0, y0), mk_pt(x0 +. 10.0, y0)])
      let v_bar = group.new([mk_pt(x0, y0 -. 10.0), mk_pt(x0, y0 +. 10.0)])
      canvas_form.stroke_only_with(form, "#111111", 1.4, None, None)
      |> canvas_form.line_group(h_bar)
      |> canvas_form.line_group(v_bar)
      |> canvas_form.fill("#ffffff")
      |> canvas_form.stroke_with("#111111", 1.6, None, None)
      |> canvas_form.point_pt_with(cent, 5.5, "circle")
      Nil
    })

  canvas_space.add_i_player(cs, player)
  canvas_space.bind_mouse(cs)
  canvas_space.bind_touch(cs)
  canvas_space.play(cs)
  Nil
}
