#![no_main]

use dom::ElementMethods;
use std::cell::RefCell;
use std::fmt::Write as _;
use std::marker::PhantomData;
use std::rc::Rc;
use tanstack_table_core::*;

fn obj(entries: &[(&str, &JsValue)]) -> MakeObjectMapResult<JsValue> {
    let o = MakeObjectMapResult::<JsValue>::create();
    for (k, v) in entries {
        o.set_at(k, (*v).clone());
    }
    o
}

fn as_array(v: &JsValue) -> JsArray<JsValue> {
    let h = v.0.clone();
    let id = h.0;
    std::mem::forget(h);
    JsArray::<JsValue>::from_handle(id)
}

fn atom(get: JsValue, set: Option<Box<dyn FnMut(JsValue)>>) -> JsHandle {
    let o = MakeObjectMapResult::<JsValue>::create();
    o.set_at("get", get);
    if let Some(mut set) = set {
        o.set_at(
            "set",
            JsValue(AnyFunction::new(move |args: JsValue| set(as_array(&args).get(0))).0.clone()),
        );
    }
    o.set_at("subscribe", JsValue(AnyFunction::new(|_: JsValue| {}).0.clone()));
    o.0.clone()
}

fn make_reactivity() -> JsValue {
    let bindings = TableReactivityBindings::create();
    bindings.set_create_options_store(false);
    bindings.set_wrap_external_atoms(false);
    let writable = TableReactivityBindingsCreateWritableAtom::new(|initial: JsValue, _| {
        let value = Rc::new(RefCell::new(initial));
        let (read, write) = (value.clone(), value);
        let get = ColumnColumnFilteringGetFilterValue::new(move || read.borrow().clone());
        Atom(
            atom(
                JsValue(get.0.clone()),
                Some(Box::new(move |v| {
                    let prev = write.borrow().clone();
                    *write.borrow_mut() = functional_update(Updater::T(v), prev);
                })),
            ),
            PhantomData,
        )
    });
    bindings.set_create_writable_atom(&writable);
    let readonly = TableReactivityBindingsCreateReadonlyAtom::new(|compute: VoidToTFn<JsValue>, _| {
        ReadonlyAtom(atom(JsValue(compute.0.clone()), None), PhantomData)
    });
    bindings.set_create_readonly_atom(&readonly);
    let untrack = TableReactivityBindingsUntrack::new(|f: VoidToTFn<JsValue>| call_void_to_t_fn(&f));
    bindings.set_untrack(&untrack);
    let run_it = FnFn::new(|f: ColumnColumnGroupingToggleGrouping| {
        call_column_column_grouping_toggle_grouping(&f);
    });
    bindings.set_batch(&run_it);
    bindings.set_schedule(&run_it);
    let noop = TableReactivityBindingsAddSubscription::new(|_| {});
    bindings.set_add_subscription(&noop);
    JsValue(bindings.0.clone())
}

fn esc_html(s: &str, out: &mut String) {
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(ch),
        }
    }
}

fn col_def(key: &str, hdr: &str) -> JsValue {
    JsValue::from(&obj(&[
        ("accessorKey", &JsValue::from(key)),
        ("header", &JsValue::from(hdr)),
    ]))
}

fn node_row(id: &str, host: &str, region: &str, lat: f64, thr: f64, up: f64) -> JsValue {
    JsValue::from(&obj(&[
        ("id", &JsValue::from(id)),
        ("host", &JsValue::from(host)),
        ("region", &JsValue::from(region)),
        ("latency", &JsValue::from(lat)),
        ("throughput", &JsValue::from(thr)),
        ("uptime", &JsValue::from(up)),
    ]))
}

fn sort_spec(id: &str, desc: bool) -> ColumnSort {
    let s = ColumnSort::create();
    s.set_id(id);
    s.set_desc(desc);
    s
}

struct State {
    table: TableCore<TableFeatures, JsValue>,
    wrap: dom::HTMLElement,
    meta: dom::HTMLElement,
}

fn render(st: &State, label: &str) {
    let rows = call_table_row_models_get_core_row_model(&st.table.get_row_model()).rows();
    let cols = call_table_columns_get_all_columns(&st.table.get_all_leaf_columns());

    let mut html = String::from("<table><thead><tr>");
    for col in cols.iter() {
        let cid = col.id();
        let hdr = MakeObjectMapResult::<JsValue>(col.column_def().0.clone(), PhantomData)
            .at("header")
            .unwrap()
            .as_string();
        let is_num = cid == "latency" || cid == "throughput" || cid == "uptime";
        html.push_str(if is_num { "<th class=\"num\">" } else { "<th>" });
        esc_html(&hdr, &mut html);
        let csort = ColumnRowSorting::<TableFeatures, JsValue>(col.0.clone(), PhantomData);
        match call_column_row_sorting_get_is_sorted(&csort.get_is_sorted()) {
            BooleanOrSortDirection::SortDirection(SortDirection::Asc) => html.push_str(" &#9650;"),
            BooleanOrSortDirection::SortDirection(SortDirection::Desc) => html.push_str(" &#9660;"),
            _ => {}
        }
        html.push_str("</th>");
    }
    html.push_str("</tr></thead><tbody>");

    let mut top_host = String::from("-");
    for i in 0..rows.len() {
        let r = rows.get(i);
        let gv = r.get_value();
        let host = call_string_to_t_value_fn(&gv, "host").as_string();
        if i == 0 {
            top_host = host.clone();
        }
        html.push_str("<tr><td>");
        esc_html(&call_string_to_t_value_fn(&gv, "id").as_string(), &mut html);
        html.push_str("</td><td>");
        esc_html(&host, &mut html);
        html.push_str("</td><td>");
        esc_html(&call_string_to_t_value_fn(&gv, "region").as_string(), &mut html);
        let _ = write!(
            html,
            "</td><td class=\"num\">{:.1}</td><td class=\"num\">{:.0}</td><td class=\"num\">{:.2}%</td></tr>",
            call_string_to_t_value_fn(&gv, "latency").as_f64(),
            call_string_to_t_value_fn(&gv, "throughput").as_f64(),
            call_string_to_t_value_fn(&gv, "uptime").as_f64(),
        );
    }
    html.push_str("</tbody></table>");
    st.wrap.set_inner_html(&html);

    let msg = format!("{label} \u{00B7} {} rows \u{00B7} top node: {top_host}", rows.len());
    st.meta.set_text_content(Some(&msg));
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let sort_fns = obj(&[
        ("alphanumeric", &JsValue(sort_fn_alphanumeric().0.clone())),
        ("basic", &JsValue(sort_fn_basic().0.clone())),
        ("text", &JsValue(sort_fn_text().0.clone())),
    ]);
    let features = obj(&[
        ("coreReactivityFeature", &make_reactivity()),
        ("rowSortingFeature", &JsValue(row_sorting_feature().0.clone())),
        ("coreRowModel", &JsValue(create_core_row_model::<TableFeatures, JsValue>().0.clone())),
        ("sortedRowModel", &JsValue(create_sorted_row_model::<TableFeatures, JsValue>().0.clone())),
        ("sortFns", &JsValue::from(&sort_fns)),
    ]);

    let data = JsArray::<JsValue>::from(vec![
        node_row("N-01", "eu-west-1a", "CH", 12.4, 4200.0, 99.98),
        node_row("N-02", "eu-west-1b", "CH", 8.7, 5100.0, 99.99),
        node_row("N-03", "eu-central-1a", "DE", 15.2, 3900.0, 99.91),
        node_row("N-04", "eu-central-1b", "DE", 6.3, 6400.0, 99.97),
        node_row("N-05", "eu-north-1a", "SE", 19.8, 2800.0, 99.85),
        node_row("N-06", "eu-north-1b", "SE", 11.1, 4750.0, 99.95),
        node_row("N-07", "us-east-1a", "US", 28.5, 7100.0, 99.94),
        node_row("N-08", "us-east-1b", "US", 21.0, 5900.0, 99.96),
    ]);
    let cols = JsArray::<JsValue>::from(vec![
        col_def("id", "Node"),
        col_def("host", "Host"),
        col_def("region", "Region"),
        col_def("latency", "Latency (ms)"),
        col_def("throughput", "Req/s"),
        col_def("uptime", "Uptime %"),
    ]);

    let init_sort = JsArray::<ColumnSort>::from(vec![sort_spec("region", false), sort_spec("latency", false)]);
    let init_state = obj(&[("sorting", &JsValue(init_sort.0.clone()))]);
    let opts = obj(&[
        ("features", &JsValue::from(&features)),
        ("data", &JsValue(data.0.clone())),
        ("columns", &JsValue(cols.0.clone())),
        ("initialState", &JsValue::from(&init_state)),
    ]);

    let doc = dom::document();
    let st = Rc::new(State {
        table: construct_table(&TableOptionsCore::<TableFeatures, JsValue>(opts.0.clone(), PhantomData)),
        wrap: doc.get_element_by_id("table-wrap").unwrap(),
        meta: doc.get_element_by_id("meta").unwrap(),
    });

    for (idx, bid) in ["btn-0", "btn-1", "btn-2", "btn-3", "btn-4"].iter().enumerate() {
        let btn = doc.get_element_by_id(bid).unwrap();
        let st_c = st.clone();
        let cb = dom::JsListener::new(move |_: dom::PointerEvent| {
            let tsort = TableRowSorting::<TableFeatures, JsValue>(st_c.table.0.clone(), PhantomData);
            let (next, label) = match idx {
                0 => (vec![sort_spec("region", false), sort_spec("latency", false)], "Sort: region asc, latency asc"),
                1 => (vec![sort_spec("latency", false)], "Sort: latency asc"),
                2 => (vec![sort_spec("throughput", true)], "Sort: throughput desc"),
                3 => (vec![sort_spec("host", false)], "Sort: host asc"),
                _ => (vec![], "Sort: unsorted (data order)"),
            };
            call_table_row_sorting_set_sorting(&tsort.set_sorting(), Updater::T(JsArray::from(next)));
            render(&st_c, label);
        });
        btn.add_event_listener(dom::HTMLElementEventMap::CLICK, &cb, None);
        std::mem::forget(cb);
    }

    render(&st, "Sort: region asc, latency asc");
}
