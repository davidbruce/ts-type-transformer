#include "bindings.hpp"

extern "C" TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

extern "C" __attribute__((export_name("run"))) void run() {
    Perms rw = Perms::Read | Perms::Write;
    CHECK(rw == Perms::ReadWrite);
    CHECK(has_flag(rw, Perms::Read));
    CHECK(has_flag(rw, Perms::Write));
    CHECK(!has_flag(rw, Perms::Exec));
    CHECK((rw & Perms::Read) == Perms::Read);
    CHECK((rw ^ Perms::Read) == Perms::Write);

    Perms acc = Perms::Read;
    acc |= Perms::Exec;
    CHECK(acc == (Perms::Read | Perms::Exec));
    acc &= Perms::Exec;
    CHECK(acc == Perms::Exec);

    Perms re = combinePerms(Perms::Read, Perms::Exec);
    CHECK(hasPerm(re, Perms::Read) && hasPerm(re, Perms::Exec) && !hasPerm(re, Perms::Write));
    CHECK(defaultPerms() == (Perms::Read | Perms::Exec));
    CHECK(defaultPerms(Perms::Write) == Perms::Write);
}
