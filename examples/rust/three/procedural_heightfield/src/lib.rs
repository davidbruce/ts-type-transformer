#![no_main]

use std::cell::RefCell;
use std::rc::Rc;

use three::*;

const GRID_N: usize = 28;

fn wave_height(mode: usize, x: f64, z: f64) -> f64 {
    match mode {
        1 => {
            let r = (x * x + z * z).sqrt();
            0.78 * (r * 2.2).cos() / (1.0 + 0.18 * r * r)
        }
        2 => 0.065 * (x * x - z * z) + 0.18 * (x * 1.6).sin(),
        _ => 0.55 * (x * 1.4).cos() * (z * 1.4).sin() + 0.25 * ((x + z) * 1.1).cos(),
    }
}

fn wave_name(mode: usize) -> &'static str {
    match mode {
        1 => "Radial Ripple",
        2 => "Saddle Ridge",
        _ => "Interference",
    }
}

struct AppState {
    renderer: WebGLRenderer,
    scene: Scene,
    camera: PerspectiveCamera,
    geo: BufferGeometry<NormalBufferAttributes, BufferGeometryEventMap>,
    pos_attr: Float32BufferAttribute,
    pts_attr: Float32BufferAttribute,
    wire_mesh: Mesh<BufferGeometry<NormalBufferAttributes, BufferGeometryEventMap>, MeshBasicMaterial>,
    wire_btn: dom::HTMLElement,
    meta: dom::HTMLElement,
    tri_count: usize,
    wave_mode: usize,
    overlay_on: bool,
    azimuth: f64,
}

fn rebuild_field(st: &AppState) {
    let span = 7.6;
    let half = span / 2.0;
    let denom = (GRID_N - 1) as f64;
    for iz in 0..GRID_N {
        for ix in 0..GRID_N {
            let idx = iz * GRID_N + ix;
            let x = -half + span * (ix as f64) / denom;
            let z = -half + span * (iz as f64) / denom;
            let y = wave_height(st.wave_mode, x, z);
            st.pos_attr.set_xyz(idx as f64, x, y, z);
        }
    }
    let mut pi = 0;
    for iz in (0..GRID_N).step_by(2) {
        for ix in (0..GRID_N).step_by(2) {
            let x = -half + span * (ix as f64) / denom;
            let z = -half + span * (iz as f64) / denom;
            let y = wave_height(st.wave_mode, x, z) + 0.06;
            st.pts_attr.set_xyz(pi as f64, x, y, z);
            pi += 1;
        }
    }

    st.pos_attr.set_needs_update(true);
    st.pts_attr.set_needs_update(true);
    st.geo.compute_vertex_normals();
    st.geo.compute_bounding_box();

    let r = 9.2;
    st.camera
        .position()
        .set(r * st.azimuth.sin(), 5.4, r * st.azimuth.cos());
    st.camera.look_at(&Vector3::new(0.0, 0.0, 0.0));
    st.wire_mesh.set_visible(st.overlay_on);
    st.wire_btn.set_text_content(if st.overlay_on {
        "Wireframe: On"
    } else {
        "Wireframe: Off"
    });
    st.renderer.render(&st.scene, &st.camera);

    let bb = st.geo.bounding_box().expect("missing bounding box");
    let msg = format!(
        "profile: {} \u{00B7} {} vertices \u{00B7} {} triangles \u{00B7} y=[{:.2}, {:.2}] \u{00B7} wireframe: {}",
        wave_name(st.wave_mode),
        GRID_N * GRID_N,
        st.tri_count,
        bb.min().y(),
        bb.max().y(),
        if st.overlay_on { "on" } else { "off" }
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
    let renderer = WebGLRenderer::new(&rp);
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(width, height, None);
    doc.get_element_by_id("stage")
        .expect("missing #stage")
        .append_child(renderer.dom_element());

    let scene = Scene::new();
    scene.set_background(Color::new(0xf7f7f7 as f64));
    let camera = PerspectiveCamera::new(42.0, width / height, 0.1, 100.0);

    scene.add([AmbientLight::new(ColorRepresentation::from(0xffffff as f64), 1.2)]);
    let key = DirectionalLight::new(ColorRepresentation::from(0xffffff as f64), 2.2);
    key.position().set(6.0, 10.0, 8.0);
    scene.add([key]);
    let rim = DirectionalLight::new(ColorRepresentation::from(0xffffff as f64), 0.9);
    rim.position().set(-7.0, 5.0, -6.0);
    scene.add([rim]);

    let grid = GridHelper::new(
        9.0,
        12.0,
        0x999999 as f64,
        0xe5e5e5 as f64,
    );
    grid.position().set_y(-1.1);
    scene.add([grid]);

    let geo = BufferGeometry::<NormalBufferAttributes, BufferGeometryEventMap>::new();
    let pos_attr = Float32BufferAttribute::new((GRID_N * GRID_N * 3) as f64, 3.0, None);

    let mut idx_vec = Vec::with_capacity((GRID_N - 1) * (GRID_N - 1) * 6);
    for iz in 0..GRID_N - 1 {
        for ix in 0..GRID_N - 1 {
            let a = (iz * GRID_N + ix) as f64;
            let b = a + 1.0;
            let c = a + GRID_N as f64;
            let d = c + 1.0;
            idx_vec.extend_from_slice(&[a, c, b, b, c, d]);
        }
    }
    let tri_count = (GRID_N - 1) * (GRID_N - 1) * 2;
    geo.set_attribute("position".to_string(), BufferAttribute::from(pos_attr.clone()));
    geo.set_index(JsArray::from(idx_vec));

    let smp = MeshStandardMaterialParameters::default()
        .with_color(0xb5b5b5 as f64)
        .with_roughness(0.72)
        .with_metalness(0.05)
        .with_flat_shading(true);
    let surf = Mesh::<_, MeshStandardMaterial>::new(
        Some(geo.clone()),
        Some(MeshStandardMaterial::new(&smp)),
    );
    scene.add([surf]);

    let wmp = MeshBasicMaterialParameters::default()
        .with_color(0x262626 as f64)
        .with_wireframe(true);
    let wire_mesh = Mesh::<_, MeshBasicMaterial>::new(
        Some(geo.clone()),
        Some(MeshBasicMaterial::new(&wmp)),
    );
    wire_mesh.position().set_y(0.015);
    scene.add([wire_mesh.clone()]);

    let pt_count = (GRID_N / 2) * (GRID_N / 2);
    let pts_geo = BufferGeometry::<NormalBufferAttributes, BufferGeometryEventMap>::new();
    let pts_attr = Float32BufferAttribute::new((pt_count * 3) as f64, 3.0, None);
    pts_geo.set_attribute("position".to_string(), BufferAttribute::from(pts_attr.clone()));
    let pmp = PointsMaterialParameters::default()
        .with_color(0x111111 as f64)
        .with_size(0.14);
    let pts = Points::<
        BufferGeometry<NormalBufferAttributes, BufferGeometryEventMap>,
        PointsMaterial,
        Object3DEventMap,
    >::new(Some(pts_geo), Some(PointsMaterial::new(&pmp)));
    scene.add([pts]);

    let st = Rc::new(RefCell::new(AppState {
        renderer,
        scene,
        camera,
        geo,
        pos_attr,
        pts_attr,
        wire_mesh,
        wire_btn: doc.get_element_by_id("btn-wire").expect("missing #btn-wire"),
        meta: doc.get_element_by_id("meta").expect("missing #meta"),
        tri_count,
        wave_mode: 0,
        overlay_on: true,
        azimuth: 0.62,
    }));

    for (i, bid) in ["btn-p0", "btn-p1", "btn-p2", "btn-wire", "btn-orbit"].iter().enumerate() {
        let btn = doc.get_element_by_id(bid).expect("missing button");
        let st_c = st.clone();
        let cb = dom::JsListener::new(move |_: dom::PointerEvent| {
            let mut s = st_c.borrow_mut();
            if i < 3 {
                s.wave_mode = i;
            } else if i == 3 {
                s.overlay_on = !s.overlay_on;
            } else {
                s.azimuth += std::f64::consts::PI / 4.0;
            }
            rebuild_field(&s);
        });
        btn.add_event_listener(dom::HTMLElementEventMap::CLICK, &cb, None);
    }

    rebuild_field(&st.borrow());
}
