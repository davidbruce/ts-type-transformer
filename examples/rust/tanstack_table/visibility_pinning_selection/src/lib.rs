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

fn task_row(id: &str, task: &str, owner: &str, zone: &str, cpu: f64, status: &str) -> JsValue {
    JsValue::object()
        .with("id", id)
        .with("task", task)
        .with("owner", owner)
        .with("zone", zone)
        .with("cpu", cpu)
        .with("status", status)
}

struct State {
    table: TableCore<TableFeatures, JsValue>,
    wrap: dom::HTMLElement,
    meta: dom::HTMLElement,
    zone_vis: Cell<bool>,
    owner_vis: Cell<bool>,
}

fn apply_vis(st: &State) {
    let cv = ColumnVisibilityState::default();
    cv.set_at("zone", st.zone_vis.get());
    cv.set_at("owner", st.owner_vis.get());
    let vis: TableColumnVisibility<TableFeatures, JsValue> = js::unchecked_cast(&st.table);
    vis.set_column_visibility(Updater::T(cv));
}

fn render(st: &State) {
    let pin: TableColumnPinning<TableFeatures, JsValue> = js::unchecked_cast(&st.table);
    let vis: TableColumnVisibility<TableFeatures, JsValue> = js::unchecked_cast(&st.table);
    let sel: TableRowSelection<TableFeatures, JsValue> = js::unchecked_cast(&st.table);

    let groups = [
        (true, pin.get_start_visible_leaf_columns()),
        (false, pin.get_center_visible_leaf_columns()),
        (true, pin.get_end_visible_leaf_columns()),
    ];

    let mut col_ids = Vec::new();
    let mut html = String::from("<table><thead><tr><th>Sel</th>");
    for (pinned, grp) in &groups {
        for col in grp.iter() {
            let cid = col.id();
            let hdr = js::unchecked_cast::<MakeObjectMapResult<JsValue>>(&col.column_def())
                .at("header")
                .expect("missing column header")
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

    let rows = st.table.get_row_model().rows();
    for r in rows.iter() {
        let is_sel = js::unchecked_cast::<RowRowSelection>(&r).get_is_selected();
        html.push_str(if is_sel { "<tr class=\"selected-row\"><td>&#9632;</td>" } else { "<tr><td>&#9633;</td>" });
        for cid in &col_ids {
            if cid == "cpu" {
                let _ = write!(html, "<td class=\"num\">{:.0}%</td>", r.get_value::<JsValue>("cpu").as_f64());
            } else {
                html.push_str("<td>");
                esc_html(&r.get_value::<JsValue>(cid).as_string(), &mut html);
                html.push_str("</td>");
            }
        }
        html.push_str("</tr>");
    }
    html.push_str("</tbody></table>");
    st.wrap.set_inner_html(&html);

    let vis_n = vis.get_visible_leaf_columns().len();
    let all_n = st.table.get_all_leaf_columns().len();
    let sel_n = sel.get_selected_row_model().rows().len();
    let msg = format!(
        "{vis_n} / {all_n} visible columns \u{00B7} start:id end:status pinned \u{00B7} {sel_n} / {} rows selected",
        rows.len()
    );
    st.meta.set_text_content(&msg);
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let features = JsValue::object()
        .with("coreReactivityFeature", make_reactivity())
        .with("columnVisibilityFeature", column_visibility_feature())
        .with("columnPinningFeature", column_pinning_feature())
        .with("rowSelectionFeature", row_selection_feature())
        .with("coreRowModel", create_core_row_model::<TableFeatures, JsValue>());

    let data = JsArray::<JsValue>::from([
        task_row("T-101", "ingest-wal-sync", "a.meyer", "eu-ch-1", 42.0, "running"),
        task_row("T-102", "index-compact", "b.rossi", "eu-ch-2", 68.0, "running"),
        task_row("T-103", "snapshot-verify", "c.keller", "eu-de-1", 15.0, "queued"),
        task_row("T-104", "ledger-reconcile", "d.weber", "eu-de-2", 79.0, "running"),
        task_row("T-105", "cert-rotation", "e.morel", "eu-fr-1", 24.0, "ready"),
        task_row("T-106", "shard-rebalance", "f.novak", "eu-fr-2", 55.0, "running"),
        task_row("T-107", "backup-archive", "g.steiner", "eu-ch-1", 31.0, "ready"),
    ]);
    let cols = JsArray::<JsValue>::from([
        col_def("id", "Task ID"),
        col_def("task", "Workload"),
        col_def("owner", "Owner"),
        col_def("zone", "Zone"),
        col_def("cpu", "CPU %"),
        col_def("status", "Status"),
    ]);

    let cv = ColumnVisibilityState::default();
    cv.set_at("zone", false);
    let cp = ColumnPinningState::default()
        .with_start(["id"])
        .with_end(["status"]);
    let rs = RowSelectionState::default();
    rs.set_at("0", true);
    rs.set_at("2", true);
    rs.set_at("4", true);

    let init_state = JsValue::object()
        .with("columnVisibility", cv)
        .with("columnPinning", cp)
        .with("rowSelection", rs);
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
        zone_vis: Cell::new(false),
        owner_vis: Cell::new(true),
    });

    for (idx, bid) in ["btn-0", "btn-1", "btn-2", "btn-3", "btn-4"].iter().enumerate() {
        let btn = doc.get_element_by_id(bid).expect("missing button");
        let st_c = st.clone();
        let cb = dom::JsListener::new(move |_: dom::PointerEvent| {
            let pin: TableColumnPinning<TableFeatures, JsValue> = js::unchecked_cast(&st_c.table);
            let sel: TableRowSelection<TableFeatures, JsValue> = js::unchecked_cast(&st_c.table);
            match idx {
                0 => {
                    st_c.zone_vis.set(!st_c.zone_vis.get());
                    apply_vis(&st_c);
                }
                1 => {
                    st_c.owner_vis.set(!st_c.owner_vis.get());
                    apply_vis(&st_c);
                }
                2 => sel.toggle_all_rows_selected(true, None),
                3 => sel.toggle_all_rows_selected(false, None),
                _ => {
                    st_c.zone_vis.set(false);
                    st_c.owner_vis.set(true);
                    apply_vis(&st_c);
                    let reset_cp = ColumnPinningState::default()
                        .with_start(["id"])
                        .with_end(["status"]);
                    pin.set_column_pinning(Updater::T(reset_cp));
                    let reset_rs = RowSelectionState::default();
                    reset_rs.set_at("0", true);
                    reset_rs.set_at("2", true);
                    reset_rs.set_at("4", true);
                    sel.set_row_selection(Updater::T(reset_rs));
                }
            }
            render(&st_c);
        });
        btn.add_event_listener(dom::HTMLElementEventMap::CLICK, &cb, None);
    }

    render(&st);
}
