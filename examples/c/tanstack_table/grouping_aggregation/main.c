#include "dom/dom.h"
#include "tanstack_table_core/tanstack_table_core.h"

static const JsValue NONE = { 0 };
static JsValue get_prop(JsValue obj, const char* key) { return RowData_map_get_at((RowData_map){ obj.h }, key); }
static JsValue store_state(JsValue tbl) { return functionalUpdate(get_prop(get_prop(tbl, "store"), "get"), NONE); }

typedef struct {
    JsValue value;
    JsValue derive;
    bool is_ro;
} AtomState;
static AtomState atoms[512];
static int atom_count;

static JsValue atom_get(void* ctx) {
    AtomState* a = ctx;
    if (a->is_ro) return call_void_fn((void_fn){ a->derive.h });
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
static JsValue no_unsub(void* ctx) { (void)ctx; return NONE; }
static JsValue atom_sub(void* ctx, JsValue obs) {
    (void)ctx;
    (void)obs;
    RowData_map sub = makeObjectMap();
    RowData_map_set_at(sub, "unsubscribe", (JsValue){ void_fn_from(no_unsub, 0).h });
    return (JsValue){ sub.h };
}
static JsValue make_atom(AtomState* a) {
    RowData_map atom = makeObjectMap();
    RowData_map_set_at(atom, "get", (JsValue){ void_fn_from(atom_get, a).h });
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
static ReadonlyAtom create_readonly(void* ctx, void_fn fn, TableAtomOptions opts) {
    (void)ctx; (void)opts;
    AtomState* a = &atoms[atom_count++];
    a->is_ro = true;
    a->derive = (JsValue){ fn.h };
    js_retain(fn);
    return (ReadonlyAtom){ make_atom(a).h };
}
static void add_sub(void* ctx, Subscription s) { (void)ctx; (void)s; }
static void unmount_cb(void* ctx) { (void)ctx; }
static void batch_cb(void* ctx, Column_ColumnGrouping_toggleGrouping fn) { (void)ctx; call_Column_ColumnGrouping_toggleGrouping(fn); }
static JsValue untrack_cb(void* ctx, void_fn fn) { (void)ctx; return call_void_fn(fn); }

static TableReactivityBindings make_reactivity(void) {
    TableReactivityBindings r = TableReactivityBindings_create();
    TableReactivityBindings_set_createOptionsStore(r, false);
    TableReactivityBindings_set_wrapExternalAtoms(r, false);
    TableReactivityBindings_set_addSubscription(r, TableReactivityBindings_addSubscription_from(add_sub, 0));
    TableReactivityBindings_set_unmount(r, Column_ColumnGrouping_toggleGrouping_from(unmount_cb, 0));
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
static void sb_fixed1(StrBuf* s, double val) {
    int scaled = (int)(val * 10.0 + 0.5);
    sb_int(s, scaled / 10);
    if (s->pos < 16380) s->buf[s->pos++] = '.';
    sb_int(s, scaled % 10);
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
    uint32_t len = JsString_copy(jstr, tmp, sizeof tmp - 1);
    tmp[len] = 0;
    sb_esc_cstr(s, tmp);
}

static JsValue col_def(const char* key, const char* header, const char* agg_fn) {
    RowData_map c = makeObjectMap();
    RowData_map_set_at(c, "accessorKey", js_string(key));
    RowData_map_set_at(c, "header", js_string(header));
    if (agg_fn) RowData_map_set_at(c, "aggregationFn", js_string(agg_fn));
    return (JsValue){ c.h };
}
static JsValue host_row(const char* region, const char* host, const char* role, double cpu, double mem) {
    RowData_map r = makeObjectMap();
    RowData_map_set_at(r, "region", js_string(region));
    RowData_map_set_at(r, "host", js_string(host));
    RowData_map_set_at(r, "role", js_string(role));
    RowData_map_set_at(r, "cpu", js_number(cpu));
    RowData_map_set_at(r, "mem", js_number(mem));
    RowData_map_set_at(r, "nodes", js_number(1.0));
    return (JsValue){ r.h };
}

static Table_Core g_table;
static Document g_doc;

static void render(void) {
    RowModel rm = call_void_to_RowModel_fn(Table_RowModels_Core_get_getRowModel((Table_RowModels_Core){ g_table.h }));
    JsArray_Row_Core rows = RowModel_get_rows(rm);
    uint32_t nrows = JsArray_Row_Core_length(rows);

    JsArray grp_arr = (JsArray){ get_prop(store_state((JsValue){ g_table.h }), "grouping").h };
    char grp_col[32] = "region";
    if (JsArray_length(grp_arr) > 0) {
        uint32_t glen = JsString_copy(js_to_string(JsArray_get(grp_arr, 0)), grp_col, sizeof grp_col - 1);
        grp_col[glen] = 0;
    }

    static StrBuf sb;
    sb.pos = 0;
    sb_str(&sb, "<table><thead><tr><th>Group / Host</th><th>Region</th><th>Role</th><th class=\"num\">CPU %</th><th class=\"num\">Mem GB</th><th class=\"num\">Nodes</th></tr></thead><tbody>");
    int groups = 0;
    for (uint32_t i = 0; i < nrows; i++) {
        Row_Core row = JsArray_Row_Core_get(rows, i);
        string_fn gv = Row_Row_get_getValue((Row_Row){ row.h });
        bool is_grp = call_void_to_boolean_fn(Row_ColumnGrouping_get_getIsGrouped((Row_ColumnGrouping){ row.h }));
        double cpu = js_to_number(call_string_fn(gv, "cpu"));
        double mem = js_to_number(call_string_fn(gv, "mem"));
        double nodes = js_to_number(call_string_fn(gv, "nodes"));
        if (is_grp) {
            groups++;
            bool is_exp = call_void_to_boolean_fn(Row_RowExpanding_get_getIsExpanded((Row_RowExpanding){ row.h }));
            uint32_t sub_n = JsArray_Row_Core_length(Row_CoreProperties_get_subRows((Row_CoreProperties){ row.h }));
            sb_str(&sb, "<tr class=\"group-row\"><td>");
            sb_str(&sb, is_exp ? "[-] " : "[+] ");
            sb_esc_cstr(&sb, grp_col);
            sb_str(&sb, "=");
            sb_js_esc(&sb, js_to_string(call_string_fn(gv, grp_col)));
            sb_str(&sb, " (");
            sb_int(&sb, (int)sub_n);
            sb_str(&sb, ")</td><td>");
            sb_js_esc(&sb, js_to_string(call_string_fn(gv, "region")));
            sb_str(&sb, "</td><td>");
            sb_js_esc(&sb, js_to_string(call_string_fn(gv, "role")));
            sb_str(&sb, "</td><td class=\"num\">");
            sb_fixed1(&sb, cpu);
            sb_str(&sb, "</td><td class=\"num\">");
            sb_int(&sb, (int)(mem + 0.5));
            sb_str(&sb, "</td><td class=\"num\">");
            sb_int(&sb, (int)(nodes + 0.5));
            sb_str(&sb, "</td></tr>");
        } else {
            sb_str(&sb, "<tr><td class=\"child-indent\">");
            sb_js_esc(&sb, js_to_string(call_string_fn(gv, "host")));
            sb_str(&sb, "</td><td>");
            sb_js_esc(&sb, js_to_string(call_string_fn(gv, "region")));
            sb_str(&sb, "</td><td>");
            sb_js_esc(&sb, js_to_string(call_string_fn(gv, "role")));
            sb_str(&sb, "</td><td class=\"num\">");
            sb_int(&sb, (int)(cpu + 0.5));
            sb_str(&sb, "</td><td class=\"num\">");
            sb_int(&sb, (int)(mem + 0.5));
            sb_str(&sb, "</td><td class=\"num\">");
            sb_int(&sb, (int)(nodes + 0.5));
            sb_str(&sb, "</td></tr>");
        }
    }
    sb_str(&sb, "</tbody></table>");
    sb.buf[sb.pos] = 0;
    Element_set_innerHTML(Document_getElementById(g_doc, "table-wrap"), sb.buf);

    sb.pos = 0;
    sb_str(&sb, "Grouped by ");
    sb_esc_cstr(&sb, grp_col);
    sb_str(&sb, " \xC2\xB7 ");
    sb_int(&sb, groups);
    sb_str(&sb, " groups \xC2\xB7 ");
    sb_int(&sb, (int)nrows);
    sb_str(&sb, " visible rows");
    sb.buf[sb.pos] = 0;
    Node_set_textContent(Document_getElementById(g_doc, "meta"), sb.buf);
}

static const int BTN_IDS[4] = { 0, 1, 2, 3 };
static void on_click(void* ctx, PointerEvent ev) {
    (void)ev;
    int idx = *(const int*)ctx;
    Table_ColumnGrouping grp = { g_table.h };
    Table_RowExpanding exp = { g_table.h };
    if (idx == 0) {
        call_Row_RowExpanding_toggleExpanded(Table_RowExpanding_get_toggleAllRowsExpanded(exp), js_some_bool(true));
    } else if (idx == 1) {
        call_Row_RowExpanding_toggleExpanded(Table_RowExpanding_get_toggleAllRowsExpanded(exp), js_some_bool(false));
    } else {
        JsArray next = JsArray_new();
        JsArray_push(next, js_string(idx == 2 ? "region" : "role"));
        call_Table_ColumnGrouping_setGrouping(Table_ColumnGrouping_get_setGrouping(grp), (JsValue){ next.h });
        call_Table_RowExpanding_setExpanded(Table_RowExpanding_get_setExpanded(exp), js_bool(true));
    }
    render();
}

__attribute__((export_name("start"))) void start(void) {
    RowData_map agg_fns = makeObjectMap();
    RowData_map_set_at(agg_fns, "sum", (JsValue){ get_aggregationFn_sum().h });
    RowData_map_set_at(agg_fns, "mean", (JsValue){ get_aggregationFn_mean().h });
    RowData_map_set_at(agg_fns, "count", (JsValue){ get_aggregationFn_count().h });

    RowData_map features = makeObjectMap();
    RowData_map_set_at(features, "coreReactivityFeature", (JsValue){ make_reactivity().h });
    RowData_map_set_at(features, "columnGroupingFeature", (JsValue){ get_columnGroupingFeature().h });
    RowData_map_set_at(features, "rowAggregationFeature", (JsValue){ get_rowAggregationFeature().h });
    RowData_map_set_at(features, "rowExpandingFeature", (JsValue){ get_rowExpandingFeature().h });
    RowData_map_set_at(features, "coreRowModel", (JsValue){ createCoreRowModel().h });
    RowData_map_set_at(features, "groupedRowModel", (JsValue){ createGroupedRowModel().h });
    RowData_map_set_at(features, "expandedRowModel", (JsValue){ createExpandedRowModel().h });
    RowData_map_set_at(features, "aggregationFns", (JsValue){ agg_fns.h });

    JsArray data = JsArray_new();
    JsArray_push(data, host_row("CH", "zurich-api-01", "api", 72, 64));
    JsArray_push(data, host_row("CH", "basel-db-01", "db", 58, 128));
    JsArray_push(data, host_row("CH", "geneva-edge-01", "edge", 35, 32));
    JsArray_push(data, host_row("DE", "munich-api-02", "api", 84, 64));
    JsArray_push(data, host_row("DE", "berlin-db-02", "db", 66, 128));
    JsArray_push(data, host_row("DE", "frankfurt-edge-02", "edge", 42, 32));
    JsArray_push(data, host_row("FR", "paris-api-03", "api", 60, 64));
    JsArray_push(data, host_row("FR", "lyon-db-03", "db", 48, 128));
    JsArray_push(data, host_row("FR", "marseille-edge-03", "edge", 30, 32));

    JsArray cols = JsArray_new();
    JsArray_push(cols, col_def("region", "Region", 0));
    JsArray_push(cols, col_def("host", "Host", 0));
    JsArray_push(cols, col_def("role", "Role", 0));
    JsArray_push(cols, col_def("cpu", "CPU % (mean)", "mean"));
    JsArray_push(cols, col_def("mem", "Mem GB (sum)", "sum"));
    JsArray_push(cols, col_def("nodes", "Nodes (count)", "count"));

    JsArray init_grp = JsArray_new();
    JsArray_push(init_grp, js_string("region"));
    RowData_map init_state = makeObjectMap();
    RowData_map_set_at(init_state, "grouping", (JsValue){ init_grp.h });
    RowData_map_set_at(init_state, "expanded", js_bool(true));

    RowData_map opts = makeObjectMap();
    RowData_map_set_at(opts, "features", (JsValue){ features.h });
    RowData_map_set_at(opts, "data", (JsValue){ data.h });
    RowData_map_set_at(opts, "columns", (JsValue){ cols.h });
    RowData_map_set_at(opts, "initialState", (JsValue){ init_state.h });

    g_table = constructTable((TableOptions_Core){ opts.h });
    js_retain(g_table);
    g_doc = get_document();
    js_retain(g_doc);

    static const char* btn_ids[4] = { "btn-0", "btn-1", "btn-2", "btn-3" };
    for (int i = 0; i < 4; i++) {
        HTMLElement btn = Document_getElementById(g_doc, btn_ids[i]);
        HTMLElement_addEventListener_click(btn, HTMLElement_PointerEvent_fn_from(on_click, (void*)&BTN_IDS[i]));
    }
    render();
}
