#include "node/node.h"
#include <stdarg.h>
#include <stdio.h>
#include <string.h>

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

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();
    char jbuf[128], nbuf[128], rbuf[128], lbuf[128], fbuf[128], tmp[128];

    const char* join_items[] = { "/workspace", "packages", "core", "..", "cli", "index.ts" };
    JsString_copy(path_join(JsArray_JsString_from(join_items, 6)), jbuf, sizeof jbuf);
    check(strcmp(jbuf, "/workspace/packages/cli/index.ts") == 0);

    JsString_copy(path_normalize("/var//log/./app/../http/access.log"), nbuf, sizeof nbuf);
    check(strcmp(nbuf, "/var/log/http/access.log") == 0);

    const char* resolve_items[] = { "/srv/app", "../shared", "config/prod.json" };
    JsString_copy(path_resolve(JsArray_JsString_from(resolve_items, 3)), rbuf, sizeof rbuf);
    check(strcmp(rbuf, "/srv/shared/config/prod.json") == 0);

    JsString_copy(path_relative("/srv/app/src/ui", "/srv/app/dist/assets"), lbuf, sizeof lbuf);
    check(strcmp(lbuf, "../../dist/assets") == 0);

    JsString_copy(path_dirname("/srv/app/archive.tar.gz"), tmp, sizeof tmp);
    check(strcmp(tmp, "/srv/app") == 0);
    JsString_copy(path_basename("/srv/app/archive.tar.gz"), tmp, sizeof tmp);
    check(strcmp(tmp, "archive.tar.gz") == 0);
    JsString_copy(path_basename("/srv/app/archive.tar.gz", ".gz"), tmp, sizeof tmp);
    check(strcmp(tmp, "archive.tar") == 0);
    JsString_copy(path_extname("/srv/app/archive.tar.gz"), tmp, sizeof tmp);
    check(strcmp(tmp, ".gz") == 0);
    check(path_isAbsolute("/srv/app") && !path_isAbsolute("src/app"));

    path_ParsedPath parsed = path_parse("/srv/app/archive.tar.gz");
    char proot[32], pdir[64], pbase[64], pname[64], pext[32];
    JsString_copy(path_ParsedPath_get_root(parsed), proot, sizeof proot);
    JsString_copy(path_ParsedPath_get_dir(parsed), pdir, sizeof pdir);
    JsString_copy(path_ParsedPath_get_base(parsed), pbase, sizeof pbase);
    JsString_copy(path_ParsedPath_get_name(parsed), pname, sizeof pname);
    JsString_copy(path_ParsedPath_get_ext(parsed), pext, sizeof pext);
    check(strcmp(proot, "/") == 0 && strcmp(pdir, "/srv/app") == 0 && strcmp(pbase, "archive.tar.gz") == 0
        && strcmp(pname, "archive.tar") == 0 && strcmp(pext, ".gz") == 0);

    path_FormatInputPathObject fmt = path_FormatInputPathObject_create();
    path_FormatInputPathObject_set_dir(fmt, "/etc/service");
    path_FormatInputPathObject_set_name(fmt, "daemon");
    path_FormatInputPathObject_set_ext(fmt, ".conf");
    JsString_copy(path_format(fmt), fbuf, sizeof fbuf);
    check(strcmp(fbuf, "/etc/service/daemon.conf") == 0);

    check(path_matchesGlob("/src/components/Button.tsx", "/src/**/*.tsx") && !path_matchesGlob("/src/components/Button.css", "/src/**/*.tsx"));

    emitf("join=%s norm=%s\nresolve=%s relative=%s format=%s\nparsed=(root=%s dir=%s base=%s ext=%s)\nPASS: path_inspector (%d checks)\n",
        jbuf, nbuf, rbuf, lbuf, fbuf, proot, pdir, pbase, pext, g_passed);
    js_scope_close();
}
