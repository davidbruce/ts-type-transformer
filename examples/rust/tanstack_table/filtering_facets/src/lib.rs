#![no_main]

use std::cell::RefCell;
use std::fmt::Write as _;
use std::rc::Rc;
use tanstack_table_core::*;

fn atom(get: JsValue, set: Option<Box<dyn FnMut(JsValue)>>) -> JsValue {
    let mut o = JsValue::object()
        .with("get", get)
        .with("subscribe", AnyFunction::new(|_: JsValue| {}));
    if let Some(mut set) = set {
        o = o.with(
            "set",
            AnyFunction::new(move |args: JsValue| set(args.as_array::<JsValue>().get(0))),
        );
    }
    o
}

fn make_reactivity() -> JsValue {
    let writable = TableReactivityBindingsCreateWritableAtom::new(|initial: JsValue, _| {
        let value = Rc::new(RefCell::new(initial));
        let (read, write) = (value.clone(), value);
        let get = ColumnColumnFilteringGetFilterValue::new(move || read.borrow().clone());
        atom(
            JsValue::from(&get),
            Some(Box::new(move |v| {
                let prev = write.borrow().clone();
                *write.borrow_mut() = functional_update(Updater::T(v), prev);
            })),
        )
        .unchecked_into::<Atom<JsValue>>()
    });
    let readonly =
        TableReactivityBindingsCreateReadonlyAtom::new(|compute: VoidToTFn<JsValue>, _| {
            atom(JsValue::from(&compute), None).unchecked_into::<ReadonlyAtom<JsValue>>()
        });
    let untrack = TableReactivityBindingsUntrack::new(|f: VoidToTFn<JsValue>| f.call());
    let run_it = FnFn::new(|f: VoidFn| {
        f.call();
    });
    let noop = TableReactivityBindingsAddSubscription::new(|_| {});
    let bindings = TableReactivityBindings::default()
        .with_create_options_store(false)
        .with_wrap_external_atoms(false)
        .with_create_writable_atom(&writable)
        .with_create_readonly_atom(&readonly)
        .with_untrack(&untrack)
        .with_batch(&run_it)
        .with_schedule(&run_it)
        .with_add_subscription(&noop);
    JsValue::from(&bindings)
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

fn col_def(key: &str, hdr: &str, filter_fn: Option<&str>) -> JsValue {
    let mut o = JsValue::object()
        .with("accessorKey", key)
        .with("header", hdr);
    if let Some(fname) = filter_fn {
        o = o.with("filterFn", fname);
    }
    o
}

fn svc_row(id: &str, svc: &str, tier: &str, region: &str, reqs: f64, errs: f64) -> JsValue {
    JsValue::object()
        .with("id", id)
        .with("service", svc)
        .with("tier", tier)
        .with("region", region)
        .with("requests", reqs)
        .with("errors", errs)
}

fn filter_spec(id: &str, value: JsValue) -> ColumnFilter {
    ColumnFilter::default().with_id(id).with_value(value)
}

struct State {
    table: TableCore<JsValue>,
    wrap: dom::HTMLElement,
    meta: dom::HTMLElement,
}

fn render(st: &State, label: &str) {
    let rows = st.table.get_row_model().rows();
    let total = st.table.get_core_row_model().rows().len();

    let mut html = String::from("<table><thead><tr><th>ID</th><th>Service</th><th>Tier</th><th>Region</th><th class=\"num\">Req/s</th><th class=\"num\">Errors</th></tr></thead><tbody>");
    for r in rows.iter() {
        html.push_str("<tr><td>");
        esc_html(&r.get_value::<JsValue>("id").as_string(), &mut html);
        html.push_str("</td><td>");
        esc_html(&r.get_value::<JsValue>("service").as_string(), &mut html);
        html.push_str("</td><td>");
        esc_html(&r.get_value::<JsValue>("tier").as_string(), &mut html);
        html.push_str("</td><td>");
        esc_html(&r.get_value::<JsValue>("region").as_string(), &mut html);
        let _ = write!(
            html,
            "</td><td class=\"num\">{:.0}</td><td class=\"num\">{:.0}</td></tr>",
            r.get_value::<JsValue>("requests").as_f64(),
            r.get_value::<JsValue>("errors").as_f64(),
        );
    }
    html.push_str("</tbody></table>");
    st.wrap.set_inner_html(&html);

    let msg = format!("{label} \u{00B7} showing {} / {total} services", rows.len());
    st.meta.set_text_content(&msg);
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let filter_fns = JsValue::object()
        .with("includesString", filter_fn_includes_string())
        .with("equalsString", filter_fn_equals_string())
        .with("greaterThanOrEqualTo", filter_fn_greater_than_or_equal_to());
    let features = JsValue::object()
        .with("coreReactivityFeature", make_reactivity())
        .with("columnFilteringFeature", column_filtering_feature())
        .with("globalFilteringFeature", global_filtering_feature())
        .with("coreRowModel", create_core_row_model::<JsValue>())
        .with("filteredRowModel", create_filtered_row_model_as::<JsValue>())
        .with("filterFns", filter_fns);

    let data = JsArray::<JsValue>::from([
        svc_row("S-01", "auth-gateway", "core", "CH", 620.0, 2.0),
        svc_row("S-02", "session-cache", "core", "CH", 480.0, 1.0),
        svc_row("S-03", "billing-ledger", "core", "DE", 510.0, 6.0),
        svc_row("S-04", "oauth-signer", "core", "FR", 430.0, 3.0),
        svc_row("S-05", "trace-ingestor", "edge", "DE", 290.0, 8.0),
        svc_row("S-06", "cdn-router", "edge", "FR", 840.0, 5.0),
        svc_row("S-07", "static-assets", "edge", "CH", 710.0, 0.0),
        svc_row("S-08", "mail-relay", "batch", "DE", 140.0, 7.0),
        svc_row("S-09", "report-export", "batch", "FR", 210.0, 4.0),
        svc_row("S-10", "audit-auth-log", "core", "US", 360.0, 9.0),
    ]);
    let cols = JsArray::<JsValue>::from([
        col_def("id", "ID", None),
        col_def("service", "Service", None),
        col_def("tier", "Tier", Some("equalsString")),
        col_def("region", "Region", None),
        col_def("requests", "Req/s", Some("greaterThanOrEqualTo")),
        col_def("errors", "Errors", Some("greaterThanOrEqualTo")),
    ]);

    let init_filters = JsArray::<ColumnFilter>::from([
        filter_spec("tier", JsValue::from("core")),
        filter_spec("requests", JsValue::from(400.0)),
    ]);
    let init_state = JsValue::object().with("columnFilters", init_filters);
    let opts = JsValue::object()
        .with("features", features)
        .with("data", data)
        .with("columns", cols)
        .with("initialState", init_state)
        .unchecked_into::<TableOptionsCore<JsValue>>();

    let doc = dom::document();
    let st = Rc::new(State {
        table: construct_table(&opts),
        wrap: doc.get_element_by_id("table-wrap").expect("missing #table-wrap"),
        meta: doc.get_element_by_id("meta").expect("missing #meta"),
    });

    for (idx, bid) in ["btn-0", "btn-1", "btn-2", "btn-3", "btn-4"].iter().enumerate() {
        let btn = doc.get_element_by_id(bid).expect("missing button");
        let st_c = st.clone();
        let cb = dom::JsListener::new(move |_: dom::PointerEvent| {
            let cf: TableColumnFiltering = js::unchecked_cast(&st_c.table);
            let gf: TableGlobalFiltering<JsValue> = js::unchecked_cast(&st_c.table);
            let label = match idx {
                0 => {
                    gf.set_global_filter(Updater::T(JsValue::from("")));
                    let next = [filter_spec("tier", JsValue::from("core")), filter_spec("requests", JsValue::from(400.0))];
                    cf.set_column_filters(Updater::T(JsArray::from(next)));
                    "Filter: tier = core & requests >= 400"
                }
                1 => {
                    gf.set_global_filter(Updater::T(JsValue::from("")));
                    cf.set_column_filters(Updater::T(JsArray::new()));
                    "Filter: all services"
                }
                2 => {
                    gf.set_global_filter(Updater::T(JsValue::from("")));
                    let next = [filter_spec("tier", JsValue::from("edge"))];
                    cf.set_column_filters(Updater::T(JsArray::from(next)));
                    "Filter: tier = edge"
                }
                3 => {
                    cf.set_column_filters(Updater::T(JsArray::new()));
                    gf.set_global_filter(Updater::T(JsValue::from("auth")));
                    "Global search: \"auth\""
                }
                _ => {
                    gf.set_global_filter(Updater::T(JsValue::from("")));
                    let next = [filter_spec("errors", JsValue::from(5.0))];
                    cf.set_column_filters(Updater::T(JsArray::from(next)));
                    "Filter: errors >= 5"
                }
            };
            render(&st_c, label);
        });
        btn.add_event_listener(dom::HTMLElementEventMap::CLICK, &cb, None);
    }

    render(&st, "Filter: tier = core & requests >= 400");
}
