#![no_main]

use std::cell::RefCell;
use std::f64::consts::{PI, TAU};
use std::rc::Rc;

use dimforge_rapier3d_compat::*;
use dom::{CanvasRenderingContext2D, FrameRequestCallback, HTMLCanvasElement, HTMLElement};

const RAY_COUNT: usize = 37;
const MAX_TOI: f64 = 16.0;
const EMITTER_X: f64 = -5.6;
const EMITTER_Y: f64 = 4.6;

fn to_cx(wx: f64) -> f64 {
    340.0 + wx * 46.0
}
fn to_cy(wy: f64) -> f64 {
    350.0 - wy * 46.0
}

struct AppState {
    ctx: CanvasRenderingContext2D,
    meta: HTMLElement,
    sweep_btn: HTMLElement,
    world: World,
    auto_sweep: bool,
    phase: f64,
    manual_offset: f64,
}

fn draw_box(
    ctx: &CanvasRenderingContext2D,
    cx: f64,
    cy: f64,
    theta: f64,
    hx: f64,
    hy: f64,
    fill_hex: &str,
) {
    let c = theta.cos();
    let s = theta.sin();
    let xs = [
        to_cx(cx + c * (-hx) - s * (-hy)),
        to_cx(cx + c * hx - s * (-hy)),
        to_cx(cx + c * hx - s * hy),
        to_cx(cx + c * (-hx) - s * hy),
    ];
    let ys = [
        to_cy(cy + s * (-hx) + c * (-hy)),
        to_cy(cy + s * hx + c * (-hy)),
        to_cy(cy + s * hx + c * hy),
        to_cy(cy + s * (-hx) + c * hy),
    ];
    ctx.set_fill_style(fill_hex);
    ctx.begin_path();
    ctx.move_to(xs[0], ys[0]);
    for i in 1..4 {
        ctx.line_to(xs[i], ys[i]);
    }
    ctx.close_path();
    ctx.fill(None);

    ctx.set_stroke_style("#111111");
    ctx.set_line_width(1.5);
    ctx.begin_path();
    ctx.move_to(xs[0], ys[0]);
    for i in 1..4 {
        ctx.line_to(xs[i], ys[i]);
    }
    ctx.close_path();
    ctx.stroke();
}

fn draw_circle(ctx: &CanvasRenderingContext2D, wx: f64, wy: f64, r: f64, fill_hex: &str) {
    let cx = to_cx(wx);
    let cy = to_cy(wy);
    let rpx = r * 46.0;
    ctx.set_fill_style(fill_hex);
    ctx.begin_path();
    ctx.arc(cx, cy, rpx, 0.0, TAU, None);
    ctx.fill(None);
    ctx.set_stroke_style("#111111");
    ctx.set_line_width(1.5);
    ctx.begin_path();
    ctx.arc(cx, cy, rpx, 0.0, TAU, None);
    ctx.stroke();
}

fn render(st: &AppState) {
    let ctx = &st.ctx;
    ctx.set_fill_style("#ffffff");
    ctx.fill_rect(0.0, 0.0, 680.0, 400.0);

    ctx.set_stroke_style("#efefef");
    ctx.set_line_width(1.0);
    for gx in -6..=6 {
        let x = to_cx(gx as f64);
        ctx.begin_path();
        ctx.move_to(x, 0.0);
        ctx.line_to(x, 350.0);
        ctx.stroke();
    }
    for gy in 1..=7 {
        let y = to_cy(gy as f64);
        ctx.begin_path();
        ctx.move_to(0.0, y);
        ctx.line_to(680.0, y);
        ctx.stroke();
    }

    draw_box(ctx, 0.0, -0.35, 0.0, 7.0, 0.35, "#f7f7f7");
    draw_box(ctx, -1.6, 0.85, 0.0, 0.70, 0.85, "#efefef");
    draw_box(ctx, 2.2, 1.35, 0.0, 0.65, 1.35, "#efefef");
    draw_box(ctx, 0.2, 1.2, -0.42, 1.3, 0.18, "#d4d4d4");
    draw_box(ctx, 0.4, 4.4, 0.0, 1.9, 0.22, "#efefef");
    draw_circle(ctx, -2.5, 3.6, 0.65, "#d4d4d4");
    draw_circle(ctx, 4.2, 3.2, 0.75, "#d4d4d4");

    let base_angle = -0.35 + st.manual_offset + 0.28 * st.phase.sin();
    let ox = to_cx(EMITTER_X);
    let oy = to_cy(EMITTER_Y);
    let origin = Vector3::new(EMITTER_X, EMITTER_Y, 0.0);

    let mut hits = 0usize;
    let mut min_toi = MAX_TOI;
    for i in 0..RAY_COUNT {
        let frac = i as f64 / (RAY_COUNT - 1) as f64 - 0.5;
        let angle = base_angle + frac * 1.35;
        let dx = angle.cos();
        let dy = angle.sin();
        let ray = Ray::new(&origin, &Vector3::new(dx, dy, 0.0));
        if let Some(hit) = st.world.cast_ray_and_get_normal(&ray, MAX_TOI, true, None, None, None, None, None) {
            hits += 1;
            let toi = hit.time_of_impact();
            if toi < min_toi {
                min_toi = toi;
            }
            let pt = ray.point_at(toi);
            let hx = to_cx(pt.x());
            let hy = to_cy(pt.y());

            ctx.set_stroke_style("#737373");
            ctx.set_line_width(1.0);
            ctx.begin_path();
            ctx.move_to(ox, oy);
            ctx.line_to(hx, hy);
            ctx.stroke();

            let n = hit.normal();
            ctx.set_stroke_style("#111111");
            ctx.set_line_width(1.4);
            ctx.begin_path();
            ctx.move_to(hx, hy);
            ctx.line_to(to_cx(pt.x() + n.x() * 0.45), to_cy(pt.y() + n.y() * 0.45));
            ctx.stroke();

            ctx.set_fill_style("#111111");
            ctx.begin_path();
            ctx.arc(hx, hy, 2.8, 0.0, TAU, None);
            ctx.fill(None);
        } else {
            ctx.set_stroke_style("#e5e5e5");
            ctx.set_line_width(0.9);
            ctx.begin_path();
            ctx.move_to(ox, oy);
            ctx.line_to(to_cx(EMITTER_X + dx * MAX_TOI), to_cy(EMITTER_Y + dy * MAX_TOI));
            ctx.stroke();
        }
    }

    ctx.set_fill_style("#111111");
    ctx.begin_path();
    ctx.arc(ox, oy, 6.0, 0.0, TAU, None);
    ctx.fill(None);
    ctx.set_stroke_style("#ffffff");
    ctx.set_line_width(1.5);
    ctx.begin_path();
    ctx.arc(ox, oy, 3.0, 0.0, TAU, None);
    ctx.stroke();

    let deg = (base_angle * 180.0 / PI).round() as i32;
    let msg = format!(
        "37 rays \u{00B7} hits={}/37 \u{00B7} nearest_toi={:.2} m \u{00B7} center={} deg ({})",
        hits,
        min_toi,
        deg,
        if st.auto_sweep { "auto" } else { "manual" }
    );
    st.meta.set_text_content(Some(&msg));
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    dimforge_rapier3d_compat::js::spawn(async {
        if dimforge_rapier3d_compat::init().await.is_err() {
            return;
        }
        let doc = dom::document();
        let canvas = HTMLCanvasElement::cast(&doc.get_element_by_id("canvas").unwrap()).unwrap();
        let ctx = canvas.get_context_2d(None).unwrap();

        let world = World::new(
            &Vector3::new(0.0, 0.0, 0.0),
            None, None, None, None, None, None, None, None, None, None, None, None, None,
        );
        let add_box = |cx: f64, cy: f64, theta: f64, hx: f64, hy: f64| {
            let q = Quaternion::new(0.0, 0.0, (theta * 0.5).sin(), (theta * 0.5).cos());
            let rb = world.create_rigid_body(
                &RigidBodyDesc::fixed().set_translation_2(cx, cy, 0.0).set_rotation_2(&q),
            );
            world.create_collider(&ColliderDesc::cuboid(hx, hy, 1.0), Some(&rb));
        };
        let add_sphere = |cx: f64, cy: f64, r: f64| {
            let rb = world.create_rigid_body(&RigidBodyDesc::fixed().set_translation_2(cx, cy, 0.0));
            world.create_collider(&ColliderDesc::ball(r), Some(&rb));
        };

        add_box(0.0, -0.35, 0.0, 7.0, 0.35);
        add_box(-1.6, 0.85, 0.0, 0.70, 0.85);
        add_box(2.2, 1.35, 0.0, 0.65, 1.35);
        add_box(0.2, 1.2, -0.42, 1.3, 0.18);
        add_box(0.4, 4.4, 0.0, 1.9, 0.22);
        add_sphere(-2.5, 3.6, 0.65);
        add_sphere(4.2, 3.2, 0.75);
        world.step(None, None);

        let st = Rc::new(RefCell::new(AppState {
            ctx,
            meta: doc.get_element_by_id("meta").unwrap(),
            sweep_btn: doc.get_element_by_id("btn-sweep").unwrap(),
            world,
            auto_sweep: true,
            phase: 0.0,
            manual_offset: 0.0,
        }));
        render(&st.borrow());

        for (i, bid) in ["btn-sweep", "btn-angle", "btn-reset"].iter().enumerate() {
            let btn = doc.get_element_by_id(bid).unwrap();
            let st_c = st.clone();
            let cb = dom::JsListener::new(move |_: dom::PointerEvent| {
                let mut s = st_c.borrow_mut();
                match i {
                    0 => {
                        s.auto_sweep = !s.auto_sweep;
                        s.sweep_btn.set_text_content(Some(if s.auto_sweep {
                            "Sweep: Auto"
                        } else {
                            "Sweep: Manual"
                        }));
                        render(&s);
                    }
                    1 => {
                        s.manual_offset += PI / 12.0;
                        if s.manual_offset > 0.8 {
                            s.manual_offset = -0.6;
                        }
                        render(&s);
                    }
                    _ => {
                        s.auto_sweep = true;
                        s.phase = 0.0;
                        s.manual_offset = 0.0;
                        s.sweep_btn.set_text_content(Some("Sweep: Auto"));
                        render(&s);
                    }
                }
            });
            btn.add_event_listener(dom::HTMLElementEventMap::CLICK, &cb, None);
        }

        let raf_holder: Rc<RefCell<Option<FrameRequestCallback>>> = Rc::new(RefCell::new(None));
        let raf_c = raf_holder.clone();
        let st_c = st.clone();
        *raf_holder.borrow_mut() = Some(FrameRequestCallback::new(move |_: f64| {
            {
                let mut s = st_c.borrow_mut();
                if s.auto_sweep {
                    s.phase += 0.025;
                    render(&s);
                }
            }
            if let Some(ref cb) = *raf_c.borrow() {
                dom::request_animation_frame(cb);
            }
        }));
        if let Some(ref cb) = *raf_holder.borrow() {
            dom::request_animation_frame(cb);
        }
    });
}
