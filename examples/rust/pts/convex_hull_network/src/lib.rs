#![no_main]

use pts::{AnimateCallbackFn, CanvasSpace, CanvasSpaceOptions, Create, Geom, Group, IPlayer, Polygon, Pt};

fn mk_pt(x: f64, y: f64) -> Pt {
    Pt::make(2.0, None, None).with_x(x).with_y(y)
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let opts = CanvasSpaceOptions::default()
        .with_bgcolor("#ffffff")
        .with_resize(true)
        .with_retina(true);

    let space = CanvasSpace::new("#pt", None).setup(&opts);
    let form = space.get_form();

    let anim = {
        let space = space.clone();
        AnimateCallbackFn::new(move |time_ms, _, _| {
            let c = space.center();
            let size = space.size();
            let pointer = space.pointer();
            let base_r = f64::max(70.0, f64::min(size.x(), size.y()) * 0.34);
            let radial = Create::radial_pts(&c, base_r, 20.0, Some(time_ms * 0.00025));

            let mut cloud_vec = Vec::with_capacity(21);
            for (i, p) in radial.as_array().iter().enumerate() {
                let fi = i as f64;
                let scale = 0.48
                    + 0.52
                        * (0.5
                            + 0.5
                                * (time_ms * 0.0011 + fi * 0.9).sin()
                                * (time_ms * 0.0007 - fi * 0.5).cos());
                let nx = c.x() + (p.x() - c.x()) * scale;
                let ny = c.y() + (p.y() - c.y()) * scale;
                cloud_vec.push(mk_pt(nx, ny));
            }
            if pointer.x() > 1.0 || pointer.y() > 1.0 {
                cloud_vec.push(mk_pt(pointer.x(), pointer.y()));
            }

            let cloud = Group::new(cloud_vec.clone());
            let hull = Polygon::convex_hull(&cloud, Some(false));
            let bbox = Geom::bounding_box(&hull);
            let cent = Polygon::centroid(&hull);
            let hull_pts: Vec<Pt> = hull.as_array().iter().collect();
            let hull_len = hull_pts.len();

            form.stroke_only("#b5b5b5", Some(1.0), None, None).rect_2(&bbox);
            form.fill("#f7f7f7")
                .stroke("#111111", Some(2.0), None, None)
                .polygon_2(&hull);

            for seg in Polygon::network(&hull, Some(0.0)).iter() {
                form.stroke_only("#d4d4d4", Some(1.0), None, None).line_2(&seg);
            }
            if hull_len > 3 {
                for seg in Polygon::network(&hull, Some((hull_len / 2) as f64)).iter() {
                    form.stroke_only("#d4d4d4", Some(1.0), None, None).line_2(&seg);
                }
            }

            for p in &cloud_vec {
                form.fill_only("#737373")
                    .point_2(p, Some(3.0), Some("circle"));
            }
            for h in &hull_pts {
                form.fill("#111111")
                    .stroke("#ffffff", Some(1.5), None, None)
                    .point_2(h, Some(5.0), Some("circle"));
            }

            let h_bar = Group::new([
                mk_pt(cent.x() - 10.0, cent.y()),
                mk_pt(cent.x() + 10.0, cent.y()),
            ]);
            let v_bar = Group::new([
                mk_pt(cent.x(), cent.y() - 10.0),
                mk_pt(cent.x(), cent.y() + 10.0),
            ]);
            form.stroke_only("#111111", Some(1.4), None, None).line_2(&h_bar);
            form.stroke_only("#111111", Some(1.4), None, None).line_2(&v_bar);
            form.fill("#ffffff")
                .stroke("#111111", Some(1.6), None, None)
                .point_2(&cent, Some(5.5), Some("circle"));
        })
    };

    let player = IPlayer::default().with_animate(&anim);
    space.add(&player);
    space.bind_mouse(None, None).bind_touch(None, None, None).play(None);
}
