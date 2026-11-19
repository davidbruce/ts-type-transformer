#![no_main]

use std::cell::RefCell;
use std::rc::Rc;

use dom::NodeMethods as _;
use three::*;

fn std_mat(hex: u32, rough: f64, metal: f64, flat: bool) -> MeshStandardMaterial {
    let mp = MeshStandardMaterialParameters::create();
    mp.set_color(ColorRepresentation::Number(hex as f64));
    mp.set_roughness(Some(rough));
    mp.set_metalness(Some(metal));
    mp.set_flat_shading(Some(flat));
    MeshStandardMaterial::new(Some(&mp))
}

struct AppState {
    renderer: WebGLRenderer,
    scene: Scene<Object3DEventMap>,
    camera: PerspectiveCamera,
    inner_pivot: Group<Object3DEventMap>,
    moon_pivot: Group<Object3DEventMap>,
    outer_pivot: Group<Object3DEventMap>,
    outer_station: Group<Object3DEventMap>,
    moon: Mesh<OctahedronGeometry, MeshStandardMaterial, Object3DEventMap>,
    station_body: Mesh<IcosahedronGeometry, MeshStandardMaterial, Object3DEventMap>,
    tilt_btn: dom::HTMLElement,
    meta: dom::HTMLElement,
    epoch: f64,
    tilted: bool,
}

fn update_orrery(st: &AppState) {
    st.inner_pivot.rotation().set_y(st.epoch);
    st.moon_pivot.rotation().set_y(st.epoch * 2.5);
    st.outer_pivot.rotation().set_y(-st.epoch * 0.55);
    st.outer_pivot
        .rotation()
        .set_z(if st.tilted { 0.38 } else { 0.0 });
    st.station_body.rotation().set_y(st.epoch * 1.4);

    st.scene.update_matrix_world(Some(true));
    let mpos = st
        .moon
        .get_world_position(&Vector3::new(None, None, None));
    let spos = st
        .outer_station
        .get_world_position(&Vector3::new(None, None, None));

    st.tilt_btn.set_text_content(Some(if st.tilted {
        "Outer Tilt: On"
    } else {
        "Outer Tilt: Off"
    }));
    st.renderer
        .render(&Object3D::from(st.scene.clone()), &st.camera);

    let deg = (st.epoch * 180.0 / std::f64::consts::PI).round() as i32;
    let msg = format!(
        "epoch: {} deg \u{00B7} moon world: ({:.2}, {:.2}, {:.2}) \u{00B7} station world: ({:.2}, {:.2}, {:.2})",
        deg,
        mpos.x(),
        mpos.y(),
        mpos.z(),
        spos.x(),
        spos.y(),
        spos.z()
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
    camera.position().set(5.6, 5.2, Some(7.6));
    camera.look_at(&Vector3::new(Some(0.0), Some(-0.1), Some(0.0)));

    scene.add([Object3D::from(AmbientLight::new(Some(ColorRepresentation::Number(0xffffff as f64)), Some(1.2)))]);
    let key = DirectionalLight::new(Some(ColorRepresentation::Number(0xffffff as f64)), Some(2.2));
    key.position().set(6.0, 10.0, Some(8.0));
    scene.add([Object3D::from(key)]);
    let rim = DirectionalLight::new(Some(ColorRepresentation::Number(0xffffff as f64)), Some(0.9));
    rim.position().set(-7.0, 5.0, Some(-6.0));
    scene.add([Object3D::from(rim)]);

    let grid = GridHelper::new(
        Some(12.0),
        Some(12.0),
        ColorRepresentation::Number(0x999999 as f64),
        ColorRepresentation::Number(0xe5e5e5 as f64),
    );
    grid.position().set_y(-1.5);
    scene.add([Object3D::from(grid)]);

    let pedestal = Mesh::<CylinderGeometry, MeshStandardMaterial, Object3DEventMap>::new(
        Some(CylinderGeometry::new(Some(0.55), Some(0.75), Some(0.35), Some(24.0), None, None, None, None)),
        Some(std_mat(0x262626, 0.7, 0.05, false)),
    );
    pedestal.position().set_y(-1.3);
    scene.add([Object3D::from(pedestal)]);

    let core = Mesh::<SphereGeometry, MeshStandardMaterial, Object3DEventMap>::new(
        Some(SphereGeometry::new(Some(0.65), Some(24.0), Some(18.0), None, None, None, None)),
        Some(std_mat(0x111111, 0.45, 0.2, false)),
    );
    scene.add([Object3D::from(core)]);

    let inner_pivot = Group::<Object3DEventMap>::new();
    scene.add([Object3D::from(inner_pivot.clone())]);

    let tp1 = MeshBasicMaterialParameters::create();
    tp1.set_color(ColorRepresentation::Number(0x999999 as f64));
    let inner_track = Mesh::<TorusGeometry, MeshBasicMaterial, Object3DEventMap>::new(
        Some(TorusGeometry::new(Some(2.1), Some(0.025), Some(8.0), Some(64.0), None, None, None)),
        Some(MeshBasicMaterial::new(Some(&tp1))),
    );
    inner_track.rotation().set_x(std::f64::consts::FRAC_PI_2);
    inner_pivot.add([Object3D::from(inner_track)]);

    let inner_planet = Group::<Object3DEventMap>::new();
    inner_planet.position().set(2.1, 0.0, Some(0.0));
    inner_pivot.add([Object3D::from(inner_planet.clone())]);

    let planet = Mesh::<SphereGeometry, MeshStandardMaterial, Object3DEventMap>::new(
        Some(SphereGeometry::new(Some(0.36), Some(20.0), Some(16.0), None, None, None, None)),
        Some(std_mat(0x555555, 0.55, 0.1, false)),
    );
    inner_planet.add([Object3D::from(planet)]);

    let moon_pivot = Group::<Object3DEventMap>::new();
    inner_planet.add([Object3D::from(moon_pivot.clone())]);
    let moon = Mesh::<OctahedronGeometry, MeshStandardMaterial, Object3DEventMap>::new(
        Some(OctahedronGeometry::new(Some(0.18), Some(0.0))),
        Some(std_mat(0xd4d4d4, 0.4, 0.1, true)),
    );
    moon.position().set(0.75, 0.0, Some(0.0));
    moon_pivot.add([Object3D::from(moon.clone())]);

    let outer_pivot = Group::<Object3DEventMap>::new();
    scene.add([Object3D::from(outer_pivot.clone())]);

    let tp2 = MeshBasicMaterialParameters::create();
    tp2.set_color(ColorRepresentation::Number(0xb5b5b5 as f64));
    let outer_track = Mesh::<TorusGeometry, MeshBasicMaterial, Object3DEventMap>::new(
        Some(TorusGeometry::new(Some(3.8), Some(0.025), Some(8.0), Some(80.0), None, None, None)),
        Some(MeshBasicMaterial::new(Some(&tp2))),
    );
    outer_track.rotation().set_x(std::f64::consts::FRAC_PI_2);
    outer_pivot.add([Object3D::from(outer_track)]);

    let outer_station = Group::<Object3DEventMap>::new();
    outer_station.position().set(3.8, 0.0, Some(0.0));
    outer_pivot.add([Object3D::from(outer_station.clone())]);

    let station_body = Mesh::<IcosahedronGeometry, MeshStandardMaterial, Object3DEventMap>::new(
        Some(IcosahedronGeometry::new(Some(0.42), Some(0.0))),
        Some(std_mat(0x404040, 0.5, 0.1, true)),
    );
    outer_station.add([Object3D::from(station_body.clone())]);

    let halo = Mesh::<TorusGeometry, MeshStandardMaterial, Object3DEventMap>::new(
        Some(TorusGeometry::new(Some(0.68), Some(0.06), Some(10.0), Some(32.0), None, None, None)),
        Some(std_mat(0x111111, 0.4, 0.1, false)),
    );
    halo.rotation().set_x(0.8);
    outer_station.add([Object3D::from(halo)]);

    let st = Rc::new(RefCell::new(AppState {
        renderer,
        scene,
        camera,
        inner_pivot,
        moon_pivot,
        outer_pivot,
        outer_station,
        moon,
        station_body,
        tilt_btn: doc.get_element_by_id("btn-tilt").unwrap(),
        meta: doc.get_element_by_id("meta").unwrap(),
        epoch: 0.6,
        tilted: false,
    }));

    for (i, bid) in ["btn-step", "btn-tilt", "btn-reset"].iter().enumerate() {
        let btn = doc.get_element_by_id(bid).unwrap();
        let st_c = st.clone();
        let cb = dom::JsListener::new(move |_: dom::PointerEvent| {
            let mut s = st_c.borrow_mut();
            match i {
                0 => s.epoch += std::f64::consts::PI / 6.0,
                1 => s.tilted = !s.tilted,
                _ => {
                    s.epoch = 0.6;
                    s.tilted = false;
                }
            }
            update_orrery(&s);
        });
        btn.add_event_listener(dom::HTMLElementEventMap::CLICK, &cb, None);
        std::mem::forget(cb);
    }

    update_orrery(&st.borrow());
}
