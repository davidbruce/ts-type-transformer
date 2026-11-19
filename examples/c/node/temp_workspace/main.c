#include "node/node.h"

TS_IMPORT(print) void host_print(const char* ptr, uint32_t len);

static int g_passed = 0;

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

__attribute__((export_name("start"))) void start(void) {
    char dir[256], notes[320], backup[320], archive[320];
    JsString made = mkdtempSync("/tmp/tsrs-node-ws-", (JsValue){ 0 });
    JsString_copy(made, dir, sizeof dir);
    js_release(made);
    Stats dir_st = statSync(P(dir));
    check(existsSync(P(dir)) && StatsBase_isDirectory(dir_st));
    js_release(dir_st);

    join2(notes, sizeof notes, dir, "notes.txt");
    writeFileSync(F(notes), "alpha\nbeta\n", (JsValue){ 0 });
    Stats st1 = statSync(P(notes));
    int s1 = (int)js_to_number(StatsBase_get_size(st1));
    check(existsSync(P(notes)) && StatsBase_isFile(st1) && s1 == 11);
    js_release(st1);

    appendFileSync(F(notes), "gamma\n", (JsValue){ 0 });
    global_BufferEncoding utf8 = { JsString_from("utf8").h };
    JsString text = readFileSync_with_BufferEncoding(F(notes), utf8);
    Stats st2 = statSync(P(notes));
    int s2 = (int)js_to_number(StatsBase_get_size(st2));
    check(JsString_equals(text, "alpha\nbeta\ngamma\n") && s2 == 17);
    js_release(st2);

    join2(backup, sizeof backup, dir, "backup.txt");
    join2(archive, sizeof archive, dir, "archive.txt");
    copyFileSync(P(notes), P(backup), js_none_double());
    check(existsSync(P(backup)));

    renameSync(P(backup), P(archive));
    check(!existsSync(P(backup)) && existsSync(P(archive)));

    JsString arch_text = readFileSync_with_BufferEncoding(F(archive), utf8);
    char tbuf[64];
    JsString_copy(text, tbuf, sizeof tbuf);
    check(JsString_equals(arch_text, tbuf));
    js_release(arch_text);
    js_release(text);
    ts_raw_js_release(utf8.h);

    unlinkSync(P(archive));
    check(!existsSync(P(archive)));

    RmOptions rm = RmOptions_create();
    RmOptions_set_recursive(rm, js_some_bool(true));
    RmOptions_set_force(rm, js_some_bool(true));
    rmSync(P(dir), rm);
    js_release(rm);
    check(!existsSync(P(dir)));
    check(1);

    emit("workspace="); emit(dir); emit("\n");
    emit("size_before="); emit_int(s1); emit(" size_after="); emit_int(s2); emit("\n");
    emit("PASS: temp_workspace ("); emit_int(g_passed); emit(" checks)\n");
}
