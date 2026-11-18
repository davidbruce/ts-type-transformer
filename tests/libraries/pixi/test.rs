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
// `add_child<U: JsType>(children: U)`, and the glue spreads it: a JsArray is
// what is given (a bare child throws "Spread syntax requires ...iterable").
fn add<C: JsType, D: JsType>(parent: &Container<C>, child: D) {
    parent.add_child(JsArray::<D>::from(vec![child]));
}
fn remove<C: JsType, D: JsType>(parent: &Container<C>, child: D) {
    parent.remove_child(JsArray::<D>::from(vec![child]));
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
    let app = Application::<JsValue>::new();
    let options = ApplicationInitOptions::create();
    // FINDING 14: `canvas: ICanvas` is a structural interface that an
    // HTMLCanvasElement satisfies; no conversion is bound, so the handle is viewed as one.
    options.set_canvas(Some(&ICanvas(test_page::canvas().0.clone())));
    options.set_width(Some(400.0));
    options.set_height(Some(300.0));
    options.set_hello(Some(false));
    let started = app.init(Some(&options)).await;
    check!(started.is_ok());
    if started.is_err() {
        return;
    }
    check!(app.screen().width() == 400.0 && app.screen().height() == 300.0);
    check!(app.canvas() == test_page::canvas());
    let stage = app.stage();
    let g = Graphics::new(GraphicsOptionsOrGraphicsContext::from(GraphicsContext::new()));
    g.rect(0.0, 0.0, 100.0, 100.0).fill(Some(&JsValue::from(255.0)));
    add(&stage, g.clone());
    check!(stage.children().len() == 1);
    app.render();
    app.destroy(None, None);
}

fn math() {
    // Points.
    let p = Point::new(Some(3.0), Some(4.0));
    check!(p.x() == 3.0 && p.y() == 4.0);
    let q = p.js_clone();
    q.set(Some(10.0), None);
    check!(q.x() == 10.0 && q.y() == 10.0);
    check!(p.x() == 3.0);
    p.set_x(7.0);
    check!(p.to_string().contains('7'));
    check!(p.equals(&q.js_clone()) == false);
    check!(Point::new(None, None).x() == 0.0);

    // Rectangles.
    let r = Rectangle::new(NumberOrString::Number(10.0), NumberOrString::Number(20.0), NumberOrString::Number(100.0), NumberOrString::Number(50.0));
    check!(r.left() == 10.0 && r.right() == 110.0 && r.top() == 20.0 && r.bottom() == 70.0);
    check!(r.width() == 100.0 && r.height() == 50.0);
    check!(!r.is_empty() && Rectangle::empty().is_empty());
    let other = Rectangle::new(NumberOrString::Number(100.0), NumberOrString::Number(60.0), NumberOrString::Number(50.0), NumberOrString::Number(50.0));
    check!(r.intersects(&other, None));
    check!(!r.intersects(&Rectangle::new(NumberOrString::Number(500.0), NumberOrString::Number(0.0), NumberOrString::Number(5.0), NumberOrString::Number(5.0)), None));
    let grown = r.js_clone().pad(Some(5.0), None);
    check!(grown.x() == 5.0 && grown.width() == 110.0 && grown.height() == 60.0);
    check!(r.width() == 100.0);
    check!(r.contains_rect(&Rectangle::new(NumberOrString::Number(20.0), NumberOrString::Number(30.0), NumberOrString::Number(10.0), NumberOrString::Number(10.0))));

    // Matrices: translate, scale, rotate, invert.
    let m = Matrix::new(None, None, None, None, None, None);
    check!(m.is_identity());
    m.translate(5.0, 6.0).scale(2.0, 3.0);
    check!(m.a() == 2.0 && m.d() == 3.0 && m.tx() == 10.0 && m.ty() == 18.0);
    let inv = m.js_clone().invert();
    check!(near(inv.a(), 0.5) && near(inv.tx(), -5.0) && near(inv.ty(), -6.0));
    let rot = Matrix::new(None, None, None, None, None, None);
    rot.rotate(std::f64::consts::FRAC_PI_2);
    check!(near(rot.a(), 0.0) && near(rot.b(), 1.0) && near(rot.c(), -1.0));
    let both = m.js_clone().append(&rot);
    check!(!both.equals(&m) && m.equals(&m.js_clone()));
}

fn scene() {
    // A parent container moved and scaled, with a child offset inside it.
    let parent = Container::<JsValue>::new(None);
    parent.set_x(100.0);
    parent.set_y(50.0);
    let child = Container::<JsValue>::new(None);
    child.set_x(10.0);
    child.set_y(20.0);
    add(&parent, child.clone());
    check!(parent.children().len() == 1);
    check!(child.parent().is_some_and(|p| p == parent.clone()));
    // (worldTransform is only brought up to date by a render pass; positions are asked for.)
    check!(near(child.get_global_position(None, None).x(), 110.0) && near(child.get_global_position(None, None).y(), 70.0));

    // The graphics: a rectangle, filled.
    let g = Graphics::new(GraphicsOptionsOrGraphicsContext::from(GraphicsContext::new()));
    g.rect(0.0, 0.0, 40.0, 30.0).fill(Some(&JsValue::from(16711680.0)));
    let b = g.bounds();
    check!(b.width() == 40.0 && b.height() == 30.0 && b.x() == 0.0);
    add(&child, g.clone());
    parent.position().set(Some(100.0), Some(50.0));
    check!(parent.position().x() == 100.0);
    check!(g.world_transform().tx() == 0.0);

    // Scale the parent: the world bounds of the graphics follow.
    parent.scale().set(Some(2.0), Some(2.0));
    let world = g.get_bounds(None, None);
    check!(near(world.x(), 120.0) && near(world.y(), 90.0));
    check!(near(world.width(), 80.0) && near(world.height(), 60.0));
    check!(near(g.get_global_position(None, None).x(), 120.0) && near(g.get_global_position(None, None).y(), 90.0));
    // The container's own size is that of its content, and sets back.
    check!(near(parent.width(), 80.0) && near(parent.height(), 60.0));
    let local = parent.get_local_bounds();
    check!(near(local.width(), 40.0));

    // A point in the graphics' space and in the world's.
    let at: Point = g.to_global(&Point::new(Some(1.0), Some(1.0)), None, None);
    check!(near(at.x(), 122.0) && near(at.y(), 92.0));
    let back: Point = g.to_local(&Point::new(Some(122.0), Some(92.0)), None, None, None);
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
    // (Awkward: a static of the generic class needs its type parameter given, though unused.)
    let white = Texture::<TextureSource<JsValue>>::white();
    check!(white.frame().width() >= 1.0 && white.frame().height() >= 1.0);
    let texture = white.upcast::<TextureSource<JsValue>>();
    let sprite = Sprite::new(SpriteOptionsOrTexture::from(texture.clone()));
    sprite.set_width(64.0);
    sprite.set_height(32.0);
    check!(near(sprite.width(), 64.0) && near(sprite.height(), 32.0));
    sprite.set_anchor(PointDataOrNumber::Number(0.5));
    check!(sprite.anchor().x() == 0.5 && sprite.anchor().y() == 0.5);
    sprite.set_x(100.0);
    sprite.set_y(100.0);
    let sb = sprite.get_bounds(None, None);
    check!(near(sb.x(), 68.0) && near(sb.y(), 84.0) && near(sb.width(), 64.0));
    check!(sprite.texture() == texture);
    // A point is tested in the sprite's own space: its size is the scale of a unit texture.
    check!(sprite.contains_point(&Point::new(Some(0.4), Some(0.4))));
    check!(!sprite.contains_point(&Point::new(Some(3.0), Some(0.0))));
}
