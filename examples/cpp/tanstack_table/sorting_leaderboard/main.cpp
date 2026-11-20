#include <cstdio>
#include <memory>
#include <string>
#include <vector>

#include "dom/dom.hpp"
#include "tanstack_table_core/tanstack_table_core.hpp"

using namespace tanstack_table_core;
using V = ts::Value;
using Obj = RowData_map;
using TableT = Table_Core<TableFeatures, Obj>;
using RowT = Row_Core<TableFeatures, Obj>;
using ColT = Column_Core<TableFeatures, Obj, V>;
using ColP = Column_CoreProperties<TableFeatures, Obj, V>;

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
    sub.set_at("unsubscribe", val(void_fn([] {})));
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
    r.unmount(void_fn([] {}));
    r.createWritableAtom(TableReactivityBindings_createWritableAtom([](V init, std::optional<TableAtomOptions<V>>) { return w_atom(init); }));
    r.createReadonlyAtom(TableReactivityBindings_createReadonlyAtom([](void_to_T_fn<V> fn, std::optional<TableAtomOptions<V>>) { return r_atom(fn); }));
    r.untrack(TableReactivityBindings_untrack([](void_to_T_fn<V> fn) { return call_void_to_T_fn<V>(fn); }));
    r.batch(fn_fn([](void_fn fn) { call_void_fn(fn); }));
    r.schedule(fn_fn([](void_fn fn) { call_void_fn(fn); }));
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
static Obj node_row(const char* id, const char* host, const char* region, double lat, double thr, double up) {
    Obj r = Obj::create();
    r.set_at("id", str_v(id));
    r.set_at("host", str_v(host));
    r.set_at("region", str_v(region));
    r.set_at("latency", V::number(lat));
    r.set_at("throughput", V::number(thr));
    r.set_at("uptime", V::number(up));
    return r;
}
static ColumnSort sort_spec(const char* id, bool desc) {
    ColumnSort cs = ColumnSort::create();
    cs.id(id);
    cs.desc(desc);
    return cs;
}

struct AppState {
    TableT table;
    dom::HTMLElement wrap;
    dom::HTMLElement meta;
};

static void render(const std::shared_ptr<AppState>& st, const std::string& label) {
    RowModel<TableFeatures, Obj> rm = call_Table_RowModels_getCoreRowModel(st->table.getRowModel());
    ts::Array<RowT> rows = rm.rows();
    ts::Array<ColT> cols = call_Table_Columns_getAllColumns(
        as<Table_Columns<TableFeatures, Obj>>(st->table).getAllLeafColumns()
    );

    std::string html = "<table><thead><tr>";
    for (uint32_t i = 0; i < cols.size(); i++) {
        ColT col = cols[i];
        ColP cp = as<ColP>(col);
        std::string cid = cp.id();
        std::string hdr = as<Obj>(cp.columnDef()).at("header").to_string();
        bool is_num = (cid == "latency" || cid == "throughput" || cid == "uptime");
        html += is_num ? "<th class=\"num\">" : "<th>";
        html += esc_html(hdr);
        auto sd = call_Column_RowSorting_getIsSorted(as<Column_RowSorting<TableFeatures, Obj>>(col).getIsSorted());
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
        std::string host = call_string_to_TValue_fn(r.getValue(), "host").to_string();
        if (i == 0) top_host = host;
        html += "<tr><td>" + esc_html(call_string_to_TValue_fn(r.getValue(), "id").to_string()) + "</td>";
        html += "<td>" + esc_html(host) + "</td>";
        html += "<td>" + esc_html(call_string_to_TValue_fn(r.getValue(), "region").to_string()) + "</td>";
        std::snprintf(nbuf, sizeof nbuf, "%.1f", call_string_to_TValue_fn(r.getValue(), "latency").to_number());
        html += std::string("<td class=\"num\">") + nbuf + "</td>";
        std::snprintf(nbuf, sizeof nbuf, "%.0f", call_string_to_TValue_fn(r.getValue(), "throughput").to_number());
        html += std::string("<td class=\"num\">") + nbuf + "</td>";
        std::snprintf(nbuf, sizeof nbuf, "%.2f", call_string_to_TValue_fn(r.getValue(), "uptime").to_number());
        html += std::string("<td class=\"num\">") + nbuf + "%</td></tr>";
    }
    html += "</tbody></table>";
    st->wrap.innerHTML(html);

    std::snprintf(nbuf, sizeof nbuf, "%zu", static_cast<size_t>(rows.size()));
    st->meta.textContent(label + " \xC2\xB7 " + nbuf + " rows \xC2\xB7 top node: " + top_host);
}

extern "C" __attribute__((export_name("start"))) void start() {
    Obj sort_fns = Obj::create();
    sort_fns.set_at("alphanumeric", val(get_sortFn_alphanumeric()));
    sort_fns.set_at("basic", val(get_sortFn_basic()));
    sort_fns.set_at("text", val(get_sortFn_text()));

    Obj features = Obj::create();
    features.set_at("coreReactivityFeature", make_reactivity());
    features.set_at("rowSortingFeature", val(get_rowSortingFeature()));
    features.set_at("coreRowModel", val(createCoreRowModel<TableFeatures, Obj>()));
    features.set_at("sortedRowModel", val(createSortedRowModel<TableFeatures, Obj>()));
    features.set_at("sortFns", val(sort_fns));

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

    Obj init_state = Obj::create();
    init_state.set_at("sorting", val(ts::Array<ColumnSort>{ sort_spec("region", false), sort_spec("latency", false) }));

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
            call_Table_RowSorting_setSorting(as<Table_RowSorting<TableFeatures, Obj>>(st->table).setSorting(), Updater<ts::Array<ColumnSort>>(next));
            render(st, label);
        };
        btn.addEventListener(dom::HTMLElementEventMapKeys::click, on_click);
    }

    render(st, "Sort: region asc, latency asc");
}
