// TanStack Table (table-core v9) from Rust, in Node. Very little of its API
// can be driven from here: see the FINDINGs. What can be: its helper
// functions, the features and initial state they give, and a table built
// with reactivity bindings written in Rust. Every check! must hold.
#![no_main]
use bindings::*;
use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;

#[link(wasm_import_module = "env")]
unsafe extern "C" {
    #[link_name = "check"]
    fn check_at(ok: i32, line: u32);
}
macro_rules! check {
    ($c:expr) => {
        unsafe { check_at($c as i32, line!()) }
    };
}

fn object(entries: &[(&str, &JsValue)]) -> MakeObjectMapResult<JsValue> {
    let o = MakeObjectMapResult::<JsValue>::create();
    for (k, v) in entries {
        o.set_at(k, (*v).clone());
    }
    o
}

// An atom for the table's reactivity: a plain object with a `get` function,
// and a `set` if writable.
fn atom(get: JsValue, set: Option<Box<dyn FnMut(JsValue)>>) -> JsHandle {
    let o = MakeObjectMapResult::<JsValue>::create();
    o.set_at("get", get);
    if let Some(mut set) = set {
        o.set_at("set", JsValue(AnyFunction::new(move |v: JsValue| set(v)).0.clone()));
    }
    o.set_at("subscribe", JsValue(AnyFunction::new(|_: JsValue| {}).0.clone()));
    o.0.clone()
}

// FINDING 11: a JsValue that is an array cannot be made a JsArray, nor a
// JsArray a JsValue (no AsRef<JsHandle>): the handle is copied by hand.
fn as_array(v: &JsValue) -> JsArray<JsValue> {
    let h = v.0.clone();
    let id = h.0;
    std::mem::forget(h);
    JsArray::<JsValue>::from_handle(id)
}

#[unsafe(no_mangle)]
pub extern "C" fn run() {
    // The state a table starts from, per feature.
    let sorting = JsValue(row_sorting_feature().0.clone());
    let visibility = JsValue(column_visibility_feature().0.clone());
    let features: TableFeatures = table_features(JsValue::from(&object(&[("rowSortingFeature", &sorting), ("columnVisibilityFeature", &visibility)])));
    check!(features.0 .0 != 0);

    // functionalUpdate: a value, or a function of the previous value.
    let five: JsValue = functional_update(Updater::T(JsValue::from(5.0)), JsValue::from(1.0));
    check!(five.as_f64() == 5.0);
    let bump = UpdaterFn2::<JsValue>::new(|prev: JsValue| JsValue::from(prev.as_f64() + 1.0));
    check!(functional_update(Updater::UpdaterFn2(bump), JsValue::from(41.0)).as_f64() == 42.0);

    // Names of table functions, split into parent and key.
    let info = get_function_name_info("column_toggleSorting", None);
    check!(info.fn_key() == "toggleSorting" && info.fn_name() == "column.toggleSorting");
    check!(get_function_name_info("table_setPageIndex", None).fn_name() == "table.setPageIndex");

    // Comparing state slices, and own properties.
    let a = object(&[("x", &JsValue::from(1.0))]);
    let b = object(&[("x", &JsValue::from(1.0))]);
    let c = object(&[("x", &JsValue::from(2.0))]);
    check!(state_slices_equal(&a, &b) && !state_slices_equal(&a, &c));
    check!(has_own(&a, &PropertyKey(JsValue::from("x").0.clone())));
    check!(!has_own(&a, &PropertyKey(JsValue::from("y").0.clone())));
    check!(is_function::<JsValue>(JsValue(AnyFunction::new(|_: JsValue| {}).0.clone())) && !is_function::<JsValue>(1.0));

    // The row sort and filter functions are values; their names are known.
    check!(sort_fn_basic().0 .0 != 0 && sort_fn_text().0 .0 != 0);
    check!(filter_fn_includes_string().0 .0 != 0);
    check!(aggregation_fn_sum().0 .0 != 0);
    check!(core_features().core_reactivity_feature().is_none());
    check!(stock_features().row_sorting_feature().0 .0 != 0);
    check!(re_split_alpha_numeric().0 .0 != 0);

    // A table, built by `constructTable` with the reactivity written here.
    // FINDING 9: `TableOptionsCore` has no setters (its properties are
    // readonly), so the options are an object made here, viewed as one.
    let bindings = TableReactivityBindings::create();
    bindings.set_create_options_store(false);
    bindings.set_wrap_external_atoms(false);
    let writable = TableReactivityBindingsCreateWritableAtom::new(|initial: JsValue, _| {
        let value = Rc::new(RefCell::new(initial));
        let (read, write) = (value.clone(), value);
        let get = ColumnColumnFilteringGetFilterValue::new(move || read.borrow().clone());
        Atom(atom(JsValue(get.0.clone()), Some(Box::new(move |v| {
            // `set` takes a value, or a function of the previous one.
            let prev = write.borrow().clone();
            *write.borrow_mut() = functional_update(Updater::T(v), prev);
        }))), PhantomData)
    });
    bindings.set_create_writable_atom(&writable);
    // FINDING 10: a value of function type (`compute: () => T`, and every
    // `table.getRowModel`, `setSorting`, `columnHelper.accessor`) can be made
    // but has no call method, so almost nothing of the API can be used. Here
    // `compute` is made the atom's `get` itself.
    let readonly = TableReactivityBindingsCreateReadonlyAtom::new(|compute: VoidToTFn<JsValue>, _| ReadonlyAtom(atom(JsValue(compute.0.clone()), None), PhantomData));
    bindings.set_create_readonly_atom(&readonly);
    // FINDING 10 again: `untrack`, `batch` and `schedule` must call the
    // function they are given; `functionalUpdate` does, and stands in for that.
    let call = |f: &JsValue| functional_update(Updater::T(f.clone()), JsValue::null());
    let call = call;
    let untrack = TableReactivityBindingsUntrack::new(move |f: VoidToTFn<JsValue>| call(&JsValue(f.0.clone())));
    bindings.set_untrack(&untrack);
    let run_it = FnFn::new(move |f: ColumnColumnGroupingToggleGrouping| {
        call(&JsValue(f.0.clone()));
    });
    bindings.set_batch(&run_it);
    bindings.set_schedule(&run_it);
    let noop = TableReactivityBindingsAddSubscription::new(|_| {});
    bindings.set_add_subscription(&noop);
    check!(bindings.create_options_store() == false);

    // The data, the columns (plain definitions: `columnHelper.accessor` is a
    // function-valued property and cannot be called), and the options.
    let person = |name: &str, age: f64| JsValue::from(&object(&[("name", &JsValue::from(name)), ("age", &JsValue::from(age))]));
    let data = JsArray::<JsValue>::from(vec![person("bo", 30.0), person("al", 20.0), person("cy", 25.0)]);
    let column = |key: &str, header: &str| JsValue::from(&object(&[("accessorKey", &JsValue::from(key)), ("header", &JsValue::from(header))]));
    let columns = JsArray::<JsValue>::from(vec![column("name", "Name"), column("age", "Age")]);
    let sorted_model = JsValue(create_sorted_row_model::<TableFeatures, JsValue>().0.clone());
    let reactivity = JsValue(bindings.0.clone());
    let sorting = JsValue(row_sorting_feature().0.clone());
    let feature_map = object(&[("coreReactivityFeature", &reactivity), ("rowSortingFeature", &sorting), ("sortedRowModel", &sorted_model)]);
    let options = object(&[("features", &JsValue::from(&feature_map)), ("data", &JsValue(data.0.clone())), ("columns", &JsValue(columns.0.clone()))]);
    let table = construct_table(&TableOptionsCore::<TableFeatures, JsValue>(options.0.clone(), PhantomData));
    check!(table.0 .0 != 0);
    let state = table.initial_state();
    check!(state.0 .0 != 0);
    check!(as_array(&MakeObjectMapResult::<JsValue>(state.0.clone(), PhantomData).at("sorting").unwrap()).len() == 0);
    check!(table.options().0 .0 != 0);

    // The row model, got by calling the function-valued property (FINDING 10's
    // stand-in again): three rows in the data's order.
    let model = RowModel::<TableFeatures, JsValue>(call(&JsValue(table.get_row_model().0.clone())).0.clone(), PhantomData);
    let rows = model.rows();
    check!(rows.len() == 3);
    let name_of = |r: &RowCore<TableFeatures, JsValue>| MakeObjectMapResult::<JsValue>(r.original().0.clone(), PhantomData).at("name").unwrap().as_string();
    let names: Vec<String> = rows.iter().map(|r| name_of(&r)).collect();
    check!(names == ["bo", "al", "cy"]);
    check!(rows.get(1).id() == "1" && rows.get(1).index() == 1.0 && rows.get(1).depth() == 0.0);
    check!(model.flat_rows().len() == 3);

    // Sorted by age, set the way a user would: setSorting([{ id, desc }]).
    let by_age = ColumnSort::create();
    by_age.set_id("age");
    by_age.set_desc(false);
    // The table's sorting members are on a type its core type is not: a view.
    let sortable = TableRowSorting::<TableFeatures, JsValue>(table.0.clone(), PhantomData);
    let sorts = JsArray::<ColumnSort>::from(vec![by_age]);
    functional_update(Updater::T(JsValue(sortable.set_sorting().0.clone())), JsValue(sorts.0.clone()));
    let model = RowModel::<TableFeatures, JsValue>(call(&JsValue(table.get_row_model().0.clone())).0.clone(), PhantomData);
    let names: Vec<String> = model.rows().iter().map(|r| name_of(&r)).collect();
    check!(names == ["al", "cy", "bo"]);
}
