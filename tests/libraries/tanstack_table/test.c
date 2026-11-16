// TanStack Table (v9 core) from C, in Node: a table of people with columns,
// data, the core and sorted row models, and sorting state with a change
// handler. Every CHECK must hold.
//
// The library has no reactivity of its own here (that is a separate subpath
// package that is not bound), so this program supplies atoms: a value you
// get and set, built as plain JavaScript objects from callbacks.
#include "bindings.h"

TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

static const JsValue NONE = { 0 };

// FINDING 13: the bindings give the table's methods (getRowModel, setSorting,
// ...) as function-typed properties, and a function handle cannot be called.
// `functionalUpdate(f, x)` calls `f(x)`, so it is the one way to call a
// function handle, with exactly one argument.
static JsValue call(JsValue f, JsValue arg) { return functionalUpdate(f, arg); }
static JsValue get(JsValue obj, const char* key) { return RowData_map_get_at((RowData_map){ obj.h }, key); }
static JsValue call0(JsValue obj, const char* method) { return call(get(obj, method), NONE); }
static JsValue call1(JsValue obj, const char* method, JsValue arg) { return call(get(obj, method), arg); }

static bool str_is(JsValue v, const char* expected) { return JsString_equals(js_to_string(v), expected); }

// ---- atoms: an object { get, set, subscribe } over a value kept here ----

typedef struct {
    JsValue value;     // a writable atom's value
    JsValue derive;    // a readonly atom's function
    bool readonly;
} AtomState;
static AtomState atoms[256];
static int atom_count;

static JsValue atom_get(void* ctx) {
    AtomState* a = ctx;
    if (a->readonly) return call(a->derive, NONE);
    js_retain(a->value);
    return a->value;
}
static JsValue atom_set(void* ctx, JsValue updater) {
    AtomState* a = ctx;
    JsValue next = call(updater, a->value);
    a->value = next;
    return NONE;
}
static JsValue no_unsubscribe(void* ctx) { return NONE; }
static JsValue atom_subscribe(void* ctx, JsValue observer) {
    RowData_map subscription = makeObjectMap();
    RowData_map_set_at(subscription, "unsubscribe", (JsValue){ void_fn_from(no_unsubscribe, 0).h });
    return (JsValue){ subscription.h };
}

static JsValue make_atom(AtomState* a) {
    RowData_map atom = makeObjectMap();
    RowData_map_set_at(atom, "get", (JsValue){ void_fn_from(atom_get, a).h });
    if (!a->readonly) RowData_map_set_at(atom, "set", (JsValue){ Updater_fn2_from(atom_set, a).h });
    RowData_map_set_at(atom, "subscribe", (JsValue){ Updater_fn2_from(atom_subscribe, 0).h });
    return (JsValue){ atom.h };
}

static Atom create_writable(void* ctx, JsValue initial, TableAtomOptions options) {
    AtomState* a = &atoms[atom_count++];
    a->value = initial;
    js_retain(initial);
    return (Atom){ make_atom(a).h };
}
static ReadonlyAtom create_readonly(void* ctx, void_fn fn, TableAtomOptions options) {
    AtomState* a = &atoms[atom_count++];
    a->readonly = true;
    a->derive = (JsValue){ fn.h };
    js_retain(fn);
    return (ReadonlyAtom){ make_atom(a).h };
}
static void add_subscription(void* ctx, Subscription s) {}
static void unmount(void* ctx) {}
static void batch(void* ctx, Column_ColumnGrouping_toggleGrouping fn) { js_release(call((JsValue){ fn.h }, NONE)); }
static JsValue untrack(void* ctx, void_fn fn) { return call((JsValue){ fn.h }, NONE); }

// ---- the table ----

static JsValue default_sorting_update;   // what the library does on a sorting change
static int sorting_changes;
static void on_sorting_change(void* ctx, JsValue updater) {
    sorting_changes++;
    js_release(call(default_sorting_update, updater));
}

static JsValue person(const char* name, double age) {
    RowData_map p = makeObjectMap();
    RowData_map_set_at(p, "name", js_string(name));
    RowData_map_set_at(p, "age", js_number(age));
    return (JsValue){ p.h };
}
static JsValue column(const char* key, const char* header) {
    RowData_map c = makeObjectMap();
    RowData_map_set_at(c, "accessorKey", js_string(key));
    RowData_map_set_at(c, "header", js_string(header));
    return (JsValue){ c.h };
}
static JsValue sort_by(const char* id, bool desc) {
    RowData_map s = makeObjectMap();
    RowData_map_set_at(s, "id", js_string(id));
    RowData_map_set_at(s, "desc", js_bool(desc));
    JsArray list = JsArray_new();
    JsArray_push(list, (JsValue){ s.h });
    return (JsValue){ list.h };
}

// The names of a row model's rows, joined by commas.
static char names_buf[256];
static const char* names(JsValue row_model) {
    JsArray_Row_Core rows = RowModel_get_rows((RowModel){ row_model.h });
    uint32_t n = JsArray_Row_Core_length(rows), at = 0;
    for (uint32_t i = 0; i < n; i++) {
        JsValue original = get((JsValue){ JsArray_Row_Core_get(rows, i).h }, "original");
        JsString name = js_to_string(get(original, "name"));
        if (i) names_buf[at++] = ',';
        at += JsString_copy(name, names_buf + at, 64);
    }
    names_buf[at] = 0;
    return names_buf;
}
static bool same(const char* a, const char* b) {
    while (*a && *a == *b) a++, b++;
    return *a == *b;
}
static const char* sorted_names(JsValue table) { return names(call0(table, "getRowModel")); }

__attribute__((export_name("run"))) void run(void) {
    // ---- static functions ----
    JsValue five = js_number(5);
    CHECK(js_to_number(functionalUpdate(five, NONE)) == 5);
    CHECK(isFunction(five) == false);
    CHECK(isFunction((JsValue){ void_fn_from(atom_get, &atoms[255]).h }));
    atoms[255].value = js_number(9);
    CHECK(js_to_number(functionalUpdate((JsValue){ void_fn_from(atom_get, &atoms[255]).h }, NONE)) == 9);
    CHECK(stateSlicesEqual(js_number(1), js_number(1)) && !stateSlicesEqual(js_number(1), js_number(2)));
    CHECK(stateSlicesEqual(sort_by("a", false), sort_by("a", false)));
    CHECK(!stateSlicesEqual(sort_by("a", false), sort_by("a", true)));
    JsValue original = sort_by("x", true);
    JsValue copy = cloneState(original);
    CHECK(!js_same(original, copy) && stateSlicesEqual(original, copy));

    // ---- reactivity: what the library asks of its host ----
    TableReactivityBindings reactivity = TableReactivityBindings_create();
    TableReactivityBindings_set_createOptionsStore(reactivity, true);
    TableReactivityBindings_set_wrapExternalAtoms(reactivity, false);
    TableReactivityBindings_set_addSubscription(reactivity, TableReactivityBindings_addSubscription_from(add_subscription, 0));
    TableReactivityBindings_set_unmount(reactivity, Column_ColumnGrouping_toggleGrouping_from((Column_ColumnGrouping_toggleGrouping_fn)unmount, 0));
    TableReactivityBindings_set_batch(reactivity, fn_fn_from(batch, 0));
    TableReactivityBindings_set_schedule(reactivity, fn_fn_from(batch, 0));
    TableReactivityBindings_set_untrack(reactivity, TableReactivityBindings_untrack_from(untrack, 0));
    TableReactivityBindings_set_createWritableAtom(reactivity, TableReactivityBindings_createWritableAtom_from(create_writable, 0));
    TableReactivityBindings_set_createReadonlyAtom(reactivity, TableReactivityBindings_createReadonlyAtom_from(create_readonly, 0));
    CHECK(TableReactivityBindings_get_createOptionsStore(reactivity) && !TableReactivityBindings_get_wrapExternalAtoms(reactivity));

    // ---- features, columns, data ----
    // FINDING 14: TableOptions_Table has only getters (its members are
    // `readonly` in the d.ts), so a table's options cannot be filled in
    // through the typed API: they are a plain object made with makeObjectMap
    // and cast to the options type.
    RowData_map features = makeObjectMap();
    RowData_map_set_at(features, "coreReactivityFeature", (JsValue){ reactivity.h });
    RowData_map_set_at(features, "rowSortingFeature", (JsValue){ get_rowSortingFeature().h });
    RowData_map_set_at(features, "sortedRowModel", (JsValue){ createSortedRowModel().h });

    JsArray_ColumnDefBase_Core columns = (JsArray_ColumnDefBase_Core){ JsArray_new().h };
    JsArray_ColumnDefBase_Core_push(columns, (ColumnDefBase_Core){ column("name", "Name").h });
    JsArray_ColumnDefBase_Core_push(columns, (ColumnDefBase_Core){ column("age", "Age").h });
    JsArray_RowData data = (JsArray_RowData){ JsArray_new().h };
    JsArray_RowData_push(data, RowData_of_RowData_map((RowData_map){ person("Bob", 30).h }));
    JsArray_RowData_push(data, RowData_of_RowData_map((RowData_map){ person("Alice", 25).h }));
    JsArray_RowData_push(data, RowData_of_RowData_map((RowData_map){ person("Carol", 35).h }));
    JsArray_RowData_push(data, RowData_of_RowData_map((RowData_map){ person("Dave", 25).h }));

    RowData_map options = makeObjectMap();
    RowData_map_set_at(options, "features", (JsValue){ features.h });
    RowData_map_set_at(options, "columns", (JsValue){ columns.h });
    RowData_map_set_at(options, "data", (JsValue){ data.h });
    RowData_map_set_at(options, "onSortingChange", (JsValue){ OnChangeFn_from(on_sorting_change, 0).h });

    Table_Core table_handle = constructTable((TableOptions_Core){ options.h });
    CHECK(table_handle.h != 0);
    JsValue table = { table_handle.h };
    default_sorting_update = (JsValue){ makeStateUpdater("sorting", (makeStateUpdater_instance){ table.h }).h };

    // ---- columns ----
    JsValue all_columns = call0(table, "getAllColumns");
    JsArray cols = (JsArray){ all_columns.h };
    CHECK(JsArray_length(cols) == 2);
    CHECK(str_is(get(JsArray_get(cols, 0), "id"), "name") && str_is(get(JsArray_get(cols, 1), "id"), "age"));
    JsValue age_column = call1(table, "getColumn", js_string("age"));
    CHECK(age_column.h != 0 && str_is(get(age_column, "id"), "age"));

    // ---- row models ----
    CHECK(JsArray_length((JsArray){ RowModel_get_rows((RowModel){ call0(table, "getCoreRowModel").h }).h }) == 4);
    CHECK(same(names(call0(table, "getCoreRowModel")), "Bob,Alice,Carol,Dave"));
    // No sorting yet: the display model is the data's order.
    CHECK(same(sorted_names(table), "Bob,Alice,Carol,Dave"));
    JsValue first_row = (JsValue){ JsArray_Row_Core_get(RowModel_get_rows((RowModel){ call0(table, "getRowModel").h }), 0).h };
    CHECK(str_is(get(first_row, "id"), "0") && js_to_number(get(first_row, "index")) == 0);
    // FINDING 13: row.getValue("name") reads `this`, which a function handle
    // called through functionalUpdate does not have: it throws. Left out; the
    // row's own data is read through `original`.
    CHECK(str_is(get(get(first_row, "original"), "name"), "Bob"));

    // ---- sorting ----
    CHECK(JsArray_length((JsArray){ get(call0(get(table, "store"), "get"), "sorting").h }) == 0);
    call1(table, "setSorting", sort_by("age", false));
    CHECK(sorting_changes == 1);
    // Ages 25, 25, 30, 35; ties keep the data's order.
    CHECK(same(sorted_names(table), "Alice,Dave,Bob,Carol"));
    JsValue state = call0(get(table, "store"), "get");
    JsArray sorting = (JsArray){ get(state, "sorting").h };
    CHECK(JsArray_length(sorting) == 1);
    CHECK(str_is(get(JsArray_get(sorting, 0), "id"), "age") && !js_to_bool(get(JsArray_get(sorting, 0), "desc")));
    // The core model is not sorted.
    CHECK(same(names(call0(table, "getCoreRowModel")), "Bob,Alice,Carol,Dave"));
    CHECK(same(names(call0(table, "getPreSortedRowModel")), "Bob,Alice,Carol,Dave"));

    call1(table, "setSorting", sort_by("name", true));
    CHECK(sorting_changes == 2);
    CHECK(same(sorted_names(table), "Dave,Carol,Bob,Alice"));
    call1(table, "setSorting", sort_by("age", true));
    CHECK(same(sorted_names(table), "Carol,Bob,Alice,Dave"));
    // An empty sorting is the original order again.
    call1(table, "setSorting", (JsValue){ JsArray_new().h });
    CHECK(same(sorted_names(table), "Bob,Alice,Carol,Dave"));
    CHECK(sorting_changes == 4);
}
