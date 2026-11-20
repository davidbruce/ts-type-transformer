#![no_main]

use pts::{AnimateCallbackFn, CanvasSpace, CanvasSpaceOptions, Curve, Group, IPlayer, Pt};

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
            let size = space.size();
            let c = space.center();
            let pointer = space.pointer();
            let w = f64::max(120.0, size.x());
            let h = f64::max(120.0, size.y());
            let amp = h * 0.24;
            let count = 11usize;

            let base_line = Group::new([mk_pt(w * 0.06, c.y()), mk_pt(w * 0.94, c.y())]);
            form.stroke_only("#e5e5e5", Some(1.0), None, None).line_2(&base_line);

            let mut pts_vec = Vec::with_capacity(count);
            for i in 0..count {
                let u = (i as f64) / ((count - 1) as f64);
                let x = w * (0.08 + 0.84 * u);
                let fi = i as f64;
                let mut y = c.y()
                    + (time_ms * 0.0014 + fi * 0.72).sin() * amp
                    + (time_ms * 0.0009 - fi * 0.45).cos() * (amp * 0.42);
                let dx = (x - pointer.x()).abs();
                if dx < w * 0.25 && (pointer.x() > 1.0 || pointer.y() > 1.0) {
                    let pull = 1.0 - dx / (w * 0.25);
                    y += (pointer.y() - y) * pull * 0.35;
                }
                pts_vec.push(mk_pt(x, y));
            }

            let ctrls = Group::new(pts_vec.clone());
            let bs = Curve::bspline(&ctrls, Some(12.0), None);
            let card = Curve::cardinal(&ctrls, Some(12.0), Some(0.35));
            let cr = Curve::catmull_rom(&ctrls, Some(12.0));

            for (i, p) in cr.as_array().iter().enumerate() {
                if i % 3 == 0 {
                    let stem = Group::new([mk_pt(p.x(), c.y()), p.clone()]);
                    form.stroke_only("#d4d4d4", Some(1.0), None, None).line_2(&stem);
                }
            }

            form.stroke_only("#d4d4d4", Some(1.0), None, None).line_2(&ctrls);
            form.stroke_only("#b5b5b5", Some(2.0), None, None).line_2(&bs);
            form.stroke_only("#555555", Some(1.8), None, None).line_2(&card);
            form.stroke_only("#111111", Some(2.6), None, None).line_2(&cr);

            for p in &pts_vec {
                form.fill("#ffffff")
                    .stroke("#111111", Some(1.5), None, None)
                    .point_2(p, Some(4.5), Some("circle"));
            }
        })
    };

    let player = IPlayer::default().with_animate(&anim);
    space.add(&player);
    space.bind_mouse(None, None).bind_touch(None, None, None).play(None);
}
