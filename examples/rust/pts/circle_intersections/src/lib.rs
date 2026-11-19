#![no_main]

use pts::{
    AnimateCallbackFn, CanvasFormMethods as _, CanvasSpace, CanvasSpaceMethods as _,
    CanvasSpaceOptions, CanvasSpaceOptionsMethods as _, Circle, Group, IPlayer, IPlayerMethods as _,
    Line, MultiTouchSpaceMethods as _, Pt, SpaceMethods as _, StringOrElement,
};

fn mk_pt(x: f64, y: f64) -> Pt {
    let p = Pt::make(2.0, None, None);
    p.set_x(x);
    p.set_y(y);
    p
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let opts = CanvasSpaceOptions::create();
    opts.set_bgcolor(Some("#ffffff"));
    opts.set_resize(Some(true));
    opts.set_retina(Some(true));

    let space = CanvasSpace::new(Some(StringOrElement::from("#pt")), None).setup(&opts);
    let form = space.get_form();

    let player = IPlayer::create();
    let anim = {
        let space = space.clone();
        AnimateCallbackFn::new(move |time_ms, _, _| {
            let c = space.center();
            let size = space.size();
            let pointer = space.pointer();
            let min_dim = f64::max(120.0, f64::min(size.x(), size.y()));

            let ang = time_ms * 0.00035;
            let half_len = min_dim * 0.44;
            let ca = ang.cos();
            let sa = ang.sin();
            let chord = Group::new([
                mk_pt(c.x() - ca * half_len, c.y() - sa * half_len),
                mk_pt(c.x() + ca * half_len, c.y() + sa * half_len),
            ]);
            form.stroke_only("#999999", Some(1.2), None, None).line_2(&chord);

            let probe_center = if pointer.x() > 1.0 || pointer.y() > 1.0 {
                pointer
            } else {
                c.clone()
            };
            let probe = Circle::from_center(&probe_center, min_dim * 0.20);

            let mut targets = Vec::with_capacity(5);
            for i in 0..5 {
                let fi = i as f64;
                let orbit_r = min_dim * (0.14 + 0.04 * fi);
                let theta = time_ms * (0.00045 + 0.00012 * fi) + fi * 1.256;
                let tc = mk_pt(c.x() + theta.cos() * orbit_r, c.y() + theta.sin() * orbit_r);
                let tr = min_dim * (0.11 + 0.025 * ((i % 3) as f64));
                let circle = Circle::from_center(&tc, tr);

                form.fill("#f7f7f7")
                    .stroke("#737373", Some(1.4), None, None)
                    .circle_2(&circle);
                if let Some(foot) = Line::perpendicular_from_pt(&chord, &tc, Some(false)) {
                    let stem = Group::new([tc, foot.clone()]);
                    form.stroke_only("#b5b5b5", Some(1.0), None, None).line_2(&stem);
                    form.fill_only("#555555")
                        .point_2(&foot, Some(2.5), Some("circle"));
                }
                targets.push(circle);
            }

            form.stroke_only("#111111", Some(2.0), None, None).circle_2(&probe);
            let px1 = Group::new([
                mk_pt(probe_center.x() - 6.0, probe_center.y()),
                mk_pt(probe_center.x() + 6.0, probe_center.y()),
            ]);
            let py1 = Group::new([
                mk_pt(probe_center.x(), probe_center.y() - 6.0),
                mk_pt(probe_center.x(), probe_center.y() + 6.0),
            ]);
            form.stroke_only("#111111", Some(1.4), None, None).line_2(&px1);
            form.stroke_only("#111111", Some(1.4), None, None).line_2(&py1);

            for circle in &targets {
                let hits = Circle::intersect_circle2d(&probe, circle);
                let hit_vec: Vec<Pt> = hits.as_array().iter().collect();
                if hit_vec.len() == 2 {
                    form.stroke_only("#111111", Some(1.4), None, None).line_2(&hits);
                }
                for h in &hit_vec {
                    form.fill("#ffffff")
                        .stroke("#111111", Some(2.0), None, None)
                        .point_2(h, Some(5.0), Some("circle"));
                }
            }
        })
    };

    player.set_animate(Some(&anim));
    space.add(&player);
    space.bind_mouse(None, None).bind_touch(None, None, None).play(None);
}
