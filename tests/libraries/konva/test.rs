// Konva from Rust, in a page: a Stage on the page's element, a Layer with
// shapes, groups, positions, sizes, lookups, events and serialization.
// Every check! must hold.
#![no_main]
use bindings::konva::*;
use bindings::*;
use std::cell::Cell;
use std::rc::Rc;

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

fn point(x: f64, y: f64) -> Vector2d {
    let p = Vector2d::create();
    p.set_x(x);
    p.set_y(y);
    p
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

#[unsafe(no_mangle)]
pub extern "C" fn run() {
    // A stage in the page's #stage, 400 by 300.
    let config = StageConfig::create();
    config.set_container(HTMLDivElementString::String("stage".to_string()));
    config.set_width(Some(400.0));
    config.set_height(Some(300.0));
    let stage = Stage::new(&config);
    check!(stage.width().invoke() == 400.0 && stage.height().invoke() == 300.0);
    check!(stage.get_content().0.0 != 0);

    // A layer on the stage, and shapes on the layer.
    let layer = Layer::new();
    stage.add(&layer, Vec::<JsValue>::new());
    check!(stage.get_layers().len() == 1);
    check!(layer.get_stage() == stage.clone());

    let rc = RectConfig::create();
    rc.set_x(Some(20.0));
    rc.set_y(Some(30.0));
    rc.set_width(Some(100.0));
    rc.set_height(Some(50.0));
    rc.set_name(Some("box"));
    rc.set_id(Some("r1"));
    let rect = Rect::new_with(Some(&rc));
    check!(rect.x().invoke() == 20.0 && rect.y().invoke() == 30.0);
    check!(rect.width().invoke() == 100.0 && rect.height().invoke() == 50.0);
    check!(rect.id().invoke() == "r1" && rect.name().invoke() == "box");
    check!(rect.get_class_name() == "Rect");

    let cc = CircleConfig::create();
    cc.set_x(Some(200.0));
    cc.set_y(Some(100.0));
    cc.set_radius(Some(25.0));
    let circle = Circle::new_with(Some(&cc));
    check!(
        circle.radius().invoke() == 25.0
            && circle.get_width() == 50.0
            && circle.get_height() == 50.0
    );

    let tc = TextConfig::create();
    tc.set_text(Some("hello konva"));
    tc.set_font_size(Some(20.0));
    tc.set_x(Some(10.0));
    tc.set_y(Some(200.0));
    let text = Text::new_with(Some(&tc));
    check!(text.text().invoke() == "hello konva" && text.font_size().invoke() == 20.0);
    check!(text.get_text_width() > 0.0 && text.get_text_height() > 0.0);
    text.set_text("hi");
    check!(text.text().invoke() == "hi");

    // Shapes added to the layer, in order; a group holding one of them.
    layer.add([Node::from(rect.clone()), Node::from(circle.clone())]);
    check!(layer.get_children().len() == 2);
    let group = Group::new();
    group.set_position(&point(50.0, 10.0));
    group.add([Node::from(text.clone())]);
    layer.add([group.clone()]);
    check!(layer.get_children().len() == 3);
    check!(group.get_children().len() == 1);
    check!(text.get_parent().is_some_and(|p| js::same(&p.0, &group.0)));
    check!(text.get_layer().is_some_and(|l| l == layer.clone()));
    check!(rect.get_z_index() == 0.0 && circle.get_z_index() == 1.0 && group.get_z_index() == 2.0);
    layer.draw();

    // Positions: local, absolute (inside the group), and moved.
    check!(text.get_absolute_position().x() == 60.0 && text.get_absolute_position().y() == 210.0);
    check!(text.get_position().x() == 10.0);
    rect.set_position(&point(40.0, 30.0));
    check!(rect.x().invoke() == 40.0 && rect.get_position().x() == 40.0);
    rect.move_(&point(5.0, 5.0));
    check!(rect.x().invoke() == 45.0 && rect.y().invoke() == 35.0);
    rect.x().invoke_2(JsValue::from(60.0));
    check!(rect.x().invoke() == 60.0);
    rect.x().invoke_2(JsValue::from(45.0));
    rect.rotate(90.0);
    check!(near(rect.rotation().invoke(), 90.0));

    // The bounding box on screen, of the circle (centre 200,100, radius 25).
    let box_ = circle.get_client_rect();
    check!(
        near(box_.x(), 175.0)
            && near(box_.y(), 75.0)
            && near(box_.width(), 50.0)
            && near(box_.height(), 50.0)
    );

    // Finding nodes: by id, by name, by class, by predicate.
    check!(
        layer
            .find_one("#r1")
            .is_some_and(|n| n.get_class_name() == "Rect")
    );
    check!(layer.find(JsValue::from(".box")).len() == 1);
    check!(layer.find(JsValue::from("Circle")).len() == 1);
    check!(layer.find(JsValue::from("Shape")).len() == 3);
    check!(layer.find_one("#nothing").is_none());

    // Events: a listener on the circle, fired, and removed.
    let clicks = Rc::new(Cell::new(0));
    let on_click = KonvaEventListener::<Node, PointerEvent>::new({
        let clicks = clicks.clone();
        move |_| clicks.set(clicks.get() + 1)
    });
    circle.on_click(&on_click);
    circle.fire("click");
    circle.fire("click");
    check!(clicks.get() == 2);
    circle.off_with(Some("click"), None);
    circle.fire("click");
    check!(clicks.get() == 2);

    // Serialization: a node as JSON, with its class and attributes.
    let json = rect.to_json();
    check!(json.contains("\"className\":\"Rect\"") && json.contains("\"id\":\"r1\""));
    check!(layer.to_json().contains("\"children\""));
    check!(layer.to_object().children().is_some_and(|c| c.len() == 3));

    // Hiding, and removal: a hidden node is not visible; destroy takes it from the layer.
    circle.hide();
    check!(!circle.is_visible().as_bool());
    circle.show();
    circle.destroy();
    check!(layer.get_children().len() == 2);
    check!(layer.find_one("Circle").is_none());
    group.remove();
    check!(layer.get_children().len() == 1);

    stage.destroy();
    check!(stage.get_layers().is_empty());
}
