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

static void take_copy(JsString s, char* out, uint32_t cap) {
    JsString_copy(s, out, cap);
    js_release(s);
}

static bool same(const char* a, const char* b) {
    while (*a && *a == *b) a++, b++;
    return *a == *b;
}

__attribute__((export_name("start"))) void start(void) {
    char jbuf[128], nbuf[128], rbuf[128], lbuf[128], fbuf[128], tmp[128];

    JsArray_JsString jparts = JsArray_JsString_new();
    JsArray_JsString_push(jparts, "/workspace");
    JsArray_JsString_push(jparts, "packages");
    JsArray_JsString_push(jparts, "core");
    JsArray_JsString_push(jparts, "..");
    JsArray_JsString_push(jparts, "cli");
    JsArray_JsString_push(jparts, "index.ts");
    take_copy(path_join(jparts), jbuf, sizeof jbuf);
    js_release(jparts);
    check(same(jbuf, "/workspace/packages/cli/index.ts"));

    take_copy(path_normalize("/var//log/./app/../http/access.log"), nbuf, sizeof nbuf);
    check(same(nbuf, "/var/log/http/access.log"));

    JsArray_JsString rparts = JsArray_JsString_new();
    JsArray_JsString_push(rparts, "/srv/app");
    JsArray_JsString_push(rparts, "../shared");
    JsArray_JsString_push(rparts, "config/prod.json");
    take_copy(path_resolve(rparts), rbuf, sizeof rbuf);
    js_release(rparts);
    check(same(rbuf, "/srv/shared/config/prod.json"));

    take_copy(path_relative("/srv/app/src/ui", "/srv/app/dist/assets"), lbuf, sizeof lbuf);
    check(same(lbuf, "../../dist/assets"));

    take_copy(path_dirname("/srv/app/archive.tar.gz"), tmp, sizeof tmp);
    check(same(tmp, "/srv/app"));
    take_copy(path_basename("/srv/app/archive.tar.gz"), tmp, sizeof tmp);
    check(same(tmp, "archive.tar.gz"));
    take_copy(path_basename("/srv/app/archive.tar.gz", ".gz"), tmp, sizeof tmp);
    check(same(tmp, "archive.tar"));
    take_copy(path_extname("/srv/app/archive.tar.gz"), tmp, sizeof tmp);
    check(same(tmp, ".gz"));
    check(path_isAbsolute("/srv/app") && !path_isAbsolute("src/app"));

    path_ParsedPath parsed = path_parse("/srv/app/archive.tar.gz");
    char proot[32], pdir[64], pbase[64], pname[64], pext[32];
    take_copy(path_ParsedPath_get_root(parsed), proot, sizeof proot);
    take_copy(path_ParsedPath_get_dir(parsed), pdir, sizeof pdir);
    take_copy(path_ParsedPath_get_base(parsed), pbase, sizeof pbase);
    take_copy(path_ParsedPath_get_name(parsed), pname, sizeof pname);
    take_copy(path_ParsedPath_get_ext(parsed), pext, sizeof pext);
    js_release(parsed);
    check(same(proot, "/") && same(pdir, "/srv/app") && same(pbase, "archive.tar.gz") && same(pname, "archive.tar") && same(pext, ".gz"));

    path_FormatInputPathObject fmt = path_FormatInputPathObject_create();
    path_FormatInputPathObject_set_dir(fmt, "/etc/service");
    path_FormatInputPathObject_set_name(fmt, "daemon");
    path_FormatInputPathObject_set_ext(fmt, ".conf");
    take_copy(path_format(fmt), fbuf, sizeof fbuf);
    js_release(fmt);
    check(same(fbuf, "/etc/service/daemon.conf"));

    check(path_matchesGlob("/src/components/Button.tsx", "/src/**/*.tsx") && !path_matchesGlob("/src/components/Button.css", "/src/**/*.tsx"));

    emit("join="); emit(jbuf); emit(" norm="); emit(nbuf); emit("\n");
    emit("resolve="); emit(rbuf); emit(" relative="); emit(lbuf); emit(" format="); emit(fbuf); emit("\n");
    emit("parsed=(root="); emit(proot); emit(" dir="); emit(pdir); emit(" base="); emit(pbase); emit(" ext="); emit(pext); emit(")\n");
    emit("PASS: path_inspector ("); emit_int(g_passed); emit(" checks)\n");
}
