// SVG from Rust: elements made in the SVG namespace, attributes set, and
// geometry, lengths and paths read back from the browser's layout. Every
// check! must hold.
#![no_main]
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

const SVG_NS: &str = "http://www.w3.org/2000/svg";

// Within a hair: layout is floating point.
fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.01
}

#[unsafe(no_mangle)]
pub extern "C" fn run() {
    let doc = document();
    let body = doc.body();

    // An <svg> made with a namespace: an SVGSVGElement only when the namespace is SVG's.
    let wrong: Element = doc.create_element_ns(Some("http://www.w3.org/1999/xhtml"), "svg", None);
    check!(wrong.namespace_uri().as_deref() == Some("http://www.w3.org/1999/xhtml"));
    let svg: SVGSVGElement = doc.create_element_ns_svg(SVGElementTagNameMap::SVG);
    check!(svg.namespace_uri().as_deref() == Some(SVG_NS) && svg.local_name() == "svg" && svg.tag_name() == "svg");
    svg.set_attribute("width", "200");
    svg.set_attribute("height", "100");
    svg.set_attribute("viewBox", "0 0 400 200");
    body.append_child(svg.clone());
    check!(svg.parent_node() == Some(body.clone().into()));

    // Shapes made typed, and one made generically with the namespace.
    let rect: SVGRectElement = doc.create_element_ns_svg(SVGElementTagNameMap::RECT);
    for (name, value) in [("x", "10"), ("y", "20"), ("width", "30"), ("height", "40"), ("fill", "red")] {
        rect.set_attribute(name, value);
    }
    let circle: SVGCircleElement = doc.create_element_ns_svg(SVGElementTagNameMap::CIRCLE);
    circle.set_attribute("cx", "100");
    circle.set_attribute("cy", "60");
    circle.set_attribute("r", "15");
    let generic = doc.create_element_ns(Some(SVG_NS), "ellipse", None);
    generic.set_attribute("rx", "8");
    generic.set_attribute("ry", "4");
    check!(generic.namespace_uri().as_deref() == Some(SVG_NS) && generic.tag_name() == "ellipse");
    let group: SVGGElement = doc.create_element_ns_svg(SVGElementTagNameMap::G);
    group.set_id("shapes");
    group.append_child(rect.clone());
    group.append_child(circle.clone());
    group.append_child(generic.clone());
    svg.append_child(group.clone());
    check!(svg.child_element_count() == 1.0 && group.child_element_count() == 3.0);
    check!(rect.get_attribute("fill").as_deref() == Some("red") && rect.get_attribute("FILL").is_none());
    check!(svg.query_selector_as::<SVGRectElement>("#shapes > rect") == Some(rect.clone()));
    check!(svg.query_selector_all("circle, ellipse").length() == 2.0);
    check!(svg.get_element_by_id("shapes").is_some());
    // Classes and style work on SVG elements as on HTML ones.
    rect.class_list().add(["bar", "wide"]);
    check!(rect.get_attribute("class").as_deref() == Some("bar wide"));
    rect.style().set_property("stroke", Some("blue"), None);
    check!(get_computed_style(&rect, None).get_property_value("stroke") == "rgb(0, 0, 255)");
    check!(get_computed_style(&rect, None).get_property_value("fill") == "rgb(255, 0, 0)");

    // Lengths: an animated length's base value, in user units.
    check!(svg.width().base_val().value() == 200.0 && svg.height().base_val().value() == 100.0);
    check!(svg.width().base_val().unit_type() == SVGLength::SVG_LENGTHTYPE_NUMBER);
    check!(svg.width().anim_val().value() == 200.0);
    check!(rect.x().base_val().value() == 10.0 && rect.width().base_val().value() == 30.0);
    check!(circle.r().base_val().value() == 15.0 && circle.cx().base_val().value_as_string() == "100");
    // Setting through the attribute shows in the length; setting the length shows in the attribute.
    rect.set_attribute("width", "50");
    check!(rect.width().base_val().value() == 50.0);
    rect.width().base_val().set_value(35.0);
    check!(rect.get_attribute("width").as_deref() == Some("35"));
    rect.width().base_val().set_value_as_string("2in");
    check!(rect.get_attribute("width").as_deref() == Some("2in"));
    check!(rect.width().base_val().value() == 192.0);
    check!(rect.width().base_val().unit_type() == SVGLength::SVG_LENGTHTYPE_IN);
    check!(rect.width().base_val().value_in_specified_units() == 2.0);
    rect.width().base_val().convert_to_specified_units(SVGLength::SVG_LENGTHTYPE_PX);
    check!(rect.get_attribute("width").as_deref() == Some("192px"));
    rect.width().base_val().new_value_specified_units(SVGLength::SVG_LENGTHTYPE_NUMBER, 30.0);
    check!(rect.get_attribute("width").as_deref() == Some("30"));
    // A percentage resolves against the viewport (the viewBox is 400 wide).
    circle.cx().base_val().set_value_as_string("25%");
    check!(circle.cx().base_val().unit_type() == SVGLength::SVG_LENGTHTYPE_PERCENTAGE);
    check!(circle.cx().base_val().value() == 100.0 && circle.cx().base_val().value_in_specified_units() == 25.0);
    // A free-standing length from the root.
    let length = svg.create_svg_length();
    length.set_value_as_string("10mm");
    check!(near(length.value(), 10.0 * 96.0 / 25.4) && length.unit_type() == SVGLength::SVG_LENGTHTYPE_MM);
    // A bad length throws and leaves the value.
    check!(js::attempt(|| length.set_value_as_string("ten")).is_err());
    check!(js::take_error().is_none() && length.unit_type() == SVGLength::SVG_LENGTHTYPE_MM);

    // viewBox: an animated rect.
    let view_box = svg.view_box().base_val();
    check!(view_box.x() == 0.0 && view_box.y() == 0.0 && view_box.width() == 400.0 && view_box.height() == 200.0);
    svg.set_attribute("viewBox", "-10 -20 800 400");
    check!(svg.view_box().base_val().x() == -10.0 && svg.view_box().base_val().height() == 400.0);
    svg.set_attribute("viewBox", "0 0 400 200");
    check!(svg.view_box().base_val().width() == 400.0);
    // The animated value follows the base value when nothing animates it.
    check!(svg.view_box().anim_val().width() == 400.0);

    // getBBox: the box of the drawn shape, in its own units.
    let bbox = rect.get_b_box(None);
    check!(bbox.x() == 10.0 && bbox.y() == 20.0 && bbox.width() == 30.0 && bbox.height() == 40.0);
    let cbox = circle.get_b_box(None);
    check!(cbox.x() == 85.0 && cbox.y() == 45.0 && cbox.width() == 30.0 && cbox.height() == 30.0);
    // A group's box holds all of its children (the ellipse is at the origin: 0..8 by 0..4, centered).
    let all = group.get_b_box(None);
    check!(all.x() == -8.0 && all.y() == -4.0 && all.width() == 123.0 && all.height() == 79.0);
    // Moving a shape moves its box.
    rect.set_attribute("x", "15");
    check!(rect.get_b_box(None).x() == 15.0);
    rect.set_attribute("x", "10");
    // A transform moves where it is drawn, not what getBBox says.
    group.set_attribute("transform", "translate(10 20)");
    check!(group.get_b_box(None).x() == -8.0);
    let ctm = rect.get_ctm().unwrap();
    // The matrix to the viewport includes the viewBox scale (400 units in 200px) and the translate.
    check!(near(ctm.a(), 0.5) && near(ctm.d(), 0.5) && near(ctm.e(), 5.0) && near(ctm.f(), 10.0));
    check!(group.transform().base_val().number_of_items() == 1.0);
    // The browser's own layout of it.
    let screen = rect.get_screen_ctm().unwrap();
    check!(near(screen.a(), 0.5) && near(screen.d(), 0.5));
    let client = rect.get_bounding_client_rect();
    check!(client.width() > 14.0 && near(client.width(), 15.0) && near(client.height(), 20.0));
    group.remove_attribute("transform");

    // Paths: length, points along it, and hit tests.
    let path: SVGPathElement = doc.create_element_ns_svg(SVGElementTagNameMap::PATH);
    path.set_attribute("d", "M 0 0 L 30 40");
    path.set_attribute("stroke", "black");
    svg.append_child(path.clone());
    check!(near(path.get_total_length(), 50.0));
    let midpoint = path.get_point_at_length(25.0);
    check!(near(midpoint.x(), 15.0) && near(midpoint.y(), 20.0));
    check!(near(path.get_point_at_length(0.0).x(), 0.0) && near(path.get_point_at_length(50.0).y(), 40.0));
    path.set_attribute("d", "M 0 0 h 100 v 100 h -100 z");
    check!(near(path.get_total_length(), 400.0));
    path.set_attribute("d", "M 10 10 C 20 -10, 40 -10, 50 10");
    let curved = path.get_total_length();
    check!(curved > 40.0 && curved < 60.0);
    // A circle drawn as a path: 2*pi*r.
    path.set_attribute("d", "M 50 0 A 50 50 0 1 1 50 100 A 50 50 0 1 1 50 0");
    check!((path.get_total_length() - 2.0 * std::f64::consts::PI * 50.0).abs() < 0.5);
    // pathLength rescales the length the attribute reports, not getTotalLength.
    path.set_attribute("pathLength", "100");
    check!(path.path_length().base_val() == 100.0);
    check!((path.get_total_length() - 314.159).abs() < 0.5);
    // Hit tests in the shape's own coordinates.
    let inside = DOMPointInit::create();
    inside.set_x(Some(15.0));
    inside.set_y(Some(30.0));
    check!(rect.is_point_in_fill(Some(&inside)) && !rect.is_point_in_stroke(Some(&inside)));
    let outside = DOMPointInit::create();
    outside.set_x(Some(500.0));
    outside.set_y(Some(500.0));
    check!(!rect.is_point_in_fill(Some(&outside)));
    check!(circle.is_point_in_fill(Some(&{ let p = DOMPointInit::create(); p.set_x(Some(100.0)); p.set_y(Some(60.0)); p })));
    svg.remove_child(path.clone());

    // Text has measurable extent.
    let label: SVGTextElement = doc.create_element_ns_svg(SVGElementTagNameMap::TEXT);
    label.set_attribute("x", "10");
    label.set_attribute("y", "90");
    label.set_attribute("font-size", "20");
    label.set_text_content(Some("Hello"));
    svg.append_child(label.clone());
    check!(label.get_number_of_chars() == 5.0 && label.text_content() == "Hello");
    check!(label.get_computed_text_length() > 20.0);
    let box_ = label.get_b_box(None);
    check!(box_.x() == 10.0 && box_.width() > 20.0 && box_.height() > 10.0 && box_.y() < 90.0);
    check!(label.get_start_position_of_char(0.0).x() == 10.0);
    check!(label.get_start_position_of_char(1.0).x() > 10.0);

    // Removed elements are gone from the drawing; the group's box shrinks.
    group.remove_child(circle.clone());
    check!(group.get_b_box(None).width() < 123.0);
    check!(svg.query_selector_all("circle").length() == 0.0);
    // The root's own geometry in the page.
    let page = svg.get_bounding_client_rect();
    check!(page.width() == 200.0 && page.height() == 100.0);
    check!(svg.get_screen_ctm().is_some_and(|m| near(m.a(), 0.5)));
    // The viewport can be asked for points and rectangles.
    let point = svg.create_svg_point();
    point.set_x(40.0);
    point.set_y(50.0);
    check!(point.x() == 40.0 && point.y() == 50.0);
    check!(svg.create_svg_rect().width() == 0.0);
}
