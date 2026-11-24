#![no_main]

use babylonjs_lite::{
    BoxOptions, CreatePbrMaterialProps, GroundOptions, JsTuple3, NumberFn, PbrMaterialProps,
    SphereOptions,
};
use dom::{GPUColorDict, HTMLCanvasElement};

fn set_status_error(msg: &str) {
    if let Some(el) = dom::document().get_element_by_id("status") {
        let text = format!("WebGPU / Babylon Lite error: {msg}");
        el.set_text_content(&text);
    }
}

fn mk_pbr(v: f64, metallic: f64, roughness: f64) -> PbrMaterialProps {
    let props = CreatePbrMaterialProps::default()
        .with_base_color_factor([v, v, v, 1.0])
        .with_metallic_factor(metallic)
        .with_roughness_factor(roughness);
    babylonjs_lite::create_pbr_material_with(&props)
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
            .with_r(0.06)
            .with_g(0.06)
            .with_b(0.06)
            .with_a(1.0);
        scene.set_clear_color(&clear);

        let camera = babylonjs_lite::create_arc_rotate_camera(
            core::f64::consts::FRAC_PI_4,
            core::f64::consts::PI / 3.1,
            12.0,
            &babylonjs_lite::vec3(0.0, 1.1, 0.0),
        );
        babylonjs_lite::add_to_scene(&scene, &camera);
        babylonjs_lite::attach_control_with(&camera, &canvas, &scene, None);

        let hemi =
            babylonjs_lite::create_hemispheric_light_with(&JsTuple3::from([0.0, 1.0, 0.2]), 0.65);
        hemi.set_diffuse_color([0.82, 0.82, 0.82]);
        hemi.set_ground_color([0.12, 0.12, 0.12]);
        babylonjs_lite::add_to_scene(&scene, &hemi);

        let dir = babylonjs_lite::create_directional_light_with([-1.0, -2.0, -1.0], 1.8);
        dir.set_diffuse([0.95, 0.95, 0.95]);
        babylonjs_lite::add_to_scene(&scene, &dir);

        let orb_light = babylonjs_lite::create_point_light_with([4.5, 3.0, 0.0], 12.0);
        orb_light.set_diffuse([1.0, 1.0, 1.0]);
        orb_light.set_range(18.0);
        babylonjs_lite::add_to_scene(&scene, &orb_light);

        let floor_opts = GroundOptions::default().with_width(14.0).with_height(14.0);
        let floor = babylonjs_lite::create_ground_with(&engine, &floor_opts);
        floor.set_material(&mk_pbr(0.11, 0.08, 0.88));
        babylonjs_lite::add_to_scene(&scene, &floor);

        let plinth_opts = BoxOptions::default()
            .with_width(8.4)
            .with_height(0.08)
            .with_depth(8.4);
        let plinth = babylonjs_lite::create_box_with(&engine, &plinth_opts);
        plinth.position().set(0.0, 0.04, 0.0);
        plinth.set_material(&mk_pbr(0.17, 0.12, 0.78));
        babylonjs_lite::add_to_scene(&scene, &plinth);

        let sopts = SphereOptions::default()
            .with_diameter(1.05)
            .with_segments(24.0);
        for row in 0..5 {
            for col in 0..5 {
                let roughness = 0.08 + 0.22 * (row as f64);
                let metallic = 0.25 * (col as f64);
                let sphere = babylonjs_lite::create_sphere_with(&engine, &sopts);
                sphere
                    .position()
                    .set(((col - 2) as f64) * 1.55, 0.75, ((row - 2) as f64) * 1.55);
                sphere.set_material(&mk_pbr(0.85, metallic, roughness));
                babylonjs_lite::add_to_scene(&scene, &sphere);
            }
        }

        let orb_props = CreatePbrMaterialProps::default()
            .with_base_color_factor([0.95, 0.95, 0.95, 1.0])
            .with_metallic_factor(0.0)
            .with_roughness_factor(0.2);
        babylonjs_lite::set_pbr_emissive(&orb_props, [0.95, 0.95, 0.95]);
        let orb_mat = babylonjs_lite::create_pbr_material_with(&orb_props);

        let orb_opts = SphereOptions::default()
            .with_diameter(0.36)
            .with_segments(16.0);
        let orb = babylonjs_lite::create_sphere_with(&engine, &orb_opts);
        orb.position().set(4.5, 3.0, 0.0);
        orb.set_material(&orb_mat);
        babylonjs_lite::add_to_scene(&scene, &orb);

        let orb_pos = orb.position();
        let orb_light_pos = orb_light.position();
        let mut elapsed = 0.0;

        let render_cb = NumberFn::new(move |delta_ms| {
            elapsed += delta_ms * 0.001;
            let x = (elapsed * 0.85).cos() * 4.5;
            let y = 3.0 + (elapsed * 1.4).sin() * 0.65;
            let z = (elapsed * 0.85).sin() * 4.5;
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
