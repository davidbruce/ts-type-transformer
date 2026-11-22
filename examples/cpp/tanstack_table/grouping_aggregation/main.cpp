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
static Obj col_def(const char* key, const char* hdr, const char* agg) {
    auto c = Obj::create().set_at("accessorKey", V::string(key)).set_at("header", V::string(hdr));
    if (agg) c.set_at("aggregationFn", V::string(agg));
    return c;
}
static Obj host_row(const char* region, const char* host, const char* role, double cpu, double mem) {
    return Obj::create()
        .set_at("region", V::string(region))
        .set_at("host", V::string(host))
        .set_at("role", V::string(role))
        .set_at("cpu", V::number(cpu))
        .set_at("mem", V::number(mem))
        .set_at("nodes", V::number(1.0));
}

struct AppState {
    TableT table;
    dom::HTMLElement wrap;
    dom::HTMLElement meta;
};

static void render(const std::shared_ptr<AppState>& st) {
    ts::Array<RowT> rows = st->table.getRowModel().rows();
    V store_v = functionalUpdate<V>(
        st->table.unsafe_as<Table_CoreProperties<TableFeatures, Obj>>().store().unsafe_as<Obj>().at("get"),
        V()
    );
    auto grp_arr = store_v.unsafe_as<Obj>().at("grouping").unsafe_as<ts::Array<std::string>>();
    std::string grp_col = (grp_arr.size() > 0) ? grp_arr[0] : "region";

    std::string html = "<table><thead><tr><th>Group / Host</th><th>Region</th><th>Role</th><th class=\"num\">CPU %</th><th class=\"num\">Mem GB</th><th class=\"num\">Nodes</th></tr></thead><tbody>";
    char nbuf[96];
    int group_cnt = 0;
    for (uint32_t i = 0; i < rows.size(); i++) {
        RowT r = rows[i];
        bool is_grp = r.unsafe_as<Row_ColumnGrouping>().getIsGrouped();
        double cpu = r.getValue("cpu").to_number();
        double mem = r.getValue("mem").to_number();
        double nodes = r.getValue("nodes").to_number();
        if (is_grp) {
            group_cnt++;
            bool is_exp = r.unsafe_as<Row_RowExpanding>().getIsExpanded();
            uint32_t sub_n = r.unsafe_as<Row_CoreProperties<TableFeatures, Obj>>().subRows().size();
            std::string gval = r.getValue(grp_col).to_string();
            std::snprintf(nbuf, sizeof nbuf, "%s %s=%s (%zu)", is_exp ? "[-]" : "[+]", esc_html(grp_col).c_str(), esc_html(gval).c_str(), static_cast<size_t>(sub_n));
            html += std::string("<tr class=\"group-row\"><td>") + nbuf + "</td>";
            html += "<td>" + esc_html(r.getValue("region").to_string()) + "</td>";
            html += "<td>" + esc_html(r.getValue("role").to_string()) + "</td>";
            std::snprintf(nbuf, sizeof nbuf, "<td class=\"num\">%.1f</td><td class=\"num\">%.0f</td><td class=\"num\">%.0f</td></tr>", cpu, mem, nodes);
            html += nbuf;
        } else {
            html += "<tr><td class=\"child-indent\">" + esc_html(r.getValue("host").to_string()) + "</td>";
            html += "<td>" + esc_html(r.getValue("region").to_string()) + "</td>";
            html += "<td>" + esc_html(r.getValue("role").to_string()) + "</td>";
            std::snprintf(nbuf, sizeof nbuf, "<td class=\"num\">%.0f</td><td class=\"num\">%.0f</td><td class=\"num\">%.0f</td></tr>", cpu, mem, nodes);
            html += nbuf;
        }
    }
    html += "</tbody></table>";
    st->wrap.innerHTML(html);

    std::snprintf(nbuf, sizeof nbuf, "Grouped by %s \xC2\xB7 %d groups \xC2\xB7 %zu visible rows", grp_col.c_str(), group_cnt, static_cast<size_t>(rows.size()));
    st->meta.textContent(nbuf);
}

TS_EXPORT(start) void start() {
    auto agg_fns = Obj::create()
        .set_at("sum", get_aggregationFn_sum())
        .set_at("mean", get_aggregationFn_mean())
        .set_at("count", get_aggregationFn_count());

    auto features = Obj::create()
        .set_at("coreReactivityFeature", make_reactivity())
        .set_at("columnGroupingFeature", get_columnGroupingFeature())
        .set_at("rowAggregationFeature", get_rowAggregationFeature())
        .set_at("rowExpandingFeature", get_rowExpandingFeature())
        .set_at("coreRowModel", createCoreRowModel<TableFeatures, Obj>())
        .set_at("groupedRowModel", createGroupedRowModel<TableFeatures, Obj>())
        .set_at("expandedRowModel", createExpandedRowModel<TableFeatures, Obj>())
        .set_at("aggregationFns", agg_fns);

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

    auto init_state = Obj::create()
        .set_at("grouping", ts::Array<std::string>{ "region" })
        .set_at("expanded", V::boolean(true));

    auto opts = Obj::create()
        .set_at("features", features)
        .set_at("data", data)
        .set_at("columns", cols)
        .set_at("initialState", init_state);

    dom::Document doc = dom::get_document();
    auto st = std::make_shared<AppState>(AppState{
        constructTable<TableFeatures, Obj>(opts.unsafe_as<TableOptions_Core<TableFeatures, Obj>>()),
        *doc.getElementById("table-wrap"),
        *doc.getElementById("meta"),
    });

    const char* btn_ids[4] = { "btn-0", "btn-1", "btn-2", "btn-3" };
    for (int i = 0; i < 4; i++) {
        doc.getElementById(btn_ids[i])->addEventListener(dom::HTMLElementEventMapKeys::click, [st, i](dom::PointerEvent) {
            auto grp = st->table.unsafe_as<Table_ColumnGrouping<TableFeatures, Obj>>();
            auto exp = st->table.unsafe_as<Table_RowExpanding<TableFeatures, Obj>>();
            if (i == 0) {
                exp.toggleAllRowsExpanded(true);
            } else if (i == 1) {
                exp.toggleAllRowsExpanded(false);
            } else {
                grp.setGrouping(ts::Array<std::string>{ (i == 2) ? std::string("region") : std::string("role") });
                exp.setExpanded(true);
            }
            render(st);
        });
    }

    render(st);
}
