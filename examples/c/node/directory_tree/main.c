#include "node/node.h"
#include <stdarg.h>
#include <stdio.h>
#include <string.h>

TS_IMPORT(print) void host_print(const char* ptr, uint32_t len);

static int g_passed = 0;
static int g_dir_count = 0;
static int g_file_count = 0;
static int g_total_bytes = 0;

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
    JsString j = path_join(JsArray_JsString_from((const char* const[]){ a, b }, 2));
    JsString_copy(j, out, cap);
}

static void walk(const char* dir) {
    js_scope_open();
    JsArray_JsString names = readdirSync(PathLike_of(dir), (readdir_options){ 0 });
    uint32_t len = JsArray_JsString_length(names);
    for (uint32_t i = 0; i < len; i++) {
        char nbuf[128], full[512];
        JsString_copy(JsArray_JsString_get(names, i), nbuf, sizeof nbuf);
        join2(full, sizeof full, dir, nbuf);
        Stats st = statSync(PathLike_of(full));
        if (Stats_isDirectory(st)) {
            g_dir_count++;
            walk(full);
        } else if (Stats_isFile(st)) {
            g_file_count++;
            g_total_bytes += (int)js_to_number(Stats_get_size(st));
        }
    }
    js_scope_close();
}

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();
    char root[256], src_dir[320], core_dir[360], cli_dir[360], docs_dir[320], file_path[400];
    JsString_copy(mkdtempSync("/tmp/tsrs-node-tree-", (ObjectEncodingOptions_or_BufferEncoding){ .tag = -1 }), root, sizeof root);
    check(existsSync(PathLike_of(root)));

    MakeDirectoryOptions mk = MakeDirectoryOptions_create();
    MakeDirectoryOptions_set_recursive(mk, true);
    join2(src_dir, sizeof src_dir, root, "src");
    join2(core_dir, sizeof core_dir, src_dir, "core");
    join2(cli_dir, sizeof cli_dir, src_dir, "cli");
    join2(docs_dir, sizeof docs_dir, root, "docs");
    mkdirSync_with_opt_MakeDirectoryOptions(PathLike_of(core_dir), mk);
    mkdirSync_with_opt_MakeDirectoryOptions(PathLike_of(cli_dir), mk);
    mkdirSync_with_opt_MakeDirectoryOptions(PathLike_of(docs_dir), mk);

    join2(file_path, sizeof file_path, root, "README.md");
    writeFileSync(PathOrFileDescriptor_of(file_path), "node tree demo", (WriteFileOptions_object_or_BufferEncoding){ .tag = -1 });
    join2(file_path, sizeof file_path, core_dir, "engine.txt");
    writeFileSync(PathOrFileDescriptor_of(file_path), "engine core module!\n", (WriteFileOptions_object_or_BufferEncoding){ .tag = -1 });
    join2(file_path, sizeof file_path, cli_dir, "args.txt");
    writeFileSync(PathOrFileDescriptor_of(file_path), "cli args parser\n", (WriteFileOptions_object_or_BufferEncoding){ .tag = -1 });
    join2(file_path, sizeof file_path, docs_dir, "guide.md");
    writeFileSync(PathOrFileDescriptor_of(file_path), "monochrome documentation\n", (WriteFileOptions_object_or_BufferEncoding){ .tag = -1 });

    walk(root);

    check(g_dir_count == 4);
    check(g_file_count == 4);
    check(g_total_bytes == 75);

    RmOptions rm = RmOptions_create();
    RmOptions_set_recursive(rm, true);
    RmOptions_set_force(rm, true);
    rmSync(PathLike_of(root), rm);
    check(!existsSync(PathLike_of(root)));
    check(g_dir_count + g_file_count == 8);
    check(1);

    emitf("root=%s\ndirs=%d files=%d total_bytes=%d\nPASS: directory_tree (%d checks)\n",
        root, g_dir_count, g_file_count, g_total_bytes, g_passed);
    js_scope_close();
}
