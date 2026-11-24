#include <cstdio>
#include <memory>
#include <string>

#include "dom/dom.hpp"
#include "tanstack_table_core/tanstack_table_core.hpp"

using namespace tanstack_table_core;
using V = ts::Value;
using Obj = RowData_map;
using TableT = Table_Core<Obj>;
using RowT = Row_Core<Obj>;

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
static Obj col_def(const char* key, const char* hdr, const char* filter_fn) {
    auto c = Obj::create().set_at("accessorKey", V::string(key)).set_at("header", V::string(hdr));
    if (filter_fn) c.set_at("filterFn", V::string(filter_fn));
    return c;
}
static Obj svc_row(const char* id, const char* svc, const char* tier, const char* region, double reqs, double errs) {
    return Obj::create()
        .set_at("id", V::string(id))
        .set_at("service", V::string(svc))
        .set_at("tier", V::string(tier))
        .set_at("region", V::string(region))
        .set_at("requests", V::number(reqs))
        .set_at("errors", V::number(errs));
}
static ColumnFilter filter_spec(const char* id, V value) {
    return ColumnFilter({.id = id, .value = value});
}

struct AppState {
    TableT table;
    dom::HTMLElement wrap;
    dom::HTMLElement meta;
};

static void render(const std::shared_ptr<AppState>& st, const std::string& label) {
    ts::Array<RowT> rows = st->table.getRowModel().rows();
    ts::Array<RowT> core = st->table.getCoreRowModel().rows();

    std::string html = "<table><thead><tr><th>ID</th><th>Service</th><th>Tier</th><th>Region</th><th class=\"num\">Req/s</th><th class=\"num\">Errors</th></tr></thead><tbody>";
    char nbuf[64];
    for (uint32_t i = 0; i < rows.size(); i++) {
        RowT r = rows[i];
        html += "<tr><td>" + esc_html(r.getValue("id").to_string()) + "</td>";
        html += "<td>" + esc_html(r.getValue("service").to_string()) + "</td>";
        html += "<td>" + esc_html(r.getValue("tier").to_string()) + "</td>";
        html += "<td>" + esc_html(r.getValue("region").to_string()) + "</td>";
        std::snprintf(nbuf, sizeof nbuf, "%.0f", r.getValue("requests").to_number());
        html += std::string("<td class=\"num\">") + nbuf + "</td>";
        std::snprintf(nbuf, sizeof nbuf, "%.0f", r.getValue("errors").to_number());
        html += std::string("<td class=\"num\">") + nbuf + "</td></tr>";
    }
    html += "</tbody></table>";
    st->wrap.innerHTML(html);

    std::snprintf(nbuf, sizeof nbuf, " \xC2\xB7 showing %zu / %zu services", static_cast<size_t>(rows.size()), static_cast<size_t>(core.size()));
    st->meta.textContent(label + nbuf);
}

TS_EXPORT(start) void start() {
    auto filter_fns = Obj::create()
        .set_at("includesString", get_filterFn_includesString())
        .set_at("equalsString", get_filterFn_equalsString())
        .set_at("greaterThanOrEqualTo", get_filterFn_greaterThanOrEqualTo());

    auto features = Obj::create()
        .set_at("coreReactivityFeature", make_reactivity())
        .set_at("columnFilteringFeature", get_columnFilteringFeature())
        .set_at("globalFilteringFeature", get_globalFilteringFeature())
        .set_at("coreRowModel", createCoreRowModel<Obj>())
        .set_at("filteredRowModel", createFilteredRowModel<Obj>())
        .set_at("filterFns", filter_fns);

    ts::Array<Obj> data{
        svc_row("S-01", "auth-gateway", "core", "CH", 620, 2),
        svc_row("S-02", "session-cache", "core", "CH", 480, 1),
        svc_row("S-03", "billing-ledger", "core", "DE", 510, 6),
        svc_row("S-04", "oauth-signer", "core", "FR", 430, 3),
        svc_row("S-05", "trace-ingestor", "edge", "DE", 290, 8),
        svc_row("S-06", "cdn-router", "edge", "FR", 840, 5),
        svc_row("S-07", "static-assets", "edge", "CH", 710, 0),
        svc_row("S-08", "mail-relay", "batch", "DE", 140, 7),
        svc_row("S-09", "report-export", "batch", "FR", 210, 4),
        svc_row("S-10", "audit-auth-log", "core", "US", 360, 9),
    };
    ts::Array<Obj> cols{
        col_def("id", "ID", nullptr),
        col_def("service", "Service", nullptr),
        col_def("tier", "Tier", "equalsString"),
        col_def("region", "Region", nullptr),
        col_def("requests", "Req/s", "greaterThanOrEqualTo"),
        col_def("errors", "Errors", "greaterThanOrEqualTo"),
    };

    auto init_state = Obj::create().set_at("columnFilters", ts::Array<ColumnFilter>{
        filter_spec("tier", V::string("core")),
        filter_spec("requests", V::number(400.0)),
    });

    auto opts = Obj::create()
        .set_at("features", features)
        .set_at("data", data)
        .set_at("columns", cols)
        .set_at("initialState", init_state);

    dom::Document doc = dom::get_document();
    auto st = std::make_shared<AppState>(AppState{
        constructTable<Obj>(opts.unsafe_as<TableOptions_Core<Obj>>()),
        *doc.getElementById("table-wrap"),
        *doc.getElementById("meta"),
    });

    const char* btn_ids[5] = { "btn-0", "btn-1", "btn-2", "btn-3", "btn-4" };
    for (int i = 0; i < 5; i++) {
        doc.getElementById(btn_ids[i])->addEventListener(dom::HTMLElementEventMapKeys::click, [st, i](dom::PointerEvent) {
            auto cf = st->table.unsafe_as<Table_ColumnFiltering>();
            auto gf = st->table.unsafe_as<Table_GlobalFiltering<Obj>>();
            ts::Array<ColumnFilter> next = ts::Array<ColumnFilter>::create();
            std::string label = "Filter: all services";
            if (i == 0) {
                gf.setGlobalFilter(V::string(""));
                next = ts::Array<ColumnFilter>{ filter_spec("tier", V::string("core")), filter_spec("requests", V::number(400.0)) };
                cf.setColumnFilters(next);
                label = "Filter: tier = core & requests >= 400";
            } else if (i == 1) {
                gf.setGlobalFilter(V::string(""));
                cf.setColumnFilters(next);
                label = "Filter: all services";
            } else if (i == 2) {
                gf.setGlobalFilter(V::string(""));
                cf.setColumnFilters(ts::Array<ColumnFilter>{ filter_spec("tier", V::string("edge")) });
                label = "Filter: tier = edge";
            } else if (i == 3) {
                cf.setColumnFilters(next);
                gf.setGlobalFilter(V::string("auth"));
                label = "Global search: \"auth\"";
            } else if (i == 4) {
                gf.setGlobalFilter(V::string(""));
                cf.setColumnFilters(ts::Array<ColumnFilter>{ filter_spec("errors", V::number(5.0)) });
                label = "Filter: errors >= 5";
            }
            render(st, label);
        });
    }

    render(st, "Filter: tier = core & requests >= 400");
}
