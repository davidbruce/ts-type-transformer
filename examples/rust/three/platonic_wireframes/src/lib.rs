#![no_main]

use std::cell::RefCell;
use std::rc::Rc;

use dom::NodeMethods as _;
use three::*;

struct AppState {
    renderer: WebGLRenderer,
    scene: Scene<Object3DEventMap>,
    camera: PerspectiveCamera,
    wire_group: Group<Object3DEventMap>,
    wire_btn: dom::HTMLElement,
    meta: dom::HTMLElement,
    wire_visible: bool,
    azimuth: f64,
}

fn add_solid(
    scene: &Scene<Object3DEventMap>,
    wire_group: &Group<Object3DEventMap>,
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
    let mp = MeshStandardMaterialParameters::create();
    mp.set_color(ColorRepresentation::Number(hex as f64));
    mp.set_roughness(Some(rough));
    mp.set_metalness(Some(metal));
    mp.set_flat_shading(Some(flat));
    let mat = MeshStandardMaterial::new(Some(&mp));

    let mesh = Mesh::<
        BufferGeometry<NormalBufferAttributes, BufferGeometryEventMap>,
        MeshStandardMaterial,
        Object3DEventMap,
    >::new(Some(geo.clone()), Some(mat));
    mesh.position().set(x, y, Some(z));
    mesh.rotation().set(0.25, 0.45, 0.0, None);
    scene.add([Object3D::from(mesh)]);

    let egeo = EdgesGeometry::new(Some(geo), Some(thresh));
    let segs = LineSegments::<
        EdgesGeometry<BufferGeometry<NormalBufferAttributes, BufferGeometryEventMap>>,
        LineBasicMaterial,
        Object3DEventMap,
    >::new(Some(egeo), Some(line_mat.clone()));
    segs.position().set(x, y, Some(z));
    segs.rotation().set(0.25, 0.45, 0.0, None);
    wire_group.add([Object3D::from(segs)]);
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
    st.renderer
        .render(&Object3D::from(st.scene.clone()), &st.camera);

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

    let rp = WebGLRendererParameters::create();
    rp.set_antialias(Some(true));
    rp.set_preserve_drawing_buffer(Some(true));
    let renderer = WebGLRenderer::new(Some(&rp));
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(width, height, None);
    doc.get_element_by_id("stage")
        .unwrap()
        .append_child(renderer.dom_element());

    let scene = Scene::<Object3DEventMap>::new();
    scene.set_background(ColorOrTexture::Color(Color::new(
        ColorRepresentation::Number(0xf7f7f7 as f64),
    )));
    let camera = PerspectiveCamera::new(Some(42.0), Some(width / height), Some(0.1), Some(100.0));

    let ambient = AmbientLight::new(Some(ColorRepresentation::Number(0xffffff as f64)), Some(1.2));
    scene.add([Object3D::from(ambient)]);
    let key = DirectionalLight::new(Some(ColorRepresentation::Number(0xffffff as f64)), Some(2.2));
    key.position().set(6.0, 10.0, Some(8.0));
    scene.add([Object3D::from(key)]);
    let rim = DirectionalLight::new(Some(ColorRepresentation::Number(0xffffff as f64)), Some(0.9));
    rim.position().set(-7.0, 5.0, Some(-6.0));
    scene.add([Object3D::from(rim)]);

    let grid = GridHelper::new(
        Some(12.0),
        Some(12.0),
        ColorRepresentation::Number(0x737373 as f64),
        ColorRepresentation::Number(0xd4d4d4 as f64),
    );
    grid.position().set_y(-1.2);
    scene.add([Object3D::from(grid)]);

    let wire_group = Group::<Object3DEventMap>::new();
    scene.add([Object3D::from(wire_group.clone())]);

    let lmp = LineBasicMaterialParameters::create();
    lmp.set_color(ColorRepresentation::Number(0x111111 as f64));
    let line_mat = LineBasicMaterial::new(Some(&lmp));

    add_solid(
        &scene,
        &wire_group,
        BufferGeometry::from(BoxGeometry::new(Some(1.45), Some(1.45), Some(1.45), None, None, None)),
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
        BufferGeometry::from(IcosahedronGeometry::new(Some(0.95), Some(0.0))),
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
        BufferGeometry::from(TorusKnotGeometry::new(Some(0.72), Some(0.24), Some(96.0), Some(16.0), None, None)),
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
        BufferGeometry::from(OctahedronGeometry::new(Some(0.95), Some(0.0))),
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
        BufferGeometry::from(TorusGeometry::new(Some(0.72), Some(0.23), Some(16.0), Some(48.0), None, None, None)),
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
        wire_btn: doc.get_element_by_id("btn-wire").unwrap(),
        meta: doc.get_element_by_id("meta").unwrap(),
        wire_visible: true,
        azimuth: 0.35,
    }));

    for (i, bid) in ["btn-wire", "btn-left", "btn-right", "btn-reset"].iter().enumerate() {
        let btn = doc.get_element_by_id(bid).unwrap();
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
        std::mem::forget(cb);
    }

    update_view(&st.borrow());
}
