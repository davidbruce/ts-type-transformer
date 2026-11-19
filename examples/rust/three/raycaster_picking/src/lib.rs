#![no_main]

use std::cell::RefCell;
use std::rc::Rc;

use dom::DOMRectReadOnlyMethods as _;
use dom::ElementMethods as _;
use dom::MouseEventMethods as _;
use dom::NodeMethods as _;
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
    scene: Scene<Object3DEventMap>,
    camera: PerspectiveCamera,
    mats: Vec<MeshStandardMaterial>,
    meshes: Vec<Object3D<Object3DEventMap>>,
    mesh_arr: JsArray<Object3D<Object3DEventMap>>,
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
        .set(r * st.yaw.sin(), 5.1, Some(r * st.yaw.cos()));
    st.camera.look_at(&Vector3::new(Some(0.0), Some(0.0), Some(0.0)));
    st.camera.update_matrix_world(Some(true));
    st.scene.update_matrix_world(Some(true));
}

fn render_scene(st: &AppState) {
    for (i, mat) in st.mats.iter().enumerate() {
        let hex = if i as isize == st.selected { 0x111111 } else { 0xd4d4d4 };
        mat.color().set_hex(hex as f64, None);
    }
    if st.selected >= 0 && (st.selected as usize) < st.meshes.len() {
        st.box3
            .set_from_object(&st.meshes[st.selected as usize], Some(true));
        st.helper.set_visible(true);
    } else {
        st.helper.set_visible(false);
    }
    st.renderer
        .render(&Object3D::from(st.scene.clone()), &st.camera);
    st.meta.set_text_content(Some(&format!(
        "picked: {} \u{00B7} 9 specimens \u{00B7} click canvas or buttons",
        st.hit_info
    )));
}

fn cast_at_ndc(st: &mut AppState, nx: f64, ny: f64) {
    st.raycaster
        .set_from_camera(&Vector2::new(Some(nx), Some(ny)), &st.camera);
    let hits = st
        .raycaster
        .intersect_objects::<Object3D<Object3DEventMap>>(st.mesh_arr.clone(), Some(false), None);
    if hits.len() > 0 {
        let hit = hits.get(0);
        let hobj = hit.object();
        let found = st.meshes.iter().position(|m| m == &hobj);
        if let Some(idx) = found {
            st.selected = idx as isize;
            st.box3.set_from_object(&st.meshes[idx], Some(true));
            let sz = st.box3.get_size(&Vector3::new(None, None, None));
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
    let wp = st.meshes[idx].get_world_position(&Vector3::new(None, None, None));
    let proj = wp.project(&st.camera);
    cast_at_ndc(st, proj.x(), proj.y());
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let doc = dom::document();
    let width = 680.0;
    let height = 400.0;

    let rp = WebGLRendererParameters::create();
    rp.set_antialias(Some(true));
    rp.set_preserve_drawing_buffer(Some(true));
    let renderer = WebGLRenderer::new(Some(&rp));
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(width, height, None);
    let canvas: dom::HTMLCanvasElement = renderer.dom_element().into();
    doc.get_element_by_id("stage")
        .unwrap()
        .append_child(canvas.clone());

    let scene = Scene::<Object3DEventMap>::new();
    scene.set_background(ColorOrTexture::Color(Color::new(
        ColorRepresentation::Number(0xf7f7f7 as f64),
    )));
    let camera = PerspectiveCamera::new(Some(42.0), Some(width / height), Some(0.1), Some(100.0));

    scene.add([Object3D::from(AmbientLight::new(Some(ColorRepresentation::Number(0xffffff as f64)), Some(1.2)))]);
    let key = DirectionalLight::new(Some(ColorRepresentation::Number(0xffffff as f64)), Some(2.2));
    key.position().set(6.0, 10.0, Some(8.0));
    scene.add([Object3D::from(key)]);
    let rim = DirectionalLight::new(Some(ColorRepresentation::Number(0xffffff as f64)), Some(0.9));
    rim.position().set(-7.0, 5.0, Some(-6.0));
    scene.add([Object3D::from(rim)]);

    let grid = GridHelper::new(
        Some(9.0),
        Some(9.0),
        ColorRepresentation::Number(0x999999 as f64),
        ColorRepresentation::Number(0xd4d4d4 as f64),
    );
    grid.position().set_y(-0.85);
    scene.add([Object3D::from(grid)]);

    let geos: Vec<BufferGeometry<NormalBufferAttributes, BufferGeometryEventMap>> = vec![
        BoxGeometry::new(Some(1.05), Some(1.05), Some(1.05), None, None, None).into(),
        ConeGeometry::new(Some(0.62), Some(1.25), Some(20.0), None, None, None, None).into(),
        CylinderGeometry::new(Some(0.48), Some(0.48), Some(1.2), Some(24.0), None, None, None, None).into(),
        SphereGeometry::new(Some(0.62), Some(24.0), Some(18.0), None, None, None, None).into(),
        OctahedronGeometry::new(Some(0.68), Some(0.0)).into(),
        DodecahedronGeometry::new(Some(0.62), Some(0.0)).into(),
        TorusGeometry::new(Some(0.5), Some(0.18), Some(14.0), Some(36.0), None, None, None).into(),
        IcosahedronGeometry::new(Some(0.64), Some(0.0)).into(),
        TetrahedronGeometry::new(Some(0.72), Some(0.0)).into(),
    ];

    let mut mats = Vec::with_capacity(9);
    let mut meshes = Vec::with_capacity(9);
    for (i, geo) in geos.into_iter().enumerate() {
        let gx = (i % 3) as f64 - 1.0;
        let gz = (i / 3) as f64 - 1.0;
        let mp = MeshStandardMaterialParameters::create();
        mp.set_color(ColorRepresentation::Number(0xd4d4d4 as f64));
        mp.set_roughness(Some(0.55));
        mp.set_metalness(Some(0.1));
        mp.set_flat_shading(Some(i >= 4));
        let mat = MeshStandardMaterial::new(Some(&mp));
        let m = Mesh::<_, MeshStandardMaterial, Object3DEventMap>::new(Some(geo), Some(mat.clone()));
        m.set_name(NAMES[i]);
        m.position().set(gx * 2.1, 0.0, Some(gz * 2.1));
        m.rotation().set(0.2, 0.35 * i as f64, 0.0, None);
        let mo = Object3D::from(m);
        scene.add([mo.clone()]);
        mats.push(mat);
        meshes.push(mo);
    }
    let mesh_arr = JsArray::from(meshes.clone());

    let box3 = Box3::new(
        Some(&Vector3::new(Some(-0.5), Some(-0.5), Some(-0.5))),
        Some(&Vector3::new(Some(0.5), Some(0.5), Some(0.5))),
    );
    let helper = Box3Helper::new(&box3, ColorRepresentation::Number(0x111111 as f64));
    helper.set_visible(false);
    scene.add([Object3D::from(helper.clone())]);

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
        raycaster: Raycaster::new(None, None, None, None),
        meta: doc.get_element_by_id("meta").unwrap(),
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
    std::mem::forget(on_canvas);

    for (i, bid) in ["btn-p0", "btn-p1", "btn-p2", "btn-orbit", "btn-clear"].iter().enumerate() {
        let btn = doc.get_element_by_id(bid).unwrap();
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
        std::mem::forget(cb);
    }

    pick_specimen(&mut st.borrow_mut(), 4);
}
