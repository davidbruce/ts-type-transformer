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
static Obj col_def(const char* key, const char* hdr, const char* agg) {
    Obj c = Obj::create();
    c.set_at("accessorKey", str_v(key));
    c.set_at("header", str_v(hdr));
    if (agg) c.set_at("aggregationFn", str_v(agg));
    return c;
}
static Obj host_row(const char* region, const char* host, const char* role, double cpu, double mem) {
    Obj r = Obj::create();
    r.set_at("region", str_v(region));
    r.set_at("host", str_v(host));
    r.set_at("role", str_v(role));
    r.set_at("cpu", V::number(cpu));
    r.set_at("mem", V::number(mem));
    r.set_at("nodes", V::number(1.0));
    return r;
}

struct AppState {
    TableT table;
    dom::HTMLElement wrap;
    dom::HTMLElement meta;
};

static void render(const std::shared_ptr<AppState>& st) {
    ts::Array<RowT> rows = call_Table_RowModels_getCoreRowModel(st->table.getRowModel()).rows();
    V store_v = apply_upd(as<Obj>(as<Table_CoreProperties<TableFeatures, Obj>>(st->table).store()).at("get"), V());
    ts::Array<std::string> grp_arr = as<ts::Array<std::string>>(as<Obj>(store_v).at("grouping"));
    std::string grp_col = (grp_arr.size() > 0) ? grp_arr[0] : "region";

    std::string html = "<table><thead><tr><th>Group / Host</th><th>Region</th><th>Role</th><th class=\"num\">CPU %</th><th class=\"num\">Mem GB</th><th class=\"num\">Nodes</th></tr></thead><tbody>";
    char nbuf[96];
    int group_cnt = 0;
    for (uint32_t i = 0; i < rows.size(); i++) {
        RowT r = rows[i];
        bool is_grp = call_void_to_boolean_fn(as<Row_ColumnGrouping>(r).getIsGrouped());
        double cpu = call_string_to_TValue_fn(r.getValue(), "cpu").to_number();
        double mem = call_string_to_TValue_fn(r.getValue(), "mem").to_number();
        double nodes = call_string_to_TValue_fn(r.getValue(), "nodes").to_number();
        if (is_grp) {
            group_cnt++;
            bool is_exp = call_void_to_boolean_fn(as<Row_RowExpanding>(r).getIsExpanded());
            uint32_t sub_n = as<Row_CoreProperties<TableFeatures, Obj>>(r).subRows().size();
            std::string gval = call_string_to_TValue_fn(r.getValue(), grp_col).to_string();
            std::snprintf(nbuf, sizeof nbuf, "%s %s=%s (%zu)", is_exp ? "[-]" : "[+]", esc_html(grp_col).c_str(), esc_html(gval).c_str(), static_cast<size_t>(sub_n));
            html += std::string("<tr class=\"group-row\"><td>") + nbuf + "</td>";
            html += "<td>" + esc_html(call_string_to_TValue_fn(r.getValue(), "region").to_string()) + "</td>";
            html += "<td>" + esc_html(call_string_to_TValue_fn(r.getValue(), "role").to_string()) + "</td>";
            std::snprintf(nbuf, sizeof nbuf, "<td class=\"num\">%.1f</td><td class=\"num\">%.0f</td><td class=\"num\">%.0f</td></tr>", cpu, mem, nodes);
            html += nbuf;
        } else {
            html += "<tr><td class=\"child-indent\">" + esc_html(call_string_to_TValue_fn(r.getValue(), "host").to_string()) + "</td>";
            html += "<td>" + esc_html(call_string_to_TValue_fn(r.getValue(), "region").to_string()) + "</td>";
            html += "<td>" + esc_html(call_string_to_TValue_fn(r.getValue(), "role").to_string()) + "</td>";
            std::snprintf(nbuf, sizeof nbuf, "<td class=\"num\">%.0f</td><td class=\"num\">%.0f</td><td class=\"num\">%.0f</td></tr>", cpu, mem, nodes);
            html += nbuf;
        }
    }
    html += "</tbody></table>";
    st->wrap.innerHTML(html);

    std::snprintf(nbuf, sizeof nbuf, "Grouped by %s \xC2\xB7 %d groups \xC2\xB7 %zu visible rows", grp_col.c_str(), group_cnt, static_cast<size_t>(rows.size()));
    st->meta.textContent(nbuf);
}

extern "C" __attribute__((export_name("start"))) void start() {
    Obj agg_fns = Obj::create();
    agg_fns.set_at("sum", val(get_aggregationFn_sum()));
    agg_fns.set_at("mean", val(get_aggregationFn_mean()));
    agg_fns.set_at("count", val(get_aggregationFn_count()));

    Obj features = Obj::create();
    features.set_at("coreReactivityFeature", make_reactivity());
    features.set_at("columnGroupingFeature", val(get_columnGroupingFeature()));
    features.set_at("rowAggregationFeature", val(get_rowAggregationFeature()));
    features.set_at("rowExpandingFeature", val(get_rowExpandingFeature()));
    features.set_at("coreRowModel", val(createCoreRowModel<TableFeatures, Obj>()));
    features.set_at("groupedRowModel", val(createGroupedRowModel<TableFeatures, Obj>()));
    features.set_at("expandedRowModel", val(createExpandedRowModel<TableFeatures, Obj>()));
    features.set_at("aggregationFns", val(agg_fns));

    ts::Array<Obj> data{
        host_row("CH", "zurich-api-01", "api", 72, 64),
        host_row("CH", "basel-db-01", "db", 58, 128),
        host_row("CH", "geneva-edge-01", "edge", 35, 32),
        host_row("DE", "munich-api-02", "api", 84, 64),
        host_row("DE", "berlin-db-02", "db", 66, 128),
        host_row("DE", "frankfurt-edge-02", "edge", 42, 32),
        host_row("FR", "paris-api-03", "api", 60, 64),
        host_row("FR", "lyon-db-03", "db", 48, 128),
        host_row("FR", "marseille-edge-03", "edge", 30, 32),
    };
    ts::Array<Obj> cols{
        col_def("region", "Region", nullptr),
        col_def("host", "Host", nullptr),
        col_def("role", "Role", nullptr),
        col_def("cpu", "CPU % (mean)", "mean"),
        col_def("mem", "Mem GB (sum)", "sum"),
        col_def("nodes", "Nodes (count)", "count"),
    };

    Obj init_state = Obj::create();
    init_state.set_at("grouping", val(ts::Array<std::string>{ "region" }));
    init_state.set_at("expanded", V::boolean(true));

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

    const char* btn_ids[4] = { "btn-0", "btn-1", "btn-2", "btn-3" };
    for (int i = 0; i < 4; i++) {
        dom::HTMLElement btn = *doc.getElementById(btn_ids[i]);
        ts::Listener<dom::PointerEvent> on_click = [st, i](dom::PointerEvent) {
            auto grp = as<Table_ColumnGrouping<TableFeatures, Obj>>(st->table);
            auto exp = as<Table_RowExpanding<TableFeatures, Obj>>(st->table);
            if (i == 0) {
                call_Row_RowExpanding_toggleExpanded(exp.toggleAllRowsExpanded(), true);
            } else if (i == 1) {
                call_Row_RowExpanding_toggleExpanded(exp.toggleAllRowsExpanded(), false);
            } else {
                ts::Array<std::string> next{ (i == 2) ? std::string("region") : std::string("role") };
                call_Table_ColumnGrouping_setGrouping(grp.setGrouping(), Updater<ts::Array<std::string>>(next));
                call_Table_RowExpanding_setExpanded(exp.setExpanded(), Updater<ExpandedState>(ExpandedState(true)));
            }
            render(st);
        };
        btn.addEventListener(dom::HTMLElementEventMapKeys::click, on_click);
    }

    render(st);
}
