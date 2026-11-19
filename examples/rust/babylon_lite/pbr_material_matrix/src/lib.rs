#![no_main]

use babylonjs_lite::{
    BoxOptions, BoxOptionsMethods as _, CreatePbrMaterialProps, CreatePbrMaterialPropsMethods as _,
    DirectionalLightMethods as _, GroundOptions, GroundOptionsMethods as _,
    HemisphericLightMethods as _, JsTuple3, JsTuple4, MeshMethods as _, NumberFn,
    ObservableVec3Methods as _, PbrMaterialProps, PointLightMethods as _, SceneNodeMethods as _,
    SphereOptions, SphereOptionsMethods as _,
};
use dom::{GPUColorDict, GPUColorDictMethods as _, HTMLCanvasElement, ParentNodeMethods as _};

fn set_status_error(msg: &str) {
    if let Some(el) = dom::document().get_element_by_id("status") {
        let text = format!("WebGPU / Babylon Lite error: {msg}");
        el.set_text_content(Some(&text));
    }
}

fn mk_pbr(v: f64, metallic: f64, roughness: f64) -> PbrMaterialProps {
    let props = CreatePbrMaterialProps::create();
    props.set_base_color_factor(Some(&JsTuple4::new(v, v, v, 1.0)));
    props.set_metallic_factor(Some(metallic));
    props.set_roughness_factor(Some(roughness));
    babylonjs_lite::create_pbr_material(Some(&props))
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    babylonjs_lite::js::spawn(async {
        let Some(canvas) = dom::document().query_selector_as::<HTMLCanvasElement>("canvas") else {
            set_status_error("missing <canvas> element");
            return;
        };

        let engine = match babylonjs_lite::create_engine_with_engine_options(&canvas, None).await {
            Ok(e) => e,
            Err(e) => {
                set_status_error(&e.message());
                return;
            }
        };

        let scene = babylonjs_lite::create_scene_context(&engine, None);
        let clear = GPUColorDict::create();
        clear.set_r(0.06);
        clear.set_g(0.06);
        clear.set_b(0.06);
        clear.set_a(1.0);
        scene.set_clear_color(&clear);

        let camera = babylonjs_lite::create_arc_rotate_camera(
            core::f64::consts::FRAC_PI_4,
            core::f64::consts::PI / 3.1,
            12.0,
            &babylonjs_lite::vec3(0.0, 1.1, 0.0),
        );
        babylonjs_lite::add_to_scene(&scene, &camera);
        let _ = babylonjs_lite::attach_control(&camera, &canvas, Some(&scene), None);

        let hemi = babylonjs_lite::create_hemispheric_light(
            Some(&JsTuple3::new(0.0, 1.0, 0.2)),
            Some(0.65),
        );
        hemi.set_diffuse_color(&JsTuple3::new(0.82, 0.82, 0.82));
        hemi.set_ground_color(&JsTuple3::new(0.12, 0.12, 0.12));
        babylonjs_lite::add_to_scene(&scene, &hemi);

        let dir =
            babylonjs_lite::create_directional_light(&JsTuple3::new(-1.0, -2.0, -1.0), Some(1.8));
        dir.set_diffuse(&JsTuple3::new(0.95, 0.95, 0.95));
        babylonjs_lite::add_to_scene(&scene, &dir);

        let orb_light =
            babylonjs_lite::create_point_light(&JsTuple3::new(4.5, 3.0, 0.0), Some(12.0));
        orb_light.set_diffuse(&JsTuple3::new(1.0, 1.0, 1.0));
        orb_light.set_range(18.0);
        babylonjs_lite::add_to_scene(&scene, &orb_light);

        let floor_opts = GroundOptions::create();
        floor_opts.set_width(Some(14.0));
        floor_opts.set_height(Some(14.0));
        let floor = babylonjs_lite::create_ground(&engine, Some(&floor_opts));
        floor.set_material(&mk_pbr(0.11, 0.08, 0.88));
        babylonjs_lite::add_to_scene(&scene, &floor);

        let plinth_opts = BoxOptions::create();
        plinth_opts.set_width(Some(8.4));
        plinth_opts.set_height(Some(0.08));
        plinth_opts.set_depth(Some(8.4));
        let plinth = babylonjs_lite::create_box(&engine, &plinth_opts);
        plinth.position().set(0.0, 0.04, 0.0);
        plinth.set_material(&mk_pbr(0.17, 0.12, 0.78));
        babylonjs_lite::add_to_scene(&scene, &plinth);

        for row in 0..5 {
            for col in 0..5 {
                let roughness = 0.08 + 0.22 * (row as f64);
                let metallic = 0.25 * (col as f64);
                let sopts = SphereOptions::create();
                sopts.set_diameter(Some(1.05));
                sopts.set_segments(Some(24.0));
                let sphere = babylonjs_lite::create_sphere(&engine, Some(&sopts));
                sphere.position().set(
                    ((col - 2) as f64) * 1.55,
                    0.75,
                    ((row - 2) as f64) * 1.55,
                );
                sphere.set_material(&mk_pbr(0.85, metallic, roughness));
                babylonjs_lite::add_to_scene(&scene, &sphere);
            }
        }

        let orb_props = CreatePbrMaterialProps::create();
        orb_props.set_base_color_factor(Some(&JsTuple4::new(0.95, 0.95, 0.95, 1.0)));
        orb_props.set_metallic_factor(Some(0.0));
        orb_props.set_roughness_factor(Some(0.2));
        babylonjs_lite::set_pbr_emissive(&orb_props, &JsTuple3::new(0.95, 0.95, 0.95));
        let orb_mat = babylonjs_lite::create_pbr_material(Some(&orb_props));

        let orb_opts = SphereOptions::create();
        orb_opts.set_diameter(Some(0.36));
        orb_opts.set_segments(Some(16.0));
        let orb = babylonjs_lite::create_sphere(&engine, Some(&orb_opts));
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
