#include <cstdint>
#include <string>
#include <string_view>

#include "node/node.hpp"

using namespace node;

extern "C" TS_IMPORT(print) void host_print(const char* ptr, uint32_t len);

static void println(std::string_view s) {
    host_print(s.data(), static_cast<uint32_t>(s.size()));
    host_print("\n", 1);
}

TS_EXPORT(start) void start() {
    int passed = 0;
    auto check = [&](bool ok) {
        if (!ok) __builtin_trap();
        passed++;
    };

    std::string joined = path::join("/workspace", "packages", "core", "..", "cli", "index.ts");
    check(joined == "/workspace/packages/cli/index.ts");

    std::string norm = path::normalize("/var//log/./app/../http/access.log");
    check(norm == "/var/log/http/access.log");

    std::string resolved = path::resolve("/srv/app", "../shared", "config/prod.json");
    check(resolved == "/srv/shared/config/prod.json");

    std::string rel = path::relative("/srv/app/src/ui", "/srv/app/dist/assets");
    check(rel == "../../dist/assets");

    check(path::dirname("/srv/app/archive.tar.gz") == "/srv/app");
    check(path::basename("/srv/app/archive.tar.gz") == "archive.tar.gz");
    check(path::basename("/srv/app/archive.tar.gz", ".gz") == "archive.tar");
    check(path::extname("/srv/app/archive.tar.gz") == ".gz");
    check(path::isAbsolute("/srv/app") && !path::isAbsolute("src/app"));

    auto parsed = path::parse("/srv/app/archive.tar.gz");
    check(parsed.root() == "/" && parsed.dir() == "/srv/app" &&
          parsed.base() == "archive.tar.gz" && parsed.name() == "archive.tar" &&
          parsed.ext() == ".gz");

    auto fmt = path_FormatInputPathObject::create()
        .dir("/etc/service")
        .name("daemon")
        .ext(".conf");
    std::string formatted = path::format(fmt);
    check(formatted == "/etc/service/daemon.conf");

    check(path::matchesGlob("/src/components/Button.tsx", "/src/**/*.tsx") &&
          !path::matchesGlob("/src/components/Button.css", "/src/**/*.tsx"));

    println("join=" + joined + " norm=" + norm);
    println("resolve=" + resolved + " relative=" + rel + " format=" + formatted);
    println("parsed=(root=" + parsed.root() + " dir=" + parsed.dir() +
            " base=" + parsed.base() + " ext=" + parsed.ext() + ")");
    println("PASS: path_inspector (" + std::to_string(passed) + " checks)");
}
