#![no_main]

use std::cell::RefCell;
use std::f64::consts::TAU;
use std::rc::Rc;

use dimforge_rapier3d_compat::*;
use dom::{CanvasRenderingContext2D, FrameRequestCallback, HTMLCanvasElement, HTMLElement};

const TRAIL_LEN: usize = 20;
const RESTITUTIONS: [f64; 5] = [0.00, 0.35, 0.60, 0.80, 0.95];
const X_COORDS: [f64; 5] = [-4.2, -2.1, 0.0, 2.1, 4.2];
const BALL_HEX: [&str; 5] = ["#efefef", "#d4d4d4", "#b5b5b5", "#737373", "#262626"];

fn to_cx(wx: f64) -> f64 {
    340.0 + wx * 46.0
}
fn to_cy(wy: f64) -> f64 {
    350.0 - wy * 46.0
}

struct AppState {
    ctx: CanvasRenderingContext2D,
    meta: HTMLElement,
    grav_btn: HTMLElement,
    pause_btn: HTMLElement,
    world: Option<World>,
    balls: Vec<RigidBody>,
    trail_x: [[f64; TRAIL_LEN]; 5],
    trail_y: [[f64; TRAIL_LEN]; 5],
    trail_count: usize,
    step_count: usize,
    earth: bool,
    paused: bool,
}

fn draw_dashed(ctx: &CanvasRenderingContext2D, x1: f64, y1: f64, x2: f64, y2: f64, segs: usize) {
    let dx = x2 - x1;
    let dy = y2 - y1;
    for i in (0..segs).step_by(2) {
        let t0 = i as f64 / segs as f64;
        let t1 = (i + 1) as f64 / segs as f64;
        ctx.begin_path();
        ctx.move_to(x1 + dx * t0, y1 + dy * t0);
        ctx.line_to(x1 + dx * t1, y1 + dy * t1);
        ctx.stroke();
    }
}

fn build_scene(st: &mut AppState) {
    if let Some(w) = st.world.take() {
        w.free();
    }
    let gy = if st.earth { -9.81 } else { -1.62 };
    let gravity = Vector3::new(0.0, gy, 0.0);
    let world = World::new(&gravity, None, None, None, None, None, None, None, None, None, None, None, None, None);

    let ground = world.create_rigid_body(&RigidBodyDesc::fixed().set_translation_2(0.0, -0.4, 0.0));
    world.create_collider(&ColliderDesc::cuboid(7.0, 0.4, 2.0), Some(&ground));
    let lw = world.create_rigid_body(&RigidBodyDesc::fixed().set_translation_2(-6.8, 3.5, 0.0));
    world.create_collider(&ColliderDesc::cuboid(0.25, 4.0, 2.0), Some(&lw));
    let rw = world.create_rigid_body(&RigidBodyDesc::fixed().set_translation_2(6.8, 3.5, 0.0));
    world.create_collider(&ColliderDesc::cuboid(0.25, 4.0, 2.0), Some(&rw));

    st.balls.clear();
    for i in 0..5 {
        let rb = world.create_rigid_body(&RigidBodyDesc::dynamic().set_translation_2(X_COORDS[i], 5.6, 0.0));
        let cd = ColliderDesc::ball(0.42)
            .set_restitution_2(RESTITUTIONS[i])
            .set_friction_2(0.2)
            .set_density_2(1.2);
        world.create_collider(&cd, Some(&rb));
        st.balls.push(rb);
    }

    st.world = Some(world);
    st.step_count = 0;
    st.trail_count = 0;
}

fn step_sim(st: &mut AppState) {
    if let Some(ref w) = st.world {
        w.step(None, None);
    }
    st.step_count += 1;
    let idx = (st.step_count - 1) % TRAIL_LEN;
    for i in 0..5 {
        let p = st.balls[i].translation(None);
        st.trail_x[i][idx] = p.x();
        st.trail_y[i][idx] = p.y();
    }
    if st.trail_count < TRAIL_LEN {
        st.trail_count += 1;
    }
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

    ctx.set_fill_style("#f7f7f7");
    ctx.fill_rect(0.0, 350.0, 680.0, 50.0);
    ctx.set_stroke_style("#d4d4d4");
    let mut hx = -20.0;
    while hx < 700.0 {
        ctx.begin_path();
        ctx.move_to(hx, 350.0);
        ctx.line_to(hx - 24.0, 400.0);
        ctx.stroke();
        hx += 18.0;
    }
    ctx.set_stroke_style("#111111");
    ctx.set_line_width(1.5);
    ctx.begin_path();
    ctx.move_to(0.0, 350.0);
    ctx.line_to(680.0, 350.0);
    ctx.stroke();

    ctx.set_stroke_style("#d4d4d4");
    ctx.set_line_width(1.0);
    draw_dashed(ctx, to_cx(-5.0), to_cy(5.6), to_cx(5.0), to_cy(5.6), 40);
    for &wx in &X_COORDS {
        let cx = to_cx(wx);
        draw_dashed(ctx, cx, to_cy(5.6), cx, 350.0, 20);
    }

    let mut energy = 0.0;
    let mut max_y = 0.0_f64;
    ctx.set_font("11px ui-monospace, monospace");
    for i in 0..5 {
        ctx.set_fill_style("#b5b5b5");
        for k in 0..st.trail_count {
            ctx.begin_path();
            ctx.arc(to_cx(st.trail_x[i][k]), to_cy(st.trail_y[i][k]), 1.6, 0.0, TAU, None);
            ctx.fill(None);
        }

        let p = st.balls[i].translation(None);
        let v = st.balls[i].linvel(None);
        energy += 0.5 * (v.x() * v.x() + v.y() * v.y());
        if p.y() > max_y {
            max_y = p.y();
        }

        let cx = to_cx(p.x());
        let cy = to_cy(p.y());
        let rpx = 0.42 * 46.0;

        ctx.set_fill_style(BALL_HEX[i]);
        ctx.begin_path();
        ctx.arc(cx, cy, rpx, 0.0, TAU, None);
        ctx.fill(None);

        ctx.set_stroke_style("#111111");
        ctx.set_line_width(1.6);
        ctx.begin_path();
        ctx.arc(cx, cy, rpx, 0.0, TAU, None);
        ctx.stroke();

        ctx.set_fill_style(if i >= 3 { "#d4d4d4" } else { "#737373" });
        ctx.begin_path();
        ctx.arc(cx - 5.0, cy - 5.0, 2.8, 0.0, TAU, None);
        ctx.fill(None);

        ctx.set_fill_style("#111111");
        let label = format!("e={:.2}", RESTITUTIONS[i]);
        ctx.fill_text(&label, cx - 22.0, 368.0, None);
    }

    let msg = format!(
        "5 spheres \u{00B7} g={} m/s\u{00B2} \u{00B7} max_y={:.2} m \u{00B7} ke={:.1} \u{00B7} step={} ({})",
        if st.earth { "Earth (-9.81)" } else { "Moon (-1.62)" },
        max_y,
        energy,
        st.step_count,
        if st.paused { "paused" } else { "running" }
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

        let st = Rc::new(RefCell::new(AppState {
            ctx,
            meta: doc.get_element_by_id("meta").unwrap(),
            grav_btn: doc.get_element_by_id("btn-gravity").unwrap(),
            pause_btn: doc.get_element_by_id("btn-pause").unwrap(),
            world: None,
            balls: Vec::new(),
            trail_x: [[0.0; TRAIL_LEN]; 5],
            trail_y: [[0.0; TRAIL_LEN]; 5],
            trail_count: 0,
            step_count: 0,
            earth: true,
            paused: false,
        }));

        {
            let mut s = st.borrow_mut();
            build_scene(&mut s);
            for _ in 0..28 {
                step_sim(&mut s);
            }
            render(&s);
        }

        for (i, bid) in ["btn-gravity", "btn-kick", "btn-pause", "btn-reset"].iter().enumerate() {
            let btn = doc.get_element_by_id(bid).unwrap();
            let st_c = st.clone();
            let cb = dom::JsListener::new(move |_: dom::PointerEvent| {
                let mut s = st_c.borrow_mut();
                match i {
                    0 => {
                        s.earth = !s.earth;
                        let gy = if s.earth { -9.81 } else { -1.62 };
                        if let Some(ref w) = s.world {
                            w.set_gravity(&Vector3::new(0.0, gy, 0.0));
                        }
                        let imp = Vector3::new(0.0, 0.25, 0.0);
                        for b in &s.balls {
                            b.apply_impulse(&imp, true);
                        }
                        s.grav_btn.set_text_content(Some(if s.earth {
                            "Gravity: Earth"
                        } else {
                            "Gravity: Moon"
                        }));
                        render(&s);
                    }
                    1 => {
                        for (j, b) in s.balls.iter().enumerate() {
                            let dx = (j as f64 - 2.0) * 0.28;
                            b.apply_impulse(&Vector3::new(dx, 3.4, 0.0), true);
                        }
                        render(&s);
                    }
                    2 => {
                        s.paused = !s.paused;
                        s.pause_btn.set_text_content(Some(if s.paused {
                            "Resume"
                        } else {
                            "Pause"
                        }));
                        render(&s);
                    }
                    _ => {
                        s.paused = false;
                        s.pause_btn.set_text_content(Some("Pause"));
                        build_scene(&mut s);
                        for _ in 0..18 {
                            step_sim(&mut s);
                        }
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
                if !s.paused {
                    step_sim(&mut s);
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
