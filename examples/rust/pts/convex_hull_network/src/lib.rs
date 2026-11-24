#![no_main]

use pts::{
    AnimateCallbackFn, CanvasSpace, CanvasSpaceOptions, Create, Geom, Group, IPlayer, Polygon, Pt,
};

fn mk_pt(x: f64, y: f64) -> Pt {
    Pt::make(2.0).with_x(x).with_y(y)
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let opts = CanvasSpaceOptions::default()
        .with_bgcolor("#ffffff")
        .with_resize(true)
        .with_retina(true);

    let space = CanvasSpace::new_with("#pt", None).setup(&opts);
    let form = space.get_form();

    let anim = {
        let space = space.clone();
        AnimateCallbackFn::new(move |time_ms, _, _| {
            let c = space.center();
            let size = space.size();
            let pointer = space.pointer();
            let base_r = f64::max(70.0, f64::min(size.x(), size.y()) * 0.34);
            let radial = Create::radial_pts_with(&c, base_r, 20.0, time_ms * 0.00025);

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
            let hull = Polygon::convex_hull_with(&cloud, false);
            let bbox = Geom::bounding_box(&hull);
            let cent = Polygon::centroid(&hull);
            let hull_pts: Vec<Pt> = hull.as_array().iter().collect();
            let hull_len = hull_pts.len();

            form.stroke_only_with("#b5b5b5", 1.0, None, None)
                .rect(&bbox);
            form.fill("#f7f7f7")
                .stroke_with("#111111", 2.0, None, None)
                .polygon(&hull);

            for seg in Polygon::network_with(&hull, 0.0).iter() {
                form.stroke_only_with("#d4d4d4", 1.0, None, None).line(&seg);
            }
            if hull_len > 3 {
                for seg in Polygon::network_with(&hull, (hull_len / 2) as f64).iter() {
                    form.stroke_only_with("#d4d4d4", 1.0, None, None).line(&seg);
                }
            }

            for p in &cloud_vec {
                form.fill_only("#737373").point_with(p, 3.0, "circle");
            }
            for h in &hull_pts {
                form.fill("#111111")
                    .stroke_with("#ffffff", 1.5, None, None)
                    .point_with(h, 5.0, "circle");
            }

            let h_bar = Group::new([
                mk_pt(cent.x() - 10.0, cent.y()),
                mk_pt(cent.x() + 10.0, cent.y()),
            ]);
            let v_bar = Group::new([
                mk_pt(cent.x(), cent.y() - 10.0),
                mk_pt(cent.x(), cent.y() + 10.0),
            ]);
            form.stroke_only_with("#111111", 1.4, None, None)
                .line(&h_bar);
            form.stroke_only_with("#111111", 1.4, None, None)
                .line(&v_bar);
            form.fill("#ffffff")
                .stroke_with("#111111", 1.6, None, None)
                .point_with(&cent, 5.5, "circle");
        })
    };

    let player = IPlayer::default().with_animate(&anim);
    space.add(&player);
    space.bind_mouse().bind_touch().play();
}
