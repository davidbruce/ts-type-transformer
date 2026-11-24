#![no_main]

use std::cell::RefCell;
use std::rc::Rc;

use three::*;

fn tower_height(mode: usize, gx: usize, gz: usize) -> f64 {
    let fx = gx as f64 - 4.5;
    let fz = gz as f64 - 4.5;
    match mode {
        1 => {
            let r = (fx * fx + fz * fz).sqrt();
            0.35 + 0.42 * r + 0.25 * (fx * 0.8).cos()
        }
        2 => {
            let diag = (fx - fz).abs();
            0.3 + 2.8 * (-diag * diag / 5.5).exp() + 0.25 * (fz * 0.9).sin()
        }
        _ => {
            let r2 = fx * fx + fz * fz;
            0.35 + 3.1 * (-r2 / 9.5).exp() + 0.2 * (fx * 1.4).cos() * (fz * 1.4).cos()
        }
    }
}

fn layout_name(mode: usize) -> &'static str {
    match mode {
        1 => "Amphitheater",
        2 => "Diagonal",
        _ => "Plaza",
    }
}

struct AppState {
    renderer: WebGLRenderer,
    scene: Scene,
    ortho_cam: OrthographicCamera,
    persp_cam: PerspectiveCamera,
    inst: InstancedMesh<BoxGeometry, MeshStandardMaterial, Object3DEventMap>,
    cam_btn: dom::HTMLElement,
    meta: dom::HTMLElement,
    use_ortho: bool,
    layout_mode: usize,
    cam_yaw: f64,
}

fn render_city(st: &AppState) {
    let pos = Vector3::new();
    let rot_q = Quaternion::new();
    let scale = Vector3::new();
    let mtx = Matrix4::new();
    let axis_y = Vector3::new_with(0.0, 1.0, 0.0);
    let col = Color::new_with(0xffffff as f64);

    let mut max_h = 0.0_f64;
    let mut idx = 0;
    for gz in 0..10 {
        for gx in 0..10 {
            let x = (gx as f64 - 4.5) * 0.78;
            let z = (gz as f64 - 4.5) * 0.78;
            let h = tower_height(st.layout_mode, gx, gz);
            if h > max_h {
                max_h = h;
            }
            pos.set_with(x, h * 0.5, z);
            rot_q.set_from_axis_angle(&axis_y, 0.08 * ((gx + gz) % 4) as f64);
            scale.set_with(1.0, h, 1.0);
            mtx.compose(&pos, &rot_q, &scale);
            st.inst.set_matrix_at(idx as f64, &mtx);

            let t = ((h - 0.3) / 3.2).clamp(0.0, 1.0);
            let shade = 0.82 - 0.70 * t;
            col.set_rgb(shade, shade, shade);
            st.inst.set_color_at(idx as f64, &col);
            idx += 1;
        }
    }

    st.inst.instance_matrix().set_needs_update(true);
    if let Some(ic) = st.inst.instance_color() {
        ic.set_needs_update(true);
    }

    let radius = 11.5;
    let cx = radius * st.cam_yaw.cos();
    let cz = radius * st.cam_yaw.sin();
    let target = Vector3::new_with(0.0, 0.9, 0.0);
    st.ortho_cam.position().set_with(cx, 8.2, cz);
    st.ortho_cam.look_at(&target);
    st.persp_cam.position().set_with(cx, 7.6, cz);
    st.persp_cam.look_at(&target);

    if st.use_ortho {
        st.renderer.render(&st.scene, &st.ortho_cam);
    } else {
        st.renderer.render(&st.scene, &st.persp_cam);
    }

    let cname = if st.use_ortho {
        "Orthographic"
    } else {
        "Perspective"
    };
    st.cam_btn.set_text_content(&format!("Camera: {cname}"));
    st.meta.set_text_content(&format!(
        "100 InstancedMesh towers \u{00B7} layout: {} \u{00B7} peak height: {:.2} \u{00B7} camera: {}",
        layout_name(st.layout_mode),
        max_h,
        cname
    ));
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let doc = dom::document();
    let width = 680.0;
    let height = 400.0;
    let aspect = width / height;

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
    let ortho_cam =
        OrthographicCamera::new_with(-5.4 * aspect, 5.4 * aspect, 5.4, -5.4, 0.1, 100.0);
    let persp_cam = PerspectiveCamera::new_with(40.0, aspect, 0.1, 100.0);

    scene.add([AmbientLight::new_with(
        ColorRepresentation::from(0xffffff as f64),
        1.25,
    )]);
    let key = DirectionalLight::new_with(ColorRepresentation::from(0xffffff as f64), 2.1);
    key.position().set_with(8.0, 12.0, 6.0);
    scene.add([key]);
    let rim = DirectionalLight::new_with(ColorRepresentation::from(0xffffff as f64), 0.9);
    rim.position().set_with(-7.0, 6.0, -8.0);
    scene.add([rim]);

    let pmp = MeshStandardMaterialParameters::default()
        .with_color(0xe5e5e5 as f64)
        .with_roughness(0.85);
    let plinth = Mesh::new_with(
        Some(PlaneGeometry::new_with(8.6, 8.6, None, None)),
        Some(MeshStandardMaterial::new_with(&pmp)),
    );
    plinth.rotation().set_x(-std::f64::consts::FRAC_PI_2);
    plinth.position().set_y(-0.01);
    scene.add([plinth]);

    let grid = GridHelper::new_with(9.0, 10.0, 0x999999 as f64, 0xd4d4d4 as f64);
    scene.add([grid]);

    let tgeo = BoxGeometry::new_with(0.54, 1.0, 0.54, None, None, None);
    let tmp = MeshStandardMaterialParameters::default()
        .with_color(0xffffff as f64)
        .with_roughness(0.65)
        .with_metalness(0.08);
    let tmat = MeshStandardMaterial::new_with(&tmp);
    let inst = InstancedMesh::new(Some(tgeo), Some(tmat), 100.0);
    scene.add([inst.clone()]);

    let st = Rc::new(RefCell::new(AppState {
        renderer,
        scene,
        ortho_cam,
        persp_cam,
        inst,
        cam_btn: doc.get_element_by_id("btn-cam").expect("missing #btn-cam"),
        meta: doc.get_element_by_id("meta").expect("missing #meta"),
        use_ortho: true,
        layout_mode: 0,
        cam_yaw: 0.785398,
    }));

    for (i, bid) in ["btn-cam", "btn-lay0", "btn-lay1", "btn-lay2", "btn-rot"]
        .iter()
        .enumerate()
    {
        let btn = doc.get_element_by_id(bid).expect("missing button");
        let st_c = st.clone();
        let cb = dom::JsListener::new(move |_: dom::PointerEvent| {
            let mut s = st_c.borrow_mut();
            if i == 0 {
                s.use_ortho = !s.use_ortho;
            } else if i <= 3 {
                s.layout_mode = i - 1;
            } else {
                s.cam_yaw += std::f64::consts::PI / 4.0;
            }
            render_city(&s);
        });
        btn.add_event_listener(dom::HTMLElementEventMap::CLICK, &cb, None);
    }

    render_city(&st.borrow());
}
