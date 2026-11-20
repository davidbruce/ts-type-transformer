#![no_main]

use pts::{AnimateCallbackFn, CanvasSpace, CanvasSpaceOptions, Circle, Group, IPlayer, Num, Polygon, Pt};

fn mk_pt(x: f64, y: f64) -> Pt {
    Pt::make(2.0, None, None).with_x(x).with_y(y)
}

const SHADES: [&str; 9] = [
    "#f7f7f7", "#efefef", "#e5e5e5",
    "#d4d4d4", "#b5b5b5", "#999999",
    "#737373", "#404040", "#262626",
];

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
            let min_dim = f64::min(size.x(), size.y());
            let radius = f64::max(80.0, min_dim * 0.40);

            let px_norm = (pointer.x() / f64::max(1.0, size.x())) - 0.5;
            let t = Num::clamp(
                0.22 + 0.16 * (time_ms * 0.0012).sin() + px_norm * 0.24,
                0.10,
                0.48,
            );

            let mut poly =
                Polygon::from_center(&c, radius, 6.0).rotate2d(time_ms * 0.00035, &c, None);
            let centroid = Polygon::centroid(&poly);

            for (depth, shade) in SHADES.iter().enumerate() {
                let sw = if depth == 0 { 1.6 } else { 1.0 };
                form.fill(*shade)
                    .stroke("#111111", Some(sw), None, None)
                    .polygon_2(&poly);
                if depth == 0 {
                    for pt in poly.as_array().iter() {
                        let ray = Group::new([centroid.clone(), pt.clone()]);
                        form.stroke_only("#d4d4d4", Some(1.0), None, None).line_2(&ray);
                        form.fill_only("#111111")
                            .point_2(&pt, Some(3.5), Some("circle"));
                    }
                }
                poly = Polygon::midpoints(&poly, Some(true), Some(t));
            }

            let h_bar = Group::new([mk_pt(c.x() - 10.0, c.y()), mk_pt(c.x() + 10.0, c.y())]);
            let v_bar = Group::new([mk_pt(c.x(), c.y() - 10.0), mk_pt(c.x(), c.y() + 10.0)]);
            form.stroke_only("#ffffff", Some(1.5), None, None).line_2(&h_bar);
            form.stroke_only("#ffffff", Some(1.5), None, None).line_2(&v_bar);
            form.fill("#ffffff")
                .stroke("#111111", Some(1.5), None, None)
                .point_2(&centroid, Some(5.0), Some("circle"));

            let ring = Circle::from_center(&pointer, 10.0);
            form.stroke_only("#737373", Some(1.5), None, None).circle_2(&ring);
        })
    };

    let player = IPlayer::default().with_animate(&anim);
    space.add(&player);
    space.bind_mouse(None, None).bind_touch(None, None, None).play(None);
}
