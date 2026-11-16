// TanStack Table v9 (table-core) from C++, in Node: a table of people with
// accessor columns and a sorted row model, its rows, cells and sorting.
// Every CHECK must hold.
//
// Two things in the packages as bound make this awkward, both noted as
// findings below: the table needs a `coreReactivityFeature` (atoms, from a
// package impl.mjs does not give), written here in C++; and the table's
// methods are function-typed properties, which the bindings give as callbacks
// that cannot be called, so `call` runs them through `functionalUpdate`.
#include <memory>

#include "bindings.hpp"

extern "C" TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

using V = ts::Value;
using Obj = RowData_map;  // a plain JavaScript object: `{}` with set_at
using Table = Table_Core<TableFeatures, RowData_map>;
using Model = RowModel<TableFeatures, RowData_map>;
using RowC = Row_Core<TableFeatures, RowData_map>;
using RowP = Row_CoreProperties<TableFeatures, RowData_map>;
using ColC = Column_Core<TableFeatures, RowData_map, ts::Value>;
using ColP = Column_CoreProperties<TableFeatures, RowData_map, ts::Value>;
using ColSort = Column_RowSorting<TableFeatures, RowData_map>;
using TabSort = Table_RowSorting<TableFeatures, RowData_map>;

static V val(const ts::Handle& h) {
    V v;
    static_cast<ts::Handle&>(v) = h;
    return v;
}
template <class T>
static T as(const ts::Handle& h) {
    T t;
    static_cast<ts::Handle&>(t) = h;
    return t;
}
static V str(std::string_view s) { return V::string(s); }

// FINDING 12: `getRowModel: () => RowModel`, `getColumn: (id) => Column`,
// `getValue: (id) => value`... are properties of function type; the bindings
// give them as `ts::Callback` structs (Table_RowModels_getCoreRowModel,
// string_to_TValue_fn...) that can be made from a lambda but not called. And
// the property getter hands back the bare function, so `this` is lost (rows
// and columns keep their methods on a prototype and fail with "Cannot read
// properties of undefined"). Two library helpers do the calling here:
// functionalUpdate(fn, x) is `typeof fn === "function" ? fn(x) : fn`, for a
// function that needs no `this`; and callMemoOrStaticFn(obj, "name", _, args)
// is `obj.name(...args)`, a method call.
static V call(const ts::Handle& fn, V arg = V()) {
    return functionalUpdate<V>(Updater<V>(std::in_place_index<0>, val(fn)), arg);
}
static V callm(const ts::Handle& obj, std::string_view name, ts::Array<> args = ts::Array<>::create()) {
    using Static = callMemoOrStaticFn_staticFn<Obj, ts::Array<>, V>;
    return callMemoOrStaticFn<Obj, ts::Array<>, V>(as<Obj>(obj), name, Static([](Obj, ts::Array<>) { return V(); }), args);
}

// FINDING 13: constructTable needs features.coreReactivityFeature (atoms);
// @tanstack/table-core only has them through `renderPhaseReactivity(primitives)`
// (needs @tanstack/store) or '@tanstack/table-core/store-reactivity-bindings',
// and impl.mjs hands over neither. Atoms are made here: objects of get/set/subscribe.
static V unsubscribe_object() {
    Obj sub = Obj::create();
    sub.set_at("unsubscribe", val(Column_ColumnGrouping_toggleGrouping([] {})));
    return val(sub);
}
static V writable_atom(V initial) {
    auto state = std::make_shared<V>(initial);
    Obj atom = Obj::create();
    atom.set_at("get", val(void_to_T_fn<V>([state] { return *state; })));
    atom.set_at("set", val(Updater_fn2<V>([state](V update) {
                    *state = call(update, *state);
                    return V();
                })));
    atom.set_at("subscribe", val(Updater_fn2<V>([](V) { return unsubscribe_object(); })));
    return val(atom);
}
static V readonly_atom(V compute) {
    Obj atom = Obj::create();
    atom.set_at("get", val(void_to_T_fn<V>([compute] { return call(compute); })));
    atom.set_at("subscribe", val(Updater_fn2<V>([](V) { return unsubscribe_object(); })));
    return val(atom);
}
static V reactivity() {
    TableReactivityBindings r = TableReactivityBindings::create();
    r.createOptionsStore(false);
    r.wrapExternalAtoms(false);
    r.addSubscription(TableReactivityBindings_addSubscription([](Subscription) {}));
    r.createWritableAtom(TableReactivityBindings_createWritableAtom([](V initial, std::optional<TableAtomOptions<V>>) { return writable_atom(initial); }));
    r.createReadonlyAtom(TableReactivityBindings_createReadonlyAtom([](void_to_T_fn<V> compute, std::optional<TableAtomOptions<V>>) { return readonly_atom(val(compute)); }));
    r.untrack(TableReactivityBindings_untrack([](void_to_T_fn<V> fn) { return call(fn); }));
    r.batch(fn_fn([](Column_ColumnGrouping_toggleGrouping fn) { call(fn); }));
    r.schedule(fn_fn([](Column_ColumnGrouping_toggleGrouping fn) { call(fn); }));
    return val(r);
}

static Obj person(std::string_view name, double age, std::string_view team) {
    Obj p = Obj::create();
    p.set_at("name", str(name));
    p.set_at("age", V::number(age));
    p.set_at("team", str(team));
    return p;
}

static Obj column(std::string_view key, std::string_view header) {
    Obj c = Obj::create();
    c.set_at("accessorKey", str(key));
    c.set_at("header", str(header));
    return c;
}

static std::string ids(const ts::Array<RowC>& rows) {
    std::string out;
    for (uint32_t i = 0; i < rows.size(); i++) out += (i ? "," : "") + as<RowP>(rows[i]).id();
    return out;
}

extern "C" __attribute__((export_name("run"))) void run() {
    // The features: sorting, its row model, and the sort functions by name.
    Obj sortFns = Obj::create();
    sortFns.set_at("basic", val(get_sortFn_basic()));
    sortFns.set_at("alphanumeric", val(get_sortFn_alphanumeric()));
    sortFns.set_at("text", val(get_sortFn_text()));
    Obj features = Obj::create();
    features.set_at("coreReactivityFeature", reactivity());
    features.set_at("rowSortingFeature", val(get_rowSortingFeature()));
    features.set_at("sortFns", val(sortFns));
    features.set_at("sortedRowModel", val(createSortedRowModel<TableFeatures, RowData_map>()));

    ts::Array<Obj> data{person("Cy", 30, "red"), person("Al", 25, "blue"), person("Bo", 41, "red")};
    ts::Array<Obj> columns{column("name", "Name"), column("age", "Age"), column("team", "Team")};
    CHECK(data.size() == 3 && columns.size() == 3);

    Obj options = Obj::create();
    options.set_at("features", val(features));
    options.set_at("columns", val(columns));
    options.set_at("data", val(data));
    // Row ids from the data: a function of the row, the index and the parent.
    options.set_at("getRowId", val(TableOptions_Rows_getRowId<RowData_map, TableFeatures>([](RowData_map row, double index, std::optional<RowC>) {
                       return row.at("name").to_string() + "#" + std::to_string(int(index));
                   })));

    Table table = constructTable<TableFeatures, RowData_map>(as<TableOptions_Core<TableFeatures, RowData_map>>(options));
    CHECK(table.h != 0);

    // The row model: three rows, by index as ids.
    Model model = as<Model>(callm(table, "getRowModel"));
    CHECK(model.rows().size() == 3);
    CHECK(ids(model.rows()) == "Cy#0,Al#1,Bo#2");
    CHECK(as<Model>(callm(table, "getCoreRowModel")).rows().size() == 3);

    // Rows: the original data, and cell values by column id.
    auto first = as<RowP>(model.rows()[0]);
    CHECK(first.original().at("name").to_string() == "Cy");
    CHECK(first.original().at("age").to_number() == 30);
    CHECK(callm(model.rows()[0], "getValue", {str("name")}).to_string() == "Cy");
    CHECK(callm(model.rows()[1], "getValue", {str("age")}).to_number() == 25);
    CHECK(callm(model.rows()[2], "getValue", {str("team")}).to_string() == "red");

    // The cells of a row, one per column, ids from row and column.
    ts::Array<> cells = as<ts::Array<>>(callm(model.rows()[0], "getAllCells"));
    CHECK(cells.size() == 3);
    using CellP = Cell_CoreProperties<TableFeatures, RowData_map, V>;
    CHECK(as<CellP>(cells[1]).id() == "Cy#0_age");

    // Columns.
    auto all = as<ts::Array<ColC>>(callm(table, "getAllColumns"));
    CHECK(all.size() == 3);
    CHECK(callm(table, "getAllLeafColumns").h != 0);
    auto age = as<ColP>(callm(table, "getColumn", {str("age")}));
    CHECK(age.id() == "age");
    CHECK(!callm(table, "getColumn", {str("nope")}).h);

    // Header groups: one row of three headers.
    ts::Array<> groups = as<ts::Array<>>(callm(table, "getHeaderGroups"));
    CHECK(groups.size() == 1);

    // Sorting: an ascending sort on age puts Al (25), Cy (30), Bo (41) in order.
    auto sorted = as<ColSort>(callm(table, "getColumn", {str("age")}));
    CHECK((callm(sorted, "getIsSorted").to_bool() == false));
    callm(sorted, "toggleSorting", {V::boolean(false)});
    model = as<Model>(callm(table, "getRowModel"));
    CHECK(ids(model.rows()) == "Al#1,Cy#0,Bo#2");
    CHECK(callm(sorted, "getIsSorted").to_string() == "asc");

    // setSorting with a new state: by name, descending: Cy, Bo, Al.
    Obj by_name = Obj::create();
    by_name.set_at("id", str("name"));
    by_name.set_at("desc", V::boolean(true));
    ts::Array<Obj> state{by_name};
    callm(as<TabSort>(table), "setSorting", {val(state)});
    model = as<Model>(callm(table, "getRowModel"));
    CHECK(ids(model.rows()) == "Cy#0,Bo#2,Al#1");
    CHECK((callm(sorted, "getIsSorted").to_bool() == false));
    CHECK(callm(as<ColSort>(callm(table, "getColumn", {str("name")})), "getIsSorted").to_string() == "desc");
    // The unsorted model is still the data's order.
    CHECK(ids(as<Model>(callm(table, "getCoreRowModel")).rows()) == "Cy#0,Al#1,Bo#2");

    // A sort by a text column, ascending.
    ts::Array<Obj> by_team = ts::Array<Obj>::create();
    Obj team = Obj::create();
    team.set_at("id", str("team"));
    team.set_at("desc", V::boolean(false));
    by_team.push(team);
    callm(as<TabSort>(table), "setSorting", {val(by_team)});
    model = as<Model>(callm(table, "getRowModel"));
    CHECK(callm(model.rows()[0], "getValue", {str("team")}).to_string() == "blue");
    CHECK(ids(model.rows()).substr(0, 5) == "Al#1,");

}
