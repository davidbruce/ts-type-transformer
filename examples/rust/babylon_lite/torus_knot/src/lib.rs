#![no_main]

use babylonjs_lite::{
    CreatePbrMaterialProps, GroundOptions, JsTuple3, NumberFn, SphereOptions, TorusKnotOptions,
};
use dom::{GPUColorDict, HTMLCanvasElement};

fn set_status_error(msg: &str) {
    if let Some(el) = dom::document().get_element_by_id("status") {
        let text = format!("WebGPU / Babylon Lite error: {msg}");
        el.set_text_content(&text);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    babylonjs_lite::js::spawn(async {
        let Some(canvas) = dom::document().query_selector_as::<HTMLCanvasElement>("canvas") else {
            set_status_error("missing <canvas> element");
            return;
        };

        let engine = match babylonjs_lite::create_engine(&canvas).await {
            Ok(e) => e,
            Err(e) => {
                set_status_error(&e.message());
                return;
            }
        };

        let scene = babylonjs_lite::create_scene_context(&engine);
        let clear = GPUColorDict::default()
            .with_r(0.05)
            .with_g(0.07)
            .with_b(0.11)
            .with_a(1.0);
        scene.set_clear_color(&clear);

        let camera = babylonjs_lite::create_arc_rotate_camera(
            core::f64::consts::FRAC_PI_4,
            core::f64::consts::FRAC_PI_3,
            8.5,
            &babylonjs_lite::vec3(0.0, 1.6, 0.0),
        );
        babylonjs_lite::add_to_scene(&scene, &camera);
        babylonjs_lite::attach_control_with(&camera, &canvas, &scene, None);

        let hemi =
            babylonjs_lite::create_hemispheric_light_with(&JsTuple3::from([0.0, 1.0, 0.2]), 0.75);
        hemi.set_diffuse_color([0.75, 0.85, 1.0]);
        hemi.set_ground_color([0.12, 0.14, 0.22]);
        babylonjs_lite::add_to_scene(&scene, &hemi);

        let dir = babylonjs_lite::create_directional_light_with([-1.0, -2.0, -1.2], 2.2);
        dir.set_diffuse([1.0, 0.96, 0.9]);
        babylonjs_lite::add_to_scene(&scene, &dir);

        let orb_light = babylonjs_lite::create_point_light_with([2.8, 1.6, 0.0], 8.0);
        orb_light.set_diffuse([1.0, 0.45, 0.35]);
        orb_light.set_range(12.0);
        babylonjs_lite::add_to_scene(&scene, &orb_light);

        let knot_props = CreatePbrMaterialProps::default()
            .with_base_color_factor([0.18, 0.78, 0.95, 1.0])
            .with_metallic_factor(0.75)
            .with_roughness_factor(0.22);
        let knot_mat = babylonjs_lite::create_pbr_material_with(&knot_props);

        let knot_opts = TorusKnotOptions::default()
            .with_radius(1.15)
            .with_tube(0.34)
            .with_radial_segments(48.0)
            .with_tubular_segments(160.0);
        let knot = babylonjs_lite::create_torus_knot_with(&engine, &knot_opts);
        knot.position().set(0.0, 2.0, 0.0);
        knot.set_material(&knot_mat);
        babylonjs_lite::add_to_scene(&scene, &knot);

        let orb_props = CreatePbrMaterialProps::default()
            .with_base_color_factor([1.0, 0.42, 0.32, 1.0])
            .with_metallic_factor(0.2)
            .with_roughness_factor(0.35);
        let orb_mat = babylonjs_lite::create_pbr_material_with(&orb_props);

        let orb_opts = SphereOptions::default()
            .with_diameter(0.55)
            .with_segments(32.0);
        let orb = babylonjs_lite::create_sphere_with(&engine, &orb_opts);
        orb.set_material(&orb_mat);
        babylonjs_lite::add_to_scene(&scene, &orb);

        let ground_mat = babylonjs_lite::create_grid_material();
        let ground_opts = GroundOptions::default().with_width(16.0).with_height(16.0);
        let ground = babylonjs_lite::create_ground_with(&engine, &ground_opts);
        ground.set_material(&ground_mat);
        babylonjs_lite::add_to_scene(&scene, &ground);

        let knot_rot = knot.rotation();
        let orb_pos = orb.position();
        let orb_light_pos = orb_light.position();
        let mut elapsed = 0.0;

        let render_cb = NumberFn::new(move |delta_ms| {
            elapsed += delta_ms * 0.001;
            knot_rot.set_y(elapsed * 0.55);
            knot_rot.set_x((elapsed * 0.4).sin() * 0.25);
            let x = (elapsed * 0.9).cos() * 2.8;
            let y = 2.0 + (elapsed * 1.8).sin() * 0.45;
            let z = (elapsed * 0.9).sin() * 2.8;
            orb_pos.set(x, y, z);
            orb_light_pos.set(x, y, z);
        });
        babylonjs_lite::on_before_render(&scene, &render_cb);

        if let Err(e) = babylonjs_lite::register_scene(&scene).await {
            set_status_error(&e.message());
            return;
        }
        if let Err(e) = babylonjs_lite::start_engine(&engine).await {
            set_status_error(&e.message());
        }
    });
}
