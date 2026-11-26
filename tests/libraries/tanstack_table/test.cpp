// TanStack Table v9 (table-core) from C++, in Node: a table of people with
// accessor columns and a sorted row model, its rows, cells and sorting.
// Every CHECK must hold.
//
// One thing in the packages as bound makes this awkward, noted as finding 13
// below: the table needs a `coreReactivityFeature` (atoms, from a package
// impl.mjs does not give), written here in C++. The table's methods are
// function-typed properties, called with the generated call_<Callback>.
#include <memory>

#include "bindings.hpp"

extern "C" TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

using V = ts::Value;
using Obj = ts::Record<ts::Value>;  // a plain JavaScript object: `{}` with set_at
using Table = Table_Core<Obj>;
using Model = RowModel<Obj>;
using RowC = Row_Core<Obj>;
using RowP = Row_CoreProperties<Obj>;
using ColC = Column_Core<Obj, ts::Value>;
using ColP = Column_CoreProperties<Obj>;
using ColSort = Column_RowSorting<Obj>;
using TabSort = Table_RowSorting<Obj>;

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

// An atom's `set` receives a value or an updater function: the library's own
// functionalUpdate(fn, x) is `typeof fn === "function" ? fn(x) : fn`.
static V apply_update(const ts::Handle& update, V old) {
    return functionalUpdate<V>(Updater<V>(std::in_place_index<0>, val(update)), old);
}

// FINDING 13: constructTable needs features.coreReactivityFeature (atoms);
// @tanstack/table-core only has them through `renderPhaseReactivity(primitives)`
// (needs @tanstack/store) or '@tanstack/table-core/store-reactivity-bindings',
// and impl.mjs hands over neither. Atoms are made here: objects of get/set/subscribe.
static V unsubscribe_object() {
    Obj sub = Obj::create();
    sub.set_at("unsubscribe", val(void_fn([] {})));
    return val(sub);
}
static V writable_atom(V initial) {
    auto state = std::make_shared<V>(initial);
    Obj atom = Obj::create();
    atom.set_at("get", val(void_to_T_fn<V>([state] { return *state; })));
    atom.set_at("set", val(UpdaterFn<V>([state](V update) {
                    *state = apply_update(update, *state);
                    return V();
                })));
    atom.set_at("subscribe", val(UpdaterFn<V>([](V) { return unsubscribe_object(); })));
    return val(atom);
}
static V readonly_atom(void_to_T_fn<V> compute) {
    Obj atom = Obj::create();
    atom.set_at("get", val(void_to_T_fn<V>([compute] { return compute(); })));
    atom.set_at("subscribe", val(UpdaterFn<V>([](V) { return unsubscribe_object(); })));
    return val(atom);
}
static V reactivity() {
    TableReactivityBindings r = TableReactivityBindings::create();
    r.createOptionsStore(false);
    r.wrapExternalAtoms(false);
    r.addSubscription(AddSubscriptionFn([](Subscription) {}));
    r.createWritableAtom(CreateWritableAtomFn([](V initial, std::optional<TableAtomOptions<V>>) { return writable_atom(initial); }));
    r.createReadonlyAtom(CreateReadonlyAtomFn([](void_to_T_fn<V> compute, std::optional<TableAtomOptions<V>>) { return readonly_atom(compute); }));
    r.untrack(UntrackFn([](void_to_T_fn<V> fn) { return fn(); }));
    r.batch(fn_fn([](void_fn fn) { fn(); }));
    r.schedule(fn_fn([](void_fn fn) { fn(); }));
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
    features.set_at("sortedRowModel", val(createSortedRowModel<Obj>()));

    ts::Array<Obj> data{person("Cy", 30, "red"), person("Al", 25, "blue"), person("Bo", 41, "red")};
    ts::Array<Obj> columns{column("name", "Name"), column("age", "Age"), column("team", "Team")};
    CHECK(data.size() == 3 && columns.size() == 3);

    Obj options = Obj::create();
    options.set_at("features", val(features));
    options.set_at("columns", val(columns));
    options.set_at("data", val(data));
    // Row ids from the data: a function of the row, the index and the parent.
    options.set_at("getRowId", val(TableOptionsRowsGetRowIdFn<Obj>([](Obj row, double index, std::optional<RowC>) {
                       return row.at("name").to_string() + "#" + std::to_string(int(index));
                   })));

    Table table = constructTable<Obj>(as<TableOptions_Core<Obj>>(options));
    CHECK(table.h != 0);

    // The row model: three rows, by index as ids.
    Model model = table.getRowModel();
    CHECK(model.rows().size() == 3);
    CHECK(ids(model.rows()) == "Cy#0,Al#1,Bo#2");
    CHECK(table.getCoreRowModel().rows().size() == 3);

    // Rows: the original data, and cell values by column id.
    auto first = as<RowP>(model.rows()[0]);
    CHECK(first.original().at("name").to_string() == "Cy");
    CHECK(first.original().at("age").to_number() == 30);
    CHECK(model.rows()[0].getValue("name").to_string() == "Cy");
    CHECK(model.rows()[1].getValue("age").to_number() == 25);
    CHECK(model.rows()[2].getValue("team").to_string() == "red");

    // The cells of a row, one per column, ids from row and column.
    auto cells = model.rows()[0].getAllCells();
    CHECK(cells.size() == 3);
    using CellP = Cell_CoreProperties<Obj>;
    CHECK(as<CellP>(cells[1]).id() == "Cy#0_age");

    // Columns.
    auto all = table.getAllColumns();
    CHECK(all.size() == 3);
    CHECK(table.getAllLeafColumns().h != 0);
    auto agec = table.getColumn("age");
    CHECK(agec.has_value());
    auto age = as<ColP>(*agec);
    CHECK(age.id() == "age");
    CHECK(!table.getColumn("nope").has_value());

    // Header groups: one row of three headers.
    CHECK(table.getHeaderGroups().size() == 1);

    // Sorting: an ascending sort on age puts Al (25), Cy (30), Bo (41) in order.
    auto sorted = as<ColSort>(*table.getColumn("age"));
    auto isSorted = [&](const ColSort& c) { return c.getIsSorted(); };
    // `false | SortDirection`: none when not sorted.
    auto notSorted = [&](const ColSort& c) { return !isSorted(c).has_value(); };
    CHECK(notSorted(sorted));
    sorted.toggleSorting(false, std::nullopt);
    model = table.getRowModel();
    CHECK(ids(model.rows()) == "Al#1,Cy#0,Bo#2");
    CHECK(isSorted(sorted) == SortDirection::asc);

    // setSorting with a new state: by name, descending: Cy, Bo, Al.
    ColumnSort by_name = ColumnSort::create();
    by_name.id("name");
    by_name.desc(true);
    ts::Array<ColumnSort> state{by_name};
    as<TabSort>(table).setSorting(Updater<ts::Array<ColumnSort>>(state));
    model = table.getRowModel();
    CHECK(ids(model.rows()) == "Cy#0,Bo#2,Al#1");
    CHECK(notSorted(sorted));
    auto byName = as<ColSort>(*table.getColumn("name"));
    CHECK(isSorted(byName) == SortDirection::desc);
    // The unsorted model is still the data's order.
    CHECK(ids(table.getCoreRowModel().rows()) == "Cy#0,Al#1,Bo#2");

    // A sort by a text column, ascending.
    ColumnSort team = ColumnSort::create();
    team.id("team");
    team.desc(false);
    ts::Array<ColumnSort> by_team{team};
    as<TabSort>(table).setSorting(Updater<ts::Array<ColumnSort>>(by_team));
    model = table.getRowModel();
    CHECK(model.rows()[0].getValue("team").to_string() == "blue");
    CHECK(ids(model.rows()).substr(0, 5) == "Al#1,");
}
