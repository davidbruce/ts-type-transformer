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
static Obj col_def(const char* key, const char* hdr, const char* filter_fn) {
    Obj c = Obj::create();
    c.set_at("accessorKey", str_v(key));
    c.set_at("header", str_v(hdr));
    if (filter_fn) c.set_at("filterFn", str_v(filter_fn));
    return c;
}
static Obj svc_row(const char* id, const char* svc, const char* tier, const char* region, double reqs, double errs) {
    Obj r = Obj::create();
    r.set_at("id", str_v(id));
    r.set_at("service", str_v(svc));
    r.set_at("tier", str_v(tier));
    r.set_at("region", str_v(region));
    r.set_at("requests", V::number(reqs));
    r.set_at("errors", V::number(errs));
    return r;
}
static ColumnFilter filter_spec(const char* id, V value) {
    ColumnFilter f = ColumnFilter::create();
    f.id(id);
    f.value(value);
    return f;
}

struct AppState {
    TableT table;
    dom::HTMLElement wrap;
    dom::HTMLElement meta;
};

static void render(const std::shared_ptr<AppState>& st, const std::string& label) {
    ts::Array<RowT> rows = call_Table_RowModels_getCoreRowModel(st->table.getRowModel()).rows();
    ts::Array<RowT> core = call_Table_RowModels_getCoreRowModel(st->table.getCoreRowModel()).rows();

    std::string html = "<table><thead><tr><th>ID</th><th>Service</th><th>Tier</th><th>Region</th><th class=\"num\">Req/s</th><th class=\"num\">Errors</th></tr></thead><tbody>";
    char nbuf[64];
    for (uint32_t i = 0; i < rows.size(); i++) {
        RowT r = rows[i];
        html += "<tr><td>" + esc_html(call_string_to_TValue_fn(r.getValue(), "id").to_string()) + "</td>";
        html += "<td>" + esc_html(call_string_to_TValue_fn(r.getValue(), "service").to_string()) + "</td>";
        html += "<td>" + esc_html(call_string_to_TValue_fn(r.getValue(), "tier").to_string()) + "</td>";
        html += "<td>" + esc_html(call_string_to_TValue_fn(r.getValue(), "region").to_string()) + "</td>";
        std::snprintf(nbuf, sizeof nbuf, "%.0f", call_string_to_TValue_fn(r.getValue(), "requests").to_number());
        html += std::string("<td class=\"num\">") + nbuf + "</td>";
        std::snprintf(nbuf, sizeof nbuf, "%.0f", call_string_to_TValue_fn(r.getValue(), "errors").to_number());
        html += std::string("<td class=\"num\">") + nbuf + "</td></tr>";
    }
    html += "</tbody></table>";
    st->wrap.innerHTML(html);

    std::snprintf(nbuf, sizeof nbuf, " \xC2\xB7 showing %zu / %zu services", static_cast<size_t>(rows.size()), static_cast<size_t>(core.size()));
    st->meta.textContent(label + nbuf);
}

extern "C" __attribute__((export_name("start"))) void start() {
    Obj filter_fns = Obj::create();
    filter_fns.set_at("includesString", val(get_filterFn_includesString()));
    filter_fns.set_at("equalsString", val(get_filterFn_equalsString()));
    filter_fns.set_at("greaterThanOrEqualTo", val(get_filterFn_greaterThanOrEqualTo()));

    Obj features = Obj::create();
    features.set_at("coreReactivityFeature", make_reactivity());
    features.set_at("columnFilteringFeature", val(get_columnFilteringFeature()));
    features.set_at("globalFilteringFeature", val(get_globalFilteringFeature()));
    features.set_at("coreRowModel", val(createCoreRowModel<TableFeatures, Obj>()));
    features.set_at("filteredRowModel", val(createFilteredRowModel<TableFeatures, Obj>()));
    features.set_at("filterFns", val(filter_fns));

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

    Obj init_state = Obj::create();
    init_state.set_at("columnFilters", val(ts::Array<ColumnFilter>{
        filter_spec("tier", str_v("core")),
        filter_spec("requests", V::number(400.0)),
    }));

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
            auto cf = as<Table_ColumnFiltering>(st->table);
            auto gf = as<Table_GlobalFiltering<TableFeatures, Obj>>(st->table);
            ts::Array<ColumnFilter> next = ts::Array<ColumnFilter>::create();
            std::string label = "Filter: all services";
            if (i == 0) {
                call_Updater_fn(gf.setGlobalFilter(), Updater<V>(str_v("")));
                next = ts::Array<ColumnFilter>{ filter_spec("tier", str_v("core")), filter_spec("requests", V::number(400.0)) };
                call_Table_ColumnFiltering_setColumnFilters(cf.setColumnFilters(), Updater<ts::Array<ColumnFilter>>(next));
                label = "Filter: tier = core & requests >= 400";
            } else if (i == 1) {
                call_Updater_fn(gf.setGlobalFilter(), Updater<V>(str_v("")));
                call_Table_ColumnFiltering_setColumnFilters(cf.setColumnFilters(), Updater<ts::Array<ColumnFilter>>(next));
                label = "Filter: all services";
            } else if (i == 2) {
                call_Updater_fn(gf.setGlobalFilter(), Updater<V>(str_v("")));
                next = ts::Array<ColumnFilter>{ filter_spec("tier", str_v("edge")) };
                call_Table_ColumnFiltering_setColumnFilters(cf.setColumnFilters(), Updater<ts::Array<ColumnFilter>>(next));
                label = "Filter: tier = edge";
            } else if (i == 3) {
                call_Table_ColumnFiltering_setColumnFilters(cf.setColumnFilters(), Updater<ts::Array<ColumnFilter>>(next));
                call_Updater_fn(gf.setGlobalFilter(), Updater<V>(str_v("auth")));
                label = "Global search: \"auth\"";
            } else if (i == 4) {
                call_Updater_fn(gf.setGlobalFilter(), Updater<V>(str_v("")));
                next = ts::Array<ColumnFilter>{ filter_spec("errors", V::number(5.0)) };
                call_Table_ColumnFiltering_setColumnFilters(cf.setColumnFilters(), Updater<ts::Array<ColumnFilter>>(next));
                label = "Filter: errors >= 5";
            }
            render(st, label);
        };
        btn.addEventListener(dom::HTMLElementEventMapKeys::click, on_click);
    }

    render(st, "Filter: tier = core & requests >= 400");
}
