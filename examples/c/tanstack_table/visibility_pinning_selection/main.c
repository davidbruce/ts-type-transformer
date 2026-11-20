#include <stdarg.h>
#include <stdio.h>
#include <string.h>
#include "dom/dom.h"
#include "tanstack_table_core/tanstack_table_core.h"

static const JsValue NONE = { 0 };

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
    js_scope_open();
    JsValue next = functionalUpdate(updater, a->value);
    js_retain(next);
    if (a->value.h != 0) js_release(a->value);
    a->value = next;
    js_scope_close();
    return NONE;
}
static void no_unsub(void* ctx) { (void)ctx; }
static JsValue atom_sub(void* ctx, JsValue obs) {
    (void)ctx; (void)obs;
    JsValue sub = js_object();
    js_set_val(sub, "unsubscribe", void_fn_from(no_unsub, NULL));
    return sub;
}
static JsValue make_atom(AtomState* a) {
    JsValue atom = js_object();
    js_set_val(atom, "get", void_to_any_fn_from(atom_get, a));
    if (!a->is_ro) js_set_val(atom, "set", Updater_fn2_from(atom_set, a));
    js_set_val(atom, "subscribe", Updater_fn2_from(atom_sub, NULL));
    return atom;
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
    TableReactivityBindings_set_addSubscription(r, TableReactivityBindings_addSubscription_from(add_sub, NULL));
    TableReactivityBindings_set_unmount(r, void_fn_from(unmount_cb, NULL));
    TableReactivityBindings_set_batch(r, fn_fn_from(batch_cb, NULL));
    TableReactivityBindings_set_schedule(r, fn_fn_from(batch_cb, NULL));
    TableReactivityBindings_set_untrack(r, TableReactivityBindings_untrack_from(untrack_cb, NULL));
    TableReactivityBindings_set_createWritableAtom(r, TableReactivityBindings_createWritableAtom_from(create_writable, NULL));
    TableReactivityBindings_set_createReadonlyAtom(r, TableReactivityBindings_createReadonlyAtom_from(create_readonly, NULL));
    return r;
}

typedef struct { char* p; char* end; } HtmlOut;
__attribute__((format(printf, 2, 3)))
static void html_fmt(HtmlOut* o, const char* fmt, ...) {
    if (o->p >= o->end) return;
    va_list ap;
    va_start(ap, fmt);
    int n = vsnprintf(o->p, (size_t)(o->end - o->p), fmt, ap);
    va_end(ap);
    if (n > 0) {
        ptrdiff_t rem = o->end - o->p - 1;
        o->p += (n <= rem) ? n : rem;
    }
}
static void html_js_esc(HtmlOut* o, JsString jstr) {
    char tmp[128];
    uint32_t len = JsString_copy(jstr, tmp, sizeof tmp - 1);
    tmp[len < sizeof tmp ? len : sizeof tmp - 1] = 0;
    for (uint32_t i = 0; tmp[i] && o->p < o->end - 1; i++) {
        if (tmp[i] == '&') html_fmt(o, "&amp;");
        else if (tmp[i] == '<') html_fmt(o, "&lt;");
        else if (tmp[i] == '>') html_fmt(o, "&gt;");
        else *o->p++ = tmp[i];
    }
    if (o->p < o->end) *o->p = 0;
}

static JsValue col_def(const char* key, const char* header) {
    JsValue c = js_object();
    js_set_str(c, "accessorKey", key);
    js_set_str(c, "header", header);
    return c;
}
static JsValue task_row(const char* id, const char* task, const char* owner, const char* zone, double cpu, const char* status) {
    JsValue r = js_object();
    js_set_str(r, "id", id);
    js_set_str(r, "task", task);
    js_set_str(r, "owner", owner);
    js_set_str(r, "zone", zone);
    js_set_num(r, "cpu", cpu);
    js_set_str(r, "status", status);
    return r;
}

static Table_Core g_table;
static Document g_doc;
static bool g_zone_visible = false;
static bool g_owner_visible = true;

static void render(void) {
    js_scope_open();
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

    static char html[16384];
    HtmlOut out = { html, html + sizeof html };
    html_fmt(&out, "<table><thead><tr><th>Sel</th>");
    for (int g = 0; g < 3; g++) {
        uint32_t gn = JsArray_Column_Core_length(groups[g]);
        for (uint32_t j = 0; j < gn && ncol < 8; j++) {
            Column_Core col = JsArray_Column_Core_get(groups[g], j);
            JsString cid = Column_Core_get_id(col);
            uint32_t clen = JsString_copy(cid, col_ids[ncol], sizeof col_ids[ncol] - 1);
            col_ids[ncol][clen < sizeof col_ids[ncol] ? clen : sizeof col_ids[ncol] - 1] = 0;
            bool is_cpu = JsString_equals(cid, "cpu");
            JsString hdr = js_to_string(js_get_prop(Column_Core_get_columnDef(col), "header"));
            html_fmt(&out, "%s", is_cpu ? "<th class=\"num\">" : "<th>");
            html_js_esc(&out, hdr);
            if (g != 1) html_fmt(&out, "<span class=\"pin-tag\">PIN</span>");
            html_fmt(&out, "</th>");
            ncol++;
        }
    }
    html_fmt(&out, "</tr></thead><tbody>");

    RowModel rm = call_void_to_RowModel_fn(Table_RowModels_Core_get_getRowModel((Table_RowModels_Core){ g_table.h }));
    JsArray_Row_Core rows = RowModel_get_rows(rm);
    uint32_t nrows = JsArray_Row_Core_length(rows);
    for (uint32_t i = 0; i < nrows; i++) {
        Row_Core row = JsArray_Row_Core_get(rows, i);
        string_fn gv = Row_Core_get_getValue(row);
        bool is_sel = call_void_to_boolean_fn(Row_RowSelection_get_getIsSelected((Row_RowSelection){ row.h }));
        html_fmt(&out, "%s", is_sel ? "<tr class=\"selected-row\"><td>&#9632;</td>" : "<tr><td>&#9633;</td>");
        for (int c = 0; c < ncol; c++) {
            if (strcmp(col_ids[c], "cpu") == 0) {
                html_fmt(&out, "<td class=\"num\">%d%%</td>", (int)(js_to_number(call_string_fn(gv, "cpu")) + 0.5));
            } else {
                html_fmt(&out, "<td>");
                html_js_esc(&out, js_to_string(call_string_fn(gv, col_ids[c])));
                html_fmt(&out, "</td>");
            }
        }
        html_fmt(&out, "</tr>");
    }
    html_fmt(&out, "</tbody></table>");
    HTMLElement_set_innerHTML(Document_getElementById(g_doc, "table-wrap"), html);

    uint32_t vis_count = JsArray_Column_Core_length(call_Table_ColumnPinning_getCenterLeafColumns(Table_ColumnVisibility_get_getVisibleLeafColumns(vis)));
    uint32_t all_count = JsArray_Column_Core_length(call_Table_ColumnPinning_getCenterLeafColumns(Table_Core_get_getAllLeafColumns(g_table)));
    RowModel sel_rm = call_void_to_RowModel_fn(Table_RowSelection_get_getSelectedRowModel(sel));
    uint32_t sel_count = JsArray_Row_Core_length(RowModel_get_rows(sel_rm));

    char meta[256];
    snprintf(meta, sizeof meta, "%u / %u visible columns \xC2\xB7 start:id end:status pinned \xC2\xB7 %u / %u rows selected",
             vis_count, all_count, sel_count, nrows);
    HTMLElement_set_textContent(Document_getElementById(g_doc, "meta"), meta);
    js_scope_close();
}

static void apply_visibility(void) {
    js_scope_open();
    JsValue cv = js_object();
    js_set_bool(cv, "zone", g_zone_visible);
    js_set_bool(cv, "owner", g_owner_visible);
    call_Table_ColumnVisibility_setColumnVisibility(
        Table_ColumnVisibility_get_setColumnVisibility((Table_ColumnVisibility){ g_table.h }),
        cv
    );
    js_scope_close();
}

static const int BTN_IDS[5] = { 0, 1, 2, 3, 4 };
static void on_click(void* ctx, PointerEvent ev) {
    (void)ev;
    js_scope_open();
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
        JsValue s = js_object();
        static const char* keys[7] = { "0", "1", "2", "3", "4", "5", "6" };
        for (int i = 0; i < 7; i++) js_set_bool(s, keys[i], true);
        call_Table_RowSelection_setRowSelection(Table_RowSelection_get_setRowSelection(sel), s);
    } else if (idx == 3) {
        call_Table_RowSelection_setRowSelection(Table_RowSelection_get_setRowSelection(sel), js_object());
    } else if (idx == 4) {
        g_zone_visible = false;
        g_owner_visible = true;
        apply_visibility();
        JsArray st = JsArray_new();
        js_push_str(st, "id");
        JsArray en = JsArray_new();
        js_push_str(en, "status");
        JsValue cp = js_object();
        js_set_val(cp, "start", st);
        js_set_val(cp, "end", en);
        call_Table_ColumnPinning_setColumnPinning(Table_ColumnPinning_get_setColumnPinning(pin), cp);
        JsValue s = js_object();
        js_set_bool(s, "0", true);
        js_set_bool(s, "2", true);
        js_set_bool(s, "4", true);
        call_Table_RowSelection_setRowSelection(Table_RowSelection_get_setRowSelection(sel), s);
    }
    render();
    js_scope_close();
}

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();
    JsValue features = js_object();
    js_set_val(features, "coreReactivityFeature", make_reactivity());
    js_set_val(features, "columnVisibilityFeature", get_columnVisibilityFeature());
    js_set_val(features, "columnPinningFeature", get_columnPinningFeature());
    js_set_val(features, "rowSelectionFeature", get_rowSelectionFeature());
    js_set_val(features, "coreRowModel", createCoreRowModel());

    JsArray data = JsArray_new();
    js_push_val(data, task_row("T-101", "ingest-wal-sync", "a.meyer", "eu-ch-1", 42, "running"));
    js_push_val(data, task_row("T-102", "index-compact", "b.rossi", "eu-ch-2", 68, "running"));
    js_push_val(data, task_row("T-103", "snapshot-verify", "c.keller", "eu-de-1", 15, "queued"));
    js_push_val(data, task_row("T-104", "ledger-reconcile", "d.weber", "eu-de-2", 79, "running"));
    js_push_val(data, task_row("T-105", "cert-rotation", "e.morel", "eu-fr-1", 24, "ready"));
    js_push_val(data, task_row("T-106", "shard-rebalance", "f.novak", "eu-fr-2", 55, "running"));
    js_push_val(data, task_row("T-107", "backup-archive", "g.steiner", "eu-ch-1", 31, "ready"));

    JsArray cols = JsArray_new();
    js_push_val(cols, col_def("id", "Task ID"));
    js_push_val(cols, col_def("task", "Workload"));
    js_push_val(cols, col_def("owner", "Owner"));
    js_push_val(cols, col_def("zone", "Zone"));
    js_push_val(cols, col_def("cpu", "CPU %"));
    js_push_val(cols, col_def("status", "Status"));

    JsValue cv = js_object();
    js_set_bool(cv, "zone", false);

    JsArray st = JsArray_new();
    js_push_str(st, "id");
    JsArray en = JsArray_new();
    js_push_str(en, "status");
    JsValue cp = js_object();
    js_set_val(cp, "start", st);
    js_set_val(cp, "end", en);

    JsValue rs = js_object();
    js_set_bool(rs, "0", true);
    js_set_bool(rs, "2", true);
    js_set_bool(rs, "4", true);

    JsValue init_state = js_object();
    js_set_val(init_state, "columnVisibility", cv);
    js_set_val(init_state, "columnPinning", cp);
    js_set_val(init_state, "rowSelection", rs);

    JsValue opts = js_object();
    js_set_val(opts, "features", features);
    js_set_val(opts, "data", data);
    js_set_val(opts, "columns", cols);
    js_set_val(opts, "initialState", init_state);

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
    js_scope_close();
}
