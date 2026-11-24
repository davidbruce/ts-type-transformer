#include <cstdio>
#include <memory>
#include <string>
#include <vector>

#include "dom/dom.hpp"
#include "tanstack_table_core/tanstack_table_core.hpp"

using namespace tanstack_table_core;
using V = ts::Value;
using Obj = RowData_map;
using TableT = Table_Core<Obj>;
using RowT = Row_Core<Obj>;
using ColT = Column_Core<Obj, V>;
using ColP = Column_CoreProperties<Obj>;

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
static Obj node_row(const char* id, const char* host, const char* region, double lat, double thr, double up) {
    return Obj::create()
        .set_at("id", V::string(id))
        .set_at("host", V::string(host))
        .set_at("region", V::string(region))
        .set_at("latency", V::number(lat))
        .set_at("throughput", V::number(thr))
        .set_at("uptime", V::number(up));
}
static ColumnSort sort_spec(const char* id, bool desc) {
    return ColumnSort({.desc = desc, .id = id});
}

struct AppState {
    TableT table;
    dom::HTMLElement wrap;
    dom::HTMLElement meta;
};

static void render(const std::shared_ptr<AppState>& st, const std::string& label) {
    RowModel<Obj> rm = st->table.getRowModel();
    ts::Array<RowT> rows = rm.rows();
    ts::Array<ColT> cols = st->table.unsafe_as<Table_Columns<Obj>>().getAllLeafColumns();

    std::string html = "<table><thead><tr>";
    for (uint32_t i = 0; i < cols.size(); i++) {
        ColT col = cols[i];
        ColP cp = col.unsafe_as<ColP>();
        std::string cid = cp.id();
        std::string hdr = cp.columnDef().unsafe_as<Obj>().at("header").to_string();
        bool is_num = (cid == "latency" || cid == "throughput" || cid == "uptime");
        html += is_num ? "<th class=\"num\">" : "<th>";
        html += esc_html(hdr);
        auto sd = col.unsafe_as<Column_RowSorting<Obj>>().getIsSorted();
        if (std::holds_alternative<SortDirection>(sd)) {
            html += (std::get<SortDirection>(sd) == SortDirection::asc) ? " &#9650;" : " &#9660;";
        }
        html += "</th>";
    }
    html += "</tr></thead><tbody>";

    char nbuf[64];
    std::string top_host = "-";
    for (uint32_t i = 0; i < rows.size(); i++) {
        RowT r = rows[i];
        std::string host = r.getValue("host").to_string();
        if (i == 0) top_host = host;
        html += "<tr><td>" + esc_html(r.getValue("id").to_string()) + "</td>";
        html += "<td>" + esc_html(host) + "</td>";
        html += "<td>" + esc_html(r.getValue("region").to_string()) + "</td>";
        std::snprintf(nbuf, sizeof nbuf, "%.1f", r.getValue("latency").to_number());
        html += std::string("<td class=\"num\">") + nbuf + "</td>";
        std::snprintf(nbuf, sizeof nbuf, "%.0f", r.getValue("throughput").to_number());
        html += std::string("<td class=\"num\">") + nbuf + "</td>";
        std::snprintf(nbuf, sizeof nbuf, "%.2f", r.getValue("uptime").to_number());
        html += std::string("<td class=\"num\">") + nbuf + "%</td></tr>";
    }
    html += "</tbody></table>";
    st->wrap.innerHTML(html);

    std::snprintf(nbuf, sizeof nbuf, "%zu", static_cast<size_t>(rows.size()));
    st->meta.textContent(label + " \xC2\xB7 " + nbuf + " rows \xC2\xB7 top node: " + top_host);
}

TS_EXPORT(start) void start() {
    auto sort_fns = Obj::create()
        .set_at("alphanumeric", get_sortFn_alphanumeric())
        .set_at("basic", get_sortFn_basic())
        .set_at("text", get_sortFn_text());

    auto features = Obj::create()
        .set_at("coreReactivityFeature", make_reactivity())
        .set_at("rowSortingFeature", get_rowSortingFeature())
        .set_at("coreRowModel", createCoreRowModel<Obj>())
        .set_at("sortedRowModel", createSortedRowModel<Obj>())
        .set_at("sortFns", sort_fns);

    ts::Array<Obj> data{
        node_row("N-01", "eu-west-1a", "CH", 12.4, 4200, 99.98),
        node_row("N-02", "eu-west-1b", "CH", 8.7, 5100, 99.99),
        node_row("N-03", "eu-central-1a", "DE", 15.2, 3900, 99.91),
        node_row("N-04", "eu-central-1b", "DE", 6.3, 6400, 99.97),
        node_row("N-05", "eu-north-1a", "SE", 19.8, 2800, 99.85),
        node_row("N-06", "eu-north-1b", "SE", 11.1, 4750, 99.95),
        node_row("N-07", "us-east-1a", "US", 28.5, 7100, 99.94),
        node_row("N-08", "us-east-1b", "US", 21.0, 5900, 99.96),
    };
    ts::Array<Obj> cols{
        col_def("id", "Node"),
        col_def("host", "Host"),
        col_def("region", "Region"),
        col_def("latency", "Latency (ms)"),
        col_def("throughput", "Req/s"),
        col_def("uptime", "Uptime %"),
    };

    auto init_state = Obj::create()
        .set_at("sorting", ts::Array<ColumnSort>{ sort_spec("region", false), sort_spec("latency", false) });

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
            ts::Array<ColumnSort> next = ts::Array<ColumnSort>::create();
            std::string label = "Sort: unsorted (data order)";
            if (i == 0) {
                next = ts::Array<ColumnSort>{ sort_spec("region", false), sort_spec("latency", false) };
                label = "Sort: region asc, latency asc";
            } else if (i == 1) {
                next = ts::Array<ColumnSort>{ sort_spec("latency", false) };
                label = "Sort: latency asc";
            } else if (i == 2) {
                next = ts::Array<ColumnSort>{ sort_spec("throughput", true) };
                label = "Sort: throughput desc";
            } else if (i == 3) {
                next = ts::Array<ColumnSort>{ sort_spec("host", false) };
                label = "Sort: host asc";
            }
            st->table.unsafe_as<Table_RowSorting<Obj>>().setSorting(next);
            render(st, label);
        });
    }

    render(st, "Sort: region asc, latency asc");
}
