#![no_main]

use std::cell::RefCell;
use std::f64::consts::TAU;
use std::rc::Rc;

use dimforge_rapier3d_compat::*;
use dom::{
    CanvasDrawPathMethods as _, CanvasFillStrokeStylesMethods as _,
    CanvasPathDrawingStylesMethods as _, CanvasPathMethods as _, CanvasRectMethods as _,
    CanvasRenderingContext2D, CanvasTextDrawingStylesMethods as _, CanvasTextMethods as _,
    FrameRequestCallback, HTMLCanvasElement, HTMLCanvasElementMethods as _, HTMLElement,
};

const DOMINO_COUNT: usize = 9;
const DOM_HX: f64 = 0.13;
const DOM_HY: f64 = 0.85;
const TRAIL_LEN: usize = 22;

fn to_cx(wx: f64) -> f64 {
    340.0 + wx * 46.0
}
fn to_cy(wy: f64) -> f64 {
    350.0 - wy * 46.0
}

struct AppState {
    ctx: CanvasRenderingContext2D,
    meta: HTMLElement,
    pause_btn: HTMLElement,
    world: Option<World>,
    dominoes: Vec<RigidBody>,
    striker: Option<RigidBody>,
    trail_x: [f64; TRAIL_LEN],
    trail_y: [f64; TRAIL_LEN],
    trail_count: usize,
    step_count: usize,
    paused: bool,
}

fn draw_rot_box(
    ctx: &CanvasRenderingContext2D,
    cx: f64,
    cy: f64,
    qx: f64,
    qy: f64,
    qz: f64,
    qw: f64,
    hx: f64,
    hy: f64,
    fill_hex: &str,
) {
    let r00 = 1.0 - 2.0 * (qy * qy + qz * qz);
    let r01 = 2.0 * (qx * qy - qw * qz);
    let r10 = 2.0 * (qx * qy + qw * qz);
    let r11 = 1.0 - 2.0 * (qx * qx + qz * qz);

    let xs = [
        to_cx(cx + r00 * (-hx) + r01 * (-hy)),
        to_cx(cx + r00 * hx + r01 * (-hy)),
        to_cx(cx + r00 * hx + r01 * hy),
        to_cx(cx + r00 * (-hx) + r01 * hy),
    ];
    let ys = [
        to_cy(cy + r10 * (-hx) + r11 * (-hy)),
        to_cy(cy + r10 * hx + r11 * (-hy)),
        to_cy(cy + r10 * hx + r11 * hy),
        to_cy(cy + r10 * (-hx) + r11 * hy),
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

fn build_scene(st: &mut AppState) {
    if let Some(w) = st.world.take() {
        w.free();
    }
    let world = World::new(
        &Vector3::new(0.0, -9.81, 0.0),
        None, None, None, None, None, None, None, None, None, None, None, None, None,
    );

    let floor = world.create_rigid_body(&RigidBodyDesc::fixed().set_translation_2(0.0, -0.35, 0.0));
    world.create_collider(&ColliderDesc::cuboid(7.2, 0.35, 2.0).set_friction_2(0.8), Some(&floor));

    st.dominoes.clear();
    for i in 0..DOMINO_COUNT {
        let x = -2.4 + i as f64 * 0.88;
        let rb = world.create_rigid_body(&RigidBodyDesc::dynamic().set_translation_2(x, DOM_HY, 0.0));
        let cd = ColliderDesc::cuboid(DOM_HX, DOM_HY, 0.35)
            .set_friction_2(0.55)
            .set_restitution_2(0.15)
            .set_density_2(1.0);
        world.create_collider(&cd, Some(&rb));
        st.dominoes.push(rb);
    }

    let striker = world.create_rigid_body(&RigidBodyDesc::dynamic().set_translation_2(-5.3, 1.05, 0.0));
    let bcd = ColliderDesc::ball(0.40)
        .set_density_2(2.8)
        .set_restitution_2(0.45)
        .set_friction_2(0.25);
    world.create_collider(&bcd, Some(&striker));
    striker.apply_impulse(&Vector3::new(4.8, 0.2, 0.0), true);

    st.striker = Some(striker);
    st.world = Some(world);
    st.step_count = 0;
    st.trail_count = 0;
}

fn step_sim(st: &mut AppState) {
    if let Some(ref w) = st.world {
        w.step(None, None);
    }
    st.step_count += 1;
    if let Some(ref striker) = st.striker {
        let sp = striker.translation(None);
        let idx = (st.step_count - 1) % TRAIL_LEN;
        st.trail_x[idx] = sp.x();
        st.trail_y[idx] = sp.y();
        if st.trail_count < TRAIL_LEN {
            st.trail_count += 1;
        }
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
    ctx.set_stroke_style("#111111");
    ctx.set_line_width(1.5);
    ctx.begin_path();
    ctx.move_to(0.0, 350.0);
    ctx.line_to(680.0, 350.0);
    ctx.stroke();

    ctx.set_fill_style("#b5b5b5");
    for i in 0..st.trail_count {
        ctx.begin_path();
        ctx.arc(to_cx(st.trail_x[i]), to_cy(st.trail_y[i]), 2.0, 0.0, TAU, None);
        ctx.fill(None);
    }

    let mut toppled = 0usize;
    for d in &st.dominoes {
        let p = d.translation(None);
        let q = d.rotation(None);
        let tilted = q.z().abs() > 0.25;
        if tilted {
            toppled += 1;
        }
        draw_rot_box(
            ctx,
            p.x(),
            p.y(),
            q.x(),
            q.y(),
            q.z(),
            q.w(),
            DOM_HX,
            DOM_HY,
            if tilted { "#d4d4d4" } else { "#f7f7f7" },
        );
    }

    let mut spx = 0.0;
    if let Some(ref striker) = st.striker {
        let sp = striker.translation(None);
        spx = sp.x();
        let scx = to_cx(sp.x());
        let scy = to_cy(sp.y());
        ctx.set_fill_style("#262626");
        ctx.begin_path();
        ctx.arc(scx, scy, 0.40 * 46.0, 0.0, TAU, None);
        ctx.fill(None);
        ctx.set_stroke_style("#111111");
        ctx.set_line_width(1.6);
        ctx.begin_path();
        ctx.arc(scx, scy, 0.40 * 46.0, 0.0, TAU, None);
        ctx.stroke();
        ctx.set_fill_style("#e5e5e5");
        ctx.begin_path();
        ctx.arc(scx - 5.0, scy - 5.0, 2.5, 0.0, TAU, None);
        ctx.fill(None);
    }

    let cap = format!("Dominoes toppled: {} / 9", toppled);
    ctx.set_fill_style("#555555");
    ctx.set_font("11px ui-monospace, monospace");
    ctx.fill_text(&cap, 18.0, 24.0, None);

    let msg = format!(
        "9 cuboids + striker sphere \u{00B7} toppled={}/9 \u{00B7} striker_x={:.2} m \u{00B7} step={} ({})",
        toppled,
        spx,
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
            pause_btn: doc.get_element_by_id("btn-pause").unwrap(),
            world: None,
            dominoes: Vec::new(),
            striker: None,
            trail_x: [0.0; TRAIL_LEN],
            trail_y: [0.0; TRAIL_LEN],
            trail_count: 0,
            step_count: 0,
            paused: false,
        }));

        {
            let mut s = st.borrow_mut();
            build_scene(&mut s);
            for _ in 0..22 {
                step_sim(&mut s);
            }
            render(&s);
        }

        for (i, bid) in ["btn-push", "btn-pause", "btn-reset"].iter().enumerate() {
            let btn = doc.get_element_by_id(bid).unwrap();
            let st_c = st.clone();
            let cb = dom::JsListener::new(move |_: dom::PointerEvent| {
                let mut s = st_c.borrow_mut();
                match i {
                    0 => {
                        if let Some(ref striker) = s.striker {
                            striker.apply_impulse(&Vector3::new(3.5, 0.6, 0.0), true);
                        }
                        render(&s);
                    }
                    1 => {
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
                        for _ in 0..12 {
                            step_sim(&mut s);
                        }
                        render(&s);
                    }
                }
            });
            btn.add_event_listener(dom::HTMLElementEventMap::CLICK, &cb, None);
            std::mem::forget(cb);
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
        std::mem::forget(raf_holder);
    });
}
