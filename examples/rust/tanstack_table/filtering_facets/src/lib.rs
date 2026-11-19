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

fn col_def(key: &str, hdr: &str, filter_fn: Option<&str>) -> JsValue {
    let k_v = JsValue::from(key);
    let h_v = JsValue::from(hdr);
    let f_v;
    if let Some(fname) = filter_fn {
        f_v = JsValue::from(fname);
        JsValue::from(&obj(&[("accessorKey", &k_v), ("header", &h_v), ("filterFn", &f_v)]))
    } else {
        JsValue::from(&obj(&[("accessorKey", &k_v), ("header", &h_v)]))
    }
}

fn svc_row(id: &str, svc: &str, tier: &str, region: &str, reqs: f64, errs: f64) -> JsValue {
    JsValue::from(&obj(&[
        ("id", &JsValue::from(id)),
        ("service", &JsValue::from(svc)),
        ("tier", &JsValue::from(tier)),
        ("region", &JsValue::from(region)),
        ("requests", &JsValue::from(reqs)),
        ("errors", &JsValue::from(errs)),
    ]))
}

fn filter_spec(id: &str, value: JsValue) -> ColumnFilter {
    let f = ColumnFilter::create();
    f.set_id(id);
    f.set_value(value);
    f
}

struct State {
    table: TableCore<TableFeatures, JsValue>,
    wrap: dom::HTMLElement,
    meta: dom::HTMLElement,
}

fn render(st: &State, label: &str) {
    let rows = call_table_row_models_get_core_row_model(&st.table.get_row_model()).rows();
    let total = call_table_row_models_get_core_row_model(&st.table.get_core_row_model()).rows().len();

    let mut html = String::from("<table><thead><tr><th>ID</th><th>Service</th><th>Tier</th><th>Region</th><th class=\"num\">Req/s</th><th class=\"num\">Errors</th></tr></thead><tbody>");
    for r in rows.iter() {
        let gv = r.get_value();
        html.push_str("<tr><td>");
        esc_html(&call_string_to_t_value_fn(&gv, "id").as_string(), &mut html);
        html.push_str("</td><td>");
        esc_html(&call_string_to_t_value_fn(&gv, "service").as_string(), &mut html);
        html.push_str("</td><td>");
        esc_html(&call_string_to_t_value_fn(&gv, "tier").as_string(), &mut html);
        html.push_str("</td><td>");
        esc_html(&call_string_to_t_value_fn(&gv, "region").as_string(), &mut html);
        let _ = write!(
            html,
            "</td><td class=\"num\">{:.0}</td><td class=\"num\">{:.0}</td></tr>",
            call_string_to_t_value_fn(&gv, "requests").as_f64(),
            call_string_to_t_value_fn(&gv, "errors").as_f64(),
        );
    }
    html.push_str("</tbody></table>");
    st.wrap.set_inner_html(&html);

    let msg = format!("{label} \u{00B7} showing {} / {total} services", rows.len());
    st.meta.set_text_content(Some(&msg));
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let filter_fns = obj(&[
        ("includesString", &JsValue(filter_fn_includes_string().0.clone())),
        ("equalsString", &JsValue(filter_fn_equals_string().0.clone())),
        ("greaterThanOrEqualTo", &JsValue(filter_fn_greater_than_or_equal_to().0.clone())),
    ]);
    let features = obj(&[
        ("coreReactivityFeature", &make_reactivity()),
        ("columnFilteringFeature", &JsValue(column_filtering_feature().0.clone())),
        ("globalFilteringFeature", &JsValue(global_filtering_feature().0.clone())),
        ("coreRowModel", &JsValue(create_core_row_model::<TableFeatures, JsValue>().0.clone())),
        ("filteredRowModel", &JsValue(create_filtered_row_model_as::<TableFeatures, JsValue>().0.clone())),
        ("filterFns", &JsValue::from(&filter_fns)),
    ]);

    let data = JsArray::<JsValue>::from(vec![
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
    let cols = JsArray::<JsValue>::from(vec![
        col_def("id", "ID", None),
        col_def("service", "Service", None),
        col_def("tier", "Tier", Some("equalsString")),
        col_def("region", "Region", None),
        col_def("requests", "Req/s", Some("greaterThanOrEqualTo")),
        col_def("errors", "Errors", Some("greaterThanOrEqualTo")),
    ]);

    let init_filters = JsArray::<ColumnFilter>::from(vec![
        filter_spec("tier", JsValue::from("core")),
        filter_spec("requests", JsValue::from(400.0)),
    ]);
    let init_state = obj(&[("columnFilters", &JsValue(init_filters.0.clone()))]);
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
            let cf = TableColumnFiltering(st_c.table.0.clone());
            let gf = TableGlobalFiltering::<TableFeatures, JsValue>(st_c.table.0.clone(), PhantomData);
            let label = match idx {
                0 => {
                    call_updater_fn(&gf.set_global_filter(), Updater::T(JsValue::from("")));
                    let next = vec![filter_spec("tier", JsValue::from("core")), filter_spec("requests", JsValue::from(400.0))];
                    call_table_column_filtering_set_column_filters(&cf.set_column_filters(), Updater::T(JsArray::from(next)));
                    "Filter: tier = core & requests >= 400"
                }
                1 => {
                    call_updater_fn(&gf.set_global_filter(), Updater::T(JsValue::from("")));
                    call_table_column_filtering_set_column_filters(&cf.set_column_filters(), Updater::T(JsArray::from(Vec::<ColumnFilter>::new())));
                    "Filter: all services"
                }
                2 => {
                    call_updater_fn(&gf.set_global_filter(), Updater::T(JsValue::from("")));
                    let next = vec![filter_spec("tier", JsValue::from("edge"))];
                    call_table_column_filtering_set_column_filters(&cf.set_column_filters(), Updater::T(JsArray::from(next)));
                    "Filter: tier = edge"
                }
                3 => {
                    call_table_column_filtering_set_column_filters(&cf.set_column_filters(), Updater::T(JsArray::from(Vec::<ColumnFilter>::new())));
                    call_updater_fn(&gf.set_global_filter(), Updater::T(JsValue::from("auth")));
                    "Global search: \"auth\""
                }
                _ => {
                    call_updater_fn(&gf.set_global_filter(), Updater::T(JsValue::from("")));
                    let next = vec![filter_spec("errors", JsValue::from(5.0))];
                    call_table_column_filtering_set_column_filters(&cf.set_column_filters(), Updater::T(JsArray::from(next)));
                    "Filter: errors >= 5"
                }
            };
            render(&st_c, label);
        });
        btn.add_event_listener(dom::HTMLElementEventMap::CLICK, &cb, None);
        std::mem::forget(cb);
    }

    render(&st, "Filter: tier = core & requests >= 400");
}
