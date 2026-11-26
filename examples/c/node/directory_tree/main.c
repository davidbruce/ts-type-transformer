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
    JsString_copy(node_path_join(a, b), out, cap);
}

static void walk(const char* dir) {
    js_scope();
    JsArray_JsString names = node_readdirSync(dir);
    uint32_t len = JsArray_JsString_length(names);
    for (uint32_t i = 0; i < len; i++) {
        char nbuf[128], full[512];
        JsString_copy(JsArray_JsString_get(names, i), nbuf, sizeof nbuf);
        join2(full, sizeof full, dir, nbuf);
        Node_Stats st = node_statSync(full);
        if (Node_Stats_isDirectory(st)) {
            g_dir_count++;
            walk(full);
        } else if (Node_Stats_isFile(st)) {
            g_file_count++;
            g_total_bytes += (int)js_to_number(Node_Stats_get_size(st));
        }
    }
}

__attribute__((export_name("start"))) void start(void) {
    js_scope();
    char root[256], src_dir[320], core_dir[360], cli_dir[360], docs_dir[320], file_path[400];
    JsString_copy(node_mkdtempSync("/tmp/tsrs-node-tree-"), root, sizeof root);
    check(node_existsSync(root));

    Node_MakeDirectoryOptions mk = Node_MakeDirectoryOptions_create(.recursive = true);
    join2(src_dir, sizeof src_dir, root, "src");
    join2(core_dir, sizeof core_dir, src_dir, "core");
    join2(cli_dir, sizeof cli_dir, src_dir, "cli");
    join2(docs_dir, sizeof docs_dir, root, "docs");
    node_mkdirSync_with_MakeDirectoryOptions(core_dir, mk);
    node_mkdirSync_with_MakeDirectoryOptions(cli_dir, mk);
    node_mkdirSync_with_MakeDirectoryOptions(docs_dir, mk);

    join2(file_path, sizeof file_path, root, "README.md");
    node_writeFileSync(file_path, "node tree demo");
    join2(file_path, sizeof file_path, core_dir, "engine.txt");
    node_writeFileSync(file_path, "engine core module!\n");
    join2(file_path, sizeof file_path, cli_dir, "args.txt");
    node_writeFileSync(file_path, "cli args parser\n");
    join2(file_path, sizeof file_path, docs_dir, "guide.md");
    node_writeFileSync(file_path, "monochrome documentation\n");

    walk(root);

    check(g_dir_count == 4);
    check(g_file_count == 4);
    check(g_total_bytes == 75);

    node_rmSync(root, Node_RmOptions_create(.recursive = true, .force = true));
    check(!node_existsSync(root));
    check(g_dir_count + g_file_count == 8);
    check(1);

    emitf("root=%s\ndirs=%d files=%d total_bytes=%d\nPASS: directory_tree (%d checks)\n",
        root, g_dir_count, g_file_count, g_total_bytes, g_passed);
}
