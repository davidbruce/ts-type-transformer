// three.js from Rust, in Node: vector, matrix and rotation math, boxes and
// spheres, a scene graph with world positions, a mesh, a ray cast, and
// geometry attributes. Every check! must hold.
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
fn v(x: f64, y: f64, z: f64) -> Vector3 {
    Vector3::new_with(Some(x), Some(y), Some(z))
}
fn at(p: &Vector3, x: f64, y: f64, z: f64) -> bool {
    near(p.x(), x) && near(p.y(), y) && near(p.z(), z)
}

#[unsafe(no_mangle)]
pub extern "C" fn run() {
    vectors();
    rotations();
    shapes();
    scene();
}

fn vectors() {
    let a = v(1.0, 2.0, 3.0);
    let b = v(4.0, 5.0, 6.0);
    // Methods change the vector and return it, so they chain.
    let sum = a.js_clone().add(&b);
    check!(at(&sum, 5.0, 7.0, 9.0));
    check!(at(&a, 1.0, 2.0, 3.0));
    check!(near(a.dot(&b), 32.0));
    check!(at(
        &v(1.0, 0.0, 0.0).cross(&v(0.0, 1.0, 0.0)),
        0.0,
        0.0,
        1.0
    ));
    check!(near(v(3.0, 4.0, 0.0).length(), 5.0));
    check!(near(v(0.0, 3.0, 4.0).normalize().length(), 1.0));
    check!(near(a.distance_to(&b), 27f64.sqrt()));
    check!(at(
        &a.js_clone().multiply_scalar(2.0).sub_scalar(1.0),
        1.0,
        3.0,
        5.0
    ));
    check!(at(&Vector3::new(), 0.0, 0.0, 0.0));
    check!(at(&a.js_clone().lerp(&b, 0.5), 2.5, 3.5, 4.5));
    check!(a.equals(&v(1.0, 2.0, 3.0)) && !a.equals(&b));
    let arr = b.to_array();
    check!(arr.len() == 3 && arr.get(2) == 6.0);
    check!(at(
        &Vector3::new().from_array(JsArray::<f64>::from(vec![7.0, 8.0, 9.0])),
        7.0,
        8.0,
        9.0
    ));
    let mut sum = 0.0;
    for c in a.iter() {
        sum += c;
    }
    check!(sum == 6.0);

    // Colors.
    let c = Color::new_with(ColorRepresentation::Number(f64::from(0xff8000)));
    check!(near(c.r(), 1.0) && near(c.g(), 0.21586050010324417) && c.b() == 0.0);
    check!(c.get_hex() == f64::from(0xff8000));
    check!(c.get_hex_string() == "ff8000");
    let red = Color::new_with(ColorRepresentation::String("red".to_string()));
    check!(red.r() == 1.0 && red.g() == 0.0);
    check!(Color::new_with_r_and_g_and_b(0.0, 0.0, 1.0).get_hex() == 255.0);
    check!(
        red.js_clone()
            .lerp(&Color::new_with_r_and_g_and_b(0.0, 0.0, 1.0), 0.5)
            .r()
            == 0.5
    );
}

fn rotations() {
    // A quarter turn about Z takes x to y.
    let q = Quaternion::new();
    check!(q.w() == 1.0);
    q.set_from_axis_angle(&v(0.0, 0.0, 1.0), std::f64::consts::FRAC_PI_2);
    let p = v(1.0, 0.0, 0.0).apply_quaternion(&q);
    check!(at(&p, 0.0, 1.0, 0.0));
    check!(near(q.length(), 1.0));
    check!(near(
        q.angle_to(&Quaternion::new()),
        std::f64::consts::FRAC_PI_2
    ));

    // Euler angles and back.
    let e = Euler::new_with(Some(0.1), Some(0.2), Some(0.3), Some(EulerOrder::XYZ));
    let back = Euler::new().set_from_quaternion_with(
        &Quaternion::new().set_from_euler(&e),
        Some(EulerOrder::XYZ),
        None,
    );
    check!(near(back.x(), 0.1) && near(back.y(), 0.2) && near(back.z(), 0.3));
    check!(e.order() == EulerOrder::XYZ);

    // Matrices: translate, rotate, scale composed, then decomposed.
    let m = Matrix4::new();
    m.make_translation_with_x_and_y_and_z(1.0, 2.0, 3.0);
    check!(at(&v(0.0, 0.0, 0.0).apply_matrix4(&m), 1.0, 2.0, 3.0));
    check!(near(m.determinant(), 1.0));
    let composed = Matrix4::new();
    composed.compose(&v(10.0, 0.0, 0.0), &q, &v(2.0, 2.0, 2.0));
    check!(at(
        &v(1.0, 0.0, 0.0).apply_matrix4(&composed),
        10.0,
        2.0,
        0.0
    ));
    let (pos, rot, scale) = (Vector3::new(), Quaternion::new(), Vector3::new());
    composed.decompose(&pos, &rot, &scale);
    check!(at(&pos, 10.0, 0.0, 0.0) && at(&scale, 2.0, 2.0, 2.0));
    check!(near(rot.z(), q.z()) && near(rot.w(), q.w()));
    check!(near(composed.determinant(), 8.0));
    let inv = composed.js_clone().invert();
    check!(at(&v(10.0, 2.0, 0.0).apply_matrix4(&inv), 1.0, 0.0, 0.0));
    check!(composed.elements().len() == 16);
    check!(m.elements().get(12).as_f64() == 1.0 && m.elements().get(13).as_f64() == 2.0);
}

fn shapes() {
    // A box grown to hold points, and a sphere around it.
    let b = Box3::new();
    check!(b.is_empty());
    b.expand_by_point(&v(-1.0, -2.0, -3.0));
    b.expand_by_point(&v(1.0, 2.0, 3.0));
    check!(!b.is_empty());
    check!(at(&b.get_center(&Vector3::new()), 0.0, 0.0, 0.0));
    check!(at(&b.get_size(&Vector3::new()), 2.0, 4.0, 6.0));
    check!(b.contains_point(&v(0.5, 0.5, 0.5)) && !b.contains_point(&v(5.0, 0.0, 0.0)));
    check!(at(&b.min(), -1.0, -2.0, -3.0) && at(&b.max(), 1.0, 2.0, 3.0));
    let other = Box3::new_with(Some(&v(0.5, 0.5, 0.5)), Some(&v(9.0, 9.0, 9.0)));
    check!(b.intersects_box(&other));
    check!(at(&b.js_clone().union(&other).max(), 9.0, 9.0, 9.0));
    check!(near(b.distance_to_point(&v(4.0, 0.0, 0.0)), 3.0));

    let s = b.get_bounding_sphere(&Sphere::new());
    check!(near(s.radius(), 14f64.sqrt()));
    let sphere = Sphere::new_with(Some(&v(0.0, 0.0, 0.0)), Some(2.0));
    check!(sphere.contains_point(&v(1.0, 1.0, 0.0)) && !sphere.contains_point(&v(2.0, 2.0, 0.0)));
    check!(near(sphere.distance_to_point(&v(5.0, 0.0, 0.0)), 3.0));
    check!(sphere.intersects_sphere(&Sphere::new_with(Some(&v(3.0, 0.0, 0.0)), Some(1.5))));
    check!(sphere.intersects_box(&b));
}

fn scene() {
    // A hierarchy: a group moved, with a child offset inside it.
    let group = Group::new();
    group.position().set_with(10.0, 0.0, Some(0.0));
    let child = Object3D::new();
    child.set_name("child");
    child.position().set_with(0.0, 5.0, Some(0.0));
    group.add([child.clone()]);
    check!(group.children().len() == 1);
    check!(
        child
            .parent()
            .is_some_and(|p| p == Object3D::from(group.clone()))
    );
    group.update_matrix_world_with(Some(true));
    check!(at(
        &child.get_world_position(&Vector3::new()),
        10.0,
        5.0,
        0.0
    ));
    check!(at(&child.local_to_world(&v(1.0, 0.0, 0.0)), 11.0, 5.0, 0.0));
    check!(at(&child.world_to_local(&v(10.0, 5.0, 3.0)), 0.0, 0.0, 3.0));

    // Rotating the group swings the child about it; scale stretches.
    group.rotation().set(0.0, 0.0, std::f64::consts::FRAC_PI_2);
    group.update_matrix_world_with(Some(true));
    check!(at(
        &child.get_world_position(&Vector3::new()),
        5.0,
        0.0,
        0.0
    ));
    group.scale().set_scalar(2.0);
    group.update_matrix_world_with(Some(true));
    check!(at(
        &child.get_world_position(&Vector3::new()),
        0.0,
        0.0,
        0.0
    ));

    // Finding and traversing.
    check!(
        group
            .get_object_by_name("child")
            .is_some_and(|o| o.name() == "child")
    );
    check!(group.get_object_by_name("none").is_none());
    let seen = std::rc::Rc::new(std::cell::Cell::new(0));
    let count = Object3DFn::new({
        let seen = seen.clone();
        move |_: Object3D| seen.set(seen.get() + 1)
    });
    group.traverse(&count);
    check!(seen.get() == 2);
    group.remove([child.clone()]);
    check!(group.children().is_empty() && child.parent().is_none());

    // A mesh: a unit box with a basic material, hit by a ray from +Z.
    let geometry = BoxGeometry::new_with(
        &BoxGeometryArgs::default()
            .with_width(2.0)
            .with_height(2.0)
            .with_depth(2.0),
    );
    let params = MeshBasicMaterialParameters::create();
    params.set_color(ColorRepresentation::Number(f64::from(0x00ff00)));
    let material = MeshBasicMaterial::new_with(Some(&params));
    check!(material.color().get_hex() == f64::from(0x00ff00));
    let mesh = Mesh::new_with(&geometry, &material);
    check!(mesh.type_() == "Mesh");
    check!(mesh.geometry().type_() == "BoxGeometry");
    // A Mesh's geometry is a BufferGeometry: cast back to the BoxGeometry it is.
    check!(
        js::unchecked_cast::<BoxGeometry>(&mesh.geometry())
            .parameters()
            .width()
            == 2.0
    );
    mesh.position().set_with(0.0, 0.0, Some(-5.0));
    mesh.update_matrix_world_with(Some(true));

    let raycaster = Raycaster::new_with(
        Some(&v(0.0, 0.0, 5.0)),
        Some(&v(0.0, 0.0, -1.0)),
        None,
        None,
    );
    let hits = raycaster.intersect_object(&mesh);
    check!(hits.len() >= 1);
    // The near face of the box is at z = -4: distance 9 from the ray's origin.
    check!(near(hits.get(0).distance(), 9.0));
    let miss = Raycaster::new_with(
        Some(&v(50.0, 0.0, 5.0)),
        Some(&v(0.0, 0.0, -1.0)),
        None,
        None,
    );
    check!(miss.intersect_object(&mesh).is_empty());

    // Geometry attributes: a box has 24 vertices with positions, normals and uvs.
    // FINDING 8: `getAttribute<K extends keyof Attributes>(name: K)` is
    // `get_attribute(name: impl Into<JsValue>) -> WebGLAttributesGetAttribute`:
    // the attribute comes back as a union to match, though a box's are all
    // BufferAttributes.
    let attr = |name: &str| match geometry.get_attribute(name) {
        BufferGeometryGetAttributeResult::BufferAttribute(a) => a,
        _ => panic!("not a BufferAttribute"),
    };
    let position = attr("position");
    check!(position.count() == 24.0 && position.item_size() == 3.0);
    check!(attr("normal").count() == 24.0);
    check!(attr("uv").item_size() == 2.0);
    check!(geometry.has_attribute("uv") && !geometry.has_attribute("color"));
    check!(geometry.get_index().is_some_and(|i| i.count() == 36.0));
    geometry.compute_bounding_box();
    let bb = geometry.bounding_box().unwrap();
    check!(at(&bb.min(), -1.0, -1.0, -1.0) && at(&bb.max(), 1.0, 1.0, 1.0));
    geometry.translate(0.0, 1.0, 0.0);
    geometry.compute_bounding_sphere();
    check!(near(geometry.bounding_sphere().unwrap().center().y(), 1.0));
}
