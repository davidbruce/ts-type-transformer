#![no_main]

use std::cell::RefCell;
use std::rc::Rc;

use three::*;

const NAMES: [&str; 9] = [
    "Box",
    "Cone",
    "Cylinder",
    "Sphere",
    "Octahedron",
    "Dodecahedron",
    "Torus",
    "Icosahedron",
    "Tetrahedron",
];

struct AppState {
    renderer: WebGLRenderer,
    canvas: dom::HTMLCanvasElement,
    scene: Scene,
    camera: PerspectiveCamera,
    mats: Vec<MeshStandardMaterial>,
    meshes: Vec<Object3D>,
    mesh_arr: JsArray<Object3D>,
    box3: Box3,
    helper: Box3Helper,
    raycaster: Raycaster,
    meta: dom::HTMLElement,
    yaw: f64,
    selected: isize,
    hit_info: String,
}

fn update_camera(st: &AppState) {
    let r = 8.4;
    st.camera
        .position()
        .set_with(r * st.yaw.sin(), 5.1, r * st.yaw.cos());
    st.camera.look_at(&Vector3::new_with(0.0, 0.0, 0.0));
    st.camera.update_matrix_world_with(true);
    st.scene.update_matrix_world_with(true);
}

fn render_scene(st: &AppState) {
    for (i, mat) in st.mats.iter().enumerate() {
        let hex = if i as isize == st.selected {
            0x111111
        } else {
            0xd4d4d4
        };
        mat.color().set_hex(hex as f64);
    }
    if st.selected >= 0 && (st.selected as usize) < st.meshes.len() {
        st.box3
            .set_from_object_with(&st.meshes[st.selected as usize], true);
        st.helper.set_visible(true);
    } else {
        st.helper.set_visible(false);
    }
    st.renderer.render(&st.scene, &st.camera);
    st.meta.set_text_content(&format!(
        "picked: {} \u{00B7} 9 specimens \u{00B7} click canvas or buttons",
        st.hit_info
    ));
}

fn cast_at_ndc(st: &mut AppState, nx: f64, ny: f64) {
    st.raycaster
        .set_from_camera(&Vector2::new_with(nx, ny), &st.camera);
    let hits = st
        .raycaster
        .intersect_objects_with(st.mesh_arr.clone(), false, None);
    if hits.len() > 0 {
        let hit = hits.get(0);
        let hobj = hit.object();
        let found = st.meshes.iter().position(|m| m == &hobj);
        if let Some(idx) = found {
            st.selected = idx as isize;
            st.box3.set_from_object_with(&st.meshes[idx], true);
            let sz = st.box3.get_size(&Vector3::new());
            let pt = hit.point();
            st.hit_info = format!(
                "{} at d={:.2} ({:.2}, {:.2}, {:.2}) box={:.2}x{:.2}x{:.2}",
                NAMES[idx],
                hit.distance(),
                pt.x(),
                pt.y(),
                pt.z(),
                sz.x(),
                sz.y(),
                sz.z()
            );
        } else {
            st.selected = -1;
            st.hit_info = String::from("unknown");
        }
    } else {
        st.selected = -1;
        st.hit_info = format!("miss at ndc ({nx:.2}, {ny:.2})");
    }
    render_scene(st);
}

fn pick_specimen(st: &mut AppState, idx: usize) {
    update_camera(st);
    let wp = st.meshes[idx].get_world_position(&Vector3::new());
    let proj = wp.project(&st.camera);
    cast_at_ndc(st, proj.x(), proj.y());
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let doc = dom::document();
    let width = 680.0;
    let height = 400.0;

    let rp = WebGLRendererParameters::default()
        .with_antialias(true)
        .with_preserve_drawing_buffer(true);
    let renderer = WebGLRenderer::new_with(&rp);
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(width, height);
    let canvas: dom::HTMLCanvasElement = renderer.dom_element().into();
    doc.get_element_by_id("stage")
        .expect("missing #stage")
        .append_child(canvas.clone());

    let scene = Scene::new();
    scene.set_background(Color::new_with(0xf7f7f7 as f64));
    let camera = PerspectiveCamera::new_with(42.0, width / height, 0.1, 100.0);

    scene.add([AmbientLight::new_with(
        ColorRepresentation::from(0xffffff as f64),
        1.2,
    )]);
    let key = DirectionalLight::new_with(ColorRepresentation::from(0xffffff as f64), 2.2);
    key.position().set_with(6.0, 10.0, 8.0);
    scene.add([key]);
    let rim = DirectionalLight::new_with(ColorRepresentation::from(0xffffff as f64), 0.9);
    rim.position().set_with(-7.0, 5.0, -6.0);
    scene.add([rim]);

    let grid = GridHelper::new_with(9.0, 9.0, 0x999999 as f64, 0xd4d4d4 as f64);
    grid.position().set_y(-0.85);
    scene.add([grid]);

    let geos: Vec<BufferGeometry> = vec![
        BoxGeometry::new_with(
            &BoxGeometryArgs::default()
                .with_width(1.05)
                .with_height(1.05)
                .with_depth(1.05),
        )
        .into(),
        ConeGeometry::new_with(
            &ConeGeometryArgs::default()
                .with_radius(0.62)
                .with_height(1.25)
                .with_radial_segments(20.0),
        )
        .into(),
        CylinderGeometry::new_with(
            &CylinderGeometryArgs::default()
                .with_radius_top(0.48)
                .with_radius_bottom(0.48)
                .with_height(1.2)
                .with_radial_segments(24.0),
        )
        .into(),
        SphereGeometry::new_with(
            &SphereGeometryArgs::default()
                .with_radius(0.62)
                .with_width_segments(24.0)
                .with_height_segments(18.0),
        )
        .into(),
        OctahedronGeometry::new_with(0.68, 0.0).into(),
        DodecahedronGeometry::new_with(0.62, 0.0).into(),
        TorusGeometry::new_with(
            &TorusGeometryArgs::default()
                .with_radius(0.5)
                .with_tube(0.18)
                .with_radial_segments(14.0)
                .with_tubular_segments(36.0),
        )
        .into(),
        IcosahedronGeometry::new_with(0.64, 0.0).into(),
        TetrahedronGeometry::new_with(0.72, 0.0).into(),
    ];

    let mut mats = Vec::with_capacity(9);
    let mut meshes = Vec::with_capacity(9);
    for (i, geo) in geos.into_iter().enumerate() {
        let gx = (i % 3) as f64 - 1.0;
        let gz = (i / 3) as f64 - 1.0;
        let mp = MeshStandardMaterialParameters::default()
            .with_color(0xd4d4d4 as f64)
            .with_roughness(0.55)
            .with_metalness(0.1)
            .with_flat_shading(i >= 4);
        let mat = MeshStandardMaterial::new_with(&mp);
        let m = Mesh::new_with(&geo, &mat);
        m.set_name(NAMES[i]);
        m.position().set_with(gx * 2.1, 0.0, gz * 2.1);
        m.rotation().set(0.2, 0.35 * i as f64, 0.0);
        let mo: Object3D = m.into();
        scene.add([mo.clone()]);
        mats.push(mat);
        meshes.push(mo);
    }
    let mesh_arr = JsArray::from(meshes.clone());

    let box3 = Box3::new_with(
        &Vector3::new_with(-0.5, -0.5, -0.5),
        &Vector3::new_with(0.5, 0.5, 0.5),
    );
    let helper = Box3Helper::new_with(&box3, 0x111111 as f64);
    helper.set_visible(false);
    scene.add([helper.clone()]);

    let st = Rc::new(RefCell::new(AppState {
        renderer,
        canvas: canvas.clone(),
        scene,
        camera,
        mats,
        meshes,
        mesh_arr,
        box3,
        helper,
        raycaster: Raycaster::new(),
        meta: doc.get_element_by_id("meta").expect("missing #meta"),
        yaw: 0.45,
        selected: -1,
        hit_info: String::from("none"),
    }));

    let st_click = st.clone();
    let on_canvas = dom::JsListener::new(move |ev: dom::PointerEvent| {
        let mut s = st_click.borrow_mut();
        let rect = s.canvas.get_bounding_client_rect();
        let nx = ((ev.client_x() - rect.left()) / rect.width()) * 2.0 - 1.0;
        let ny = -((ev.client_y() - rect.top()) / rect.height()) * 2.0 + 1.0;
        update_camera(&s);
        cast_at_ndc(&mut s, nx, ny);
    });
    canvas.add_event_listener(dom::HTMLElementEventMap::CLICK, &on_canvas, None);

    for (i, bid) in ["btn-p0", "btn-p1", "btn-p2", "btn-orbit", "btn-clear"]
        .iter()
        .enumerate()
    {
        let btn = doc.get_element_by_id(bid).expect("missing button");
        let st_c = st.clone();
        let cb = dom::JsListener::new(move |_: dom::PointerEvent| {
            let mut s = st_c.borrow_mut();
            match i {
                0 => pick_specimen(&mut s, 4),
                1 => pick_specimen(&mut s, 0),
                2 => pick_specimen(&mut s, 8),
                3 => {
                    s.yaw += std::f64::consts::PI / 4.0;
                    update_camera(&s);
                    render_scene(&s);
                }
                _ => {
                    s.selected = -1;
                    s.hit_info = String::from("none");
                    update_camera(&s);
                    render_scene(&s);
                }
            }
        });
        btn.add_event_listener(dom::HTMLElementEventMap::CLICK, &cb, None);
    }

    pick_specimen(&mut st.borrow_mut(), 4);
}
