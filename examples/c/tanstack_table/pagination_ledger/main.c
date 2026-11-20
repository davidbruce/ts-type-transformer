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
static void sb_js_esc(StrBuf* s, JsString jstr) {
    char tmp[128];
    uint32_t len = JsString_copy(jstr, tmp, sizeof tmp);
    if (len >= sizeof tmp) len = sizeof tmp - 1;
    for (uint32_t i = 0; i < len; i++) {
        char c = tmp[i];
        if (c == '&') sb_str(s, "&amp;");
        else if (c == '<') sb_str(s, "&lt;");
        else if (c == '>') sb_str(s, "&gt;");
        else if (s->pos < 16380) s->buf[s->pos++] = c;
    }
}

static JsValue col_def(const char* key, const char* header) {
    RowData_map c = makeObjectMap();
    RowData_map_set_at(c, "accessorKey", js_string(key));
    RowData_map_set_at(c, "header", js_string(header));
    return (JsValue){ c.h };
}

static Table_Core g_table;
static Document g_doc;

static void render(void) {
    RowModel rm = call_void_to_RowModel_fn(Table_RowModels_Core_get_getRowModel((Table_RowModels_Core){ g_table.h }));
    RowModel core = call_void_to_RowModel_fn(Table_RowModels_Core_get_getCoreRowModel((Table_RowModels_Core){ g_table.h }));
    JsArray_Row_Core rows = RowModel_get_rows(rm);
    uint32_t nrows = JsArray_Row_Core_length(rows);
    uint32_t ntotal = JsArray_Row_Core_length(RowModel_get_rows(core));

    static StrBuf sb;
    sb.pos = 0;
    sb_str(&sb, "<table><thead><tr><th>Entry</th><th>Account</th><th>Category</th><th class=\"num\">Units</th><th class=\"num\">Amount (CHF)</th></tr></thead><tbody>");
    for (uint32_t i = 0; i < nrows; i++) {
        Row_Core row = JsArray_Row_Core_get(rows, i);
        string_fn gv = Row_Row_get_getValue((Row_Row){ row.h });
        sb_str(&sb, "<tr><td>");
        sb_js_esc(&sb, js_to_string(call_string_fn(gv, "id")));
        sb_str(&sb, "</td><td>");
        sb_js_esc(&sb, js_to_string(call_string_fn(gv, "account")));
        sb_str(&sb, "</td><td>");
        sb_js_esc(&sb, js_to_string(call_string_fn(gv, "category")));
        sb_str(&sb, "</td><td class=\"num\">");
        sb_int(&sb, (int)(js_to_number(call_string_fn(gv, "units")) + 0.5));
        sb_str(&sb, "</td><td class=\"num\">");
        sb_int(&sb, (int)(js_to_number(call_string_fn(gv, "amount")) + 0.5));
        sb_str(&sb, "</td></tr>");
    }
    sb_str(&sb, "</tbody></table>");
    sb.buf[sb.pos] = 0;
    Element_set_innerHTML(Document_getElementById(g_doc, "table-wrap"), sb.buf);

    JsValue state = store_state((JsValue){ g_table.h });
    JsValue pg = get_prop(state, "pagination");
    int page_idx = (int)(js_to_number(get_prop(pg, "pageIndex")) + 0.5);
    int page_size = (int)(js_to_number(get_prop(pg, "pageSize")) + 0.5);
    int page_count = (int)(call_void_to_number_fn(Table_RowPagination_get_getPageCount((Table_RowPagination){ g_table.h })) + 0.5);

    sb.pos = 0;
    sb_str(&sb, "Page ");
    sb_int(&sb, page_idx + 1);
    sb_str(&sb, " / ");
    sb_int(&sb, page_count);
    sb_str(&sb, " \xC2\xB7 pageSize ");
    sb_int(&sb, page_size);
    sb_str(&sb, " \xC2\xB7 showing ");
    sb_int(&sb, (int)nrows);
    sb_str(&sb, " of ");
    sb_int(&sb, (int)ntotal);
    sb_str(&sb, " rows");
    sb.buf[sb.pos] = 0;
    Node_set_textContent(Document_getElementById(g_doc, "meta"), sb.buf);
}

static const int BTN_IDS[5] = { 0, 1, 2, 3, 4 };
static void on_click(void* ctx, PointerEvent ev) {
    (void)ev;
    int idx = *(const int*)ctx;
    Table_RowPagination pag = { g_table.h };
    JsValue pg = get_prop(store_state((JsValue){ g_table.h }), "pagination");
    int page_idx = (int)(js_to_number(get_prop(pg, "pageIndex")) + 0.5);
    int page_count = (int)(call_void_to_number_fn(Table_RowPagination_get_getPageCount(pag)) + 0.5);

    if (idx == 0) {
        if (page_idx > 0) call_Table_RowPagination_setPageIndex(Table_RowPagination_get_setPageIndex(pag), js_number(page_idx - 1));
    } else if (idx == 1) {
        if (page_idx + 1 < page_count) call_Table_RowPagination_setPageIndex(Table_RowPagination_get_setPageIndex(pag), js_number(page_idx + 1));
    } else {
        int new_sz = (idx == 2) ? 5 : ((idx == 3) ? 6 : 9);
        call_Table_RowPagination_setPageIndex(Table_RowPagination_get_setPageIndex(pag), js_number(0.0));
        call_Table_RowPagination_setPageIndex(Table_RowPagination_get_setPageSize(pag), js_number((double)new_sz));
    }
    render();
}

__attribute__((export_name("start"))) void start(void) {
    static const char* accounts[3] = { "ops-pool", "clearing-ch", "reserve-de" };
    static const char* cats[4] = { "compute", "storage", "egress", "audit" };

    RowData_map features = makeObjectMap();
    RowData_map_set_at(features, "coreReactivityFeature", (JsValue){ make_reactivity().h });
    RowData_map_set_at(features, "rowPaginationFeature", (JsValue){ get_rowPaginationFeature().h });
    RowData_map_set_at(features, "coreRowModel", (JsValue){ createCoreRowModel().h });
    RowData_map_set_at(features, "paginatedRowModel", (JsValue){ createPaginatedRowModel().h });

    JsArray data = JsArray_new();
    for (int i = 0; i < 18; i++) {
        char id_buf[16];
        int num = i + 1;
        id_buf[0] = 'T'; id_buf[1] = 'X'; id_buf[2] = '-';
        id_buf[3] = (char)('0' + (num / 100) % 10);
        id_buf[4] = (char)('0' + (num / 10) % 10);
        id_buf[5] = (char)('0' + (num % 10));
        id_buf[6] = 0;
        RowData_map row = makeObjectMap();
        RowData_map_set_at(row, "id", js_string(id_buf));
        RowData_map_set_at(row, "account", js_string(accounts[i % 3]));
        RowData_map_set_at(row, "category", js_string(cats[i % 4]));
        RowData_map_set_at(row, "units", js_number((double)((i * 7) % 19 + 4)));
        RowData_map_set_at(row, "amount", js_number((double)(((i * 37) % 90 + 15) * 12)));
        JsArray_push(data, (JsValue){ row.h });
    }

    JsArray cols = JsArray_new();
    JsArray_push(cols, col_def("id", "Entry"));
    JsArray_push(cols, col_def("account", "Account"));
    JsArray_push(cols, col_def("category", "Category"));
    JsArray_push(cols, col_def("units", "Units"));
    JsArray_push(cols, col_def("amount", "Amount (CHF)"));

    RowData_map pg = makeObjectMap();
    RowData_map_set_at(pg, "pageIndex", js_number(0.0));
    RowData_map_set_at(pg, "pageSize", js_number(5.0));
    RowData_map init_state = makeObjectMap();
    RowData_map_set_at(init_state, "pagination", (JsValue){ pg.h });

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
