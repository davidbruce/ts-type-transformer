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

    std::string dir = fs::mkdtempSync("/tmp/tsrs-node-ws-");
    check(fs::existsSync(dir) && fs::statSync(dir).isDirectory());

    std::string notes = path::join(dir, "notes.txt");
    fs::writeFileSync(notes, "alpha\nbeta\n");
    Stats st1 = fs::statSync(notes);
    check(fs::existsSync(notes) && st1.isFile() && st1.size() == 11);

    fs::appendFileSync(notes, "gamma\n");
    std::string text = fs::readFileSync(notes).toString();
    Stats st2 = fs::statSync(notes);
    check(text == "alpha\nbeta\ngamma\n" && st2.size() == 17);

    std::string backup = path::join(dir, "backup.txt");
    std::string archive = path::join(dir, "archive.txt");
    fs::copyFileSync(notes, backup);
    check(fs::existsSync(backup));

    fs::renameSync(backup, archive);
    check(!fs::existsSync(backup) && fs::existsSync(archive));
    check(fs::readFileSync(archive).toString() == text);

    fs::unlinkSync(archive);
    check(!fs::existsSync(archive));

    auto rm = RmOptions::create().recursive(true).force(true);
    fs::rmSync(dir, rm);
    check(!fs::existsSync(dir));
    check(true);

    println("workspace=" + dir);
    println("size_before=" + std::to_string(static_cast<int>(st1.size())) +
            " size_after=" + std::to_string(static_cast<int>(st2.size())));
    println("PASS: temp_workspace (" + std::to_string(passed) + " checks)");
}
