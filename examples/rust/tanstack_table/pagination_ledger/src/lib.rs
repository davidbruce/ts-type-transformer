#![no_main]

use std::cell::{Cell, RefCell};
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

struct State {
    table: TableCore<JsValue>,
    wrap: dom::HTMLElement,
    meta: dom::HTMLElement,
    page_idx: Cell<u32>,
    page_size: Cell<u32>,
}

fn render(st: &State) {
    let rows = st.table.get_row_model().rows();
    let total = st.table.get_core_row_model().rows().len();

    let mut html = String::from("<table><thead><tr><th>Entry</th><th>Account</th><th>Category</th><th class=\"num\">Units</th><th class=\"num\">Amount (CHF)</th></tr></thead><tbody>");
    for r in rows.iter() {
        html.push_str("<tr><td>");
        esc_html(&r.get_value::<JsValue>("id").as_string(), &mut html);
        html.push_str("</td><td>");
        esc_html(&r.get_value::<JsValue>("account").as_string(), &mut html);
        html.push_str("</td><td>");
        esc_html(&r.get_value::<JsValue>("category").as_string(), &mut html);
        let _ = write!(
            html,
            "</td><td class=\"num\">{:.0}</td><td class=\"num\">{:.0}</td></tr>",
            r.get_value::<JsValue>("units").as_f64(),
            r.get_value::<JsValue>("amount").as_f64(),
        );
    }
    html.push_str("</tbody></table>");
    st.wrap.set_inner_html(&html);

    let pag: TableRowPagination<JsValue> = js::unchecked_cast(&st.table);
    let page_cnt = pag.get_page_count() as u32;
    let msg = format!(
        "Page {} / {} \u{00B7} pageSize {} \u{00B7} showing {} of {} rows",
        st.page_idx.get() + 1,
        page_cnt,
        st.page_size.get(),
        rows.len(),
        total,
    );
    st.meta.set_text_content(&msg);
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let features = JsValue::object()
        .with("coreReactivityFeature", make_reactivity())
        .with("rowPaginationFeature", row_pagination_feature())
        .with("coreRowModel", create_core_row_model::<JsValue>())
        .with("paginatedRowModel", create_paginated_row_model_as::<JsValue>());

    let accounts = ["ops-pool", "clearing-ch", "reserve-de"];
    let cats = ["compute", "storage", "egress", "audit"];
    let mut rows_vec = Vec::with_capacity(18);
    for i in 0..18usize {
        let id = format!("TX-{:03}", i + 1);
        rows_vec.push(
            JsValue::object()
                .with("id", id)
                .with("account", accounts[i % 3])
                .with("category", cats[i % 4])
                .with("units", ((i * 7) % 19 + 4) as f64)
                .with("amount", (((i * 37) % 90 + 15) * 12) as f64),
        );
    }
    let data = JsArray::<JsValue>::from(rows_vec);
    let cols = JsArray::<JsValue>::from([
        col_def("id", "Entry"),
        col_def("account", "Account"),
        col_def("category", "Category"),
        col_def("units", "Units"),
        col_def("amount", "Amount (CHF)"),
    ]);

    let init_pg = PaginationState::default()
        .with_page_index(0.0)
        .with_page_size(5.0);
    let init_state = JsValue::object().with("pagination", init_pg);
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
        page_idx: Cell::new(0),
        page_size: Cell::new(5),
    });

    for (idx, bid) in ["btn-0", "btn-1", "btn-2", "btn-3", "btn-4"].iter().enumerate() {
        let btn = doc.get_element_by_id(bid).expect("missing button");
        let st_c = st.clone();
        let cb = dom::JsListener::new(move |_: dom::PointerEvent| {
            let pag: TableRowPagination<JsValue> = js::unchecked_cast(&st_c.table);
            let page_cnt = pag.get_page_count() as u32;
            match idx {
                0 => {
                    let cur = st_c.page_idx.get();
                    if cur > 0 {
                        st_c.page_idx.set(cur - 1);
                        pag.set_page_index(Updater::T((cur - 1) as f64));
                    }
                }
                1 => {
                    let cur = st_c.page_idx.get();
                    if cur + 1 < page_cnt {
                        st_c.page_idx.set(cur + 1);
                        pag.set_page_index(Updater::T((cur + 1) as f64));
                    }
                }
                _ => {
                    let new_sz = match idx { 2 => 5u32, 3 => 6u32, _ => 9u32 };
                    st_c.page_idx.set(0);
                    st_c.page_size.set(new_sz);
                    pag.set_page_index(Updater::T(0.0));
                    pag.set_page_size(Updater::T(new_sz as f64));
                }
            }
            render(&st_c);
        });
        btn.add_event_listener(dom::HTMLElementEventMap::CLICK, &cb, None);
    }

    render(&st);
}
