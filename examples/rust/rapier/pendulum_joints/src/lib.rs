#![no_main]

use std::cell::RefCell;
use std::f64::consts::TAU;
use std::rc::Rc;

use dimforge_rapier3d_compat::*;
use dom::{CanvasRenderingContext2D, FrameRequestCallback, HTMLCanvasElement, HTMLElement};

const TRAIL_LEN: usize = 36;
const LINK_HX: f64 = 0.14;
const LINK_HY: f64 = 0.62;

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
    chain: Vec<RigidBody>,
    arm: Option<RigidBody>,
    bob: Option<RigidBody>,
    ax: [f64; TRAIL_LEN],
    ay: [f64; TRAIL_LEN],
    bx: [f64; TRAIL_LEN],
    by: [f64; TRAIL_LEN],
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

fn draw_pivot(ctx: &CanvasRenderingContext2D, wx: f64, wy: f64) {
    let cx = to_cx(wx);
    let cy = to_cy(wy);
    ctx.set_fill_style("#ffffff");
    ctx.begin_path();
    ctx.arc(cx, cy, 4.0, 0.0, TAU, None);
    ctx.fill(None);
    ctx.set_stroke_style("#111111");
    ctx.set_line_width(1.6);
    ctx.begin_path();
    ctx.arc(cx, cy, 4.0, 0.0, TAU, None);
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

    let gantry = world.create_rigid_body(&RigidBodyDesc::fixed().set_translation_2(0.0, 6.2, 0.0));
    world.create_collider(&ColliderDesc::cuboid(6.0, 0.22, 0.6), &gantry);
    let z_axis = Vector3::new(0.0, 0.0, 1.0);

    st.chain.clear();
    let xs = [-2.8, -3.3, -3.8];
    let ys = [5.4, 4.1, 2.8];
    for i in 0..3 {
        let rb = world.create_rigid_body(&RigidBodyDesc::dynamic().set_translation_2(xs[i], ys[i], 0.0));
        world.create_collider(
            &ColliderDesc::cuboid(LINK_HX, LINK_HY, 0.2).set_density_2(if i == 2 { 1.6 } else { 1.4 }),
            &rb,
        );
        st.chain.push(rb);
    }

    world.create_impulse_joint(
        &JointData::revolute(&Vector3::new(-2.2, -0.22, 0.0), &Vector3::new(0.0, LINK_HY + 0.06, 0.0), &z_axis),
        &gantry, &st.chain[0], true,
    );
    world.create_impulse_joint(
        &JointData::revolute(&Vector3::new(0.0, -LINK_HY - 0.06, 0.0), &Vector3::new(0.0, LINK_HY + 0.06, 0.0), &z_axis),
        &st.chain[0], &st.chain[1], true,
    );
    world.create_impulse_joint(
        &JointData::revolute(&Vector3::new(0.0, -LINK_HY - 0.06, 0.0), &Vector3::new(0.0, LINK_HY + 0.06, 0.0), &z_axis),
        &st.chain[1], &st.chain[2], true,
    );
    st.chain[2].apply_impulse(&Vector3::new(-1.4, 0.0, 0.0), true);

    let arm = world.create_rigid_body(&RigidBodyDesc::dynamic().set_translation_2(3.1, 5.2, 0.0));
    world.create_collider(&ColliderDesc::cuboid(0.14, 0.85, 0.2).set_density_2(1.2), &arm);
    let bob = world.create_rigid_body(&RigidBodyDesc::dynamic().set_translation_2(3.8, 3.6, 0.0));
    world.create_collider(&ColliderDesc::ball(0.42).set_density_2(3.0), &bob);

    world.create_impulse_joint(
        &JointData::revolute(&Vector3::new(2.3, -0.22, 0.0), &Vector3::new(0.0, 0.88, 0.0), &z_axis),
        &gantry, &arm, true,
    );
    world.create_impulse_joint(
        &JointData::revolute(&Vector3::new(0.0, -0.88, 0.0), &Vector3::new(0.0, 0.72, 0.0), &z_axis),
        &arm, &bob, true,
    );
    bob.apply_impulse(&Vector3::new(1.8, 0.2, 0.0), true);

    st.arm = Some(arm);
    st.bob = Some(bob);
    st.world = Some(world);
    st.step_count = 0;
    st.trail_count = 0;
}

fn step_sim(st: &mut AppState) {
    if let Some(ref w) = st.world {
        w.step(None, None);
    }
    st.step_count += 1;
    let pa = st.chain[2].translation(None);
    let pb = st.bob.as_ref().unwrap().translation(None);
    let idx = (st.step_count - 1) % TRAIL_LEN;
    st.ax[idx] = pa.x();
    st.ay[idx] = pa.y();
    st.bx[idx] = pb.x();
    st.by[idx] = pb.y();
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
        ctx.line_to(x, 400.0);
        ctx.stroke();
    }
    for gy in 0..=7 {
        let y = to_cy(gy as f64);
        ctx.begin_path();
        ctx.move_to(0.0, y);
        ctx.line_to(680.0, y);
        ctx.stroke();
    }

    draw_rot_box(ctx, 0.0, 6.2, 0.0, 0.0, 0.0, 1.0, 6.0, 0.22, "#efefef");

    ctx.set_fill_style("#b5b5b5");
    for i in 0..st.trail_count {
        ctx.begin_path();
        ctx.arc(to_cx(st.ax[i]), to_cy(st.ay[i]), 1.6, 0.0, TAU, None);
        ctx.fill(None);
        ctx.begin_path();
        ctx.arc(to_cx(st.bx[i]), to_cy(st.by[i]), 1.6, 0.0, TAU, None);
        ctx.fill(None);
    }

    let mut pin_x = [0.0; 2];
    let mut pin_y = [0.0; 2];
    let mut tip_x = 0.0;
    let mut tip_y = 0.0;
    for (i, link) in st.chain.iter().enumerate() {
        let p = link.translation(None);
        let q = link.rotation(None);
        draw_rot_box(
            ctx, p.x(), p.y(), q.x(), q.y(), q.z(), q.w(),
            LINK_HX, LINK_HY, if i == 2 { "#d4d4d4" } else { "#f7f7f7" },
        );
        if i < 2 {
            let r01 = 2.0 * (q.x() * q.y() - q.w() * q.z());
            let r11 = 1.0 - 2.0 * (q.x() * q.x() + q.z() * q.z());
            pin_x[i] = p.x() - r01 * (LINK_HY + 0.06);
            pin_y[i] = p.y() - r11 * (LINK_HY + 0.06);
        } else {
            tip_x = p.x();
            tip_y = p.y();
        }
    }

    let ap = st.arm.as_ref().unwrap().translation(None);
    let aq = st.arm.as_ref().unwrap().rotation(None);
    let bp = st.bob.as_ref().unwrap().translation(None);
    draw_rot_box(ctx, ap.x(), ap.y(), aq.x(), aq.y(), aq.z(), aq.w(), 0.14, 0.85, "#f7f7f7");

    let bcx = to_cx(bp.x());
    let bcy = to_cy(bp.y());
    ctx.set_fill_style("#262626");
    ctx.begin_path();
    ctx.arc(bcx, bcy, 0.42 * 46.0, 0.0, TAU, None);
    ctx.fill(None);
    ctx.set_stroke_style("#111111");
    ctx.set_line_width(1.6);
    ctx.begin_path();
    ctx.arc(bcx, bcy, 0.42 * 46.0, 0.0, TAU, None);
    ctx.stroke();

    draw_pivot(ctx, -2.2, 5.98);
    draw_pivot(ctx, pin_x[0], pin_y[0]);
    draw_pivot(ctx, pin_x[1], pin_y[1]);
    draw_pivot(ctx, 2.3, 5.98);
    let arm_r01 = 2.0 * (aq.x() * aq.y() - aq.w() * aq.z());
    let arm_r11 = 1.0 - 2.0 * (aq.x() * aq.x() + aq.z() * aq.z());
    draw_pivot(ctx, ap.x() - arm_r01 * 0.88, ap.y() - arm_r11 * 0.88);

    ctx.set_fill_style("#555555");
    ctx.set_font("11px ui-monospace, monospace");
    ctx.fill_text("3-Link Revolute Chain", to_cx(-3.5), 36.0, None);
    ctx.fill_text("2-Body Bob Pendulum", to_cx(1.1), 36.0, None);

    let msg = format!(
        "5 revolute joints \u{00B7} chain_tip=({:.2}, {:.2}) \u{00B7} bob=({:.2}, {:.2}) \u{00B7} step={} ({})",
        tip_x, tip_y, bp.x(), bp.y(), st.step_count, if st.paused { "paused" } else { "running" }
    );
    st.meta.set_text_content(&msg);
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
            chain: Vec::new(),
            arm: None,
            bob: None,
            ax: [0.0; TRAIL_LEN],
            ay: [0.0; TRAIL_LEN],
            bx: [0.0; TRAIL_LEN],
            by: [0.0; TRAIL_LEN],
            trail_count: 0,
            step_count: 0,
            paused: false,
        }));

        {
            let mut s = st.borrow_mut();
            build_scene(&mut s);
            for _ in 0..20 {
                step_sim(&mut s);
            }
            render(&s);
        }

        for (i, bid) in ["btn-kick", "btn-pause", "btn-reset"].iter().enumerate() {
            let btn = doc.get_element_by_id(bid).unwrap();
            let st_c = st.clone();
            let cb = dom::JsListener::new(move |_: dom::PointerEvent| {
                let mut s = st_c.borrow_mut();
                match i {
                    0 => {
                        s.chain[2].apply_impulse(&Vector3::new(1.6, 0.3, 0.0), true);
                        if let Some(ref bob) = s.bob {
                            bob.apply_impulse(&Vector3::new(-2.0, 0.4, 0.0), true);
                        }
                        render(&s);
                    }
                    1 => {
                        s.paused = !s.paused;
                        s.pause_btn.set_text_content(if s.paused {
                            "Resume"
                        } else {
                            "Pause"
                        });
                        render(&s);
                    }
                    _ => {
                        s.paused = false;
                        s.pause_btn.set_text_content("Pause");
                        build_scene(&mut s);
                        for _ in 0..15 {
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
