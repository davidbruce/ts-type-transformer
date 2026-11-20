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
    JsString j = path_join(JsArray_JsString_from((const char* const[]){ a, b }, 2));
    JsString_copy(j, out, cap);
}

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();
    char dir[256], notes[320], backup[320], archive[320];
    JsString_copy(mkdtempSync("/tmp/tsrs-node-ws-", (JsValue){ 0 }), dir, sizeof dir);
    Stats dir_st = statSync(PathLike_of(dir));
    check(existsSync(PathLike_of(dir)) && Stats_isDirectory(dir_st));

    join2(notes, sizeof notes, dir, "notes.txt");
    writeFileSync(PathOrFileDescriptor_of(notes), "alpha\nbeta\n", (JsValue){ 0 });
    Stats st1 = statSync(PathLike_of(notes));
    int s1 = (int)js_to_number(Stats_get_size(st1));
    check(existsSync(PathLike_of(notes)) && Stats_isFile(st1) && s1 == 11);

    appendFileSync(PathOrFileDescriptor_of(notes), "gamma\n", (JsValue){ 0 });
    global_BufferEncoding utf8 = { JsString_from("utf8").h };
    JsString text = readFileSync_with_BufferEncoding(PathOrFileDescriptor_of(notes), utf8);
    Stats st2 = statSync(PathLike_of(notes));
    int s2 = (int)js_to_number(Stats_get_size(st2));
    check(JsString_equals(text, "alpha\nbeta\ngamma\n") && s2 == 17);

    join2(backup, sizeof backup, dir, "backup.txt");
    join2(archive, sizeof archive, dir, "archive.txt");
    copyFileSync(PathLike_of(notes), PathLike_of(backup), js_none_double());
    check(existsSync(PathLike_of(backup)));

    renameSync(PathLike_of(backup), PathLike_of(archive));
    check(!existsSync(PathLike_of(backup)) && existsSync(PathLike_of(archive)));

    JsString arch_text = readFileSync_with_BufferEncoding(PathOrFileDescriptor_of(archive), utf8);
    check(JsString_equals(arch_text, text));

    unlinkSync(PathLike_of(archive));
    check(!existsSync(PathLike_of(archive)));

    RmOptions rm = RmOptions_create();
    RmOptions_set_recursive(rm, true);
    RmOptions_set_force(rm, true);
    rmSync(PathLike_of(dir), rm);
    check(!existsSync(PathLike_of(dir)));
    check(1);

    emitf("workspace=%s\nsize_before=%d size_after=%d\nPASS: temp_workspace (%d checks)\n",
        dir, s1, s2, g_passed);
    js_scope_close();
}
