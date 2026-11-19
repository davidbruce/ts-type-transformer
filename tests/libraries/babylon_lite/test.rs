// @babylonjs/lite from Rust, in Node: vectors, cameras, lights, materials,
// procedural mesh buffers, accessibility tree, and math tuples. Every check! must hold.
#![no_main]
use bindings::*;

#[link(wasm_import_module = "env")]
unsafe extern "C" {
    #[link_name = "check"]
    fn check_at(ok: i32, line: u32);
}
macro_rules! check {
    ($c:expr) => {
        unsafe { check_at($c as i32, line!()) }
    };
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

#[unsafe(no_mangle)]
pub extern "C" fn run() {
    {
        // Vectors.
        let target = vec3(1.0, 2.0, 3.0);
        check!(target.x() == 1.0 && target.y() == 2.0 && target.z() == 3.0);
        target.set_y(1.6);
        check!(near(target.y(), 1.6));

        // Camera.
        let camera = create_arc_rotate_camera(0.75, 1.0, 8.5, &target);
        check!(near(camera.alpha(), 0.75) && near(camera.beta(), 1.0));
        check!(camera.radius() == 8.5 && near(camera.target().y(), 1.6));

        // Lights.
        let hemi = create_hemispheric_light(Some(&JsTuple3::new(0.0, 1.0, 0.2)), Some(0.75));
        hemi.set_diffuse_color(&JsTuple3::new(0.75, 0.85, 1.0));
        hemi.set_ground_color(&JsTuple3::new(0.12, 0.14, 0.22));
        let dc = hemi.diffuse_color();
        let gc = hemi.ground_color();
        check!(near(hemi.intensity(), 0.75));
        check!(near(dc.get0(), 0.75) && near(dc.get1(), 0.85) && near(dc.get2(), 1.0));
        check!(near(gc.get0(), 0.12));

        let dir = create_directional_light(&JsTuple3::new(-1.0, -2.0, -1.2), Some(2.2));
        dir.set_diffuse(&JsTuple3::new(1.0, 0.96, 0.9));
        check!(near(dir.intensity(), 2.2) && near(dir.diffuse().get1(), 0.96));

        let pt = create_point_light(&JsTuple3::new(2.8, 1.6, 0.0), Some(8.0));
        pt.set_range(12.0);
        check!(near(pt.intensity(), 8.0) && near(pt.range(), 12.0));

        // Materials.
        let props = CreatePbrMaterialProps::create();
        props.set_base_color_factor(Some(&JsTuple4::new(0.18, 0.78, 0.95, 1.0)));
        props.set_metallic_factor(Some(0.75));
        props.set_roughness_factor(Some(0.22));
        let pbr = create_pbr_material(Some(&props));
        check!(pbr.metallic_factor() == Some(0.75) && pbr.roughness_factor() == Some(0.22));

        let grid = create_grid_material(None);
        check!(!grid.vertex_source().is_empty() && !grid.fragment_source().is_empty());

        // Procedural geometry data.
        let sopts = SphereOptions::create();
        sopts.set_diameter(Some(2.0));
        sopts.set_segments(Some(8.0));
        let sphere = create_sphere_data(Some(&sopts));
        check!(sphere.vertex_count() > 0.0 && sphere.index_count() > 0.0);
        check!(!sphere.positions().to_vec().is_empty() && sphere.indices().byte_len() > 0);

        let topts = TorusKnotOptions::create();
        topts.set_radius(Some(1.0));
        topts.set_tube(Some(0.3));
        let knot = create_torus_knot_data(Some(&topts));
        check!(!knot.positions().is_empty() && knot.indices().byte_len() > 0);

        // Accessibility tree.
        let tree = create_accessibility_tree();
        let tag = AccessibilityTag::create();
        tag.set_name(Some("hero"));
        tag.set_role(Some("button"));
        let nopts = AccessibilityNodeOptions::create();
        nopts.set_tag(Some(&tag));
        let node = add_accessibility_node(&tree, &nopts);
        check!(tree.roots().len() == 1 && !tree.disposed());
        let got_tag = node.tag().unwrap();
        check!(got_tag.name().as_deref() == Some("hero") && got_tag.role().as_deref() == Some("button"));

        // Quaternion -> Euler tuple conversion.
        let euler = quat_to_euler_xyz_tuple(0.0, 0.0, 0.0, 1.0);
        check!(near(euler.get0(), 0.0) && near(euler.get1(), 0.0) && near(euler.get2(), 0.0));
    }
    check!(js::live_handles() == 0);
}
