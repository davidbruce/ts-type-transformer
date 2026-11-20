#include <stdarg.h>
#include <stdio.h>
#include "dom/dom.h"
#include "tanstack_table_core/tanstack_table_core.h"

static const JsValue NONE = { 0 };
static JsValue store_state(JsValue tbl) { return functionalUpdate(js_get_prop(js_get_prop(tbl, "store"), "get"), NONE); }

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
static JsValue node_row(const char* id, const char* host, const char* region, double lat, double thr, double up) {
    JsValue r = js_object();
    js_set_str(r, "id", id);
    js_set_str(r, "host", host);
    js_set_str(r, "region", region);
    js_set_num(r, "latency", lat);
    js_set_num(r, "throughput", thr);
    js_set_num(r, "uptime", up);
    return r;
}
static JsValue sort_spec(const char* id, bool desc) {
    JsValue s = js_object();
    js_set_str(s, "id", id);
    js_set_bool(s, "desc", desc);
    return s;
}

static Table_Core g_table;
static Document g_doc;

static void render(const char* label) {
    js_scope_open();
    RowModel rm = call_void_to_RowModel_fn(Table_RowModels_Core_get_getRowModel((Table_RowModels_Core){ g_table.h }));
    JsArray_Row_Core rows = RowModel_get_rows(rm);
    JsArray_Column_Core cols = call_Table_ColumnPinning_getCenterLeafColumns(
        Table_Core_get_getAllLeafColumns(g_table)
    );
    (void)store_state(js_value(g_table));

    static char html[16384];
    HtmlOut out = { html, html + sizeof html };
    html_fmt(&out, "<table><thead><tr>");
    uint32_t ncols = JsArray_Column_Core_length(cols);
    for (uint32_t c = 0; c < ncols; c++) {
        Column_Core col = JsArray_Column_Core_get(cols, c);
        JsString cid = Column_Core_get_id(col);
        JsString hdr = js_to_string(js_get_prop(Column_Core_get_columnDef(col), "header"));
        bool is_num = JsString_equals(cid, "latency") || JsString_equals(cid, "throughput") || JsString_equals(cid, "uptime");
        html_fmt(&out, "%s", is_num ? "<th class=\"num\">" : "<th>");
        html_js_esc(&out, hdr);
        boolean_or_SortDirection sd = call_Column_RowSorting_getIsSorted(Column_RowSorting_get_getIsSorted((Column_RowSorting){ col.h }));
        if (sd.tag == boolean_or_SortDirection_SortDirection) {
            if (sd.as.SortDirection == SortDirection_asc) html_fmt(&out, " &#9650;");
            else if (sd.as.SortDirection == SortDirection_desc) html_fmt(&out, " &#9660;");
        }
        html_fmt(&out, "</th>");
    }
    html_fmt(&out, "</tr></thead><tbody>");

    uint32_t nrows = JsArray_Row_Core_length(rows);
    for (uint32_t i = 0; i < nrows; i++) {
        Row_Core row = JsArray_Row_Core_get(rows, i);
        string_fn gv = Row_Core_get_getValue(row);
        html_fmt(&out, "<tr><td>");
        html_js_esc(&out, js_to_string(call_string_fn(gv, "id")));
        html_fmt(&out, "</td><td>");
        html_js_esc(&out, js_to_string(call_string_fn(gv, "host")));
        html_fmt(&out, "</td><td>");
        html_js_esc(&out, js_to_string(call_string_fn(gv, "region")));
        html_fmt(&out, "</td><td class=\"num\">%.1f</td><td class=\"num\">%d</td><td class=\"num\">%.2f%%</td></tr>",
                 js_to_number(call_string_fn(gv, "latency")),
                 (int)(js_to_number(call_string_fn(gv, "throughput")) + 0.5),
                 js_to_number(call_string_fn(gv, "uptime")));
    }
    html_fmt(&out, "</tbody></table>");
    HTMLElement_set_innerHTML(Document_getElementById(g_doc, "table-wrap"), html);

    char top_host[128] = "-";
    if (nrows > 0) {
        Row_Core first = JsArray_Row_Core_get(rows, 0);
        JsString th = js_to_string(call_string_fn(Row_Core_get_getValue(first), "host"));
        uint32_t len = JsString_copy(th, top_host, sizeof top_host - 1);
        top_host[len < sizeof top_host ? len : sizeof top_host - 1] = 0;
    }
    char meta[256];
    snprintf(meta, sizeof meta, "%s \xC2\xB7 %u rows \xC2\xB7 top node: %s", label, nrows, top_host);
    HTMLElement_set_textContent(Document_getElementById(g_doc, "meta"), meta);
    js_scope_close();
}

static const int BTN_IDS[5] = { 0, 1, 2, 3, 4 };
static void on_click(void* ctx, PointerEvent ev) {
    (void)ev;
    js_scope_open();
    int idx = *(const int*)ctx;
    JsArray next = JsArray_new();
    const char* label = "Sort: unsorted (data order)";
    if (idx == 0) {
        js_push_val(next, sort_spec("region", false));
        js_push_val(next, sort_spec("latency", false));
        label = "Sort: region asc, latency asc";
    } else if (idx == 1) {
        js_push_val(next, sort_spec("latency", false));
        label = "Sort: latency asc";
    } else if (idx == 2) {
        js_push_val(next, sort_spec("throughput", true));
        label = "Sort: throughput desc";
    } else if (idx == 3) {
        js_push_val(next, sort_spec("host", false));
        label = "Sort: host asc";
    }
    call_Table_RowSorting_setSorting(
        Table_RowSorting_get_setSorting((Table_RowSorting){ g_table.h }),
        js_value(next)
    );
    render(label);
    js_scope_close();
}

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();
    JsValue sort_fns = js_object();
    js_set_val(sort_fns, "alphanumeric", get_sortFn_alphanumeric());
    js_set_val(sort_fns, "basic", get_sortFn_basic());
    js_set_val(sort_fns, "text", get_sortFn_text());

    JsValue features = js_object();
    js_set_val(features, "coreReactivityFeature", make_reactivity());
    js_set_val(features, "rowSortingFeature", get_rowSortingFeature());
    js_set_val(features, "coreRowModel", createCoreRowModel());
    js_set_val(features, "sortedRowModel", createSortedRowModel());
    js_set_val(features, "sortFns", sort_fns);

    JsArray data = JsArray_new();
    js_push_val(data, node_row("N-01", "eu-west-1a", "CH", 12.4, 4200, 99.98));
    js_push_val(data, node_row("N-02", "eu-west-1b", "CH", 8.7, 5100, 99.99));
    js_push_val(data, node_row("N-03", "eu-central-1a", "DE", 15.2, 3900, 99.91));
    js_push_val(data, node_row("N-04", "eu-central-1b", "DE", 6.3, 6400, 99.97));
    js_push_val(data, node_row("N-05", "eu-north-1a", "SE", 19.8, 2800, 99.85));
    js_push_val(data, node_row("N-06", "eu-north-1b", "SE", 11.1, 4750, 99.95));
    js_push_val(data, node_row("N-07", "us-east-1a", "US", 28.5, 7100, 99.94));
    js_push_val(data, node_row("N-08", "us-east-1b", "US", 21.0, 5900, 99.96));

    JsArray cols = JsArray_new();
    js_push_val(cols, col_def("id", "Node"));
    js_push_val(cols, col_def("host", "Host"));
    js_push_val(cols, col_def("region", "Region"));
    js_push_val(cols, col_def("latency", "Latency (ms)"));
    js_push_val(cols, col_def("throughput", "Req/s"));
    js_push_val(cols, col_def("uptime", "Uptime %"));

    JsArray init_sort = JsArray_new();
    js_push_val(init_sort, sort_spec("region", false));
    js_push_val(init_sort, sort_spec("latency", false));
    JsValue init_state = js_object();
    js_set_val(init_state, "sorting", init_sort);

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
    render("Sort: region asc, latency asc");
    js_scope_close();
}
