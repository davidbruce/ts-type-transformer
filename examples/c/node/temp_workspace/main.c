#include "node/node.h"
#include <stdarg.h>
#include <stdio.h>

TS_IMPORT(print) void host_print(const char* ptr, uint32_t len);

static int g_passed = 0;

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

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();
    char dir[256], notes[320], backup[320], archive[320];
    JsString_copy(node_mkdtempSync("/tmp/tsrs-node-ws-", (Node_ObjectEncodingOptions_or_BufferEncoding){ .tag = -1 }), dir, sizeof dir);
    Node_Stats dir_st = node_statSync(Node_PathLike_of(dir));
    check(node_existsSync(Node_PathLike_of(dir)) && Node_Stats_isDirectory(dir_st));

    join2(notes, sizeof notes, dir, "notes.txt");
    node_writeFileSync(Node_PathOrFileDescriptor_of(notes), "alpha\nbeta\n", (Node_WriteFileOptions_object_or_BufferEncoding){ .tag = -1 });
    Node_Stats st1 = node_statSync(Node_PathLike_of(notes));
    int s1 = (int)js_to_number(Node_Stats_get_size(st1));
    check(node_existsSync(Node_PathLike_of(notes)) && Node_Stats_isFile(st1) && s1 == 11);

    node_appendFileSync(Node_PathOrFileDescriptor_of(notes), "gamma\n", (Node_WriteFileOptions_object_or_BufferEncoding){ .tag = -1 });
    JsString text = node_readFileSync_with_BufferEncoding(Node_PathOrFileDescriptor_of(notes), Node_BufferEncoding_utf8);
    Node_Stats st2 = node_statSync(Node_PathLike_of(notes));
    int s2 = (int)js_to_number(Node_Stats_get_size(st2));
    check(JsString_equals(text, "alpha\nbeta\ngamma\n") && s2 == 17);

    join2(backup, sizeof backup, dir, "backup.txt");
    join2(archive, sizeof archive, dir, "archive.txt");
    node_copyFileSync(Node_PathLike_of(notes), Node_PathLike_of(backup), js_none_double());
    check(node_existsSync(Node_PathLike_of(backup)));

    node_renameSync(Node_PathLike_of(backup), Node_PathLike_of(archive));
    check(!node_existsSync(Node_PathLike_of(backup)) && node_existsSync(Node_PathLike_of(archive)));

    JsString arch_text = node_readFileSync_with_BufferEncoding(Node_PathOrFileDescriptor_of(archive), Node_BufferEncoding_utf8);
    check(JsString_equals(arch_text, text));

    node_unlinkSync(Node_PathLike_of(archive));
    check(!node_existsSync(Node_PathLike_of(archive)));

    Node_RmOptions rm = Node_RmOptions_create();
    Node_RmOptions_set_recursive(rm, true);
    Node_RmOptions_set_force(rm, true);
    node_rmSync(Node_PathLike_of(dir), rm);
    check(!node_existsSync(Node_PathLike_of(dir)));
    check(1);

    emitf("workspace=%s\nsize_before=%d size_after=%d\nPASS: temp_workspace (%d checks)\n",
        dir, s1, s2, g_passed);
    js_scope_close();
}
