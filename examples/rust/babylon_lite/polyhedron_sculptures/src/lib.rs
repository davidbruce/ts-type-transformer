#![no_main]

use babylonjs_lite::{
    CylinderOptions, CylinderOptionsMethods as _, DirectionalLightMethods as _,
    EulerProxyMethods as _, GroundOptions, GroundOptionsMethods as _, HemisphericLightMethods as _,
    JsTuple3, MeshMethods as _, NumberFn, ObservableVec3, ObservableVec3Methods as _,
    PolyhedronOptions, PolyhedronOptionsMethods as _, SceneNodeMethods as _, SpotLightMethods,
    StandardMaterialProps, StandardMaterialPropsMethods as _,
};
use dom::{GPUColorDict, GPUColorDictMethods as _, HTMLCanvasElement, ParentNodeMethods as _};

fn set_status_error(msg: &str) {
    if let Some(el) = dom::document().get_element_by_id("status") {
        let text = format!("WebGPU / Babylon Lite error: {msg}");
        el.set_text_content(Some(&text));
    }
}

fn mk_std(diff: f64, spec: f64, power: f64) -> StandardMaterialProps {
    let mat = babylonjs_lite::create_standard_material();
    mat.set_diffuse_color(&JsTuple3::new(diff, diff, diff));
    mat.set_specular_color(&JsTuple3::new(spec, spec, spec));
    mat.set_specular_power(power);
    mat
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
            core::f64::consts::FRAC_PI_2,
            core::f64::consts::PI / 2.8,
            11.5,
            &babylonjs_lite::vec3(0.0, 1.6, 0.0),
        );
        babylonjs_lite::add_to_scene(&scene, &camera);
        let _ = babylonjs_lite::attach_control(&camera, &canvas, Some(&scene), None);

        let hemi = babylonjs_lite::create_hemispheric_light(
            Some(&JsTuple3::new(0.0, 1.0, 0.0)),
            Some(0.42),
        );
        hemi.set_diffuse_color(&JsTuple3::new(0.70, 0.70, 0.70));
        hemi.set_ground_color(&JsTuple3::new(0.08, 0.08, 0.08));
        babylonjs_lite::add_to_scene(&scene, &hemi);

        let spot = babylonjs_lite::create_spot_light(
            &JsTuple3::new(0.0, 6.2, 3.0),
            &JsTuple3::new(0.0, -1.0, -0.55),
            core::f64::consts::PI / 3.2,
            3.0,
            Some(26.0),
        );
        spot.set_diffuse(&JsTuple3::new(1.0, 1.0, 1.0));
        spot.set_range(22.0);
        babylonjs_lite::add_to_scene(&scene, &spot);

        let fill =
            babylonjs_lite::create_directional_light(&JsTuple3::new(1.2, -1.5, -1.0), Some(0.65));
        fill.set_diffuse(&JsTuple3::new(0.80, 0.80, 0.80));
        babylonjs_lite::add_to_scene(&scene, &fill);

        let floor_opts = GroundOptions::create();
        floor_opts.set_width(Some(18.0));
        floor_opts.set_height(Some(18.0));
        let floor = babylonjs_lite::create_ground(&engine, Some(&floor_opts));
        floor.set_material(&mk_std(0.10, 0.25, 24.0));
        babylonjs_lite::add_to_scene(&scene, &floor);

        let plinth_mat = mk_std(0.18, 0.30, 32.0);
        let mut poly_state: Vec<(ObservableVec3, _, f64, f64)> = Vec::with_capacity(5);

        for i in 0..5 {
            let fi = i as f64;
            let x = (fi - 2.0) * 2.25;
            let z = -(fi - 2.0).abs() * 0.55;

            let copts = CylinderOptions::create();
            copts.set_height(Some(1.1));
            copts.set_diameter(Some(1.15));
            copts.set_tessellation(Some(28.0));
            let plinth = babylonjs_lite::create_cylinder(&engine, Some(&copts));
            plinth.position().set(x, 0.55, z);
            plinth.set_material(&plinth_mat);
            babylonjs_lite::add_to_scene(&scene, &plinth);

            let v = 0.90 - fi * 0.13;
            let popts = PolyhedronOptions::create();
            popts.set_type(Some(fi));
            popts.set_size(Some(0.62));
            popts.set_flat(Some(true));
            let mesh = babylonjs_lite::create_polyhedron(&engine, Some(&popts));
            mesh.position().set(x, 2.05, z);
            mesh.set_material(&mk_std(v, 0.95, 18.0 + fi * 24.0));
            babylonjs_lite::add_to_scene(&scene, &mesh);
            poly_state.push((mesh.position(), mesh.rotation(), x, z));
        }

        let spot_pos = SpotLightMethods::position(&spot);
        let spot_dir = spot.direction();
        let mut elapsed = 0.0;

        let render_cb = NumberFn::new(move |delta_ms| {
            elapsed += delta_ms * 0.001;
            let sx = (elapsed * 0.65).sin() * 5.2;
            spot_pos.set(sx, 6.2, 3.0);
            spot_dir.set(-sx * 0.15, -1.0, -0.55);
            for (i, (pos, rot, x, z)) in poly_state.iter().enumerate() {
                let fi = i as f64;
                pos.set(*x, 2.05 + (elapsed * 1.3 + fi * 1.1).sin() * 0.18, *z);
                rot.set_y(elapsed * (0.45 + 0.12 * fi));
                rot.set_x((elapsed * 0.7 + fi).sin() * 0.35);
            }
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
