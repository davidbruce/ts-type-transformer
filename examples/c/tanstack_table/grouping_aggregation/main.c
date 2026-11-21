#include <stdarg.h>
#include <stdio.h>
#include "dom/dom.h"
#include "tanstack_table_core/tanstack_table_core.h"

static const JsValue NONE = { 0 };
static JsValue store_state(JsValue tbl) { return table_functionalUpdate(js_get_prop(js_get_prop(tbl, "store"), "get"), NONE); }

typedef struct {
    JsValue value;
    JsValue derive;
    bool is_ro;
} AtomState;
static AtomState atoms[512];
static int atom_count;

static JsValue atom_get(void* ctx) {
    AtomState* a = ctx;
    if (a->is_ro) return table_call_void_to_any_fn((Table_void_to_any_fn){ a->derive.h });
    js_retain(a->value);
    return a->value;
}
static JsValue atom_set(void* ctx, JsValue updater) {
    AtomState* a = ctx;
    js_scope_open();
    JsValue next = table_functionalUpdate(updater, a->value);
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
    Table_void_fn unsub = Table_void_fn_from(no_unsub, NULL);
    js_set_val(sub, "unsubscribe", unsub);
    js_release(unsub);
    return sub;
}
static JsValue make_atom(AtomState* a) {
    JsValue atom = js_object();
    Table_void_to_any_fn get_fn = Table_void_to_any_fn_from(atom_get, a);
    js_set_val(atom, "get", get_fn);
    js_release(get_fn);
    if (!a->is_ro) {
        Table_Updater_fn2 set_fn = Table_Updater_fn2_from(atom_set, a);
        js_set_val(atom, "set", set_fn);
        js_release(set_fn);
    }
    Table_Updater_fn2 sub_fn = Table_Updater_fn2_from(atom_sub, NULL);
    js_set_val(atom, "subscribe", sub_fn);
    js_release(sub_fn);
    return atom;
}
static Table_Atom create_writable(void* ctx, JsValue initial, TableAtomOptions opts) {
    (void)ctx; (void)opts;
    if (atom_count >= (int)(sizeof(atoms) / sizeof(atoms[0]))) __builtin_trap();
    AtomState* a = &atoms[atom_count++];
    a->value = initial;
    js_retain(initial);
    return (Table_Atom){ make_atom(a).h };
}
static Table_ReadonlyAtom create_readonly(void* ctx, Table_void_to_any_fn fn, TableAtomOptions opts) {
    (void)ctx; (void)opts;
    if (atom_count >= (int)(sizeof(atoms) / sizeof(atoms[0]))) __builtin_trap();
    AtomState* a = &atoms[atom_count++];
    a->is_ro = true;
    a->derive = (JsValue){ fn.h };
    js_retain(fn);
    return (Table_ReadonlyAtom){ make_atom(a).h };
}
static void add_sub(void* ctx, Table_Subscription s) { (void)ctx; (void)s; }
static void unmount_cb(void* ctx) { (void)ctx; }
static void batch_cb(void* ctx, Table_void_fn fn) { (void)ctx; table_call_void_fn(fn); }
static JsValue untrack_cb(void* ctx, Table_void_to_any_fn fn) { (void)ctx; return table_call_void_to_any_fn(fn); }

static TableReactivityBindings make_reactivity(void) {
    TableReactivityBindings r = TableReactivityBindings_create();
    TableReactivityBindings_set_createOptionsStore(r, false);
    TableReactivityBindings_set_wrapExternalAtoms(r, false);
    TableReactivityBindings_set_addSubscription(r, TableReactivityBindings_addSubscription_from(add_sub, NULL));
    TableReactivityBindings_set_unmount(r, Table_void_fn_from(unmount_cb, NULL));
    TableReactivityBindings_set_batch(r, Table_fn_fn_from(batch_cb, NULL));
    TableReactivityBindings_set_schedule(r, Table_fn_fn_from(batch_cb, NULL));
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
static void html_esc(HtmlOut* o, const char* s) {
    for (; *s && o->p < o->end - 1; s++) {
        if (*s == '&') html_fmt(o, "&amp;");
        else if (*s == '<') html_fmt(o, "&lt;");
        else if (*s == '>') html_fmt(o, "&gt;");
        else *o->p++ = *s;
    }
    if (o->p < o->end) *o->p = 0;
}
static void html_js_esc(HtmlOut* o, JsString jstr) {
    char tmp[128];
    uint32_t len = JsString_copy(jstr, tmp, sizeof tmp - 1);
    tmp[len < sizeof tmp ? len : sizeof tmp - 1] = 0;
    html_esc(o, tmp);
}

static JsValue col_def(const char* key, const char* header, const char* agg_fn) {
    JsValue c = js_object();
    js_set_str(c, "accessorKey", key);
    js_set_str(c, "header", header);
    if (agg_fn) js_set_str(c, "aggregationFn", agg_fn);
    return c;
}
static JsValue host_row(const char* region, const char* host, const char* role, double cpu, double mem) {
    JsValue r = js_object();
    js_set_str(r, "region", region);
    js_set_str(r, "host", host);
    js_set_str(r, "role", role);
    js_set_num(r, "cpu", cpu);
    js_set_num(r, "mem", mem);
    js_set_num(r, "nodes", 1.0);
    return r;
}

static Table_Core g_table;
static Document g_doc;

static void render(void) {
    js_scope_open();
    Table_RowModel rm = table_call_void_to_RowModel_fn(Table_RowModels_Core_get_getRowModel((Table_RowModels_Core){ g_table.h }));
    JsArray_Table_Row_Core rows = Table_RowModel_get_rows(rm);
    uint32_t nrows = JsArray_Table_Row_Core_length(rows);

    JsArray grp_arr = js_to_array(js_get_prop(store_state(js_value(g_table)), "grouping"));
    char grp_col[32] = "region";
    if (JsArray_length(grp_arr) > 0) {
        uint32_t glen = JsString_copy(js_to_string(JsArray_get(grp_arr, 0)), grp_col, sizeof grp_col - 1);
        grp_col[glen < sizeof grp_col ? glen : sizeof grp_col - 1] = 0;
    }

    static char html[16384];
    HtmlOut out = { html, html + sizeof html };
    html_fmt(&out, "<table><thead><tr><th>Group / Host</th><th>Region</th><th>Role</th><th class=\"num\">CPU %%</th><th class=\"num\">Mem GB</th><th class=\"num\">Nodes</th></tr></thead><tbody>");
    int groups = 0;
    for (uint32_t i = 0; i < nrows; i++) {
        Table_Row_Core row = JsArray_Table_Row_Core_get(rows, i);
        Table_string_fn gv = Table_Row_Core_get_getValue(row);
        bool is_grp = table_call_void_to_boolean_fn(Table_Row_ColumnGrouping_get_getIsGrouped((Table_Row_ColumnGrouping){ row.h }));
        double cpu = js_to_number(table_call_string_fn(gv, "cpu"));
        double mem = js_to_number(table_call_string_fn(gv, "mem"));
        double nodes = js_to_number(table_call_string_fn(gv, "nodes"));
        if (is_grp) {
            groups++;
            bool is_exp = table_call_void_to_boolean_fn(Table_Row_RowExpanding_get_getIsExpanded((Table_Row_RowExpanding){ row.h }));
            uint32_t sub_n = JsArray_Table_Row_Core_length(Table_Row_Core_get_subRows(row));
            html_fmt(&out, "<tr class=\"group-row\"><td>%s", is_exp ? "[-] " : "[+] ");
            html_esc(&out, grp_col);
            html_fmt(&out, "=");
            html_js_esc(&out, js_to_string(table_call_string_fn(gv, grp_col)));
            html_fmt(&out, " (%u)</td><td>", sub_n);
            html_js_esc(&out, js_to_string(table_call_string_fn(gv, "region")));
            html_fmt(&out, "</td><td>");
            html_js_esc(&out, js_to_string(table_call_string_fn(gv, "role")));
            html_fmt(&out, "</td><td class=\"num\">%.1f</td><td class=\"num\">%d</td><td class=\"num\">%d</td></tr>",
                     cpu, (int)(mem + 0.5), (int)(nodes + 0.5));
        } else {
            html_fmt(&out, "<tr><td class=\"child-indent\">");
            html_js_esc(&out, js_to_string(table_call_string_fn(gv, "host")));
            html_fmt(&out, "</td><td>");
            html_js_esc(&out, js_to_string(table_call_string_fn(gv, "region")));
            html_fmt(&out, "</td><td>");
            html_js_esc(&out, js_to_string(table_call_string_fn(gv, "role")));
            html_fmt(&out, "</td><td class=\"num\">%d</td><td class=\"num\">%d</td><td class=\"num\">%d</td></tr>",
                     (int)(cpu + 0.5), (int)(mem + 0.5), (int)(nodes + 0.5));
        }
    }
    html_fmt(&out, "</tbody></table>");
    HTMLElement_set_innerHTML(Document_getElementById(g_doc, "table-wrap"), html);

    char meta[256];
    snprintf(meta, sizeof meta, "Grouped by %s \xC2\xB7 %d groups \xC2\xB7 %u visible rows", grp_col, groups, nrows);
    HTMLElement_set_textContent(Document_getElementById(g_doc, "meta"), meta);
    js_scope_close();
}

static const int BTN_IDS[4] = { 0, 1, 2, 3 };
static void on_click(void* ctx, PointerEvent ev) {
    (void)ev;
    js_scope_open();
    int idx = *(const int*)ctx;
    Table_ColumnGrouping grp = { g_table.h };
    Table_RowExpanding exp = { g_table.h };
    if (idx == 0) {
        table_call_Row_RowExpanding_toggleExpanded(Table_RowExpanding_get_toggleAllRowsExpanded(exp), true);
    } else if (idx == 1) {
        table_call_Row_RowExpanding_toggleExpanded(Table_RowExpanding_get_toggleAllRowsExpanded(exp), false);
    } else {
        JsArray next = JsArray_new();
        js_push_str(next, idx == 2 ? "region" : "role");
        table_call_Table_ColumnGrouping_setGrouping(Table_ColumnGrouping_get_setGrouping(grp), js_value(next));
        table_call_Table_RowExpanding_setExpanded(Table_RowExpanding_get_setExpanded(exp), js_bool(true));
    }
    render();
    js_scope_close();
}

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();
    JsValue agg_fns = js_object();
    js_set_val(agg_fns, "sum", table_get_aggregationFn_sum());
    js_set_val(agg_fns, "mean", table_get_aggregationFn_mean());
    js_set_val(agg_fns, "count", table_get_aggregationFn_count());

    JsValue features = js_object();
    js_set_val(features, "coreReactivityFeature", make_reactivity());
    js_set_val(features, "columnGroupingFeature", table_get_columnGroupingFeature());
    js_set_val(features, "rowAggregationFeature", table_get_rowAggregationFeature());
    js_set_val(features, "rowExpandingFeature", table_get_rowExpandingFeature());
    js_set_val(features, "coreRowModel", table_createCoreRowModel());
    js_set_val(features, "groupedRowModel", table_createGroupedRowModel());
    js_set_val(features, "expandedRowModel", table_createExpandedRowModel());
    js_set_val(features, "aggregationFns", agg_fns);

    JsArray data = JsArray_new();
    js_push_val(data, host_row("CH", "zurich-api-01", "api", 72, 64));
    js_push_val(data, host_row("CH", "basel-db-01", "db", 58, 128));
    js_push_val(data, host_row("CH", "geneva-edge-01", "edge", 35, 32));
    js_push_val(data, host_row("DE", "munich-api-02", "api", 84, 64));
    js_push_val(data, host_row("DE", "berlin-db-02", "db", 66, 128));
    js_push_val(data, host_row("DE", "frankfurt-edge-02", "edge", 42, 32));
    js_push_val(data, host_row("FR", "paris-api-03", "api", 60, 64));
    js_push_val(data, host_row("FR", "lyon-db-03", "db", 48, 128));
    js_push_val(data, host_row("FR", "marseille-edge-03", "edge", 30, 32));

    JsArray cols = JsArray_new();
    js_push_val(cols, col_def("region", "Region", NULL));
    js_push_val(cols, col_def("host", "Host", NULL));
    js_push_val(cols, col_def("role", "Role", NULL));
    js_push_val(cols, col_def("cpu", "CPU % (mean)", "mean"));
    js_push_val(cols, col_def("mem", "Mem GB (sum)", "sum"));
    js_push_val(cols, col_def("nodes", "Nodes (count)", "count"));

    JsArray init_grp = JsArray_new();
    js_push_str(init_grp, "region");
    JsValue init_state = js_object();
    js_set_val(init_state, "grouping", init_grp);
    js_set_bool(init_state, "expanded", true);

    JsValue opts = js_object();
    js_set_val(opts, "features", features);
    js_set_val(opts, "data", data);
    js_set_val(opts, "columns", cols);
    js_set_val(opts, "initialState", init_state);

    g_table = table_constructTable((TableOptions_Core){ opts.h });
    js_retain(g_table);
    g_doc = get_document();
    js_retain(g_doc);

    static const char* btn_ids[4] = { "btn-0", "btn-1", "btn-2", "btn-3" };
    for (int i = 0; i < 4; i++) {
        HTMLElement btn = Document_getElementById(g_doc, btn_ids[i]);
        HTMLElement_addEventListener_click(btn, HTMLElement_PointerEvent_fn_from(on_click, (void*)&BTN_IDS[i]));
    }
    render();
    js_scope_close();
}
