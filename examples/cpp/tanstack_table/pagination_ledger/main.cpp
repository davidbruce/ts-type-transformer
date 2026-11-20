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

static V unsub_obj() {
    return Obj::create().set_at("unsubscribe", void_fn([] {}));
}
static V w_atom(V initial) {
    auto state = std::make_shared<V>(initial);
    return Obj::create()
        .set_at("get", void_to_T_fn<V>([state] { return *state; }))
        .set_at("set", Updater_fn2<V>([state](V u) { *state = functionalUpdate<V>(u, *state); return V(); }))
        .set_at("subscribe", Updater_fn2<V>([](V) { return unsub_obj(); }));
}
static V r_atom(void_to_T_fn<V> compute) {
    return Obj::create()
        .set_at("get", void_to_T_fn<V>([compute] { return compute(); }))
        .set_at("subscribe", Updater_fn2<V>([](V) { return unsub_obj(); }));
}
static V make_reactivity() {
    return TableReactivityBindings::create()
        .createOptionsStore(false)
        .wrapExternalAtoms(false)
        .addSubscription([](Subscription) {})
        .unmount([] {})
        .createWritableAtom([](V init, std::optional<TableAtomOptions<V>>) { return w_atom(init); })
        .createReadonlyAtom([](void_to_T_fn<V> fn, std::optional<TableAtomOptions<V>>) { return r_atom(fn); })
        .untrack([](void_to_T_fn<V> fn) { return fn(); })
        .batch([](void_fn fn) { fn(); })
        .schedule([](void_fn fn) { fn(); });
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
    return Obj::create().set_at("accessorKey", V::string(key)).set_at("header", V::string(hdr));
}

struct AppState {
    TableT table;
    dom::HTMLElement wrap;
    dom::HTMLElement meta;
};

static PaginationState current_page(const TableT& table) {
    V store_v = functionalUpdate<V>(
        table.as<Table_CoreProperties<TableFeatures, Obj>>().store().as<Obj>().at("get"),
        V()
    );
    return store_v.as<Obj>().at("pagination").as<PaginationState>();
}

static void render(const std::shared_ptr<AppState>& st) {
    ts::Array<RowT> rows = st->table.getRowModel()().rows();
    ts::Array<RowT> core = st->table.getCoreRowModel()().rows();

    std::string html = "<table><thead><tr><th>Entry</th><th>Account</th><th>Category</th><th class=\"num\">Units</th><th class=\"num\">Amount (CHF)</th></tr></thead><tbody>";
    char nbuf[96];
    for (uint32_t i = 0; i < rows.size(); i++) {
        RowT r = rows[i];
        html += "<tr><td>" + esc_html(r.getValue()("id").to_string()) + "</td>";
        html += "<td>" + esc_html(r.getValue()("account").to_string()) + "</td>";
        html += "<td>" + esc_html(r.getValue()("category").to_string()) + "</td>";
        std::snprintf(nbuf, sizeof nbuf, "%.0f", r.getValue()("units").to_number());
        html += std::string("<td class=\"num\">") + nbuf + "</td>";
        std::snprintf(nbuf, sizeof nbuf, "%.0f", r.getValue()("amount").to_number());
        html += std::string("<td class=\"num\">") + nbuf + "</td></tr>";
    }
    html += "</tbody></table>";
    st->wrap.innerHTML(html);

    PaginationState pg = current_page(st->table);
    int page_idx = int(pg.pageIndex() + 0.5);
    int page_sz = int(pg.pageSize() + 0.5);
    int page_cnt = int(st->table.as<Table_RowPagination<TableFeatures, Obj>>().getPageCount()() + 0.5);
    std::snprintf(
        nbuf, sizeof nbuf,
        "Page %d / %d \xC2\xB7 pageSize %d \xC2\xB7 showing %zu of %zu rows",
        page_idx + 1, page_cnt, page_sz,
        static_cast<size_t>(rows.size()), static_cast<size_t>(core.size())
    );
    st->meta.textContent(nbuf);
}

TS_EXPORT(start) void start() {
    auto features = Obj::create()
        .set_at("coreReactivityFeature", make_reactivity())
        .set_at("rowPaginationFeature", get_rowPaginationFeature())
        .set_at("coreRowModel", createCoreRowModel<TableFeatures, Obj>())
        .set_at("paginatedRowModel", createPaginatedRowModel<TableFeatures, Obj>());

    const char* accounts[3] = { "ops-pool", "clearing-ch", "reserve-de" };
    const char* cats[4] = { "compute", "storage", "egress", "audit" };
    ts::Array<Obj> data = ts::Array<Obj>::create();
    for (int i = 0; i < 18; i++) {
        char idbuf[16];
        std::snprintf(idbuf, sizeof idbuf, "TX-%03d", i + 1);
        data.push(
            Obj::create()
                .set_at("id", V::string(idbuf))
                .set_at("account", V::string(accounts[i % 3]))
                .set_at("category", V::string(cats[i % 4]))
                .set_at("units", V::number(double((i * 7) % 19 + 4)))
                .set_at("amount", V::number(double(((i * 37) % 90 + 15) * 12)))
        );
    }

    ts::Array<Obj> cols{
        col_def("id", "Entry"),
        col_def("account", "Account"),
        col_def("category", "Category"),
        col_def("units", "Units"),
        col_def("amount", "Amount (CHF)"),
    };

    auto init_pg = PaginationState::create().pageIndex(0.0).pageSize(5.0);
    auto init_state = Obj::create().set_at("pagination", init_pg);

    auto opts = Obj::create()
        .set_at("features", features)
        .set_at("data", data)
        .set_at("columns", cols)
        .set_at("initialState", init_state);

    dom::Document doc = dom::get_document();
    auto st = std::make_shared<AppState>(AppState{
        constructTable<TableFeatures, Obj>(opts.as<TableOptions_Core<TableFeatures, Obj>>()),
        *doc.getElementById("table-wrap"),
        *doc.getElementById("meta"),
    });

    const char* btn_ids[5] = { "btn-0", "btn-1", "btn-2", "btn-3", "btn-4" };
    for (int i = 0; i < 5; i++) {
        doc.getElementById(btn_ids[i])->addEventListener(dom::HTMLElementEventMapKeys::click, [st, i](dom::PointerEvent) {
            auto pag = st->table.as<Table_RowPagination<TableFeatures, Obj>>();
            PaginationState pg = current_page(st->table);
            double idx = pg.pageIndex();
            double max_idx = pag.getPageCount()() - 1.0;
            if (i == 0) {
                if (idx > 0.0) pag.setPageIndex()(idx - 1.0);
            } else if (i == 1) {
                if (idx < max_idx) pag.setPageIndex()(idx + 1.0);
            } else {
                double new_sz = (i == 2) ? 5.0 : ((i == 3) ? 6.0 : 9.0);
                pag.setPageIndex()(0.0);
                pag.setPageSize()(new_sz);
            }
            render(st);
        });
    }

    render(st);
}
