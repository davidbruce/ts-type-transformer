#![no_main]

use babylonjs_lite::{
    BoxOptions, BoxOptionsMethods as _, CapsuleOptions, CapsuleOptionsMethods as _,
    ClearCoatProps, ClearCoatPropsMethods as _, CreatePbrMaterialProps,
    CreatePbrMaterialPropsMethods as _, DirectionalLightMethods as _, EulerProxyMethods as _,
    GroundOptions, GroundOptionsMethods as _, HemisphericLightMethods as _, JsTuple3, JsTuple4,
    MeshMethods as _, NumberFn, ObservableVec3Methods as _, PbrMaterialProps,
    PointLightMethods as _, SceneNodeMethods as _,
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
        clear.set_r(0.05);
        clear.set_g(0.05);
        clear.set_b(0.05);
        clear.set_a(1.0);
        scene.set_clear_color(&clear);

        let camera = babylonjs_lite::create_arc_rotate_camera(
            core::f64::consts::FRAC_PI_4,
            core::f64::consts::PI / 3.1,
            12.8,
            &babylonjs_lite::vec3(0.0, 1.4, 0.0),
        );
        babylonjs_lite::add_to_scene(&scene, &camera);
        let _ = babylonjs_lite::attach_control(&camera, &canvas, Some(&scene), None);

        let hemi = babylonjs_lite::create_hemispheric_light(
            Some(&JsTuple3::new(0.0, 1.0, 0.15)),
            Some(0.55),
        );
        hemi.set_diffuse_color(&JsTuple3::new(0.76, 0.76, 0.76));
        hemi.set_ground_color(&JsTuple3::new(0.10, 0.10, 0.10));
        babylonjs_lite::add_to_scene(&scene, &hemi);

        let dir =
            babylonjs_lite::create_directional_light(&JsTuple3::new(-1.0, -2.2, -0.8), Some(1.35));
        dir.set_diffuse(&JsTuple3::new(0.90, 0.90, 0.90));
        babylonjs_lite::add_to_scene(&scene, &dir);

        let light1 =
            babylonjs_lite::create_point_light(&JsTuple3::new(5.2, 4.2, 0.0), Some(10.0));
        light1.set_diffuse(&JsTuple3::new(1.0, 1.0, 1.0));
        light1.set_range(16.0);
        babylonjs_lite::add_to_scene(&scene, &light1);

        let light2 =
            babylonjs_lite::create_point_light(&JsTuple3::new(-5.2, 3.4, 0.0), Some(7.5));
        light2.set_diffuse(&JsTuple3::new(0.78, 0.78, 0.78));
        light2.set_range(16.0);
        babylonjs_lite::add_to_scene(&scene, &light2);

        let floor_opts = GroundOptions::create();
        floor_opts.set_width(Some(18.0));
        floor_opts.set_height(Some(18.0));
        let floor = babylonjs_lite::create_ground(&engine, Some(&floor_opts));
        floor.set_material(&mk_pbr(0.09, 0.10, 0.88));
        babylonjs_lite::add_to_scene(&scene, &floor);

        let shades = [0.22, 0.44, 0.68, 0.88];
        let col_mats: [PbrMaterialProps; 4] = [
            mk_pbr(shades[0], 0.35, 0.36),
            mk_pbr(shades[1], 0.35, 0.36),
            mk_pbr(shades[2], 0.35, 0.36),
            mk_pbr(shades[3], 0.35, 0.36),
        ];

        let bopts = BoxOptions::create();
        bopts.set_width(Some(0.72));
        bopts.set_height(Some(1.0));
        bopts.set_depth(Some(0.72));

        let mut columns = Vec::with_capacity(36);
        for row in 0..6 {
            for col in 0..6 {
                let fx = (col as f64) - 2.5;
                let fz = (row as f64) - 2.5;
                let x = fx * 1.15;
                let z = fz * 1.15;
                let m = babylonjs_lite::create_box(&engine, &bopts);
                m.position().set(x, 0.5, z);
                m.set_material(&col_mats[(row + col) % 4]);
                babylonjs_lite::add_to_scene(&scene, &m);
                columns.push((
                    m.position(),
                    m.scaling(),
                    x,
                    z,
                    (fx * fx + fz * fz).sqrt(),
                    fx - fz,
                ));
            }
        }

        let cap_props = CreatePbrMaterialProps::create();
        cap_props.set_base_color_factor(Some(&JsTuple4::new(0.92, 0.92, 0.92, 1.0)));
        cap_props.set_metallic_factor(Some(0.50));
        cap_props.set_roughness_factor(Some(0.15));
        let cc = ClearCoatProps::create();
        cc.set_is_enabled(Some(true));
        cc.set_intensity(Some(1.0));
        cc.set_roughness(Some(0.08));
        babylonjs_lite::set_pbr_clear_coat(&cap_props, &cc);
        babylonjs_lite::set_pbr_emissive(&cap_props, &JsTuple3::new(0.16, 0.16, 0.16));
        let cap_mat = babylonjs_lite::create_pbr_material(Some(&cap_props));

        let copts = CapsuleOptions::create();
        copts.set_height(Some(1.15));
        copts.set_radius(Some(0.22));
        copts.set_tessellation(Some(20.0));
        let mut capsules = Vec::with_capacity(4);
        for _ in 0..4 {
            let cap = babylonjs_lite::create_capsule(&engine, Some(&copts));
            cap.set_material(&cap_mat);
            babylonjs_lite::add_to_scene(&scene, &cap);
            capsules.push((cap.position(), cap.rotation()));
        }

        let l1_pos = light1.position();
        let l2_pos = light2.position();
        let mut elapsed = 0.0;

        let render_cb = NumberFn::new(move |delta_ms| {
            elapsed += delta_ms * 0.001;
            for (pos, scale, x, z, dist, diag) in &columns {
                let h = 0.55
                    + 1.15 * (0.5 + 0.5 * (elapsed * 2.2 - dist * 1.15).sin())
                    + 0.30 * (elapsed * 1.3 + diag * 0.7).cos();
                scale.set(1.0, h, 1.0);
                pos.set(*x, h * 0.5, *z);
            }
            for (k, (pos, rot)) in capsules.iter().enumerate() {
                let fk = k as f64;
                let theta = elapsed * 0.7 + fk * core::f64::consts::FRAC_PI_2;
                pos.set(
                    theta.cos() * 4.9,
                    2.6 + (elapsed * 1.6 + fk * 1.57).sin() * 0.55,
                    theta.sin() * 4.9,
                );
                rot.set_z((elapsed + fk).sin() * 0.35);
                rot.set_y(-theta);
            }
            l1_pos.set((elapsed * 0.65).cos() * 5.2, 4.2, (elapsed * 0.65).sin() * 5.2);
            l2_pos.set((-elapsed * 0.52).cos() * 5.2, 3.4, (-elapsed * 0.52).sin() * 5.2);
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
