#include <cstdio>
#include <memory>
#include <string>
#include <utility>
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
static Obj task_row(const char* id, const char* task, const char* owner, const char* zone, double cpu, const char* status) {
    Obj r = Obj::create();
    r.set_at("id", str_v(id));
    r.set_at("task", str_v(task));
    r.set_at("owner", str_v(owner));
    r.set_at("zone", str_v(zone));
    r.set_at("cpu", V::number(cpu));
    r.set_at("status", str_v(status));
    return r;
}

struct AppState {
    TableT table;
    dom::HTMLElement wrap;
    dom::HTMLElement meta;
    bool zone_vis;
    bool owner_vis;
};

static void render(const std::shared_ptr<AppState>& st) {
    auto pin = as<Table_ColumnPinning<TableFeatures, Obj>>(st->table);
    auto vis = as<Table_ColumnVisibility<TableFeatures, Obj>>(st->table);
    auto sel = as<Table_RowSelection<TableFeatures, Obj>>(st->table);

    ts::Array<ColT> groups[3] = {
        call_Table_ColumnPinning_getCenterLeafColumns(pin.getStartVisibleLeafColumns()),
        call_Table_ColumnPinning_getCenterLeafColumns(pin.getCenterVisibleLeafColumns()),
        call_Table_ColumnPinning_getCenterLeafColumns(pin.getEndVisibleLeafColumns()),
    };

    std::vector<std::string> col_ids;
    std::string html = "<table><thead><tr><th>Sel</th>";
    for (int g = 0; g < 3; g++) {
        for (uint32_t j = 0; j < groups[g].size(); j++) {
            ColP cp = as<ColP>(groups[g][j]);
            std::string cid = cp.id();
            std::string hdr = as<Obj>(cp.columnDef()).at("header").to_string();
            col_ids.push_back(cid);
            html += (cid == "cpu") ? "<th class=\"num\">" : "<th>";
            html += esc_html(hdr);
            if (g != 1) html += "<span class=\"pin-tag\">PIN</span>";
            html += "</th>";
        }
    }
    html += "</tr></thead><tbody>";

    ts::Array<RowT> rows = call_Table_RowModels_getCoreRowModel(st->table.getRowModel()).rows();
    char nbuf[96];
    for (uint32_t i = 0; i < rows.size(); i++) {
        RowT r = rows[i];
        bool is_sel = call_void_to_boolean_fn(as<Row_RowSelection>(r).getIsSelected());
        html += is_sel ? "<tr class=\"selected-row\"><td>&#9632;</td>" : "<tr><td>&#9633;</td>";
        for (const std::string& cid : col_ids) {
            if (cid == "cpu") {
                std::snprintf(nbuf, sizeof nbuf, "%.0f", call_string_to_TValue_fn(r.getValue(), "cpu").to_number());
                html += std::string("<td class=\"num\">") + nbuf + "%</td>";
            } else {
                html += "<td>" + esc_html(call_string_to_TValue_fn(r.getValue(), cid).to_string()) + "</td>";
            }
        }
        html += "</tr>";
    }
    html += "</tbody></table>";
    st->wrap.innerHTML(html);

    size_t vis_n = call_Table_ColumnVisibility_getVisibleFlatColumns(vis.getVisibleLeafColumns()).size();
    size_t all_n = call_Table_Columns_getAllColumns(as<Table_Columns<TableFeatures, Obj>>(st->table).getAllLeafColumns()).size();
    size_t sel_n = call_Table_RowSelection_getFilteredSelectedRowModel(sel.getSelectedRowModel()).rows().size();
    std::snprintf(
        nbuf, sizeof nbuf,
        "%zu / %zu visible columns \xC2\xB7 start:id end:status pinned \xC2\xB7 %zu / %zu rows selected",
        vis_n, all_n, sel_n, static_cast<size_t>(rows.size())
    );
    st->meta.textContent(nbuf);
}

static void apply_vis(const std::shared_ptr<AppState>& st) {
    ColumnVisibilityState cv = ColumnVisibilityState::create();
    cv.set_at("zone", st->zone_vis);
    cv.set_at("owner", st->owner_vis);
    call_Table_ColumnVisibility_setColumnVisibility(
        as<Table_ColumnVisibility<TableFeatures, Obj>>(st->table).setColumnVisibility(),
        Updater<ColumnVisibilityState>(cv)
    );
}

extern "C" __attribute__((export_name("start"))) void start() {
    Obj features = Obj::create();
    features.set_at("coreReactivityFeature", make_reactivity());
    features.set_at("columnVisibilityFeature", val(get_columnVisibilityFeature()));
    features.set_at("columnPinningFeature", val(get_columnPinningFeature()));
    features.set_at("rowSelectionFeature", val(get_rowSelectionFeature()));
    features.set_at("coreRowModel", val(createCoreRowModel<TableFeatures, Obj>()));

    ts::Array<Obj> data{
        task_row("T-101", "ingest-wal-sync", "a.meyer", "eu-ch-1", 42, "running"),
        task_row("T-102", "index-compact", "b.rossi", "eu-ch-2", 68, "running"),
        task_row("T-103", "snapshot-verify", "c.keller", "eu-de-1", 15, "queued"),
        task_row("T-104", "ledger-reconcile", "d.weber", "eu-de-2", 79, "running"),
        task_row("T-105", "cert-rotation", "e.morel", "eu-fr-1", 24, "ready"),
        task_row("T-106", "shard-rebalance", "f.novak", "eu-fr-2", 55, "running"),
        task_row("T-107", "backup-archive", "g.steiner", "eu-ch-1", 31, "ready"),
    };
    ts::Array<Obj> cols{
        col_def("id", "Task ID"),
        col_def("task", "Workload"),
        col_def("owner", "Owner"),
        col_def("zone", "Zone"),
        col_def("cpu", "CPU %"),
        col_def("status", "Status"),
    };

    ColumnVisibilityState cv = ColumnVisibilityState::create();
    cv.set_at("zone", false);

    ColumnPinningState cp = ColumnPinningState::create();
    cp.start(ts::Array<std::string>{ "id" });
    cp.end(ts::Array<std::string>{ "status" });

    RowSelectionState rs = RowSelectionState::create();
    rs.set_at("0", true);
    rs.set_at("2", true);
    rs.set_at("4", true);

    Obj init_state = Obj::create();
    init_state.set_at("columnVisibility", val(cv));
    init_state.set_at("columnPinning", val(cp));
    init_state.set_at("rowSelection", val(rs));

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
        false,
        true,
    });

    const char* btn_ids[5] = { "btn-0", "btn-1", "btn-2", "btn-3", "btn-4" };
    for (int i = 0; i < 5; i++) {
        dom::HTMLElement btn = *doc.getElementById(btn_ids[i]);
        ts::Listener<dom::PointerEvent> on_click = [st, i](dom::PointerEvent) {
            auto pin = as<Table_ColumnPinning<TableFeatures, Obj>>(st->table);
            auto sel = as<Table_RowSelection<TableFeatures, Obj>>(st->table);
            if (i == 0) {
                st->zone_vis = !st->zone_vis;
                apply_vis(st);
            } else if (i == 1) {
                st->owner_vis = !st->owner_vis;
                apply_vis(st);
            } else if (i == 2) {
                call_Table_RowSelection_toggleAllPageRowsSelected(sel.toggleAllRowsSelected(), true, std::nullopt);
            } else if (i == 3) {
                call_Table_RowSelection_toggleAllPageRowsSelected(sel.toggleAllRowsSelected(), false, std::nullopt);
            } else if (i == 4) {
                st->zone_vis = false;
                st->owner_vis = true;
                apply_vis(st);
                ColumnPinningState reset_cp = ColumnPinningState::create();
                reset_cp.start(ts::Array<std::string>{ "id" });
                reset_cp.end(ts::Array<std::string>{ "status" });
                call_Table_ColumnPinning_setColumnPinning(pin.setColumnPinning(), Updater<ColumnPinningState>(reset_cp));
                RowSelectionState reset_rs = RowSelectionState::create();
                reset_rs.set_at("0", true);
                reset_rs.set_at("2", true);
                reset_rs.set_at("4", true);
                call_Table_RowSelection_setRowSelection(sel.setRowSelection(), Updater<RowSelectionState>(reset_rs));
            }
            render(st);
        };
        btn.addEventListener(dom::HTMLElementEventMapKeys::click, on_click);
    }

    render(st);
}
