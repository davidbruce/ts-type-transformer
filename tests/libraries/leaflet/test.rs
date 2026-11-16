// Leaflet from Rust, in a page: a map on the page's element, layers on it,
// geometry, and events. No tiles: nothing is fetched. Every check! must hold.
#![no_main]
use bindings::leaflet::*;
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

#[unsafe(no_mangle)]
pub extern "C" fn run() {
    // A map in the page's #map, centred on London.
    let london = lat_lng(51.505, -0.09, None);
    let m = map(test_page::element("map"), None);
    m.set_view(&london, Some(13.0), None);
    check!(m.get_zoom() == 13.0);
    check!(m.get_center().equals(&london, Some(1e-9)));
    check!(m.get_size().x() == 400.0 && m.get_size().y() == 300.0);

    // The centre is the middle of the container.
    let middle = m.lat_lng_to_container_point(&london);
    check!(middle.x() == 200.0 && middle.y() == 150.0);

    // Geometry: distances in metres, bounds that grow.
    let paris = lat_lng(48.8566, 2.3522, None);
    let km = london.distance_to(&paris) / 1000.0;
    check!(km > 340.0 && km < 345.0);
    let bounds = lat_lng_bounds(&london, &london);
    bounds.extend(&paris);
    check!(bounds.contains(&lat_lng(50.0, 1.0, None)));
    check!(bounds.get_north_east().lat() == 51.505);

    // Layers: a marker added to the map and found there, then removed.
    let pin = marker(&paris, None);
    pin.add_to(&m);
    check!(m.has_layer(&pin));
    check!(pin.get_lat_lng().equals(&paris, None));
    let group = layer_group(None, None);
    group.add_layer(&circle_with_radius(&london, 500.0, None));
    group.add_to(&m);
    m.remove_layer(&pin);
    check!(!m.has_layer(&pin) && m.has_layer(&group));

    // Fitting the bounds zooms out to show both cities.
    m.fit_bounds(&bounds, None);
    check!(m.get_zoom() < 13.0);
    check!(m.get_bounds().contains(&paris) && m.get_bounds().contains(&london));

    // An event fired on the map reaches the listener, with its position.
    let clicks = Rc::new(Cell::new(0));
    let on_click = LeafletMouseEventHandlerFn::new({
        let clicks = clicks.clone();
        move |e: LeafletMouseEvent| {
            if e.latlng().lat() > 0.0 {
                clicks.set(clicks.get() + 1);
            }
        }
    });
    m.on_with_evented_add_event_listener_type_click(EventedAddEventListenerTypeClick::Click, &on_click, None);
    let data = LeafletMouseEvent::create();
    data.set_latlng(&london);
    let data = JsValue::from(&data);
    m.fire("click", Some(&data), None);
    m.fire("click", Some(&data), None);
    check!(clicks.get() == 2);
}
