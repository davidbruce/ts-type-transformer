#include <stdarg.h>
#include <stdio.h>
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

static JsValue col_def(const char* key, const char* header, const char* filter_fn) {
    JsValue c = js_object();
    js_set_str(c, "accessorKey", key);
    js_set_str(c, "header", header);
    if (filter_fn) js_set_str(c, "filterFn", filter_fn);
    return c;
}
static JsValue svc_row(const char* id, const char* service, const char* tier, const char* region, double reqs, double errs) {
    JsValue r = js_object();
    js_set_str(r, "id", id);
    js_set_str(r, "service", service);
    js_set_str(r, "tier", tier);
    js_set_str(r, "region", region);
    js_set_num(r, "requests", reqs);
    js_set_num(r, "errors", errs);
    return r;
}
static JsValue filter_spec(const char* id, JsValue value) {
    JsValue f = js_object();
    js_set_str(f, "id", id);
    js_set_val(f, "value", value);
    return f;
}

static Table_Core g_table;
static Document g_doc;

static void render(const char* label) {
    js_scope_open();
    RowModel rm = call_void_to_RowModel_fn(Table_RowModels_Core_get_getRowModel((Table_RowModels_Core){ g_table.h }));
    RowModel core = call_void_to_RowModel_fn(Table_RowModels_Core_get_getCoreRowModel((Table_RowModels_Core){ g_table.h }));
    JsArray_Row_Core rows = RowModel_get_rows(rm);
    uint32_t nrows = JsArray_Row_Core_length(rows);
    uint32_t ntotal = JsArray_Row_Core_length(RowModel_get_rows(core));

    static char html[16384];
    HtmlOut out = { html, html + sizeof html };
    html_fmt(&out, "<table><thead><tr><th>ID</th><th>Service</th><th>Tier</th><th>Region</th><th class=\"num\">Req/s</th><th class=\"num\">Errors</th></tr></thead><tbody>");
    for (uint32_t i = 0; i < nrows; i++) {
        Row_Core row = JsArray_Row_Core_get(rows, i);
        string_fn gv = Row_Core_get_getValue(row);
        html_fmt(&out, "<tr><td>");
        html_js_esc(&out, js_to_string(call_string_fn(gv, "id")));
        html_fmt(&out, "</td><td>");
        html_js_esc(&out, js_to_string(call_string_fn(gv, "service")));
        html_fmt(&out, "</td><td>");
        html_js_esc(&out, js_to_string(call_string_fn(gv, "tier")));
        html_fmt(&out, "</td><td>");
        html_js_esc(&out, js_to_string(call_string_fn(gv, "region")));
        html_fmt(&out, "</td><td class=\"num\">%d</td><td class=\"num\">%d</td></tr>",
                 (int)(js_to_number(call_string_fn(gv, "requests")) + 0.5),
                 (int)(js_to_number(call_string_fn(gv, "errors")) + 0.5));
    }
    html_fmt(&out, "</tbody></table>");
    HTMLElement_set_innerHTML(Document_getElementById(g_doc, "table-wrap"), html);

    char meta[256];
    snprintf(meta, sizeof meta, "%s \xC2\xB7 showing %u / %u services", label, nrows, ntotal);
    HTMLElement_set_textContent(Document_getElementById(g_doc, "meta"), meta);
    js_scope_close();
}

static const int BTN_IDS[5] = { 0, 1, 2, 3, 4 };
static void on_click(void* ctx, PointerEvent ev) {
    (void)ev;
    js_scope_open();
    int idx = *(const int*)ctx;
    Table_ColumnFiltering cf = { g_table.h };
    Table_GlobalFiltering gf = { g_table.h };
    JsArray filters = JsArray_new();
    const char* label = "Filter: all services";
    if (idx == 0) {
        call_Updater_fn(Table_GlobalFiltering_get_setGlobalFilter(gf), js_string(""));
        JsArray_push(filters, filter_spec("tier", js_string("core")));
        JsArray_push(filters, filter_spec("requests", js_number(400.0)));
        call_Table_ColumnFiltering_setColumnFilters(Table_ColumnFiltering_get_setColumnFilters(cf), js_value(filters));
        label = "Filter: tier = core & requests >= 400";
    } else if (idx == 1) {
        call_Updater_fn(Table_GlobalFiltering_get_setGlobalFilter(gf), js_string(""));
        call_Table_ColumnFiltering_setColumnFilters(Table_ColumnFiltering_get_setColumnFilters(cf), js_value(filters));
        label = "Filter: all services";
    } else if (idx == 2) {
        call_Updater_fn(Table_GlobalFiltering_get_setGlobalFilter(gf), js_string(""));
        JsArray_push(filters, filter_spec("tier", js_string("edge")));
        call_Table_ColumnFiltering_setColumnFilters(Table_ColumnFiltering_get_setColumnFilters(cf), js_value(filters));
        label = "Filter: tier = edge";
    } else if (idx == 3) {
        call_Table_ColumnFiltering_setColumnFilters(Table_ColumnFiltering_get_setColumnFilters(cf), js_value(filters));
        call_Updater_fn(Table_GlobalFiltering_get_setGlobalFilter(gf), js_string("auth"));
        label = "Global search: \"auth\"";
    } else if (idx == 4) {
        call_Updater_fn(Table_GlobalFiltering_get_setGlobalFilter(gf), js_string(""));
        JsArray_push(filters, filter_spec("errors", js_number(5.0)));
        call_Table_ColumnFiltering_setColumnFilters(Table_ColumnFiltering_get_setColumnFilters(cf), js_value(filters));
        label = "Filter: errors >= 5";
    }
    render(label);
    js_scope_close();
}

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();
    JsValue filter_fns = js_object();
    js_set_val(filter_fns, "includesString", get_filterFn_includesString());
    js_set_val(filter_fns, "equalsString", get_filterFn_equalsString());
    js_set_val(filter_fns, "greaterThanOrEqualTo", get_filterFn_greaterThanOrEqualTo());

    JsValue features = js_object();
    js_set_val(features, "coreReactivityFeature", make_reactivity());
    js_set_val(features, "columnFilteringFeature", get_columnFilteringFeature());
    js_set_val(features, "globalFilteringFeature", get_globalFilteringFeature());
    js_set_val(features, "coreRowModel", createCoreRowModel());
    js_set_val(features, "filteredRowModel", createFilteredRowModel());
    js_set_val(features, "filterFns", filter_fns);

    JsArray data = JsArray_new();
    js_push_val(data, svc_row("S-01", "auth-gateway", "core", "CH", 620, 2));
    js_push_val(data, svc_row("S-02", "session-cache", "core", "CH", 480, 1));
    js_push_val(data, svc_row("S-03", "billing-ledger", "core", "DE", 510, 6));
    js_push_val(data, svc_row("S-04", "oauth-signer", "core", "FR", 430, 3));
    js_push_val(data, svc_row("S-05", "trace-ingestor", "edge", "DE", 290, 8));
    js_push_val(data, svc_row("S-06", "cdn-router", "edge", "FR", 840, 5));
    js_push_val(data, svc_row("S-07", "static-assets", "edge", "CH", 710, 0));
    js_push_val(data, svc_row("S-08", "mail-relay", "batch", "DE", 140, 7));
    js_push_val(data, svc_row("S-09", "report-export", "batch", "FR", 210, 4));
    js_push_val(data, svc_row("S-10", "audit-auth-log", "core", "US", 360, 9));

    JsArray cols = JsArray_new();
    js_push_val(cols, col_def("id", "ID", NULL));
    js_push_val(cols, col_def("service", "Service", NULL));
    js_push_val(cols, col_def("tier", "Tier", "equalsString"));
    js_push_val(cols, col_def("region", "Region", NULL));
    js_push_val(cols, col_def("requests", "Req/s", "greaterThanOrEqualTo"));
    js_push_val(cols, col_def("errors", "Errors", "greaterThanOrEqualTo"));

    JsArray init_filters = JsArray_new();
    js_push_val(init_filters, filter_spec("tier", js_string("core")));
    js_push_val(init_filters, filter_spec("requests", js_number(400.0)));
    JsValue init_state = js_object();
    js_set_val(init_state, "columnFilters", init_filters);

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
    render("Filter: tier = core & requests >= 400");
    js_scope_close();
}
