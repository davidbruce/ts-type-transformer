// PixiJS v8 from Rust, in a page: points, rectangles and matrices, a scene
// graph of containers with transforms and bounds, Graphics shapes, and a
// Sprite from the built-in white texture. Nothing is rendered: no GPU is
// needed. Every check! must hold.
#![no_main]
use bindings::pixi_js::*;
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

// FINDING 13: `addChild<U extends C[]>(...children: U): U[0]` is bound as
// `add_child(children: impl Into<JsArray<Container>>)`, and the glue spreads
// it: a JsArray is what is given.
fn add(parent: &Container, child: impl Into<Container>) {
    parent.add_child(JsArray::<Container>::from(vec![child.into()]));
}
fn remove(parent: &Container, child: impl Into<Container>) {
    parent.remove_child(JsArray::<Container>::from(vec![child.into()]));
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-4
}

thread_local! {
    static DONE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

// Nonzero until the Application is up: the runner waits for it.
#[unsafe(no_mangle)]
pub extern "C" fn busy() -> i32 {
    !DONE.with(std::cell::Cell::get) as i32
}

#[unsafe(no_mangle)]
pub extern "C" fn run() {
    math();
    scene();
    js::spawn(async {
        application().await;
        DONE.with(|d| d.set(true));
    });
}

// An Application on the page's canvas: renders if the browser has WebGL.
async fn application() {
    let app = Application::new();
    let options = ApplicationInitOptions::create();
    // FINDING 14: `canvas: ICanvas` is a structural interface that an
    // HTMLCanvasElement satisfies; no conversion is bound, so the handle is viewed as one.
    options.set_canvas(Some(&ICanvas(test_page::canvas().0.clone())));
    options.set_width(Some(400.0));
    options.set_height(Some(300.0));
    options.set_hello(Some(false));
    let started = app.init_with(Some(&options)).await;
    check!(started.is_ok());
    if started.is_err() {
        return;
    }
    check!(app.screen().width() == 400.0 && app.screen().height() == 300.0);
    check!(app.canvas() == test_page::canvas());
    let stage = app.stage();
    let g = Graphics::new_with(GraphicsOptionsGraphicsContext::from(
        GraphicsContext::new(),
    ));
    g.rect(0.0, 0.0, 100.0, 100.0)
        .fill_with(Some(&JsValue::from(255.0)));
    add(&stage, g.clone());
    check!(stage.children().len() == 1);
    app.render();
    app.destroy();
}

fn math() {
    // Points.
    let p = Point::new_with(Some(3.0), Some(4.0));
    check!(p.x() == 3.0 && p.y() == 4.0);
    let q = p.js_clone();
    q.set_with(Some(10.0), None);
    check!(q.x() == 10.0 && q.y() == 10.0);
    check!(p.x() == 3.0);
    p.set_x(7.0);
    check!(p.to_string().contains('7'));
    check!(p.equals(&q.js_clone()) == false);
    check!(Point::new().x() == 0.0);

    // Rectangles.
    let r = Rectangle::new_with(
        StringNumber::Number(10.0),
        StringNumber::Number(20.0),
        StringNumber::Number(100.0),
        StringNumber::Number(50.0),
    );
    check!(r.left() == 10.0 && r.right() == 110.0 && r.top() == 20.0 && r.bottom() == 70.0);
    check!(r.width() == 100.0 && r.height() == 50.0);
    check!(!r.is_empty() && Rectangle::empty().is_empty());
    let other = Rectangle::new_with(
        StringNumber::Number(100.0),
        StringNumber::Number(60.0),
        StringNumber::Number(50.0),
        StringNumber::Number(50.0),
    );
    check!(r.intersects(&other));
    check!(!r.intersects(&Rectangle::new_with(
        StringNumber::Number(500.0),
        StringNumber::Number(0.0),
        StringNumber::Number(5.0),
        StringNumber::Number(5.0)
    )));
    let grown = r.js_clone().pad_with(Some(5.0), None);
    check!(grown.x() == 5.0 && grown.width() == 110.0 && grown.height() == 60.0);
    check!(r.width() == 100.0);
    check!(r.contains_rect(&Rectangle::new_with(
        StringNumber::Number(20.0),
        StringNumber::Number(30.0),
        StringNumber::Number(10.0),
        StringNumber::Number(10.0)
    )));

    // Matrices: translate, scale, rotate, invert.
    let m = Matrix::new();
    check!(m.is_identity());
    m.translate(5.0, 6.0).scale(2.0, 3.0);
    check!(m.a() == 2.0 && m.d() == 3.0 && m.tx() == 10.0 && m.ty() == 18.0);
    let inv = m.js_clone().invert();
    check!(near(inv.a(), 0.5) && near(inv.tx(), -5.0) && near(inv.ty(), -6.0));
    let rot = Matrix::new();
    rot.rotate(std::f64::consts::FRAC_PI_2);
    check!(near(rot.a(), 0.0) && near(rot.b(), 1.0) && near(rot.c(), -1.0));
    let both = m.js_clone().append(&rot);
    check!(!both.equals(&m) && m.equals(&m.js_clone()));
}

fn scene() {
    // A parent container moved and scaled, with a child offset inside it.
    let parent = Container::new();
    parent.set_x(100.0);
    parent.set_y(50.0);
    let child = Container::new();
    child.set_x(10.0);
    child.set_y(20.0);
    add(&parent, child.clone());
    check!(parent.children().len() == 1);
    check!(child.parent().is_some_and(|p| p == parent.clone()));
    // (worldTransform is only brought up to date by a render pass; positions are asked for.)
    check!(
        near(child.get_global_position().x(), 110.0) && near(child.get_global_position().y(), 70.0)
    );

    // The graphics: a rectangle, filled.
    let g = Graphics::new_with(GraphicsOptionsGraphicsContext::from(
        GraphicsContext::new(),
    ));
    g.rect(0.0, 0.0, 40.0, 30.0)
        .fill_with(Some(&JsValue::from(16711680.0)));
    let b = g.bounds();
    check!(b.width() == 40.0 && b.height() == 30.0 && b.x() == 0.0);
    add(&child, g.clone());
    parent.position().set_with(Some(100.0), Some(50.0));
    check!(parent.position().x() == 100.0);
    check!(g.world_transform().tx() == 0.0);

    // Scale the parent: the world bounds of the graphics follow.
    parent.scale().set_with(Some(2.0), Some(2.0));
    let world = g.get_bounds();
    check!(near(world.x(), 120.0) && near(world.y(), 90.0));
    check!(near(world.width(), 80.0) && near(world.height(), 60.0));
    check!(near(g.get_global_position().x(), 120.0) && near(g.get_global_position().y(), 90.0));
    // The container's own size is that of its content, and sets back.
    check!(near(parent.width(), 80.0) && near(parent.height(), 60.0));
    let local = parent.get_local_bounds();
    check!(near(local.width(), 40.0));

    // A point in the graphics' space and in the world's.
    let at = g.to_global(&Point::new_with(Some(1.0), Some(1.0)));
    check!(near(at.x(), 122.0) && near(at.y(), 92.0));
    let back = g.to_local(&Point::new_with(Some(122.0), Some(92.0)));
    check!(near(back.x(), 1.0) && near(back.y(), 1.0));

    // Rotation, alpha and visibility.
    child.set_angle(90.0);
    check!(near(child.rotation(), std::f64::consts::FRAC_PI_2));
    child.set_alpha(0.5);
    check!(child.alpha() == 0.5 && child.visible());
    child.set_visible(false);
    check!(!child.visible());

    // Removing a child.
    remove(&parent, child.clone());
    check!(parent.children().is_empty());
    check!(child.parent().is_none());

    // A sprite from the built-in white texture, sized and anchored.
    let texture = Texture::white();
    check!(texture.frame().width() >= 1.0 && texture.frame().height() >= 1.0);
    let sprite = Sprite::new_with(SpriteArg::from(texture.clone()));
    sprite.set_width(64.0);
    sprite.set_height(32.0);
    check!(near(sprite.width(), 64.0) && near(sprite.height(), 32.0));
    sprite.set_anchor(PointDataNumber::Number(0.5));
    check!(sprite.anchor().x() == 0.5 && sprite.anchor().y() == 0.5);
    sprite.set_x(100.0);
    sprite.set_y(100.0);
    let sb = sprite.get_bounds();
    check!(near(sb.x(), 68.0) && near(sb.y(), 84.0) && near(sb.width(), 64.0));
    check!(sprite.texture() == texture);
    // A point is tested in the sprite's own space: its size is the scale of a unit texture.
    check!(sprite.contains_point(&Point::new_with(Some(0.4), Some(0.4))));
    check!(!sprite.contains_point(&Point::new_with(Some(3.0), Some(0.0))));
}
