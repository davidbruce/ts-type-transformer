#![no_main]

use std::cell::RefCell;
use std::f64::consts::TAU;
use std::rc::Rc;

use dimforge_rapier3d_compat::*;
use dom::{CanvasRenderingContext2D, FrameRequestCallback, HTMLCanvasElement, HTMLElement};

const BODY_COUNT: usize = 6;
const IS_BOX: [bool; BODY_COUNT] = [false, false, true, false, true, false];

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
    queue: Option<EventQueue>,
    bodies: Vec<RigidBody>,
    sensor_handle: f64,
    step_count: usize,
    solid_contacts: usize,
    sensor_crossings: usize,
    active_in_sensor: usize,
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
    ctx.fill();

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

fn draw_segs(ctx: &CanvasRenderingContext2D, x1: f64, y1: f64, x2: f64, y2: f64, n: usize) {
    let dx = x2 - x1;
    let dy = y2 - y1;
    for i in (0..n).step_by(2) {
        let t0 = i as f64 / n as f64;
        let t1 = (i + 1) as f64 / n as f64;
        ctx.begin_path();
        ctx.move_to(x1 + dx * t0, y1 + dy * t0);
        ctx.line_to(x1 + dx * t1, y1 + dy * t1);
        ctx.stroke();
    }
}

fn build_scene(st: &mut AppState) {
    if let Some(q) = st.queue.take() {
        q.free();
    }
    if let Some(w) = st.world.take() {
        w.free();
    }
    let world = World::new(&Vector3::new(0.0, -9.81, 0.0));
    let queue = EventQueue::new(true);

    let add_ramp = |cx: f64, cy: f64, hx: f64, hy: f64, theta: f64| {
        let q = Quaternion::new(0.0, 0.0, (theta * 0.5).sin(), (theta * 0.5).cos());
        let rb = world.create_rigid_body(
            &RigidBodyDesc::fixed()
                .set_translation(cx, cy, 0.0)
                .set_rotation(&q),
        );
        let cd = ColliderDesc::cuboid(hx, hy, 1.0)
            .set_friction(0.15)
            .set_restitution(0.35);
        world.create_collider_with(&cd, &rb);
    };

    add_ramp(0.0, -0.35, 6.8, 0.35, 0.0);
    add_ramp(-5.4, 2.8, 0.25, 3.2, 0.0);
    add_ramp(5.4, 2.8, 0.25, 3.2, 0.0);
    add_ramp(-2.1, 4.2, 2.2, 0.18, -0.36);
    add_ramp(1.8, 2.6, 2.2, 0.18, 0.36);

    let sbody = world.create_rigid_body(&RigidBodyDesc::fixed().set_translation(0.0, 1.35, 0.0));
    let sc = world.create_collider_with(
        &ColliderDesc::cuboid(1.85, 0.30, 1.2)
            .set_sensor(true)
            .set_active_events(ActiveEvents::CollisionEvents),
        &sbody,
    );
    st.sensor_handle = sc.handle();

    st.bodies.clear();
    for (i, &is_box) in IS_BOX.iter().enumerate() {
        let x = -3.7 + i as f64 * 0.52;
        let y = 5.4 + (i % 3) as f64 * 0.58;
        let rb = world.create_rigid_body(&RigidBodyDesc::dynamic().set_translation(x, y, 0.0));
        let cd = if is_box {
            ColliderDesc::cuboid(0.28, 0.28, 0.28)
        } else {
            ColliderDesc::ball(0.32)
        };
        world.create_collider_with(
            &cd.set_restitution(0.35)
                .set_friction(0.25)
                .set_active_events(ActiveEvents::CollisionEvents),
            &rb,
        );
        st.bodies.push(rb);
    }

    st.world = Some(world);
    st.queue = Some(queue);
    st.step_count = 0;
    st.solid_contacts = 0;
    st.sensor_crossings = 0;
    st.active_in_sensor = 0;
}

fn step_sim(st: &mut AppState) {
    let sh = st.sensor_handle;
    let mut crossings = st.sensor_crossings;
    let mut active = st.active_in_sensor;
    let mut contacts = st.solid_contacts;
    if let (Some(w), Some(q)) = (st.world.as_ref(), st.queue.as_ref()) {
        w.step_with(q, None);
        DrainCollisionEventsFn::scoped(
            |h1: f64, h2: f64, started: bool| {
                if h1 == sh || h2 == sh {
                    if started {
                        crossings += 1;
                        active += 1;
                    } else if active > 0 {
                        active -= 1;
                    }
                } else if started {
                    contacts += 1;
                }
            },
            |cb| q.drain_collision_events(cb),
        );
    }
    st.sensor_crossings = crossings;
    st.active_in_sensor = active;
    st.solid_contacts = contacts;
    st.step_count += 1;
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

    let sx = to_cx(-1.85);
    let sy = to_cy(1.65);
    let sw = 3.70 * 46.0;
    let sh = 0.60 * 46.0;
    if st.active_in_sensor > 0 {
        ctx.set_fill_style("#efefef");
        ctx.fill_rect(sx, sy, sw, sh);
        ctx.set_stroke_style("#111111");
        ctx.set_line_width(2.0);
        ctx.stroke_rect(sx, sy, sw, sh);
    } else {
        ctx.set_stroke_style("#737373");
        ctx.set_line_width(1.4);
        draw_segs(ctx, sx, sy, sx + sw, sy, 14);
        draw_segs(ctx, sx + sw, sy, sx + sw, sy + sh, 4);
        draw_segs(ctx, sx + sw, sy + sh, sx, sy + sh, 14);
        draw_segs(ctx, sx, sy + sh, sx, sy, 4);
    }

    let gcap = format!("SENSOR GATE ({} inside)", st.active_in_sensor);
    ctx.set_fill_style("#555555");
    ctx.set_font("11px ui-monospace, monospace");
    ctx.fill_text(&gcap, sx + 8.0, sy + 17.0);

    draw_rot_box(ctx, 0.0, -0.35, 0.0, 0.0, 0.0, 1.0, 6.8, 0.35, "#f7f7f7");
    draw_rot_box(ctx, -5.4, 2.8, 0.0, 0.0, 0.0, 1.0, 0.25, 3.2, "#efefef");
    draw_rot_box(ctx, 5.4, 2.8, 0.0, 0.0, 0.0, 1.0, 0.25, 3.2, "#efefef");
    draw_rot_box(
        ctx,
        -2.1,
        4.2,
        0.0,
        0.0,
        (-0.18_f64).sin(),
        (-0.18_f64).cos(),
        2.2,
        0.18,
        "#e5e5e5",
    );
    draw_rot_box(
        ctx,
        1.8,
        2.6,
        0.0,
        0.0,
        (0.18_f64).sin(),
        (0.18_f64).cos(),
        2.2,
        0.18,
        "#e5e5e5",
    );

    for (i, b) in st.bodies.iter().enumerate() {
        let p = b.translation();
        let q = b.rotation();
        if IS_BOX[i] {
            draw_rot_box(
                ctx,
                p.x(),
                p.y(),
                q.x(),
                q.y(),
                q.z(),
                q.w(),
                0.28,
                0.28,
                "#d4d4d4",
            );
        } else {
            let cx = to_cx(p.x());
            let cy = to_cy(p.y());
            ctx.set_fill_style("#262626");
            ctx.begin_path();
            ctx.arc(cx, cy, 0.32 * 46.0, 0.0, TAU);
            ctx.fill();
            ctx.set_stroke_style("#111111");
            ctx.set_line_width(1.5);
            ctx.begin_path();
            ctx.arc(cx, cy, 0.32 * 46.0, 0.0, TAU);
            ctx.stroke();
        }
    }

    let msg = format!(
        "EventQueue \u{00B7} solid_contacts={} \u{00B7} sensor_crossings={} \u{00B7} inside={} \u{00B7} step={} ({})",
        st.solid_contacts,
        st.sensor_crossings,
        st.active_in_sensor,
        st.step_count,
        if st.paused { "paused" } else { "running" }
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
        let ctx = canvas.get_context_2d().unwrap();

        let st = Rc::new(RefCell::new(AppState {
            ctx,
            meta: doc.get_element_by_id("meta").unwrap(),
            pause_btn: doc.get_element_by_id("btn-pause").unwrap(),
            world: None,
            queue: None,
            bodies: Vec::new(),
            sensor_handle: 0.0,
            step_count: 0,
            solid_contacts: 0,
            sensor_crossings: 0,
            active_in_sensor: 0,
            paused: false,
        }));

        {
            let mut s = st.borrow_mut();
            build_scene(&mut s);
            for _ in 0..40 {
                step_sim(&mut s);
            }
            render(&s);
        }

        for (i, bid) in ["btn-drop", "btn-pause", "btn-reset"].iter().enumerate() {
            let btn = doc.get_element_by_id(bid).unwrap();
            let st_c = st.clone();
            let cb = dom::JsListener::new(move |_: dom::PointerEvent| {
                let mut s = st_c.borrow_mut();
                match i {
                    0 => {
                        let v = Vector3::new(0.4, 0.0, 0.0);
                        for (j, b) in s.bodies.iter().enumerate() {
                            let x = -3.7 + j as f64 * 0.52;
                            let y = 5.4 + (j % 3) as f64 * 0.58;
                            b.set_translation(&Vector3::new(x, y, 0.0), true);
                            b.set_linvel(&v, true);
                        }
                        render(&s);
                    }
                    1 => {
                        s.paused = !s.paused;
                        s.pause_btn
                            .set_text_content(if s.paused { "Resume" } else { "Pause" });
                        render(&s);
                    }
                    _ => {
                        s.paused = false;
                        s.pause_btn.set_text_content("Pause");
                        build_scene(&mut s);
                        for _ in 0..25 {
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
