#include <cstdio>
#include <memory>
#include <string>

#include "dom/dom.hpp"
#include "tanstack_table_core/tanstack_table_core.hpp"

using namespace tanstack_table_core;
using V = ts::Value;
using Obj = RowData_map;
using TableT = Table_Core<TableFeatures, Obj>;
using RowT = Row_Core<TableFeatures, Obj>;

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
static V str_v(std::string_view s) { return V::string(s); }
static V apply_upd(const ts::Handle& u, V old) {
    return functionalUpdate<V>(Updater<V>(std::in_place_index<0>, val(u)), old);
}

static V unsub_obj() {
    Obj sub = Obj::create();
    sub.set_at("unsubscribe", val(Column_ColumnGrouping_toggleGrouping([] {})));
    return val(sub);
}
static V w_atom(V initial) {
    auto state = std::make_shared<V>(initial);
    Obj atom = Obj::create();
    atom.set_at("get", val(void_to_T_fn<V>([state] { return *state; })));
    atom.set_at("set", val(Updater_fn2<V>([state](V u) { *state = apply_upd(u, *state); return V(); })));
    atom.set_at("subscribe", val(Updater_fn2<V>([](V) { return unsub_obj(); })));
    return val(atom);
}
static V r_atom(void_to_T_fn<V> compute) {
    Obj atom = Obj::create();
    atom.set_at("get", val(void_to_T_fn<V>([compute] { return call_void_to_T_fn<V>(compute); })));
    atom.set_at("subscribe", val(Updater_fn2<V>([](V) { return unsub_obj(); })));
    return val(atom);
}
static V make_reactivity() {
    TableReactivityBindings r = TableReactivityBindings::create();
    r.createOptionsStore(false);
    r.wrapExternalAtoms(false);
    r.addSubscription(TableReactivityBindings_addSubscription([](Subscription) {}));
    r.unmount(Column_ColumnGrouping_toggleGrouping([] {}));
    r.createWritableAtom(TableReactivityBindings_createWritableAtom([](V init, std::optional<TableAtomOptions<V>>) { return w_atom(init); }));
    r.createReadonlyAtom(TableReactivityBindings_createReadonlyAtom([](void_to_T_fn<V> fn, std::optional<TableAtomOptions<V>>) { return r_atom(fn); }));
    r.untrack(TableReactivityBindings_untrack([](void_to_T_fn<V> fn) { return call_void_to_T_fn<V>(fn); }));
    r.batch(fn_fn([](Column_ColumnGrouping_toggleGrouping fn) { call_Column_ColumnGrouping_toggleGrouping(fn); }));
    r.schedule(fn_fn([](Column_ColumnGrouping_toggleGrouping fn) { call_Column_ColumnGrouping_toggleGrouping(fn); }));
    return val(r);
}

static std::string esc_html(const std::string& s) {
    std::string out;
    out.reserve(s.size());
    for (char c : s) {
        if (c == '&') out += "&amp;";
        else if (c == '<') out += "&lt;";
        else if (c == '>') out += "&gt;";
        else out += c;
    }
    return out;
}
static Obj col_def(const char* key, const char* hdr) {
    Obj c = Obj::create();
    c.set_at("accessorKey", str_v(key));
    c.set_at("header", str_v(hdr));
    return c;
}

struct AppState {
    TableT table;
    dom::HTMLElement wrap;
    dom::HTMLElement meta;
};

static PaginationState current_page(const TableT& table) {
    V store_v = apply_upd(as<Obj>(as<Table_CoreProperties<TableFeatures, Obj>>(table).store()).at("get"), V());
    return as<PaginationState>(as<Obj>(store_v).at("pagination"));
}

static void render(const std::shared_ptr<AppState>& st) {
    ts::Array<RowT> rows = call_Table_RowModels_getCoreRowModel(st->table.getRowModel()).rows();
    ts::Array<RowT> core = call_Table_RowModels_getCoreRowModel(st->table.getCoreRowModel()).rows();

    std::string html = "<table><thead><tr><th>Entry</th><th>Account</th><th>Category</th><th class=\"num\">Units</th><th class=\"num\">Amount (CHF)</th></tr></thead><tbody>";
    char nbuf[96];
    for (uint32_t i = 0; i < rows.size(); i++) {
        RowT r = rows[i];
        html += "<tr><td>" + esc_html(call_string_to_TValue_fn(r.getValue(), "id").to_string()) + "</td>";
        html += "<td>" + esc_html(call_string_to_TValue_fn(r.getValue(), "account").to_string()) + "</td>";
        html += "<td>" + esc_html(call_string_to_TValue_fn(r.getValue(), "category").to_string()) + "</td>";
        std::snprintf(nbuf, sizeof nbuf, "%.0f", call_string_to_TValue_fn(r.getValue(), "units").to_number());
        html += std::string("<td class=\"num\">") + nbuf + "</td>";
        std::snprintf(nbuf, sizeof nbuf, "%.0f", call_string_to_TValue_fn(r.getValue(), "amount").to_number());
        html += std::string("<td class=\"num\">") + nbuf + "</td></tr>";
    }
    html += "</tbody></table>";
    st->wrap.innerHTML(html);

    PaginationState pg = current_page(st->table);
    int page_idx = int(pg.pageIndex() + 0.5);
    int page_sz = int(pg.pageSize() + 0.5);
    int page_cnt = int(call_void_to_number_fn(as<Table_RowPagination<TableFeatures, Obj>>(st->table).getPageCount()) + 0.5);
    std::snprintf(
        nbuf, sizeof nbuf,
        "Page %d / %d \xC2\xB7 pageSize %d \xC2\xB7 showing %zu of %zu rows",
        page_idx + 1, page_cnt, page_sz,
        static_cast<size_t>(rows.size()), static_cast<size_t>(core.size())
    );
    st->meta.textContent(nbuf);
}

extern "C" __attribute__((export_name("start"))) void start() {
    Obj features = Obj::create();
    features.set_at("coreReactivityFeature", make_reactivity());
    features.set_at("rowPaginationFeature", val(get_rowPaginationFeature()));
    features.set_at("coreRowModel", val(createCoreRowModel<TableFeatures, Obj>()));
    features.set_at("paginatedRowModel", val(createPaginatedRowModel<TableFeatures, Obj>()));

    const char* accounts[3] = { "ops-pool", "clearing-ch", "reserve-de" };
    const char* cats[4] = { "compute", "storage", "egress", "audit" };
    ts::Array<Obj> data = ts::Array<Obj>::create();
    for (int i = 0; i < 18; i++) {
        char idbuf[16];
        std::snprintf(idbuf, sizeof idbuf, "TX-%03d", i + 1);
        Obj row = Obj::create();
        row.set_at("id", str_v(idbuf));
        row.set_at("account", str_v(accounts[i % 3]));
        row.set_at("category", str_v(cats[i % 4]));
        row.set_at("units", V::number(double((i * 7) % 19 + 4)));
        row.set_at("amount", V::number(double(((i * 37) % 90 + 15) * 12)));
        data.push(row);
    }

    ts::Array<Obj> cols{
        col_def("id", "Entry"),
        col_def("account", "Account"),
        col_def("category", "Category"),
        col_def("units", "Units"),
        col_def("amount", "Amount (CHF)"),
    };

    PaginationState init_pg = PaginationState::create();
    init_pg.pageIndex(0.0);
    init_pg.pageSize(5.0);
    Obj init_state = Obj::create();
    init_state.set_at("pagination", val(init_pg));

    Obj opts = Obj::create();
    opts.set_at("features", val(features));
    opts.set_at("data", val(data));
    opts.set_at("columns", val(cols));
    opts.set_at("initialState", val(init_state));

    dom::Document doc = dom::get_document();
    auto st = std::make_shared<AppState>(AppState{
        constructTable<TableFeatures, Obj>(as<TableOptions_Core<TableFeatures, Obj>>(opts)),
        *doc.getElementById("table-wrap"),
        *doc.getElementById("meta"),
    });

    const char* btn_ids[5] = { "btn-0", "btn-1", "btn-2", "btn-3", "btn-4" };
    for (int i = 0; i < 5; i++) {
        dom::HTMLElement btn = *doc.getElementById(btn_ids[i]);
        ts::Listener<dom::PointerEvent> on_click = [st, i](dom::PointerEvent) {
            auto pag = as<Table_RowPagination<TableFeatures, Obj>>(st->table);
            PaginationState pg = current_page(st->table);
            double idx = pg.pageIndex();
            double max_idx = call_void_to_number_fn(pag.getPageCount()) - 1.0;
            if (i == 0) {
                if (idx > 0.0) call_Table_RowPagination_setPageIndex(pag.setPageIndex(), Updater<double>(idx - 1.0));
            } else if (i == 1) {
                if (idx < max_idx) call_Table_RowPagination_setPageIndex(pag.setPageIndex(), Updater<double>(idx + 1.0));
            } else {
                double new_sz = (i == 2) ? 5.0 : ((i == 3) ? 6.0 : 9.0);
                call_Table_RowPagination_setPageIndex(pag.setPageIndex(), Updater<double>(0.0));
                call_Table_RowPagination_setPageIndex(pag.setPageSize(), Updater<double>(new_sz));
            }
            render(st);
        };
        btn.addEventListener(dom::HTMLElementEventMapKeys::click, on_click);
    }

    render(st);
}
