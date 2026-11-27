#include "bindings.h"

TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

__attribute__((export_name("run"))) void run(void) {
    Perms rw = Perms_or(Perms_Read, Perms_Write);
    CHECK(rw == Perms_ReadWrite);
    CHECK(Perms_has(rw, Perms_Read));
    CHECK(Perms_has(rw, Perms_Write));
    CHECK(!Perms_has(rw, Perms_Exec));
    CHECK(Perms_and(rw, Perms_Read) == Perms_Read);
    CHECK(Perms_xor(rw, Perms_Read) == Perms_Write);
    CHECK(Perms_not(Perms_None) != Perms_None);

    Perms re = combinePerms(Perms_Read, Perms_Exec);
    CHECK(hasPerm(re, Perms_Read) && hasPerm(re, Perms_Exec) && !hasPerm(re, Perms_Write));
    CHECK(defaultPerms(js_none_double()) == Perms_or(Perms_Read, Perms_Exec));
    CHECK(defaultPerms(js_some_double((double)Perms_Write)) == Perms_Write);
}
