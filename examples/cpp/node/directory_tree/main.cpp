#include <algorithm>
#include <cstdint>
#include <string>
#include <string_view>
#include <vector>

#include "node/node.hpp"

using namespace node;

extern "C" TS_IMPORT(print) void host_print(const char* ptr, uint32_t len);

static void println(std::string_view s) {
    host_print(s.data(), static_cast<uint32_t>(s.size()));
    host_print("\n", 1);
}

static void walk_dir(const std::string& dir, int& dirs, int& files, double& total_bytes) {
    auto entries = fs::readdirSync(dir);
    std::vector<std::string> sorted;
    for (std::string name : entries) {
        sorted.push_back(name);
    }
    std::sort(sorted.begin(), sorted.end());
    for (const std::string& name : sorted) {
        std::string full = path::join(dir, name);
        Stats st = fs::statSync(full);
        if (st.isDirectory()) {
            dirs++;
            walk_dir(full, dirs, files, total_bytes);
        } else if (st.isFile()) {
            files++;
            total_bytes += st.size();
        }
    }
}

TS_EXPORT(start) void start() {
    int passed = 0;
    auto check = [&](bool ok) {
        if (!ok) __builtin_trap();
        passed++;
    };

    std::string root = fs::mkdtempSync("/tmp/tsrs-node-tree-");
    check(fs::existsSync(root));

    auto mk = mkdir_options::create();
    check(fs::mkdirSync(path::join(root, "src", "core"), mk).has_value());
    check(fs::mkdirSync(path::join(root, "src", "cli"), mk).has_value());
    check(fs::mkdirSync(path::join(root, "docs"), mk).has_value());

    fs::writeFileSync(path::join(root, "README.md"), "node tree demo");
    fs::writeFileSync(path::join(root, "src", "core", "engine.txt"), "engine core module!\n");
    fs::writeFileSync(path::join(root, "src", "cli", "args.txt"), "cli args parser\n");
    fs::writeFileSync(path::join(root, "docs", "guide.md"), "monochrome documentation\n");

    int dir_count = 0;
    int file_count = 0;
    double total_bytes = 0.0;
    walk_dir(root, dir_count, file_count, total_bytes);

    check(dir_count == 4);
    check(file_count == 4);
    check(total_bytes == 75.0);

    fs::rmSync(root, {{.force = true, .recursive = true}});
    check(!fs::existsSync(root));
    check(dir_count + file_count == 8);
    check(true);

    println("root=" + root);
    println("dirs=" + std::to_string(dir_count) +
            " files=" + std::to_string(file_count) +
            " total_bytes=" + std::to_string(static_cast<int>(total_bytes)));
    println("PASS: directory_tree (" + std::to_string(passed) + " checks)");
}
