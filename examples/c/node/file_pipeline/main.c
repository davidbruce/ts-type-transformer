#include "node/node.h"

TS_IMPORT(print) void host_print(const char* ptr, uint32_t len);

static int g_passed = 0;
static int g_stages = 0;
static int g_done = 0;

static void check(int ok) {
    if (!ok) __builtin_trap();
    g_passed++;
}

static void emit(const char* s) {
    host_print(s, ts_strlen(s));
}

static void emit_int(int n) {
    char buf[24];
    int i = 0;
    if (n == 0) {
        buf[i++] = '0';
    } else {
        char rev[24];
        int r = 0;
        if (n < 0) {
            emit("-");
            n = -n;
        }
        while (n > 0) {
            rev[r++] = (char)('0' + (n % 10));
            n /= 10;
        }
        while (r > 0) buf[i++] = rev[--r];
    }
    host_print(buf, (uint32_t)i);
}

static PathLike P(const char* s) { return PathLike_of_string(JsString_from(s)); }
static PathOrFileDescriptor F(const char* s) { return PathOrFileDescriptor_of_string(JsString_from(s)); }

static void join2(char* out, uint32_t cap, const char* a, const char* b) {
    JsArray_JsString parts = JsArray_JsString_new();
    JsArray_JsString_push(parts, a);
    JsArray_JsString_push(parts, b);
    JsString j = path_join(parts);
    JsString_copy(j, out, cap);
    js_release(j);
    js_release(parts);
}

static bool has_str(const char* hay, const char* needle) {
    uint32_t n = ts_strlen(needle);
    if (n == 0) return true;
    for (uint32_t i = 0; hay[i]; i++) {
        uint32_t j = 0;
        while (needle[j] && hay[i + j] == needle[j]) j++;
        if (j == n) return true;
    }
    return false;
}

static void on_stage(void* ctx, JsArray args) {
    (void)ctx;
    (void)args;
    g_stages++;
}

static void on_done(void* ctx, JsArray args) {
    (void)ctx;
    (void)args;
    g_done++;
}

static void emit_stage(NodeJS_EventEmitter bus, const char* name, const char* label) {
    JsArray a = JsArray_new();
    JsValue v = js_string(label);
    JsArray_push(a, v);
    NodeJS_EventEmitter_emit(bus, name, a);
    js_release(v);
    js_release(a);
}

__attribute__((export_name("start"))) void start(void) {
    EventEmitter raw_bus = EventEmitter_new((EventEmitterOptions){ 0 });
    NodeJS_EventEmitter bus = EventEmitter_as_NodeJS_EventEmitter(raw_bus);
    any_array_fn stage_cb = any_array_fn_from(on_stage, 0);
    any_array_fn done_cb = any_array_fn_from(on_done, 0);
    NodeJS_EventEmitter_on(bus, "stage", stage_cb);
    NodeJS_EventEmitter_once(bus, "done", done_cb);

    char root[256], in_dir[320], out_dir[320], fp[384], combined[384];
    JsString made = mkdtempSync("/tmp/tsrs-node-pipe-", (JsValue){ 0 });
    JsString_copy(made, root, sizeof root);
    js_release(made);

    join2(in_dir, sizeof in_dir, root, "in");
    join2(out_dir, sizeof out_dir, root, "out");
    MakeDirectoryOptions mk = MakeDirectoryOptions_create();
    MakeDirectoryOptions_set_recursive(mk, js_some_bool(true));
    js_release(mkdirSync_with_opt_MakeDirectoryOptions(P(in_dir), mk));
    js_release(mkdirSync_with_opt_MakeDirectoryOptions(P(out_dir), mk));
    js_release(mk);

    join2(fp, sizeof fp, in_dir, "a.log");
    writeFileSync(F(fp), "100 OK\n", (JsValue){ 0 });
    join2(fp, sizeof fp, in_dir, "b.log");
    writeFileSync(F(fp), "200 OK\n", (JsValue){ 0 });
    join2(fp, sizeof fp, in_dir, "c.log");
    writeFileSync(F(fp), "300 OK\n", (JsValue){ 0 });
    emit_stage(bus, "stage", "seed");
    check(g_stages == 1);

    const char* log_names[3] = { "a.log", "b.log", "c.log" };
    JsArray_JsString listed = readdirSync(P(in_dir), (readdir_options){ 0 });
    check(JsArray_JsString_length(listed) == 3);
    js_release(listed);

    join2(combined, sizeof combined, out_dir, "combined.log");
    global_BufferEncoding utf8 = { JsString_from("utf8").h };
    for (int i = 0; i < 3; i++) {
        JsString ext = path_extname(log_names[i]);
        check(JsString_equals(ext, ".log"));
        js_release(ext);
        join2(fp, sizeof fp, in_dir, log_names[i]);
        JsString chunk = readFileSync_with_BufferEncoding(F(fp), utf8);
        char cbuf[64];
        JsString_copy(chunk, cbuf, sizeof cbuf);
        js_release(chunk);
        appendFileSync(F(combined), cbuf, (JsValue){ 0 });
    }
    emit_stage(bus, "stage", "bundle");
    check(g_stages == 2);

    Stats comb_st = statSync(P(combined));
    int comb_size = (int)js_to_number(StatsBase_get_size(comb_st));
    js_release(comb_st);
    JsString comb_text = readFileSync_with_BufferEncoding(F(combined), utf8);
    char comb_buf[128];
    JsString_copy(comb_text, comb_buf, sizeof comb_buf);
    js_release(comb_text);
    check(comb_size == 21 && has_str(comb_buf, "200 OK"));

    join2(fp, sizeof fp, out_dir, "missing.log");
    JsString missing = readFileSync_with_BufferEncoding(F(fp), utf8);
    check(missing.h == 0 && js_failed());
    JsValue err = js_take_error();
    JsString msg = js_error_message(err);
    char mbuf[256];
    JsString_copy(msg, mbuf, sizeof mbuf);
    check(has_str(mbuf, "ENOENT"));
    js_release(msg); js_release(err);
    ts_raw_js_release(utf8.h);

    emit_stage(bus, "stage", "verify");
    emit_stage(bus, "done", "21");
    check(g_stages == 3 && g_done == 1);

    RmOptions rm = RmOptions_create();
    RmOptions_set_recursive(rm, js_some_bool(true));
    RmOptions_set_force(rm, js_some_bool(true));
    rmSync(P(root), rm);
    js_release(rm);
    check(!existsSync(P(root)));

    emit("pipeline_root="); emit(root); emit(" combined_bytes="); emit_int(comb_size); emit("\n");
    emit("stages="); emit_int(g_stages); emit(" done="); emit_int(g_done); emit(" enoent=true\n");
    emit("PASS: file_pipeline ("); emit_int(g_passed); emit(" checks)\n");

    js_release(done_cb); js_release(stage_cb); js_release(raw_bus);
}
