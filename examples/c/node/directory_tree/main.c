#include "node/node.h"

TS_IMPORT(print) void host_print(const char* ptr, uint32_t len);

static int g_passed = 0;
static int g_dir_count = 0;
static int g_file_count = 0;
static int g_total_bytes = 0;

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

static void mk_dir_rec(const char* p, MakeDirectoryOptions mk) {
    JsString res = mkdirSync_with_opt_MakeDirectoryOptions(P(p), mk);
    js_release(res);
}

static void walk(const char* dir) {
    JsArray_JsString names = readdirSync(P(dir), (readdir_options){ 0 });
    uint32_t len = JsArray_JsString_length(names);
    for (uint32_t i = 0; i < len; i++) {
        JsString n = JsArray_JsString_get(names, i);
        char nbuf[128], full[512];
        JsString_copy(n, nbuf, sizeof nbuf);
        js_release(n);
        join2(full, sizeof full, dir, nbuf);
        Stats st = statSync(P(full));
        if (StatsBase_isDirectory(st)) {
            g_dir_count++;
            js_release(st);
            walk(full);
        } else if (StatsBase_isFile(st)) {
            g_file_count++;
            g_total_bytes += (int)js_to_number(StatsBase_get_size(st));
            js_release(st);
        } else {
            js_release(st);
        }
    }
    js_release(names);
}

__attribute__((export_name("start"))) void start(void) {
    char root[256], src_dir[320], core_dir[360], cli_dir[360], docs_dir[320], file_path[400];
    JsString made = mkdtempSync("/tmp/tsrs-node-tree-", (JsValue){ 0 });
    JsString_copy(made, root, sizeof root);
    js_release(made);
    check(existsSync(P(root)));

    MakeDirectoryOptions mk = MakeDirectoryOptions_create();
    MakeDirectoryOptions_set_recursive(mk, js_some_bool(true));
    join2(src_dir, sizeof src_dir, root, "src");
    join2(core_dir, sizeof core_dir, src_dir, "core");
    join2(cli_dir, sizeof cli_dir, src_dir, "cli");
    join2(docs_dir, sizeof docs_dir, root, "docs");
    mk_dir_rec(core_dir, mk);
    mk_dir_rec(cli_dir, mk);
    mk_dir_rec(docs_dir, mk);
    js_release(mk);

    join2(file_path, sizeof file_path, root, "README.md");
    writeFileSync(F(file_path), "node tree demo", (JsValue){ 0 });
    join2(file_path, sizeof file_path, core_dir, "engine.txt");
    writeFileSync(F(file_path), "engine core module!\n", (JsValue){ 0 });
    join2(file_path, sizeof file_path, cli_dir, "args.txt");
    writeFileSync(F(file_path), "cli args parser\n", (JsValue){ 0 });
    join2(file_path, sizeof file_path, docs_dir, "guide.md");
    writeFileSync(F(file_path), "monochrome documentation\n", (JsValue){ 0 });

    walk(root);

    check(g_dir_count == 4);
    check(g_file_count == 4);
    check(g_total_bytes == 75);

    RmOptions rm = RmOptions_create();
    RmOptions_set_recursive(rm, js_some_bool(true));
    RmOptions_set_force(rm, js_some_bool(true));
    rmSync(P(root), rm);
    js_release(rm);
    check(!existsSync(P(root)));
    check(g_dir_count + g_file_count == 8);
    check(1);

    emit("root="); emit(root); emit("\n");
    emit("dirs="); emit_int(g_dir_count); emit(" files="); emit_int(g_file_count); emit(" total_bytes="); emit_int(g_total_bytes); emit("\n");
    emit("PASS: directory_tree ("); emit_int(g_passed); emit(" checks)\n");
}
