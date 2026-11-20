#include "dom/dom.h"
#include "tanstack_table_core/tanstack_table_core.h"

static const JsValue NONE = { 0 };
static JsValue get_prop(JsValue obj, const char* key) { return RowData_map_get_at((RowData_map){ obj.h }, key); }

typedef struct {
    JsValue value;
    JsValue derive;
    bool is_ro;
} AtomState;
static AtomState atoms[512];
static int atom_count;

static JsValue atom_get(void* ctx) {
    AtomState* a = ctx;
    if (a->is_ro) return call_void_to_any_fn((void_to_any_fn){ a->derive.h });
    js_retain(a->value);
    return a->value;
}
static JsValue atom_set(void* ctx, JsValue updater) {
    AtomState* a = ctx;
    JsValue next = functionalUpdate(updater, a->value);
    js_retain(next);
    a->value = next;
    return NONE;
}
static void no_unsub(void* ctx) { (void)ctx; }
static JsValue atom_sub(void* ctx, JsValue obs) {
    (void)ctx;
    (void)obs;
    RowData_map sub = makeObjectMap();
    RowData_map_set_at(sub, "unsubscribe", (JsValue){ void_fn_from(no_unsub, 0).h });
    return (JsValue){ sub.h };
}
static JsValue make_atom(AtomState* a) {
    RowData_map atom = makeObjectMap();
    RowData_map_set_at(atom, "get", (JsValue){ void_to_any_fn_from(atom_get, a).h });
    if (!a->is_ro) RowData_map_set_at(atom, "set", (JsValue){ Updater_fn2_from(atom_set, a).h });
    RowData_map_set_at(atom, "subscribe", (JsValue){ Updater_fn2_from(atom_sub, 0).h });
    return (JsValue){ atom.h };
}
static Atom create_writable(void* ctx, JsValue initial, TableAtomOptions opts) {
    (void)ctx; (void)opts;
    AtomState* a = &atoms[atom_count++];
    a->value = initial;
    js_retain(initial);
    return (Atom){ make_atom(a).h };
}
static ReadonlyAtom create_readonly(void* ctx, void_to_any_fn fn, TableAtomOptions opts) {
    (void)ctx; (void)opts;
    AtomState* a = &atoms[atom_count++];
    a->is_ro = true;
    a->derive = (JsValue){ fn.h };
    js_retain(fn);
    return (ReadonlyAtom){ make_atom(a).h };
}
static void add_sub(void* ctx, Subscription s) { (void)ctx; (void)s; }
static void unmount_cb(void* ctx) { (void)ctx; }
static void batch_cb(void* ctx, void_fn fn) { (void)ctx; call_void_fn(fn); }
static JsValue untrack_cb(void* ctx, void_to_any_fn fn) { (void)ctx; return call_void_to_any_fn(fn); }

static TableReactivityBindings make_reactivity(void) {
    TableReactivityBindings r = TableReactivityBindings_create();
    TableReactivityBindings_set_createOptionsStore(r, false);
    TableReactivityBindings_set_wrapExternalAtoms(r, false);
    TableReactivityBindings_set_addSubscription(r, TableReactivityBindings_addSubscription_from(add_sub, 0));
    TableReactivityBindings_set_unmount(r, void_fn_from(unmount_cb, 0));
    TableReactivityBindings_set_batch(r, fn_fn_from(batch_cb, 0));
    TableReactivityBindings_set_schedule(r, fn_fn_from(batch_cb, 0));
    TableReactivityBindings_set_untrack(r, TableReactivityBindings_untrack_from(untrack_cb, 0));
    TableReactivityBindings_set_createWritableAtom(r, TableReactivityBindings_createWritableAtom_from(create_writable, 0));
    TableReactivityBindings_set_createReadonlyAtom(r, TableReactivityBindings_createReadonlyAtom_from(create_readonly, 0));
    return r;
}

typedef struct {
    char buf[16384];
    int pos;
} StrBuf;

static void sb_str(StrBuf* s, const char* txt) {
    while (*txt && s->pos < 16380) s->buf[s->pos++] = *txt++;
}
static void sb_int(StrBuf* s, int val) {
    char tmp[24];
    int n = 0;
    if (val == 0) tmp[n++] = '0';
    while (val > 0) {
        tmp[n++] = (char)('0' + (val % 10));
        val /= 10;
    }
    for (int i = n - 1; i >= 0 && s->pos < 16380; i--) s->buf[s->pos++] = tmp[i];
}
static void sb_esc_cstr(StrBuf* s, const char* txt) {
    for (int i = 0; txt[i]; i++) {
        char c = txt[i];
        if (c == '&') sb_str(s, "&amp;");
        else if (c == '<') sb_str(s, "&lt;");
        else if (c == '>') sb_str(s, "&gt;");
        else if (s->pos < 16380) s->buf[s->pos++] = c;
    }
}
static void sb_js_esc(StrBuf* s, JsString jstr) {
    char tmp[128];
    JsString_copy(jstr, tmp, sizeof tmp);
    sb_esc_cstr(s, tmp);
}

static JsValue col_def(const char* key, const char* header) {
    RowData_map c = makeObjectMap();
    RowData_map_set_at(c, "accessorKey", js_string(key));
    RowData_map_set_at(c, "header", js_string(header));
    return (JsValue){ c.h };
}
static JsValue task_row(const char* id, const char* task, const char* owner, const char* zone, double cpu, const char* status) {
    RowData_map r = makeObjectMap();
    RowData_map_set_at(r, "id", js_string(id));
    RowData_map_set_at(r, "task", js_string(task));
    RowData_map_set_at(r, "owner", js_string(owner));
    RowData_map_set_at(r, "zone", js_string(zone));
    RowData_map_set_at(r, "cpu", js_number(cpu));
    RowData_map_set_at(r, "status", js_string(status));
    return (JsValue){ r.h };
}

static Table_Core g_table;
static Document g_doc;
static bool g_zone_visible = false;
static bool g_owner_visible = true;

static void render(void) {
    Table_ColumnPinning pin = { g_table.h };
    Table_ColumnVisibility vis = { g_table.h };
    Table_RowSelection sel = { g_table.h };

    JsArray_Column_Core groups[3] = {
        call_Table_ColumnPinning_getCenterLeafColumns(Table_ColumnPinning_get_getStartVisibleLeafColumns(pin)),
        call_Table_ColumnPinning_getCenterLeafColumns(Table_ColumnPinning_get_getCenterVisibleLeafColumns(pin)),
        call_Table_ColumnPinning_getCenterLeafColumns(Table_ColumnPinning_get_getEndVisibleLeafColumns(pin))
    };

    char col_ids[8][16];
    int ncol = 0;

    static StrBuf sb;
    sb.pos = 0;
    sb_str(&sb, "<table><thead><tr><th>Sel</th>");
    for (int g = 0; g < 3; g++) {
        uint32_t gn = JsArray_Column_Core_length(groups[g]);
        for (uint32_t j = 0; j < gn && ncol < 8; j++) {
            Column_Core col = JsArray_Column_Core_get(groups[g], j);
            JsString cid = Column_CoreProperties_get_id((Column_CoreProperties){ col.h });
            JsString_copy(cid, col_ids[ncol], sizeof col_ids[ncol]);
            bool is_cpu = JsString_equals(cid, "cpu");
            JsString hdr = js_to_string(get_prop((JsValue){ Column_CoreProperties_get_columnDef((Column_CoreProperties){ col.h }).h }, "header"));
            sb_str(&sb, is_cpu ? "<th class=\"num\">" : "<th>");
            sb_js_esc(&sb, hdr);
            if (g != 1) sb_str(&sb, "<span class=\"pin-tag\">PIN</span>");
            sb_str(&sb, "</th>");
            ncol++;
        }
    }
    sb_str(&sb, "</tr></thead><tbody>");

    RowModel rm = call_void_to_RowModel_fn(Table_RowModels_Core_get_getRowModel((Table_RowModels_Core){ g_table.h }));
    JsArray_Row_Core rows = RowModel_get_rows(rm);
    uint32_t nrows = JsArray_Row_Core_length(rows);
    for (uint32_t i = 0; i < nrows; i++) {
        Row_Core row = JsArray_Row_Core_get(rows, i);
        string_fn gv = Row_Row_get_getValue((Row_Row){ row.h });
        bool is_sel = call_void_to_boolean_fn(Row_RowSelection_get_getIsSelected((Row_RowSelection){ row.h }));
        sb_str(&sb, is_sel ? "<tr class=\"selected-row\"><td>&#9632;</td>" : "<tr><td>&#9633;</td>");
        for (int c = 0; c < ncol; c++) {
            if (col_ids[c][0] == 'c' && col_ids[c][1] == 'p' && col_ids[c][2] == 'u' && col_ids[c][3] == 0) {
                sb_str(&sb, "<td class=\"num\">");
                sb_int(&sb, (int)(js_to_number(call_string_fn(gv, "cpu")) + 0.5));
                sb_str(&sb, "%</td>");
            } else {
                sb_str(&sb, "<td>");
                sb_js_esc(&sb, js_to_string(call_string_fn(gv, col_ids[c])));
                sb_str(&sb, "</td>");
            }
        }
        sb_str(&sb, "</tr>");
    }
    sb_str(&sb, "</tbody></table>");
    sb.buf[sb.pos] = 0;
    Element_set_innerHTML(Document_getElementById(g_doc, "table-wrap"), sb.buf);

    uint32_t vis_count = JsArray_Column_Core_length(call_Table_ColumnPinning_getCenterLeafColumns(Table_ColumnVisibility_get_getVisibleLeafColumns(vis)));
    uint32_t all_count = JsArray_Column_Core_length(call_Table_ColumnPinning_getCenterLeafColumns(Table_Columns_get_getAllLeafColumns((Table_Columns){ g_table.h })));
    RowModel sel_rm = call_void_to_RowModel_fn(Table_RowSelection_get_getSelectedRowModel(sel));
    uint32_t sel_count = JsArray_Row_Core_length(RowModel_get_rows(sel_rm));

    sb.pos = 0;
    sb_int(&sb, (int)vis_count);
    sb_str(&sb, " / ");
    sb_int(&sb, (int)all_count);
    sb_str(&sb, " visible columns \xC2\xB7 start:id end:status pinned \xC2\xB7 ");
    sb_int(&sb, (int)sel_count);
    sb_str(&sb, " / ");
    sb_int(&sb, (int)nrows);
    sb_str(&sb, " rows selected");
    sb.buf[sb.pos] = 0;
    Node_set_textContent(Document_getElementById(g_doc, "meta"), sb.buf);
}

static void apply_visibility(void) {
    RowData_map cv = makeObjectMap();
    RowData_map_set_at(cv, "zone", js_bool(g_zone_visible));
    RowData_map_set_at(cv, "owner", js_bool(g_owner_visible));
    call_Table_ColumnVisibility_setColumnVisibility(
        Table_ColumnVisibility_get_setColumnVisibility((Table_ColumnVisibility){ g_table.h }),
        (JsValue){ cv.h }
    );
}

static const int BTN_IDS[5] = { 0, 1, 2, 3, 4 };
static void on_click(void* ctx, PointerEvent ev) {
    (void)ev;
    int idx = *(const int*)ctx;
    Table_ColumnPinning pin = { g_table.h };
    Table_RowSelection sel = { g_table.h };
    if (idx == 0) {
        g_zone_visible = !g_zone_visible;
        apply_visibility();
    } else if (idx == 1) {
        g_owner_visible = !g_owner_visible;
        apply_visibility();
    } else if (idx == 2) {
        RowData_map s = makeObjectMap();
        static const char* keys[7] = { "0", "1", "2", "3", "4", "5", "6" };
        for (int i = 0; i < 7; i++) RowData_map_set_at(s, keys[i], js_bool(true));
        call_Table_RowSelection_setRowSelection(Table_RowSelection_get_setRowSelection(sel), (JsValue){ s.h });
    } else if (idx == 3) {
        RowData_map s = makeObjectMap();
        call_Table_RowSelection_setRowSelection(Table_RowSelection_get_setRowSelection(sel), (JsValue){ s.h });
    } else if (idx == 4) {
        g_zone_visible = false;
        g_owner_visible = true;
        apply_visibility();
        JsArray st = JsArray_new();
        JsArray_push(st, js_string("id"));
        JsArray en = JsArray_new();
        JsArray_push(en, js_string("status"));
        RowData_map cp = makeObjectMap();
        RowData_map_set_at(cp, "start", (JsValue){ st.h });
        RowData_map_set_at(cp, "end", (JsValue){ en.h });
        call_Table_ColumnPinning_setColumnPinning(Table_ColumnPinning_get_setColumnPinning(pin), (JsValue){ cp.h });
        RowData_map s = makeObjectMap();
        RowData_map_set_at(s, "0", js_bool(true));
        RowData_map_set_at(s, "2", js_bool(true));
        RowData_map_set_at(s, "4", js_bool(true));
        call_Table_RowSelection_setRowSelection(Table_RowSelection_get_setRowSelection(sel), (JsValue){ s.h });
    }
    render();
}

__attribute__((export_name("start"))) void start(void) {
    RowData_map features = makeObjectMap();
    RowData_map_set_at(features, "coreReactivityFeature", (JsValue){ make_reactivity().h });
    RowData_map_set_at(features, "columnVisibilityFeature", (JsValue){ get_columnVisibilityFeature().h });
    RowData_map_set_at(features, "columnPinningFeature", (JsValue){ get_columnPinningFeature().h });
    RowData_map_set_at(features, "rowSelectionFeature", (JsValue){ get_rowSelectionFeature().h });
    RowData_map_set_at(features, "coreRowModel", (JsValue){ createCoreRowModel().h });

    JsArray data = JsArray_new();
    JsArray_push(data, task_row("T-101", "ingest-wal-sync", "a.meyer", "eu-ch-1", 42, "running"));
    JsArray_push(data, task_row("T-102", "index-compact", "b.rossi", "eu-ch-2", 68, "running"));
    JsArray_push(data, task_row("T-103", "snapshot-verify", "c.keller", "eu-de-1", 15, "queued"));
    JsArray_push(data, task_row("T-104", "ledger-reconcile", "d.weber", "eu-de-2", 79, "running"));
    JsArray_push(data, task_row("T-105", "cert-rotation", "e.morel", "eu-fr-1", 24, "ready"));
    JsArray_push(data, task_row("T-106", "shard-rebalance", "f.novak", "eu-fr-2", 55, "running"));
    JsArray_push(data, task_row("T-107", "backup-archive", "g.steiner", "eu-ch-1", 31, "ready"));

    JsArray cols = JsArray_new();
    JsArray_push(cols, col_def("id", "Task ID"));
    JsArray_push(cols, col_def("task", "Workload"));
    JsArray_push(cols, col_def("owner", "Owner"));
    JsArray_push(cols, col_def("zone", "Zone"));
    JsArray_push(cols, col_def("cpu", "CPU %"));
    JsArray_push(cols, col_def("status", "Status"));

    RowData_map cv = makeObjectMap();
    RowData_map_set_at(cv, "zone", js_bool(false));

    JsArray st = JsArray_new();
    JsArray_push(st, js_string("id"));
    JsArray en = JsArray_new();
    JsArray_push(en, js_string("status"));
    RowData_map cp = makeObjectMap();
    RowData_map_set_at(cp, "start", (JsValue){ st.h });
    RowData_map_set_at(cp, "end", (JsValue){ en.h });

    RowData_map rs = makeObjectMap();
    RowData_map_set_at(rs, "0", js_bool(true));
    RowData_map_set_at(rs, "2", js_bool(true));
    RowData_map_set_at(rs, "4", js_bool(true));

    RowData_map init_state = makeObjectMap();
    RowData_map_set_at(init_state, "columnVisibility", (JsValue){ cv.h });
    RowData_map_set_at(init_state, "columnPinning", (JsValue){ cp.h });
    RowData_map_set_at(init_state, "rowSelection", (JsValue){ rs.h });

    RowData_map opts = makeObjectMap();
    RowData_map_set_at(opts, "features", (JsValue){ features.h });
    RowData_map_set_at(opts, "data", (JsValue){ data.h });
    RowData_map_set_at(opts, "columns", (JsValue){ cols.h });
    RowData_map_set_at(opts, "initialState", (JsValue){ init_state.h });

    g_table = constructTable((TableOptions_Core){ opts.h });
    js_retain(g_table);
    g_doc = get_document();
    js_retain(g_doc);

    static const char* btn_ids[5] = { "btn-0", "btn-1", "btn-2", "btn-3", "btn-4" };
    for (int i = 0; i < 5; i++) {
        HTMLElement btn = Document_getElementById(g_doc, btn_ids[i]);
        HTMLElement_addEventListener_click(btn, HTMLElement_PointerEvent_fn_from(on_click, (void*)&BTN_IDS[i]));
    }
    render();
}
