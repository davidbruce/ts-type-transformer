#![no_main]

use std::cell::RefCell;
use std::rc::Rc;

use three::*;

fn std_mat(hex: u32, rough: f64, metal: f64, flat: bool) -> MeshStandardMaterial {
    let mp = MeshStandardMaterialParameters::default()
        .with_color(hex as f64)
        .with_roughness(rough)
        .with_metalness(metal)
        .with_flat_shading(flat);
    MeshStandardMaterial::new_with(&mp)
}

struct AppState {
    renderer: WebGLRenderer,
    scene: Scene,
    camera: PerspectiveCamera,
    inner_pivot: Group,
    moon_pivot: Group,
    outer_pivot: Group,
    outer_station: Group,
    moon: Mesh,
    station_body: Mesh,
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

    st.scene.update_matrix_world_with(true);
    let mpos = st.moon.get_world_position(&Vector3::new());
    let spos = st.outer_station.get_world_position(&Vector3::new());

    st.tilt_btn.set_text_content(if st.tilted {
        "Outer Tilt: On"
    } else {
        "Outer Tilt: Off"
    });
    st.renderer.render(&st.scene, &st.camera);

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
    st.meta.set_text_content(&msg);
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
    doc.get_element_by_id("stage")
        .expect("missing #stage")
        .append_child(renderer.dom_element());

    let scene = Scene::new();
    scene.set_background(Color::new_with(0xf7f7f7 as f64));
    let camera = PerspectiveCamera::new_with(42.0, width / height, 0.1, 100.0);
    camera.position().set_with(5.6, 5.2, 7.6);
    camera.look_at(&Vector3::new_with(0.0, -0.1, 0.0));

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

    let grid = GridHelper::new_with(12.0, 12.0, 0x999999 as f64, 0xe5e5e5 as f64);
    grid.position().set_y(-1.5);
    scene.add([grid]);

    let pedestal = Mesh::new_with(
        &CylinderGeometry::new_with(
            &CylinderGeometryArgs::default()
                .with_radius_top(0.55)
                .with_radius_bottom(0.75)
                .with_height(0.35)
                .with_radial_segments(24.0),
        ),
        std_mat(0x262626, 0.7, 0.05, false),
    );
    pedestal.position().set_y(-1.3);
    scene.add([pedestal]);

    let core = Mesh::new_with(
        &SphereGeometry::new_with(
            &SphereGeometryArgs::default()
                .with_radius(0.65)
                .with_width_segments(24.0)
                .with_height_segments(18.0),
        ),
        std_mat(0x111111, 0.45, 0.2, false),
    );
    scene.add([core]);

    let inner_pivot = Group::new();
    scene.add([inner_pivot.clone()]);

    let tp1 = MeshBasicMaterialParameters::default().with_color(0x999999 as f64);
    let inner_track = Mesh::new_with(
        &TorusGeometry::new_with(
            &TorusGeometryArgs::default()
                .with_radius(2.1)
                .with_tube(0.025)
                .with_radial_segments(8.0)
                .with_tubular_segments(64.0),
        ),
        MeshBasicMaterial::new_with(&tp1),
    );
    inner_track.rotation().set_x(std::f64::consts::FRAC_PI_2);
    inner_pivot.add([inner_track]);

    let inner_planet = Group::new();
    inner_planet.position().set_with(2.1, 0.0, 0.0);
    inner_pivot.add([inner_planet.clone()]);

    let planet = Mesh::new_with(
        &SphereGeometry::new_with(
            &SphereGeometryArgs::default()
                .with_radius(0.36)
                .with_width_segments(20.0)
                .with_height_segments(16.0),
        ),
        std_mat(0x555555, 0.55, 0.1, false),
    );
    inner_planet.add([planet]);

    let moon_pivot = Group::new();
    inner_planet.add([moon_pivot.clone()]);
    let moon = Mesh::new_with(
        &OctahedronGeometry::new_with(0.18, 0.0),
        std_mat(0xd4d4d4, 0.4, 0.1, true),
    );
    moon.position().set_with(0.75, 0.0, 0.0);
    moon_pivot.add([moon.clone()]);

    let outer_pivot = Group::new();
    scene.add([outer_pivot.clone()]);

    let tp2 = MeshBasicMaterialParameters::default().with_color(0xb5b5b5 as f64);
    let outer_track = Mesh::new_with(
        &TorusGeometry::new_with(
            &TorusGeometryArgs::default()
                .with_radius(3.8)
                .with_tube(0.025)
                .with_radial_segments(8.0)
                .with_tubular_segments(80.0),
        ),
        MeshBasicMaterial::new_with(&tp2),
    );
    outer_track.rotation().set_x(std::f64::consts::FRAC_PI_2);
    outer_pivot.add([outer_track]);

    let outer_station = Group::new();
    outer_station.position().set_with(3.8, 0.0, 0.0);
    outer_pivot.add([outer_station.clone()]);

    let station_body = Mesh::new_with(
        &IcosahedronGeometry::new_with(0.42, 0.0),
        std_mat(0x404040, 0.5, 0.1, true),
    );
    outer_station.add([station_body.clone()]);

    let halo = Mesh::new_with(
        &TorusGeometry::new_with(
            &TorusGeometryArgs::default()
                .with_radius(0.68)
                .with_tube(0.06)
                .with_radial_segments(10.0)
                .with_tubular_segments(32.0),
        ),
        std_mat(0x111111, 0.4, 0.1, false),
    );
    halo.rotation().set_x(0.8);
    outer_station.add([halo]);

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
        tilt_btn: doc
            .get_element_by_id("btn-tilt")
            .expect("missing #btn-tilt"),
        meta: doc.get_element_by_id("meta").expect("missing #meta"),
        epoch: 0.6,
        tilted: false,
    }));

    for (i, bid) in ["btn-step", "btn-tilt", "btn-reset"].iter().enumerate() {
        let btn = doc.get_element_by_id(bid).expect("missing button");
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
    }

    update_orrery(&st.borrow());
}
