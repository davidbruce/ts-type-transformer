#![no_main]

use std::cell::RefCell;
use std::rc::Rc;

use three::*;

struct AppState {
    renderer: WebGLRenderer,
    scene: Scene,
    camera: PerspectiveCamera,
    wire_group: Group,
    wire_btn: dom::HTMLElement,
    meta: dom::HTMLElement,
    wire_visible: bool,
    azimuth: f64,
}

fn add_solid(
    scene: &Scene,
    wire_group: &Group,
    geo: BufferGeometry<NormalBufferAttributes, BufferGeometryEventMap>,
    hex: u32,
    rough: f64,
    metal: f64,
    flat: bool,
    x: f64,
    y: f64,
    z: f64,
    thresh: f64,
    line_mat: &LineBasicMaterial,
) {
    let mp = MeshStandardMaterialParameters::default()
        .with_color(hex as f64)
        .with_roughness(rough)
        .with_metalness(metal)
        .with_flat_shading(flat);
    let mat = MeshStandardMaterial::new(Some(&mp));

    let mesh = Mesh::<
        BufferGeometry<NormalBufferAttributes, BufferGeometryEventMap>,
        MeshStandardMaterial,
    >::new(Some(geo.clone()), Some(mat));
    mesh.position().set(x, y, Some(z));
    mesh.rotation().set(0.25, 0.45, 0.0, None);
    scene.add([mesh]);

    let egeo = EdgesGeometry::new(Some(geo), Some(thresh));
    let segs = LineSegments::<
        EdgesGeometry<BufferGeometry<NormalBufferAttributes, BufferGeometryEventMap>>,
        LineBasicMaterial,
        Object3DEventMap,
    >::new(Some(egeo), Some(line_mat.clone()));
    segs.position().set(x, y, Some(z));
    segs.rotation().set(0.25, 0.45, 0.0, None);
    wire_group.add([segs]);
}

fn update_view(st: &AppState) {
    let r = 8.6;
    st.camera
        .position()
        .set(r * st.azimuth.sin(), 3.6, Some(r * st.azimuth.cos()));
    st.camera.look_at(&Vector3::new(Some(0.0), Some(0.0), Some(0.0)));
    st.wire_group.set_visible(st.wire_visible);
    st.wire_btn.set_text_content(Some(if st.wire_visible {
        "Wireframes: On"
    } else {
        "Wireframes: Off"
    }));
    st.renderer.render(&st.scene, &st.camera);

    let deg = (st.azimuth * 180.0 / std::f64::consts::PI).round() as i32;
    let msg = format!(
        "5 monochrome solids \u{00B7} EdgesGeometry wireframes: {} \u{00B7} azimuth: {} deg \u{00B7} 680x400 WebGL",
        if st.wire_visible { "on" } else { "off" },
        deg
    );
    st.meta.set_text_content(Some(&msg));
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let doc = dom::document();
    let width = 680.0;
    let height = 400.0;

    let rp = WebGLRendererParameters::default()
        .with_antialias(true)
        .with_preserve_drawing_buffer(true);
    let renderer = WebGLRenderer::new(Some(&rp));
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(width, height, None);
    doc.get_element_by_id("stage")
        .expect("missing #stage")
        .append_child(renderer.dom_element());

    let scene = Scene::new();
    scene.set_background(Color::new(0xf7f7f7 as f64));
    let camera = PerspectiveCamera::new(Some(42.0), Some(width / height), Some(0.1), Some(100.0));

    let ambient = AmbientLight::new(Some(ColorRepresentation::from(0xffffff as f64)), Some(1.2));
    scene.add([ambient]);
    let key = DirectionalLight::new(Some(ColorRepresentation::from(0xffffff as f64)), Some(2.2));
    key.position().set(6.0, 10.0, Some(8.0));
    scene.add([key]);
    let rim = DirectionalLight::new(Some(ColorRepresentation::from(0xffffff as f64)), Some(0.9));
    rim.position().set(-7.0, 5.0, Some(-6.0));
    scene.add([rim]);

    let grid = GridHelper::new(
        Some(12.0),
        Some(12.0),
        0x737373 as f64,
        0xd4d4d4 as f64,
    );
    grid.position().set_y(-1.2);
    scene.add([grid]);

    let wire_group = Group::new();
    scene.add([wire_group.clone()]);

    let lmp = LineBasicMaterialParameters::default().with_color(0x111111 as f64);
    let line_mat = LineBasicMaterial::new(Some(&lmp));

    add_solid(
        &scene,
        &wire_group,
        BoxGeometry::new(Some(1.45), Some(1.45), Some(1.45), None, None, None).into(),
        0x262626,
        0.65,
        0.1,
        false,
        -3.2,
        0.0,
        -0.6,
        1.0,
        &line_mat,
    );
    add_solid(
        &scene,
        &wire_group,
        IcosahedronGeometry::new(Some(0.95), Some(0.0)).into(),
        0xd4d4d4,
        0.55,
        0.08,
        true,
        -1.55,
        0.1,
        0.55,
        1.0,
        &line_mat,
    );
    add_solid(
        &scene,
        &wire_group,
        TorusKnotGeometry::new(Some(0.72), Some(0.24), Some(96.0), Some(16.0), None, None).into(),
        0x737373,
        0.45,
        0.15,
        false,
        0.0,
        0.25,
        -0.2,
        24.0,
        &line_mat,
    );
    add_solid(
        &scene,
        &wire_group,
        OctahedronGeometry::new(Some(0.95), Some(0.0)).into(),
        0xefefef,
        0.5,
        0.05,
        true,
        1.6,
        0.1,
        0.55,
        1.0,
        &line_mat,
    );
    add_solid(
        &scene,
        &wire_group,
        TorusGeometry::new(Some(0.72), Some(0.23), Some(16.0), Some(48.0), None, None, None).into(),
        0x404040,
        0.55,
        0.12,
        false,
        3.2,
        0.05,
        -0.5,
        22.0,
        &line_mat,
    );

    let st = Rc::new(RefCell::new(AppState {
        renderer,
        scene,
        camera,
        wire_group,
        wire_btn: doc.get_element_by_id("btn-wire").expect("missing #btn-wire"),
        meta: doc.get_element_by_id("meta").expect("missing #meta"),
        wire_visible: true,
        azimuth: 0.35,
    }));

    for (i, bid) in ["btn-wire", "btn-left", "btn-right", "btn-reset"].iter().enumerate() {
        let btn = doc.get_element_by_id(bid).expect("missing button");
        let st_c = st.clone();
        let cb = dom::JsListener::new(move |_: dom::PointerEvent| {
            let mut s = st_c.borrow_mut();
            match i {
                0 => s.wire_visible = !s.wire_visible,
                1 => s.azimuth -= std::f64::consts::PI / 6.0,
                2 => s.azimuth += std::f64::consts::PI / 6.0,
                _ => {
                    s.azimuth = 0.35;
                    s.wire_visible = true;
                }
            }
            update_view(&s);
        });
        btn.add_event_listener(dom::HTMLElementEventMap::CLICK, &cb, None);
    }

    update_view(&st.borrow());
}
