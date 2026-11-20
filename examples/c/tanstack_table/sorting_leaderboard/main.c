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
    if (val < 0) {
        if (s->pos < 16380) s->buf[s->pos++] = '-';
        val = -val;
    }
    if (val == 0) tmp[n++] = '0';
    while (val > 0) {
        tmp[n++] = (char)('0' + (val % 10));
        val /= 10;
    }
    for (int i = n - 1; i >= 0 && s->pos < 16380; i--) s->buf[s->pos++] = tmp[i];
}
static void sb_fixed(StrBuf* s, double val, int decimals) {
    int mult = (decimals == 2) ? 100 : 10;
    int scaled = (int)(val * (double)mult + 0.5);
    sb_int(s, scaled / mult);
    if (s->pos < 16380) s->buf[s->pos++] = '.';
    int frac = scaled % mult;
    if (decimals == 2 && frac < 10 && s->pos < 16380) s->buf[s->pos++] = '0';
    sb_int(s, frac);
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
static JsValue node_row(const char* id, const char* host, const char* region, double lat, double thr, double up) {
    RowData_map r = makeObjectMap();
    RowData_map_set_at(r, "id", js_string(id));
    RowData_map_set_at(r, "host", js_string(host));
    RowData_map_set_at(r, "region", js_string(region));
    RowData_map_set_at(r, "latency", js_number(lat));
    RowData_map_set_at(r, "throughput", js_number(thr));
    RowData_map_set_at(r, "uptime", js_number(up));
    return (JsValue){ r.h };
}
static JsValue sort_spec(const char* id, bool desc) {
    RowData_map s = makeObjectMap();
    RowData_map_set_at(s, "id", js_string(id));
    RowData_map_set_at(s, "desc", js_bool(desc));
    return (JsValue){ s.h };
}

static Table_Core g_table;
static Document g_doc;

static void render(const char* label) {
    RowModel rm = call_void_to_RowModel_fn(Table_RowModels_Core_get_getRowModel((Table_RowModels_Core){ g_table.h }));
    JsArray_Row_Core rows = RowModel_get_rows(rm);
    JsArray_Column_Core cols = call_Table_ColumnPinning_getCenterLeafColumns(
        Table_Columns_get_getAllLeafColumns((Table_Columns){ g_table.h })
    );
    (void)store_state((JsValue){ g_table.h });

    static StrBuf sb;
    sb.pos = 0;
    sb_str(&sb, "<table><thead><tr>");
    uint32_t ncols = JsArray_Column_Core_length(cols);
    for (uint32_t c = 0; c < ncols; c++) {
        Column_Core col = JsArray_Column_Core_get(cols, c);
        JsString cid = Column_CoreProperties_get_id((Column_CoreProperties){ col.h });
        JsString hdr = js_to_string(get_prop((JsValue){ Column_CoreProperties_get_columnDef((Column_CoreProperties){ col.h }).h }, "header"));
        bool is_num = JsString_equals(cid, "latency") || JsString_equals(cid, "throughput") || JsString_equals(cid, "uptime");
        sb_str(&sb, is_num ? "<th class=\"num\">" : "<th>");
        sb_js_esc(&sb, hdr);
        boolean_or_SortDirection sd = call_Column_RowSorting_getIsSorted(Column_RowSorting_get_getIsSorted((Column_RowSorting){ col.h }));
        if (sd.tag == boolean_or_SortDirection_SortDirection) {
            if (sd.as.SortDirection == SortDirection_asc) sb_str(&sb, " &#9650;");
            else if (sd.as.SortDirection == SortDirection_desc) sb_str(&sb, " &#9660;");
        }
        sb_str(&sb, "</th>");
    }
    sb_str(&sb, "</tr></thead><tbody>");

    uint32_t nrows = JsArray_Row_Core_length(rows);
    for (uint32_t i = 0; i < nrows; i++) {
        Row_Core row = JsArray_Row_Core_get(rows, i);
        string_fn gv = Row_Row_get_getValue((Row_Row){ row.h });
        sb_str(&sb, "<tr><td>");
        sb_js_esc(&sb, js_to_string(call_string_fn(gv, "id")));
        sb_str(&sb, "</td><td>");
        sb_js_esc(&sb, js_to_string(call_string_fn(gv, "host")));
        sb_str(&sb, "</td><td>");
        sb_js_esc(&sb, js_to_string(call_string_fn(gv, "region")));
        sb_str(&sb, "</td><td class=\"num\">");
        sb_fixed(&sb, js_to_number(call_string_fn(gv, "latency")), 1);
        sb_str(&sb, "</td><td class=\"num\">");
        sb_int(&sb, (int)(js_to_number(call_string_fn(gv, "throughput")) + 0.5));
        sb_str(&sb, "</td><td class=\"num\">");
        sb_fixed(&sb, js_to_number(call_string_fn(gv, "uptime")), 2);
        sb_str(&sb, "%</td></tr>");
    }
    sb_str(&sb, "</tbody></table>");
    sb.buf[sb.pos] = 0;

    HTMLElement wrap = Document_getElementById(g_doc, "table-wrap");
    Element_set_innerHTML(wrap, sb.buf);

    sb.pos = 0;
    sb_str(&sb, label);
    sb_str(&sb, " \xC2\xB7 ");
    sb_int(&sb, (int)nrows);
    sb_str(&sb, " rows \xC2\xB7 top node: ");
    if (nrows > 0) {
        Row_Core first = JsArray_Row_Core_get(rows, 0);
        sb_js_esc(&sb, js_to_string(call_string_fn(Row_Row_get_getValue((Row_Row){ first.h }), "host")));
    } else {
        sb_str(&sb, "-");
    }
    sb.buf[sb.pos] = 0;
    HTMLElement meta = Document_getElementById(g_doc, "meta");
    Node_set_textContent(meta, sb.buf);
}

static const int BTN_IDS[5] = { 0, 1, 2, 3, 4 };
static void on_click(void* ctx, PointerEvent ev) {
    (void)ev;
    int idx = *(const int*)ctx;
    JsArray next = JsArray_new();
    const char* label = "Sort: unsorted (data order)";
    if (idx == 0) {
        JsArray_push(next, sort_spec("region", false));
        JsArray_push(next, sort_spec("latency", false));
        label = "Sort: region asc, latency asc";
    } else if (idx == 1) {
        JsArray_push(next, sort_spec("latency", false));
        label = "Sort: latency asc";
    } else if (idx == 2) {
        JsArray_push(next, sort_spec("throughput", true));
        label = "Sort: throughput desc";
    } else if (idx == 3) {
        JsArray_push(next, sort_spec("host", false));
        label = "Sort: host asc";
    }
    call_Table_RowSorting_setSorting(
        Table_RowSorting_get_setSorting((Table_RowSorting){ g_table.h }),
        (JsValue){ next.h }
    );
    render(label);
}

__attribute__((export_name("start"))) void start(void) {
    RowData_map sort_fns = makeObjectMap();
    RowData_map_set_at(sort_fns, "alphanumeric", (JsValue){ get_sortFn_alphanumeric().h });
    RowData_map_set_at(sort_fns, "basic", (JsValue){ get_sortFn_basic().h });
    RowData_map_set_at(sort_fns, "text", (JsValue){ get_sortFn_text().h });

    RowData_map features = makeObjectMap();
    RowData_map_set_at(features, "coreReactivityFeature", (JsValue){ make_reactivity().h });
    RowData_map_set_at(features, "rowSortingFeature", (JsValue){ get_rowSortingFeature().h });
    RowData_map_set_at(features, "coreRowModel", (JsValue){ createCoreRowModel().h });
    RowData_map_set_at(features, "sortedRowModel", (JsValue){ createSortedRowModel().h });
    RowData_map_set_at(features, "sortFns", (JsValue){ sort_fns.h });

    JsArray data = JsArray_new();
    JsArray_push(data, node_row("N-01", "eu-west-1a", "CH", 12.4, 4200, 99.98));
    JsArray_push(data, node_row("N-02", "eu-west-1b", "CH", 8.7, 5100, 99.99));
    JsArray_push(data, node_row("N-03", "eu-central-1a", "DE", 15.2, 3900, 99.91));
    JsArray_push(data, node_row("N-04", "eu-central-1b", "DE", 6.3, 6400, 99.97));
    JsArray_push(data, node_row("N-05", "eu-north-1a", "SE", 19.8, 2800, 99.85));
    JsArray_push(data, node_row("N-06", "eu-north-1b", "SE", 11.1, 4750, 99.95));
    JsArray_push(data, node_row("N-07", "us-east-1a", "US", 28.5, 7100, 99.94));
    JsArray_push(data, node_row("N-08", "us-east-1b", "US", 21.0, 5900, 99.96));

    JsArray cols = JsArray_new();
    JsArray_push(cols, col_def("id", "Node"));
    JsArray_push(cols, col_def("host", "Host"));
    JsArray_push(cols, col_def("region", "Region"));
    JsArray_push(cols, col_def("latency", "Latency (ms)"));
    JsArray_push(cols, col_def("throughput", "Req/s"));
    JsArray_push(cols, col_def("uptime", "Uptime %"));

    JsArray init_sort = JsArray_new();
    JsArray_push(init_sort, sort_spec("region", false));
    JsArray_push(init_sort, sort_spec("latency", false));
    RowData_map init_state = makeObjectMap();
    RowData_map_set_at(init_state, "sorting", (JsValue){ init_sort.h });

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
    render("Sort: region asc, latency asc");
}
