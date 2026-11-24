#include "node/node.h"
#include <stdarg.h>
#include <stdio.h>
#include <string.h>

TS_IMPORT(print) void host_print(const char* ptr, uint32_t len);

static int g_passed = 0;
static int g_stages = 0;
static int g_done = 0;

static void check(bool ok) {
    if (!ok) __builtin_trap();
    g_passed++;
}

static void emitf(const char* fmt, ...) {
    char buf[512];
    va_list ap;
    va_start(ap, fmt);
    int n = vsnprintf(buf, sizeof buf, fmt, ap);
    va_end(ap);
    if (n > 0) host_print(buf, (uint32_t)(n < (int)sizeof buf ? n : (int)sizeof buf - 1));
}

static void join2(char* out, uint32_t cap, const char* a, const char* b) {
    JsString j = node_path_join(JsArray_JsString_from((const char* const[]){ a, b }, 2));
    JsString_copy(j, out, cap);
}

static void on_stage(void* ctx, JsArray args) {
    (void)ctx; (void)args;
    g_stages++;
}

static void on_done(void* ctx, JsArray args) {
    (void)ctx; (void)args;
    g_done++;
}

static void emit_stage(Node_EventEmitter bus, const char* name, const char* label) {
    JsArray a = JsArray_new();
    js_push_str(a, label);
    Node_EventEmitter_emit(bus, name, a);
}

__attribute__((export_name("start"))) void start(void) {
    js_scope();
    Node_EventEmitter bus = Node_EventEmitter_new();
    NodeJS_EventEmitter_on(bus, "stage", Node_any_array_fn_from(on_stage, 0));
    NodeJS_EventEmitter_once(bus, "done", Node_any_array_fn_from(on_done, 0));

    char root[256], in_dir[320], out_dir[320], fp[384], combined[384];
    JsString_copy(node_mkdtempSync("/tmp/tsrs-node-pipe-", (Node_ObjectEncodingOptions_or_BufferEncoding){ .tag = -1 }), root, sizeof root);

    join2(in_dir, sizeof in_dir, root, "in");
    join2(out_dir, sizeof out_dir, root, "out");
    Node_MakeDirectoryOptions mk = Node_MakeDirectoryOptions_create(.recursive = true);
    node_mkdirSync_with_opt_MakeDirectoryOptions(in_dir, mk);
    node_mkdirSync_with_opt_MakeDirectoryOptions(out_dir, mk);

    join2(fp, sizeof fp, in_dir, "a.log");
    node_writeFileSync(fp, "100 OK\n", (Node_WriteFileOptions_object_or_BufferEncoding){ .tag = -1 });
    join2(fp, sizeof fp, in_dir, "b.log");
    node_writeFileSync(fp, "200 OK\n", (Node_WriteFileOptions_object_or_BufferEncoding){ .tag = -1 });
    join2(fp, sizeof fp, in_dir, "c.log");
    node_writeFileSync(fp, "300 OK\n", (Node_WriteFileOptions_object_or_BufferEncoding){ .tag = -1 });
    emit_stage(bus, "stage", "seed");
    check(g_stages == 1);

    const char* log_names[3] = { "a.log", "b.log", "c.log" };
    check(JsArray_JsString_length(node_readdirSync(in_dir, (Node_readdir_options){ 0 })) == 3);

    join2(combined, sizeof combined, out_dir, "combined.log");
    for (int i = 0; i < 3; i++) {
        check(JsString_equals(node_path_extname(log_names[i]), ".log"));
        join2(fp, sizeof fp, in_dir, log_names[i]);
        JsString chunk = node_readFileSync_with_readFileSync_options(fp, Node_BufferEncoding_utf8);
        node_appendFileSync(combined, chunk, (Node_WriteFileOptions_object_or_BufferEncoding){ .tag = -1 });
    }
    emit_stage(bus, "stage", "bundle");
    check(g_stages == 2);

    Node_Stats comb_st = node_statSync(combined);
    int comb_size = (int)js_to_number(Node_Stats_get_size(comb_st));
    char comb_buf[128];
    JsString_copy(node_readFileSync_with_readFileSync_options(combined, Node_BufferEncoding_utf8), comb_buf, sizeof comb_buf);
    check(comb_size == 21 && strstr(comb_buf, "200 OK") != 0);

    join2(fp, sizeof fp, out_dir, "missing.log");
    JsString missing = node_readFileSync_with_readFileSync_options(fp, Node_BufferEncoding_utf8);
    check(missing.h == 0 && js_failed());
    char mbuf[256];
    JsString_copy(js_error_message(js_take_error()), mbuf, sizeof mbuf);
    check(strstr(mbuf, "ENOENT") != 0);

    emit_stage(bus, "stage", "verify");
    emit_stage(bus, "done", "21");
    check(g_stages == 3 && g_done == 1);

    node_rmSync(root, Node_RmOptions_create(.recursive = true, .force = true));
    check(!node_existsSync(root));

    emitf("pipeline_root=%s combined_bytes=%d\nstages=%d done=%d enoent=true\nPASS: file_pipeline (%d checks)\n",
        root, comb_size, g_stages, g_done, g_passed);
}
