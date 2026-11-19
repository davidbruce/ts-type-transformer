#![no_main]

use dom::ElementMethods;
use std::cell::{Cell, RefCell};
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

struct State {
    table: TableCore<TableFeatures, JsValue>,
    wrap: dom::HTMLElement,
    meta: dom::HTMLElement,
    page_idx: Cell<u32>,
    page_size: Cell<u32>,
}

fn render(st: &State) {
    let rows = call_table_row_models_get_core_row_model(&st.table.get_row_model()).rows();
    let total = call_table_row_models_get_core_row_model(&st.table.get_core_row_model()).rows().len();

    let mut html = String::from("<table><thead><tr><th>Entry</th><th>Account</th><th>Category</th><th class=\"num\">Units</th><th class=\"num\">Amount (CHF)</th></tr></thead><tbody>");
    for r in rows.iter() {
        let gv = r.get_value();
        html.push_str("<tr><td>");
        esc_html(&call_string_to_t_value_fn(&gv, "id").as_string(), &mut html);
        html.push_str("</td><td>");
        esc_html(&call_string_to_t_value_fn(&gv, "account").as_string(), &mut html);
        html.push_str("</td><td>");
        esc_html(&call_string_to_t_value_fn(&gv, "category").as_string(), &mut html);
        let _ = write!(
            html,
            "</td><td class=\"num\">{:.0}</td><td class=\"num\">{:.0}</td></tr>",
            call_string_to_t_value_fn(&gv, "units").as_f64(),
            call_string_to_t_value_fn(&gv, "amount").as_f64(),
        );
    }
    html.push_str("</tbody></table>");
    st.wrap.set_inner_html(&html);

    let pag = TableRowPagination::<TableFeatures, JsValue>(st.table.0.clone(), PhantomData);
    let page_cnt = call_void_to_number_fn(&pag.get_page_count()) as u32;
    let msg = format!(
        "Page {} / {} \u{00B7} pageSize {} \u{00B7} showing {} of {} rows",
        st.page_idx.get() + 1,
        page_cnt,
        st.page_size.get(),
        rows.len(),
        total,
    );
    st.meta.set_text_content(Some(&msg));
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let features = obj(&[
        ("coreReactivityFeature", &make_reactivity()),
        ("rowPaginationFeature", &JsValue(row_pagination_feature().0.clone())),
        ("coreRowModel", &JsValue(create_core_row_model::<TableFeatures, JsValue>().0.clone())),
        ("paginatedRowModel", &JsValue(create_paginated_row_model_as::<TableFeatures, JsValue>().0.clone())),
    ]);

    let accounts = ["ops-pool", "clearing-ch", "reserve-de"];
    let cats = ["compute", "storage", "egress", "audit"];
    let mut rows_vec = Vec::with_capacity(18);
    for i in 0..18usize {
        let id = format!("TX-{:03}", i + 1);
        rows_vec.push(JsValue::from(&obj(&[
            ("id", &JsValue::from(id.as_str())),
            ("account", &JsValue::from(accounts[i % 3])),
            ("category", &JsValue::from(cats[i % 4])),
            ("units", &JsValue::from(((i * 7) % 19 + 4) as f64)),
            ("amount", &JsValue::from((((i * 37) % 90 + 15) * 12) as f64)),
        ])));
    }
    let data = JsArray::<JsValue>::from(rows_vec);
    let cols = JsArray::<JsValue>::from(vec![
        col_def("id", "Entry"),
        col_def("account", "Account"),
        col_def("category", "Category"),
        col_def("units", "Units"),
        col_def("amount", "Amount (CHF)"),
    ]);

    let init_pg = PaginationState::create();
    init_pg.set_page_index(0.0);
    init_pg.set_page_size(5.0);
    let init_state = obj(&[("pagination", &JsValue(init_pg.0.clone()))]);
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
        page_idx: Cell::new(0),
        page_size: Cell::new(5),
    });

    for (idx, bid) in ["btn-0", "btn-1", "btn-2", "btn-3", "btn-4"].iter().enumerate() {
        let btn = doc.get_element_by_id(bid).unwrap();
        let st_c = st.clone();
        let cb = dom::JsListener::new(move |_: dom::PointerEvent| {
            let pag = TableRowPagination::<TableFeatures, JsValue>(st_c.table.0.clone(), PhantomData);
            let page_cnt = call_void_to_number_fn(&pag.get_page_count()) as u32;
            match idx {
                0 => {
                    let cur = st_c.page_idx.get();
                    if cur > 0 {
                        st_c.page_idx.set(cur - 1);
                        call_table_row_pagination_set_page_index(&pag.set_page_index(), Updater::T((cur - 1) as f64));
                    }
                }
                1 => {
                    let cur = st_c.page_idx.get();
                    if cur + 1 < page_cnt {
                        st_c.page_idx.set(cur + 1);
                        call_table_row_pagination_set_page_index(&pag.set_page_index(), Updater::T((cur + 1) as f64));
                    }
                }
                _ => {
                    let new_sz = match idx { 2 => 5u32, 3 => 6u32, _ => 9u32 };
                    st_c.page_idx.set(0);
                    st_c.page_size.set(new_sz);
                    call_table_row_pagination_set_page_index(&pag.set_page_index(), Updater::T(0.0));
                    call_table_row_pagination_set_page_index(&pag.set_page_size(), Updater::T(new_sz as f64));
                }
            }
            render(&st_c);
        });
        btn.add_event_listener(dom::HTMLElementEventMap::CLICK, &cb, None);
        std::mem::forget(cb);
    }

    render(&st);
}
