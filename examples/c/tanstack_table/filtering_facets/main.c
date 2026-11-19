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
static void sb_js_esc(StrBuf* s, JsString jstr) {
    char tmp[128];
    uint32_t len = JsString_copy(jstr, tmp, sizeof tmp - 1);
    tmp[len] = 0;
    for (uint32_t i = 0; i < len; i++) {
        char c = tmp[i];
        if (c == '&') sb_str(s, "&amp;");
        else if (c == '<') sb_str(s, "&lt;");
        else if (c == '>') sb_str(s, "&gt;");
        else if (s->pos < 16380) s->buf[s->pos++] = c;
    }
}

static JsValue col_def(const char* key, const char* header, const char* filter_fn) {
    RowData_map c = makeObjectMap();
    RowData_map_set_at(c, "accessorKey", js_string(key));
    RowData_map_set_at(c, "header", js_string(header));
    if (filter_fn) RowData_map_set_at(c, "filterFn", js_string(filter_fn));
    return (JsValue){ c.h };
}
static JsValue svc_row(const char* id, const char* service, const char* tier, const char* region, double reqs, double errs) {
    RowData_map r = makeObjectMap();
    RowData_map_set_at(r, "id", js_string(id));
    RowData_map_set_at(r, "service", js_string(service));
    RowData_map_set_at(r, "tier", js_string(tier));
    RowData_map_set_at(r, "region", js_string(region));
    RowData_map_set_at(r, "requests", js_number(reqs));
    RowData_map_set_at(r, "errors", js_number(errs));
    return (JsValue){ r.h };
}
static JsValue filter_spec(const char* id, JsValue value) {
    RowData_map f = makeObjectMap();
    RowData_map_set_at(f, "id", js_string(id));
    RowData_map_set_at(f, "value", value);
    return (JsValue){ f.h };
}

static Table_Core g_table;
static Document g_doc;

static void render(const char* label) {
    RowModel rm = call_void_to_RowModel_fn(Table_RowModels_Core_get_getRowModel((Table_RowModels_Core){ g_table.h }));
    RowModel core = call_void_to_RowModel_fn(Table_RowModels_Core_get_getCoreRowModel((Table_RowModels_Core){ g_table.h }));
    JsArray_Row_Core rows = RowModel_get_rows(rm);
    uint32_t nrows = JsArray_Row_Core_length(rows);
    uint32_t ntotal = JsArray_Row_Core_length(RowModel_get_rows(core));

    static StrBuf sb;
    sb.pos = 0;
    sb_str(&sb, "<table><thead><tr><th>ID</th><th>Service</th><th>Tier</th><th>Region</th><th class=\"num\">Req/s</th><th class=\"num\">Errors</th></tr></thead><tbody>");
    for (uint32_t i = 0; i < nrows; i++) {
        Row_Core row = JsArray_Row_Core_get(rows, i);
        string_fn gv = Row_Row_get_getValue((Row_Row){ row.h });
        sb_str(&sb, "<tr><td>");
        sb_js_esc(&sb, js_to_string(call_string_fn(gv, "id")));
        sb_str(&sb, "</td><td>");
        sb_js_esc(&sb, js_to_string(call_string_fn(gv, "service")));
        sb_str(&sb, "</td><td>");
        sb_js_esc(&sb, js_to_string(call_string_fn(gv, "tier")));
        sb_str(&sb, "</td><td>");
        sb_js_esc(&sb, js_to_string(call_string_fn(gv, "region")));
        sb_str(&sb, "</td><td class=\"num\">");
        sb_int(&sb, (int)(js_to_number(call_string_fn(gv, "requests")) + 0.5));
        sb_str(&sb, "</td><td class=\"num\">");
        sb_int(&sb, (int)(js_to_number(call_string_fn(gv, "errors")) + 0.5));
        sb_str(&sb, "</td></tr>");
    }
    sb_str(&sb, "</tbody></table>");
    sb.buf[sb.pos] = 0;
    Element_set_innerHTML(Document_getElementById(g_doc, "table-wrap"), sb.buf);

    sb.pos = 0;
    sb_str(&sb, label);
    sb_str(&sb, " \xC2\xB7 showing ");
    sb_int(&sb, (int)nrows);
    sb_str(&sb, " / ");
    sb_int(&sb, (int)ntotal);
    sb_str(&sb, " services");
    sb.buf[sb.pos] = 0;
    Node_set_textContent(Document_getElementById(g_doc, "meta"), sb.buf);
}

static const int BTN_IDS[5] = { 0, 1, 2, 3, 4 };
static void on_click(void* ctx, PointerEvent ev) {
    (void)ev;
    int idx = *(const int*)ctx;
    Table_ColumnFiltering cf = { g_table.h };
    Table_GlobalFiltering gf = { g_table.h };
    JsArray filters = JsArray_new();
    const char* label = "Filter: all services";
    if (idx == 0) {
        call_Updater_fn(Table_GlobalFiltering_get_setGlobalFilter(gf), js_string(""));
        JsArray_push(filters, filter_spec("tier", js_string("core")));
        JsArray_push(filters, filter_spec("requests", js_number(400.0)));
        call_Table_ColumnFiltering_setColumnFilters(Table_ColumnFiltering_get_setColumnFilters(cf), (JsValue){ filters.h });
        label = "Filter: tier = core & requests >= 400";
    } else if (idx == 1) {
        call_Updater_fn(Table_GlobalFiltering_get_setGlobalFilter(gf), js_string(""));
        call_Table_ColumnFiltering_setColumnFilters(Table_ColumnFiltering_get_setColumnFilters(cf), (JsValue){ filters.h });
        label = "Filter: all services";
    } else if (idx == 2) {
        call_Updater_fn(Table_GlobalFiltering_get_setGlobalFilter(gf), js_string(""));
        JsArray_push(filters, filter_spec("tier", js_string("edge")));
        call_Table_ColumnFiltering_setColumnFilters(Table_ColumnFiltering_get_setColumnFilters(cf), (JsValue){ filters.h });
        label = "Filter: tier = edge";
    } else if (idx == 3) {
        call_Table_ColumnFiltering_setColumnFilters(Table_ColumnFiltering_get_setColumnFilters(cf), (JsValue){ filters.h });
        call_Updater_fn(Table_GlobalFiltering_get_setGlobalFilter(gf), js_string("auth"));
        label = "Global search: \"auth\"";
    } else if (idx == 4) {
        call_Updater_fn(Table_GlobalFiltering_get_setGlobalFilter(gf), js_string(""));
        JsArray_push(filters, filter_spec("errors", js_number(5.0)));
        call_Table_ColumnFiltering_setColumnFilters(Table_ColumnFiltering_get_setColumnFilters(cf), (JsValue){ filters.h });
        label = "Filter: errors >= 5";
    }
    render(label);
}

__attribute__((export_name("start"))) void start(void) {
    RowData_map filter_fns = makeObjectMap();
    RowData_map_set_at(filter_fns, "includesString", (JsValue){ get_filterFn_includesString().h });
    RowData_map_set_at(filter_fns, "equalsString", (JsValue){ get_filterFn_equalsString().h });
    RowData_map_set_at(filter_fns, "greaterThanOrEqualTo", (JsValue){ get_filterFn_greaterThanOrEqualTo().h });

    RowData_map features = makeObjectMap();
    RowData_map_set_at(features, "coreReactivityFeature", (JsValue){ make_reactivity().h });
    RowData_map_set_at(features, "columnFilteringFeature", (JsValue){ get_columnFilteringFeature().h });
    RowData_map_set_at(features, "globalFilteringFeature", (JsValue){ get_globalFilteringFeature().h });
    RowData_map_set_at(features, "coreRowModel", (JsValue){ createCoreRowModel().h });
    RowData_map_set_at(features, "filteredRowModel", (JsValue){ createFilteredRowModel().h });
    RowData_map_set_at(features, "filterFns", (JsValue){ filter_fns.h });

    JsArray data = JsArray_new();
    JsArray_push(data, svc_row("S-01", "auth-gateway", "core", "CH", 620, 2));
    JsArray_push(data, svc_row("S-02", "session-cache", "core", "CH", 480, 1));
    JsArray_push(data, svc_row("S-03", "billing-ledger", "core", "DE", 510, 6));
    JsArray_push(data, svc_row("S-04", "oauth-signer", "core", "FR", 430, 3));
    JsArray_push(data, svc_row("S-05", "trace-ingestor", "edge", "DE", 290, 8));
    JsArray_push(data, svc_row("S-06", "cdn-router", "edge", "FR", 840, 5));
    JsArray_push(data, svc_row("S-07", "static-assets", "edge", "CH", 710, 0));
    JsArray_push(data, svc_row("S-08", "mail-relay", "batch", "DE", 140, 7));
    JsArray_push(data, svc_row("S-09", "report-export", "batch", "FR", 210, 4));
    JsArray_push(data, svc_row("S-10", "audit-auth-log", "core", "US", 360, 9));

    JsArray cols = JsArray_new();
    JsArray_push(cols, col_def("id", "ID", 0));
    JsArray_push(cols, col_def("service", "Service", 0));
    JsArray_push(cols, col_def("tier", "Tier", "equalsString"));
    JsArray_push(cols, col_def("region", "Region", 0));
    JsArray_push(cols, col_def("requests", "Req/s", "greaterThanOrEqualTo"));
    JsArray_push(cols, col_def("errors", "Errors", "greaterThanOrEqualTo"));

    JsArray init_filters = JsArray_new();
    JsArray_push(init_filters, filter_spec("tier", js_string("core")));
    JsArray_push(init_filters, filter_spec("requests", js_number(400.0)));
    RowData_map init_state = makeObjectMap();
    RowData_map_set_at(init_state, "columnFilters", (JsValue){ init_filters.h });

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
    render("Filter: tier = core & requests >= 400");
}
