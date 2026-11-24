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
static Obj task_row(const char* id, const char* task, const char* owner, const char* zone, double cpu, const char* status) {
    return Obj::create()
        .set_at("id", V::string(id))
        .set_at("task", V::string(task))
        .set_at("owner", V::string(owner))
        .set_at("zone", V::string(zone))
        .set_at("cpu", V::number(cpu))
        .set_at("status", V::string(status));
}

struct AppState {
    TableT table;
    dom::HTMLElement wrap;
    dom::HTMLElement meta;
    bool zone_vis;
    bool owner_vis;
};

static void render(const std::shared_ptr<AppState>& st) {
    auto pin = st->table.unsafe_as<Table_ColumnPinning<Obj>>();
    auto vis = st->table.unsafe_as<Table_ColumnVisibility<Obj>>();
    auto sel = st->table.unsafe_as<Table_RowSelection<Obj>>();

    ts::Array<ColT> groups[3] = {
        pin.getStartVisibleLeafColumns(),
        pin.getCenterVisibleLeafColumns(),
        pin.getEndVisibleLeafColumns(),
    };

    std::vector<std::string> col_ids;
    std::string html = "<table><thead><tr><th>Sel</th>";
    for (int g = 0; g < 3; g++) {
        for (uint32_t j = 0; j < groups[g].size(); j++) {
            ColP cp = groups[g][j].unsafe_as<ColP>();
            std::string cid = cp.id();
            std::string hdr = cp.columnDef().unsafe_as<Obj>().at("header").to_string();
            col_ids.push_back(cid);
            html += (cid == "cpu") ? "<th class=\"num\">" : "<th>";
            html += esc_html(hdr);
            if (g != 1) html += "<span class=\"pin-tag\">PIN</span>";
            html += "</th>";
        }
    }
    html += "</tr></thead><tbody>";

    ts::Array<RowT> rows = st->table.getRowModel().rows();
    char nbuf[96];
    for (uint32_t i = 0; i < rows.size(); i++) {
        RowT r = rows[i];
        bool is_sel = r.unsafe_as<Row_RowSelection>().getIsSelected();
        html += is_sel ? "<tr class=\"selected-row\"><td>&#9632;</td>" : "<tr><td>&#9633;</td>";
        for (const std::string& cid : col_ids) {
            if (cid == "cpu") {
                std::snprintf(nbuf, sizeof nbuf, "%.0f", r.getValue("cpu").to_number());
                html += std::string("<td class=\"num\">") + nbuf + "%</td>";
            } else {
                html += "<td>" + esc_html(r.getValue(cid).to_string()) + "</td>";
            }
        }
        html += "</tr>";
    }
    html += "</tbody></table>";
    st->wrap.innerHTML(html);

    size_t vis_n = vis.getVisibleLeafColumns().size();
    size_t all_n = st->table.unsafe_as<Table_Columns<Obj>>().getAllLeafColumns().size();
    size_t sel_n = sel.getSelectedRowModel().rows().size();
    std::snprintf(
        nbuf, sizeof nbuf,
        "%zu / %zu visible columns \xC2\xB7 start:id end:status pinned \xC2\xB7 %zu / %zu rows selected",
        vis_n, all_n, sel_n, static_cast<size_t>(rows.size())
    );
    st->meta.textContent(nbuf);
}

static void apply_vis(const std::shared_ptr<AppState>& st) {
    st->table.unsafe_as<Table_ColumnVisibility<Obj>>().setColumnVisibility(
        ColumnVisibilityState::create().set_at("zone", st->zone_vis).set_at("owner", st->owner_vis)
    );
}

TS_EXPORT(start) void start() {
    auto features = Obj::create()
        .set_at("coreReactivityFeature", make_reactivity())
        .set_at("columnVisibilityFeature", get_columnVisibilityFeature())
        .set_at("columnPinningFeature", get_columnPinningFeature())
        .set_at("rowSelectionFeature", get_rowSelectionFeature())
        .set_at("coreRowModel", createCoreRowModel<Obj>());

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

    auto cv = ColumnVisibilityState::create().set_at("zone", false);
    auto cp = ColumnPinningState({.start = ts::Array<std::string>{ "id" }, .end = ts::Array<std::string>{ "status" }});
    auto rs = RowSelectionState::create().set_at("0", true).set_at("2", true).set_at("4", true);

    auto init_state = Obj::create()
        .set_at("columnVisibility", cv)
        .set_at("columnPinning", cp)
        .set_at("rowSelection", rs);

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
        false,
        true,
    });

    const char* btn_ids[5] = { "btn-0", "btn-1", "btn-2", "btn-3", "btn-4" };
    for (int i = 0; i < 5; i++) {
        doc.getElementById(btn_ids[i])->addEventListener(dom::HTMLElementEventMapKeys::click, [st, i](dom::PointerEvent) {
            auto pin = st->table.unsafe_as<Table_ColumnPinning<Obj>>();
            auto sel = st->table.unsafe_as<Table_RowSelection<Obj>>();
            if (i == 0) {
                st->zone_vis = !st->zone_vis;
                apply_vis(st);
            } else if (i == 1) {
                st->owner_vis = !st->owner_vis;
                apply_vis(st);
            } else if (i == 2) {
                sel.toggleAllRowsSelected(true, std::nullopt);
            } else if (i == 3) {
                sel.toggleAllRowsSelected(false, std::nullopt);
            } else if (i == 4) {
                st->zone_vis = false;
                st->owner_vis = true;
                apply_vis(st);
                pin.setColumnPinning(ColumnPinningState({.start = ts::Array<std::string>{ "id" }, .end = ts::Array<std::string>{ "status" }}));
                sel.setRowSelection(
                    RowSelectionState::create().set_at("0", true).set_at("2", true).set_at("4", true)
                );
            }
            render(st);
        });
    }

    render(st);
}
