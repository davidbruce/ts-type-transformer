#include <algorithm>
#include <cstdint>
#include <string>
#include <string_view>
#include <vector>

#include "node/node.hpp"

using namespace node;

extern "C" __attribute__((import_module("env"), import_name("print")))
void host_print(const char* ptr, uint32_t len);

static int g_passed = 0;

static void check(bool ok) {
    if (!ok) __builtin_trap();
    g_passed++;
}

static void println(std::string_view s) {
    host_print(s.data(), static_cast<uint32_t>(s.size()));
    host_print("\n", 1);
}

static void walk_dir(const std::string& dir, int& dirs, int& files, double& total_bytes) {
    ts::Array<std::string> entries = fs::readdirSync(dir);
    std::vector<std::string> sorted;
    for (std::string name : entries) {
        sorted.push_back(name);
    }
    std::sort(sorted.begin(), sorted.end());
    for (const std::string& name : sorted) {
        std::string full = path::join({dir, name});
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

extern "C" __attribute__((export_name("start"))) void start() {
    std::string root = fs::mkdtempSync("/tmp/tsrs-node-tree-");
    check(fs::existsSync(root));

    mkdir_options mk = mkdir_options::create();
    check(fs::mkdirSync(path::join({root, "src", "core"}), mk).has_value());
    check(fs::mkdirSync(path::join({root, "src", "cli"}), mk).has_value());
    check(fs::mkdirSync(path::join({root, "docs"}), mk).has_value());

    fs::writeFileSync(path::join({root, "README.md"}), std::string("node tree demo"));
    fs::writeFileSync(path::join({root, "src", "core", "engine.txt"}), std::string("engine core module!\n"));
    fs::writeFileSync(path::join({root, "src", "cli", "args.txt"}), std::string("cli args parser\n"));
    fs::writeFileSync(path::join({root, "docs", "guide.md"}), std::string("monochrome documentation\n"));

    int dir_count = 0;
    int file_count = 0;
    double total_bytes = 0.0;
    walk_dir(root, dir_count, file_count, total_bytes);

    check(dir_count == 4);
    check(file_count == 4);
    check(total_bytes == 75.0);

    RmOptions rm = RmOptions::create();
    rm.recursive(true);
    rm.force(true);
    fs::rmSync(root, rm);
    check(!fs::existsSync(root));
    check(dir_count + file_count == 8);
    check(true);

    println("root=" + root);
    println("dirs=" + std::to_string(dir_count) +
            " files=" + std::to_string(file_count) +
            " total_bytes=" + std::to_string(static_cast<int>(total_bytes)));
    println("PASS: directory_tree (" + std::to_string(g_passed) + " checks)");
}
