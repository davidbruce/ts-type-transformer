#![no_main]

use std::cell::RefCell;
use std::rc::Rc;

use babylonjs_lite as b;
use dimforge_rapier3d_compat as rp;
use dom::{GPUColorDict, HTMLCanvasElement, HTMLElement};

const LAYERS: usize = 6;
const PER_LAYER: usize = 3;
const PLANK_COUNT: usize = LAYERS * PER_LAYER;
const HALF_LONG: f64 = 0.85;
const HALF_HEIGHT: f64 = 0.22;
const HALF_THIN: f64 = 0.26;
const SPHERE_RADIUS: f64 = 0.65;

struct AppState {
    meta: HTMLElement,
    pause_btn: HTMLElement,
    world: Option<rp::World>,
    planks: Vec<rp::RigidBody>,
    striker: Option<rp::RigidBody>,
    plank_meshes: Vec<b::Mesh>,
    striker_mesh: Option<b::Mesh>,
    paused: bool,
    step_count: usize,
    shot_idx: usize,
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

    st.planks.clear();
    for idx in 0..PLANK_COUNT {
        let layer = idx / PER_LAYER;
        let slot = idx % PER_LAYER;
        let offset = ((slot as f64) - 1.0) * 0.57;
        let y = HALF_HEIGHT + (layer as f64) * 0.445;
        let (x, z, hx, hz) = if layer % 2 == 0 {
            (0.0, offset, HALF_LONG, HALF_THIN)
        } else {
            (offset, 0.0, HALF_THIN, HALF_LONG)
        };
        let rb = world.create_rigid_body(&rp::RigidBodyDesc::dynamic().set_translation_2(x, y, z));
        let cd = rp::ColliderDesc::cuboid(hx, HALF_HEIGHT, hz)
            .set_friction_2(0.65)
            .set_restitution_2(0.12)
            .set_density_2(1.0);
        world.create_collider(&cd, &rb);
        st.planks.push(rb);
    }

    let striker = world.create_rigid_body(
        &rp::RigidBodyDesc::dynamic()
            .set_translation_2(-6.2, 1.55, 0.12)
            .set_linvel_2(13.5, 1.1, -0.25),
    );
    world.create_collider(
        &rp::ColliderDesc::ball(SPHERE_RADIUS)
            .set_density_2(4.5)
            .set_restitution_2(0.35)
            .set_friction_2(0.3),
        &striker,
    );

    st.striker = Some(striker);
    st.world = Some(world);
    st.step_count = 0;
}

fn sync_meshes(st: &AppState) {
    for (rb, mesh) in st.planks.iter().zip(st.plank_meshes.iter()) {
        let p = rb.translation(None);
        let q = rb.rotation(None);
        mesh.position().set(p.x(), p.y(), p.z());
        mesh.rotation_quaternion().set(q.x(), q.y(), q.z(), q.w());
    }
    if let (Some(rb), Some(mesh)) = (&st.striker, &st.striker_mesh) {
        let p = rb.translation(None);
        let q = rb.rotation(None);
        mesh.position().set(p.x(), p.y(), p.z());
        mesh.rotation_quaternion().set(q.x(), q.y(), q.z(), q.w());
    }
}

fn update_meta(st: &AppState) {
    let mut max_y = 0.0_f64;
    let mut ke = 0.0_f64;
    for rb in &st.planks {
        let p = rb.translation(None);
        let v = rb.linvel(None);
        if p.y() > max_y {
            max_y = p.y();
        }
        ke += 0.5 * (v.x() * v.x() + v.y() * v.y() + v.z() * v.z());
    }
    if let Some(ref rb) = st.striker {
        let v = rb.linvel(None);
        ke += 2.25 * (v.x() * v.x() + v.y() * v.y() + v.z() * v.z());
    }
    let state_text = if st.paused { "paused" } else { "running" };
    let text = format!(
        "19 bodies (18 planks + sphere) · max_y={max_y:.2} m · ke={ke:.1} · step={} ({state_text})",
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
            planks: Vec::new(),
            striker: None,
            plank_meshes: Vec::new(),
            striker_mesh: None,
            paused: false,
            step_count: 0,
            shot_idx: 0,
            _render_cb: None,
        }));

        {
            let mut s = st.borrow_mut();
            build_world(&mut s);
            for _ in 0..18 {
                if let Some(ref w) = s.world {
                    w.step(None, None);
                }
                s.step_count += 1;
            }
            update_meta(&s);
        }

        for (i, bid) in ["btn-launch", "btn-pause", "btn-reset"].iter().enumerate() {
            let btn = doc.get_element_by_id(bid).unwrap();
            let st_c = st.clone();
            let cb = dom::JsListener::new(move |_: dom::PointerEvent| {
                let mut s = st_c.borrow_mut();
                match i {
                    0 => {
                        s.shot_idx += 1;
                        let angle = (s.shot_idx as f64) * 0.9;
                        let dist = 6.2;
                        let sx = -dist * angle.cos();
                        let sz = dist * angle.sin();
                        let sy = 1.2 + ((s.shot_idx % 3) as f64) * 0.45;
                        if let Some(ref striker) = s.striker {
                            striker.set_translation(&rp::Vector3::new(sx, sy, sz), true);
                            striker.set_linvel(
                                &rp::Vector3::new(14.4 * angle.cos(), 0.9, -14.4 * angle.sin()),
                                true,
                            );
                            striker.set_angvel(&rp::Vector3::new(0.0, 0.0, 0.0), true);
                        }
                        sync_meshes(&s);
                        update_meta(&s);
                    }
                    1 => {
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
                        for _ in 0..12 {
                            if let Some(ref w) = s.world {
                                w.step(None, None);
                            }
                            s.step_count += 1;
                        }
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

        let camera = b::create_arc_rotate_camera(0.85, 1.12, 11.5, &b::vec3(0.0, 1.5, 0.0));
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
        let palette = [
            mk_mat(0.86, 0.86, 0.86, 0.15, 0.45),
            mk_mat(0.58, 0.58, 0.60, 0.25, 0.38),
            mk_mat(0.30, 0.31, 0.33, 0.35, 0.35),
        ];

        {
            let mut s = st.borrow_mut();
            for idx in 0..PLANK_COUNT {
                let layer = idx / PER_LAYER;
                let (w_box, d_box) = if layer % 2 == 0 {
                    (HALF_LONG * 2.0, HALF_THIN * 2.0)
                } else {
                    (HALF_THIN * 2.0, HALF_LONG * 2.0)
                };
                let m = b::create_box(
                    &engine,
                    &b::BoxOptions::default()
                        .with_width(w_box)
                        .with_height(HALF_HEIGHT * 2.0)
                        .with_depth(d_box),
                );
                m.set_material(&palette[layer % 3]);
                b::add_to_scene(&scene, &m);
                s.plank_meshes.push(m);
            }

            let sm = b::create_sphere(
                &engine,
                &b::SphereOptions::default().with_diameter(SPHERE_RADIUS * 2.0).with_segments(24.0),
            );
            sm.set_material(&mk_mat(0.95, 0.42, 0.28, 0.55, 0.22));
            b::add_to_scene(&scene, &sm);
            s.striker_mesh = Some(sm);
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
