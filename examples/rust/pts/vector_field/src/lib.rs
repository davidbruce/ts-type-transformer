#![no_main]

use pts::{
    AnimateCallbackFn, CanvasFormMethods as _, CanvasSpace, CanvasSpaceMethods as _,
    CanvasSpaceOptions, CanvasSpaceOptionsMethods as _, Circle, Create, IPlayer, IPlayerMethods as _,
    Line, MultiTouchSpaceMethods as _, Num, PtMethods as _, SpaceMethods as _, StringOrElement,
};

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
        AnimateCallbackFn::new(move |time, _, _| {
            let pointer = space.pointer();
            let grid = Create::grid_pts(&space.inner_bound(), 18.0, 11.0, None);
            let size = space.size();
            let max_dist = f64::max(1.0, f64::max(size.x(), size.y()) * 0.55);

            for pt in grid.as_array().iter() {
                let delta = pt.subtract_new([&pointer]);
                let dist = delta.magnitude();
                let t = 1.0 - Num::clamp(dist / max_dist, 0.0, 1.0);
                let angle = delta.angle(None) + (time * 0.0015 + dist * 0.01).sin() * 0.35;
                let len = 8.0 + 34.0 * t;
                let seg = Line::from_angle(&pt, angle, len);

                let light = 65.0 - 57.0 * t;
                let alpha = 0.25 + 0.75 * t;
                let color = format!("hsla(0, 0%, {light:.0}%, {alpha:.2})");

                form.stroke_only(color.as_str(), Some(1.5 + 2.5 * t), None, None)
                    .line_2(&seg);
                let dot = Circle::from_center(&pt, 1.5 + 4.5 * t * t);
                form.fill_only(color.as_str()).circle_2(&dot);
            }

            let ring_radius = 18.0 + 6.0 * (time * 0.004).sin();
            let ring = Circle::from_center(&pointer, ring_radius);
            form.stroke_only("#11111199", Some(2.0), None, None)
                .circle_2(&ring);
        })
    };

    player.set_animate(Some(&anim));
    space.add(&player);
    space.bind_mouse(None, None).bind_touch(None, None, None).play(None);
}
