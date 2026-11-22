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

fn col_def(key: &str, hdr: &str) -> JsValue {
    JsValue::object()
        .with("accessorKey", key)
        .with("header", hdr)
}

fn node_row(id: &str, host: &str, region: &str, lat: f64, thr: f64, up: f64) -> JsValue {
    JsValue::object()
        .with("id", id)
        .with("host", host)
        .with("region", region)
        .with("latency", lat)
        .with("throughput", thr)
        .with("uptime", up)
}

fn sort_spec(id: &str, desc: bool) -> ColumnSort {
    ColumnSort::default().with_id(id).with_desc(desc)
}

struct State {
    table: TableCore<TableFeatures, JsValue>,
    wrap: dom::HTMLElement,
    meta: dom::HTMLElement,
}

fn render(st: &State, label: &str) {
    let rows = st.table.get_row_model().rows();
    let cols = st.table.get_all_leaf_columns();

    let mut html = String::from("<table><thead><tr>");
    for col in cols.iter() {
        let cid = col.id();
        let hdr = js::unchecked_cast::<MakeObjectMapResult<JsValue>>(&col.column_def())
            .at("header")
            .expect("missing column header")
            .as_string();
        let is_num = cid == "latency" || cid == "throughput" || cid == "uptime";
        html.push_str(if is_num { "<th class=\"num\">" } else { "<th>" });
        esc_html(&hdr, &mut html);
        let csort: ColumnRowSorting<TableFeatures, JsValue> = js::unchecked_cast(&col);
        match csort.get_is_sorted() {
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
        let host = r.get_value::<JsValue>("host").as_string();
        if i == 0 {
            top_host = host.clone();
        }
        html.push_str("<tr><td>");
        esc_html(&r.get_value::<JsValue>("id").as_string(), &mut html);
        html.push_str("</td><td>");
        esc_html(&host, &mut html);
        html.push_str("</td><td>");
        esc_html(&r.get_value::<JsValue>("region").as_string(), &mut html);
        let _ = write!(
            html,
            "</td><td class=\"num\">{:.1}</td><td class=\"num\">{:.0}</td><td class=\"num\">{:.2}%</td></tr>",
            r.get_value::<JsValue>("latency").as_f64(),
            r.get_value::<JsValue>("throughput").as_f64(),
            r.get_value::<JsValue>("uptime").as_f64(),
        );
    }
    html.push_str("</tbody></table>");
    st.wrap.set_inner_html(&html);

    let msg = format!("{label} \u{00B7} {} rows \u{00B7} top node: {top_host}", rows.len());
    st.meta.set_text_content(Some(&msg));
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let sort_fns = JsValue::object()
        .with("alphanumeric", sort_fn_alphanumeric())
        .with("basic", sort_fn_basic())
        .with("text", sort_fn_text());
    let features = JsValue::object()
        .with("coreReactivityFeature", make_reactivity())
        .with("rowSortingFeature", row_sorting_feature())
        .with("coreRowModel", create_core_row_model::<TableFeatures, JsValue>())
        .with("sortedRowModel", create_sorted_row_model::<TableFeatures, JsValue>())
        .with("sortFns", sort_fns);

    let data = JsArray::<JsValue>::from([
        node_row("N-01", "eu-west-1a", "CH", 12.4, 4200.0, 99.98),
        node_row("N-02", "eu-west-1b", "CH", 8.7, 5100.0, 99.99),
        node_row("N-03", "eu-central-1a", "DE", 15.2, 3900.0, 99.91),
        node_row("N-04", "eu-central-1b", "DE", 6.3, 6400.0, 99.97),
        node_row("N-05", "eu-north-1a", "SE", 19.8, 2800.0, 99.85),
        node_row("N-06", "eu-north-1b", "SE", 11.1, 4750.0, 99.95),
        node_row("N-07", "us-east-1a", "US", 28.5, 7100.0, 99.94),
        node_row("N-08", "us-east-1b", "US", 21.0, 5900.0, 99.96),
    ]);
    let cols = JsArray::<JsValue>::from([
        col_def("id", "Node"),
        col_def("host", "Host"),
        col_def("region", "Region"),
        col_def("latency", "Latency (ms)"),
        col_def("throughput", "Req/s"),
        col_def("uptime", "Uptime %"),
    ]);

    let init_sort = JsArray::<ColumnSort>::from([sort_spec("region", false), sort_spec("latency", false)]);
    let init_state = JsValue::object().with("sorting", init_sort);
    let opts = JsValue::object()
        .with("features", features)
        .with("data", data)
        .with("columns", cols)
        .with("initialState", init_state)
        .unchecked_into::<TableOptionsCore<TableFeatures, JsValue>>();

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
            let tsort: TableRowSorting<TableFeatures, JsValue> = js::unchecked_cast(&st_c.table);
            let (next, label) = match idx {
                0 => (vec![sort_spec("region", false), sort_spec("latency", false)], "Sort: region asc, latency asc"),
                1 => (vec![sort_spec("latency", false)], "Sort: latency asc"),
                2 => (vec![sort_spec("throughput", true)], "Sort: throughput desc"),
                3 => (vec![sort_spec("host", false)], "Sort: host asc"),
                _ => (vec![], "Sort: unsorted (data order)"),
            };
            tsort.set_sorting(Updater::T(JsArray::from(next)));
            render(&st_c, label);
        });
        btn.add_event_listener(dom::HTMLElementEventMap::CLICK, &cb, None);
    }

    render(&st, "Sort: region asc, latency asc");
}
