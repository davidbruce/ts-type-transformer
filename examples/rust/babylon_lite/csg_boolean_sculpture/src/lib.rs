#![no_main]

use babylonjs_lite::{
    BoxOptions, BoxOptionsMethods as _, ClearCoatProps, ClearCoatPropsMethods as _,
    CreatePbrMaterialProps, CreatePbrMaterialPropsMethods as _, CylinderOptions,
    CylinderOptionsMethods as _, DirectionalLightMethods as _, DiscOptions,
    DiscOptionsMethods as _, EulerProxyMethods as _, GroundOptions, GroundOptionsMethods as _,
    HemisphericLightMethods as _, JsTuple3, JsTuple4, MeshMethods as _, NumberFn,
    ObservableVec3Methods as _, PbrMaterialProps, PointLightMethods as _, SceneNodeMethods as _,
    SphereOptions, SphereOptionsMethods as _, TorusOptions, TorusOptionsMethods as _,
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
            core::f64::consts::PI / 4.2,
            core::f64::consts::PI / 2.9,
            8.8,
            &babylonjs_lite::vec3(0.0, 2.1, 0.0),
        );
        babylonjs_lite::add_to_scene(&scene, &camera);
        let _ = babylonjs_lite::attach_control(&camera, &canvas, Some(&scene), None);

        let hemi = babylonjs_lite::create_hemispheric_light(
            Some(&JsTuple3::new(0.0, 1.0, 0.2)),
            Some(0.72),
        );
        hemi.set_diffuse_color(&JsTuple3::new(0.85, 0.85, 0.85));
        hemi.set_ground_color(&JsTuple3::new(0.14, 0.14, 0.14));
        babylonjs_lite::add_to_scene(&scene, &hemi);

        let dir =
            babylonjs_lite::create_directional_light(&JsTuple3::new(-1.2, -2.0, -1.1), Some(2.1));
        dir.set_diffuse(&JsTuple3::new(1.0, 1.0, 1.0));
        babylonjs_lite::add_to_scene(&scene, &dir);

        let key_light =
            babylonjs_lite::create_point_light(&JsTuple3::new(3.2, 3.6, 2.4), Some(8.0));
        key_light.set_diffuse(&JsTuple3::new(0.92, 0.92, 0.92));
        key_light.set_range(14.0);
        babylonjs_lite::add_to_scene(&scene, &key_light);

        let floor_opts = GroundOptions::create();
        floor_opts.set_width(Some(16.0));
        floor_opts.set_height(Some(16.0));
        let floor = babylonjs_lite::create_ground(&engine, Some(&floor_opts));
        floor.set_material(&mk_pbr(0.09, 0.10, 0.88));
        babylonjs_lite::add_to_scene(&scene, &floor);

        let disc_opts = DiscOptions::create();
        disc_opts.set_radius(Some(2.6));
        disc_opts.set_tessellation(Some(48.0));
        let disc = babylonjs_lite::create_disc(&engine, Some(&disc_opts));
        disc.rotation().set_x(core::f64::consts::FRAC_PI_2);
        disc.position().set(0.0, 0.03, 0.0);
        disc.set_material(&mk_pbr(0.22, 0.25, 0.65));
        babylonjs_lite::add_to_scene(&scene, &disc);

        let sbopts = BoxOptions::create();
        sbopts.set_size(Some(2.1));
        let src_box = babylonjs_lite::create_box(&engine, &sbopts);
        let copts = CylinderOptions::create();
        copts.set_height(Some(3.0));
        copts.set_diameter(Some(1.18));
        copts.set_tessellation(Some(16.0));
        let cyl_y = babylonjs_lite::create_cylinder(&engine, Some(&copts));
        let cyl_x = babylonjs_lite::create_cylinder(&engine, Some(&copts));
        cyl_x.rotation().set_z(core::f64::consts::FRAC_PI_2);
        let cyl_z = babylonjs_lite::create_cylinder(&engine, Some(&copts));
        cyl_z.rotation().set_x(core::f64::consts::FRAC_PI_2);

        let s1 = babylonjs_lite::csg_subtract(
            &babylonjs_lite::create_csg_from_mesh(&src_box, None),
            &babylonjs_lite::create_csg_from_mesh(&cyl_y, None),
        );
        let s2 =
            babylonjs_lite::csg_subtract(&s1, &babylonjs_lite::create_csg_from_mesh(&cyl_x, None));
        let s3 =
            babylonjs_lite::csg_subtract(&s2, &babylonjs_lite::create_csg_from_mesh(&cyl_z, None));
        let monolith = babylonjs_lite::create_mesh_from_csg(&engine, &s3, Some("monolith"));
        monolith.position().set(0.0, 2.15, 0.0);

        let mprops = CreatePbrMaterialProps::create();
        mprops.set_base_color_factor(Some(&JsTuple4::new(0.86, 0.86, 0.86, 1.0)));
        mprops.set_metallic_factor(Some(0.65));
        mprops.set_roughness_factor(Some(0.24));
        let cc = ClearCoatProps::create();
        cc.set_is_enabled(Some(true));
        cc.set_intensity(Some(0.90));
        cc.set_roughness(Some(0.10));
        babylonjs_lite::set_pbr_clear_coat(&mprops, &cc);
        monolith.set_material(&babylonjs_lite::create_pbr_material(Some(&mprops)));
        babylonjs_lite::add_to_scene(&scene, &monolith);

        let cbopts = BoxOptions::create();
        cbopts.set_size(Some(0.82));
        let core_box = babylonjs_lite::create_box(&engine, &cbopts);
        let csopts = SphereOptions::create();
        csopts.set_diameter(Some(1.02));
        csopts.set_segments(Some(14.0));
        let core_sphere = babylonjs_lite::create_sphere(&engine, Some(&csopts));
        let core_solid = babylonjs_lite::csg_intersect(
            &babylonjs_lite::create_csg_from_mesh(&core_box, None),
            &babylonjs_lite::create_csg_from_mesh(&core_sphere, None),
        );
        let core = babylonjs_lite::create_mesh_from_csg(&engine, &core_solid, Some("core"));
        core.position().set(0.0, 2.15, 0.0);
        core.set_material(&mk_pbr(0.18, 0.85, 0.16));
        babylonjs_lite::add_to_scene(&scene, &core);

        let otopts = TorusOptions::create();
        otopts.set_diameter(Some(4.2));
        otopts.set_thickness(Some(0.09));
        otopts.set_tessellation(Some(48.0));
        let outer_ring = babylonjs_lite::create_torus(&engine, Some(&otopts));
        outer_ring.position().set(0.0, 2.15, 0.0);
        outer_ring.set_material(&mk_pbr(0.55, 0.80, 0.25));
        babylonjs_lite::add_to_scene(&scene, &outer_ring);

        let itopts = TorusOptions::create();
        itopts.set_diameter(Some(3.55));
        itopts.set_thickness(Some(0.07));
        itopts.set_tessellation(Some(48.0));
        let inner_ring = babylonjs_lite::create_torus(&engine, Some(&itopts));
        inner_ring.position().set(0.0, 2.15, 0.0);
        inner_ring.set_material(&mk_pbr(0.88, 0.40, 0.30));
        babylonjs_lite::add_to_scene(&scene, &inner_ring);

        let mono_rot = monolith.rotation();
        let core_rot = core.rotation();
        let outer_rot = outer_ring.rotation();
        let inner_rot = inner_ring.rotation();
        let mut elapsed = 0.0;

        let render_cb = NumberFn::new(move |delta_ms| {
            elapsed += delta_ms * 0.001;
            mono_rot.set_y(elapsed * 0.35);
            mono_rot.set_x((elapsed * 0.3).sin() * 0.20);
            core_rot.set_y(-elapsed * 0.90);
            core_rot.set_z(elapsed * 0.50);
            outer_rot.set_x(elapsed * 0.42);
            outer_rot.set_y(elapsed * 0.25);
            inner_rot.set_z(-elapsed * 0.58);
            inner_rot.set_x(elapsed * 0.31);
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
