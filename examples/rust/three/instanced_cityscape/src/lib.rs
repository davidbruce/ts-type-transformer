#![no_main]

use std::cell::RefCell;
use std::rc::Rc;

use dom::NodeMethods as _;
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
    scene: Scene<Object3DEventMap>,
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
    let pos = Vector3::new(None, None, None);
    let rot_q = Quaternion::new(None, None, None, None);
    let scale = Vector3::new(None, None, None);
    let mtx = Matrix4::new();
    let axis_y = Vector3::new(Some(0.0), Some(1.0), Some(0.0));
    let col = Color::new(ColorRepresentation::Number(0xffffff as f64));

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
            pos.set(x, h * 0.5, Some(z));
            rot_q.set_from_axis_angle(&axis_y, 0.08 * ((gx + gz) % 4) as f64);
            scale.set(1.0, h, Some(1.0));
            mtx.compose(&pos, &rot_q, &scale);
            st.inst.set_matrix_at(idx as f64, &mtx);

            let t = ((h - 0.3) / 3.2).clamp(0.0, 1.0);
            let shade = 0.82 - 0.70 * t;
            col.set_rgb(shade, shade, shade, None);
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
    let target = Vector3::new(Some(0.0), Some(0.9), Some(0.0));
    st.ortho_cam.position().set(cx, 8.2, Some(cz));
    st.ortho_cam.look_at(&target);
    st.persp_cam.position().set(cx, 7.6, Some(cz));
    st.persp_cam.look_at(&target);

    if st.use_ortho {
        st.renderer
            .render(&Object3D::from(st.scene.clone()), &st.ortho_cam);
    } else {
        st.renderer
            .render(&Object3D::from(st.scene.clone()), &st.persp_cam);
    }

    let cname = if st.use_ortho {
        "Orthographic"
    } else {
        "Perspective"
    };
    st.cam_btn
        .set_text_content(Some(&format!("Camera: {cname}")));
    st.meta.set_text_content(Some(&format!(
        "100 InstancedMesh towers \u{00B7} layout: {} \u{00B7} peak height: {:.2} \u{00B7} camera: {}",
        layout_name(st.layout_mode),
        max_h,
        cname
    )));
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let doc = dom::document();
    let width = 680.0;
    let height = 400.0;
    let aspect = width / height;

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
    let ortho_cam = OrthographicCamera::new(
        Some(-5.4 * aspect),
        Some(5.4 * aspect),
        Some(5.4),
        Some(-5.4),
        Some(0.1),
        Some(100.0),
    );
    let persp_cam = PerspectiveCamera::new(Some(40.0), Some(aspect), Some(0.1), Some(100.0));

    scene.add([Object3D::from(AmbientLight::new(Some(ColorRepresentation::Number(0xffffff as f64)), Some(1.25)))]);
    let key = DirectionalLight::new(Some(ColorRepresentation::Number(0xffffff as f64)), Some(2.1));
    key.position().set(8.0, 12.0, Some(6.0));
    scene.add([Object3D::from(key)]);
    let rim = DirectionalLight::new(Some(ColorRepresentation::Number(0xffffff as f64)), Some(0.9));
    rim.position().set(-7.0, 6.0, Some(-8.0));
    scene.add([Object3D::from(rim)]);

    let pmp = MeshStandardMaterialParameters::create();
    pmp.set_color(ColorRepresentation::Number(0xe5e5e5 as f64));
    pmp.set_roughness(Some(0.85));
    let plinth = Mesh::<PlaneGeometry, MeshStandardMaterial, Object3DEventMap>::new(
        Some(PlaneGeometry::new(Some(8.6), Some(8.6), None, None)),
        Some(MeshStandardMaterial::new(Some(&pmp))),
    );
    plinth.rotation().set_x(-std::f64::consts::FRAC_PI_2);
    plinth.position().set_y(-0.01);
    scene.add([Object3D::from(plinth)]);

    let grid = GridHelper::new(
        Some(9.0),
        Some(10.0),
        ColorRepresentation::Number(0x999999 as f64),
        ColorRepresentation::Number(0xd4d4d4 as f64),
    );
    scene.add([Object3D::from(grid)]);

    let tgeo = BoxGeometry::new(Some(0.54), Some(1.0), Some(0.54), None, None, None);
    let tmp = MeshStandardMaterialParameters::create();
    tmp.set_color(ColorRepresentation::Number(0xffffff as f64));
    tmp.set_roughness(Some(0.65));
    tmp.set_metalness(Some(0.08));
    let tmat = MeshStandardMaterial::new(Some(&tmp));
    let inst = InstancedMesh::<BoxGeometry, MeshStandardMaterial, Object3DEventMap>::new(
        Some(tgeo),
        Some(tmat),
        100.0,
    );
    scene.add([Object3D::from(inst.clone())]);

    let st = Rc::new(RefCell::new(AppState {
        renderer,
        scene,
        ortho_cam,
        persp_cam,
        inst,
        cam_btn: doc.get_element_by_id("btn-cam").unwrap(),
        meta: doc.get_element_by_id("meta").unwrap(),
        use_ortho: true,
        layout_mode: 0,
        cam_yaw: 0.785398,
    }));

    for (i, bid) in ["btn-cam", "btn-lay0", "btn-lay1", "btn-lay2", "btn-rot"].iter().enumerate() {
        let btn = doc.get_element_by_id(bid).unwrap();
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
        std::mem::forget(cb);
    }

    render_city(&st.borrow());
}
