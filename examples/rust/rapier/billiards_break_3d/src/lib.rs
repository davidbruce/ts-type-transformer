#![no_main]

use std::cell::RefCell;
use std::rc::Rc;

use babylonjs_lite as b;
use dimforge_rapier3d_compat as rp;
use dom::{GPUColorDict, HTMLCanvasElement, HTMLElement};

const RACK_COUNT: usize = 10;
const BALL_COUNT: usize = 11;
const BALL_RADIUS: f64 = 0.36;

struct AppState {
    meta: HTMLElement,
    pause_btn: HTMLElement,
    world: Option<rp::World>,
    balls: Vec<rp::RigidBody>,
    ball_meshes: Vec<b::Mesh>,
    paused: bool,
    step_count: usize,
    _render_cb: Option<b::NumberFn>,
}

fn set_status_error(msg: &str) {
    if let Some(el) = dom::document().get_element_by_id("status") {
        let text = format!("WebGPU / Babylon Lite error: {msg}");
        el.set_text_content(&text);
    }
}

fn build_world(st: &mut AppState, shoot_cue: bool) {
    if let Some(w) = st.world.take() {
        w.free();
    }
    let world = rp::World::new(
        &rp::Vector3::new(0.0, -9.81, 0.0),
        None, None, None, None, None, None, None, None, None, None, None, None, None,
    );

    let bed = world.create_rigid_body(&rp::RigidBodyDesc::fixed().set_translation(0.0, -0.25, 0.0));
    world.create_collider(
        &rp::ColliderDesc::cuboid(4.6, 0.25, 7.6).set_friction(0.14).set_restitution(0.35),
        &bed,
    );

    let rails = [
        (-4.35, 0.45, 0.0, 0.25, 0.45, 7.6),
        (4.35, 0.45, 0.0, 0.25, 0.45, 7.6),
        (0.0, 0.45, -7.35, 4.6, 0.45, 0.25),
        (0.0, 0.45, 7.35, 4.6, 0.45, 0.25),
    ];
    for (rx, ry, rz, hx, hy, hz) in rails {
        let rb = world.create_rigid_body(&rp::RigidBodyDesc::fixed().set_translation(rx, ry, rz));
        world.create_collider(
            &rp::ColliderDesc::cuboid(hx, hy, hz).set_friction(0.12).set_restitution(0.88),
            &rb,
        );
    }

    st.balls.clear();
    for row in 0..4 {
        for col in 0..=row {
            let x = ((col as f64) - (row as f64) * 0.5) * 0.74;
            let z = -1.6 - (row as f64) * 0.645;
            let rb = world.create_rigid_body(
                &rp::RigidBodyDesc::dynamic()
                    .set_translation(x, BALL_RADIUS, z)
                    .set_linear_damping(0.30)
                    .set_angular_damping(0.30),
            );
            world.create_collider(
                &rp::ColliderDesc::ball(BALL_RADIUS)
                    .set_restitution(0.92)
                    .set_friction(0.10)
                    .set_density(1.0),
                &rb,
            );
            st.balls.push(rb);
        }
    }

    let mut cue_desc = rp::RigidBodyDesc::dynamic()
        .set_translation(0.04, BALL_RADIUS, 4.4)
        .set_linear_damping(0.22)
        .set_angular_damping(0.22);
    if shoot_cue {
        cue_desc = cue_desc.set_linvel(-0.18, 0.0, -16.0);
    }
    let cue = world.create_rigid_body(&cue_desc);
    world.create_collider(
        &rp::ColliderDesc::ball(BALL_RADIUS)
            .set_restitution(0.94)
            .set_friction(0.10)
            .set_density(1.25),
        &cue,
    );
    st.balls.push(cue);

    st.world = Some(world);
    st.step_count = 0;
}

fn sync_meshes(st: &AppState) {
    for (rb, mesh) in st.balls.iter().zip(st.ball_meshes.iter()) {
        let p = rb.translation(None);
        let q = rb.rotation(None);
        mesh.position().set(p.x(), p.y(), p.z());
        mesh.rotation_quaternion().set(q.x(), q.y(), q.z(), q.w());
    }
}

fn update_meta(st: &AppState) {
    let mut moving = 0usize;
    let mut ke = 0.0_f64;
    for rb in &st.balls {
        let v = rb.linvel(None);
        let s2 = v.x() * v.x() + v.y() * v.y() + v.z() * v.z();
        if s2 > 0.05 {
            moving += 1;
        }
        ke += 0.5 * s2;
    }
    let state_text = if st.paused { "paused" } else { "running" };
    let text = format!(
        "11 spheres · moving={moving} · ke={ke:.1} · step={} ({state_text})",
        st.step_count
    );
    st.meta.set_text_content(&text);
}

fn step_sim(st: &mut AppState) {
    if let Some(ref w) = st.world {
        w.step(None, None);
    }
    st.step_count += 1;
    sync_meshes(st);
    update_meta(st);
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    rp::js::spawn(async {
        if rp::init().await.is_err() {
            return;
        }
        let doc = dom::document();
        let st = Rc::new(RefCell::new(AppState {
            meta: doc.get_element_by_id("meta").unwrap(),
            pause_btn: doc.get_element_by_id("btn-pause").unwrap(),
            world: None,
            balls: Vec::new(),
            ball_meshes: Vec::new(),
            paused: false,
            step_count: 0,
            _render_cb: None,
        }));

        {
            let mut s = st.borrow_mut();
            build_world(&mut s, true);
            for _ in 0..26 {
                if let Some(ref w) = s.world {
                    w.step(None, None);
                }
                s.step_count += 1;
            }
            update_meta(&s);
        }

        for (i, bid) in ["btn-break", "btn-scatter", "btn-pause", "btn-reset"].iter().enumerate() {
            let btn = doc.get_element_by_id(bid).unwrap();
            let st_c = st.clone();
            let cb = dom::JsListener::new(move |_: dom::PointerEvent| {
                let mut s = st_c.borrow_mut();
                match i {
                    0 => {
                        s.paused = false;
                        s.pause_btn.set_text_content("Pause");
                        build_world(&mut s, true);
                        sync_meshes(&s);
                        update_meta(&s);
                    }
                    1 => {
                        for (idx, rb) in s.balls.iter().enumerate() {
                            let ang = (idx as f64) * 1.15;
                            rb.apply_impulse(&rp::Vector3::new(ang.cos() * 1.4, 0.25, ang.sin() * 1.4), true);
                        }
                        update_meta(&s);
                    }
                    2 => {
                        s.paused = !s.paused;
                        s.pause_btn.set_text_content(if s.paused {
                            "Resume"
                        } else {
                            "Pause"
                        });
                        update_meta(&s);
                    }
                    _ => {
                        s.paused = false;
                        s.pause_btn.set_text_content("Pause");
                        build_world(&mut s, false);
                        sync_meshes(&s);
                        update_meta(&s);
                    }
                }
            });
            btn.add_event_listener(dom::HTMLElementEventMap::CLICK, &cb, None);
        }

        let Some(canvas) = doc.query_selector_as::<HTMLCanvasElement>("canvas") else {
            set_status_error("missing <canvas> element");
            return;
        };
        let engine = match b::create_engine(&canvas, None).await {
            Ok(e) => e,
            Err(e) => {
                set_status_error(&e.message());
                return;
            }
        };

        let scene = b::create_scene_context(&engine, None);
        let clear = GPUColorDict::default().with_r(0.05).with_g(0.07).with_b(0.09).with_a(1.0);
        scene.set_clear_color(&clear);

        let camera = b::create_arc_rotate_camera(1.18, 0.92, 13.5, &b::vec3(0.0, 0.3, -0.4));
        b::add_to_scene(&scene, &camera);
        b::attach_control(&camera, &canvas, &scene, None);

        let hemi = b::create_hemispheric_light(&b::JsTuple3::from([0.0, 1.0, 0.2]), 0.72);
        hemi.set_diffuse_color([0.85, 0.9, 0.95]);
        hemi.set_ground_color([0.12, 0.14, 0.16]);
        b::add_to_scene(&scene, &hemi);

        let dir = b::create_directional_light([-1.0, -2.0, -1.0], 2.1);
        dir.set_diffuse([1.0, 0.97, 0.92]);
        b::add_to_scene(&scene, &dir);

        let mk_mat = |r: f64, g: f64, bl: f64, m: f64, ro: f64| {
            let props = b::CreatePbrMaterialProps::default()
                .with_base_color_factor([r, g, bl, 1.0])
                .with_metallic_factor(m)
                .with_roughness_factor(ro);
            b::create_pbr_material(&props)
        };

        let table = b::create_box(
            &engine,
            &b::BoxOptions::default().with_width(9.2).with_height(0.5).with_depth(15.2),
        );
        table.position().set(0.0, -0.25, 0.0);
        table.set_material(&mk_mat(0.08, 0.24, 0.18, 0.05, 0.78));
        b::add_to_scene(&scene, &table);

        let rail_mat = mk_mat(0.18, 0.18, 0.20, 0.35, 0.42);
        for (rx, ry, rz, wr, hr, dr) in [
            (-4.35, 0.45, 0.0, 0.5, 0.9, 15.2),
            (4.35, 0.45, 0.0, 0.5, 0.9, 15.2),
            (0.0, 0.45, -7.35, 9.2, 0.9, 0.5),
            (0.0, 0.45, 7.35, 9.2, 0.9, 0.5),
        ] {
            let r = b::create_box(
                &engine,
                &b::BoxOptions::default().with_width(wr).with_height(hr).with_depth(dr),
            );
            r.position().set(rx, ry, rz);
            r.set_material(&rail_mat);
            b::add_to_scene(&scene, &r);
        }

        let cue_mat = mk_mat(0.96, 0.96, 0.95, 0.15, 0.18);
        let rack_mats = [
            mk_mat(0.85, 0.24, 0.22, 0.2, 0.22),
            mk_mat(0.22, 0.48, 0.85, 0.2, 0.22),
            mk_mat(0.92, 0.68, 0.18, 0.2, 0.22),
            mk_mat(0.20, 0.20, 0.22, 0.25, 0.2),
        ];

        {
            let mut s = st.borrow_mut();
            let sopts = b::SphereOptions::default().with_diameter(BALL_RADIUS * 2.0).with_segments(24.0);
            for i in 0..BALL_COUNT {
                let m = b::create_sphere(&engine, &sopts);
                m.set_material(if i == RACK_COUNT { &cue_mat } else { &rack_mats[i % 4] });
                b::add_to_scene(&scene, &m);
                s.ball_meshes.push(m);
            }
            sync_meshes(&s);

            let st_c = st.clone();
            let render_cb = b::NumberFn::new(move |_| {
                let mut s = st_c.borrow_mut();
                if !s.paused {
                    step_sim(&mut s);
                }
            });
            b::on_before_render(&scene, &render_cb);
            s._render_cb = Some(render_cb);
        }

        if let Err(e) = b::register_scene(&scene).await {
            set_status_error(&e.message());
            return;
        }
        if let Err(e) = b::start_engine(&engine).await {
            set_status_error(&e.message());
        }
    });
}
