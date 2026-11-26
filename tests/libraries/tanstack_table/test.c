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

// An Updater<T> is a value or a function of the old value: functionalUpdate
// resolves it.
static JsValue call(JsValue f, JsValue arg) { return table_functionalUpdate(f, arg); }
static JsValue get(JsValue obj, const char* key) { return Table_RowData_map_get_at((Table_RowData_map){ obj.h }, key); }
// The table's methods are function-typed properties: read bound to the table,
// called with call_<Type>.
// ReadonlyStore is opaque in the bindings (no methods): its `get` is read as a
// property of the object and called.
static JsValue store_state(JsValue table) { return call(get(get(table, "store"), "get"), NONE); }
static JsValue row_model(Table_RowModels_Core t) { return (JsValue){ Table_RowModels_Core_getRowModel(t).h }; }
static JsValue core_row_model(Table_RowModels t) { return (JsValue){ Table_RowModels_getCoreRowModel(t).h }; }
static JsValue pre_sorted_row_model(Table_RowModels t) { return (JsValue){ Table_RowModels_getPreSortedRowModel(t).h }; }
static void set_sorting(Table_RowSorting t, JsValue sorting) { Table_RowSorting_setSorting(t, sorting); }

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
    if (a->readonly) return table_call_Getter((Table_Getter){ a->derive.h });
    js_retain(a->value);
    return a->value;
}
static JsValue atom_set(void* ctx, JsValue updater) {
    AtomState* a = ctx;
    JsValue next = call(updater, a->value);
    a->value = next;
    return NONE;
}
static void no_unsubscribe(void* ctx) { (void)ctx; }
static JsValue atom_subscribe(void* ctx, JsValue observer) {
    Table_RowData_map subscription = (Table_RowData_map){ table_makeObjectMap().h };
    Table_RowData_map_set_at(subscription, "unsubscribe", (JsValue){ Table_void_fn_from(no_unsubscribe, 0).h });
    return (JsValue){ subscription.h };
}

static JsValue make_atom(AtomState* a) {
    Table_RowData_map atom = (Table_RowData_map){ table_makeObjectMap().h };
    Table_RowData_map_set_at(atom, "get", (JsValue){ Table_Getter_from(atom_get, a).h });
    if (!a->readonly) Table_RowData_map_set_at(atom, "set", (JsValue){ Table_UpdaterFn_from(atom_set, a).h });
    Table_RowData_map_set_at(atom, "subscribe", (JsValue){ Table_UpdaterFn_from(atom_subscribe, 0).h });
    return (JsValue){ atom.h };
}

static Table_Atom create_writable(void* ctx, JsValue initial, TableAtomOptions options) {
    AtomState* a = &atoms[atom_count++];
    a->value = initial;
    js_retain(initial);
    return (Table_Atom){ make_atom(a).h };
}
static Table_ReadonlyAtom create_readonly(void* ctx, Table_Getter fn, TableAtomOptions options) {
    AtomState* a = &atoms[atom_count++];
    a->readonly = true;
    a->derive = (JsValue){ fn.h };
    js_retain(fn);
    return (Table_ReadonlyAtom){ make_atom(a).h };
}
static void add_subscription(void* ctx, Table_Subscription s) {}
static void unmount(void* ctx) {}
static void batch(void* ctx, Table_void_fn fn) { table_call_void_fn(fn); }
static JsValue untrack(void* ctx, Table_Getter fn) { return table_call_Getter(fn); }

// ---- the table ----

static Table_Updater_fn default_sorting_update;   // what the library does on a sorting change
static int sorting_changes;
static void on_sorting_change(void* ctx, JsValue updater) {
    sorting_changes++;
    table_call_Updater_fn(default_sorting_update, updater);
}

static JsValue person(const char* name, double age) {
    Table_RowData_map p = (Table_RowData_map){ table_makeObjectMap().h };
    Table_RowData_map_set_at(p, "name", js_string(name));
    Table_RowData_map_set_at(p, "age", js_number(age));
    return (JsValue){ p.h };
}
static JsValue column(const char* key, const char* header) {
    Table_RowData_map c = (Table_RowData_map){ table_makeObjectMap().h };
    Table_RowData_map_set_at(c, "accessorKey", js_string(key));
    Table_RowData_map_set_at(c, "header", js_string(header));
    return (JsValue){ c.h };
}
static JsValue sort_by(const char* id, bool desc) {
    Table_RowData_map s = (Table_RowData_map){ table_makeObjectMap().h };
    Table_RowData_map_set_at(s, "id", js_string(id));
    Table_RowData_map_set_at(s, "desc", js_bool(desc));
    JsArray list = JsArray_new();
    JsArray_push(list, (JsValue){ s.h });
    return (JsValue){ list.h };
}

// The names of a row model's rows, joined by commas.
static char names_buf[256];
static const char* names(JsValue row_model) {
    JsArray_Table_Row_Core rows = Table_RowModel_get_rows((Table_RowModel){ row_model.h });
    uint32_t n = JsArray_Table_Row_Core_length(rows), at = 0;
    for (uint32_t i = 0; i < n; i++) {
        JsValue original = get((JsValue){ JsArray_Table_Row_Core_get(rows, i).h }, "original");
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
static const char* sorted_names(JsValue table) { return names(row_model((Table_RowModels_Core){ table.h })); }

__attribute__((export_name("run"))) void run(void) {
    // ---- static functions ----
    JsValue five = js_number(5);
    CHECK(js_to_number(table_functionalUpdate(five, NONE)) == 5);
    CHECK(table_isFunction(five) == false);
    CHECK(table_isFunction((JsValue){ Table_Getter_from(atom_get, &atoms[255]).h }));
    atoms[255].value = js_number(9);
    CHECK(js_to_number(table_functionalUpdate((JsValue){ Table_Getter_from(atom_get, &atoms[255]).h }, NONE)) == 9);
    CHECK(table_stateSlicesEqual(js_number(1), js_number(1)) && !table_stateSlicesEqual(js_number(1), js_number(2)));
    CHECK(table_stateSlicesEqual(sort_by("a", false), sort_by("a", false)));
    CHECK(!table_stateSlicesEqual(sort_by("a", false), sort_by("a", true)));
    JsValue original = sort_by("x", true);
    JsValue copy = table_cloneState(original);
    CHECK(!js_same(original, copy) && table_stateSlicesEqual(original, copy));

    // ---- reactivity: what the library asks of its host ----
    TableReactivityBindings reactivity = TableReactivityBindings_create();
    TableReactivityBindings_set_createOptionsStore(reactivity, true);
    TableReactivityBindings_set_wrapExternalAtoms(reactivity, false);
    TableReactivityBindings_set_addSubscription(reactivity, Table_AddSubscriptionFn_from(add_subscription, 0));
    TableReactivityBindings_set_unmount(reactivity, Table_void_fn_from(unmount, 0));
    TableReactivityBindings_set_batch(reactivity, Table_fn_fn_from(batch, 0));
    TableReactivityBindings_set_schedule(reactivity, Table_fn_fn_from(batch, 0));
    TableReactivityBindings_set_untrack(reactivity, Table_UntrackFn_from(untrack, 0));
    TableReactivityBindings_set_createWritableAtom(reactivity, Table_CreateWritableAtomFn_from(create_writable, 0));
    TableReactivityBindings_set_createReadonlyAtom(reactivity, Table_CreateReadonlyAtomFn_from(create_readonly, 0));
    CHECK(TableReactivityBindings_get_createOptionsStore(reactivity) && !TableReactivityBindings_get_wrapExternalAtoms(reactivity));

    // ---- features, columns, data ----
    // FINDING 14: TableOptions_Table has only getters (its members are
    // `readonly` in the d.ts), so a table's options cannot be filled in
    // through the typed API: they are a plain object made with makeObjectMap
    // and cast to the options type.
    Table_RowData_map features = (Table_RowData_map){ table_makeObjectMap().h };
    Table_RowData_map_set_at(features, "coreReactivityFeature", (JsValue){ reactivity.h });
    Table_RowData_map_set_at(features, "rowSortingFeature", (JsValue){ table_get_rowSortingFeature().h });
    Table_RowData_map_set_at(features, "sortedRowModel", (JsValue){ table_createSortedRowModel().h });

    JsArray_Table_ColumnDefBase_Core columns = (JsArray_Table_ColumnDefBase_Core){ JsArray_new().h };
    JsArray_Table_ColumnDefBase_Core_push(columns, (Table_ColumnDefBase_Core){ column("name", "Name").h });
    JsArray_Table_ColumnDefBase_Core_push(columns, (Table_ColumnDefBase_Core){ column("age", "Age").h });
    JsArray_Table_RowData data = (JsArray_Table_RowData){ JsArray_new().h };
    JsArray_Table_RowData_push(data, Table_RowData_of_Table_RowData_map((Table_RowData_map){ person("Bob", 30).h }));
    JsArray_Table_RowData_push(data, Table_RowData_of_Table_RowData_map((Table_RowData_map){ person("Alice", 25).h }));
    JsArray_Table_RowData_push(data, Table_RowData_of_Table_RowData_map((Table_RowData_map){ person("Carol", 35).h }));
    JsArray_Table_RowData_push(data, Table_RowData_of_Table_RowData_map((Table_RowData_map){ person("Dave", 25).h }));

    Table_RowData_map options = (Table_RowData_map){ table_makeObjectMap().h };
    Table_RowData_map_set_at(options, "features", (JsValue){ features.h });
    Table_RowData_map_set_at(options, "columns", (JsValue){ columns.h });
    Table_RowData_map_set_at(options, "data", (JsValue){ data.h });
    Table_RowData_map_set_at(options, "onSortingChange", (JsValue){ Table_OnChangeFn_from(on_sorting_change, 0).h });

    Table_Core table_handle = table_constructTable((TableOptions_Core){ options.h });
    CHECK(table_handle.h != 0);
    JsValue table = { table_handle.h };
    default_sorting_update = table_makeStateUpdater("sorting", (Table_makeStateUpdater_instance){ table.h });

    // ---- columns ----
    JsArray cols = (JsArray){ Table_Columns_getAllColumns((Table_Columns){ table.h }).h };
    CHECK(JsArray_length(cols) == 2);
    CHECK(str_is(get(JsArray_get(cols, 0), "id"), "name") && str_is(get(JsArray_get(cols, 1), "id"), "age"));
    JsValue age_column = (JsValue){ Table_Columns_getColumn((Table_Columns){ table.h }, "age").h };
    CHECK(age_column.h != 0 && str_is(get(age_column, "id"), "age"));

    // ---- row models ----
    CHECK(JsArray_length((JsArray){ Table_RowModel_get_rows((Table_RowModel){ core_row_model((Table_RowModels){ table.h }).h }).h }) == 4);
    CHECK(same(names(core_row_model((Table_RowModels){ table.h })), "Bob,Alice,Carol,Dave"));
    // No sorting yet: the display model is the data's order.
    CHECK(same(sorted_names(table), "Bob,Alice,Carol,Dave"));
    JsValue first_row = (JsValue){ JsArray_Table_Row_Core_get(Table_RowModel_get_rows((Table_RowModel){ row_model((Table_RowModels_Core){ table.h }).h }), 0).h };
    CHECK(str_is(get(first_row, "id"), "0") && js_to_number(get(first_row, "index")) == 0);
    // row.getValue keeps its `this`.
    CHECK(str_is(Table_Row_Row_getValue((Table_Row_Row){ first_row.h }, "name"), "Bob"));
    CHECK(js_to_number(Table_Row_Row_getValue((Table_Row_Row){ first_row.h }, "age")) == 30);
    CHECK(str_is(get(get(first_row, "original"), "name"), "Bob"));

    // ---- sorting ----
    CHECK(JsArray_length((JsArray){ get(store_state(table), "sorting").h }) == 0);
    set_sorting((Table_RowSorting){ table.h }, sort_by("age", false));
    CHECK(sorting_changes == 1);
    // Ages 25, 25, 30, 35; ties keep the data's order.
    CHECK(same(sorted_names(table), "Alice,Dave,Bob,Carol"));
    JsValue state = store_state(table);
    JsArray sorting = (JsArray){ get(state, "sorting").h };
    CHECK(JsArray_length(sorting) == 1);
    CHECK(str_is(get(JsArray_get(sorting, 0), "id"), "age") && !js_to_bool(get(JsArray_get(sorting, 0), "desc")));
    // The core model is not sorted.
    CHECK(same(names(core_row_model((Table_RowModels){ table.h })), "Bob,Alice,Carol,Dave"));
    CHECK(same(names(pre_sorted_row_model((Table_RowModels){ table.h })), "Bob,Alice,Carol,Dave"));

    set_sorting((Table_RowSorting){ table.h }, sort_by("name", true));
    CHECK(sorting_changes == 2);
    CHECK(same(sorted_names(table), "Dave,Carol,Bob,Alice"));
    set_sorting((Table_RowSorting){ table.h }, sort_by("age", true));
    CHECK(same(sorted_names(table), "Carol,Bob,Alice,Dave"));
    // An empty sorting is the original order again.
    set_sorting((Table_RowSorting){ table.h }, (JsValue){ JsArray_new().h });
    CHECK(same(sorted_names(table), "Bob,Alice,Carol,Dave"));
    CHECK(sorting_changes == 4);
}
