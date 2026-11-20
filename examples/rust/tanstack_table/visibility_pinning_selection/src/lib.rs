#![no_main]

use dom::ElementMethods;
use std::cell::Cell;
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
    let run_it = FnFn::new(|f: VoidFn| {
        call_void_fn(&f);
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

fn task_row(id: &str, task: &str, owner: &str, zone: &str, cpu: f64, status: &str) -> JsValue {
    JsValue::from(&obj(&[
        ("id", &JsValue::from(id)),
        ("task", &JsValue::from(task)),
        ("owner", &JsValue::from(owner)),
        ("zone", &JsValue::from(zone)),
        ("cpu", &JsValue::from(cpu)),
        ("status", &JsValue::from(status)),
    ]))
}

use std::cell::RefCell;

struct State {
    table: TableCore<TableFeatures, JsValue>,
    wrap: dom::HTMLElement,
    meta: dom::HTMLElement,
    zone_vis: Cell<bool>,
    owner_vis: Cell<bool>,
}

fn apply_vis(st: &State) {
    let cv = ColumnVisibilityState::create();
    cv.set_at("zone", st.zone_vis.get());
    cv.set_at("owner", st.owner_vis.get());
    let vis = TableColumnVisibility::<TableFeatures, JsValue>(st.table.0.clone(), PhantomData);
    call_table_column_visibility_set_column_visibility(&vis.set_column_visibility(), Updater::T(cv));
}

fn render(st: &State) {
    let pin = TableColumnPinning::<TableFeatures, JsValue>(st.table.0.clone(), PhantomData);
    let vis = TableColumnVisibility::<TableFeatures, JsValue>(st.table.0.clone(), PhantomData);
    let sel = TableRowSelection::<TableFeatures, JsValue>(st.table.0.clone(), PhantomData);

    let groups = [
        (true, call_table_column_pinning_get_center_leaf_columns(&pin.get_start_visible_leaf_columns())),
        (false, call_table_column_pinning_get_center_leaf_columns(&pin.get_center_visible_leaf_columns())),
        (true, call_table_column_pinning_get_center_leaf_columns(&pin.get_end_visible_leaf_columns())),
    ];

    let mut col_ids = Vec::new();
    let mut html = String::from("<table><thead><tr><th>Sel</th>");
    for (pinned, grp) in &groups {
        for col in grp.iter() {
            let cid = col.id();
            let hdr = MakeObjectMapResult::<JsValue>(col.column_def().0.clone(), PhantomData)
                .at("header")
                .unwrap()
                .as_string();
            html.push_str(if cid == "cpu" { "<th class=\"num\">" } else { "<th>" });
            esc_html(&hdr, &mut html);
            if *pinned {
                html.push_str("<span class=\"pin-tag\">PIN</span>");
            }
            html.push_str("</th>");
            col_ids.push(cid);
        }
    }
    html.push_str("</tr></thead><tbody>");

    let rows = call_table_row_models_get_core_row_model(&st.table.get_row_model()).rows();
    for r in rows.iter() {
        let gv = r.get_value();
        let is_sel = call_void_to_boolean_fn(&RowRowSelection(r.0.clone()).get_is_selected());
        html.push_str(if is_sel { "<tr class=\"selected-row\"><td>&#9632;</td>" } else { "<tr><td>&#9633;</td>" });
        for cid in &col_ids {
            if cid == "cpu" {
                let _ = write!(html, "<td class=\"num\">{:.0}%</td>", call_string_to_t_value_fn(&gv, "cpu").as_f64());
            } else {
                html.push_str("<td>");
                esc_html(&call_string_to_t_value_fn(&gv, cid).as_string(), &mut html);
                html.push_str("</td>");
            }
        }
        html.push_str("</tr>");
    }
    html.push_str("</tbody></table>");
    st.wrap.set_inner_html(&html);

    let vis_n = call_table_column_visibility_get_visible_flat_columns(&vis.get_visible_leaf_columns()).len();
    let all_n = call_table_columns_get_all_columns(&st.table.get_all_leaf_columns()).len();
    let sel_n = call_table_row_selection_get_filtered_selected_row_model(&sel.get_selected_row_model()).rows().len();
    let msg = format!(
        "{vis_n} / {all_n} visible columns \u{00B7} start:id end:status pinned \u{00B7} {sel_n} / {} rows selected",
        rows.len()
    );
    st.meta.set_text_content(Some(&msg));
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let features = obj(&[
        ("coreReactivityFeature", &make_reactivity()),
        ("columnVisibilityFeature", &JsValue(column_visibility_feature().0.clone())),
        ("columnPinningFeature", &JsValue(column_pinning_feature().0.clone())),
        ("rowSelectionFeature", &JsValue(row_selection_feature().0.clone())),
        ("coreRowModel", &JsValue(create_core_row_model::<TableFeatures, JsValue>().0.clone())),
    ]);

    let data = JsArray::<JsValue>::from(vec![
        task_row("T-101", "ingest-wal-sync", "a.meyer", "eu-ch-1", 42.0, "running"),
        task_row("T-102", "index-compact", "b.rossi", "eu-ch-2", 68.0, "running"),
        task_row("T-103", "snapshot-verify", "c.keller", "eu-de-1", 15.0, "queued"),
        task_row("T-104", "ledger-reconcile", "d.weber", "eu-de-2", 79.0, "running"),
        task_row("T-105", "cert-rotation", "e.morel", "eu-fr-1", 24.0, "ready"),
        task_row("T-106", "shard-rebalance", "f.novak", "eu-fr-2", 55.0, "running"),
        task_row("T-107", "backup-archive", "g.steiner", "eu-ch-1", 31.0, "ready"),
    ]);
    let cols = JsArray::<JsValue>::from(vec![
        col_def("id", "Task ID"),
        col_def("task", "Workload"),
        col_def("owner", "Owner"),
        col_def("zone", "Zone"),
        col_def("cpu", "CPU %"),
        col_def("status", "Status"),
    ]);

    let cv = ColumnVisibilityState::create();
    cv.set_at("zone", false);
    let cp = ColumnPinningState::create();
    cp.set_start(JsArray::from(vec![String::from("id")]));
    cp.set_end(JsArray::from(vec![String::from("status")]));
    let rs = RowSelectionState::create();
    rs.set_at("0", true);
    rs.set_at("2", true);
    rs.set_at("4", true);

    let init_state = obj(&[
        ("columnVisibility", &JsValue(cv.0.clone())),
        ("columnPinning", &JsValue(cp.0.clone())),
        ("rowSelection", &JsValue(rs.0.clone())),
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
        zone_vis: Cell::new(false),
        owner_vis: Cell::new(true),
    });

    for (idx, bid) in ["btn-0", "btn-1", "btn-2", "btn-3", "btn-4"].iter().enumerate() {
        let btn = doc.get_element_by_id(bid).unwrap();
        let st_c = st.clone();
        let cb = dom::JsListener::new(move |_: dom::PointerEvent| {
            let pin = TableColumnPinning::<TableFeatures, JsValue>(st_c.table.0.clone(), PhantomData);
            let sel = TableRowSelection::<TableFeatures, JsValue>(st_c.table.0.clone(), PhantomData);
            match idx {
                0 => {
                    st_c.zone_vis.set(!st_c.zone_vis.get());
                    apply_vis(&st_c);
                }
                1 => {
                    st_c.owner_vis.set(!st_c.owner_vis.get());
                    apply_vis(&st_c);
                }
                2 => call_table_row_selection_toggle_all_page_rows_selected(&sel.toggle_all_rows_selected(), Some(true), None),
                3 => call_table_row_selection_toggle_all_page_rows_selected(&sel.toggle_all_rows_selected(), Some(false), None),
                _ => {
                    st_c.zone_vis.set(false);
                    st_c.owner_vis.set(true);
                    apply_vis(&st_c);
                    let reset_cp = ColumnPinningState::create();
                    reset_cp.set_start(JsArray::from(vec![String::from("id")]));
                    reset_cp.set_end(JsArray::from(vec![String::from("status")]));
                    call_table_column_pinning_set_column_pinning(&pin.set_column_pinning(), Updater::T(reset_cp));
                    let reset_rs = RowSelectionState::create();
                    reset_rs.set_at("0", true);
                    reset_rs.set_at("2", true);
                    reset_rs.set_at("4", true);
                    call_table_row_selection_set_row_selection(&sel.set_row_selection(), Updater::T(reset_rs));
                }
            }
            render(&st_c);
        });
        btn.add_event_listener(dom::HTMLElementEventMap::CLICK, &cb, None);
        std::mem::forget(cb);
    }

    render(&st);
}
