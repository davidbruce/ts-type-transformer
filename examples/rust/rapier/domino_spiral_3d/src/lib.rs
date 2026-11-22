#![no_main]

use std::cell::RefCell;
use std::rc::Rc;

use babylonjs_lite as b;
use dimforge_rapier3d_compat as rp;
use dom::{GPUColorDict, HTMLCanvasElement, HTMLElement};

const DOM_COUNT: usize = 14;
const CYL_COUNT: usize = 3;
const DOM_HX: f64 = 0.09;
const DOM_HY: f64 = 0.65;
const DOM_HZ: f64 = 0.34;
const MARBLE_RADIUS: f64 = 0.36;
const RAMP_ANGLE: f64 = -0.38;
const RAMP_X: f64 = -3.55;
const RAMP_Y: f64 = 1.02;
const RAMP_Z: f64 = -2.5;

fn spiral_pose(i: usize) -> (f64, f64, f64, f64) {
    let t = (i as f64) * 0.255;
    let r = 2.6 - (i as f64) * 0.045;
    let x = -1.35 + r * t.sin();
    let z = -2.5 + 2.6 - r * t.cos();
    (x, DOM_HY, z, -t)
}

struct AppState {
    meta: HTMLElement,
    pause_btn: HTMLElement,
    world: Option<rp::World>,
    marble: Option<rp::RigidBody>,
    doms: Vec<rp::RigidBody>,
    cyls: Vec<rp::RigidBody>,
    marble_mesh: Option<b::Mesh>,
    dom_meshes: Vec<b::Mesh>,
    cyl_meshes: Vec<b::Mesh>,
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

fn build_world(st: &mut AppState) {
    if let Some(w) = st.world.take() {
        w.free();
    }
    let world = rp::World::new(
        &rp::Vector3::new(0.0, -9.81, 0.0),
        None, None, None, None, None, None, None, None, None, None, None, None, None,
    );

    let floor_body = world.create_rigid_body(&rp::RigidBodyDesc::fixed().set_translation_2(0.0, -0.3, 0.0));
    world.create_collider(
        &rp::ColliderDesc::cuboid(10.0, 0.3, 10.0).set_friction_2(0.75).set_restitution_2(0.1),
        &floor_body,
    );

    let ramp_rot = rp::Quaternion::new(0.0, 0.0, (RAMP_ANGLE * 0.5).sin(), (RAMP_ANGLE * 0.5).cos());
    let ramp_body = world.create_rigid_body(
        &rp::RigidBodyDesc::fixed()
            .set_translation_2(RAMP_X, RAMP_Y, RAMP_Z)
            .set_rotation_2(&ramp_rot),
    );
    world.create_collider(
        &rp::ColliderDesc::cuboid(1.75, 0.10, 0.65).set_friction_2(0.15).set_restitution_2(0.2),
        &ramp_body,
    );

    let marble = world.create_rigid_body(
        &rp::RigidBodyDesc::dynamic()
            .set_translation_2(-4.75, 2.02, -2.5)
            .set_linvel_2(3.0, 0.0, 0.0),
    );
    world.create_collider(
        &rp::ColliderDesc::ball(MARBLE_RADIUS)
            .set_density_2(3.2)
            .set_restitution_2(0.32)
            .set_friction_2(0.18),
        &marble,
    );
    st.marble = Some(marble);

    st.doms.clear();
    for i in 0..DOM_COUNT {
        let (x, y, z, yaw) = spiral_pose(i);
        let q = rp::Quaternion::new(0.0, (yaw * 0.5).sin(), 0.0, (yaw * 0.5).cos());
        let rb = world.create_rigid_body(
            &rp::RigidBodyDesc::dynamic().set_translation_2(x, y, z).set_rotation_2(&q),
        );
        world.create_collider(
            &rp::ColliderDesc::cuboid(DOM_HX, DOM_HY, DOM_HZ)
                .set_friction_2(0.55)
                .set_restitution_2(0.18)
                .set_density_2(1.0),
            &rb,
        );
        st.doms.push(rb);
    }

    st.cyls.clear();
    let (end_x, _, end_z, end_yaw) = spiral_pose(DOM_COUNT - 1);
    let fwd_x = (-end_yaw).cos();
    let fwd_z = (-end_yaw).sin();
    let perp_x = -fwd_z;
    let perp_z = fwd_x;
    for j in 0..CYL_COUNT {
        let spread = ((j as f64) - 1.0) * 0.52;
        let cx = end_x + fwd_x * 0.72 + perp_x * spread;
        let cz = end_z + fwd_z * 0.72 + perp_z * spread;
        let rb = world.create_rigid_body(&rp::RigidBodyDesc::dynamic().set_translation_2(cx, 0.42, cz));
        world.create_collider(
            &rp::ColliderDesc::cylinder(0.42, 0.25)
                .set_friction_2(0.55)
                .set_restitution_2(0.25)
                .set_density_2(1.0),
            &rb,
        );
        st.cyls.push(rb);
    }

    st.world = Some(world);
    st.step_count = 0;
}

fn sync_pair(rb: &rp::RigidBody, mesh: &b::Mesh) {
    let p = rb.translation(None);
    let q = rb.rotation(None);
    mesh.position().set(p.x(), p.y(), p.z());
    mesh.rotation_quaternion().set(q.x(), q.y(), q.z(), q.w());
}

fn sync_meshes(st: &AppState) {
    if let (Some(rb), Some(mesh)) = (&st.marble, &st.marble_mesh) {
        sync_pair(rb, mesh);
    }
    for (rb, mesh) in st.doms.iter().zip(st.dom_meshes.iter()) {
        sync_pair(rb, mesh);
    }
    for (rb, mesh) in st.cyls.iter().zip(st.cyl_meshes.iter()) {
        sync_pair(rb, mesh);
    }
}

fn update_meta(st: &AppState) {
    let mut fallen = 0usize;
    let mut ke = 0.0_f64;
    if let Some(ref mb) = st.marble {
        let v = mb.linvel(None);
        ke += 0.5 * (v.x() * v.x() + v.y() * v.y() + v.z() * v.z());
    }
    for rb in &st.doms {
        let p = rb.translation(None);
        let v = rb.linvel(None);
        if p.y() < 0.48 {
            fallen += 1;
        }
        ke += 0.5 * (v.x() * v.x() + v.y() * v.y() + v.z() * v.z());
    }
    for rb in &st.cyls {
        let v = rb.linvel(None);
        ke += 0.5 * (v.x() * v.x() + v.y() * v.y() + v.z() * v.z());
    }
    let state_text = if st.paused { "paused" } else { "running" };
    let text = format!(
        "18 bodies (marble + 14 dominoes + 3 cylinders) · fallen={fallen} · ke={ke:.1} · step={} ({state_text})",
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
            marble: None,
            doms: Vec::new(),
            cyls: Vec::new(),
            marble_mesh: None,
            dom_meshes: Vec::new(),
            cyl_meshes: Vec::new(),
            paused: false,
            step_count: 0,
            _render_cb: None,
        }));

        {
            let mut s = st.borrow_mut();
            build_world(&mut s);
            for _ in 0..20 {
                if let Some(ref w) = s.world {
                    w.step(None, None);
                }
                s.step_count += 1;
            }
            update_meta(&s);
        }

        for (i, bid) in ["btn-launch", "btn-nudge", "btn-pause", "btn-reset"].iter().enumerate() {
            let btn = doc.get_element_by_id(bid).unwrap();
            let st_c = st.clone();
            let cb = dom::JsListener::new(move |_: dom::PointerEvent| {
                let mut s = st_c.borrow_mut();
                match i {
                    0 => {
                        if let Some(ref mb) = s.marble {
                            mb.set_translation(&rp::Vector3::new(-4.75, 2.02, -2.5), true);
                            mb.set_linvel(&rp::Vector3::new(3.0, 0.0, 0.0), true);
                            mb.set_angvel(&rp::Vector3::new(0.0, 0.0, 0.0), true);
                        }
                        sync_meshes(&s);
                        update_meta(&s);
                    }
                    1 => {
                        if let Some(first) = s.doms.first() {
                            first.apply_impulse(&rp::Vector3::new(1.6, 0.0, 0.0), true);
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
                        build_world(&mut s);
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
        let engine = match b::create_engine_with_engine_options(&canvas, None).await {
            Ok(e) => e,
            Err(e) => {
                set_status_error(&e.message());
                return;
            }
        };

        let scene = b::create_scene_context(&engine, None);
        let clear = GPUColorDict::default().with_r(0.06).with_g(0.07).with_b(0.09).with_a(1.0);
        scene.set_clear_color(&clear);

        let camera = b::create_arc_rotate_camera(0.65, 1.05, 12.0, &b::vec3(-0.4, 0.8, 0.0));
        b::add_to_scene(&scene, &camera);
        b::attach_control(&camera, &canvas, &scene, None);

        let hemi = b::create_hemispheric_light(&b::JsTuple3::from([0.0, 1.0, 0.2]), 0.72);
        hemi.set_diffuse_color([0.85, 0.88, 0.95]);
        hemi.set_ground_color([0.14, 0.14, 0.16]);
        b::add_to_scene(&scene, &hemi);

        let dir = b::create_directional_light([-1.0, -2.0, -1.2], 2.1);
        dir.set_diffuse([1.0, 0.97, 0.92]);
        b::add_to_scene(&scene, &dir);

        let ground_mat = b::create_grid_material(None);
        let ground = b::create_ground(&engine, &b::GroundOptions::default().with_width(20.0).with_height(20.0));
        ground.set_material(&ground_mat);
        b::add_to_scene(&scene, &ground);

        let mk_mat = |r: f64, g: f64, bl: f64, m: f64, ro: f64| {
            let props = b::CreatePbrMaterialProps::default()
                .with_base_color_factor([r, g, bl, 1.0])
                .with_metallic_factor(m)
                .with_roughness_factor(ro);
            b::create_pbr_material(&props)
        };

        let ramp_mesh = b::create_box(
            &engine,
            &b::BoxOptions::default().with_width(3.5).with_height(0.2).with_depth(1.3),
        );
        ramp_mesh.position().set(RAMP_X, RAMP_Y, RAMP_Z);
        ramp_mesh
            .rotation_quaternion()
            .set(0.0, 0.0, (RAMP_ANGLE * 0.5).sin(), (RAMP_ANGLE * 0.5).cos());
        ramp_mesh.set_material(&mk_mat(0.28, 0.30, 0.34, 0.35, 0.48));
        b::add_to_scene(&scene, &ramp_mesh);

        {
            let mut s = st.borrow_mut();
            let mb = b::create_sphere(
                &engine,
                &b::SphereOptions::default().with_diameter(MARBLE_RADIUS * 2.0).with_segments(24.0),
            );
            mb.set_material(&mk_mat(0.95, 0.42, 0.28, 0.55, 0.2));
            b::add_to_scene(&scene, &mb);
            s.marble_mesh = Some(mb);

            let mat_a = mk_mat(0.88, 0.88, 0.88, 0.15, 0.4);
            let mat_b = mk_mat(0.45, 0.46, 0.48, 0.25, 0.38);
            let dom_opts = b::BoxOptions::default()
                .with_width(DOM_HX * 2.0)
                .with_height(DOM_HY * 2.0)
                .with_depth(DOM_HZ * 2.0);
            for i in 0..DOM_COUNT {
                let m = b::create_box(&engine, &dom_opts);
                m.set_material(if i % 2 == 0 { &mat_a } else { &mat_b });
                b::add_to_scene(&scene, &m);
                s.dom_meshes.push(m);
            }

            let cyl_mat = mk_mat(0.26, 0.62, 0.88, 0.45, 0.25);
            let cyl_opts = b::CylinderOptions::default()
                .with_height(0.84)
                .with_diameter(0.5)
                .with_tessellation(24.0);
            for _ in 0..CYL_COUNT {
                let m = b::create_cylinder(&engine, &cyl_opts);
                m.set_material(&cyl_mat);
                b::add_to_scene(&scene, &m);
                s.cyl_meshes.push(m);
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
