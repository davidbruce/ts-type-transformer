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
    let writable = CreateWritableAtomFn::new(|initial: JsValue, _| {
        let value = Rc::new(RefCell::new(initial));
        let (read, write) = (value.clone(), value);
        let get = GetFilterValueFn::new(move || read.borrow().clone());
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
        CreateReadonlyAtomFn::new(|compute: VoidToTFn<JsValue>, _| {
            atom(JsValue::from(&compute), None).unchecked_into::<ReadonlyAtom<JsValue>>()
        });
    let untrack = UntrackFn::new(|f: VoidToTFn<JsValue>| f.call());
    let run_it = FnFn::new(|f: VoidFn| {
        f.call();
    });
    let noop = AddSubscriptionFn::new(|_| {});
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

fn col_def(key: &str, hdr: &str, agg_fn: Option<&str>) -> JsValue {
    let mut o = JsValue::object()
        .with("accessorKey", key)
        .with("header", hdr);
    if let Some(aname) = agg_fn {
        o = o.with("aggregationFn", aname);
    }
    o
}

fn host_row(region: &str, host: &str, role: &str, cpu: f64, mem: f64) -> JsValue {
    JsValue::object()
        .with("region", region)
        .with("host", host)
        .with("role", role)
        .with("cpu", cpu)
        .with("mem", mem)
        .with("nodes", 1.0)
}

struct State {
    table: TableCore<JsValue>,
    wrap: dom::HTMLElement,
    meta: dom::HTMLElement,
    grp_col: RefCell<String>,
}

fn render(st: &State) {
    let rows = st.table.get_row_model().rows();
    let grp_col = st.grp_col.borrow().clone();

    let mut html = String::from(
        "<table><thead><tr><th>Group / Host</th><th>Region</th><th>Role</th><th class=\"num\">CPU %</th><th class=\"num\">Mem GB</th><th class=\"num\">Nodes</th></tr></thead><tbody>",
    );
    let mut group_cnt = 0usize;
    for r in rows.iter() {
        let is_grp = js::unchecked_cast::<RowColumnGrouping>(&r).get_is_grouped();
        let cpu = r.get_value::<JsValue>("cpu").as_f64();
        let mem = r.get_value::<JsValue>("mem").as_f64();
        let nodes = r.get_value::<JsValue>("nodes").as_f64();
        if is_grp {
            group_cnt += 1;
            let is_exp = js::unchecked_cast::<RowRowExpanding>(&r).get_is_expanded();
            let marker = if is_exp { "[-]" } else { "[+]" };
            let sub_n = r.sub_rows().len();
            let gval = r.get_value::<JsValue>(&grp_col).as_string();
            let reg = r.get_value::<JsValue>("region").as_string();
            let role = r.get_value::<JsValue>("role").as_string();
            html.push_str("<tr class=\"group-row\"><td>");
            let _ = write!(html, "{marker} ");
            esc_html(&grp_col, &mut html);
            html.push('=');
            esc_html(&gval, &mut html);
            let _ = write!(html, " ({sub_n})</td><td>");
            esc_html(&reg, &mut html);
            html.push_str("</td><td>");
            esc_html(&role, &mut html);
            let _ = write!(
                html,
                "</td><td class=\"num\">{cpu:.1}</td><td class=\"num\">{mem:.0}</td><td class=\"num\">{nodes:.0}</td></tr>"
            );
        } else {
            html.push_str("<tr><td class=\"child-indent\">");
            esc_html(&r.get_value::<JsValue>("host").as_string(), &mut html);
            html.push_str("</td><td>");
            esc_html(&r.get_value::<JsValue>("region").as_string(), &mut html);
            html.push_str("</td><td>");
            esc_html(&r.get_value::<JsValue>("role").as_string(), &mut html);
            let _ = write!(
                html,
                "</td><td class=\"num\">{cpu:.0}</td><td class=\"num\">{mem:.0}</td><td class=\"num\">{nodes:.0}</td></tr>"
            );
        }
    }
    html.push_str("</tbody></table>");
    st.wrap.set_inner_html(&html);

    let msg = format!(
        "Grouped by {grp_col} \u{00B7} {group_cnt} groups \u{00B7} {} visible rows",
        rows.len()
    );
    st.meta.set_text_content(&msg);
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let agg_fns = JsValue::object()
        .with("sum", aggregation_fn_sum())
        .with("mean", aggregation_fn_mean())
        .with("count", aggregation_fn_count());
    let features = JsValue::object()
        .with("coreReactivityFeature", make_reactivity())
        .with("columnGroupingFeature", column_grouping_feature())
        .with("rowAggregationFeature", row_aggregation_feature())
        .with("rowExpandingFeature", row_expanding_feature())
        .with("coreRowModel", create_core_row_model::<JsValue>())
        .with("groupedRowModel", create_grouped_row_model_as::<JsValue>())
        .with(
            "expandedRowModel",
            create_expanded_row_model_as::<JsValue>(),
        )
        .with("aggregationFns", agg_fns);

    let data = JsArray::<JsValue>::from([
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
    let cols = JsArray::<JsValue>::from([
        col_def("region", "Region", None),
        col_def("host", "Host", None),
        col_def("role", "Role", None),
        col_def("cpu", "CPU % (mean)", Some("mean")),
        col_def("mem", "Mem GB (sum)", Some("sum")),
        col_def("nodes", "Nodes (count)", Some("count")),
    ]);

    let init_grp = JsArray::<String>::from(["region"]);
    let init_state = JsValue::object()
        .with("grouping", init_grp)
        .with("expanded", true);
    let opts = JsValue::object()
        .with("features", features)
        .with("data", data)
        .with("columns", cols)
        .with("initialState", init_state)
        .unchecked_into::<TableOptionsCore<JsValue>>();

    let doc = dom::document();
    let st = Rc::new(State {
        table: construct_table(&opts),
        wrap: doc
            .get_element_by_id("table-wrap")
            .expect("missing #table-wrap"),
        meta: doc.get_element_by_id("meta").expect("missing #meta"),
        grp_col: RefCell::new(String::from("region")),
    });

    for (idx, bid) in ["btn-0", "btn-1", "btn-2", "btn-3"].iter().enumerate() {
        let btn = doc.get_element_by_id(bid).expect("missing button");
        let st_c = st.clone();
        let cb = dom::JsListener::new(move |_: dom::PointerEvent| {
            let grp: TableColumnGrouping<JsValue> = js::unchecked_cast(&st_c.table);
            let exp: TableRowExpanding<JsValue> = js::unchecked_cast(&st_c.table);
            match idx {
                0 => exp.toggle_all_rows_expanded_with(true),
                1 => exp.toggle_all_rows_expanded_with(false),
                _ => {
                    let key = if idx == 2 { "region" } else { "role" };
                    *st_c.grp_col.borrow_mut() = String::from(key);
                    grp.set_grouping(Updater::T(JsArray::from([key])));
                    exp.set_expanded(Updater::T(ExpandedState::Boolean(true)));
                }
            }
            render(&st_c);
        });
        btn.add_event_listener(dom::HTMLElementEventMap::CLICK, &cb, None);
    }

    render(&st);
}
