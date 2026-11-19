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

fn col_def(key: &str, hdr: &str, agg_fn: Option<&str>) -> JsValue {
    let k_v = JsValue::from(key);
    let h_v = JsValue::from(hdr);
    let a_v;
    if let Some(aname) = agg_fn {
        a_v = JsValue::from(aname);
        JsValue::from(&obj(&[("accessorKey", &k_v), ("header", &h_v), ("aggregationFn", &a_v)]))
    } else {
        JsValue::from(&obj(&[("accessorKey", &k_v), ("header", &h_v)]))
    }
}

fn host_row(region: &str, host: &str, role: &str, cpu: f64, mem: f64) -> JsValue {
    JsValue::from(&obj(&[
        ("region", &JsValue::from(region)),
        ("host", &JsValue::from(host)),
        ("role", &JsValue::from(role)),
        ("cpu", &JsValue::from(cpu)),
        ("mem", &JsValue::from(mem)),
        ("nodes", &JsValue::from(1.0)),
    ]))
}

struct State {
    table: TableCore<TableFeatures, JsValue>,
    wrap: dom::HTMLElement,
    meta: dom::HTMLElement,
    grp_col: RefCell<String>,
}

fn render(st: &State) {
    let rows = call_table_row_models_get_core_row_model(&st.table.get_row_model()).rows();
    let grp_col = st.grp_col.borrow().clone();

    let mut html = String::from("<table><thead><tr><th>Group / Host</th><th>Region</th><th>Role</th><th class=\"num\">CPU %</th><th class=\"num\">Mem GB</th><th class=\"num\">Nodes</th></tr></thead><tbody>");
    let mut group_cnt = 0usize;
    for r in rows.iter() {
        let gv = r.get_value();
        let is_grp = call_void_to_boolean_fn(&RowColumnGrouping(r.0.clone()).get_is_grouped());
        let cpu = call_string_to_t_value_fn(&gv, "cpu").as_f64();
        let mem = call_string_to_t_value_fn(&gv, "mem").as_f64();
        let nodes = call_string_to_t_value_fn(&gv, "nodes").as_f64();
        if is_grp {
            group_cnt += 1;
            let is_exp = call_void_to_boolean_fn(&RowRowExpanding(r.0.clone()).get_is_expanded());
            let marker = if is_exp { "[-]" } else { "[+]" };
            let sub_n = r.sub_rows().len();
            let gval = call_string_to_t_value_fn(&gv, &grp_col).as_string();
            let reg = call_string_to_t_value_fn(&gv, "region").as_string();
            let role = call_string_to_t_value_fn(&gv, "role").as_string();
            html.push_str("<tr class=\"group-row\"><td>");
            let _ = write!(html, "{marker} ");
            esc_html(&grp_col, &mut html);
            html.push('=');
            esc_html(&gval, &mut html);
            let _ = write!(html, " ({sub_n})</td><td>");
            esc_html(&reg, &mut html);
            html.push_str("</td><td>");
            esc_html(&role, &mut html);
            let _ = write!(html, "</td><td class=\"num\">{cpu:.1}</td><td class=\"num\">{mem:.0}</td><td class=\"num\">{nodes:.0}</td></tr>");
        } else {
            html.push_str("<tr><td class=\"child-indent\">");
            esc_html(&call_string_to_t_value_fn(&gv, "host").as_string(), &mut html);
            html.push_str("</td><td>");
            esc_html(&call_string_to_t_value_fn(&gv, "region").as_string(), &mut html);
            html.push_str("</td><td>");
            esc_html(&call_string_to_t_value_fn(&gv, "role").as_string(), &mut html);
            let _ = write!(html, "</td><td class=\"num\">{cpu:.0}</td><td class=\"num\">{mem:.0}</td><td class=\"num\">{nodes:.0}</td></tr>");
        }
    }
    html.push_str("</tbody></table>");
    st.wrap.set_inner_html(&html);

    let msg = format!("Grouped by {grp_col} \u{00B7} {group_cnt} groups \u{00B7} {} visible rows", rows.len());
    st.meta.set_text_content(Some(&msg));
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let agg_fns = obj(&[
        ("sum", &JsValue(aggregation_fn_sum().0.clone())),
        ("mean", &JsValue(aggregation_fn_mean().0.clone())),
        ("count", &JsValue(aggregation_fn_count().0.clone())),
    ]);
    let features = obj(&[
        ("coreReactivityFeature", &make_reactivity()),
        ("columnGroupingFeature", &JsValue(column_grouping_feature().0.clone())),
        ("rowAggregationFeature", &JsValue(row_aggregation_feature().0.clone())),
        ("rowExpandingFeature", &JsValue(row_expanding_feature().0.clone())),
        ("coreRowModel", &JsValue(create_core_row_model::<TableFeatures, JsValue>().0.clone())),
        ("groupedRowModel", &JsValue(create_grouped_row_model_as::<TableFeatures, JsValue>().0.clone())),
        ("expandedRowModel", &JsValue(create_expanded_row_model_as::<TableFeatures, JsValue>().0.clone())),
        ("aggregationFns", &JsValue::from(&agg_fns)),
    ]);

    let data = JsArray::<JsValue>::from(vec![
        host_row("CH", "zurich-api-01", "api", 72.0, 64.0),
        host_row("CH", "basel-db-01", "db", 58.0, 128.0),
        host_row("CH", "geneva-edge-01", "edge", 35.0, 32.0),
        host_row("DE", "munich-api-02", "api", 84.0, 64.0),
        host_row("DE", "berlin-db-02", "db", 66.0, 128.0),
        host_row("DE", "frankfurt-edge-02", "edge", 42.0, 32.0),
        host_row("FR", "paris-api-03", "api", 60.0, 64.0),
        host_row("FR", "lyon-db-03", "db", 48.0, 128.0),
        host_row("FR", "marseille-edge-03", "edge", 30.0, 32.0),
    ]);
    let cols = JsArray::<JsValue>::from(vec![
        col_def("region", "Region", None),
        col_def("host", "Host", None),
        col_def("role", "Role", None),
        col_def("cpu", "CPU % (mean)", Some("mean")),
        col_def("mem", "Mem GB (sum)", Some("sum")),
        col_def("nodes", "Nodes (count)", Some("count")),
    ]);

    let init_grp = JsArray::<String>::from(vec![String::from("region")]);
    let init_state = obj(&[
        ("grouping", &JsValue(init_grp.0.clone())),
        ("expanded", &JsValue::from(true)),
    ]);
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
        grp_col: RefCell::new(String::from("region")),
    });

    for (idx, bid) in ["btn-0", "btn-1", "btn-2", "btn-3"].iter().enumerate() {
        let btn = doc.get_element_by_id(bid).unwrap();
        let st_c = st.clone();
        let cb = dom::JsListener::new(move |_: dom::PointerEvent| {
            let grp = TableColumnGrouping::<TableFeatures, JsValue>(st_c.table.0.clone(), PhantomData);
            let exp = TableRowExpanding::<TableFeatures, JsValue>(st_c.table.0.clone(), PhantomData);
            match idx {
                0 => call_row_row_expanding_toggle_expanded(&exp.toggle_all_rows_expanded(), Some(true)),
                1 => call_row_row_expanding_toggle_expanded(&exp.toggle_all_rows_expanded(), Some(false)),
                _ => {
                    let key = if idx == 2 { "region" } else { "role" };
                    *st_c.grp_col.borrow_mut() = String::from(key);
                    call_table_column_grouping_set_grouping(&grp.set_grouping(), Updater::T(JsArray::from(vec![String::from(key)])));
                    call_table_row_expanding_set_expanded(&exp.set_expanded(), Updater::T(ExpandedState::Boolean(true)));
                }
            }
            render(&st_c);
        });
        btn.add_event_listener(dom::HTMLElementEventMap::CLICK, &cb, None);
        std::mem::forget(cb);
    }

    render(&st);
}
