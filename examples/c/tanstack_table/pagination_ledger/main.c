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
    js_set_val(sub, "unsubscribe", Table_void_fn_from(no_unsub, NULL));
    return sub;
}
static JsValue make_atom(AtomState* a) {
    JsValue atom = js_object();
    js_set_val(atom, "get", Table_void_to_any_fn_from(atom_get, a));
    if (!a->is_ro) js_set_val(atom, "set", Table_Updater_fn2_from(atom_set, a));
    js_set_val(atom, "subscribe", Table_Updater_fn2_from(atom_sub, NULL));
    return atom;
}
static Table_Atom create_writable(void* ctx, JsValue initial, TableAtomOptions opts) {
    (void)ctx; (void)opts;
    AtomState* a = &atoms[atom_count++];
    a->value = initial;
    js_retain(initial);
    return (Table_Atom){ make_atom(a).h };
}
static Table_ReadonlyAtom create_readonly(void* ctx, Table_void_to_any_fn fn, TableAtomOptions opts) {
    (void)ctx; (void)opts;
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

static Table_Core g_table;
static Document g_doc;

static void render(void) {
    js_scope_open();
    Table_RowModel rm = table_call_void_to_RowModel_fn(Table_RowModels_Core_get_getRowModel((Table_RowModels_Core){ g_table.h }));
    Table_RowModel core = table_call_void_to_RowModel_fn(Table_RowModels_Core_get_getCoreRowModel((Table_RowModels_Core){ g_table.h }));
    JsArray_Table_Row_Core rows = Table_RowModel_get_rows(rm);
    uint32_t nrows = JsArray_Table_Row_Core_length(rows);
    uint32_t ntotal = JsArray_Table_Row_Core_length(Table_RowModel_get_rows(core));

    static char html[16384];
    HtmlOut out = { html, html + sizeof html };
    html_fmt(&out, "<table><thead><tr><th>Entry</th><th>Account</th><th>Category</th><th class=\"num\">Units</th><th class=\"num\">Amount (CHF)</th></tr></thead><tbody>");
    for (uint32_t i = 0; i < nrows; i++) {
        Table_Row_Core row = JsArray_Table_Row_Core_get(rows, i);
        Table_string_fn gv = Table_Row_Core_get_getValue(row);
        html_fmt(&out, "<tr><td>");
        html_js_esc(&out, js_to_string(table_call_string_fn(gv, "id")));
        html_fmt(&out, "</td><td>");
        html_js_esc(&out, js_to_string(table_call_string_fn(gv, "account")));
        html_fmt(&out, "</td><td>");
        html_js_esc(&out, js_to_string(table_call_string_fn(gv, "category")));
        html_fmt(&out, "</td><td class=\"num\">%d</td><td class=\"num\">%d</td></tr>",
                 (int)(js_to_number(table_call_string_fn(gv, "units")) + 0.5),
                 (int)(js_to_number(table_call_string_fn(gv, "amount")) + 0.5));
    }
    html_fmt(&out, "</tbody></table>");
    HTMLElement_set_innerHTML(Document_getElementById(g_doc, "table-wrap"), html);

    JsValue state = store_state(js_value(g_table));
    JsValue pg = js_get_prop(state, "pagination");
    int page_idx = (int)(js_to_number(js_get_prop(pg, "pageIndex")) + 0.5);
    int page_size = (int)(js_to_number(js_get_prop(pg, "pageSize")) + 0.5);
    int page_count = (int)(table_call_void_to_number_fn(Table_RowPagination_get_getPageCount((Table_RowPagination){ g_table.h })) + 0.5);

    char meta[256];
    snprintf(meta, sizeof meta, "Page %d / %d \xC2\xB7 pageSize %d \xC2\xB7 showing %u of %u rows",
             page_idx + 1, page_count, page_size, nrows, ntotal);
    HTMLElement_set_textContent(Document_getElementById(g_doc, "meta"), meta);
    js_scope_close();
}

static const int BTN_IDS[5] = { 0, 1, 2, 3, 4 };
static void on_click(void* ctx, PointerEvent ev) {
    (void)ev;
    js_scope_open();
    int idx = *(const int*)ctx;
    Table_RowPagination pag = { g_table.h };
    JsValue pg = js_get_prop(store_state(js_value(g_table)), "pagination");
    int page_idx = (int)(js_to_number(js_get_prop(pg, "pageIndex")) + 0.5);
    int page_count = (int)(table_call_void_to_number_fn(Table_RowPagination_get_getPageCount(pag)) + 0.5);

    if (idx == 0) {
        if (page_idx > 0) table_call_Table_RowPagination_setPageIndex(Table_RowPagination_get_setPageIndex(pag), js_number(page_idx - 1));
    } else if (idx == 1) {
        if (page_idx + 1 < page_count) table_call_Table_RowPagination_setPageIndex(Table_RowPagination_get_setPageIndex(pag), js_number(page_idx + 1));
    } else {
        int new_sz = (idx == 2) ? 5 : ((idx == 3) ? 6 : 9);
        table_call_Table_RowPagination_setPageIndex(Table_RowPagination_get_setPageIndex(pag), js_number(0.0));
        table_call_Table_RowPagination_setPageIndex(Table_RowPagination_get_setPageSize(pag), js_number((double)new_sz));
    }
    render();
    js_scope_close();
}

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();
    static const char* accounts[3] = { "ops-pool", "clearing-ch", "reserve-de" };
    static const char* cats[4] = { "compute", "storage", "egress", "audit" };

    JsValue features = js_object();
    js_set_val(features, "coreReactivityFeature", make_reactivity());
    js_set_val(features, "rowPaginationFeature", table_get_rowPaginationFeature());
    js_set_val(features, "coreRowModel", table_createCoreRowModel());
    js_set_val(features, "paginatedRowModel", table_createPaginatedRowModel());

    JsArray data = JsArray_new();
    for (int i = 0; i < 18; i++) {
        char id_buf[16];
        snprintf(id_buf, sizeof id_buf, "TX-%03d", i + 1);
        JsValue row = js_object();
        js_set_str(row, "id", id_buf);
        js_set_str(row, "account", accounts[i % 3]);
        js_set_str(row, "category", cats[i % 4]);
        js_set_num(row, "units", (double)((i * 7) % 19 + 4));
        js_set_num(row, "amount", (double)(((i * 37) % 90 + 15) * 12));
        js_push_val(data, row);
    }

    JsArray cols = JsArray_new();
    js_push_val(cols, col_def("id", "Entry"));
    js_push_val(cols, col_def("account", "Account"));
    js_push_val(cols, col_def("category", "Category"));
    js_push_val(cols, col_def("units", "Units"));
    js_push_val(cols, col_def("amount", "Amount (CHF)"));

    JsValue pg = js_object();
    js_set_num(pg, "pageIndex", 0.0);
    js_set_num(pg, "pageSize", 5.0);
    JsValue init_state = js_object();
    js_set_val(init_state, "pagination", pg);

    JsValue opts = js_object();
    js_set_val(opts, "features", features);
    js_set_val(opts, "data", data);
    js_set_val(opts, "columns", cols);
    js_set_val(opts, "initialState", init_state);

    g_table = table_constructTable((TableOptions_Core){ opts.h });
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
